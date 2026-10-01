use harness::prelude::*;
use std::cmp::Reverse;

// Rewritten (rewrites/3782.sql): the PostRank window is tie-broken on p.Id.
// WITH UserBadges AS (SELECT ub.UserId, COUNT(*) AS BadgeCount, SUM(CASE WHEN ub.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN ub.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN ub.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Badges ub GROUP BY ub.UserId),
// RecentPosts AS (SELECT p.Id AS PostId, p.OwnerUserId, p.Title, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC, p.Id) AS PostRank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days')
// SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(ub.BadgeCount, 0) AS TotalBadges, COALESCE(ub.GoldBadges, 0) AS GoldBadges, COALESCE(ub.SilverBadges, 0) AS SilverBadges,
//        COALESCE(ub.BronzeBadges, 0) AS BronzeBadges, rp.PostId, rp.Title AS RecentTitle, rp.CreationDate AS RecentCreationDate
// FROM Users u LEFT JOIN UserBadges ub ON u.Id = ub.UserId LEFT JOIN RecentPosts rp ON u.Id = rp.OwnerUserId AND rp.PostRank = 1
// WHERE u.Reputation > 1000 ORDER BY u.Reputation DESC, RecentCreationDate DESC FETCH FIRST 10 ROWS ONLY;
fn q3782(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let Post { owner_user, creation_date, .. } = &db.post;
    let recent = drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(owner_user));
    let top = top_per(recent, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let pp = rel(top.into_iter().map(|(p, u)| (u, p)).collect());
    let by_user: HashIdx<Id<User>, (Id<User>, Id<Post>)> = (&pp).map(|(u, _)| u).inv().select(&pp).collect();
    let v = drain(db.user.with((&db.user.reputation).gt(1000)).select((&ub).and((&by_user).map(|(_, p)| p).opt())));
    let v = top_n(v, |&(u, (_, p))| (Reverse(db.user.reputation.get(u).unwrap()), p.is_none(), Reverse(p.map(|p| creation_date.get(p).unwrap()))), 10);
    rows(v.into_iter().map(|(u, (b, p))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(b.map(V::I));
        f.extend(match p {
            Some(p) => post_fields(db, p, &["id", "title", "created"]),
            None => vec![V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 WHEN v.VoteTypeId = 3 THEN -1 ELSE 0 END), 0) AS NetVotes,
//        COUNT(c.Id) AS CommentCount, COUNT(b.Id) AS BadgeCount
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Badges b ON p.OwnerUserId = b.UserId
//     WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate),
// TopPosts AS (SELECT PostId, Title, CreationDate, NetVotes, CommentCount, RANK() OVER (ORDER BY NetVotes DESC, CommentCount DESC) AS Rank FROM RankedPosts)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.NetVotes, tp.CommentCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY tp.Rank ORDER BY tp.CreationDate DESC) AS RowInRank
// FROM TopPosts tp JOIN Users u ON u.Id = (SELECT OwnerUserId FROM Posts WHERE Id = tp.PostId) WHERE tp.Rank <= 10 ORDER BY tp.Rank, RowInRank;
fn q5813(db: &'static So) -> String {
    let Post { owner_user, creation_date, .. } = &db.post;
    let rp = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()).and(owner_user.select(badges_of(db)).opt()))
        .fold([0i64; 2], |a, ((t, c), _)| [a[0] + match t { Some(2) => 1, Some(3) => -1, _ => 0 }, a[1] + c.is_some() as i64]);
    let v = ranked(drain(&rp), |&(_, a)| (Reverse(a[0]), Reverse(a[1])), false);
    type R = (Id<Post>, [i64; 2], i64);
    let tp = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|((p, a), r)| (p, a, r)).collect::<Vec<R>>());
    let v = drain((&tp).select(Same::<R>::new().and(Same::<R>::new().map(|(p, _, _)| p).select(owner_user))));
    let v = ranked(v.into_iter().map(|(_, x)| x).collect(), |&((p, _, r), _)| (r, Reverse(creation_date.get(p).unwrap()), p), false);
    let v = per_group(v, |&((_, _, r), _)| r);
    rows(v.into_iter().map(|(((p, a, _), u), n)| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(a[0]), V::I(a[1]), user_col(db, u, "name"), V::I(n)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, P.AnswerCount, U.DisplayName AS OwnerName,
//        RANK() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC) AS PostRank
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// UserPostStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts,
//        SUM(CASE WHEN P.Score < 0 THEN 1 ELSE 0 END) AS NegativePosts, SUM(P.ViewCount) AS TotalViews
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName)
// SELECT R.PostId, R.Title, R.CreationDate, R.Score, R.ViewCount, R.AnswerCount, R.OwnerName, U.TotalPosts, U.PositivePosts, U.NegativePosts, U.TotalViews
// FROM RankedPosts R JOIN UserPostStats U ON R.OwnerName = U.DisplayName WHERE R.PostRank <= 5 ORDER BY R.Score DESC, U.TotalViews DESC;
fn q6953(db: &'static So) -> String {
    let Post { owner_user, creation_date, post_type_id, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| Reverse(score.get(p).unwrap()), 5, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ups = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(score.and(view_count.opt())).opt()).fold([0i64; 5], |a, p| match p {
        Some((s, w)) => [a[0] + 1, a[1] + (s > 0) as i64, a[2] + (s < 0) as i64, a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0)],
        None => a,
    });
    let by_name: HashIdx<Str, Id<User>> = (&db.user.display_name).inv().collect();
    let v = drain((&tp).select(owner_user.select(&db.user.display_name).select(&by_name).select(&ups)));
    rows(v.into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), nullable(a[4], a[3])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS OwnerDisplayName, p.Score, p.ViewCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate ASC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// TopPosts AS (SELECT PostId, Title, CreationDate, OwnerDisplayName, Score, ViewCount FROM RankedPosts WHERE Rank <= 5),
// PostVoteCounts AS (SELECT PostId, COUNT(CASE WHEN VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN VoteTypeId = 3 THEN 1 END) AS DownVotes FROM Votes GROUP BY PostId)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.OwnerDisplayName, tp.Score, tp.ViewCount, pvc.UpVotes, pvc.DownVotes,
//        (tp.Score + COALESCE(pvc.UpVotes, 0) - COALESCE(pvc.DownVotes, 0)) AS NetScore
// FROM TopPosts tp LEFT JOIN PostVoteCounts pvc ON tp.PostId = pvc.PostId ORDER BY NetScore DESC;
fn q8926(db: &'static So) -> String {
    let Post { owner_user, creation_date, post_type_id, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), creation_date.get(p).unwrap(), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| [a[0] + t.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain(&pv).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner", "score", "views"]);
        let (u, d) = if a[0] == 0 { (V::Null, V::Null) } else { (V::I(a[1]), V::I(a[2])) };
        f.extend([u, d, V::I(score.get(p).unwrap() + a[1] - a[2])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, u.DisplayName AS OwnerName, t.TagName AS PrimaryTag,
//        ROW_NUMBER() OVER (PARTITION BY u.Id ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id JOIN Tags t ON t.WikiPostId = p.Id
//     WHERE u.Reputation > 1000 AND p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 YEAR'),
// RecentTopPosts AS (SELECT rp.* FROM RankedPosts rp WHERE rp.PostRank = 1)
// SELECT rtp.PostId, rtp.Title, rtp.CreationDate, rtp.Score, rtp.OwnerName, rtp.PrimaryTag, COUNT(DISTINCT c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM RecentTopPosts rtp LEFT JOIN Comments c ON rtp.PostId = c.PostId LEFT JOIN Votes v ON rtp.PostId = v.PostId
// GROUP BY rtp.PostId, rtp.Title, rtp.CreationDate, rtp.Score, rtp.OwnerName, rtp.PrimaryTag ORDER BY rtp.Score DESC, rtp.CreationDate DESC LIMIT 10;
fn q8150(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, .. } = &db.post;
    let wiki: HashIdx<Id<Post>, Id<Tag>> = (&db.tag.wiki_post).inv().collect();
    let rich = Ident::<User>::new().with((&db.user.reputation).gt(1000));
    let v = drain(db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user.select(rich).and(&wiki)));
    let top = top_per(v, |&(_, (u, _))| u, |&(p, (_, t))| (Reverse(creation_date.get(p).unwrap()), p, t), 1, false);
    type R = (Id<Post>, Id<Tag>);
    let rtp = rel(top.into_iter().map(|(p, (_, t))| (p, t)).collect::<Vec<R>>());
    let posts: MatSet<Id<Post>> = (&rtp).map(|(p, _)| p).collect();
    let pv = (&posts).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = (&posts).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&rtp).select(Same::<R>::new().and(Same::<R>::new().map(|(p, _)| p).select((&pv).and(&cc)))));
    let v = top_n(v, |&(_, ((p, t), _))| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p, t), 10);
    rows(v.into_iter().map(|(_, ((p, t), (a, c)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "owner"]);
        f.extend([V::S(db.tag.tag_name.get(t).unwrap()), V::I(c), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotesCount,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotesCount, COUNT(DISTINCT CASE WHEN P.PostTypeId = 1 THEN P.Id END) AS QuestionsCount,
//        COUNT(DISTINCT CASE WHEN P.PostTypeId = 2 THEN P.Id END) AS AnswersCount, COUNT(DISTINCT B.Id) AS BadgesCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, UpVotesCount, DownVotesCount, QuestionsCount, AnswersCount, BadgesCount, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserStats)
// SELECT T.DisplayName, T.Reputation, T.UpVotesCount, T.DownVotesCount, T.QuestionsCount, T.AnswersCount, T.BadgesCount FROM TopUsers T WHERE T.Rank <= 10 ORDER BY T.Reputation DESC;
//
// Rank reads only Reputation, so the ten users are picked first and the posts x votes x badges product is driven for those alone.
fn q6079(db: &'static So) -> String {
    let tu = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let prod = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt().and(badges_of(db).opt()))
        .fold([0i64; 2], |a, (t, _)| [a[0] + (t == Some(Some(2))) as i64, a[1] + (t == Some(Some(3))) as i64]);
    let qa = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(&db.post.post_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(1)) as i64, a[1] + (t == Some(2)) as i64]);
    let bc = (&tu).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    rows(drain((&prod).and(&qa).and(&bc)).into_iter().map(|(u, ((a, q), b))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(q[0]), V::I(q[1]), V::I(b)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.CreationDate, COUNT(DISTINCT c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY COUNT(DISTINCT c.Id) DESC) AS PostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.OwnerUserId, p.CreationDate),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, RANK() OVER (ORDER BY u.Reputation DESC) AS ReputationRank FROM Users u)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.CommentCount, rp.UpVoteCount, rp.DownVoteCount, ur.Reputation, ur.ReputationRank
// FROM RankedPosts rp JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId WHERE rp.PostRank <= 5 ORDER BY rp.CommentCount DESC, ur.Reputation DESC;
//
// PostRank reads only the distinct comment count, so the posts are picked on that first and the comment x vote product is driven for those alone.
fn q7388(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let qs = db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))));
    let cc = qs.group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&cc).and(owner_user.opt()));
    let top = top_per(v, |&(_, (_, u))| u, |&(_, (c, _))| Reverse(c), 5, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pv = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let ur = rel(ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false).into_iter().map(|((u, _), r)| (u, r)).collect());
    let by_user: HashIdx<Id<User>, (Id<User>, i64)> = (&ur).map(|(u, _)| u).inv().select(&ur).collect();
    rows(drain((&pv).and(&cc).and(owner_user.select(&by_user))).into_iter().map(|(p, ((a, c), (u, r)))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1]), user_col(db, u, "rep"), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.Tags ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, OwnerDisplayName FROM RankedPosts WHERE Rank <= 5)
// SELECT t.PostId, t.Title, t.CreationDate, t.Score, t.ViewCount, t.OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes
// FROM TopPosts t LEFT JOIN Comments c ON t.PostId = c.PostId LEFT JOIN Votes v ON t.PostId = v.PostId
// GROUP BY t.PostId, t.Title, t.CreationDate, t.Score, t.ViewCount, t.OwnerDisplayName ORDER BY t.Score DESC, t.ViewCount DESC;
fn q7087(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, tags_str, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).with(owner_user).select(tags_str.opt()));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pv = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain(&pv).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, COUNT(DISTINCT v.Id) AS VoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
//        SUM(COALESCE(b.Class, 0)) AS BadgeCount, MAX(p.CreationDate) AS LastPostDate
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, VoteCount, UpVoteCount, DownVoteCount, BadgeCount, LastPostDate,
//        ROW_NUMBER() OVER (ORDER BY Reputation DESC, PostCount DESC) AS Rank FROM UserStatistics)
// SELECT tu.Rank, tu.DisplayName, tu.Reputation, tu.PostCount, tu.VoteCount, tu.UpVoteCount, tu.DownVoteCount, tu.BadgeCount, tu.LastPostDate
// FROM TopUsers tu WHERE tu.Rank <= 10 ORDER BY tu.Rank;
//
// Rank reads Reputation and the distinct post count, so the ten users are picked on those first and the posts x votes x badges product is driven for those alone.
fn q8448(db: &'static So) -> String {
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let tu = top_n(drain((&db.user.reputation).and(&pc)), |&(u, (r, n))| (Reverse(r), Reverse(n), u), 10);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let Vote { vote_type_id, .. } = &db.vote;
    let prod = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.creation_date).and(votes_of(db).select(vote_type_id).opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64, 0, 0, i64::MIN], |a, (p, c)| {
            let t = p.and_then(|(_, t)| t);
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.unwrap_or(0), a[3].max(p.map_or(i64::MIN, |(d, _)| d))]
        });
    let vc = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(votes_of(db)).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let v = top_n(drain((&prod).and(&pc).and(&vc)), |&(u, ((_, n), _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(n), u), 0);
    rows(v.into_iter().enumerate().map(|(i, (u, ((a, n), c)))| {
        let mut f = vec![V::I(i as i64 + 1)];
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::I(n), V::I(c), V::I(a[0]), V::I(a[1]), V::I(a[2]), tmax(a[3])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Tags, p.CreationDate, u.DisplayName AS OwnerDisplayName, COUNT(a.Id) AS AnswerCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
//        ROW_NUMBER() OVER (PARTITION BY array_to_string(string_to_array(p.Tags, '><'), ',') ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Posts a ON a.ParentId = p.Id LEFT JOIN Votes v ON v.PostId = p.Id
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= (cast('2024-10-01' as date) - INTERVAL '30 days') GROUP BY p.Id, p.Title, p.Tags, p.CreationDate, u.DisplayName),
// FilteredPosts AS (SELECT *, CONCAT('Answers: ', AnswerCount, ', Votes: ', UpVotes - DownVotes) AS Summary FROM RankedPosts WHERE Rank <= 5)
// SELECT fp.PostId, fp.Title, fp.Tags, fp.OwnerDisplayName, fp.CreationDate, fp.Summary FROM FilteredPosts fp ORDER BY fp.Tags, fp.CreationDate DESC;
//
// Rank reads only Tags and CreationDate, so the posts are picked first and the answer x vote product is driven for those alone.
fn q29445(db: &'static So) -> String {
    let Post { post_type_id, creation_date, tags_str, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_days(date(2024, 10, 1), -30)))).select(tags_str.opt()));
    let top = top_per(v, |&(_, t)| t.map(|t| t.split("><").collect::<Vec<_>>().join(",")), |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pv = (&tp)
        .group_by(Ident::<Post>::new())
        .select(children_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain(&pv).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "tags", "owner", "created"]);
        f.push(V::Owned(format!("Answers: {}, Votes: {}", a[0], a[1] - a[2])));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS TotalPosts, COUNT(DISTINCT C.Id) AS TotalComments,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsAsked, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersGiven,
//        SUM(COALESCE(V.BountyAmount, 0)) AS TotalBountyReceived
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON U.Id = V.UserId AND V.VoteTypeId = 8
//     WHERE U.Reputation > 1000 GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, TotalComments, QuestionsAsked, AnswersGiven, TotalBountyReceived, RANK() OVER (ORDER BY Reputation DESC) AS UserRank FROM UserActivity)
// SELECT TU.DisplayName, TU.Reputation, TU.TotalPosts, TU.TotalComments, TU.QuestionsAsked, TU.AnswersGiven, TU.TotalBountyReceived FROM TopUsers TU WHERE TU.UserRank <= 10 ORDER BY TU.UserRank;
//
// UserRank reads only Reputation, so the users are picked first and the posts x comments x bounty votes product is driven for those alone.
fn q7604(db: &'static So) -> String {
    let tu = ranked(drain(db.user.with((&db.user.reputation).gt(1000)).select(&db.user.reputation)), |&(_, r)| Reverse(r), false);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let bounty = votes_by(db).select(Ident::<Vote>::new().with(vote_type_id.eq(8))).select(bounty_amount.opt());
    let prod = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(comments_of(db).opt())).opt().and(bounty.opt()))
        .fold([0i64; 3], |a, (p, b)| {
            let t = p.map(|(t, _)| t);
            [a[0] + (t == Some(1)) as i64, a[1] + (t == Some(2)) as i64, a[2] + b.flatten().unwrap_or(0)]
        });
    let pc = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let cc = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    rows(drain((&prod).and(&pc).and(&cc)).into_iter().map(|(u, ((a, p), c))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(p), V::I(c), V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY pt.Name ORDER BY p.Score DESC) AS PostRank
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'),
// UserEngagement AS (SELECT u.Id AS UserId, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT v.Id) AS VoteCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
//     FROM Users u LEFT JOIN Comments c ON u.Id = c.UserId LEFT JOIN Votes v ON u.Id = v.UserId WHERE u.Reputation >= 100 GROUP BY u.Id)
// SELECT up.UserId, up.CommentCount, up.VoteCount, up.UpVoteCount, up.DownVoteCount, rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount
// FROM UserEngagement up JOIN RankedPosts rp ON up.UserId = rp.PostId WHERE up.CommentCount > 0 OR up.VoteCount > 0 ORDER BY up.UserId, rp.Score DESC LIMIT 100;
//
// `up.UserId = rp.PostId` joins a user id to a post id, so it goes through the raw ids. PostRank is never read.
fn q2831(db: &'static So) -> String {
    let recent: HashIdx<i64, Id<Post>> = db.post.with((&db.post.creation_date).ge(add_years(date(2024, 10, 1), -1))).with(&db.post.post_type).select(&db.post.origid).inv().collect();
    let users: MatSet<Id<User>> = db.user.with((&db.user.reputation).ge(100)).with((&db.user.origid).select(&recent)).collect();
    let ue = (&users)
        .group_by(Ident::<User>::new())
        .select(comments_by(db).opt().and(votes_by(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = (&users).group_by(Ident::<User>::new()).select(comments_by(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = (&users).group_by(Ident::<User>::new()).select(votes_by(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let v = drain((&ue).and(&cc).and(&vc).filt(|((_, c), v)| c > 0 || v > 0).and((&db.user.origid).select(&recent)));
    let v = top_n(v, |&(u, (_, p))| (db.user.origid.get(u).unwrap(), Reverse(db.post.score.get(p).unwrap()), p), 100);
    rows(v.into_iter().map(|(u, (((a, c), n), p))| {
        let mut f = vec![user_col(db, u, "uid"), V::I(c), V::I(n), V::I(a[0]), V::I(a[1])];
        f.extend(post_fields(db, p, &["id", "title", "created", "score", "views"]));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN P.PostTypeId IN (1, 2) THEN P.Score ELSE 0 END) AS TotalScore
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, TotalQuestions, TotalAnswers, TotalScore, ROW_NUMBER() OVER (ORDER BY Reputation DESC, TotalScore DESC) AS Rank FROM UserReputation)
// SELECT TU.DisplayName, TU.Reputation, TU.TotalPosts, TU.TotalQuestions, TU.TotalAnswers, TU.TotalScore, (SELECT AVG(Reputation) FROM Users) AS AverageReputation,
//        (SELECT COUNT(*) FROM Posts WHERE CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days')) AS RecentPostCount
// FROM TopUsers TU WHERE TU.Rank <= 10;
fn q8068(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, .. } = &db.post;
    let ur = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(score)).opt()).fold([0i64; 4], |a, p| match p {
        Some((t, s)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + if t == 1 || t == 2 { s } else { 0 }],
        None => a,
    });
    let reps = db.user.select(&db.user.reputation).fold_flat([0i64; 2], |a, r| [a[0] + r, a[1] + 1]);
    let recent = count(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))));
    let v = top_n(drain(&ur), |&(u, a)| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(a[3]), u), 10);
    rows(v.into_iter().map(|(u, a)| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(a.map(V::I));
        f.extend([avg(reps[0], reps[1]), V::I(recent)]);
        row(f)
    }))
}

// WITH UserBadges AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Badges b GROUP BY b.UserId),
// RecentPosts AS (SELECT p.Id, p.OwnerUserId, p.Title, p.Score, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days')
// SELECT u.Id AS UserId, u.DisplayName, COALESCE(ub.BadgeCount, 0) AS TotalBadges, COALESCE(ub.GoldBadges, 0) AS GoldBadges, COALESCE(ub.SilverBadges, 0) AS SilverBadges,
//        COALESCE(ub.BronzeBadges, 0) AS BronzeBadges, rp.Title AS RecentPostTitle, rp.Score AS RecentPostScore, rp.CreationDate AS RecentPostDate
// FROM Users u LEFT JOIN UserBadges ub ON u.Id = ub.UserId LEFT JOIN RecentPosts rp ON u.Id = rp.OwnerUserId AND rp.rn = 1 WHERE u.Reputation >= 1000 ORDER BY u.Reputation DESC LIMIT 10;
fn q1307(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let Post { owner_user, creation_date, .. } = &db.post;
    let recent = drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(owner_user));
    let top = top_per(recent, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let pp = rel(top.into_iter().map(|(p, u)| (u, p)).collect());
    let by_user: HashIdx<Id<User>, (Id<User>, Id<Post>)> = (&pp).map(|(u, _)| u).inv().select(&pp).collect();
    let v = drain(db.user.with((&db.user.reputation).ge(1000)).select((&ub).and((&by_user).map(|(_, p)| p).opt())));
    let v = top_n(v, |&(u, _)| (Reverse(db.user.reputation.get(u).unwrap()), u), 10);
    rows(v.into_iter().map(|(u, (b, p))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(b.map(V::I));
        f.extend(match p {
            Some(p) => post_fields(db, p, &["title", "score", "created"]),
            None => vec![V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, AVG(COALESCE(p.Score, 0)) AS AveragePostScore, SUM(COALESCE(c.Score, 0)) AS TotalCommentScore, SUM(b.Class) AS TotalBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Badges b ON u.Id = b.UserId
//     WHERE u.Reputation > 50 GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, AveragePostScore, TotalCommentScore, TotalBadges, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserActivity)
// SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, AveragePostScore, TotalCommentScore, TotalBadges FROM TopUsers WHERE Rank <= 10;
//
// Rank reads only Reputation, so the ten users are picked first and the posts x comments x badges product is driven for those alone.
fn q6774(db: &'static So) -> String {
    let tu = top_n(drain(db.user.with((&db.user.reputation).gt(50)).select(&db.user.reputation)), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let prod = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(&db.post.score).and(comments_of(db).select(&db.comment.score).opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 7], |a, (p, b)| {
            let (t, s, c) = p.map_or((0, 0, 0), |((t, s), c)| (t, s, c.unwrap_or(0)));
            [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + s, a[3] + 1, a[4] + c, a[5] + b.is_some() as i64, a[6] + b.unwrap_or(0)]
        });
    let pc = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    rows(drain((&prod).and(&pc)).into_iter().map(|(u, (a, n))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), avg(a[2], a[3]), V::I(a[4]), nullable(a[6], a[5])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.ViewCount DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 YEAR' AND p.PostTypeId = 1),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 8 GROUP BY u.Id, u.Reputation)
// SELECT u.DisplayName, ur.Reputation, ur.PostCount, ur.TotalBounty, rp.Title, rp.ViewCount, COALESCE(COUNT(c.Id), 0) AS CommentCount
// FROM UserReputation ur JOIN Users u ON ur.UserId = u.Id LEFT JOIN RankedPosts rp ON u.Id = rp.OwnerUserId AND rp.Rank = 1 LEFT JOIN Comments c ON rp.Id = c.PostId
// GROUP BY u.DisplayName, ur.Reputation, ur.PostCount, ur.TotalBounty, rp.Title, rp.ViewCount ORDER BY ur.Reputation DESC, ur.TotalBounty DESC LIMIT 10;
fn q4076(db: &'static So) -> String {
    let Post { owner_user, creation_date, post_type_id, view_count, title, .. } = &db.post;
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let bounty = votes_of(db).select(Ident::<Vote>::new().with(vote_type_id.eq(8))).select(bounty_amount.opt());
    let ub = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(bounty.opt()).opt()).fold([0i64; 2], |a, b| [a[0] + b.flatten().flatten().is_some() as i64, a[1] + b.flatten().flatten().unwrap_or(0)]);
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let rp = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.eq(1))).select(owner_user));
    let top = top_per(rp, |&(_, u)| u, |&(p, _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), p)
    }, 1, false);
    let pp = rel(top.into_iter().map(|(p, u)| (u, p)).collect());
    let by_user: HashIdx<Id<User>, (Id<User>, Id<Post>)> = (&pp).map(|(u, _)| u).inv().select(&pp).collect();
    let rpu = (&by_user).map(|(_, p)| p);
    let key = (&db.user.display_name)
        .and(&db.user.reputation)
        .and(&pc)
        .and((&ub).map(|a| a[1]))
        .and((&rpu).map(|p| (title.get(p), view_count.get(p))).opt().map(|o: Option<(Option<Str>, Option<i64>)>| o.unwrap_or((None, None))));
    let g = db.user.group_by(key).select((&rpu).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = top_n(drain(&g), |&(((((n, r), _), b), _), _)| (Reverse(r), Reverse(b), n), 10);
    rows(v.into_iter().map(|(((((n, r), pc), b), (t, w)), c)| row(vec![V::S(n), V::I(r), V::I(pc), V::I(b), ostr(t), oint(w), V::I(c)])))
}

// WITH UserStats AS (SELECT Users.Id AS UserId, COUNT(DISTINCT Posts.Id) AS PostCount, SUM(CASE WHEN Posts.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN Posts.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(UpVotes) AS TotalUpVotes, SUM(DownVotes) AS TotalDownVotes
//     FROM Users LEFT JOIN Posts ON Users.Id = Posts.OwnerUserId GROUP BY Users.Id),
// PostPerformance AS (SELECT Posts.Id, Posts.Title, Posts.CreationDate, Posts.Score, Posts.ViewCount, COALESCE(UserStats.PostCount, 0) AS UserPostCount,
//        COALESCE(UserStats.QuestionCount, 0) AS UserQuestionCount, COALESCE(UserStats.AnswerCount, 0) AS UserAnswerCount FROM Posts LEFT JOIN UserStats ON Posts.OwnerUserId = UserStats.UserId),
// TopPosts AS (SELECT Id, Title, CreationDate, Score, ViewCount, RANK() OVER (ORDER BY Score DESC, ViewCount DESC) AS Rank FROM PostPerformance)
// SELECT Id, Title, CreationDate, Score, ViewCount, Rank FROM TopPosts WHERE Rank <= 100 ORDER BY Rank;
//
// UserStats has one row per user, so the LEFT JOIN keeps each post once, and none of its columns is read.
fn q14468(db: &'static So) -> String {
    let Post { score, view_count, .. } = &db.post;
    let v = ranked(drain(&db.post.id), |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    }, false);
    rows(v.into_iter().take_while(|x| x.1 <= 100).map(|((p, _), r)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.push(V::I(r));
        row(f)
    }))
}

// WITH TagStatistics AS (SELECT t.TagName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersCount, SUM(CASE WHEN p.PostTypeId IN (3, 4, 5) THEN 1 ELSE 0 END) AS WikiCount, COUNT(c.Id) AS CommentsCount,
//        COALESCE(AVG(u.Reputation), 0) AS AverageUserReputation, MAX(p.CreationDate) AS MostRecentPostDate
//     FROM Tags t LEFT JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' LEFT JOIN Comments c ON c.PostId = p.Id LEFT JOIN Users u ON p.OwnerUserId = u.Id GROUP BY t.TagName),
// TopTags AS (SELECT TagName, PostCount, QuestionsCount, AnswersCount, WikiCount, CommentsCount, AverageUserReputation, MostRecentPostDate, ROW_NUMBER() OVER (ORDER BY PostCount DESC) AS Rank FROM TagStatistics)
// SELECT TagName, PostCount, QuestionsCount, AnswersCount, WikiCount, CommentsCount, AverageUserReputation, MostRecentPostDate FROM TopTags WHERE Rank <= 10 ORDER BY AverageUserReputation DESC;
//
// Rank reads only the distinct post count, so the ten tags are picked on that first and the posts x comments product is driven for those alone.
fn q27392(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let lt = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&lt).map(|(_, t)| t).inv().collect();
    let tposts = (&by_tag).map(|(p, _)| p);
    let by_name = db.tag.group_by(&db.tag.tag_name).select((&tposts).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let top = top_n(drain(&by_name), |&(n, c)| (Reverse(c), n), 10);
    let top = rel(top.into_iter().map(|x| x.0).collect());
    let tn: HashIdx<Str, Id<Tag>> = (&db.tag.tag_name).inv().collect();
    let tt: MatSet<Id<Tag>> = (&top).select(&tn).map(|t| t).collect();
    let s = (&tt)
        .group_by(&db.tag.tag_name)
        .select((&tposts).select(post_type_id.and(creation_date).and(comments_of(db).opt()).and(owner_user.select(&db.user.reputation).opt())).opt())
        .fold([0i64, 0, 0, 0, 0, 0, i64::MIN], |a, p| match p {
            Some((((t, d), c), r)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + matches!(t, 3 | 4 | 5) as i64, a[3] + c.is_some() as i64, a[4] + r.is_some() as i64, a[5] + r.unwrap_or(0), a[6].max(d)],
            None => a,
        });
    rows(drain((&s).and(&by_name)).into_iter().map(|(n, (a, pc))| {
        row(vec![V::S(n), V::I(pc), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), if a[4] == 0 { V::F(0.0) } else { avg(a[5], a[4]) }, tmax(a[6])])
    }))
}

// WITH UserEngagement AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, Questions, Answers, UpVotes, DownVotes, RANK() OVER (ORDER BY PostCount DESC, UpVotes - DownVotes DESC) AS EngagementRank FROM UserEngagement)
// SELECT TU.DisplayName, TU.PostCount, TU.Questions, TU.Answers, TU.UpVotes, TU.DownVotes,
//        CASE WHEN TU.PostCount > 50 THEN 'Highly Active' WHEN TU.PostCount BETWEEN 20 AND 50 THEN 'Moderately Active' ELSE 'Less Active' END AS ActivityLevel
// FROM TopUsers TU WHERE TU.EngagementRank <= 10 ORDER BY TU.PostCount DESC, TU.UpVotes - TU.DownVotes DESC;
fn q28561(db: &'static So) -> String {
    let ue = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, p| {
            let (t, v) = p.map_or((0, None), |(t, v)| (t, v));
            [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64]
        });
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let v = ranked(drain((&ue).and(&pc)), |&(_, (a, n))| (Reverse(n), Reverse(a[2] - a[3])), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, (a, n)), _)| {
        let mut f = vec![user_col(db, u, "name"), V::I(n)];
        f.extend(a.map(V::I));
        f.push(V::S(if n > 50 { "Highly Active" } else if n >= 20 { "Moderately Active" } else { "Less Active" }));
        row(f)
    }))
}

// WITH UserVoteSummary AS (SELECT U.Id AS UserId, U.DisplayName, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        COUNT(DISTINCT P.Id) AS PostCount, AVG(COALESCE(P.Score, 0)) AS AvgScore
//     FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId LEFT JOIN Posts P ON V.PostId = P.Id AND P.OwnerUserId = U.Id GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, UpVotes, DownVotes, PostCount, AvgScore, DENSE_RANK() OVER (ORDER BY UpVotes DESC) AS RankByUpVotes,
//        DENSE_RANK() OVER (ORDER BY DownVotes DESC) AS RankByDownVotes FROM UserVoteSummary)
// SELECT A.UserId, A.DisplayName, A.UpVotes, A.DownVotes, A.PostCount, A.AvgScore, B.RankByUpVotes, B.RankByDownVotes
// FROM UserVoteSummary A JOIN TopUsers B ON A.UserId = B.UserId
// WHERE (A.UpVotes > (SELECT AVG(UpVotes) FROM UserVoteSummary) OR A.DownVotes < (SELECT AVG(DownVotes) / 2 FROM UserVoteSummary))
// ORDER BY A.UpVotes DESC, A.DownVotes ASC FETCH FIRST 10 ROWS ONLY;
//
// The comparisons with the averages are done exactly, as x * n > sum.
fn q4474(db: &'static So) -> String {
    let Vote { user, post, vote_type_id, .. } = &db.vote;
    let own: HashIdx<Id<Vote>, Id<Post>> = db.vote.with(user.and(post.select(&db.post.owner_user)).filt(|(a, b)| a == b)).select(post).collect();
    let uvs = db
        .user
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(vote_type_id.and((&own).select(&db.post.score).opt())).opt())
        .fold([0i64; 4], |a, v| {
            let (t, s) = v.map_or((0, None), |(t, s)| (t, s));
            [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + 1, a[3] + s.unwrap_or(0)]
        });
    let pc = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&own).opt()).buf_fold(distinct_some);
    let tot = (&uvs).fold_flat([0i64; 3], |t, a| [t[0] + a[0], t[1] + a[1], t[2] + 1]);
    let v = ranked(drain((&uvs).and(&pc)), |&(_, (a, _))| Reverse(a[0]), true);
    let v = ranked(v, |&((_, (a, _)), _)| Reverse(a[1]), true);
    let keep: Vec<_> = v.into_iter().map(|(((u, (a, n)), ru), rd)| (u, a, n, ru, rd)).collect();
    let keep = rel(keep);
    type R = (Id<User>, [i64; 4], i64, i64, i64);
    let v = drain((&keep).filt(|(_, a, _, _, _): R| a[0] * tot[2] > tot[0] || 2 * a[1] * tot[2] < tot[1]));
    let v = top_n(v, |&(_, (u, a, _, _, _))| (Reverse(a[0]), a[1], u), 10);
    rows(v.into_iter().map(|(_, (u, a, n, ru, rd))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(n), avg(a[3], a[2]), V::I(ru), V::I(rd)]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(CASE WHEN P.PostTypeId IN (4, 5) THEN 1 ELSE 0 END) AS TagWikis, SUM(COALESCE(P.Score, 0)) AS TotalScore,
//        SUM(COALESCE(P.ViewCount, 0)) AS TotalViews, SUM(COALESCE(B.Class, 0)) AS TotalBadges
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Badges B ON U.Id = B.UserId WHERE U.CreationDate >= '2020-01-01' GROUP BY U.Id, U.DisplayName),
// ActiveUsers AS (SELECT UserId, DisplayName, TotalPosts, Questions, Answers, TagWikis, TotalScore, TotalViews, TotalBadges, RANK() OVER (ORDER BY TotalPosts DESC, TotalScore DESC) AS Rank FROM UserActivity)
// SELECT AU.DisplayName, AU.TotalPosts, AU.Questions, AU.Answers, AU.TagWikis, AU.TotalScore, AU.TotalViews, AU.TotalBadges FROM ActiveUsers AU WHERE AU.Rank <= 10 ORDER BY AU.Rank;
fn q5003(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let users = || db.user.with((&db.user.creation_date).ge(ts(2020, 1, 1, 0, 0, 0)));
    let ua = users()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(view_count.opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 6], |a, (p, b)| {
            let (t, s, w) = p.map_or((0, 0, 0), |((t, s), w)| (t, s, w.unwrap_or(0)));
            [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + matches!(t, 4 | 5) as i64, a[3] + s, a[4] + w, a[5] + b.unwrap_or(0)]
        });
    let pc = users().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let v = ranked(drain((&ua).and(&pc)), |&(_, (a, n))| (Reverse(n), Reverse(a[3])), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, (a, n)), _)| {
        let mut f = vec![user_col(db, u, "name"), V::I(n)];
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        COUNT(DISTINCT c.Id) AS CommentCount, COALESCE(SUM(b.Class), 0) AS TotalBadges, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, QuestionCount, AnswerCount, CommentCount, TotalBadges, UpVotes, DownVotes, RANK() OVER (ORDER BY QuestionCount DESC, TotalBadges DESC) AS UserRank FROM UserStatistics)
// SELECT UserRank, DisplayName, QuestionCount, AnswerCount, CommentCount, TotalBadges, UpVotes, DownVotes FROM TopUsers WHERE UserRank <= 10 ORDER BY UserRank;
//
// UserRank leads with the distinct post count, so only users with at least the tenth-highest count can rank in the top ten; the product is driven for those alone,
// and every other user ranks below all of them.
fn q7964(db: &'static So) -> String {
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let tenth = top_n(drain(&pc), |&(u, n)| (Reverse(n), u), 10).last().unwrap().1;
    let cand: MatSet<Id<User>> = db.user.with((&pc).filt(|n| n >= tenth)).collect();
    let prod = (&cand)
        .group_by(Ident::<User>::new())
        .select(
            posts_of(db)
                .select((&db.post.post_type_id).and(comments_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()))
                .opt()
                .and(badges_of(db).select(&db.badge.class).opt()),
        )
        .fold([0i64; 4], |a, (p, b)| {
            let (t, v) = p.map_or((0, None), |((t, _), v)| (t, v));
            [a[0] + (t == 2) as i64, a[1] + b.unwrap_or(0), a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64]
        });
    let cc = (&cand).group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = ranked(drain((&prod).and(&pc).and(&cc)), |&(_, ((a, n), _))| (Reverse(n), Reverse(a[1])), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, ((a, n), c)), r)| {
        row(vec![V::I(r), user_col(db, u, "name"), V::I(n), V::I(a[0]), V::I(c), V::I(a[1]), V::I(a[2]), V::I(a[3])])
    }))
}

// WITH UserScores AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COUNT(DISTINCT P.Id) AS PostsCount,
//        COALESCE(SUM(CASE WHEN P.PostTypeId = 1 THEN P.AnswerCount ELSE 0 END), 0) AS QuestionsAnswered, COALESCE(SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS AnswersCount
//     FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// RankedUsers AS (SELECT UserId, DisplayName, Reputation, UpVotes, DownVotes, PostsCount, QuestionsAnswered, AnswersCount, RANK() OVER (ORDER BY Reputation DESC, UpVotes DESC) AS Rank FROM UserScores)
// SELECT R.DisplayName, R.Reputation, R.UpVotes, R.DownVotes, R.PostsCount, R.QuestionsAnswered, R.AnswersCount, R.Rank, (SELECT COUNT(*) FROM Users) AS TotalUsers
// FROM RankedUsers R WHERE R.Rank <= 10 ORDER BY R.Rank;
//
// Rank leads with Reputation, so only users with at least the tenth-highest reputation can rank in the top ten; the votes x posts product is driven for those alone.
fn q5535(db: &'static So) -> String {
    let tenth = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10).last().unwrap().1;
    let cand: MatSet<Id<User>> = db.user.with((&db.user.reputation).ge(tenth)).collect();
    let Post { post_type_id, answer_count, .. } = &db.post;
    let us = (&cand)
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt().and(posts_of(db).select(post_type_id.and(answer_count.opt())).opt()))
        .fold([0i64; 4], |a, (v, p)| {
            let (t, n) = p.map_or((0, None), |x| x);
            [a[0] + (v == Some(2)) as i64, a[1] + (v == Some(3)) as i64, a[2] + if t == 1 { n.unwrap_or(0) } else { 0 }, a[3] + (t == 2) as i64]
        });
    let pc = (&cand).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let total = count(&db.user.id);
    let v = ranked(drain((&us).and(&pc)), |&(u, (a, _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(a[0])), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, (a, n)), r)| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(n), V::I(a[2]), V::I(a[3]), V::I(r), V::I(total)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank, p.OwnerUserId
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) FILTER (WHERE b.Class = 1) AS GoldBadges, COUNT(b.Id) FILTER (WHERE b.Class = 2) AS SilverBadges,
//        COUNT(b.Id) FILTER (WHERE b.Class = 3) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id)
// SELECT u.DisplayName, COALESCE(rb.PostRank, 0) AS Rank, COALESCE(rb.Title, 'No Posts') AS TopPostTitle, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges,
//        CASE WHEN ub.GoldBadges > 0 THEN 'Gold Star' WHEN ub.SilverBadges > 0 THEN 'Silver Star' ELSE 'No Medal' END AS BadgeStatus
// FROM Users u LEFT JOIN UserBadges ub ON u.Id = ub.UserId LEFT JOIN RankedPosts rb ON u.Id = rb.OwnerUserId AND rb.PostRank = 1
// WHERE u.Reputation > 1000 ORDER BY u.Reputation DESC, Rank ASC LIMIT 20;
fn q670(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    let Post { owner_user, creation_date, score, title, .. } = &db.post;
    let recent = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user));
    let top = top_per(recent, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 1, false);
    let pp = rel(top.into_iter().map(|(p, u)| (u, p)).collect());
    let by_user: HashIdx<Id<User>, (Id<User>, Id<Post>)> = (&pp).map(|(u, _)| u).inv().select(&pp).collect();
    let v = drain(db.user.with((&db.user.reputation).gt(1000)).select((&ub).and((&by_user).map(|(_, p)| p).opt())));
    let v = top_n(v, |&(u, (_, p))| (Reverse(db.user.reputation.get(u).unwrap()), p.is_some() as i64, u), 20);
    rows(v.into_iter().map(|(u, (b, p))| {
        let mut f = vec![user_col(db, u, "name"), V::I(p.is_some() as i64), V::S(p.and_then(|p| title.get(p)).unwrap_or("No Posts"))];
        f.extend(b.map(V::I));
        f.push(V::S(if b[0] > 0 { "Gold Star" } else if b[1] > 0 { "Silver Star" } else { "No Medal" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.UserId) AS VoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 YEAR' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.CommentCount, rp.VoteCount, RANK() OVER (ORDER BY rp.Score DESC, rp.ViewCount DESC) AS PostRank FROM RankedPosts rp)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.CommentCount, tp.VoteCount, COALESCE(ps.Name, 'N/A') AS PostType
// FROM TopPosts tp LEFT JOIN PostTypes ps ON (SELECT p.PostTypeId FROM Posts p WHERE p.Id = tp.PostId) = ps.Id WHERE tp.PostRank <= 10 ORDER BY tp.Score DESC, tp.ViewCount DESC;
//
// PostRank reads only Score and ViewCount, so the posts are picked first and the comment x vote product is driven for those alone.
fn q9191(db: &'static So) -> String {
    let Post { creation_date, score, view_count, .. } = &db.post;
    let v = ranked(drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(score)), |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    }, false);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select((&db.vote.user).opt()).opt())).buf_fold(|x| distinct_some(x.iter().map(|&(_, u)| u.flatten())));
    rows(drain((&cc).and(&vc).and(ptype_name(db).opt())).into_iter().map(|(p, ((c, n), t))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(c), V::I(n), V::S(t.unwrap_or("N/A"))]);
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.CreationDate, COUNT(cm.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount, ROW_NUMBER() OVER (ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN Comments cm ON p.Id = cm.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.ViewCount, p.CreationDate),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.ViewCount, rp.CommentCount, rp.UpVoteCount, rp.DownVoteCount,
//        CASE WHEN rp.UpVoteCount > COALESCE(rp.DownVoteCount, 0) THEN 'Positive' ELSE 'Negative' END AS Sentiment FROM RecentPosts rp WHERE rp.rn <= 10)
// SELECT tp.Title, tp.ViewCount, tp.CommentCount, tp.UpVoteCount, tp.DownVoteCount, tp.Sentiment, CASE WHEN tp.CommentCount > 0 THEN 'Has Comments' ELSE 'No Comments' END AS Comment_Status
// FROM TopPosts tp ORDER BY tp.ViewCount DESC;
//
// rn reads only CreationDate, so the ten newest posts are picked first and the comment x vote product is driven for those alone.
fn q3053(db: &'static So) -> String {
    let Post { creation_date, .. } = &db.post;
    let v = top_n(drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(creation_date)), |&(p, d)| (Reverse(d), p), 10);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain(&s).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["title", "views"]);
        f.extend(a.map(V::I));
        f.push(V::S(if a[1] > a[2] { "Positive" } else { "Negative" }));
        f.push(V::S(if a[0] > 0 { "Has Comments" } else { "No Comments" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank,
//        COUNT(c.Id) AS CommentTotal, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpvoteTotal, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownvoteTotal
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '6 months'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.PostTypeId),
// TopPostTypes AS (SELECT PostTypeId, COUNT(*) AS PostCount FROM Posts GROUP BY PostTypeId HAVING COUNT(*) > 5)
// SELECT pt.Name AS PostType, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.CommentTotal, rp.UpvoteTotal, rp.DownvoteTotal
// FROM RankedPosts rp JOIN PostTypes pt ON rp.PostId = pt.Id JOIN TopPostTypes tpt ON pt.Id = tpt.PostTypeId WHERE rp.Rank <= 5 ORDER BY pt.Name, rp.Rank;
//
// `rp.PostId = pt.Id` joins a post id to a post type id, so it goes through the raw ids. Rank reads only base columns, so the posts are picked first.
fn q23902(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, origid, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_months(current_date(), -6))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let top = rel(top.into_iter().map(|(p, _)| p).collect());
    let tp: MatSet<Id<Post>> = (&top).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let tpt = db.post.group_by(post_type_id).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let ptidx: HashIdx<i64, Id<PostType>> = db.post_type.with((&db.post_type.origid).select((&tpt).filt(|n| n > 5))).select(&db.post_type.origid).inv().collect();
    let rank = |p: Id<Post>| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p);
    let mut v = drain((&s).and(origid.select(&ptidx)));
    v.sort_by_key(|&(p, (_, t))| (db.post_type.name.get(t).unwrap(), rank(p)));
    rows(v.into_iter().map(|(p, (a, t))| {
        let mut f = vec![V::S(db.post_type.name.get(t).unwrap())];
        f.extend(post_fields(db, p, &["title", "created", "score", "views"]));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserVoteSummary AS (SELECT u.Id AS UserId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        COUNT(DISTINCT p.Id) AS TotalPosts, AVG(COALESCE(NULLIF(p.Score, 0), NULL)) AS AvgScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id),
// RankedUsers AS (SELECT UserId, UpVotes, DownVotes, TotalPosts, AvgScore, RANK() OVER (ORDER BY AvgScore DESC, UpVotes DESC) AS ScoreRank FROM UserVoteSummary),
// TopBadges AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount FROM Badges b WHERE b.Class = 1 GROUP BY b.UserId)
// SELECT u.DisplayName, COALESCE(b.BadgeCount, 0) AS GoldBadgeCount, r.UpVotes, r.DownVotes, r.TotalPosts, r.AvgScore, r.ScoreRank
// FROM Users u LEFT JOIN RankedUsers r ON u.Id = r.UserId LEFT JOIN TopBadges b ON u.Id = b.UserId
// WHERE (r.ScoreRank <= 10 OR r.UpVotes >= 100) AND u.Reputation > 500 ORDER BY r.ScoreRank, u.DisplayName;
fn q2931(db: &'static So) -> String {
    let uv = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.score).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some((s, t)) => [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + (s != 0) as i64, a[3] + s],
            None => a,
        });
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let mean = |a: [i64; 4]| if a[2] == 0 { None } else { Some(a[3] as f64 / a[2] as f64) };
    let v = ranked(drain((&uv).and(&pc)), |&(_, (a, _))| (mean(a).is_none(), Reverse(mean(a).map(fkey)), Reverse(a[0])), false);
    type R = (Id<User>, [i64; 4], i64, i64);
    let r = rel(v.into_iter().map(|((u, (a, n)), r)| (u, a, n, r)).collect::<Vec<R>>());
    let by_user: HashIdx<Id<User>, R> = (&r).map(|(u, _, _, _)| u).inv().select(&r).collect();
    let gold = db.badge.with((&db.badge.class).eq(1)).group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let v = drain(db.user.with((&db.user.reputation).gt(500)).select((&by_user).filt(|(_, a, _, r): R| r <= 10 || a[0] >= 100).and((&gold).opt())));
    rows(v.into_iter().map(|(u, ((_, a, n, r), g))| {
        row(vec![user_col(db, u, "name"), V::I(g.unwrap_or(0)), V::I(a[0]), V::I(a[1]), V::I(n), ofloat(mean(a)), V::I(r)])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '6 months'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.PostTypeId),
// TopPosts AS (SELECT rp.* FROM RankedPosts rp WHERE rp.Rank <= 10)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.CommentCount, tp.UpVotes, tp.DownVotes, COALESCE(AVG(u.Reputation), 0) AS AverageReputation
// FROM TopPosts tp LEFT JOIN Users u ON tp.PostId = u.Id GROUP BY tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.CommentCount, tp.UpVotes, tp.DownVotes
// ORDER BY tp.Score DESC, tp.ViewCount DESC;
//
// `tp.PostId = u.Id` joins a post id to a user id, so it goes through the raw ids. Rank reads only base columns, so the posts are picked first.
fn q7960(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -6))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    rows(drain((&s).and((&db.post.origid).select(&uidx).select(&db.user.reputation).opt())).into_iter().map(|(p, (a, r))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend(a.map(V::I));
        f.push(V::F(r.map_or(0.0, |r| r as f64)));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN P.PostTypeId = 2 AND P.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswerCount,
//        SUM(CASE WHEN C.Id IS NOT NULL THEN 1 ELSE 0 END) AS CommentCount, SUM(V.BountyAmount) AS TotalBountyAmount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.Reputation),
// TopUsers AS (SELECT UserId, Reputation, PostCount, QuestionCount, AnswerCount, AcceptedAnswerCount, CommentCount, TotalBountyAmount, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserStats)
// SELECT UserId, Reputation, PostCount, QuestionCount, AnswerCount, AcceptedAnswerCount, CommentCount, TotalBountyAmount, ReputationRank FROM TopUsers WHERE ReputationRank <= 10;
//
// ReputationRank reads only Reputation, so the users are picked first and the posts x comments x votes product is driven for those alone.
fn q12898(db: &'static So) -> String {
    let tu = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let tu = rel(tu.into_iter().take_while(|x| x.1 <= 10).map(|((u, _), r)| (u, r)).collect());
    let rank: HashIdx<Id<User>, (Id<User>, i64)> = (&tu).map(|(u, _)| u).inv().select(&tu).collect();
    let cand: MatSet<Id<User>> = (&tu).map(|(u, _)| u).collect();
    let Post { post_type_id, accepted_answer_id, .. } = &db.post;
    let us = (&cand)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(accepted_answer_id.opt()).and(comments_of(db).opt()).and(votes_of(db).select((&db.vote.bounty_amount).opt()).opt())).opt())
        .fold([0i64; 6], |a, p| match p {
            Some((((t, acc), c), b)) => {
                let b = b.flatten();
                [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (t == 2 && acc.is_some()) as i64, a[3] + c.is_some() as i64, a[4] + b.is_some() as i64, a[5] + b.unwrap_or(0)]
            }
            None => a,
        });
    let pc = (&cand).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    rows(drain((&us).and(&pc).and(&rank)).into_iter().map(|(u, ((a, n), (_, r)))| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[5], a[4]), V::I(r)]);
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id, p.Title, p.OwnerUserId, p.CreationDate, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.PostTypeId = 1),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, CASE WHEN u.Reputation >= 10000 THEN 'Expert' WHEN u.Reputation >= 1000 THEN 'Veteran' ELSE 'Newbie' END AS ReputationTier FROM Users u)
// SELECT rp.Title AS PostTitle, u.DisplayName AS Author, rp.CreationDate, rp.ViewCount, ur.Reputation, ur.ReputationTier, COALESCE(ph.NumberOfEdits, 0) AS EditCount
// FROM RecentPosts rp JOIN Users u ON rp.OwnerUserId = u.Id
// LEFT JOIN (SELECT ph.PostId, COUNT(ph.Id) AS NumberOfEdits FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (4, 5, 6) GROUP BY ph.PostId) ph ON rp.Id = ph.PostId
// JOIN UserReputation ur ON u.Id = ur.UserId WHERE rp.rn = 1 ORDER BY rp.ViewCount DESC LIMIT 10;
fn q2783(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.eq(1))).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let edits = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([4, 5, 6])));
    let ec = (&tp).group_by(Ident::<Post>::new()).select(edits.opt()).fold(0i64, |n, h| n + h.is_some() as i64);
    let v = top_n(drain((&ec).and(owner_user)), |&(p, _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), p)
    }, 10);
    rows(v.into_iter().map(|(p, (n, u))| {
        let r = db.user.reputation.get(u).unwrap();
        let mut f = post_fields(db, p, &["title", "owner", "created", "views"]);
        f.extend([V::I(r), V::S(if r >= 10000 { "Expert" } else if r >= 1000 { "Veteran" } else { "Newbie" }), V::I(n)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.Score, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS Rank,
//        COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.OwnerUserId, p.Score, p.CreationDate),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, AVG(rp.Score) AS AverageScore, COUNT(rp.PostId) AS TotalPosts FROM Users u JOIN RankedPosts rp ON u.Id = rp.OwnerUserId
//     WHERE rp.Rank <= 3 GROUP BY u.Id, u.DisplayName)
// SELECT tu.DisplayName, tu.AverageScore, tu.TotalPosts, COALESCE(b.Name, 'No Badge') AS BadgeName
// FROM TopUsers tu LEFT JOIN Badges b ON tu.UserId = b.UserId AND b.Class = 1 /* Gold Badge */ ORDER BY tu.AverageScore DESC, tu.TotalPosts DESC;
//
// Only Score and the owner of RankedPosts are read, so the comment x vote product is not needed.
fn q5988(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 3, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let tu = (&tp).group_by(owner_user).select(score).fold([0i64; 2], |a, s| [a[0] + s, a[1] + 1]);
    let gold = badges_of(db).select(Ident::<Badge>::new().with((&db.badge.class).eq(1)));
    rows(drain((&tu).and(gold.opt())).into_iter().map(|(u, (a, b))| {
        row(vec![user_col(db, u, "name"), avg(a[0], a[1]), V::I(a[1]), V::S(b.map_or("No Badge", |b| db.badge.name.get(b).unwrap()))])
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN p.PostTypeId = 1 AND p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, AcceptedCount, UpVotes, DownVotes, RANK() OVER (ORDER BY PostCount DESC) AS PostRank FROM UserStats)
// SELECT tu.DisplayName, tu.PostCount, tu.QuestionCount, tu.AnswerCount, tu.AcceptedCount, tu.UpVotes, tu.DownVotes,
//        CASE WHEN tu.UpVotes > tu.DownVotes THEN 'Net Positive' ELSE 'Net Negative' END AS VoteSentiment
// FROM TopUsers tu WHERE tu.PostRank <= 10 ORDER BY tu.PostCount DESC;
//
// PostRank reads only the distinct post count, so the users are picked on that first and the posts x votes product is driven for those alone.
fn q8733(db: &'static So) -> String {
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let tu = ranked(drain(&pc), |&(_, n)| Reverse(n), false);
    let cand: MatSet<Id<User>> = rel(tu.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let Post { post_type_id, accepted_answer_id, .. } = &db.post;
    let us = (&cand)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(accepted_answer_id.opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 5], |a, p| match p {
            Some(((t, acc), v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (t == 1 && acc.is_some()) as i64, a[3] + (v == Some(2)) as i64, a[4] + (v == Some(3)) as i64],
            None => a,
        });
    rows(drain((&us).and(&pc)).into_iter().map(|(u, (a, n))| {
        let mut f = vec![user_col(db, u, "name"), V::I(n)];
        f.extend(a.map(V::I));
        f.push(V::S(if a[3] > a[4] { "Net Positive" } else { "Net Negative" }));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN P.PostTypeId = 1 AND P.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswerCount,
//        SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END) AS UpvotedPostCount, SUM(CASE WHEN P.Score < 0 THEN 1 ELSE 0 END) AS DownvotedPostCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, AcceptedAnswerCount, UpvotedPostCount, DownvotedPostCount,
//        ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserStats)
// SELECT Rank, UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, AcceptedAnswerCount, UpvotedPostCount, DownvotedPostCount FROM TopUsers WHERE Rank <= 10 ORDER BY Rank;
fn q12309(db: &'static So) -> String {
    let Post { post_type_id, accepted_answer_id, score, .. } = &db.post;
    let us = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(accepted_answer_id.opt()).and(score)).opt()).fold([0i64; 6], |a, p| match p {
        Some(((t, acc), s)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (t == 1 && acc.is_some()) as i64, a[4] + (s > 0) as i64, a[5] + (s < 0) as i64],
        None => a,
    });
    let v = top_n(drain(&us), |&(u, _)| (Reverse(db.user.reputation.get(u).unwrap()), u), 10);
    rows(v.into_iter().enumerate().map(|(i, (u, a))| {
        let mut f = vec![V::I(i as i64 + 1)];
        f.extend(ucols(db, u, &["uid", "name", "rep"]));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount,
//        COALESCE((SELECT SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) FROM Votes v WHERE v.PostId = p.Id), 0) AS Upvotes,
//        COALESCE((SELECT SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) FROM Votes v WHERE v.PostId = p.Id), 0) AS Downvotes,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS RowNum FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount, MAX(c.CreationDate) AS LastCommentDate FROM Comments c GROUP BY c.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.Upvotes, rp.Downvotes, COALESCE(pc.CommentCount, 0) AS CommentCount, pc.LastCommentDate,
//        CASE WHEN rp.RowNum <= 10 THEN 'Top 10 Posts' ELSE 'Other Posts' END AS PostRank
// FROM RankedPosts rp LEFT JOIN PostComments pc ON rp.PostId = pc.PostId WHERE rp.RowNum <= 100 ORDER BY rp.Score DESC, rp.CreationDate DESC;
fn q9478(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let v = ranked(v, |&(p, t)| (t, Reverse(creation_date.get(p).unwrap()), p), false);
    let v = per_group(v, |&(_, t)| t);
    let rp = rel(v.into_iter().filter(|x| x.1 <= 100).map(|((p, _), r)| (p, r)).collect());
    let rn: HashIdx<Id<Post>, (Id<Post>, i64)> = (&rp).map(|(p, _)| p).inv().select(&rp).collect();
    let tp: MatSet<Id<Post>> = (&rp).map(|(p, _)| p).collect();
    let vs = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let pc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.creation_date).opt()).fold((0i64, i64::MIN), |(n, m), d| match d {
        Some(d) => (n + 1, m.max(d)),
        None => (n, m),
    });
    rows(drain((&vs).and(&pc).and(&rn)).into_iter().map(|(p, ((a, (n, m)), (_, r)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(n), tmax(m), V::S(if r <= 10 { "Top 10 Posts" } else { "Other Posts" })]);
        row(f)
    }))
}

// WITH PostAggregates AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT v.Id) AS VoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year' AND p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount),
// TopPosts AS (SELECT pa.PostId, pa.Title, pa.CreationDate, pa.Score, pa.ViewCount, pa.CommentCount, pa.VoteCount, pa.UpVotes, pa.DownVotes,
//        RANK() OVER (ORDER BY pa.Score DESC, pa.ViewCount DESC) AS Rank FROM PostAggregates pa)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.CommentCount, tp.VoteCount, tp.UpVotes, tp.DownVotes FROM TopPosts tp WHERE tp.Rank <= 10 ORDER BY tp.Rank;
//
// Rank reads only Score and ViewCount, so the posts are picked first and the comment x vote product is driven for those alone.
fn q9501(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, .. } = &db.post;
    let v = ranked(drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.eq(1))).select(score)), |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    }, false);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    rows(drain((&s).and(&cc).and(&vc)).into_iter().map(|(p, ((a, c), n))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(c), V::I(n), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN P.PostTypeId = 1 AND P.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers, SUM(CASE WHEN C.Id IS NOT NULL THEN 1 ELSE 0 END) AS CommentCount,
//        SUM(CASE WHEN B.Id IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount, SUM(V.BountyAmount) AS TotalBounty
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Badges B ON U.Id = B.UserId LEFT JOIN Votes V ON P.Id = V.PostId
//     GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, AnswerCount, AcceptedAnswers, CommentCount, BadgeCount, TotalBounty,
//        ROW_NUMBER() OVER (ORDER BY Reputation DESC, PostCount DESC) AS Rank FROM UserStats)
// SELECT Rank, DisplayName, Reputation, PostCount, AnswerCount, AcceptedAnswers, CommentCount, BadgeCount, TotalBounty FROM TopUsers WHERE Rank <= 10 ORDER BY Rank;
//
// Rank reads Reputation and the distinct post count, so the ten users are picked on those first and the product is driven for those alone.
fn q8842(db: &'static So) -> String {
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let tu = top_n(drain((&db.user.reputation).and(&pc)), |&(u, (r, n))| (Reverse(r), Reverse(n), u), 10);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let Post { post_type_id, accepted_answer_id, .. } = &db.post;
    let us = (&tu)
        .group_by(Ident::<User>::new())
        .select(
            posts_of(db)
                .select(post_type_id.and(accepted_answer_id.opt()).and(comments_of(db).opt()).and(votes_of(db).select((&db.vote.bounty_amount).opt()).opt()))
                .opt()
                .and(badges_of(db).opt()),
        )
        .fold([0i64; 6], |a, (p, b)| {
            let (t, acc, c, v) = p.map_or((0, false, false, None), |(((t, acc), c), v)| (t, acc.is_some(), c.is_some(), v.flatten()));
            [a[0] + (t == 2) as i64, a[1] + (t == 1 && acc) as i64, a[2] + c as i64, a[3] + b.is_some() as i64, a[4] + v.is_some() as i64, a[5] + v.unwrap_or(0)]
        });
    let v = top_n(drain((&us).and(&pc)), |&(u, (_, n))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(n), u), 0);
    rows(v.into_iter().enumerate().map(|(i, (u, (a, n)))| {
        let mut f = vec![V::I(i as i64 + 1)];
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[5], a[4])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.ViewCount, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn,
//        COUNT(c.Id) AS CommentTotal, COUNT(DISTINCT v.UserId) AS UniqueVoters
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (2, 3) GROUP BY p.Id, p.Title, p.ViewCount, p.CreationDate, p.OwnerUserId),
// FilteredPosts AS (SELECT rp.Id, rp.Title, rp.ViewCount, rp.CreationDate, rp.CommentTotal, rp.UniqueVoters FROM RankedPosts rp WHERE rp.rn = 1),
// TopPosts AS (SELECT fp.*, (fp.ViewCount * 1.0 / NULLIF(fp.CommentTotal, 0)) AS ViewToCommentRatio FROM FilteredPosts fp WHERE fp.CommentTotal > 5)
// SELECT t.Id, t.Title, t.ViewCount, t.CommentTotal, t.UniqueVoters, t.ViewToCommentRatio,
//        CASE WHEN t.ViewToCommentRatio > 10 THEN 'High Engagement' WHEN t.ViewToCommentRatio >= 5 THEN 'Moderate Engagement' ELSE 'Low Engagement' END AS EngagementLevel
// FROM TopPosts t WHERE t.UniqueVoters > 3 ORDER BY t.ViewToCommentRatio DESC LIMIT 50;
//
// rn reads only the owner and CreationDate, so each owner's newest post is picked first and the comment x vote product is driven for those alone.
fn q1655(db: &'static So) -> String {
    let Post { owner_user, creation_date, view_count, .. } = &db.post;
    let top = top_per(drain(db.post.select(owner_user.opt())), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let updown = || votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([2, 3])));
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(updown().opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let uv = (&tp).group_by(Ident::<Post>::new()).select(updown().select((&db.vote.user).opt()).opt()).buf_fold(|x| distinct_some(x.iter().map(|u| u.flatten())));
    let v = drain((&cc).filt(|n| n > 5).and((&uv).filt(|n| n > 3)));
    let ratio = |p: Id<Post>, c: i64| view_count.get(p).map(|w| w as f64 * 1.0 / c as f64);
    let v = top_n(v, |&(p, (c, _))| (ratio(p, c).is_none(), Reverse(ratio(p, c).map(fkey)), p), 50);
    rows(v.into_iter().map(|(p, (c, n))| {
        let r = ratio(p, c);
        let mut f = post_fields(db, p, &["id", "title", "views"]);
        f.extend([V::I(c), V::I(n), ofloat(r)]);
        f.push(V::S(match r {
            Some(r) if r > 10.0 => "High Engagement",
            Some(r) if r >= 5.0 => "Moderate Engagement",
            _ => "Low Engagement",
        }));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostsCreated, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersCreated, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsCreated
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId WHERE u.CreationDate >= '2022-01-01' GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostsCreated, TotalViews, AnswersCreated, QuestionsCreated, RANK() OVER (ORDER BY TotalViews DESC) AS ViewRank FROM UserActivity)
// SELECT tu.UserId, tu.DisplayName, tu.PostsCreated, tu.TotalViews, tu.AnswersCreated, tu.QuestionsCreated, b.Name AS BadgeName, COUNT(DISTINCT c.Id) AS CommentCount
// FROM TopUsers tu LEFT JOIN Badges b ON tu.UserId = b.UserId LEFT JOIN Comments c ON c.UserId = tu.UserId WHERE tu.ViewRank <= 10
// GROUP BY tu.UserId, tu.DisplayName, tu.PostsCreated, tu.TotalViews, tu.AnswersCreated, tu.QuestionsCreated, b.Name ORDER BY tu.TotalViews DESC;
//
// The comments depend only on the user, so each (user, badge name) group counts the user's distinct comments.
fn q6445(db: &'static So) -> String {
    let Post { post_type_id, view_count, .. } = &db.post;
    let ua = db
        .user
        .with((&db.user.creation_date).ge(ts(2022, 1, 1, 0, 0, 0)))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(view_count.opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some((t, w)) => [a[0] + 1, a[1] + w.unwrap_or(0), a[2] + (t == 2) as i64, a[3] + (t == 1) as i64],
            None => a,
        });
    let v = ranked(drain(&ua), |&(_, a)| Reverse(a[1]), false);
    let tu = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, a), _)| (u, a)).collect());
    let stats: HashIdx<Id<User>, (Id<User>, [i64; 4])> = (&tu).map(|(u, _)| u).inv().select(&tu).collect();
    let users: MatSet<Id<User>> = (&tu).map(|(u, _)| u).collect();
    let g = (&users).group_by(Ident::<User>::new().and(badges_of(db).select(&db.badge.name).opt())).select(comments_by(db).opt()).buf_fold(|x| distinct_some(x.iter().copied()));
    rows(drain((&g).and(Same::<(Id<User>, Option<Str>)>::new().map(|(u, _)| u).select(&stats))).into_iter().map(|((u, b), (c, (_, a)))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        f.extend([ostr(b), V::I(c)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(a.Id) AS AnswerCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, ROW_NUMBER() OVER (ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p LEFT JOIN Posts a ON p.Id = a.ParentId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount),
// TopRankedPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, AnswerCount, UpVotes, DownVotes, Rank FROM RankedPosts WHERE Rank <= 50)
// SELECT tr.PostId, tr.Title, tr.CreationDate, tr.Score, tr.ViewCount, tr.AnswerCount, tr.UpVotes, tr.DownVotes, u.DisplayName AS AuthorName, u.Reputation, u.CreationDate AS UserCreationDate
// FROM TopRankedPosts tr JOIN Users u ON tr.PostId IN (SELECT DISTINCT OwnerUserId FROM Posts WHERE Id = tr.PostId) ORDER BY tr.Rank;
//
// The ON clause names only tr (a post whose OwnerUserId equals its own Id, compared as raw ids), so those posts are crossed with every user.
// Rank reads only Score and ViewCount, so the fifty posts are picked first and the answer x vote product is driven for those alone.
fn q6982(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, origid, owner_user_id, .. } = &db.post;
    let v = top_n(drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(score)), |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 50);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .with(origid.and(owner_user_id).filt(|(a, b)| a == b))
        .group_by(Ident::<Post>::new())
        .select(children_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let mut v = Vec::new();
    (&s).cross(&db.user.id).drive(|(p, u), (a, _)| v.push((p, u, a)));
    rows(v.into_iter().map(|(p, u, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend(a.map(V::I));
        f.extend(ucols(db, u, &["name", "rep", "ucreated"]));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Tags, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.UserId) AS UniqueVoterCount, COUNT(DISTINCT b.Id) AS BadgeCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY COUNT(c.Id) DESC, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON p.OwnerUserId = b.UserId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.Tags, p.PostTypeId),
// FilteredRankedPosts AS (SELECT * FROM RankedPosts WHERE Rank <= 10)
// SELECT r.PostId, r.Title, r.Tags, r.CommentCount, r.UniqueVoterCount, r.UpvoteCount, r.DownvoteCount, b.Name AS UserBadge
// FROM FilteredRankedPosts r LEFT JOIN Users u ON r.PostId = u.Id LEFT JOIN Badges b ON u.Id = b.UserId ORDER BY r.UpvoteCount DESC, r.CommentCount DESC;
//
// `r.PostId = u.Id` joins a post id to a user id, so it goes through the raw ids.
fn q25346(db: &'static So) -> String {
    let Post { owner_user, creation_date, post_type_id, .. } = &db.post;
    let rp = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(owner_user.select(badges_of(db)).opt()))
        .fold([0i64; 3], |a, ((c, t), _)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let top = top_per(drain((&rp).and(post_type_id)), |&(_, (_, t))| t, |&(p, (a, _))| (Reverse(a[0]), Reverse(a[1]), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let uv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select((&db.vote.user).opt()).opt()).buf_fold(|x| distinct_some(x.iter().map(|u| u.flatten())));
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let badge = (&db.post.origid).select(&uidx).select(badges_of(db).select(&db.badge.name)).opt();
    rows(drain((&tp).select((&rp).and(&uv).and(badge))).into_iter().map(|(p, ((a, n), b))| {
        let mut f = post_fields(db, p, &["id", "title", "tags"]);
        f.extend([V::I(a[0]), V::I(n), V::I(a[1]), V::I(a[2]), ostr(b)]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.CreationDate, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(P.ViewCount) AS TotalViews, SUM(V.BountyAmount) AS TotalBounty
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId = 9 WHERE U.Reputation > 1000 GROUP BY U.Id, U.DisplayName, U.Reputation, U.CreationDate),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, AnswerCount, QuestionCount, TotalViews, TotalBounty, ROW_NUMBER() OVER (ORDER BY Reputation DESC, TotalViews DESC) AS Ranking FROM UserActivity)
// SELECT T.DisplayName, T.Reputation, T.PostCount, T.AnswerCount, T.QuestionCount, T.TotalViews, COALESCE(T.TotalBounty, 0) AS TotalBounty,
//        LPAD(COALESCE(T.Ranking::text, '999'), 3, '0') AS Ranking
// FROM TopUsers T WHERE T.Ranking <= 10 ORDER BY T.Ranking ASC;
//
// Ranking leads with Reputation, so only users with at least the tenth-highest reputation can rank in the top ten; the posts x bounty votes product is driven for those alone.
fn q816(db: &'static So) -> String {
    let rich = || db.user.with((&db.user.reputation).gt(1000));
    let tenth = top_n(drain(rich().select(&db.user.reputation)), |&(u, r)| (Reverse(r), u), 10).last().unwrap().1;
    let cand: MatSet<Id<User>> = rich().with((&db.user.reputation).ge(tenth)).collect();
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let bounty = votes_of(db).select(Ident::<Vote>::new().with(vote_type_id.eq(9))).select(bounty_amount.opt());
    let ua = (&cand)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and((&db.post.view_count).opt()).and(bounty.opt())).opt())
        .fold([0i64; 6], |a, p| match p {
            Some(((t, w), b)) => {
                let b = b.flatten();
                [a[0] + (t == 2) as i64, a[1] + (t == 1) as i64, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + b.is_some() as i64, a[5] + b.unwrap_or(0)]
            }
            None => a,
        });
    let pc = (&cand).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let v = top_n(drain((&ua).and(&pc)), |&(u, (a, _))| (Reverse(db.user.reputation.get(u).unwrap()), a[2] == 0, Reverse(a[3]), u), 10);
    rows(v.into_iter().enumerate().map(|(i, (u, (a, n)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), nullable(a[3], a[2]), V::I(a[5]), V::Owned(format!("{:03}", i + 1))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, u.DisplayName AS OwnerDisplayName, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT v.Id) AS VoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= DATE '2024-10-01' - INTERVAL '1 month' GROUP BY p.Id, u.DisplayName, p.Title, p.CreationDate, p.ViewCount, p.PostTypeId),
// TopPosts AS (SELECT PostId, Title, CreationDate, ViewCount, OwnerDisplayName, CommentCount, VoteCount FROM RankedPosts WHERE Rank <= 5),
// PostHistoryCounts AS (SELECT ph.PostId, COUNT(*) AS HistoryCount FROM PostHistory ph GROUP BY ph.PostId)
// SELECT tp.Title, tp.CreationDate, tp.ViewCount, tp.OwnerDisplayName, tp.CommentCount, tp.VoteCount, COALESCE(phc.HistoryCount, 0) AS TotalHistoryRecords
// FROM TopPosts tp LEFT JOIN PostHistoryCounts phc ON tp.PostId = phc.PostId ORDER BY tp.ViewCount DESC;
fn q9847(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_months(date(2024, 10, 1), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let hc = (&tp).group_by(Ident::<Post>::new()).select(history_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    rows(drain((&cc).and(&vc).and(&hc)).into_iter().map(|(p, ((c, n), h))| {
        let mut f = post_fields(db, p, &["title", "created", "views", "owner"]);
        f.extend([V::I(c), V::I(n), V::I(h)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, u.DisplayName AS OwnerDisplayName, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT CASE WHEN v.VoteTypeId = 2 THEN v.Id END) AS UpVoteCount,
//        COUNT(DISTINCT CASE WHEN v.VoteTypeId = 3 THEN v.Id END) AS DownVoteCount, SUM(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS CloseCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.LastActivityDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, u.DisplayName, p.PostTypeId, p.LastActivityDate),
// TopPosts AS (SELECT rp.* FROM RankedPosts rp WHERE rp.Rank <= 5)
// SELECT tp.PostId, tp.Title, tp.OwnerDisplayName, tp.CommentCount, tp.UpVoteCount, tp.DownVoteCount, tp.CloseCount,
//        (SELECT COUNT(*) FROM Badges b WHERE b.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = tp.PostId)) AS BadgeCount
// FROM TopPosts tp ORDER BY tp.UpVoteCount DESC, tp.CommentCount DESC;
//
// Rank reads only base columns, so the posts are picked first and the comment x vote x history product is driven for those alone.
fn q8098(db: &'static So) -> String {
    let Post { post_type_id, creation_date, last_activity_date, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(last_activity_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let close = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).opt()).and(history_of(db).select(&db.post_history.post_history_type_id).opt()))
        .fold(0i64, |n, (_, h)| n + (h == Some(10)) as i64);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let bc = owner_user.select(db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64));
    rows(drain((&close).and(&cc).and(&vc).and(bc)).into_iter().map(|(p, (((k, c), a), b))| {
        let mut f = post_fields(db, p, &["id", "title", "owner"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1]), V::I(k), V::I(b)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.CreationDate, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(coalesce(vs.VoteCount, 0)) AS VoteCount, SUM(COALESCE(b.Class, 0)) AS BadgeCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN (SELECT PostId, COUNT(*) AS VoteCount FROM Votes GROUP BY PostId) vs ON p.Id = vs.PostId
//     LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation, u.CreationDate),
// RankedUsers AS (SELECT us.*, RANK() OVER (ORDER BY us.Reputation DESC, us.PostCount DESC) AS ReputationRank FROM UserStats us)
// SELECT ru.UserId, ru.DisplayName, ru.Reputation, ru.CreationDate, ru.PostCount, ru.QuestionCount, ru.AnswerCount, ru.VoteCount, ru.BadgeCount, (SELECT COUNT(*) FROM Users) AS TotalUsers
// FROM RankedUsers ru WHERE ru.ReputationRank <= 10 ORDER BY ru.Reputation DESC;
//
// ReputationRank leads with Reputation, so only users with at least the tenth-highest reputation can rank in the top ten; the product is driven for those alone.
fn q5485(db: &'static So) -> String {
    let tenth = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10).last().unwrap().1;
    let cand: MatSet<Id<User>> = db.user.with((&db.user.reputation).ge(tenth)).collect();
    let vs = db.vote.group_by(&db.vote.post).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let us = (&cand)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and((&vs).opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 4], |a, (p, b)| {
            let (t, n) = p.map_or((0, None), |x| x);
            [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + n.unwrap_or(0), a[3] + b.unwrap_or(0)]
        });
    let pc = (&cand).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let total = count(&db.user.id);
    let v = ranked(drain((&us).and(&pc)), |&(u, (_, n))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(n)), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, (a, n)), _)| {
        let mut f = ucols(db, u, &["uid", "name", "rep", "ucreated"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(total)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, COUNT(c.Id) AS CommentCount, COUNT(v.Id) AS VoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS RN
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.PostTypeId),
// TopPosts AS (SELECT PostId, Title, CreationDate, ViewCount, Score, CommentCount, VoteCount FROM RankedPosts WHERE RN <= 10)
// SELECT tp.Title, tp.CreationDate, tp.ViewCount, tp.Score, tp.CommentCount, tp.VoteCount, u.DisplayName AS OwnerDisplayName, u.Reputation AS OwnerReputation,
//        CASE WHEN bp.Name IS NOT NULL THEN 'Has Badge' ELSE 'No Badge' END AS BadgeStatus
// FROM TopPosts tp JOIN Posts p ON tp.PostId = p.Id JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Badges bp ON u.Id = bp.UserId AND bp.Class = 1 ORDER BY tp.Score DESC, tp.ViewCount DESC;
//
// RN reads only base columns, so the posts are picked first and the comment x vote product is driven for those alone.
fn q6623(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold([0i64; 2], |a, (c, v)| [a[0] + c.is_some() as i64, a[1] + v.is_some() as i64]);
    let gold = badges_of(db).select(Ident::<Badge>::new().with((&db.badge.class).eq(1)));
    rows(drain((&s).and(owner_user.select(Ident::<User>::new().and(gold.opt())))).into_iter().map(|(p, (a, (u, b)))| {
        let mut f = post_fields(db, p, &["title", "created", "views", "score"]);
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.push(V::S(if b.is_some() { "Has Badge" } else { "No Badge" }));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, COALESCE(SUM(CASE WHEN vote.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN vote.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COUNT(DISTINCT bh.Id) AS BadgeCount, MAX(p.CreationDate) AS LastActive
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes vote ON p.Id = vote.PostId LEFT JOIN Badges bh ON u.Id = bh.UserId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, UpVotes, DownVotes, BadgeCount, LastActive, RANK() OVER (ORDER BY PostCount DESC, UpVotes DESC) AS Rank FROM UserActivity)
// SELECT t.UserId, t.DisplayName, t.PostCount, t.UpVotes, t.DownVotes, t.BadgeCount, t.LastActive,
//        CASE WHEN t.Rank <= 10 THEN 'Top Contributor' WHEN t.Rank <= 50 THEN 'Active Contributor' ELSE 'Regular User' END AS UserType
// FROM TopUsers t WHERE t.Rank <= 100 ORDER BY t.Rank;
//
// Rank leads with the distinct post count, so only users with at least the hundredth-highest count can rank in the top hundred; the product is driven for those alone.
fn q7703(db: &'static So) -> String {
    let rich = || db.user.with((&db.user.reputation).gt(1000));
    let pc = rich().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let hundredth = top_n(drain(&pc), |&(u, n)| (Reverse(n), u), 100).last().unwrap().1;
    let cand: MatSet<Id<User>> = db.user.with((&pc).filt(|n| n >= hundredth)).collect();
    let ua = (&cand)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.creation_date).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64, 0, i64::MIN], |a, (p, _)| {
            let t = p.and_then(|(_, t)| t);
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2].max(p.map_or(i64::MIN, |(d, _)| d))]
        });
    let bc = (&cand).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = ranked(drain((&ua).and(&pc).and(&bc)), |&(_, ((a, n), _))| (Reverse(n), Reverse(a[0])), false);
    rows(v.into_iter().take_while(|x| x.1 <= 100).map(|((u, ((a, n), b)), r)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(b), tmax(a[2])]);
        f.push(V::S(if r <= 10 { "Top Contributor" } else if r <= 50 { "Active Contributor" } else { "Regular User" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank, COUNT(c.Id) AS CommentCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.ViewCount > 100
//     GROUP BY p.Id, p.Title, p.Score, p.ViewCount, p.CreationDate, p.PostTypeId),
// ClosedPosts AS (SELECT ph.PostId, ph.UserId, ph.CreationDate, MIN(ph.Comment) AS CloseComment FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.PostId, ph.UserId, ph.CreationDate)
// SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, COALESCE(cp.CloseComment, 'No closure comments') AS ClosureDescription, rp.CreationDate,
//        CASE WHEN rp.Rank <= 5 THEN 'Top Post' ELSE 'Regular Post' END AS PostCategory
// FROM RankedPosts rp LEFT JOIN ClosedPosts cp ON rp.PostId = cp.PostId WHERE rp.CommentCount > 0 ORDER BY rp.Score DESC, rp.ViewCount DESC LIMIT 50;
fn q22656(db: &'static So) -> String {
    let Post { post_type_id, creation_date, view_count, score, .. } = &db.post;
    let base = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(view_count.gt(100)));
    let v = ranked(drain(base().select(post_type_id)), |&(p, t)| (t, Reverse(score.get(p).unwrap()), p), false);
    let v = per_group(v, |&(_, t)| t);
    let rp = rel(v.into_iter().map(|((p, _), r)| (p, r)).collect());
    let rn: HashIdx<Id<Post>, (Id<Post>, i64)> = (&rp).map(|(p, _)| p).inv().select(&rp).collect();
    let cc = base().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let PostHistory { post, user, comment, creation_date: hd, post_history_type_id, .. } = &db.post_history;
    let cp = db.post_history.with(post_history_type_id.is_in([10, 11])).group_by(post.and(user.opt()).and(hd)).select(comment.opt()).fold(None, |m: Option<Str>, c| match (m, c) {
        (Some(m), Some(c)) => Some(m.min(c)),
        (m, c) => m.or(c),
    });
    let cv = rel(drain(&cp));
    let by_post: HashIdx<Id<Post>, (((Id<Post>, Option<Id<User>>), i64), Option<Str>)> = (&cv).map(|(((p, _), _), _)| p).inv().select(&cv).collect();
    let v = drain((&cc).filt(|n| n > 0).and(&rn).and((&by_post).opt()));
    let v = top_n(v, |&(p, (_, c))| (Reverse(score.get(p).unwrap()), Reverse(view_count.get(p)), p, c.map(|(k, _)| k)), 50);
    rows(v.into_iter().map(|(p, ((_, (_, r)), c))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views"]);
        f.push(V::S(c.and_then(|(_, m)| m).unwrap_or("No closure comments")));
        f.extend(post_fields(db, p, &["created"]));
        f.push(V::S(if r <= 5 { "Top Post" } else { "Regular Post" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.ViewCount, P.CreationDate, U.DisplayName AS OwnerDisplay, RANK() OVER (PARTITION BY P.OwnerUserId ORDER BY P.ViewCount DESC) AS RankedViewCount
//     FROM Posts P INNER JOIN Users U ON P.OwnerUserId = U.Id WHERE P.PostTypeId = 1 AND P.ViewCount IS NOT NULL),
// TopComments AS (SELECT C.PostId, COUNT(C.Id) AS CommentCount FROM Comments C GROUP BY C.PostId),
// RecentPostHistory AS (SELECT PH.PostId, PH.CreationDate, PHT.Name AS ChangeType FROM PostHistory PH INNER JOIN PostHistoryTypes PHT ON PH.PostHistoryTypeId = PHT.Id
//     WHERE PH.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days')
// SELECT RP.PostId, RP.Title, RP.ViewCount, RP.OwnerDisplay, COALESCE(TC.CommentCount, 0) AS TotalComments, COALESCE(PH.ChangeType, 'No Recent Changes') AS RecentChangeType
// FROM RankedPosts RP LEFT JOIN TopComments TC ON RP.PostId = TC.PostId LEFT JOIN RecentPostHistory PH ON RP.PostId = PH.PostId WHERE RP.RankedViewCount = 1 ORDER BY RP.ViewCount DESC;
fn q4881(db: &'static So) -> String {
    let Post { post_type_id, owner_user, view_count, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1)).with(view_count).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| Reverse(view_count.get(p).unwrap()), 1, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let recent = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.creation_date).ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))).select(htype_name(db));
    rows(drain((&cc).and(recent.opt())).into_iter().map(|(p, (c, t))| {
        let mut f = post_fields(db, p, &["id", "title", "views", "owner"]);
        f.extend([V::I(c), V::S(t.unwrap_or("No Recent Changes"))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS Owner, COUNT(c.Id) AS CommentCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 WHEN v.VoteTypeId = 3 THEN -1 END), 0) AS VoteScore, ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS RN
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, Owner, CommentCount, VoteScore, RANK() OVER (ORDER BY (Score + VoteScore + CommentCount) DESC) AS PostRank FROM RankedPosts)
// SELECT tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.Owner, tp.CommentCount, tp.VoteScore, tp.PostRank,
//        EXTRACT(EPOCH FROM cast('2024-10-01 12:34:56' as timestamp) - tp.CreationDate) AS AgeInSeconds
// FROM TopPosts tp WHERE tp.PostRank <= 10 ORDER BY tp.PostRank;
fn q6654(db: &'static So) -> String {
    let Post { creation_date, score, .. } = &db.post;
    let s = db
        .post
        .with(creation_date.ge(add_days(date(2024, 10, 1), -30)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + match t { Some(2) => 1, Some(3) => -1, _ => 0 }]);
    let v = ranked(drain(&s), |&(p, a)| Reverse(score.get(p).unwrap() + a[1] + a[0]), false);
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((p, a), r)| {
        let mut f = post_fields(db, p, &["title", "created", "score", "views", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(r), V::F(secs(t0 - creation_date.get(p).unwrap()))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS rn
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.Score > 0),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.ViewCount IS NULL THEN 0 ELSE p.ViewCount END) AS TotalViews,
//        SUM(CASE WHEN p.Score IS NULL THEN 0 ELSE p.Score END) AS TotalScore FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT us.UserId, us.DisplayName, us.PostCount, us.TotalViews, us.TotalScore, DENSE_RANK() OVER (ORDER BY us.TotalScore DESC) AS Rank FROM UserStats us WHERE us.PostCount > 5)
// SELECT tu.DisplayName, tu.TotalScore, tu.TotalViews, RP.Title AS TopPostTitle, RP.CreationDate AS TopPostDate
// FROM TopUsers tu LEFT JOIN RankedPosts RP ON tu.UserId = RP.OwnerUserId AND RP.rn = 1 WHERE tu.Rank <= 10 ORDER BY tu.TotalScore DESC, tu.TotalViews DESC;
fn q729(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, view_count, .. } = &db.post;
    let us = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(score.and(view_count.opt())).opt()).fold([0i64; 3], |a, p| match p {
        Some((s, w)) => [a[0] + 1, a[1] + w.unwrap_or(0), a[2] + s],
        None => a,
    });
    let v = ranked(drain((&us).filt(|a| a[0] > 5)), |&(_, a)| Reverse(a[2]), true);
    let tu = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, a), _)| (u, a)).collect());
    let top = top_per(drain(db.post.with(post_type_id.eq(1).and(score.gt(0))).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 1, false);
    let pp = rel(top.into_iter().map(|(p, u)| (u, p)).collect());
    let by_user: HashIdx<Id<User>, (Id<User>, Id<Post>)> = (&pp).map(|(u, _)| u).inv().select(&pp).collect();
    type R = (Id<User>, [i64; 3]);
    let v = drain((&tu).select(Same::<R>::new().and(Same::<R>::new().map(|(u, _)| u).select((&by_user).map(|(_, p)| p)).opt())));
    rows(v.into_iter().map(|(_, ((u, a), p))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[2]), V::I(a[1])];
        f.extend(match p {
            Some(p) => post_fields(db, p, &["title", "created"]),
            None => vec![V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank, COUNT(c.Id) AS CommentCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, AVG(p.Score) AS AvgScore FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY u.Id, u.DisplayName, u.Reputation HAVING AVG(p.Score) > 10)
// SELECT tu.UserId, tu.DisplayName, tu.Reputation, rp.Title AS LatestPostTitle, rp.CreationDate AS LatestPostDate, rp.CommentCount AS LatestPostComments, COALESCE(b.Name, 'No Badge') AS BadgeName
// FROM TopUsers tu LEFT JOIN RankedPosts rp ON tu.UserId = rp.OwnerUserId AND rp.PostRank = 1 LEFT JOIN Badges b ON tu.UserId = b.UserId
// WHERE (b.Class = 1 OR b.Class = 2 OR b.Class IS NULL) ORDER BY tu.Reputation DESC, rp.CreationDate DESC;
//
// PostRank reads only the owner and CreationDate, so each owner's newest post is picked first and its comments are counted for those alone.
// AVG(p.Score) > 10 is compared exactly, as sum > 10 * n.
fn q481(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, .. } = &db.post;
    let tu = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(owner_user).select(score).fold([0i64; 2], |a, s| [a[0] + s, a[1] + 1]);
    let top = top_per(drain(db.post.select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let pp = rel(top.into_iter().map(|(p, u)| (u, p)).collect());
    let by_user: HashIdx<Id<User>, (Id<User>, Id<Post>)> = (&pp).map(|(u, _)| u).inv().select(&pp).collect();
    let latest: MatSet<Id<Post>> = (&pp).map(|(_, p)| p).collect();
    let pcc = (&latest).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let rp = (&by_user).map(|(_, p)| p).select(Ident::<Post>::new().and(&pcc)).opt();
    let badge = badges_of(db).select(Ident::<Badge>::new().and(&db.badge.class)).opt().filt(|b: Option<(Id<Badge>, i64)>| b.map_or(true, |(_, c)| c == 1 || c == 2));
    let v = drain((&tu).filt(|a| a[0] > 10 * a[1]).and(rp).and(badge));
    rows(v.into_iter().map(|(u, ((_, p), b))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(match p {
            Some((p, c)) => [harness::fmt::ostr(db.post.title.get(p)), V::T(creation_date.get(p).unwrap()), V::I(c)],
            None => [V::Null, V::Null, V::Null],
        });
        f.push(V::S(b.map_or("No Badge", |(b, _)| db.badge.name.get(b).unwrap())));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COUNT(DISTINCT c.Id) AS CommentCount, RANK() OVER (ORDER BY p.Score DESC, p.ViewCount DESC) AS RankScore
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.UpVotes, rp.DownVotes, rp.CommentCount, rp.RankScore, ROW_NUMBER() OVER (ORDER BY rp.RankScore) AS RowNum FROM RankedPosts rp)
// SELECT tp.Title, tp.CreationDate, tp.UpVotes, tp.DownVotes, tp.CommentCount, tp.ViewCount, CASE WHEN tp.RowNum <= 10 THEN 'Top 10' ELSE 'Not Top 10' END AS RankCategory
// FROM TopPosts tp WHERE tp.ViewCount > 100 ORDER BY tp.RankScore ASC;
fn q6955(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let qs = || db.post.with(post_type_id.eq(1));
    let s = qs()
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()))
        .fold([0i64; 2], |a, (t, _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = qs().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = ranked(drain(qs().select(score)), |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    }, false);
    if v.len() > 10 && v[9].1 == v[10].1 {
        eprintln!("tie at the RowNum cut");
    }
    let rn = rel(v.into_iter().enumerate().map(|(i, ((p, _), _))| (p, i as i64 + 1)).collect());
    let rnk: HashIdx<Id<Post>, (Id<Post>, i64)> = (&rn).map(|(p, _)| p).inv().select(&rn).collect();
    rows(drain(qs().with(view_count.gt(100)).select((&s).and(&cc).and(&rnk))).into_iter().map(|(p, ((a, c), (_, n)))| {
        let mut f = post_fields(db, p, &["title", "created"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c)]);
        f.extend(post_fields(db, p, &["views"]));
        f.push(V::S(if n <= 10 { "Top 10" } else { "Not Top 10" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS timestamp) - INTERVAL '1 year'),
// PopularTags AS (SELECT t.TagName, COUNT(p.Id) AS PostCount FROM Tags t JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' GROUP BY t.TagName HAVING COUNT(p.Id) > 5),
// UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounty, COUNT(DISTINCT p.Id) AS PostsCount, MAX(u.Reputation) AS Reputation
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName)
// SELECT rp.Title, rp.ViewCount, rp.Score, pt.TagName, ur.DisplayName AS UserDisplayName, ur.TotalBounty, ur.Reputation
// FROM RankedPosts rp JOIN PopularTags pt ON rp.Title LIKE '%' || pt.TagName || '%' JOIN UserReputation ur ON rp.PostId = ur.UserId
// WHERE rp.Rank <= 3 AND ur.Reputation > 1000 ORDER BY rp.Score DESC, rp.ViewCount DESC LIMIT 10;
//
// `rp.PostId = ur.UserId` joins a post id to a user id, so it goes through the raw ids; UserReputation is computed for the users that join.
fn q33639(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, title, origid, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 3, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let lt = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&lt).map(|(_, t)| t).inv().collect();
    let pt = db.tag.group_by(&db.tag.tag_name).select((&by_tag).map(|(p, _)| p)).fold(0i64, |n, _| n + 1);
    let ptv = rel(drain((&pt).filt(|n| n > 5)).into_iter().map(|(t, _)| t).collect());
    let ptk: HashIdx<Str, Str> = (&ptv).map(|t| t).inv().select(&ptv).collect();
    let titles: MatSet<Str> = (&tp).select(title).collect();
    let like: HashIdx<Str, Str> = (&titles).select_where(&ptk, |t: Str, n: Str| t.contains(n)).collect();
    let uidx: HashIdx<i64, Id<User>> = db.user.with((&db.user.reputation).gt(1000)).select(&db.user.origid).inv().collect();
    let cand: MatSet<Id<User>> = (&tp).select(origid.select(&uidx)).map(|u| u).collect();
    let ur = (&cand).group_by(Ident::<User>::new()).select(votes_by(db).select((&db.vote.bounty_amount).opt()).opt().and(posts_of(db).opt())).fold(0i64, |s, (b, _)| s + b.flatten().unwrap_or(0));
    let v = drain((&tp).select(title.select(&like).and(origid.select(&uidx).select(Ident::<User>::new().and(&ur)))));
    let v = top_n(v, |&(p, (n, (u, _)))| (Reverse(score.get(p).unwrap()), Reverse(view_count.get(p)), p, n, u), 10);
    rows(v.into_iter().map(|(p, (n, (u, b)))| {
        let mut f = post_fields(db, p, &["title", "views", "score"]);
        f.extend([V::S(n), user_col(db, u, "name"), V::I(b), user_col(db, u, "rep")]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes, SUM(COALESCE(B.Class, 0)) AS TotalBadges
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Badges B ON U.Id = B.UserId WHERE U.Reputation > 0 GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalUpVotes, TotalDownVotes, TotalBadges,
//        RANK() OVER (ORDER BY TotalPosts DESC) AS PostRank, RANK() OVER (ORDER BY TotalUpVotes DESC) AS UpVoteRank FROM UserActivity)
// SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalUpVotes, TotalDownVotes, TotalBadges, PostRank, UpVoteRank
// FROM TopUsers WHERE PostRank <= 10 OR UpVoteRank <= 10 ORDER BY PostRank, UpVoteRank;
fn q5367(db: &'static So) -> String {
    let users = || db.user.with((&db.user.reputation).gt(0));
    let ua = users()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 5], |a, (p, b)| {
            let (t, v) = p.map_or((0, None), |x| x);
            [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64, a[4] + b.unwrap_or(0)]
        });
    let pc = users().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let v = ranked(drain((&ua).and(&pc)), |&(_, (_, n))| Reverse(n), false);
    let v = ranked(v, |&((_, (a, _)), _)| Reverse(a[2]), false);
    let v = rel(v.into_iter().map(|(((u, (a, n)), pr), ur)| (u, a, n, pr, ur)).collect());
    type R = (Id<User>, [i64; 5], i64, i64, i64);
    rows(drain((&v).filt(|(_, _, _, pr, ur): R| pr <= 10 || ur <= 10)).into_iter().map(|(_, (u, a, n, pr, ur))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.extend([V::I(pr), V::I(ur)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(DISTINCT c.Id) AS CommentCount, COALESCE(MAX(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(MAX(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank, p.OwnerUserId
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId),
// UserReputation AS (SELECT u.Id AS UserId, SUM(u.Reputation) AS TotalReputation FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.CommentCount, rp.UpVotes, rp.DownVotes, ur.TotalReputation, rp.Rank
// FROM RankedPosts rp JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId WHERE rp.Rank <= 5 ORDER BY ur.TotalReputation DESC, rp.Score DESC;
//
// Rank reads only base columns, so each owner's top five questions are picked first and the comment x vote product is driven for those alone.
fn q5575(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, creation_date, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1)).select(owner_user.opt()));
    let v = ranked(v, |&(p, u)| (u, Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), false);
    let v = per_group(v, |&(_, u)| u);
    let rp = rel(v.into_iter().filter(|x| x.1 <= 5).map(|((p, _), r)| (p, r)).collect());
    let rn: HashIdx<Id<Post>, (Id<Post>, i64)> = (&rp).map(|(p, _)| p).inv().select(&rp).collect();
    let tp: MatSet<Id<Post>> = (&rp).map(|(p, _)| p).collect();
    let s = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0].max((t == Some(2)) as i64), a[1].max((t == Some(3)) as i64)]);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ur = db.user.group_by(Ident::<User>::new()).select((&db.user.reputation).and(posts_of(db))).fold(0i64, |s, (r, _)| s + r);
    rows(drain((&s).and(&cc).and(&rn).and(owner_user.select(&ur))).into_iter().map(|(p, (((a, c), (_, r)), t))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1]), V::I(t), V::I(r)]);
        row(f)
    }))
}

// WITH RankedUsers AS (SELECT Id, DisplayName, Reputation, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM Users),
// ActivePosts AS (SELECT P.Id AS PostId, P.Title, P.OwnerUserId, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount, COUNT(CASE WHEN V.Id IS NOT NULL THEN 1 END) AS VoteCount,
//        MAX(P.LastActivityDate) AS LastActivity FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId
//     WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY P.Id, P.Title, P.OwnerUserId),
// PostDetails AS (SELECT AP.PostId, AP.Title, U.DisplayName AS OwnerDisplayName, RU.ReputationRank, AP.CommentCount, AP.VoteCount, AP.LastActivity
//     FROM ActivePosts AP JOIN Users U ON AP.OwnerUserId = U.Id JOIN RankedUsers RU ON U.Id = RU.Id WHERE AP.CommentCount > 0 OR AP.VoteCount > 0),
// TopPosts AS (SELECT *, RANK() OVER (PARTITION BY ReputationRank ORDER BY CommentCount DESC, VoteCount DESC) AS RankWithinGroup FROM PostDetails)
// SELECT PostId, Title, OwnerDisplayName, ReputationRank, CommentCount, VoteCount, LastActivity FROM TopPosts WHERE RankWithinGroup <= 5 ORDER BY ReputationRank, RankWithinGroup;
fn q5063(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let ru = rel(ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false).into_iter().map(|((u, _), r)| (u, r)).collect());
    let by_user: HashIdx<Id<User>, (Id<User>, i64)> = (&ru).map(|(u, _)| u).inv().select(&ru).collect();
    let ap = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).opt()))
        .fold([0i64; 2], |a, (c, v)| [a[0] + c.is_some() as i64, a[1] + v.is_some() as i64]);
    let v = drain((&ap).filt(|a| a[0] > 0 || a[1] > 0).and(owner_user.select(&by_user)));
    let top = top_per(v, |&(_, (_, (_, r)))| r, |&(_, (a, _))| (Reverse(a[0]), Reverse(a[1])), 5, true);
    rows(top.into_iter().map(|(p, (a, (u, r)))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([user_col(db, u, "name"), V::I(r), V::I(a[0]), V::I(a[1])]);
        f.extend(post_fields(db, p, &["activity"]));
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, AVG(p.Score) AS AverageScore, SUM(v.BountyAmount) AS TotalBounty, MAX(p.CreationDate) AS LastPostDate
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON v.PostId = p.Id GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, AverageScore, TotalBounty, LastPostDate, RANK() OVER (ORDER BY TotalPosts DESC) AS PostRank FROM UserPostStats)
// SELECT tu.DisplayName, tu.TotalPosts, tu.TotalQuestions, tu.TotalAnswers, COALESCE(tu.AverageScore, 0) AS AverageScore, COALESCE(tu.TotalBounty, 0) AS TotalBounty,
//        CASE WHEN tu.LastPostDate < cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 YEAR' THEN 'Inactive' ELSE 'Active' END AS UserStatus
// FROM TopUsers tu WHERE tu.PostRank <= 10 ORDER BY tu.TotalPosts DESC;
fn q1040(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, .. } = &db.post;
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(creation_date).and(votes_of(db).select((&db.vote.bounty_amount).opt()).opt())).opt())
        .fold([0i64, 0, 0, 0, 0, 0, i64::MIN], |a, p| match p {
            Some((((t, s), d), b)) => {
                let b = b.flatten();
                [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + b.is_some() as i64, a[5] + b.unwrap_or(0), a[6].max(d)]
            }
            None => a,
        });
    let v = ranked(drain(&us), |&(_, a)| Reverse(a[0]), false);
    let cut = add_years(ts(2024, 10, 1, 12, 34, 56), -1);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, a), _)| {
        let status = if a[0] > 0 && a[6] < cut { "Inactive" } else { "Active" };
        row(vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), if a[0] == 0 { V::F(0.0) } else { avg(a[3], a[0]) }, V::I(a[5]), V::S(status)])
    }))
}

// WITH UserBadges AS (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId),
// RecentPosts AS (SELECT P.Id, P.Title, P.OwnerUserId, P.CreationDate, P.ViewCount, RANK() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS RecentRank
//     FROM Posts P WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// TopUsers AS (SELECT U.Id, U.DisplayName, U.Reputation, COALESCE(UB.BadgeCount, 0) AS BadgeCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS UserRank
//     FROM Users U LEFT JOIN UserBadges UB ON U.Id = UB.UserId LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.DisplayName, U.Reputation, UB.BadgeCount)
// SELECT TU.DisplayName, TU.Reputation, TU.BadgeCount, R.Title AS RecentPostTitle, R.ViewCount AS RecentPostViews,
//        CASE WHEN R.RecentRank = 1 THEN 'Most Recent' ELSE 'Not Most Recent' END AS RecentPostStatus
// FROM TopUsers TU LEFT JOIN RecentPosts R ON TU.Id = R.OwnerUserId WHERE TU.UserRank <= 10 ORDER BY TU.Reputation DESC, RecentPostViews DESC NULLS LAST;
//
// UserRank reads only Reputation, and nothing from the vote sums is projected.
fn q2656(db: &'static So) -> String {
    let tu = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let bc = (&tu).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let Post { creation_date, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(owner_user));
    let v = ranked(v, |&(p, u)| (u, Reverse(creation_date.get(p).unwrap())), false);
    let v = per_group(v, |&(_, u)| u);
    let rp = rel(v.into_iter().map(|((p, u), r)| (u, (p, r))).collect());
    let recent: HashIdx<Id<User>, (Id<User>, (Id<Post>, i64))> = (&rp).map(|(u, _)| u).inv().select(&rp).collect();
    rows(drain((&bc).and((&recent).map(|(_, x)| x).opt())).into_iter().map(|(u, (b, r))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(b));
        f.extend(match r {
            Some((p, k)) => {
                let mut g = post_fields(db, p, &["title", "views"]);
                g.push(V::S(if k == 1 { "Most Recent" } else { "Not Most Recent" }));
                g
            }
            None => vec![V::Null, V::Null, V::S("Not Most Recent")],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 month' GROUP BY p.Id, p.Title, p.CreationDate, u.DisplayName, p.PostTypeId),
// SelectedPosts AS (SELECT PostId, Title, OwnerDisplayName, CommentCount, UpVoteCount, DownVoteCount FROM RankedPosts WHERE Rank <= 10)
// SELECT sp.PostId, sp.Title, sp.OwnerDisplayName, sp.CommentCount, sp.UpVoteCount, sp.DownVoteCount, COALESCE(b.Name, 'No Badge') AS UserBadge
// FROM SelectedPosts sp LEFT JOIN Badges b ON sp.PostId = b.UserId ORDER BY sp.UpVoteCount DESC, sp.CommentCount DESC;
//
// `sp.PostId = b.UserId` joins a post id to a user id, so it goes through the raw ids. Rank reads only base columns, so the posts are picked first.
fn q7814(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let bidx: HashIdx<i64, Id<Badge>> = (&db.badge.user_id).inv().collect();
    rows(drain((&s).and((&db.post.origid).select(&bidx).opt())).into_iter().map(|(p, (a, b))| {
        let mut f = post_fields(db, p, &["id", "title", "owner"]);
        f.extend(a.map(V::I));
        f.push(V::S(b.map_or("No Badge", |b| db.badge.name.get(b).unwrap())));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes, COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 4 THEN 1 ELSE 0 END), 0) AS OffensiveVotes, COALESCE(SUM(CASE WHEN V.VoteTypeId = 10 THEN 1 ELSE 0 END), 0) AS PostDeletions
//     FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.DisplayName),
// RecentPosts AS (SELECT P.OwnerUserId, P.Title, P.ViewCount, P.Score, RANK() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS Rank
//     FROM Posts P WHERE P.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 DAY')
// SELECT U.DisplayName, US.Upvotes, US.Downvotes, US.PostDeletions, SP.Title AS RecentPostTitle, SP.ViewCount, SP.Score
// FROM UserStats US LEFT JOIN RecentPosts SP ON US.UserId = SP.OwnerUserId AND SP.Rank = 1 JOIN Users U ON US.UserId = U.Id
// WHERE US.Upvotes > US.Downvotes AND US.PostDeletions = 0 ORDER BY US.Upvotes DESC, U.Reputation DESC FETCH FIRST 10 ROWS ONLY;
fn q235(db: &'static So) -> String {
    let us = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + (t == Some(10)) as i64]);
    let Post { creation_date, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| Reverse(creation_date.get(p).unwrap()), 1, true);
    let rp = rel(top.into_iter().map(|(p, u)| (u, p)).collect());
    let recent: HashIdx<Id<User>, (Id<User>, Id<Post>)> = (&rp).map(|(u, _)| u).inv().select(&rp).collect();
    let v = drain((&us).filt(|a| a[0] > a[1] && a[2] == 0).and((&recent).map(|(_, p)| p).opt()));
    let v = top_n(v, |&(u, (a, p))| (Reverse(a[0]), Reverse(db.user.reputation.get(u).unwrap()), u, p), 10);
    rows(v.into_iter().map(|(u, (a, p))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2])];
        f.extend(match p {
            Some(p) => post_fields(db, p, &["title", "views", "score"]),
            None => vec![V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts,
//        SUM(CASE WHEN P.Score < 0 THEN 1 ELSE 0 END) AS NegativePosts, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, PositivePosts, NegativePosts, UpVotes, DownVotes, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserReputation WHERE TotalPosts > 10)
// SELECT T.DisplayName, T.Reputation, T.TotalPosts, T.PositivePosts, T.NegativePosts, T.UpVotes, T.DownVotes, (T.UpVotes * 1.0 / NULLIF(T.TotalPosts, 0)) * 100 AS UpVotePercentage,
//        (T.DownVotes * 1.0 / NULLIF(T.TotalPosts, 0)) * 100 AS DownVotePercentage
// FROM TopUsers T WHERE T.Rank <= 10 ORDER BY T.Rank;
//
// Rank reads only Reputation among users with more than ten posts, so those ten users are picked first and the posts x votes product is driven for them alone.
fn q8861(db: &'static So) -> String {
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let tu = top_n(drain((&pc).filt(|n| n > 10)), |&(u, _)| (Reverse(db.user.reputation.get(u).unwrap()), u), 10);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let us = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.score).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some((s, t)) => [a[0] + (s > 0) as i64, a[1] + (s < 0) as i64, a[2] + (t == Some(2)) as i64, a[3] + (t == Some(3)) as i64],
            None => a,
        });
    rows(drain((&us).and(&pc)).into_iter().map(|(u, (a, n))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.extend([V::F(a[2] as f64 * 1.0 / n as f64 * 100.0), V::F(a[3] as f64 * 1.0 / n as f64 * 100.0)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT v.Id) AS VoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= '2024-10-01 12:34:56'::timestamp - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.PostTypeId),
// TopRankedPosts AS (SELECT PostId, Title, CreationDate, Score, CommentCount, VoteCount FROM RankedPosts WHERE Rank <= 10)
// SELECT u.DisplayName AS Author, tp.Title AS PostTitle, tp.Score AS PostScore, tp.CommentCount AS TotalComments, tp.VoteCount AS TotalVotes, COALESCE(b.Name, 'No Badge') AS BadgeName, tp.CreationDate
// FROM TopRankedPosts tp LEFT JOIN Users u ON u.Id = (SELECT AcceptedAnswerId FROM Posts WHERE Id = tp.PostId) LEFT JOIN Badges b ON u.Id = b.UserId AND b.Date > tp.CreationDate
// ORDER BY tp.Score DESC, tp.CreationDate DESC;
//
// `u.Id = AcceptedAnswerId` joins a user id to a post id, so it goes through the raw ids. Rank reads only base columns, so the posts are picked first.
fn q6336(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, accepted_answer_id, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    type R = (Id<Post>, Option<Id<User>>);
    let pu = rel(drain((&tp).select(accepted_answer_id.select(&uidx).opt())));
    let later = Same::<R>::new()
        .map(|(p, _): R| creation_date.get(p).unwrap())
        .and(Same::<R>::new().flat_map(|(_, u): R| u).select(badges_of(db)).select(Ident::<Badge>::new().and(&db.badge.date)))
        .filt(|(c, (_, d)): (i64, (Id<Badge>, i64))| d > c)
        .map(|(_, (b, _)): (i64, (Id<Badge>, i64))| b);
    let v = drain((&pu).select(Same::<R>::new().and(later.opt()).and(Same::<R>::new().map(|(p, _): R| p).select((&cc).and(&vc)))));
    rows(v.into_iter().map(|(_, (((p, u), b), (c, n)))| {
        let mut f = vec![u.map_or(V::Null, |u| user_col(db, u, "name"))];
        f.extend(post_fields(db, p, &["title", "score"]));
        f.extend([V::I(c), V::I(n), V::S(b.map_or("No Badge", |b| db.badge.name.get(b).unwrap()))]);
        f.extend(post_fields(db, p, &["created"]));
        row(f)
    }))
}

// WITH UserScoreStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(COALESCE(VoteCount, 0)) AS TotalVotes,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes, COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN (SELECT PostId, COUNT(*) AS VoteCount FROM Votes GROUP BY PostId) AS VCounts ON P.Id = VCounts.PostId
//     LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, TotalVotes, Upvotes, Downvotes, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank,
//        RANK() OVER (ORDER BY TotalVotes DESC) AS VoteRank FROM UserScoreStats)
// SELECT UserId, DisplayName, Reputation, PostCount, TotalVotes, Upvotes, Downvotes, ReputationRank, VoteRank FROM TopUsers
// WHERE ReputationRank <= 10 OR VoteRank <= 10 ORDER BY Reputation DESC, VoteRank;
fn q7296(db: &'static So) -> String {
    let vs = db.vote.group_by(&db.vote.post).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&vs).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 3], |a, p| match p {
            Some((n, t)) => [a[0] + n.unwrap_or(0), a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64],
            None => a,
        });
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let v = ranked(drain((&us).and(&pc)), |&(u, _)| Reverse(db.user.reputation.get(u).unwrap()), false);
    let v = ranked(v, |&((_, (a, _)), _)| Reverse(a[0]), false);
    type R = (Id<User>, [i64; 3], i64, i64, i64);
    let v = rel(v.into_iter().map(|(((u, (a, n)), rr), vr)| (u, a, n, rr, vr)).collect::<Vec<R>>());
    rows(drain((&v).filt(|(_, _, _, rr, vr): R| rr <= 10 || vr <= 10)).into_iter().map(|(_, (u, a, n, rr, vr))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.extend([V::I(rr), V::I(vr)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, U.DisplayName AS Owner, COUNT(c.Id) AS CommentCount,
//        COUNT(DISTINCT CASE WHEN v.VoteTypeId = 2 THEN v.Id END) AS UpvoteCount, COUNT(DISTINCT CASE WHEN v.VoteTypeId = 3 THEN v.Id END) AS DownvoteCount,
//        RANK() OVER (ORDER BY p.Score DESC, p.CreationDate DESC) AS ScoreRank
//     FROM Posts p LEFT JOIN Users U ON p.OwnerUserId = U.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= '2024-10-01 12:34:56'::timestamp - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, U.DisplayName),
// TopPosts AS (SELECT rp.*, ROW_NUMBER() OVER (ORDER BY rp.ViewCount DESC) AS ViewRank FROM RankedPosts rp)
// SELECT t.PostId, t.Title, t.CreationDate, t.ViewCount, t.Score, t.Owner, t.CommentCount, t.UpvoteCount, t.DownvoteCount, t.ScoreRank, t.ViewRank
// FROM TopPosts t WHERE t.ScoreRank <= 10 AND t.ViewRank <= 10 ORDER BY t.ScoreRank, t.ViewRank;
fn q9374(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, .. } = &db.post;
    let qs = || db.post.with(post_type_id.eq(1).and(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))));
    let cc = qs().group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let vc = qs().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = ranked(drain((&cc).and(&vc)), |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())), false);
    let v = ranked(v, |&((p, _), _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), p)
    }, false);
    rows(v.into_iter().filter(|&((_, s), w)| s <= 10 && w <= 10).map(|(((p, (c, a)), s), w)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "owner"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1]), V::I(s), V::I(w)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RN
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// PopularUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName HAVING COUNT(v.Id) >= 1),
// CommentStats AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount, AVG(c.Score) AS AverageScore FROM Comments c GROUP BY c.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, pu.TotalUpvotes, pu.TotalDownvotes, cs.CommentCount, cs.AverageScore
// FROM RankedPosts rp LEFT JOIN PopularUsers pu ON rp.OwnerUserId = pu.UserId LEFT JOIN CommentStats cs ON rp.PostId = cs.PostId
// WHERE rp.RN = 1 AND pu.TotalUpvotes IS NOT NULL ORDER BY rp.Score DESC, rp.ViewCount DESC LIMIT 50;
fn q134(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(owner_user.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pu = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let cs = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.score)).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let v = drain((&tp).select(owner_user.select(&pu).and((&cs).opt())));
    let v = top_n(v, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 50);
    rows(v.into_iter().map(|(p, (a, c))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.extend(match c {
            Some(c) => [V::I(c[0]), avg(c[1], c[0])],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(COALESCE(p.Score, 0)) AS TotalScore, MIN(p.CreationDate) AS FirstPostDate,
//        RANK() OVER (PARTITION BY u.Id ORDER BY SUM(COALESCE(p.Score, 0)) DESC) AS ScoreRank FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// HighScorers AS (SELECT UserId, DisplayName, PostCount, TotalScore, FirstPostDate FROM UserActivity WHERE ScoreRank <= 10),
// RecentPosts AS (SELECT p.Id AS PostId, p.OwnerUserId, p.Title, p.CreationDate, p.Score, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RecentRank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days')
// SELECT hs.DisplayName, hs.PostCount, hs.TotalScore, hs.FirstPostDate, rp.PostId, rp.Title, rp.CreationDate, rp.Score
// FROM HighScorers hs LEFT JOIN RecentPosts rp ON hs.UserId = rp.OwnerUserId AND rp.RecentRank <= 5 ORDER BY hs.TotalScore DESC, hs.DisplayName;
//
// ScoreRank is partitioned by the user itself, so every user ranks 1 and is kept.
fn q678(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, .. } = &db.post;
    let ua = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(score.and(creation_date)).opt()).fold([0i64, 0, i64::MAX], |a, p| match p {
        Some((s, d)) => [a[0] + 1, a[1] + s, a[2].min(d)],
        None => a,
    });
    let v = drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| Reverse(creation_date.get(p).unwrap()), 5, true);
    let rp = rel(top.into_iter().map(|(p, u)| (u, p)).collect());
    let recent: HashIdx<Id<User>, (Id<User>, Id<Post>)> = (&rp).map(|(u, _)| u).inv().select(&rp).collect();
    rows(drain((&ua).and((&recent).map(|(_, p)| p).opt())).into_iter().map(|(u, (a, p))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), tmin(a[2])];
        f.extend(match p {
            Some(p) => post_fields(db, p, &["id", "title", "created", "score"]),
            None => vec![V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(COALESCE(V.BountyAmount, 0)) AS TotalBounty, RANK() OVER (ORDER BY COUNT(P.Id) DESC) AS ActivityRank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId IN (8, 9) GROUP BY U.Id, U.DisplayName),
// PopularPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, ROW_NUMBER() OVER (ORDER BY P.Score DESC, P.ViewCount DESC) AS PopularityRank
//     FROM Posts P WHERE P.CreationDate > (cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'))
// SELECT UA.DisplayName, UA.TotalPosts, UA.QuestionCount, UA.AnswerCount, UA.TotalBounty, PP.Title AS PopularPostTitle, PP.CreationDate AS PopularPostDate, PP.Score AS PopularPostScore
// FROM UserActivity UA LEFT JOIN PopularPosts PP ON UA.QuestionCount > 0 WHERE UA.ActivityRank <= 10 ORDER BY UA.TotalPosts DESC, UA.TotalBounty DESC;
//
// The ON clause names only UA, so a user with questions is crossed with every recent post, and one without gets the single NULL row. PopularityRank is never read.
fn q1134(db: &'static So) -> String {
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let bounty = votes_of(db).select(Ident::<Vote>::new().with(vote_type_id.is_in([8, 9]))).select(bounty_amount.opt());
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(bounty.opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some((t, b)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + b.flatten().unwrap_or(0)],
            None => a,
        });
    let v = ranked(drain(&ua), |&(_, a)| Reverse(a[0]), false);
    let tu = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0).collect());
    let pp = rel(drain(db.post.with((&db.post.creation_date).gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))).into_iter().map(|(p, _)| p).collect());
    let with_q = (&tu).filt(|(_, a): (Id<User>, [i64; 4])| a[1] > 0);
    let without = (&tu).filt(|(_, a): (Id<User>, [i64; 4])| a[1] == 0);
    let mut v: Vec<((Id<User>, [i64; 4]), Option<Id<Post>>)> = Vec::new();
    with_q.cross(&pp).drive(|_, (x, p)| v.push((x, Some(p))));
    without.drive(|_, x| v.push((x, None)));
    rows(v.into_iter().map(|((u, a), p)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(a.map(V::I));
        f.extend(match p {
            Some(p) => post_fields(db, p, &["title", "created", "score"]),
            None => vec![V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COUNT(c.Id) AS CommentCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) - SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Score, ROW_NUMBER() OVER (ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate),
// TopPosts AS (SELECT PostId, Title, CreationDate, CommentCount, Score FROM RankedPosts WHERE rn <= 10),
// UserPosts AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName)
// SELECT up.UserId, up.DisplayName, up.PostCount, up.AcceptedAnswers, tp.Title, tp.CommentCount, tp.Score FROM UserPosts up JOIN TopPosts tp ON up.PostCount > 0 ORDER BY tp.Score DESC, up.PostCount DESC;
//
// The ON clause names only up, so users with posts are crossed with the ten newest questions. rn reads only CreationDate, so those are picked first.
fn q6138(db: &'static So) -> String {
    let Post { post_type_id, creation_date, accepted_answer_id, .. } = &db.post;
    let v = top_n(drain(db.post.with(post_type_id.eq(1)).select(creation_date)), |&(p, d)| (Reverse(d), p), 10);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + match t { Some(2) => 1, Some(3) => -1, _ => 0 }]);
    let up = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(accepted_answer_id.opt()).opt()).fold([0i64; 2], |a, p| match p {
        Some(acc) => [a[0] + 1, a[1] + acc.is_some() as i64],
        None => a,
    });
    let mut v = Vec::new();
    (&up).filt(|a| a[0] > 0).cross(&s).drive(|(u, p), (a, b)| v.push((u, a, p, b)));
    rows(v.into_iter().map(|(u, a, p, b)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.extend(post_fields(db, p, &["title"]));
        f.extend([V::I(b[0]), V::I(b[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS Author, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes, ROW_NUMBER() OVER (ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days') GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, Author, Upvotes, Downvotes, Rank FROM RankedPosts WHERE Rank <= 10)
// SELECT tp.Title, tp.Score, tp.ViewCount, tp.Author, (tp.Upvotes - tp.Downvotes) AS NetVotes, (SELECT COUNT(*) FROM Comments c WHERE c.PostId = tp.PostId) AS CommentCount,
//        (SELECT COUNT(*) FROM Votes v WHERE v.PostId = tp.PostId AND v.VoteTypeId = 9) AS BountyCount
// FROM TopPosts tp ORDER BY NetVotes DESC, tp.Score DESC;
fn q6660(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, .. } = &db.post;
    let v = top_n(drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))).select(score)), |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 10);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let vs = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + (t == Some(9)) as i64]);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    rows(drain((&vs).and(&cc)).into_iter().map(|(p, (a, c))| {
        let mut f = post_fields(db, p, &["title", "score", "views", "owner"]);
        f.extend([V::I(a[0] - a[1]), V::I(c), V::I(a[2])]);
        row(f)
    }))
}

// WITH UserVotes AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(V.Id) AS TotalVotes, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.DisplayName),
// PostStats AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, COALESCE(P.AnswerCount, 0) AS AnswerCount, COALESCE(V.TotalVotes, 0) AS TotalVotes, COALESCE(V.UpVotes, 0) AS UpVotes,
//        COALESCE(V.DownVotes, 0) AS DownVotes, ROW_NUMBER() OVER (ORDER BY P.CreationDate DESC) AS PostRank
//     FROM Posts P LEFT JOIN UserVotes V ON P.OwnerUserId = V.UserId WHERE P.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year')
// SELECT PS.PostId, PS.Title, PS.CreationDate, PS.AnswerCount, PS.TotalVotes, PS.UpVotes, PS.DownVotes,
//        CASE WHEN PS.PostRank <= 10 THEN 'Top Post' WHEN PS.TotalVotes = 0 THEN 'No Votes' ELSE 'Average Post' END AS PostCategory
// FROM PostStats PS WHERE PS.AnswerCount > 0 ORDER BY PS.TotalVotes DESC, PS.CreationDate ASC LIMIT 100;
fn q3643(db: &'static So) -> String {
    let Post { creation_date, owner_user, answer_count, .. } = &db.post;
    let uv = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| [a[0] + t.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = drain(db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1))).select(creation_date));
    let v = top_n(v, |&(p, d)| (Reverse(d), p), 0);
    let ps = rel(v.into_iter().enumerate().map(|(i, (p, _))| (p, i as i64 + 1)).collect());
    type R = (Id<Post>, i64);
    let v = drain((&ps).filt(|(p, _): R| answer_count.get(p).unwrap_or(0) > 0).select(Same::<R>::new().and(Same::<R>::new().map(|(p, _): R| p).select(owner_user.select(&uv).opt()))));
    let v = top_n(v, |&(_, ((p, _), a))| (Reverse(a.map_or(0, |a| a[0])), creation_date.get(p).unwrap(), p), 100);
    rows(v.into_iter().map(|(_, ((p, r), a))| {
        let a = a.unwrap_or([0; 3]);
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.push(V::I(answer_count.get(p).unwrap_or(0)));
        f.extend(a.map(V::I));
        f.push(V::S(if r <= 10 { "Top Post" } else if a[0] == 0 { "No Votes" } else { "Average Post" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.ViewCount, P.Score, U.DisplayName AS OwnerDisplayName, COUNT(C.Id) AS CommentCount, RANK() OVER (ORDER BY P.Score DESC, P.ViewCount DESC) AS Rank
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id LEFT JOIN Comments C ON P.Id = C.PostId
//     WHERE P.PostTypeId = 1 AND P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY P.Id, P.Title, P.ViewCount, P.Score, U.DisplayName),
// TopPosts AS (SELECT PostId, Title, ViewCount, Score, OwnerDisplayName FROM RankedPosts WHERE Rank <= 10)
// SELECT T.Title, T.ViewCount, T.Score, T.OwnerDisplayName, COALESCE(SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges,
//        COALESCE(SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadges, COALESCE(SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadges
// FROM TopPosts T LEFT JOIN Badges B ON B.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = T.PostId)
// GROUP BY T.PostId, T.Title, T.ViewCount, T.Score, T.OwnerDisplayName ORDER BY T.Score DESC, T.ViewCount DESC;
fn q5756(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, owner_user, .. } = &db.post;
    let v = ranked(drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).with(owner_user).select(score)), |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    }, false);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let b = (&tp).group_by(Ident::<Post>::new()).select(owner_user.select(badges_of(db)).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    rows(drain(&b).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["title", "views", "score", "owner"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COALESCE(SUM(CASE WHEN c.Id IS NOT NULL THEN 1 ELSE 0 END), 0) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.PostTypeId),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.UpVotes, rp.DownVotes, rp.CommentCount FROM RankedPosts rp WHERE rp.Rank <= 10)
// SELECT tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.UpVotes, tp.DownVotes, tp.CommentCount, COALESCE(u.DisplayName, 'Anonymous') AS OwnerDisplayName, u.Reputation
// FROM TopPosts tp LEFT JOIN Users u ON tp.PostId = u.Id ORDER BY tp.Score DESC, tp.CreationDate DESC;
//
// `tp.PostId = u.Id` joins a post id to a user id, so it goes through the raw ids. Rank reads only base columns, so the posts are picked first.
fn q7003(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, .. } = &db.post;
    let top = top_per(drain(db.post.select(post_type_id)), |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()))
        .fold([0i64; 3], |a, (t, c)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64]);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    rows(drain((&s).and((&db.post.origid).select(&uidx).opt())).into_iter().map(|(p, (a, u))| {
        let mut f = post_fields(db, p, &["title", "created", "score", "views"]);
        f.extend(a.map(V::I));
        f.extend(match u {
            Some(u) => ucols(db, u, &["name", "rep"]),
            None => vec![V::S("Anonymous"), V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId IN (1, 2) AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.PostTypeId),
// FilteredPosts AS (SELECT rp.*, pt.Name AS PostTypeName, u.DisplayName AS OwnerDisplayName FROM RankedPosts rp JOIN PostTypes pt ON rp.PostId = pt.Id JOIN Users u ON rp.PostId = u.Id WHERE rp.Rank <= 10)
// SELECT f.PostId, f.Title, f.CreationDate, f.Score, f.ViewCount, f.CommentCount, f.UpVotes, f.DownVotes, f.PostTypeName, f.OwnerDisplayName FROM FilteredPosts f ORDER BY f.Rank;
//
// `rp.PostId = pt.Id` and `rp.PostId = u.Id` join a post id to other tables' ids, so they go through the raw ids. Rank reads only base columns, so the posts are picked first.
fn q5234(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, origid, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.is_in([1, 2]).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let tidx: HashIdx<i64, Id<PostType>> = (&db.post_type.origid).inv().collect();
    rows(drain((&s).and(origid.select(&tidx)).and(origid.select(&uidx))).into_iter().map(|(p, ((a, t), u))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend(a.map(V::I));
        f.extend([V::S(db.post_type.name.get(t).unwrap()), user_col(db, u, "name")]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId WHERE u.Reputation > 100 GROUP BY u.Id, u.DisplayName),
// UserBadges AS (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId),
// ActiveUsers AS (SELECT ua.UserId, ua.DisplayName, ua.PostCount, ua.QuestionCount, ua.AnswerCount, ua.UpVotes, ua.DownVotes, COALESCE(ub.BadgeCount, 0) AS BadgeCount
//     FROM UserActivity ua LEFT JOIN UserBadges ub ON ua.UserId = ub.UserId)
// SELECT au.DisplayName, au.PostCount, au.QuestionCount, au.AnswerCount, au.UpVotes, au.DownVotes, au.BadgeCount, ROW_NUMBER() OVER (ORDER BY au.PostCount DESC, au.UpVotes DESC) AS Rank
// FROM ActiveUsers au ORDER BY au.PostCount DESC, au.UpVotes DESC LIMIT 10;
//
// The order leads with the distinct post count, so only users with at least the tenth-highest count can make the cut; the posts x votes product is driven for those alone.
fn q9613(db: &'static So) -> String {
    let rich = || db.user.with((&db.user.reputation).gt(100));
    let pc = rich().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let tenth = top_n(drain(&pc), |&(u, n)| (Reverse(n), u), 10).last().unwrap().1;
    let cand: MatSet<Id<User>> = db.user.with((&pc).filt(|n| n >= tenth)).collect();
    let ua = (&cand)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some((t, v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64],
            None => a,
        });
    let bc = (&cand).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = top_n(drain((&ua).and(&pc).and(&bc)), |&(u, ((a, n), _))| (Reverse(n), Reverse(a[2]), u), 10);
    rows(v.into_iter().enumerate().map(|(i, (u, ((a, n), b)))| {
        let mut f = vec![user_col(db, u, "name"), V::I(n)];
        f.extend(a.map(V::I));
        f.extend([V::I(b), V::I(i as i64 + 1)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN P.ViewCount IS NOT NULL THEN P.ViewCount ELSE 0 END) AS TotalViews,
//        SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId WHERE U.Reputation > 100 GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, TotalQuestions, TotalAnswers, TotalViews, TotalUpVotes, TotalDownVotes, RANK() OVER (ORDER BY Reputation DESC) AS Rank FROM UserStats)
// SELECT T.DisplayName, T.Reputation, T.TotalPosts, T.TotalQuestions, T.TotalAnswers, T.TotalViews, T.TotalUpVotes, T.TotalDownVotes FROM TopUsers T WHERE T.Rank <= 10 ORDER BY T.Reputation DESC;
//
// Rank reads only Reputation, so the users are picked first and the posts x votes product is driven for those alone.
fn q5301(db: &'static So) -> String {
    let tu = ranked(drain(db.user.with((&db.user.reputation).gt(100)).select(&db.user.reputation)), |&(_, r)| Reverse(r), false);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let us = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and((&db.post.view_count).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 5], |a, p| match p {
            Some(((t, w), v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + w.unwrap_or(0), a[3] + (v == Some(2)) as i64, a[4] + (v == Some(3)) as i64],
            None => a,
        });
    let pc = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    rows(drain((&us).and(&pc)).into_iter().map(|(u, (a, n))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS TotalPosts, COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS QuestionsPosted,
//        COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS AnswersPosted, SUM(p.Score) AS TotalScore, SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounty
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (8, 9) WHERE u.Reputation > 100 GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, QuestionsPosted, AnswersPosted, TotalScore, TotalBounty, ROW_NUMBER() OVER (ORDER BY TotalScore DESC) AS Rank FROM UserStatistics)
// SELECT t.UserId, t.DisplayName, t.Reputation, t.TotalPosts, t.QuestionsPosted, t.AnswersPosted, t.TotalScore, t.TotalBounty,
//        CASE WHEN t.Rank <= 10 THEN 'Top Contributor' WHEN t.Rank <= 50 THEN 'High Contributor' ELSE 'Contributor' END AS ContributorLevel
// FROM TopUsers t WHERE t.TotalPosts >= 5 ORDER BY t.TotalScore DESC, ContributorLevel;
fn q5619(db: &'static So) -> String {
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let bounty = votes_of(db).select(Ident::<Vote>::new().with(vote_type_id.is_in([8, 9]))).select(bounty_amount.opt());
    let users = || db.user.with((&db.user.reputation).gt(100));
    let us = users().group_by(Ident::<User>::new()).select(posts_of(db).select((&db.post.score).and(bounty.opt())).opt()).fold([0i64; 3], |a, p| match p {
        Some((s, b)) => [a[0] + 1, a[1] + s, a[2] + b.flatten().unwrap_or(0)],
        None => a,
    });
    let qa = users().group_by(Ident::<User>::new()).select(posts_of(db).select(&db.post.post_type_id).opt()).fold([0i64; 3], |a, t| [a[0] + t.is_some() as i64, a[1] + (t == Some(1)) as i64, a[2] + (t == Some(2)) as i64]);
    let v = top_n(drain((&us).and(&qa)), |&(u, (a, _))| (a[0] == 0, Reverse(a[1]), u), 0);
    for c in [10, 50] {
        if v.len() > c && (v[c - 1].1).0[1] == (v[c].1).0[1] {
            eprintln!("tie at the Rank cut {c}");
        }
    }
    let t = rel(v.into_iter().enumerate().map(|(i, (u, (a, q)))| (u, a, q, i as i64 + 1)).collect());
    type R = (Id<User>, [i64; 3], [i64; 3], i64);
    rows(drain((&t).filt(|(_, _, q, _): R| q[0] >= 5)).into_iter().map(|(_, (u, a, q, r))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(q[0]), V::I(q[1]), V::I(q[2]), nullable(a[1], a[0]), V::I(a[2])]);
        f.push(V::S(if r <= 10 { "Top Contributor" } else if r <= 50 { "High Contributor" } else { "Contributor" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.CreationDate, p.Score, u.DisplayName AS OwnerName, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT v.Id) AS VoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate > CURRENT_DATE - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.ViewCount, p.CreationDate, p.Score, u.DisplayName, p.PostTypeId),
// TopPosts AS (SELECT PostId, Title, ViewCount, CreationDate, Score, OwnerName, CommentCount, VoteCount FROM RankedPosts WHERE Rank <= 10)
// SELECT t.PostId, t.Title, t.ViewCount, t.CreationDate, t.Score, t.OwnerName, t.CommentCount, t.VoteCount, COALESCE(b.BadgesCount, 0) AS BadgeCount
// FROM TopPosts t LEFT JOIN (SELECT UserId, COUNT(*) AS BadgesCount FROM Badges GROUP BY UserId) b ON b.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = t.PostId LIMIT 1) ORDER BY t.Score DESC;
fn q7371(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.gt(add_years(current_date(), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    rows(drain((&cc).and(&vc).and(owner_user.select(&bc).opt())).into_iter().map(|(p, ((c, n), b))| {
        let mut f = post_fields(db, p, &["id", "title", "views", "created", "score", "owner"]);
        f.extend([V::I(c), V::I(n), V::I(b.unwrap_or(0))]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositiveScoreCount,
//        SUM(CASE WHEN p.Score < 0 THEN 1 ELSE 0 END) AS NegativeScoreCount, AVG(COALESCE(p.Score, 0)) AS AverageScore FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, PositiveScoreCount, NegativeScoreCount, AverageScore, ROW_NUMBER() OVER (ORDER BY TotalPosts DESC) AS UserRank FROM UserPostStats WHERE TotalPosts > 0)
// SELECT tu.DisplayName, tu.TotalPosts, tu.PositiveScoreCount, tu.NegativeScoreCount, tu.AverageScore, COALESCE(b.BadgeCount, 0) AS BadgeCount,
//        CASE WHEN tu.AverageScore IS NULL THEN 'No Score' WHEN tu.AverageScore > 0 THEN 'Positive' ELSE 'Negative' END AS ScoreCategory
// FROM TopUsers tu LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId) b ON tu.UserId = b.UserId WHERE tu.UserRank <= 10 ORDER BY tu.TotalPosts DESC;
fn q4298(db: &'static So) -> String {
    let us = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(&db.post.score).opt()).fold([0i64; 4], |a, s| match s {
        Some(s) => [a[0] + 1, a[1] + (s > 0) as i64, a[2] + (s < 0) as i64, a[3] + s],
        None => a,
    });
    let tu = top_n(drain((&us).filt(|a| a[0] > 0)), |&(u, a)| (Reverse(a[0]), u), 10);
    let tu = rel(tu);
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    type R = (Id<User>, [i64; 4]);
    rows(drain((&tu).select(Same::<R>::new().and(Same::<R>::new().map(|(u, _): R| u).select(&bc).opt()))).into_iter().map(|(_, ((u, a), b))| {
        row(vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[3], a[0]), V::I(b.unwrap_or(0)), V::S(if a[3] > 0 { "Positive" } else { "Negative" })])
    }))
}

// WITH UserMetrics AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.CreationDate, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(CASE WHEN P.PostTypeId = 1 AND P.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedQuestions,
//        SUM(CASE WHEN B.Id IS NOT NULL THEN 1 ELSE 0 END) AS BadgesCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Badges B ON U.Id = B.UserId WHERE U.Reputation > 50 GROUP BY U.Id, U.DisplayName, U.Reputation, U.CreationDate),
// MostActiveUsers AS (SELECT UserId, DisplayName, Reputation, CreationDate, TotalPosts, Questions, Answers, AcceptedQuestions, BadgesCount, RANK() OVER (ORDER BY TotalPosts DESC) AS PostRank FROM UserMetrics)
// SELECT M.UserId, M.DisplayName, M.Reputation, M.CreationDate, M.TotalPosts, M.Questions, M.Answers, M.AcceptedQuestions, M.BadgesCount FROM MostActiveUsers M WHERE M.PostRank <= 10 ORDER BY M.PostRank;
//
// PostRank reads only the distinct post count, so the users are picked on that first and the posts x badges product is driven for those alone.
fn q9236(db: &'static So) -> String {
    let pc = db.user.with((&db.user.reputation).gt(50)).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let tu = ranked(drain(&pc), |&(_, n)| Reverse(n), false);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let Post { post_type_id, accepted_answer_id, .. } = &db.post;
    let um = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(accepted_answer_id.opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 4], |a, (p, b)| {
            let (t, acc) = p.map_or((0, false), |(t, acc)| (t, acc.is_some()));
            [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (t == 1 && acc) as i64, a[3] + b.is_some() as i64]
        });
    rows(drain((&um).and(&pc)).into_iter().map(|(u, (a, n))| {
        let mut f = ucols(db, u, &["uid", "name", "rep", "ucreated"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVoteCount,
//        COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVoteCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.PostTypeId),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.CommentCount, rp.UpVoteCount, rp.DownVoteCount FROM RankedPosts rp WHERE rp.Rank <= 10)
// SELECT tp.*, COALESCE(t.TagName, 'No Tag') AS TagName, COALESCE(b.Name, 'No Badge') AS BadgeName
// FROM TopPosts tp LEFT JOIN Tags t ON tp.PostId = t.ExcerptPostId LEFT JOIN Badges b ON tp.PostId = b.UserId WHERE b.Class = 1 OR b.Class = 2 ORDER BY tp.Score DESC, tp.ViewCount DESC;
//
// `tp.PostId = b.UserId` joins a post id to a user id, so it goes through the raw ids. Rank reads only base columns, so the posts are picked first.
fn q9503(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, origid, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let excerpt: HashIdx<Id<Post>, Id<Tag>> = (&db.tag.excerpt_post).inv().collect();
    let bidx: HashIdx<i64, Id<Badge>> = db.badge.with((&db.badge.class).is_in([1, 2])).select(&db.badge.user_id).inv().collect();
    rows(drain((&s).and((&excerpt).opt()).and(origid.select(&bidx))).into_iter().map(|(p, ((a, t), b))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend(a.map(V::I));
        f.extend([V::S(t.map_or("No Tag", |t| db.tag.tag_name.get(t).unwrap())), V::S(db.badge.name.get(b).unwrap())]);
        row(f)
    }))
}

// WITH UserBadges AS (SELECT b.UserId, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Badges b GROUP BY b.UserId),
// TopPosts AS (SELECT p.OwnerUserId, p.Id AS PostId, p.Title, p.Score, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS Rank FROM Posts p WHERE p.Score > 0)
// SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(ub.GoldBadges, 0) AS GoldBadges, COALESCE(ub.SilverBadges, 0) AS SilverBadges, COALESCE(ub.BronzeBadges, 0) AS BronzeBadges,
//        tp.Title AS TopPostTitle, tp.Score AS TopPostScore, tp.CreationDate AS TopPostCreationDate, CASE WHEN tp.Rank = 1 THEN 'Top Post' ELSE 'Other Post' END AS PostRank
// FROM Users u LEFT JOIN UserBadges ub ON u.Id = ub.UserId LEFT JOIN TopPosts tp ON u.Id = tp.OwnerUserId AND tp.Rank <= 3
// WHERE u.Reputation IS NOT NULL AND u.Reputation > (SELECT AVG(Reputation) FROM Users) ORDER BY u.Reputation DESC, tp.Score DESC NULLS LAST LIMIT 50;
//
// Reputation > AVG(Reputation) is compared exactly, as r * n > sum.
fn q3808(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    let tot = db.user.select(&db.user.reputation).fold_flat([0i64; 2], |a, r| [a[0] + r, a[1] + 1]);
    let Post { owner_user, score, .. } = &db.post;
    let v = drain(db.post.with(score.gt(0)).select(owner_user));
    let v = ranked(v, |&(p, u)| (u, Reverse(score.get(p).unwrap()), p), false);
    let v = per_group(v, |&(_, u)| u);
    let tp = rel(v.into_iter().filter(|x| x.1 <= 3).map(|((p, u), r)| (u, (p, r))).collect());
    let top: HashIdx<Id<User>, (Id<User>, (Id<Post>, i64))> = (&tp).map(|(u, _)| u).inv().select(&tp).collect();
    let v = drain(db.user.with((&db.user.reputation).filt(|r| r * tot[1] > tot[0])).select((&ub).and((&top).map(|(_, x)| x).opt())));
    let v = top_n(v, |&(u, (_, t))| (Reverse(db.user.reputation.get(u).unwrap()), t.is_none(), Reverse(t.map(|(p, _)| score.get(p).unwrap())), u, t), 50);
    rows(v.into_iter().map(|(u, (b, t))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(b.map(V::I));
        f.extend(match t {
            Some((p, r)) => {
                let mut g = post_fields(db, p, &["title", "score", "created"]);
                g.push(V::S(if r == 1 { "Top Post" } else { "Other Post" }));
                g
            }
            None => vec![V::Null, V::Null, V::Null, V::S("Other Post")],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.PostTypeId = 1),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounties, SUM(COALESCE(u.UpVotes, 0) - COALESCE(u.DownVotes, 0)) AS ReputationScore
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName HAVING COUNT(DISTINCT v.PostId) > 3),
// PostInfo AS (SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.CreationDate, tu.DisplayName, tu.TotalBounties
//     FROM RankedPosts rp JOIN TopUsers tu ON rp.PostId IN (SELECT PostId FROM Votes WHERE UserId = tu.UserId) WHERE rp.Rank <= 10)
// SELECT pi.Title, pi.Score, pi.ViewCount, pi.CreationDate, pi.DisplayName, pi.TotalBounties FROM PostInfo pi ORDER BY pi.Score DESC, pi.ViewCount DESC;
//
// The IN subquery is a semi-join: each (post, voter) pair counts once however many votes the voter cast on it.
fn q7182(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.eq(1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let tb = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select((&db.vote.bounty_amount).opt())).fold(0i64, |s, b| s + b.unwrap_or(0));
    let dp = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.post_id)).count_distinct();
    let pairs: MatSet<(Id<Post>, Id<User>)> = (&tp).select(Ident::<Post>::new().and(votes_of(db).select(&db.vote.user))).map(|x| x).collect();
    type R = (Id<Post>, Id<User>);
    let v = drain((&pairs).select(Same::<R>::new().and(Same::<R>::new().map(|(_, u): R| u).select((&tb).and((&dp).filt(|n| n > 3))))));
    rows(v.into_iter().map(|(_, ((p, u), (b, _)))| {
        let mut f = post_fields(db, p, &["title", "score", "views", "created"]);
        f.extend([user_col(db, u, "name"), V::I(b)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year' AND p.Score > 10),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, CASE WHEN u.Reputation > 1000 THEN 'High' WHEN u.Reputation BETWEEN 501 AND 1000 THEN 'Medium' ELSE 'Low' END AS ReputationCategory FROM Users u),
// PostVotes AS (SELECT p.Id AS PostId, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id)
// SELECT rp.Title, rp.CreationDate, ur.ReputationCategory, pv.UpVotes, pv.DownVotes, pv.UpVotes - pv.DownVotes AS VoteBalance
// FROM RankedPosts rp JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId LEFT JOIN PostVotes pv ON rp.Id = pv.PostId WHERE rp.Rank = 1 ORDER BY VoteBalance DESC LIMIT 10;
fn q4484(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1)).and(score.gt(10))).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = top_n(drain((&pv).and(owner_user)), |&(p, (a, _))| (Reverse(a[0] - a[1]), p), 10);
    rows(v.into_iter().map(|(p, (a, u))| {
        let r = db.user.reputation.get(u).unwrap();
        let mut f = post_fields(db, p, &["title", "created"]);
        f.extend([V::S(if r > 1000 { "High" } else if r >= 501 { "Medium" } else { "Low" }), V::I(a[0]), V::I(a[1]), V::I(a[0] - a[1])]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS TotalPosts, COUNT(DISTINCT CASE WHEN P.PostTypeId = 1 THEN P.Id END) AS TotalQuestions,
//        COUNT(DISTINCT CASE WHEN P.PostTypeId = 2 THEN P.Id END) AS TotalAnswers, SUM(COALESCE(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END, 0)) AS TotalUpVotes,
//        SUM(COALESCE(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END, 0)) AS TotalDownVotes
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId WHERE U.Reputation > 1000 GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, TotalQuestions, TotalAnswers, TotalUpVotes, TotalDownVotes, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserStats)
// SELECT U.DisplayName, U.Reputation, U.TotalPosts, U.TotalQuestions, U.TotalAnswers, U.TotalUpVotes, U.TotalDownVotes, RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank
// FROM TopUsers U WHERE U.Rank <= 100 ORDER BY U.TotalPosts DESC, U.Reputation DESC;
//
// Rank reads only Reputation, so the hundred users are picked first and the posts x votes product is driven for those alone.
fn q7106(db: &'static So) -> String {
    let tu = top_n(drain(db.user.with((&db.user.reputation).gt(1000)).select(&db.user.reputation)), |&(u, r)| (Reverse(r), u), 100);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let us = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt())
        .fold([0i64; 2], |a, t| [a[0] + (t == Some(Some(2))) as i64, a[1] + (t == Some(Some(3))) as i64]);
    let qa = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(&db.post.post_type_id).opt()).fold([0i64; 3], |a, t| [a[0] + t.is_some() as i64, a[1] + (t == Some(1)) as i64, a[2] + (t == Some(2)) as i64]);
    let v = ranked(drain((&us).and(&qa)), |&(u, _)| Reverse(db.user.reputation.get(u).unwrap()), false);
    rows(v.into_iter().map(|((u, (a, q)), r)| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(q[0]), V::I(q[1]), V::I(q[2]), V::I(a[0]), V::I(a[1]), V::I(r)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(CASE WHEN p.PostTypeId IN (10, 11) THEN 1 ELSE 0 END) AS CloseVotes,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, Questions, Answers, CloseVotes, Upvotes, Downvotes, RANK() OVER (ORDER BY TotalPosts DESC) AS Rank FROM UserStats)
// SELECT t.DisplayName, t.TotalPosts, t.Questions, t.Answers, t.CloseVotes, t.Upvotes, t.Downvotes, ROUND((CAST(t.Upvotes AS DECIMAL) / NULLIF(t.TotalPosts, 0)) * 100, 2) AS UpvotePercentage,
//        ROUND((CAST(t.Downvotes AS DECIMAL) / NULLIF(t.TotalPosts, 0)) * 100, 2) AS DownvotePercentage
// FROM TopUsers t WHERE t.Rank <= 10 ORDER BY t.Rank;
//
// Rank reads only the distinct post count, so the users are picked on that first and the posts x votes product is driven for those alone.
fn q5325(db: &'static So) -> String {
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let tu = ranked(drain(&pc), |&(_, n)| Reverse(n), false);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let us = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 5], |a, p| match p {
            Some((t, v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + matches!(t, 10 | 11) as i64, a[3] + (v == Some(2)) as i64, a[4] + (v == Some(3)) as i64],
            None => a,
        });
    let pct = |x: i64, n: i64| if n == 0 { V::Null } else { V::F((x as f64 / n as f64 * 100.0 * 100.0).round() / 100.0) };
    rows(drain((&us).and(&pc)).into_iter().map(|(u, (a, n))| {
        let mut f = vec![user_col(db, u, "name"), V::I(n)];
        f.extend(a.map(V::I));
        f.extend([pct(a[3], n), pct(a[4], n)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.PostTypeId, p.CreationDate, p.Score, p.ViewCount, COALESCE(COUNT(c.Id), 0) AS CommentCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
//        DENSE_RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.PostTypeId, p.CreationDate, p.Score, p.ViewCount),
// TopPosts AS (SELECT PostId, Title, PostTypeId, CreationDate, Score, ViewCount, CommentCount, UpVotes, DownVotes FROM RankedPosts WHERE Rank <= 10)
// SELECT tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.CommentCount, tp.UpVotes, tp.DownVotes, pt.Name AS PostTypeName
// FROM TopPosts tp JOIN PostTypes pt ON tp.PostTypeId = pt.Id ORDER BY tp.Score DESC, tp.ViewCount DESC;
//
// Rank reads only Score, so the posts are picked first and the comment x vote product is driven for those alone.
fn q9880(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, post_type, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let v = ranked(v, |&(p, t)| (t, Reverse(score.get(p).unwrap())), true);
    let v = per_group(v, |&(_, t)| t);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().filter(|x| x.1 <= 10).map(|((p, _), _)| p).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain((&s).and(post_type.select(&db.post_type.name))).into_iter().map(|(p, (a, n))| {
        let mut f = post_fields(db, p, &["title", "created", "score", "views"]);
        f.extend(a.map(V::I));
        f.push(V::S(n));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN p.PostTypeId = 1 AND ph.Id IS NOT NULL THEN 1 ELSE 0 END) AS ClosedQuestionCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN PostHistory ph ON p.Id = ph.PostId AND ph.PostHistoryTypeId = 10 GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, ClosedQuestionCount, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserReputation)
// SELECT t.DisplayName, t.Reputation, t.PostCount, t.QuestionCount, t.AnswerCount, t.ClosedQuestionCount,
//        CASE WHEN t.ReputationRank <= 10 THEN 'Top User' WHEN t.ReputationRank <= 50 THEN 'Moderately Active User' ELSE 'New User' END AS UserCategory
// FROM TopUsers t WHERE t.Reputation > 1000 ORDER BY t.Reputation DESC, t.PostCount DESC;
fn q5216(db: &'static So) -> String {
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10)));
    let rich = || db.user.with((&db.user.reputation).gt(1000));
    let ur = rich().group_by(Ident::<User>::new()).select(posts_of(db).select((&db.post.post_type_id).and(closes.opt())).opt()).fold([0i64; 3], |a, p| match p {
        Some((t, h)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (t == 1 && h.is_some()) as i64],
        None => a,
    });
    let pc = rich().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let rank = rel(ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false).into_iter().map(|((u, _), r)| (u, r)).collect());
    let rk: HashIdx<Id<User>, (Id<User>, i64)> = (&rank).map(|(u, _)| u).inv().select(&rank).collect();
    rows(drain((&ur).and(&pc).and(&rk)).into_iter().map(|(u, ((a, n), (_, r)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.push(V::S(if r <= 10 { "Top User" } else if r <= 50 { "Moderately Active User" } else { "New User" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT p2.Id) AS AnswerCount,
//        RANK() OVER (ORDER BY COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) - COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) DESC) AS Rank
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Posts p2 ON p.Id = p2.ParentId WHERE p.PostTypeId = 1
//     GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount),
// TopRankedPosts AS (SELECT * FROM RankedPosts WHERE Rank <= 10)
// SELECT trp.PostId, trp.Title, trp.CreationDate, trp.ViewCount, trp.UpVotes, trp.DownVotes, trp.CommentCount, trp.AnswerCount, ut.DisplayName AS OwnerDisplayName, ut.Reputation AS OwnerReputation
// FROM TopRankedPosts trp JOIN Users ut ON ut.Id = (SELECT p.OwnerUserId FROM Posts p WHERE p.Id = trp.PostId) ORDER BY trp.Rank, trp.ViewCount DESC;
fn q8547(db: &'static So) -> String {
    let Post { post_type_id, owner_user, .. } = &db.post;
    let qs = || db.post.with(post_type_id.eq(1));
    let s = qs()
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()).and(children_of(db).opt()))
        .fold([0i64; 2], |a, ((t, _), _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = ranked(drain(&s), |&(_, a)| Reverse(a[0] - a[1]), false);
    type R = (Id<Post>, [i64; 2], i64);
    let top = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|((p, a), r)| (p, a, r)).collect::<Vec<R>>());
    let tp: MatSet<Id<Post>> = (&top).map(|(p, _, _)| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ac = (&tp).group_by(Ident::<Post>::new()).select(children_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&top).select(Same::<R>::new().and(Same::<R>::new().map(|(p, _, _): R| p).select((&cc).and(&ac).and(owner_user)))));
    rows(v.into_iter().map(|(_, ((p, a, _), ((c, n), u)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c), V::I(n)]);
        f.extend(ucols(db, u, &["name", "rep"]));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes,
//        SUM(CASE WHEN B.Id IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount, MAX(U.CreationDate) AS LastJoined
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId AND V.UserId = U.Id LEFT JOIN Badges B ON U.Id = B.UserId
//     WHERE U.Reputation > 1000 GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, AnswerCount, QuestionCount, Upvotes, Downvotes, BadgeCount, LastJoined, RANK() OVER (ORDER BY PostCount DESC) AS Rank FROM UserActivity)
// SELECT UserId, DisplayName, PostCount, AnswerCount, QuestionCount, Upvotes, Downvotes, BadgeCount, LastJoined FROM TopUsers WHERE Rank <= 10 ORDER BY Rank;
//
// Rank reads only the distinct post count, so the users are picked on that first and the product is driven for those alone.
// P is the user's own post, so `V.UserId = U.Id` is a vote cast by the post's owner (own_votes).
fn q7980(db: &'static So) -> String {
    let pc = db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let tu = ranked(drain(&pc), |&(_, n)| Reverse(n), false);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let own = own_votes(db);
    let ua = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and((&own).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 5], |a, (p, b)| {
            let (t, v) = p.map_or((0, None), |x| x);
            [a[0] + (t == 2) as i64, a[1] + (t == 1) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64, a[4] + b.is_some() as i64]
        });
    rows(drain((&ua).and(&pc)).into_iter().map(|(u, (a, n))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.push(user_col(db, u, "ucreated"));
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN p.PostTypeId = 2 AND p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswerCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, AcceptedAnswerCount, RANK() OVER (ORDER BY PostCount DESC) AS PostRank,
//        RANK() OVER (ORDER BY AcceptedAnswerCount DESC) AS AcceptedAnswerRank FROM UserPostStats)
// SELECT tu.DisplayName, tu.PostCount, tu.QuestionCount, tu.AnswerCount, tu.AcceptedAnswerCount, CASE WHEN tu.PostRank <= 10 THEN 'Top Contributors' ELSE 'Other Contributors' END AS ContributorType,
//        COALESCE(b.Name, 'No Badge') AS TopBadge
// FROM TopUsers tu LEFT JOIN Badges b ON tu.UserId = b.UserId AND b.Class = 1 WHERE tu.QuestionCount > 0 ORDER BY tu.PostCount DESC, tu.AcceptedAnswerCount DESC;
fn q9267(db: &'static So) -> String {
    let Post { post_type_id, accepted_answer_id, .. } = &db.post;
    let us = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(accepted_answer_id.opt())).opt()).fold([0i64; 4], |a, p| match p {
        Some((t, acc)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (t == 2 && acc.is_some()) as i64],
        None => a,
    });
    let v = ranked(drain(&us), |&(_, a)| Reverse(a[0]), false);
    let t = rel(v.into_iter().map(|((u, a), r)| (u, a, r)).collect());
    type R = (Id<User>, [i64; 4], i64);
    let gold = badges_of(db).select(Ident::<Badge>::new().with((&db.badge.class).eq(1)));
    rows(drain((&t).filt(|(_, a, _): R| a[1] > 0).select(Same::<R>::new().and(Same::<R>::new().map(|(u, _, _): R| u).select(gold.opt())))).into_iter().map(|(_, ((u, a, r), b))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(a.map(V::I));
        f.push(V::S(if r <= 10 { "Top Contributors" } else { "Other Contributors" }));
        f.push(V::S(b.map_or("No Badge", |b| db.badge.name.get(b).unwrap())));
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS QuestionCount, COALESCE(SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS AnswerCount,
//        SUM(COALESCE(c.Score, 0)) AS TotalCommentScore, SUM(COALESCE(b.Class, 0)) AS TotalBadges, SUM(CASE WHEN v.UserId = u.Id AND v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON p.Id = v.PostId
//     GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, QuestionCount, AnswerCount, TotalCommentScore, TotalBadges, TotalUpvotes, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank
//     FROM UserStats WHERE QuestionCount > 0)
// SELECT UserId, DisplayName, Reputation, QuestionCount, AnswerCount, TotalCommentScore, TotalBadges, TotalUpvotes, ReputationRank FROM TopUsers WHERE ReputationRank <= 10 ORDER BY ReputationRank;
//
// ReputationRank reads only Reputation among users with a post, so those users are picked first and the product is driven for them alone.
fn q25347(db: &'static So) -> String {
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let tu = ranked(drain((&pc).filt(|n| n > 0)), |&(u, _)| Reverse(db.user.reputation.get(u).unwrap()), false);
    let tu = rel(tu.into_iter().take_while(|x| x.1 <= 10).map(|((u, _), r)| (u, r)).collect());
    let rank: HashIdx<Id<User>, (Id<User>, i64)> = (&tu).map(|(u, _)| u).inv().select(&tu).collect();
    let cand: MatSet<Id<User>> = (&tu).map(|(u, _)| u).collect();
    let Vote { user, vote_type_id, .. } = &db.vote;
    let us = (&cand)
        .group_by(Ident::<User>::new())
        .select(
            Ident::<User>::new().and(
                posts_of(db)
                    .select((&db.post.post_type_id).and(comments_of(db).select(&db.comment.score).opt()).and(votes_of(db).select(user.opt().and(vote_type_id)).opt()))
                    .opt()
                    .and(badges_of(db).select(&db.badge.class).opt()),
            ),
        )
        .fold([0i64; 4], |a, (u, (p, b))| {
            let (t, c, v) = p.map_or((0, 0, None), |((t, c), v)| (t, c.unwrap_or(0), v));
            [a[0] + (t == 2) as i64, a[1] + c, a[2] + b.unwrap_or(0), a[3] + (v == Some((Some(u), 2))) as i64]
        });
    rows(drain((&us).and(&pc).and(&rank)).into_iter().map(|(u, ((a, n), (_, r)))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH PostStats AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.OwnerUserId, U.DisplayName AS OwnerDisplayName, COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpvoteCount,
//        COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownvoteCount, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
//        COALESCE(SUM(CASE WHEN P.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END), 0) AS AcceptedAnswerCount
//     FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Users U ON P.OwnerUserId = U.Id
//     WHERE P.PostTypeId = 1 GROUP BY P.Id, P.Title, P.CreationDate, P.OwnerUserId, U.DisplayName),
// TopPosts AS (SELECT PS.PostId, PS.Title, PS.CreationDate, PS.OwnerDisplayName, PS.UpvoteCount, PS.DownvoteCount, PS.CommentCount, PS.AcceptedAnswerCount,
//        ROW_NUMBER() OVER (ORDER BY PS.UpvoteCount DESC) AS Rank FROM PostStats PS)
// SELECT T.Title, T.OwnerDisplayName, T.UpvoteCount, T.DownvoteCount, T.CommentCount, T.AcceptedAnswerCount FROM TopPosts T WHERE T.Rank <= 10 ORDER BY T.UpvoteCount DESC;
fn q5426(db: &'static So) -> String {
    let Post { post_type_id, accepted_answer_id, .. } = &db.post;
    let ps = db
        .post
        .with(post_type_id.eq(1))
        .group_by(Ident::<Post>::new())
        .select(accepted_answer_id.opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(comments_of(db).opt()))
        .fold([0i64; 4], |a, ((acc, t), c)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64, a[3] + acc.is_some() as i64]);
    let v = top_n(drain(&ps), |&(p, a)| (Reverse(a[0]), p), 10);
    rows(v.into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["title", "owner"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN p.PostTypeId = 1 AND p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswerCount,
//        AVG(COALESCE(p.Score, 0)) AS AvgPostScore, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews, SUM(COALESCE(c.CommentCount, 0)) AS TotalComments
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN (SELECT PostId, COUNT(Id) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, QuestionCount, AnswerCount, AcceptedAnswerCount, AvgPostScore, TotalViews, TotalComments,
//        ROW_NUMBER() OVER (ORDER BY TotalPosts DESC, TotalViews DESC) AS Rank FROM UserStats)
// SELECT Rank, DisplayName, TotalPosts, QuestionCount, AnswerCount, AcceptedAnswerCount, AvgPostScore, TotalViews, TotalComments FROM TopUsers WHERE Rank <= 10 ORDER BY Rank;
fn q9284(db: &'static So) -> String {
    let Post { post_type_id, accepted_answer_id, score, view_count, .. } = &db.post;
    let cc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(accepted_answer_id.opt()).and(score).and(view_count.opt()).and((&cc).opt())).opt())
        .fold([0i64; 7], |a, p| match p {
            Some(((((t, acc), s), w), c)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (t == 1 && acc.is_some()) as i64, a[4] + s, a[5] + w.unwrap_or(0), a[6] + c.unwrap_or(0)],
            None => a,
        });
    let v = top_n(drain(&us), |&(u, a)| (Reverse(a[0]), Reverse(a[5]), u), 10);
    rows(v.into_iter().enumerate().map(|(i, (u, a))| {
        row(vec![V::I(i as i64 + 1), user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), avg(a[4], a[0]), V::I(a[5]), V::I(a[6])])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, u.DisplayName AS Author, COUNT(DISTINCT c.Id) AS TotalComments, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.PostTypeId IN (1, 2) GROUP BY p.Id, p.Title, p.CreationDate, p.Score, u.DisplayName, p.PostTypeId),
// TopPosts AS (SELECT PostId, Title, CreationDate, Author, TotalComments, UpVotes, DownVotes, Rank FROM RankedPosts WHERE Rank <= 10)
// SELECT t.PostId, t.Title, t.CreationDate, t.Author, t.TotalComments, t.UpVotes, t.DownVotes, (t.UpVotes - t.DownVotes) AS NetScore FROM TopPosts t ORDER BY NetScore DESC, t.CreationDate DESC;
//
// Rank reads only base columns, so the posts are picked first and the comment x vote product is driven for those alone.
fn q9601(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.is_in([1, 2]))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    rows(drain((&s).and(&cc)).into_iter().map(|(p, (a, c))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1]), V::I(a[0] - a[1])]);
        row(f)
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("3782", q3782),
    ("5813", q5813),
    ("6953", q6953),
    ("8926", q8926),
    ("8150", q8150),
    ("6079", q6079),
    ("7388", q7388),
    ("7087", q7087),
    ("8448", q8448),
    ("29445", q29445),
    ("7604", q7604),
    ("2831", q2831),
    ("8068", q8068),
    ("1307", q1307),
    ("6774", q6774),
    ("4076", q4076),
    ("14468", q14468),
    ("27392", q27392),
    ("28561", q28561),
    ("4474", q4474),
    ("5003", q5003),
    ("7964", q7964),
    ("5535", q5535),
    ("670", q670),
    ("9191", q9191),
    ("3053", q3053),
    ("23902", q23902),
    ("2931", q2931),
    ("7960", q7960),
    ("12898", q12898),
    ("2783", q2783),
    ("5988", q5988),
    ("8733", q8733),
    ("12309", q12309),
    ("9478", q9478),
    ("9501", q9501),
    ("8842", q8842),
    ("1655", q1655),
    ("6445", q6445),
    ("6982", q6982),
    ("25346", q25346),
    ("816", q816),
    ("9847", q9847),
    ("8098", q8098),
    ("5485", q5485),
    ("6623", q6623),
    ("7703", q7703),
    ("22656", q22656),
    ("4881", q4881),
    ("6654", q6654),
    ("729", q729),
    ("481", q481),
    ("6955", q6955),
    ("33639", q33639),
    ("5367", q5367),
    ("5575", q5575),
    ("5063", q5063),
    ("1040", q1040),
    ("2656", q2656),
    ("7814", q7814),
    ("235", q235),
    ("8861", q8861),
    ("6336", q6336),
    ("7296", q7296),
    ("9374", q9374),
    ("134", q134),
    ("678", q678),
    ("1134", q1134),
    ("6138", q6138),
    ("6660", q6660),
    ("3643", q3643),
    ("5756", q5756),
    ("7003", q7003),
    ("5234", q5234),
    ("9613", q9613),
    ("5301", q5301),
    ("5619", q5619),
    ("7371", q7371),
    ("4298", q4298),
    ("9236", q9236),
    ("9503", q9503),
    ("3808", q3808),
    ("7182", q7182),
    ("4484", q4484),
    ("7106", q7106),
    ("5325", q5325),
    ("9880", q9880),
    ("5216", q5216),
    ("8547", q8547),
    ("7980", q7980),
    ("9267", q9267),
    ("25347", q25347),
    ("5426", q5426),
    ("9284", q9284),
    ("9601", q9601),
];
