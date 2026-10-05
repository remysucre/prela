use harness::prelude::*;
use std::cmp::Reverse;

fn by_first<A: Copy + Eq + std::hash::Hash, B: Copy + Eq + std::hash::Hash>(m: &MatSet<(A, B)>) -> HashIdx<A, B> {
    m.map(|(a, _)| a).inv().map(|(_, b): (A, B)| b).collect()
}

// WITH UserVoteStatistics AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(V.Id) AS TotalVotes, SUM(CASE WHEN V.VoteTypeId IN (2, 8) THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, AVG(COALESCE(P.Score, 0)) AS AvgPostScore
//     FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId LEFT JOIN Posts P ON V.PostId = P.Id GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalVotes, UpVotes, DownVotes, AvgPostScore, RANK() OVER (ORDER BY TotalVotes DESC) AS UserRank FROM UserVoteStatistics),
// RecentPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.OwnerUserId, COUNT(C.Id) AS CommentCount, COUNT(DISTINCT PL.RelatedPostId) AS RelatedPostsCount
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN PostLinks PL ON P.Id = PL.PostId
//     WHERE P.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' GROUP BY P.Id, P.Title, P.CreationDate, P.OwnerUserId)
// SELECT U.DisplayName, U.UpVotes, U.DownVotes, U.TotalVotes, R.PostId, R.Title, R.CreationDate, R.CommentCount, R.RelatedPostsCount, U.AvgPostScore
// FROM TopUsers U JOIN RecentPosts R ON U.UserId = R.OwnerUserId WHERE U.UserRank <= 10 ORDER BY U.TotalVotes DESC, R.CreationDate DESC;
fn q3056(db: &'static So) -> String {
    let Vote { vote_type_id, post, .. } = &db.vote;
    let Post { creation_date, owner_user, .. } = &db.post;
    let uv = db
        .user
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(vote_type_id.and(post.select(&db.post.score).opt())).opt())
        .fold([0i64; 5], |a, x| match x {
            Some((t, s)) => [a[0] + 1, a[1] + matches!(t, 2 | 8) as i64, a[2] + (t == 3) as i64, a[3] + s.unwrap_or(0), a[4] + 1],
            None => [a[0], a[1], a[2], a[3], a[4] + 1],
        });
    let w = whole(&uv).select(Ident::<User>::new().and(&uv)).window(rank, |(_, a): (Id<User>, [i64; 5])| Reverse(a[0]), asc);
    let tu: MatSet<Id<User>> = (&w).filt(|(_, r)| r <= 10).map(|((u, _), _)| u).collect();
    let recent = || db.post.with(creation_date.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).with(owner_user.select(Ident::<User>::new().with(&tu)));
    let cc = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(links_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let rc = recent().group_by(Ident::<Post>::new()).select(links_of(db).select(&db.post_link.related_post_id)).count_distinct();
    let v = drain((&cc).and((&rc).opt()).and(owner_user.select(Ident::<User>::new().and(&uv))));
    rows(v.into_iter().map(|(p, ((c, r), (u, a)))| {
        let r = r.unwrap_or(0);
        let mut f = vec![user_col(db, u, "name"), V::I(a[1]), V::I(a[2]), V::I(a[0])];
        f.extend(post_fields(db, p, &["id", "title", "created"]));
        f.extend([V::I(c), V::I(r), avg(a[3], a[4])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.CreationDate, p.Score, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COALESCE(SUM(CASE WHEN c.Id IS NOT NULL THEN 1 ELSE 0 END), 0) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.Id, p.OwnerUserId, p.Title, p.CreationDate, p.Score),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(rp.PostId) AS PostCount, SUM(rp.UpVotes) AS TotalUpVotes, SUM(rp.DownVotes) AS TotalDownVotes,
//        SUM(rp.CommentCount) AS TotalComments FROM Users u LEFT JOIN RankedPosts rp ON u.Id = rp.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation)
// SELECT us.UserId, us.DisplayName, us.Reputation, us.PostCount, us.TotalUpVotes, us.TotalDownVotes, us.TotalComments
// FROM UserStats us WHERE us.Reputation > 100 ORDER BY us.Reputation DESC, us.PostCount DESC LIMIT 10;
fn q9276(db: &'static So) -> String {
    let rp = db
        .post
        .with((&db.post.creation_date).ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()))
        .fold([0i64; 3], |a, (t, c)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64]);
    let us = db
        .user
        .with((&db.user.reputation).gt(100))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(&rp).opt())
        .fold([0i64; 4], |a, x| match x {
            Some(r) => [a[0] + 1, a[1] + r[0], a[2] + r[1], a[3] + r[2]],
            None => a,
        });
    let v = top_n(drain(&us), |&(u, a)| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(a[0]), u), 10);
    rows(v.into_iter().map(|(u, a)| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[0]), nullable(a[1], a[0]), nullable(a[2], a[0]), nullable(a[3], a[0])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank FROM Posts p WHERE p.PostTypeId = 1 AND p.Score IS NOT NULL),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c GROUP BY c.PostId),
// UserScores AS (SELECT u.Id AS UserId, SUM(u.UpVotes) - SUM(u.DownVotes) AS NetVotes FROM Users u GROUP BY u.Id),
// ClosedPosts AS (SELECT ph.PostId FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 AND ph.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year')
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, COALESCE(pc.CommentCount, 0) AS CommentCount, COALESCE(us.NetVotes, 0) AS UserNetVotes,
//        CASE WHEN cp.PostId IS NOT NULL THEN 'Closed' ELSE 'Open' END AS PostStatus
// FROM RankedPosts rp LEFT JOIN PostComments pc ON rp.PostId = pc.PostId LEFT JOIN Users u ON rp.OwnerUserId = u.Id LEFT JOIN UserScores us ON u.Id = us.UserId
// LEFT JOIN ClosedPosts cp ON rp.PostId = cp.PostId WHERE rp.Rank <= 5 ORDER BY rp.CreationDate DESC;
fn q1021(db: &'static So) -> String {
    let Post { post_type_id, owner_user, owner_user_id, creation_date, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1)).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d): (Id<Post>, i64)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let PostHistory { post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let closed = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.eq(10)).with(hd.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))));
    let nv = Ident::<User>::new().and(&db.user.up_votes).and(&db.user.down_votes).map(|((_, a), b)| a - b);
    let v = drain((&cc).and(owner_user.select(nv).opt()).and(closed.opt()));
    rows(v.into_iter().map(|(p, ((c, n), h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(c), V::I(n.unwrap_or(0)), V::S(if h.is_some() { "Closed" } else { "Open" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.AnswerCount, p.Score, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank,
//        COALESCE(b.Name, 'No Badge') AS UserBadge
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Badges b ON u.Id = b.UserId AND b.Class = 1
//     WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year' AND p.AnswerCount > 0 AND p.Score > 10),
// RecentVotes AS (SELECT v.PostId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotesCount, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotesCount
//     FROM Votes v WHERE v.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 month' GROUP BY v.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.AnswerCount, rp.Score, rp.UserBadge, rv.UpVotesCount, rv.DownVotesCount,
//        (COALESCE(rv.UpVotesCount, 0) - COALESCE(rv.DownVotesCount, 0)) AS NetVotes, CASE WHEN rp.Rank <= 5 THEN 'Top Post' ELSE 'Regular Post' END AS PostCategory
// FROM RankedPosts rp LEFT JOIN RecentVotes rv ON rp.PostId = rv.PostId WHERE (rp.UserBadge IS NOT NULL OR rp.Score > 100) ORDER BY rp.Score DESC, rp.CreationDate DESC LIMIT 50;
//
// The ROW_NUMBER numbers the post x gold-badge rows; rows of one post tie on Score and are ordered here by badge id. Every tied row prints the same columns apart
// from PostCategory, so only how many of them rank <= 5 is observable, and that does not depend on the order.
fn q1463(db: &'static So) -> String {
    let Post { post_type_id, creation_date, answer_count, score, owner_user, .. } = &db.post;
    let gold = badges_of(db).select(Ident::<Badge>::new().with((&db.badge.class).eq(1)));
    let base = db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1))).with(answer_count.gt(0)).with(score.gt(10));
    let w = base
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(owner_user.select(gold).opt()))
        .window(row_number, |((p, s), b): ((Id<Post>, i64), Option<Id<Badge>>)| (Reverse(s), p, b), asc);
    let rk: MatSet<(Id<Post>, (Option<Id<Badge>>, i64))> = (&w).map(|(((p, _), b), r)| (p, (b, r))).collect();
    let Vote { creation_date: vd, vote_type_id, post, .. } = &db.vote;
    let rv = db.vote.with(vd.ge(add_months(date(2024, 10, 1), -1))).group_by(post).select(vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let v = drain(by_first(&rk).and((&rv).opt()));
    let v = top_n(v, |&(p, ((b, _), _))| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p, b), 50);
    rows(v.into_iter().map(|(p, ((b, r), x))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "answers", "score"]);
        f.push(V::S(b.map_or("No Badge", |b| db.badge.name.get(b).unwrap())));
        f.extend(match x {
            Some(a) => [V::I(a[0]), V::I(a[1]), V::I(a[0] - a[1])],
            None => [V::Null, V::Null, V::I(0)],
        });
        f.push(V::S(if r <= 5 { "Top Post" } else { "Regular Post" }));
        row(f)
    }))
}

// WITH TagStats AS (SELECT t.TagName, COUNT(DISTINCT p.Id) AS PostCount, SUM(COALESCE(v.UpVoteCount, 0)) AS TotalUpVotes, SUM(COALESCE(v.DownVoteCount, 0)) AS TotalDownVotes,
//        AVG(p.ViewCount) AS AverageViews, COUNT(DISTINCT c.Id) AS CommentCount
//     FROM Tags t LEFT JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%'
//     LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
//                FROM Votes GROUP BY PostId) v ON p.Id = v.PostId
//     LEFT JOIN Comments c ON p.Id = c.PostId WHERE t.TagName IS NOT NULL GROUP BY t.TagName),
// PopularTags AS (SELECT TagName, PostCount, TotalUpVotes - TotalDownVotes AS NetVotes, AverageViews, CommentCount, RANK() OVER (ORDER BY TotalUpVotes DESC) AS RankByUpVotes,
//        RANK() OVER (ORDER BY PostCount DESC) AS RankByPosts FROM TagStats)
// SELECT TagName, PostCount, NetVotes, AverageViews, CommentCount, CASE WHEN RankByUpVotes <= 10 THEN 'Top 10 by UpVotes' ELSE 'Not Top 10 by UpVotes' END AS UpVoteRanking,
//        CASE WHEN RankByPosts <= 10 THEN 'Top 10 by Posts' ELSE 'Not Top 10 by Posts' END AS PostRanking
// FROM PopularTags WHERE PostCount > 0 ORDER BY NetVotes DESC, PostCount DESC;
//
// COUNT(DISTINCT p.Id) is a fold over one row per (tag, post) pair; COUNT(DISTINCT c.Id) counts the product's comment rows, each comment being on one post.
fn q28881(db: &'static So) -> String {
    let lt = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&lt).map(|(_, t)| t).inv().collect();
    let va = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let tp = || (&by_tag).map(|(p, _)| p);
    let ts = db
        .tag
        .group_by(Ident::<Tag>::new())
        .select(tp().select((&va).opt().and((&db.post.view_count).opt()).and(comments_of(db).opt())).opt())
        .fold([0i64; 5], |a, x| match x {
            Some(((v, w), c)) => {
                let v = v.unwrap_or([0, 0]);
                [a[0] + v[0], a[1] + v[1], a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + c.is_some() as i64]
            }
            None => a,
        });
    let pc = db.tag.group_by(Ident::<Tag>::new()).select(tp().opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let w = whole(&ts).select(Ident::<Tag>::new().and(&ts).and(&pc)).window(rank, |((_, a), _): ((Id<Tag>, [i64; 5]), i64)| Reverse(a[0]), asc);
    let w = (&w).window(rank, |(((_, _), n), _): (((Id<Tag>, [i64; 5]), i64), i64)| Reverse(n), asc);
    let v = drain((&w).filt(|((((_, _), n), _), _)| n > 0));
    rows(v.into_iter().map(|(_, ((((t, a), n), ru), rp))| {
        row(vec![
            V::S(db.tag.tag_name.get(t).unwrap()),
            V::I(n),
            V::I(a[0] - a[1]),
            avg(a[3], a[2]),
            V::I(a[4]),
            V::S(if ru <= 10 { "Top 10 by UpVotes" } else { "Not Top 10 by UpVotes" }),
            V::S(if rp <= 10 { "Top 10 by Posts" } else { "Not Top 10 by Posts" }),
        ])
    }))
}

// WITH UserAggregates AS (SELECT U.Id as UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.Score > 0 THEN P.Score ELSE 0 END) AS PositiveScore,
//        COUNT(DISTINCT B.Id) AS BadgeCount, MAX(P.CreationDate) AS LastPostDate
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// RecentPostHistory AS (SELECT PH.UserId, PH.PostId, PH.CreationDate, P.Title, P.Score, PH.UserDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY PH.UserId ORDER BY PH.CreationDate DESC) AS RecentActionRank
//     FROM PostHistory PH JOIN Posts P ON PH.PostId = P.Id WHERE PH.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopUsers AS (SELECT UA.UserId, UA.DisplayName, UA.Reputation, UA.PostCount, UA.PositiveScore, UA.BadgeCount, UA.LastPostDate,
//        ROW_NUMBER() OVER (ORDER BY UA.Reputation DESC) AS Rank FROM UserAggregates UA WHERE UA.PostCount > 10)
// SELECT U.*, COALESCE(RPH.Title, 'No Recent Activity') AS RecentActivityTitle, COALESCE(RPH.Score, 0) AS RecentActivityScore
// FROM TopUsers U LEFT JOIN RecentPostHistory RPH ON U.UserId = RPH.UserId AND RPH.RecentActionRank = 1 WHERE U.Rank <= 10 ORDER BY U.Reputation DESC;
//
// Rank reads only Reputation and the distinct post count, so the ten users are picked first and the posts x badges product is driven for them alone.
// RecentActionRank ties on CreationDate are broken by the history id.
fn q3247(db: &'static So) -> String {
    let Post { score, creation_date, title, .. } = &db.post;
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let w = whole((&pc).filt(|n| n > 10)).select(Ident::<User>::new().and(&db.user.reputation)).window(row_number, |(u, r): (Id<User>, i64)| (Reverse(r), u), asc);
    let rk: MatSet<(Id<User>, i64)> = (&w).filt(|(_, r)| r <= 10).map(|((u, _), r)| (u, r)).collect();
    let rank = by_first(&rk);
    let tus: MatSet<Id<User>> = (&rk).map(|(u, _)| u).collect();
    let ua = (&tus)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score.and(creation_date)).opt().and(badges_of(db).opt()))
        .fold((0i64, i64::MIN), |(s, m), (p, _)| match p {
            Some((sc, d)) => (s + sc.max(0), m.max(d)),
            None => (s, m),
        });
    let bc = (&tus).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let PostHistory { user, creation_date: hd, post, .. } = &db.post_history;
    let rh = db
        .post_history
        .with(hd.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(user.select(Ident::<User>::new().with(&tus)))
        .group_by(user)
        .select(Ident::<PostHistory>::new().and(hd))
        .window(row_number, |(h, d): (Id<PostHistory>, i64)| (Reverse(d), h), asc);
    let last = (&rh).filt(|(_, r)| r == 1).map(|((h, _), _)| h);
    let v = drain((&rank).and(&pc).and(&ua).and(&bc).and(last.select(post).opt()));
    rows(v.into_iter().map(|(u, ((((r, n), (s, m)), b), p))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(n), V::I(s), V::I(b), tmax(m), V::I(r)]);
        f.push(V::S(p.and_then(|p| title.get(p)).unwrap_or("No Recent Activity")));
        f.push(V::I(p.map_or(0, |p| score.get(p).unwrap())));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.CreationDate, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COUNT(DISTINCT c.Id) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY COALESCE(SUM(v.VoteTypeId), 0) DESC) AS UserPostRank
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.OwnerUserId, p.CreationDate),
// TopUsers AS (SELECT u.Id, u.DisplayName, RANK() OVER (ORDER BY SUM(p.UpVotes - p.DownVotes) DESC) AS UserRank
//     FROM Users u JOIN RankedPosts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName)
// SELECT tu.UserRank, tu.DisplayName, rp.Title, rp.UpVotes, rp.DownVotes, rp.CommentCount,
//        CASE WHEN rp.UpVotes - rp.DownVotes > 0 THEN 'Positive' WHEN rp.UpVotes - rp.DownVotes < 0 THEN 'Negative' ELSE 'Neutral' END AS VoteSentiment
// FROM TopUsers tu INNER JOIN RankedPosts rp ON tu.Id = rp.OwnerUserId WHERE tu.UserRank <= 10 ORDER BY tu.UserRank, rp.UpVotes DESC;
//
// UserPostRank is never read, so it is not computed.
fn q831(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let base = || db.post.with(post_type_id.eq(1)).with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let rp = base()
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()))
        .fold([0i64; 2], |a, (t, _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = base().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let tu = base().group_by(owner_user).select(&rp).fold(0i64, |n, a| n + a[0] - a[1]);
    let w = whole(&tu).select(Ident::<User>::new().and(&tu)).window(rank, |(_, n): (Id<User>, i64)| Reverse(n), asc);
    let rk: MatSet<(Id<User>, i64)> = (&w).filt(|(_, r)| r <= 10).map(|((u, _), r)| (u, r)).collect();
    let rank = by_first(&rk);
    let v = drain((&rp).and(&cc).and(owner_user.select(Ident::<User>::new().and(&rank))));
    rows(v.into_iter().map(|(p, ((a, c), (u, r)))| {
        let n = a[0] - a[1];
        let mut f = vec![V::I(r), user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c), V::S(if n > 0 { "Positive" } else if n < 0 { "Negative" } else { "Neutral" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS ScoreRank, COUNT(c.Id) AS CommentCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.Score, p.PostTypeId),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, ROW_NUMBER() OVER (ORDER BY u.Reputation DESC) AS ReputationRank FROM Users u),
// PostVoteSummary AS (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesCount
//     FROM Votes v GROUP BY v.PostId)
// SELECT rp.PostId, rp.Title, rp.Score, rp.ScoreRank, ru.UserId, ru.Reputation, ru.ReputationRank, COALESCE(pvs.UpVotesCount, 0) AS UpVotes, COALESCE(pvs.DownVotesCount, 0) AS DownVotes,
//        CASE WHEN rp.CommentCount IS NULL THEN 'No Comments' ELSE CONCAT(rp.CommentCount, ' Comments') END AS CommentInformation
// FROM RankedPosts rp JOIN UserReputation ru ON ru.UserId = rp.PostId LEFT JOIN PostVoteSummary pvs ON pvs.PostId = rp.PostId
// WHERE rp.ScoreRank <= 5 ORDER BY rp.Score DESC, ru.ReputationRank LIMIT 25;
//
// `ru.UserId = rp.PostId` joins a user id to a post id, so it goes through the raw ids. ScoreRank reads only Score, so the top posts are picked before the comment join.
fn q952(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, origid, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(post_type_id).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s): (Id<Post>, i64)| (Reverse(s), p), asc);
    let tr: MatSet<(Id<Post>, i64)> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), r)| (p, r)).collect();
    let sr = by_first(&tr);
    let uw = whole(&db.user.reputation).select(Ident::<User>::new().and(&db.user.reputation)).window(row_number, |(u, r): (Id<User>, i64)| (Reverse(r), u), asc);
    let ur: MatSet<(Id<User>, i64)> = (&uw).map(|((u, _), r)| (u, r)).collect();
    let urank = by_first(&ur);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let tps: MatSet<Id<Post>> = (&tr).map(|(p, _)| p).collect();
    let cc = (&tps).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let pvs = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let v = drain((&sr).and(&cc).and(origid.select(&uidx).select(Ident::<User>::new().and(&urank))).and((&pvs).opt()));
    let v = top_n(v, |&(p, ((_, (u, r)), _))| (Reverse(score.get(p).unwrap()), r, p, u), 25);
    rows(v.into_iter().map(|(p, (((sr, c), (u, r)), a))| {
        let mut f = post_fields(db, p, &["id", "title", "score"]);
        f.push(V::I(sr));
        f.extend(ucols(db, u, &["uid", "rep"]));
        let a = a.unwrap_or([0, 0]);
        f.extend([V::I(r), V::I(a[0]), V::I(a[1]), V::Owned(format!("{c} Comments"))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS ScoreRank
//     FROM Posts p WHERE p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// PostDetails AS (SELECT rp.PostId, rp.Title, rp.Score, COALESCE(ph.UserDisplayName, 'Unknown') AS LastEditor, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
//     FROM RankedPosts rp LEFT JOIN Posts p ON rp.PostId = p.Id LEFT JOIN PostHistory ph ON p.LastEditorUserId = ph.UserId LEFT JOIN Comments c ON p.Id = c.PostId
//     LEFT JOIN Votes v ON p.Id = v.PostId WHERE rp.ScoreRank <= 5 GROUP BY rp.PostId, rp.Title, rp.Score, ph.UserDisplayName)
// SELECT pd.PostId, pd.Title, pd.Score, pd.LastEditor, pd.CommentCount, pd.UpVoteCount, pd.DownVoteCount, (pd.UpVoteCount - pd.DownVoteCount) AS NetVotes,
//        CASE WHEN pd.Score > 10 THEN 'High' WHEN pd.Score BETWEEN 1 AND 10 THEN 'Medium' ELSE 'Low' END AS Popularity
// FROM PostDetails pd WHERE pd.CommentCount > 0 AND pd.LastEditor IS NOT NULL ORDER BY pd.Score DESC;
//
// The GROUP BY names ph.UserDisplayName, a column of the joined history rows, so the (post, history row) pairs are materialised and grouped (see limitations.md).
fn q4904(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, last_editor_user_id, .. } = &db.post;
    let w = db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(post_type_id).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s): (Id<Post>, i64)| (Reverse(s), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let by_user: HashIdx<i64, Id<PostHistory>> = (&db.post_history.user_id).inv().collect();
    let hname = Ident::<PostHistory>::new().and((&db.post_history.user_display_name).opt());
    type J = (Id<Post>, Option<(Id<PostHistory>, Option<Str>)>);
    let j: MatSet<J> = (&tp).select(Ident::<Post>::new().and(last_editor_user_id.select(&by_user).select(hname).opt())).collect();
    let pd = (&j)
        .group_by(Same::<J>::new().map(|(p, h): J| (p, h.and_then(|h| h.1))))
        .select(Same::<J>::new().map(|(p, _): J| p).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = drain((&pd).filt(|a| a[0] > 0));
    rows(v.into_iter().map(|((p, n), a)| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "score"]);
        f.push(V::S(n.unwrap_or("Unknown")));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[1] - a[2]), V::S(if s > 10 { "High" } else if s >= 1 { "Medium" } else { "Low" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(p.Score) AS TotalScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// PostHistorySummary AS (SELECT ph.UserId, COUNT(ph.Id) AS EditCount, MIN(ph.CreationDate) AS FirstEditDate, MAX(ph.CreationDate) AS LastEditDate
//     FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (4, 5, 6) GROUP BY ph.UserId)
// SELECT u.DisplayName, COALESCE(us.PostCount, 0) AS TotalPosts, COALESCE(us.TotalScore, 0) AS AggregateScore, COALESCE(phs.EditCount, 0) AS TotalEdits, phs.FirstEditDate,
//        phs.LastEditDate, rp.Title AS LatestPostTitle
// FROM Users u LEFT JOIN UserStats us ON u.Id = us.UserId LEFT JOIN PostHistorySummary phs ON u.Id = phs.UserId LEFT JOIN RankedPosts rp ON u.Id = rp.PostId
// WHERE u.Reputation > 1000 AND (phs.LastEditDate IS NULL OR phs.LastEditDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days')
// ORDER BY AggregateScore DESC, TotalPosts DESC LIMIT 100;
//
// `u.Id = rp.PostId` joins a user id to a post id, so it goes through the raw ids. PostRank is never read.
fn q3359(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let pidx: HashIdx<i64, Id<Post>> = db.post.with((&db.post.creation_date).ge(add_years(t0, -1))).select(&db.post.origid).inv().collect();
    let us = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(&db.post.score).opt()).fold([0i64; 2], |a, s| match s {
        Some(s) => [a[0] + 1, a[1] + s],
        None => a,
    });
    let PostHistory { user, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let phs = db.post_history.with(post_history_type_id.is_in([4, 5, 6])).group_by(user).select(hd).fold((0i64, i64::MAX, i64::MIN), |(n, lo, hi), d| (n + 1, lo.min(d), hi.max(d)));
    let lim = add_days(t0, -30);
    let v = drain(
        db.user
            .with((&db.user.reputation).gt(1000))
            .select((&us).and((&phs).opt()).and((&db.user.origid).select(&pidx).opt()))
            .filt(move |((_, h), _): (([i64; 2], Option<(i64, i64, i64)>), Option<Id<Post>>)| h.map_or(true, |(_, _, hi)| hi >= lim)),
    );
    let v = top_n(v, |&(u, ((a, _), _))| (Reverse(a[1]), Reverse(a[0]), u), 100);
    rows(v.into_iter().map(|(u, ((a, h), p))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1])];
        f.extend(match h {
            Some((n, lo, hi)) => [V::I(n), V::T(lo), V::T(hi)],
            None => [V::I(0), V::Null, V::Null],
        });
        f.push(p.map_or(V::Null, |p| ostr(db.post.title.get(p))));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.Score, p.CreationDate, COALESCE(u.DisplayName, 'Community User') AS Author,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS PostRank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.PostTypeId IN (1, 2)),
// PostDetails AS (SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.CreationDate, rp.Author, ph.Comment, ph.UserDisplayName AS Editor, ph.CreationDate AS EditDate,
//        ph.Text AS EditContent FROM RankedPosts rp LEFT JOIN PostHistory ph ON rp.PostId = ph.PostId WHERE rp.PostRank <= 5),
// AggregatedPosts AS (SELECT pd.PostId, pd.Title, pd.Author, SUM(CASE WHEN pd.Comment IS NOT NULL THEN 1 ELSE 0 END) AS EditCount, AVG(pd.Score) AS AverageScore,
//        AVG(pd.ViewCount) AS AverageViewCount FROM PostDetails pd GROUP BY pd.PostId, pd.Title, pd.Author)
// SELECT ap.PostId, ap.Title, ap.Author, ap.EditCount, ap.AverageScore, ap.AverageViewCount FROM AggregatedPosts ap ORDER BY ap.AverageScore DESC, ap.AverageViewCount DESC;
fn q5928(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(post_type_id.is_in([1, 2]))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(row_number, |((p, s), w): ((Id<Post>, i64), Option<i64>)| (Reverse(s), w.is_none(), Reverse(w), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|(((p, _), _), _)| p).collect();
    let ap = (&tp)
        .group_by(Ident::<Post>::new())
        .select(score.and(view_count.opt()).and(history_of(db).select((&db.post_history.comment).opt()).opt()))
        .fold([0i64; 5], |a, ((s, w), h)| [a[0] + 1, a[1] + h.flatten().is_some() as i64, a[2] + s, a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0)]);
    let v = drain(&ap);
    rows(v.into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.push(V::S(db.post.owner_user.get(p).map_or("Community User", |u| db.user.display_name.get(u).unwrap())));
        f.extend([V::I(a[1]), avg(a[2], a[0]), avg(a[4], a[3])]);
        row(f)
    }))
}

// WITH UserPopularity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.ViewCount IS NOT NULL THEN P.ViewCount ELSE 0 END) AS TotalViews,
//        SUM(CASE WHEN P.Score IS NOT NULL THEN P.Score ELSE 0 END) AS TotalScore FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId WHERE U.Reputation > 0 GROUP BY U.Id, U.DisplayName),
// PostDetails AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, COALESCE(PV.TotalVotes, 0) AS VoteCount
//     FROM Posts P LEFT JOIN (SELECT PostId, COUNT(*) AS TotalVotes FROM Votes GROUP BY PostId) PV ON P.Id = PV.PostId),
// PopularUsers AS (SELECT UP.UserId, UP.DisplayName, UP.TotalPosts, UP.TotalViews, UP.TotalScore, ROW_NUMBER() OVER (ORDER BY UP.TotalScore DESC) AS Rank FROM UserPopularity UP WHERE UP.TotalPosts > 5)
// SELECT PU.DisplayName AS PopularUser, P.Title AS PostTitle, P.CreationDate AS PostCreationDate, P.Score AS PostScore, P.VoteCount AS PostVoteCount
// FROM PopularUsers PU JOIN PostDetails P ON PU.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = P.PostId LIMIT 1) WHERE PU.Rank <= 10 ORDER BY PU.TotalScore DESC, P.VoteCount DESC;
//
// The correlated subquery looks a post up by its primary key, so it is the post's own OwnerUserId.
fn q1927(db: &'static So) -> String {
    let Post { owner_user, score, view_count, .. } = &db.post;
    let up = db
        .user
        .with((&db.user.reputation).gt(0))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score.and(view_count.opt())).opt())
        .fold([0i64; 3], |a, p| match p {
            Some((s, w)) => [a[0] + 1, a[1] + w.unwrap_or(0), a[2] + s],
            None => a,
        });
    let tu = top_n(drain((&up).filt(|a| a[0] > 5)), |&(u, a)| (Reverse(a[2]), u), 10);
    let tus: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let vc = db.post.with(owner_user.select(Ident::<User>::new().with(&tus))).group_by(Ident::<Post>::new()).select(votes_of(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let v = drain((&vc).and(owner_user));
    rows(v.into_iter().map(|(p, (n, u))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title", "created", "score"]));
        f.push(V::I(n));
        row(f)
    }))
}

// WITH UserBadges AS (SELECT U.Id AS UserId, COUNT(B.Id) AS TotalBadges, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id),
// RecentPosts AS (SELECT P.Id AS PostId, P.OwnerUserId, P.Title, P.CreationDate, P.Score, COALESCE((SELECT COUNT(*) FROM Comments C WHERE C.PostId = P.Id), 0) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS RN FROM Posts P WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days')
// SELECT U.DisplayName, U.Reputation, UB.TotalBadges, UB.GoldBadges, UB.SilverBadges, UB.BronzeBadges, RP.Title AS RecentPostTitle, RP.CreationDate AS RecentPostDate,
//        RP.CommentCount AS RecentPostComments, CASE WHEN RP.Score IS NULL THEN 'No Score' ELSE CAST(RP.Score AS VARCHAR) END AS PostScore
// FROM Users U LEFT JOIN UserBadges UB ON U.Id = UB.UserId LEFT JOIN RecentPosts RP ON U.Id = RP.OwnerUserId AND RP.RN = 1
// WHERE U.Location IS NOT NULL OR U.WebsiteUrl IS NOT NULL ORDER BY U.Reputation DESC, RecentPostDate DESC NULLS LAST LIMIT 100;
fn q939(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let Post { owner_user, creation_date, score, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d): (Id<Post>, i64)| (Reverse(d), p), asc);
    let last = || (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p);
    let lp: MatSet<Id<Post>> = last().collect();
    let cc = (&lp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let listed = (&db.user.location).opt().and((&db.user.website_url).opt()).filt(|(l, w): (Option<Str>, Option<Str>)| l.is_some() || w.is_some());
    let v = drain(db.user.with(listed).select((&ub).and(last().select(Ident::<Post>::new().and(&cc)).opt())));
    let v = top_n(v, |&(u, (_, p))| (Reverse(db.user.reputation.get(u).unwrap()), p.is_none(), Reverse(p.map(|(p, _)| creation_date.get(p).unwrap())), u), 100);
    rows(v.into_iter().map(|(u, (b, p))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(b.map(V::I));
        f.extend(match p {
            Some((p, c)) => [ostr(db.post.title.get(p)), V::T(creation_date.get(p).unwrap()), V::I(c), V::Owned(score.get(p).unwrap().to_string())],
            None => [V::Null, V::Null, V::Null, V::S("No Score")],
        });
        row(f)
    }))
}

// WITH PostStats AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, COALESCE(COUNT(DISTINCT c.Id), 0) AS CommentCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
//        COALESCE(MAX(b.Class), 0) AS HighestBadgeClass, COALESCE(MAX(u.Reputation), 0) AS HighestReputation
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON p.OwnerUserId = b.UserId LEFT JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount),
// RankedPosts AS (SELECT ps.PostId, ps.Title, ps.CreationDate, ps.ViewCount, ps.CommentCount, ps.UpVotes, ps.DownVotes, ps.HighestBadgeClass, ps.HighestReputation,
//        RANK() OVER (ORDER BY ps.ViewCount DESC, ps.UpVotes - ps.DownVotes DESC) AS Rank FROM PostStats ps)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.CommentCount, rp.UpVotes, rp.DownVotes, rp.HighestBadgeClass, rp.HighestReputation
// FROM RankedPosts rp WHERE rp.Rank <= 10 ORDER BY rp.Rank;
fn q5754(db: &'static So) -> String {
    let Post { creation_date, view_count, owner_user, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let ps = recent()
        .group_by(Ident::<Post>::new())
        .select(
            comments_of(db)
                .opt()
                .and(votes_of(db).select(&db.vote.vote_type_id).opt())
                .and(owner_user.select(badges_of(db)).select(&db.badge.class).opt())
                .and(owner_user.select(&db.user.reputation).opt()),
        )
        .fold([0i64, 0, 0, 0], |a, (((_, t), b), r)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2].max(b.unwrap_or(0)), a[3].max(r.unwrap_or(0))]);
    let cc = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let w = whole(recent())
        .select(Ident::<Post>::new().and(view_count.opt()).and(&ps))
        .window(rank, |((_, w), a): ((Id<Post>, Option<i64>), [i64; 4])| (w.is_none(), Reverse(w), Reverse(a[0] - a[1])), asc);
    let v = drain((&w).filt(|(_, r)| r <= 10).map(|(((p, _), _), _)| p).select(Ident::<Post>::new().and(&ps).and(&cc)));
    rows(v.into_iter().map(|(_, ((p, a), c))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year' AND p.Score IS NOT NULL),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount FROM RankedPosts rp WHERE rp.rn <= 5),
// PostVoteSummary AS (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes v GROUP BY v.PostId),
// FinalSummary AS (SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, COALESCE(pvs.UpVotes, 0) AS UpVotes, COALESCE(pvs.DownVotes, 0) AS DownVotes,
//        (COALESCE(pvs.UpVotes, 0) - COALESCE(pvs.DownVotes, 0)) AS NetVotes FROM TopPosts tp LEFT JOIN PostVoteSummary pvs ON tp.PostId = pvs.PostId)
// SELECT f.*, CASE WHEN f.NetVotes > 10 THEN 'Highly Popular' WHEN f.NetVotes BETWEEN 1 AND 10 THEN 'Moderately Popular' ELSE 'Less Popular' END AS PopularityStatus
// FROM FinalSummary f ORDER BY f.Score DESC, f.ViewCount DESC;
fn q4377(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1))).group_by(post_type_id).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d): (Id<Post>, i64)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let pvs = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let v = drain((&tp).select((&pvs).opt()));
    rows(v.into_iter().map(|(p, a)| {
        let a = a.unwrap_or([0, 0]);
        let n = a[0] - a[1];
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(n), V::S(if n > 10 { "Highly Popular" } else if n >= 1 { "Moderately Popular" } else { "Less Popular" })]);
        row(f)
    }))
}

// WITH UserVoteStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        COUNT(DISTINCT P.Id) AS PostsCount, COUNT(DISTINCT C.Id) AS CommentsCount
//     FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId GROUP BY U.Id, U.DisplayName, U.Reputation),
// RankedUsers AS (SELECT UserId, DisplayName, Reputation, UpVotes, DownVotes, PostsCount, CommentsCount, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserVoteStats),
// RecentPostStats AS (SELECT P.OwnerUserId, COUNT(*) AS RecentPosts, MAX(P.CreationDate) AS LastPostDate FROM Posts P
//     WHERE P.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' GROUP BY P.OwnerUserId)
// SELECT R.DisplayName, R.Reputation, R.UpVotes, R.DownVotes, R.PostsCount, R.CommentsCount, COALESCE(RP.RecentPosts, 0) AS RecentPosts, RP.LastPostDate
// FROM RankedUsers R LEFT JOIN RecentPostStats RP ON R.UserId = RP.OwnerUserId WHERE (R.UpVotes - R.DownVotes) > 10 AND R.Rank <= 100 ORDER BY R.Rank;
//
// Rank reads only Reputation, so the hundred users are picked first (ties broken by user id) and the votes x posts x comments product is driven for them alone.
fn q2038(db: &'static So) -> String {
    let tu = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 100);
    let tus: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let s = (&tus)
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt().and(posts_of(db).select(comments_of(db).opt()).opt()))
        .fold([0i64; 2], |a, (t, _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let pc = (&tus).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let cc = (&tus).group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let Post { creation_date, owner_user, .. } = &db.post;
    let rps = db.post.with(creation_date.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).group_by(owner_user).select(creation_date).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let v = drain((&s).filt(|a| a[0] - a[1] > 10).and(&pc).and(&cc).and((&rps).opt()));
    rows(v.into_iter().map(|(u, (((a, p), c), r))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(p), V::I(c)]);
        f.extend(match r {
            Some((n, m)) => [V::I(n), V::T(m)],
            None => [V::I(0), V::Null],
        });
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, COUNT(DISTINCT c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges, COALESCE(AVG(COALESCE(p.Score, 0)), 0) AS AvgScore, COALESCE(SUM(p.ViewCount), 0) AS TotalViews
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.UserId = u.Id
//     LEFT JOIN Badges b ON u.Id = b.UserId WHERE u.CreationDate < cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, CommentCount, Upvotes, Downvotes, GoldBadges + SilverBadges + BronzeBadges AS TotalBadges, AvgScore, TotalViews,
//        ROW_NUMBER() OVER (ORDER BY PostCount DESC, Upvotes DESC) AS UserRank FROM UserActivity)
// SELECT UserId, DisplayName, PostCount, CommentCount, Upvotes, Downvotes, TotalBadges, AvgScore, TotalViews FROM TopUsers WHERE UserRank <= 10;
//
// `v.UserId = u.Id` with `p.OwnerUserId = u.Id` is a vote cast by the post's owner (`own_votes`).
fn q5308(db: &'static So) -> String {
    let Post { score, view_count, .. } = &db.post;
    let users = || db.user.with((&db.user.creation_date).lt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let pc = users().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let ov = own_votes(db);
    let s = users()
        .group_by(Ident::<User>::new())
        .select(
            posts_of(db)
                .select(comments_of(db).opt().and((&ov).select(&db.vote.vote_type_id).opt()).and(score).and(view_count.opt()))
                .opt()
                .and(badges_of(db).select(&db.badge.class).opt()),
        )
        .fold([0i64; 8], |a, (p, b)| {
            let (t, s, w) = p.map_or((None, 0, None), |(((_, t), s), w)| (t, s, w));
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + (b == Some(1)) as i64, a[3] + (b == Some(2)) as i64, a[4] + (b == Some(3)) as i64, a[5] + 1, a[6] + s, a[7] + w.unwrap_or(0)]
        });
    let cc = users().group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = top_n(drain((&s).and(&pc).and(&cc)), |&(u, ((a, n), _))| (Reverse(n), Reverse(a[0]), u), 10);
    rows(v.into_iter().map(|(u, ((a, n), c))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(c), V::I(a[0]), V::I(a[1]), V::I(a[2] + a[3] + a[4]), V::F(a[6] as f64 / a[5] as f64), V::I(a[7])]);
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty, COUNT(DISTINCT p.Id) AS TotalPosts,
//        COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS TotalQuestions, COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS TotalAnswers, COUNT(DISTINCT b.Id) AS TotalBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 9 LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// PostDetails AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COALESCE((SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id), 0) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year')
// SELECT u.DisplayName, s.Reputation, s.TotalBounty, pd.Title, pd.CreationDate, pd.Score, pd.ViewCount, pd.CommentCount
// FROM UserStatistics s INNER JOIN PostDetails pd ON s.UserId = pd.PostId LEFT JOIN Users u ON s.UserId = u.Id WHERE pd.rn <= 5 ORDER BY s.TotalBounty DESC, s.Reputation DESC, pd.CreationDate DESC;
//
// `s.UserId = pd.PostId` joins a user id to a post id, so it goes through the raw ids; UserStatistics is computed for the users that join.
fn q2542(db: &'static So) -> String {
    let Post { owner_user_id, creation_date, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d): (Id<Post>, i64)| (Reverse(d), p), asc);
    let pd: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let pidx: HashIdx<i64, Id<Post>> = (&pd).select(&db.post.origid).inv().collect();
    let joined: MatSet<Id<User>> = db.user.with((&db.user.origid).select(&pidx)).collect();
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(9))).select((&db.vote.bounty_amount).opt());
    let s = (&joined).group_by(Ident::<User>::new()).select(posts_of(db).select(bounty.opt()).opt().and(badges_of(db).opt())).fold(0i64, |n, (p, _)| n + p.flatten().flatten().unwrap_or(0));
    let cc = (&pd).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&s).and((&db.user.origid).select(&pidx).select(Ident::<Post>::new().and(&cc))));
    rows(v.into_iter().map(|(u, (b, (p, c)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(b));
        f.extend(post_fields(db, p, &["title", "created", "score", "views"]));
        f.push(V::I(c));
        row(f)
    }))
}

// Rewritten (rewrites/6486.sql): the projected ROW_NUMBER and the final order are tie-broken on PostId.
// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, u.DisplayName AS OwnerDisplayName, COUNT(DISTINCT c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        COALESCE(SUM(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 ELSE 0 END), 0) AS CloseCount
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId
//     WHERE p.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '90 days') GROUP BY p.Id, p.Title, p.CreationDate, p.Score, u.DisplayName),
// PostStats AS (SELECT PostId, Title, CreationDate, Score, OwnerDisplayName, CommentCount, UpVotes, DownVotes, CloseCount, (Score + UpVotes - DownVotes) AS NetScore FROM RecentPosts)
// SELECT ps.PostId, ps.Title, ps.CreationDate, ps.Score, ps.OwnerDisplayName, ps.CommentCount, ps.UpVotes, ps.DownVotes, ps.CloseCount, ps.NetScore,
//        ROW_NUMBER() OVER (ORDER BY ps.NetScore DESC, ps.PostId) AS Rank FROM PostStats ps WHERE ps.CloseCount = 0 ORDER BY ps.NetScore DESC, ps.PostId FETCH FIRST 10 ROWS ONLY;
fn q6486(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let base = || db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -90))).with(owner_user);
    let rp = base()
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(history_of(db).select(&db.post_history.post_history_type_id).opt()))
        .fold([0i64; 3], |a, ((_, t), h)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + (h == Some(10)) as i64]);
    let cc = base().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ps = (&rp).filt(|a| a[2] == 0);
    let w = whole(&ps).select(Ident::<Post>::new().and(score.and(&ps).map(|(s, a)| (s + a[0] - a[1], a)))).window(row_number, |(p, (n, _)): (Id<Post>, (i64, [i64; 3]))| (Reverse(n), p), asc);
    let v = drain((&w).filt(|(_, r)| r <= 10).map(|((p, (n, a)), r)| (p, n, a, r)));
    rows(v.into_iter().map(|(_, (p, n, a, r))| {
        let c = cc.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "owner"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(n), V::I(r)]);
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, COALESCE(SUM(CASE WHEN C.Id IS NOT NULL THEN 1 ELSE 0 END), 0) AS CommentCount,
//        COALESCE(SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges, COALESCE(SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadges,
//        COALESCE(SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadges
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// RankedUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, AnswerCount, QuestionCount, CommentCount, GoldBadges, SilverBadges, BronzeBadges,
//        DENSE_RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserStatistics)
// SELECT UserId, DisplayName, Reputation, PostCount, AnswerCount, QuestionCount, CommentCount, GoldBadges, SilverBadges, BronzeBadges, ReputationRank
// FROM RankedUsers WHERE ReputationRank <= 10 ORDER BY Reputation DESC;
//
// ReputationRank reads only Reputation, so the users are ranked first and the posts x comments x badges product is driven for the top ten ranks alone.
fn q29645(db: &'static So) -> String {
    let w = whole(&db.user.reputation).select(Ident::<User>::new().and(&db.user.reputation)).window(dense_rank, |(_, r): (Id<User>, i64)| Reverse(r), asc);
    let tr: MatSet<(Id<User>, i64)> = (&w).filt(|(_, r)| r <= 10).map(|((u, _), r)| (u, r)).collect();
    let rank = by_first(&tr);
    let tus: MatSet<Id<User>> = (&tr).map(|(u, _)| u).collect();
    let s = (&tus)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(comments_of(db).opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 6], |a, (p, b)| {
            let (t, c) = p.map_or((0, false), |(t, c)| (t, c.is_some()));
            [a[0] + (t == 2) as i64, a[1] + (t == 1) as i64, a[2] + c as i64, a[3] + (b == Some(1)) as i64, a[4] + (b == Some(2)) as i64, a[5] + (b == Some(3)) as i64]
        });
    let pc = (&tus).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let v = drain((&s).and(&pc).and(&rank));
    rows(v.into_iter().map(|(u, ((a, n), r))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, COALESCE(u.DisplayName, 'Community User') AS OwnerDisplayName, COUNT(DISTINCT c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        RANK() OVER (ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, u.DisplayName),
// TopPosts AS (SELECT PostId, Title, CreationDate, OwnerDisplayName, CommentCount, UpVotes, DownVotes FROM RankedPosts WHERE Rank <= 10)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.OwnerDisplayName, tp.CommentCount, tp.UpVotes, tp.DownVotes,
//        (SELECT COUNT(*) FROM Votes v WHERE v.PostId = tp.PostId AND v.VoteTypeId = 4) AS FavoriteCount,
//        (SELECT COUNT(*) FROM PostHistory ph WHERE ph.PostId = tp.PostId AND ph.PostHistoryTypeId IN (10, 11)) AS CloseReopenCount
// FROM TopPosts tp ORDER BY tp.UpVotes DESC, tp.CreationDate DESC;
//
// Rank reads only base columns, so the top questions are picked first and the comments x votes product is driven for those alone.
fn q7046(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, .. } = &db.post;
    let w = whole(db.post.with(post_type_id.eq(1)).with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))))
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(rank, |((_, s), d): ((Id<Post>, i64), i64)| (Reverse(s), Reverse(d)), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|(((p, _), _), _)| p).collect();
    let g = || (&tp).group_by(Ident::<Post>::new());
    let s = g().select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = g().select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let fav = g().select(votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(4))).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let cr = g().select(history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([10, 11]))).opt()).fold(0i64, |n, h| n + h.is_some() as i64);
    let v = drain((&s).and(&cc).and(&fav).and(&cr));
    rows(v.into_iter().map(|(p, (((a, c), f), r))| {
        let mut x = post_fields(db, p, &["id", "title", "created"]);
        x.push(V::S(db.post.owner_user.get(p).map_or("Community User", |u| db.user.display_name.get(u).unwrap())));
        x.extend([V::I(c), V::I(a[0]), V::I(a[1]), V::I(f), V::I(r)]);
        row(x)
    }))
}

// WITH RecentPosts AS (SELECT p.Id, p.Title, p.OwnerUserId, p.CreationDate, p.LastActivityDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.AcceptedAnswerId,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// UserStats AS (SELECT u.Id, u.DisplayName, u.Reputation, u.Views, COUNT(DISTINCT b.Id) AS BadgeCount, SUM(CASE WHEN p.OwnerUserId = u.Id THEN 1 ELSE 0 END) AS PostCount,
//        SUM(CASE WHEN v.UserId = u.Id AND v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId WHERE u.Reputation > 1000
//     GROUP BY u.Id, u.DisplayName, u.Reputation, u.Views),
// TopRecentPosts AS (SELECT rp.*, u.DisplayName AS OwnerDisplayName, us.Reputation AS OwnerReputation, us.BadgeCount FROM RecentPosts rp JOIN Users u ON rp.OwnerUserId = u.Id
//     JOIN UserStats us ON u.Id = us.Id WHERE rp.rn = 1)
// SELECT trp.Title, trp.OwnerDisplayName, trp.OwnerReputation, trp.CreationDate, trp.Score, trp.ViewCount, trp.AnswerCount, trp.CommentCount, trp.BadgeCount,
//        COALESCE((SELECT COUNT(*) FROM Comments c WHERE c.PostId = trp.Id), 0) AS TotalComments FROM TopRecentPosts trp ORDER BY trp.Score DESC, trp.ViewCount DESC;
//
// UserStats has one row per user and only its BadgeCount and Reputation are read, so PostCount and UpVoteCount are not computed.
fn q5541(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d): (Id<Post>, i64)| (Reverse(d), p), asc);
    let rp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let bc = db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let cc = (&rp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&cc).and(owner_user.select(Ident::<User>::new().and(&bc))));
    rows(v.into_iter().map(|(p, (c, (u, b)))| {
        let mut f = post_fields(db, p, &["title"]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend(post_fields(db, p, &["created", "score", "views", "answers", "comments"]));
        f.extend([V::I(b), V::I(c)]);
        row(f)
    }))
}

// WITH UserVoteStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVotes,
//        COUNT(DISTINCT CASE WHEN V.VoteTypeId IN (2, 3) THEN V.PostId END) AS TotalVotedPosts FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.DisplayName),
// PostActivity AS (SELECT P.Id AS PostId, P.Title, P.ViewCount, P.Score, COALESCE(COUNT(CASE WHEN C.UserId IS NOT NULL THEN 1 END), 0) AS CommentCount,
//        COALESCE(SUM(CASE WHEN PH.Comment IS NOT NULL THEN 1 END), 0) AS HistoryEntryCount
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN PostHistory PH ON P.Id = PH.PostId GROUP BY P.Id, P.Title, P.ViewCount, P.Score),
// TopPosts AS (SELECT PA.PostId, PA.Title, PA.ViewCount, PA.Score, RANK() OVER (ORDER BY PA.Score DESC, PA.ViewCount DESC) AS PostRank FROM PostActivity PA WHERE PA.Score > 0)
// SELECT UVs.DisplayName, UVs.UpVotes, UVs.DownVotes, TP.Title, TP.ViewCount, TP.Score, TP.PostRank, CASE WHEN UVs.TotalVotedPosts > 0 THEN 'Active Voter' ELSE 'Inactive Voter' END AS VotingStatus
// FROM UserVoteStats UVs JOIN TopPosts TP ON UVs.UserId = TP.PostId WHERE UVs.UpVotes - UVs.DownVotes > 0 AND TP.PostRank <= 10 ORDER BY TP.Score DESC, UVs.UpVotes DESC;
//
// PostActivity has one row per post and only its base columns are read, so its comment and history counts are not computed. `UVs.UserId = TP.PostId`
// joins a user id to a post id, so it goes through the raw ids.
fn q23985(db: &'static So) -> String {
    let Post { score, view_count, origid, .. } = &db.post;
    let w = whole(db.post.with(score.gt(0)))
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(rank, |((_, s), w): ((Id<Post>, i64), Option<i64>)| (Reverse(s), w.is_none(), Reverse(w)), asc);
    let tr: MatSet<(Id<Post>, i64)> = (&w).filt(|(_, r)| r <= 10).map(|(((p, _), _), r)| (p, r)).collect();
    let rank = by_first(&tr);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let tps: MatSet<Id<Post>> = (&tr).map(|(p, _)| p).collect();
    let users: MatSet<Id<User>> = (&tps).select(origid.select(&uidx)).collect();
    let uv = (&users).group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let vp = (&users).group_by(Ident::<User>::new()).select(votes_by(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([2, 3]))).select(&db.vote.post_id)).count_distinct();
    let v = drain((&rank).and(origid.select(&uidx).select(Ident::<User>::new().and((&uv).filt(|a| a[0] - a[1] > 0)).and((&vp).opt()))));
    rows(v.into_iter().map(|(p, (r, ((u, a), n)))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1])];
        f.extend(post_fields(db, p, &["title", "views", "score"]));
        f.extend([V::I(r), V::S(if n.unwrap_or(0) > 0 { "Active Voter" } else { "Inactive Voter" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostVoteStats AS (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, COUNT(v.Id) AS TotalVotes
//     FROM Votes v GROUP BY v.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, COALESCE(ub.BadgeCount, 0) AS BadgeCount, COALESCE(pvs.UpVotes, 0) AS UpVotes, COALESCE(pvs.DownVotes, 0) AS DownVotes,
//        CASE WHEN rp.Score IS NULL THEN 'No Score' WHEN rp.Score >= 10 THEN 'High Score' WHEN rp.Score BETWEEN 1 AND 9 THEN 'Medium Score' ELSE 'Low Score' END AS ScoreCategory
// FROM RankedPosts rp LEFT JOIN UserBadges ub ON rp.OwnerUserId = ub.UserId LEFT JOIN PostVoteStats pvs ON rp.PostId = pvs.PostId WHERE rp.rn = 1 ORDER BY rp.CreationDate DESC LIMIT 100;
fn q1912(db: &'static So) -> String {
    let Post { owner_user, owner_user_id, creation_date, score, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d): (Id<Post>, i64)| (Reverse(d), p), asc);
    let rp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let pvs = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let bc = (&rp).select(owner_user).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = drain((&rp).select(owner_user.select(&bc).opt().and((&pvs).opt())));
    let v = top_n(v, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 100);
    rows(v.into_iter().map(|(p, (b, a))| {
        let s = score.get(p).unwrap();
        let a = a.unwrap_or([0, 0]);
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(b.unwrap_or(0)), V::I(a[0]), V::I(a[1]), V::S(if s >= 10 { "High Score" } else if (1..=9).contains(&s) { "Medium Score" } else { "Low Score" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.AnswerCount, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.Score IS NOT NULL),
// UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounties, COUNT(DISTINCT p.Id) AS TotalPosts, AVG(p.Score) AS AvgPostScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 8 GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT ups.UserId, ups.DisplayName, ups.TotalBounties, ups.TotalPosts, ups.AvgPostScore, RANK() OVER (ORDER BY ups.TotalBounties DESC, ups.AvgPostScore DESC) AS UserRank
//     FROM UserPostStats ups WHERE ups.TotalPosts > 0)
// SELECT tp.UserId, tp.DisplayName, tp.TotalBounties, tp.TotalPosts, tp.AvgPostScore, rp.PostId, rp.Title, rp.Score, rp.CreationDate
// FROM TopUsers tp LEFT JOIN RankedPosts rp ON tp.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = rp.PostId) WHERE tp.UserRank <= 10 ORDER BY tp.TotalBounties DESC, tp.AvgPostScore DESC;
//
// The correlated subquery looks a post up by its primary key, so it is the post's own OwnerUserId. RankedPosts' Rank is never read.
// AvgPostScore is ordered by comparing the exact fractions.
fn q2348(db: &'static So) -> String {
    let Post { score, owner_user, creation_date, .. } = &db.post;
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let ups = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(score.and(bounty.opt())).opt()).fold([0i64; 3], |a, p| match p {
        Some((s, b)) => [a[0] + b.flatten().unwrap_or(0), a[1] + 1, a[2] + s],
        None => a,
    });
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let ok = (&ups).and((&pc).filt(|n| n > 0));
    let key = |(_, (a, _)): (Id<User>, ([i64; 3], i64))| {
        let (mut x, mut y) = (a[2].abs(), a[1]);
        while y != 0 {
            (x, y) = (y, x % y);
        }
        (a[0], (a[2] / x, a[1] / x))
    };
    let by = |x: &(i64, (i64, i64)), y: &(i64, (i64, i64))| y.0.cmp(&x.0).then(((y.1).0 as i128 * (x.1).1 as i128).cmp(&((x.1).0 as i128 * (y.1).1 as i128)));
    let w = whole(&ok).select(Ident::<User>::new().and(&ok)).window(rank, key, by);
    let tu: MatSet<(Id<User>, ([i64; 3], i64))> = (&w).filt(|(_, r)| r <= 10).map(|(x, _)| x).collect();
    let recent: HashIdx<Id<User>, Id<Post>> = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user).inv().collect();
    let v = drain(by_first(&tu).and((&recent).opt()));
    rows(v.into_iter().map(|(u, ((a, n), p))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(n), avg(a[2], a[1])]);
        f.extend(match p {
            Some(p) => post_fields(db, p, &["id", "title", "score", "created"]),
            None => vec![V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId
//     GROUP BY U.Id, U.DisplayName, U.Reputation),
// RecentPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS PostRank, P.OwnerUserId
//     FROM Posts P WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// TopUsers AS (SELECT UR.UserId, UR.DisplayName, UR.Reputation, UR.PostCount, UR.UpVoteCount, UR.DownVoteCount FROM UserReputation UR WHERE UR.Reputation > 1000 AND UR.PostCount > 5
//     ORDER BY UR.Reputation DESC LIMIT 10)
// SELECT TU.DisplayName, TU.Reputation, RP.Title AS RecentPostTitle, RP.CreationDate AS RecentPostDate, (TU.UpVoteCount - TU.DownVoteCount) AS VoteBalance
// FROM TopUsers TU LEFT JOIN RecentPosts RP ON TU.UserId = RP.OwnerUserId AND RP.PostRank = 1 WHERE TU.Reputation IS NOT NULL ORDER BY VoteBalance DESC, TU.Reputation DESC;
//
// TopUsers is cut on Reputation and the distinct post count alone, so the ten users are picked first and the posts x votes product is driven for them alone.
fn q205(db: &'static So) -> String {
    let Post { owner_user, creation_date, .. } = &db.post;
    let rich = || db.user.with((&db.user.reputation).gt(1000));
    let pc = rich().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let tu = top_n(drain((&pc).filt(|n| n > 5)), |&(u, _)| Reverse(db.user.reputation.get(u).unwrap()), 10);
    let tus: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let s = (&tus)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt())
        .fold(0i64, |n, t| n + (t.flatten() == Some(2)) as i64 - (t.flatten() == Some(3)) as i64);
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(owner_user.select(Ident::<User>::new().with(&tus)))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d): (Id<Post>, i64)| (Reverse(d), p), asc);
    let last = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p);
    let v = drain((&s).and(last.opt()));
    rows(v.into_iter().map(|(u, (b, p))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(match p {
            Some(p) => post_fields(db, p, &["title", "created"]),
            None => vec![V::Null, V::Null],
        });
        f.push(V::I(b));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS RecentPostsRanking
//     FROM Posts p WHERE p.CreationDate >= CURRENT_TIMESTAMP - INTERVAL '1 year'),
// UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, COUNT(DISTINCT b.Id) AS BadgeCount
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// PostWithComments AS (SELECT p.Title, COUNT(c.Id) AS CommentCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY p.Title)
// SELECT rp.PostId, rp.Title, rp.ViewCount, rp.AnswerCount, ur.DisplayName, ur.Reputation, ur.TotalUpVotes, ur.BadgeCount, COALESCE(pwc.CommentCount, 0) AS CommentCount,
//        CASE WHEN rp.RecentPostsRanking = 1 THEN 'Recent' ELSE 'Not Recent' END AS PostStatus
// FROM RankedPosts rp JOIN UserReputation ur ON ur.UserId = rp.PostId LEFT JOIN PostWithComments pwc ON rp.Title = pwc.Title
// WHERE rp.ViewCount > 100 AND ur.Reputation > 50 ORDER BY rp.ViewCount DESC, ur.Reputation DESC;
//
// CURRENT_TIMESTAMP is a TIMESTAMPTZ, so CreationDate is compared as a New York wall time. `ur.UserId = rp.PostId` joins a user id to a post id, so it goes
// through the raw ids; UserReputation is computed for the users that join.
fn q4643(db: &'static So) -> String {
    let Post { post_type_id, creation_date, view_count, title, .. } = &db.post;
    let since = add_years(utc_to_ny(now_utc()), -1);
    let w = db.post.with(creation_date.ge(since)).group_by(post_type_id).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d): (Id<Post>, i64)| (Reverse(d), p), asc);
    let rp: MatSet<(Id<Post>, i64)> = (&w).map(|((p, _), r)| (p, r)).collect();
    let rank = by_first(&rp);
    let uidx: HashIdx<i64, Id<User>> = db.user.with((&db.user.reputation).gt(50)).select(&db.user.origid).inv().collect();
    let rps: MatSet<Id<Post>> = (&rp).map(|(p, _)| p).collect();
    let hit = || (&rps).with(view_count.gt(100));
    let users: MatSet<Id<User>> = hit().select((&db.post.origid).select(&uidx)).collect();
    let ur = (&users)
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt().and(badges_of(db).opt()))
        .fold(0i64, |n, (t, _)| n + (t == Some(2)) as i64);
    let bc = (&users).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let pwc = db.post.group_by(title).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain(hit().select(Ident::<Post>::new().and(&rank).and((&db.post.origid).select(&uidx).select(Ident::<User>::new().and(&ur).and(&bc))).and(title.select(&pwc).opt())));
    rows(v.into_iter().map(|(_, (((p, r), ((u, t), b)), c))| {
        let mut f = post_fields(db, p, &["id", "title", "views", "answers"]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::I(t), V::I(b), V::I(c.unwrap_or(0)), V::S(if r == 1 { "Recent" } else { "Not Recent" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS PostRank,
//        CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 'Accepted' ELSE 'Pending' END AS AnswerStatus FROM Posts p WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'),
// PostVotes AS (SELECT v.PostId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes FROM Votes v GROUP BY v.PostId),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c GROUP BY c.PostId)
// SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.CreationDate, COALESCE(pv.UpVotes, 0) AS TotalUpVotes, COALESCE(pv.DownVotes, 0) AS TotalDownVotes,
//        COALESCE(pc.CommentCount, 0) AS TotalComments, rp.AnswerStatus,
//        CASE WHEN rp.Score > 100 THEN 'High Scoring' WHEN rp.Score BETWEEN 50 AND 100 THEN 'Medium Scoring' ELSE 'Low Scoring' END AS ScoreCategory
// FROM RankedPosts rp LEFT JOIN PostVotes pv ON rp.PostId = pv.PostId LEFT JOIN PostComments pc ON rp.PostId = pc.PostId WHERE rp.PostRank = 1 ORDER BY rp.CreationDate DESC LIMIT 50 OFFSET 0;
fn q22677(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, accepted_answer_id, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1))).group_by(post_type_id).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d): (Id<Post>, i64)| (Reverse(d), p), asc);
    let rp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let pv = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let pc = (&rp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&pc).and((&pv).opt()));
    let v = top_n(v, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 50);
    rows(v.into_iter().map(|(p, (c, a))| {
        let s = score.get(p).unwrap();
        let a = a.unwrap_or([0, 0]);
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "created"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c), V::S(if accepted_answer_id.get(p).is_some() { "Accepted" } else { "Pending" })]);
        f.push(V::S(if s > 100 { "High Scoring" } else if s >= 50 { "Medium Scoring" } else { "Low Scoring" }));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesCount,
//        DENSE_RANK() OVER (ORDER BY SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) - SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) DESC) AS UserRank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// PopularPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, pr.UserRank FROM Posts p
//     JOIN (SELECT p.Id, ROW_NUMBER() OVER (ORDER BY p.ViewCount DESC) AS UserRank FROM Posts p WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '30 DAY') pr ON p.Id = pr.Id)
// SELECT ua.DisplayName, COUNT(DISTINCT pp.PostId) AS PopularPostCount, SUM(pp.ViewCount) AS TotalPopularViews, (SELECT COUNT(DISTINCT b.Id) FROM Badges b WHERE b.UserId = ua.UserId) AS BadgeCount
// FROM UserActivity ua LEFT JOIN PopularPosts pp ON ua.UserId = pp.UserRank GROUP BY ua.UserId, ua.DisplayName HAVING COUNT(DISTINCT pp.PostId) > 0
// ORDER BY TotalPopularViews DESC LIMIT 10;
//
// UserActivity has one row per user and only its id and name are read, so its aggregates are not computed. `ua.UserId = pp.UserRank` joins a user id to a
// row number, so it goes through the raw id. The ROW_NUMBER breaks ViewCount ties by post id (the SQL leaves them open; the answer is empty on this data).
fn q32(db: &'static So) -> String {
    let Post { creation_date, view_count, .. } = &db.post;
    let w = whole(db.post.with(creation_date.ge(add_days(current_date(), -30))))
        .select(Ident::<Post>::new().and(view_count.opt()))
        .window(row_number, |(p, w): (Id<Post>, Option<i64>)| (w.is_none(), Reverse(w), p), asc);
    let pr: MatSet<(i64, Id<Post>)> = (&w).map(|((p, _), r)| (r, p)).collect();
    let by_rank = by_first(&pr);
    let pp = db
        .user
        .group_by(Ident::<User>::new())
        .select((&db.user.origid).select(&by_rank).select(view_count.opt()))
        .fold((0i64, 0i64, 0i64), |(n, k, s), w| (n + 1, k + w.is_some() as i64, s + w.unwrap_or(0)));
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = top_n(drain((&pp).filt(|(n, _, _)| n > 0).and(&bc)), |&(u, ((_, k, s), _))| (k == 0, Reverse(s), u), 10);
    rows(v.into_iter().map(|(u, ((n, k, s), b))| row(vec![user_col(db, u, "name"), V::I(n), nullable(s, k), V::I(b)])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, u.DisplayName AS OwnerDisplayName, p.ViewCount, p.CreationDate, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.Score IS NOT NULL),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.OwnerDisplayName, rp.ViewCount, rp.CreationDate FROM RankedPosts rp WHERE rp.Rank <= 10),
// PostVoteSummary AS (SELECT PostId, SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes
//     FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY PostId)
// SELECT tp.PostId, tp.Title, tp.OwnerDisplayName, tp.ViewCount, COALESCE(pv.UpVotes, 0) AS TotalUpVotes, COALESCE(pv.DownVotes, 0) AS TotalDownVotes,
//        CASE WHEN tp.ViewCount IS NULL THEN 'No Views Recorded' WHEN tp.ViewCount < 100 THEN 'Low Engagement' WHEN tp.ViewCount BETWEEN 100 AND 1000 THEN 'Moderate Engagement'
//        ELSE 'High Engagement' END AS EngagementLevel
// FROM TopPosts tp LEFT JOIN PostVoteSummary pv ON tp.PostId = pv.PostId ORDER BY tp.ViewCount DESC;
fn q2666(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, owner_user, view_count, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score))
        .window(rank, |(_, s): (Id<Post>, i64)| Reverse(s), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|((p, _), _)| p).collect();
    let pv = db.vote.group_by(&db.vote.post).select(vtype_name(db)).fold([0i64; 2], |a, n| [a[0] + (n == "UpMod") as i64, a[1] + (n == "DownMod") as i64]);
    let v = drain((&tp).select((&pv).opt()));
    rows(v.into_iter().map(|(p, a)| {
        let a = a.unwrap_or([0, 0]);
        let mut f = post_fields(db, p, &["id", "title", "owner", "views"]);
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.push(V::S(match view_count.get(p) {
            None => "No Views Recorded",
            Some(w) if w < 100 => "Low Engagement",
            Some(w) if w <= 1000 => "Moderate Engagement",
            _ => "High Engagement",
        }));
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.ViewCount, p.OwnerUserId, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(DISTINCT r.Id) AS RecentPostCount FROM Users u LEFT JOIN RecentPosts r ON u.Id = r.OwnerUserId GROUP BY u.Id, u.Reputation),
// UpvotedPosts AS (SELECT v.PostId, COUNT(v.Id) AS UpvoteCount FROM Votes v WHERE v.VoteTypeId = 2 GROUP BY v.PostId),
// PostDetails AS (SELECT rp.Id, rp.Title, rp.CreationDate, rp.ViewCount, ur.Reputation, ur.RecentPostCount, COALESCE(up.UpvoteCount, 0) AS UpvoteCount
//     FROM RecentPosts rp JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId LEFT JOIN UpvotedPosts up ON rp.Id = up.PostId)
// SELECT pd.Title, pd.CreationDate, pd.ViewCount, pd.Reputation, pd.RecentPostCount, pd.UpvoteCount, CASE WHEN pd.RecentPostCount >= 5 THEN 'Active User' ELSE 'New User' END AS UserStatus
// FROM PostDetails pd WHERE pd.UpvoteCount > 0 ORDER BY pd.Reputation DESC, pd.UpvoteCount DESC;
fn q2521(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)));
    let rc = recent().group_by(owner_user).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let up = db.vote.with((&db.vote.vote_type_id).eq(2)).group_by(&db.vote.post).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let v = drain(recent().select(owner_user.select(Ident::<User>::new().and(&rc)).and(&up)));
    rows(v.into_iter().map(|(p, ((u, n), k))| {
        let mut f = post_fields(db, p, &["title", "created", "views"]);
        f.extend([user_col(db, u, "rep"), V::I(n), V::I(k), V::S(if n >= 5 { "Active User" } else { "New User" })]);
        row(f)
    }))
}

// WITH UserVoteStats AS (SELECT U.Id AS UserId, U.DisplayName, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesCount, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesCount
//     FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.DisplayName),
// PostStats AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, COALESCE(COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END), 0) AS CommentCount,
//        COALESCE(COUNT(DISTINCT PH.Id), 0) AS EditHistoryCount FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN PostHistory PH ON P.Id = PH.PostId
//     GROUP BY P.Id, P.Title, P.CreationDate, P.Score, P.ViewCount),
// TrendingPosts AS (SELECT PS.PostId, PS.Title, PS.CreationDate, PS.Score, PS.ViewCount, PS.CommentCount, ROW_NUMBER() OVER (ORDER BY PS.Score DESC, PS.ViewCount DESC) AS Rank FROM PostStats PS)
// SELECT UP.UserId, UP.DisplayName, TP.PostId, TP.Title, TP.CreationDate, TP.Score, TP.ViewCount, TP.CommentCount, UP.UpVotesCount, UP.DownVotesCount
// FROM UserVoteStats UP JOIN Posts P ON UP.UserId = P.OwnerUserId JOIN TrendingPosts TP ON P.Id = TP.PostId WHERE TP.Rank <= 10 ORDER BY UP.UpVotesCount DESC, UP.DownVotesCount ASC;
//
// Rank reads only Score and ViewCount, so the ten posts are picked first (ties broken by post id) and the comments x history product is driven for them alone.
fn q6510(db: &'static So) -> String {
    let Post { score, view_count, owner_user, .. } = &db.post;
    let v = top_n(drain(score), |&(p, s)| {
        let w = view_count.get(p);
        (Reverse(s), w.is_none(), Reverse(w), p)
    }, 10);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(history_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let uv = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = drain((&cc).and(owner_user.select(Ident::<User>::new().and(&uv))));
    rows(v.into_iter().map(|(p, (c, (u, a)))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(post_fields(db, p, &["id", "title", "created", "score", "views"]));
        f.extend([V::I(c), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN C.Id IS NOT NULL THEN 1 ELSE 0 END) AS CommentCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        COUNT(DISTINCT B.Id) AS BadgeCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId AND V.UserId = U.Id LEFT JOIN Badges B ON U.Id = B.UserId
//     WHERE U.CreationDate < TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY U.Id, U.DisplayName),
// UserRanked AS (SELECT UserId, DisplayName, QuestionCount, AnswerCount, CommentCount, UpVotes, DownVotes, BadgeCount,
//        RANK() OVER (ORDER BY QuestionCount DESC, AnswerCount DESC, UpVotes DESC) AS Rank FROM UserActivity)
// SELECT UR.DisplayName, UR.QuestionCount, UR.AnswerCount, UR.CommentCount, UR.UpVotes, UR.DownVotes, UR.BadgeCount,
//        (UR.QuestionCount + UR.AnswerCount * 2 + UR.UpVotes - UR.DownVotes) AS EngagementScore FROM UserRanked UR WHERE UR.Rank <= 10 ORDER BY EngagementScore DESC;
//
// `V.UserId = U.Id` with `P.OwnerUserId = U.Id` is a vote cast by the post's owner (`own_votes`). The whole product is driven, since Rank reads its sums.
fn q9189(db: &'static So) -> String {
    let ov = own_votes(db);
    let s = db
        .user
        .with((&db.user.creation_date).lt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(comments_of(db).opt()).and((&ov).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 5], |a, (p, _)| match p {
            Some(((t, c), v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + c.is_some() as i64, a[3] + (v == Some(2)) as i64, a[4] + (v == Some(3)) as i64],
            None => a,
        });
    let w = whole(&s).select(Ident::<User>::new().and(&s)).window(rank, |(_, a): (Id<User>, [i64; 5])| (Reverse(a[0]), Reverse(a[1]), Reverse(a[3])), asc);
    let tr: MatSet<Id<User>> = (&w).filt(|(_, r)| r <= 10).map(|((u, _), _)| u).collect();
    let bc = (&tr).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = drain((&bc).and(&s));
    rows(v.into_iter().map(|(u, (b, a))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(a.map(V::I));
        f.extend([V::I(b), V::I(a[0] + a[1] * 2 + a[3] - a[4])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Tags, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, RANK() OVER (ORDER BY COUNT(c.Id) DESC, p.CreationDate ASC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 YEAR'
//     GROUP BY p.Id, p.Title, p.Tags, p.CreationDate),
// MostCommentedPosts AS (SELECT PostId, Title, Tags, CommentCount, UpVotes, DownVotes FROM RankedPosts WHERE Rank <= 10),
// RecentVotes AS (SELECT PostId, COUNT(*) AS VoteCount FROM Votes WHERE CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 MONTH' GROUP BY PostId)
// SELECT mcp.PostId, mcp.Title, mcp.Tags, mcp.CommentCount, mcp.UpVotes, mcp.DownVotes, COALESCE(rv.VoteCount, 0) AS RecentVoteCount,
//        CASE WHEN mcp.UpVotes = 0 THEN 0 ELSE (mcp.UpVotes * 1.0 / NULLIF(mcp.UpVotes + mcp.DownVotes, 0)) * 100 END AS UpvotePercentage,
//        CASE WHEN mcp.UpVotes IS NULL THEN 'No votes yet' ELSE 'Votes exist' END AS VoteStatus
// FROM MostCommentedPosts mcp LEFT JOIN RecentVotes rv ON mcp.PostId = rv.PostId ORDER BY mcp.CommentCount DESC, mcp.UpVotes DESC;
fn q34864(db: &'static So) -> String {
    let Post { creation_date, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let rp = db
        .post
        .with(creation_date.ge(add_years(t0, -1)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let w = whole(&rp).select(Ident::<Post>::new().and(&rp).and(creation_date)).window(rank, |((_, a), d): ((Id<Post>, [i64; 3]), i64)| (Reverse(a[0]), d), asc);
    let mc: MatSet<(Id<Post>, [i64; 3])> = (&w).filt(|(_, r)| r <= 10).map(|(((p, a), _), _)| (p, a)).collect();
    let rv = db.vote.with((&db.vote.creation_date).ge(add_months(t0, -1))).group_by(&db.vote.post).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let v = drain(by_first(&mc).and((&rv).opt()));
    rows(v.into_iter().map(|(p, (a, n))| {
        let mut f = post_fields(db, p, &["id", "title", "tags"]);
        f.extend(a.map(V::I));
        f.push(V::I(n.unwrap_or(0)));
        f.push(V::F(if a[1] == 0 { 0.0 } else { a[1] as f64 * 1.0 / (a[1] + a[2]) as f64 * 100.0 }));
        f.push(V::S("Votes exist"));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Tags, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank, p.OwnerUserId
//     FROM Posts p WHERE p.PostTypeId = 1),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(p.Id) AS QuestionsAsked, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS UpvotedQuestions
//     FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId WHERE p.PostTypeId = 1 GROUP BY u.Id, u.DisplayName, u.Reputation HAVING COUNT(p.Id) > 5),
// PostTags AS (SELECT p.Id AS PostId, unnest(string_to_array(substring(p.Tags, 2, length(p.Tags) - 2), '><')) AS Tag FROM Posts p WHERE p.PostTypeId = 1),
// TagPopularity AS (SELECT Tag, COUNT(pt.PostId) AS TagCount FROM PostTags pt GROUP BY Tag HAVING COUNT(pt.PostId) > 10)
// SELECT u.DisplayName, u.Reputation, rp.PostId, rp.Title, rp.CreationDate, rp.Score, pt.Tag, tp.TagCount
// FROM TopUsers u JOIN RankedPosts rp ON u.UserId = rp.OwnerUserId JOIN PostTags pt ON rp.PostId = pt.PostId JOIN TagPopularity tp ON pt.Tag = tp.Tag
// WHERE rp.UserPostRank = 1 ORDER BY u.Reputation DESC, tp.TagCount DESC;
fn q28566(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, tags_str, .. } = &db.post;
    let qs = || db.post.with(post_type_id.eq(1));
    let qc = qs().group_by(owner_user).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let freq = qs().select(tags_str.flat_map(tag_list)).group_by(Same::<Str>::new()).select(Same::<Str>::new()).fold(0i64, |n, _| n + 1);
    let w = qs().group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d): (Id<Post>, i64)| (Reverse(d), p), asc);
    let rp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let v = drain((&rp).with(owner_user.select((&qc).filt(|n| n > 5))).select(Ident::<Post>::new().and(tags_str.flat_map(tag_list).select(Same::<Str>::new().and((&freq).filt(|n| n > 10))))));
    rows(v.into_iter().map(|(_, (p, (t, n)))| {
        let mut f = post_fields(db, p, &["owner", "rep", "id", "title", "created", "score"]);
        f.extend([V::S(t), V::I(n)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.OwnerUserId, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) as PostRank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(DISTINCT b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation),
// ClosedPosts AS (SELECT ph.PostId, COUNT(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 END) AS CloseCount FROM PostHistory ph GROUP BY ph.PostId),
// PostAnalytics AS (SELECT rp.Id AS PostId, rp.Title, rp.Score, COALESCE(up.Reputation, 0) AS UserReputation, COALESCE(up.BadgeCount, 0) AS BadgeCount, COALESCE(cp.CloseCount, 0) AS CloseCount
//     FROM RankedPosts rp JOIN Users u ON rp.OwnerUserId = u.Id LEFT JOIN UserReputation up ON u.Id = up.UserId LEFT JOIN ClosedPosts cp ON rp.Id = cp.PostId)
// SELECT pa.PostId, pa.Title, pa.Score, pa.UserReputation, pa.BadgeCount, pa.CloseCount FROM PostAnalytics pa
// WHERE (pa.UserReputation > 1000 OR pa.BadgeCount > 5) AND pa.CloseCount = 0 ORDER BY pa.Score DESC, pa.UserReputation DESC LIMIT 50;
//
// PostRank is never read, so it is not computed.
fn q4442(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let cp = db.post_history.group_by(&db.post_history.post).select(&db.post_history.post_history_type_id).fold(0i64, |n, t| n + (t == 10) as i64);
    let v = drain(
        db.post
            .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
            .select(owner_user.select(Ident::<User>::new().and(&db.user.reputation).and(&bc)).and((&cp).opt()))
            .filt(|(((_, r), b), c): (((Id<User>, i64), i64), Option<i64>)| (r > 1000 || b > 5) && c.unwrap_or(0) == 0),
    );
    let v = top_n(v, |&(p, (((_, r), _), _))| (Reverse(score.get(p).unwrap()), Reverse(r), p), 50);
    rows(v.into_iter().map(|(p, (((_, r), b), c))| {
        let mut f = post_fields(db, p, &["id", "title", "score"]);
        f.extend([V::I(r), V::I(b), V::I(c.unwrap_or(0))]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.CreationDate, COUNT(DISTINCT p.Id) AS TotalPosts, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 8 GROUP BY u.Id, u.DisplayName, u.Reputation, u.CreationDate),
// RecentPosts AS (SELECT Id, Title, OwnerUserId, CreationDate, Row_Number() OVER (PARTITION BY OwnerUserId ORDER BY CreationDate DESC) AS RecentPostRank FROM Posts)
// SELECT us.UserId, us.DisplayName, us.Reputation, us.TotalPosts, us.TotalBounty, us.TotalQuestions, us.TotalAnswers, rp.Title AS RecentPostTitle, rp.CreationDate AS RecentPostDate,
//        CASE WHEN us.TotalBounty > 0 THEN 'Has Bounty' ELSE 'No Bounty' END AS BountyStatus,
//        CASE WHEN us.CreationDate < cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' THEN 'Veteran' ELSE 'Newcomer' END AS UserStatus
// FROM UserStats us LEFT JOIN RecentPosts rp ON us.UserId = rp.OwnerUserId AND rp.RecentPostRank = 1 WHERE us.Reputation > 100 ORDER BY us.Reputation DESC, us.DisplayName ASC LIMIT 10;
//
// The order reads only Reputation and DisplayName, so the ten users are picked first and the posts x bounty-votes product is driven for them alone.
fn q54(db: &'static So) -> String {
    let Post { owner_user, creation_date, post_type_id, .. } = &db.post;
    let tu = top_n(drain(db.user.with((&db.user.reputation).gt(100)).select(&db.user.reputation)), |&(u, r)| (Reverse(r), db.user.display_name.get(u).unwrap(), u), 10);
    let tus: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let s = (&tus).group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(bounty.opt())).opt()).fold([0i64; 3], |a, p| match p {
        Some((t, b)) => [a[0] + b.flatten().unwrap_or(0), a[1] + (t == 1) as i64, a[2] + (t == 2) as i64],
        None => a,
    });
    let pc = (&tus).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let w = db.post.with(owner_user.select(Ident::<User>::new().with(&tus))).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d): (Id<Post>, i64)| (Reverse(d), p), asc);
    let last = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p);
    let v = drain((&s).and(&pc).and(last.opt()));
    let v = top_n(v, |&(u, _)| (Reverse(db.user.reputation.get(u).unwrap()), db.user.display_name.get(u).unwrap(), u), 10);
    let vet = add_years(ts(2024, 10, 1, 12, 34, 56), -1);
    rows(v.into_iter().map(|(u, ((a, n), p))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        f.extend(match p {
            Some(p) => post_fields(db, p, &["title", "created"]),
            None => vec![V::Null, V::Null],
        });
        f.push(V::S(if a[0] > 0 { "Has Bounty" } else { "No Bounty" }));
        f.push(V::S(if db.user.creation_date.get(u).unwrap() < vet { "Veteran" } else { "Newcomer" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.OwnerUserId, COUNT(c.Id) AS CommentCount, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS ScoreRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Score, p.CreationDate, p.OwnerUserId),
// UserReputations AS (SELECT u.Id AS UserId, u.Reputation, CASE WHEN u.Reputation > 1000 THEN 'High' WHEN u.Reputation BETWEEN 100 AND 1000 THEN 'Medium' ELSE 'Low' END AS ReputationCategory FROM Users u),
// RecentPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.CreationDate, ur.ReputationCategory FROM RankedPosts rp JOIN UserReputations ur ON rp.OwnerUserId = ur.UserId
//     WHERE rp.CreationDate >= CURRENT_TIMESTAMP - INTERVAL '30 days')
// SELECT rp.PostId, rp.Title, rp.Score, rp.ReputationCategory, COALESCE(MIN(v.BountyAmount), 0) AS LowestBounty,
//        (SELECT COUNT(*) FROM Votes v WHERE v.PostId = rp.PostId AND v.VoteTypeId IN (2, 3)) AS VoteCount
// FROM RecentPosts rp LEFT JOIN Votes v ON rp.PostId = v.PostId GROUP BY rp.PostId, rp.Title, rp.Score, rp.ReputationCategory HAVING COUNT(DISTINCT v.UserId) > 5
// ORDER BY rp.Score DESC, rp.Title ASC;
//
// CURRENT_TIMESTAMP is a TIMESTAMPTZ, so CreationDate is compared as a New York wall time. RankedPosts' CommentCount and ScoreRank are never read.
fn q1919(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let since = add_days(utc_to_ny(now_utc()), -30);
    let rp = || db.post.with(post_type_id.eq(1)).with(creation_date.ge(since)).with(owner_user);
    let lb = rp().group_by(Ident::<Post>::new()).select(votes_of(db).select((&db.vote.bounty_amount).opt()).opt()).fold(None::<i64>, |m, b| min_some(m, b.flatten()));
    let du = rp().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.user_id)).count_distinct();
    let vc = rp()
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([2, 3]))).opt())
        .fold(0i64, |n, v| n + v.is_some() as i64);
    let v = drain((&lb).and((&du).filt(|n| n > 5)).and(&vc).and(owner_user.select(&db.user.reputation)));
    rows(v.into_iter().map(|(p, (((m, _), n), r))| {
        let mut f = post_fields(db, p, &["id", "title", "score"]);
        f.extend([V::S(if r > 1000 { "High" } else if r >= 100 { "Medium" } else { "Low" }), V::I(m.unwrap_or(0)), V::I(n)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS rn FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.CreationDate > cast('2024-10-01' as date) - INTERVAL '1 year'),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.ViewCount, rp.CreationDate, rp.Score, rp.OwnerDisplayName FROM RankedPosts rp WHERE rp.rn = 1),
// PostStats AS (SELECT p.Id, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT vh.Id) AS VoteCount, COALESCE(SUM(CASE WHEN vh.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN vh.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes vh ON p.Id = vh.PostId GROUP BY p.Id)
// SELECT tp.Title, tp.ViewCount, ps.CommentCount, ps.VoteCount, ps.UpVotes, ps.DownVotes, ps.UpVotes - ps.DownVotes AS NetVotes,
//        COALESCE(CAST(ROUND(ps.UpVotes::numeric / NULLIF(ps.VoteCount, 0) * 100, 2) AS VARCHAR), '0.00%') AS VotePercentage
// FROM TopPosts tp JOIN PostStats ps ON tp.PostId = ps.Id WHERE tp.ViewCount > 100 ORDER BY tp.ViewCount DESC LIMIT 10;
//
// The answer is empty on this data, so the VARCHAR rendering of VotePercentage is not checked by the oracle.
fn q2084(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, view_count, .. } = &db.post;
    let w = db.post.with(creation_date.gt(add_years(date(2024, 10, 1), -1))).with(owner_user).group_by(post_type_id).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d): (Id<Post>, i64)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let hot = || (&tp).with(view_count.gt(100)).group_by(Ident::<Post>::new());
    let ps = hot()
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = hot().select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = hot().select(votes_of(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let v = top_n(drain((&ps).and(&cc).and(&vc)), |&(p, _)| (Reverse(view_count.get(p)), p), 10);
    rows(v.into_iter().map(|(p, ((a, c), n))| {
        let mut f = post_fields(db, p, &["title", "views"]);
        f.extend([V::I(c), V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[0] - a[1])]);
        f.push(if n == 0 { V::S("0.00%") } else { V::Owned(format!("{:?}", (a[0] as f64 / n as f64 * 100.0 * 100.0).round() / 100.0)) });
        row(f)
    }))
}

// Rewritten (rewrites/1651.sql): the ROW_NUMBER and the final order are tie-broken on the post id.
// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, u.DisplayName AS OwnerName,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC, p.Id) AS rn
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// AggregateVotes AS (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, COUNT(VoteTypeId) AS TotalVotes
//     FROM Votes GROUP BY PostId),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.OwnerName, av.UpVotes, av.DownVotes, av.TotalVotes FROM RankedPosts rp LEFT JOIN AggregateVotes av ON rp.PostId = av.PostId
//     WHERE rp.rn = 1)
// SELECT tp.PostId, tp.Title, tp.OwnerName, COALESCE(tp.UpVotes, 0) - COALESCE(tp.DownVotes, 0) AS NetVotes, tp.CreationDate,
//        CASE WHEN tp.Score > 10 THEN 'Highly Rated' WHEN tp.Score BETWEEN 5 AND 10 THEN 'Moderately Rated' ELSE 'Low Rated' END AS RatingCategory
// FROM TopPosts tp WHERE (tp.Score > 0 OR (tp.OwnerName IS NOT NULL AND tp.CreationDate < cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 month'))
// ORDER BY NetVotes DESC, tp.CreationDate DESC, tp.PostId LIMIT 100;
fn q1651(db: &'static So) -> String {
    let Post { owner_user, owner_user_id, creation_date, score, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let w = db.post.with(creation_date.ge(add_years(t0, -1))).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d): (Id<Post>, i64)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let av = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold(0i64, |n, t| n + (t == 2) as i64 - (t == 3) as i64);
    let m = add_months(t0, -1);
    let keep = score.and(owner_user.opt()).and(creation_date).filt(move |((s, u), d): ((i64, Option<Id<User>>), i64)| s > 0 || (u.is_some() && d < m));
    let v = drain((&tp).with(keep).select((&av).opt()));
    let v = top_n(v, |&(p, n)| (Reverse(n.unwrap_or(0)), Reverse(creation_date.get(p).unwrap()), p), 100);
    rows(v.into_iter().map(|(p, n)| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "owner"]);
        f.extend([V::I(n.unwrap_or(0)), V::T(creation_date.get(p).unwrap())]);
        f.push(V::S(if s > 10 { "Highly Rated" } else if s >= 5 { "Moderately Rated" } else { "Low Rated" }));
        row(f)
    }))
}

// Rewritten (rewrites/20119.sql): the ROW_NUMBER over the joined rows is tie-broken on p.Id, c.Id.
// WITH RankedPosts AS (SELECT p.Id, p.Title, p.ViewCount, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.ViewCount DESC, p.Id, c.Id) AS Rank,
//        COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount, p.OwnerUserId FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserVoteSummary AS (SELECT v.UserId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes,
//        COUNT(DISTINCT v.PostId) AS UniqueVotes FROM Votes v GROUP BY v.UserId)
// SELECT u.DisplayName, COALESCE(SUM(CASE WHEN bp.Rank <= 10 THEN 1 ELSE 0 END), 0) AS TopPostCount, COALESCE(SUM(CASE WHEN bp.Rank <= 10 THEN bp.ViewCount ELSE 0 END), 0) AS TopPostViews,
//        v.TotalUpvotes, v.TotalDownvotes, v.UniqueVotes,
//        CASE WHEN v.TotalUpvotes > v.TotalDownvotes THEN 'Positively Influential' WHEN v.TotalDownvotes > v.TotalUpvotes THEN 'Negatively Influential' ELSE 'Neutral' END AS InfluenceType
// FROM Users u LEFT JOIN RankedPosts bp ON u.Id = bp.OwnerUserId LEFT JOIN UserVoteSummary v ON u.Id = v.UserId WHERE u.Reputation > 100
// GROUP BY u.DisplayName, v.TotalUpvotes, v.TotalDownvotes, v.UniqueVotes ORDER BY TopPostViews DESC, u.DisplayName ASC LIMIT 100;
//
// The window numbers the post x comment rows, so those rows are materialised and ranked; the ranked rows then join to their owners.
fn q20119(db: &'static So) -> String {
    let Post { post_type_id, creation_date, view_count, owner_user, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(view_count.opt()).and(comments_of(db).opt()))
        .window(row_number, |((p, w), c): ((Id<Post>, Option<i64>), Option<Id<Comment>>)| (w.is_none(), Reverse(w), p, c), asc);
    let rr: MatSet<(Id<Post>, (Option<i64>, i64))> = (&w).map(|(((p, w), _), r)| (p, (w, r))).collect();
    let by_owner: HashIdx<Id<User>, (Id<Post>, (Option<i64>, i64))> = (&rr).map(|(p, _)| p).select(owner_user).inv().collect();
    let Vote { user, vote_type_id, post_id, .. } = &db.vote;
    let uvs = db.vote.group_by(user).select(vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let uvd = db.vote.group_by(user).select(post_id).count_distinct();
    let vs = (&uvs).and(&uvd).map(|(a, n)| [a[0], a[1], n]);
    let g = db
        .user
        .with((&db.user.reputation).gt(100))
        .group_by((&db.user.display_name).and(vs.opt()))
        .select(by_owner.opt())
        .fold([0i64; 2], |a, x| match x {
            Some((_, (w, r))) if r <= 10 => [a[0] + 1, a[1] + w.unwrap_or(0)],
            _ => a,
        });
    let v = top_n(drain(&g), |&((n, x), a)| (Reverse(a[1]), n, x), 100);
    rows(v.into_iter().map(|((n, x), a)| {
        let mut f = vec![V::S(n), V::I(a[0]), V::I(a[1])];
        f.extend(match x {
            Some(b) => [V::I(b[0]), V::I(b[1]), V::I(b[2]), V::S(if b[0] > b[1] { "Positively Influential" } else if b[1] > b[0] { "Negatively Influential" } else { "Neutral" })],
            None => [V::Null, V::Null, V::Null, V::S("Neutral")],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, p.Score, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS Rnk
//     FROM Posts p WHERE p.CreationDate >= DATE '2024-10-01' - INTERVAL '1 year'),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN p.Id IS NOT NULL THEN 1 ELSE 0 END) AS PostCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// PostWithComments AS (SELECT p.Id AS PostId, COUNT(c.Id) AS CommentCount, COALESCE(SUM(c.Score), 0) AS TotalCommentScore FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY p.Id)
// SELECT us.UserId, us.DisplayName, us.BadgeCount, us.PostCount, us.UpvoteCount, rp.Title, rp.CreationDate, pwc.CommentCount, pwc.TotalCommentScore
// FROM UserStats us JOIN RankedPosts rp ON us.UserId = rp.OwnerUserId AND rp.Rnk = 1 LEFT JOIN PostWithComments pwc ON rp.PostId = pwc.PostId
// WHERE us.BadgeCount > 0 AND (us.PostCount > 5 OR us.UpvoteCount > 10) ORDER BY us.UpvoteCount DESC, us.PostCount DESC;
//
// UserStats is only read for owners of a recent post, so the badges x posts x votes product is driven for those users alone.
fn q1743(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1)));
    let owners: MatSet<Id<User>> = recent().select(owner_user).collect();
    let us = (&owners)
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt()))
        .fold([0i64; 3], |a, (b, p)| [a[0] + b.is_some() as i64, a[1] + p.is_some() as i64, a[2] + (p.flatten() == Some(2)) as i64]);
    let w = recent().group_by(owner_user).select(Ident::<Post>::new().and(score)).window(rank, |(_, s): (Id<Post>, i64)| Reverse(s), asc);
    let first = || (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p);
    let fp: MatSet<Id<Post>> = first().collect();
    let pwc = (&fp).group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.score).opt()).fold([0i64; 2], |a, s| match s {
        Some(s) => [a[0] + 1, a[1] + s],
        None => a,
    });
    let v = drain((&us).filt(|a| a[0] > 0 && (a[1] > 5 || a[2] > 10)).and(first().select(Ident::<Post>::new().and(&pwc))));
    rows(v.into_iter().map(|(u, (a, (p, c)))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        f.extend(post_fields(db, p, &["title", "created"]));
        f.extend([V::I(c[0]), V::I(c[1])]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COUNT(DISTINCT p.Id) AS PostCount, COUNT(DISTINCT c.Id) AS CommentCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.UserId = u.Id
//     GROUP BY u.Id, u.DisplayName, u.Reputation),
// RankedUsers AS (SELECT UserId, DisplayName, Reputation, UpVotes, DownVotes, PostCount, CommentCount, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS ReputationRank,
//        RANK() OVER (ORDER BY UpVotes - DownVotes DESC) AS EngagementRank FROM UserStats)
// SELECT ru.DisplayName, ru.Reputation, ru.UpVotes, ru.DownVotes, ru.PostCount, ru.CommentCount,
//        CASE WHEN ru.ReputationRank <= 10 THEN 'Top Contributor' WHEN ru.CommentCount > 50 THEN 'Active Commentator' ELSE 'Regular User' END AS UserType,
//        (SELECT COUNT(*) FROM Posts p WHERE p.OwnerUserId = ru.UserId AND p.CreationDate >= (TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year')) AS RecentPostsCount
// FROM RankedUsers ru WHERE ru.PostCount > 5 ORDER BY EngagementRank ASC, Reputation DESC LIMIT 20;
//
// `v.UserId = u.Id` with `p.OwnerUserId = u.Id` is a vote cast by the post's owner (`own_votes`). ReputationRank ties are broken by user id.
fn q149(db: &'static So) -> String {
    let ov = own_votes(db);
    let Post { creation_date, owner_user, .. } = &db.post;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(comments_of(db).opt().and((&ov).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 2], |a, p| match p {
            Some((_, t)) => [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64],
            None => a,
        });
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let cc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let rw = whole(&db.user.reputation).select(Ident::<User>::new().and(&db.user.reputation)).window(row_number, |(u, r): (Id<User>, i64)| (Reverse(r), u), asc);
    let rm: MatSet<(Id<User>, i64)> = (&rw).map(|((u, _), r)| (u, r)).collect();
    let rr = by_first(&rm);
    let rc = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(owner_user).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let all = (&s).and(&pc).and(&cc).and((&rc).opt()).and(&rr);
    let ew = whole(&s).select(Ident::<User>::new().and(all)).window(rank, |(_, ((((a, _), _), _), _)): (Id<User>, (((([i64; 2], i64), i64), Option<i64>), i64))| Reverse(a[0] - a[1]), asc);
    let v = drain((&ew).filt(|((_, ((((_, n), _), _), _)), _)| n > 5));
    let v = top_n(v, |&(_, ((u, _), r))| (r, Reverse(db.user.reputation.get(u).unwrap()), u), 20);
    rows(v.into_iter().map(|(_, ((u, ((((a, n), c), rc), t)), _))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(n), V::I(c)]);
        f.push(V::S(if t <= 10 { "Top Contributor" } else if c > 50 { "Active Commentator" } else { "Regular User" }));
        f.push(V::I(rc.unwrap_or(0)));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS PostCount, SUM(COALESCE(P.ViewCount, 0)) AS TotalViews, SUM(COALESCE(P.Score, 0)) AS TotalScore,
//        ROW_NUMBER() OVER(ORDER BY SUM(COALESCE(P.Score, 0)) DESC) AS ActivityRank FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, TotalViews, TotalScore FROM UserActivity WHERE ActivityRank <= 10),
// PostStatistics AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.LastActivityDate, CASE WHEN P.AcceptedAnswerId IS NOT NULL THEN 'Yes' ELSE 'No' END AS HasAcceptedAnswer,
//        COALESCE(P.Score, 0) AS Score, COALESCE(COUNT(C.Id), 0) AS CommentCount, P.ViewCount, P.OwnerUserId FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId
//     GROUP BY P.Id, P.Title, P.CreationDate, P.LastActivityDate, P.AcceptedAnswerId, P.Score, P.ViewCount, P.OwnerUserId)
// SELECT TU.DisplayName, P.Title AS PostTitle, P.CreationDate, P.LastActivityDate, P.HasAcceptedAnswer, P.Score, P.CommentCount, P.ViewCount
// FROM TopUsers TU INNER JOIN PostStatistics P ON TU.UserId = P.OwnerUserId ORDER BY TU.TotalScore DESC, P.Score DESC;
fn q2399(db: &'static So) -> String {
    let Post { owner_user, score, accepted_answer_id, .. } = &db.post;
    let ts_ = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(score).opt()).fold(0i64, |n, s| n + s.unwrap_or(0));
    let tu = top_n(drain(&ts_), |&(u, s)| (Reverse(s), u), 10);
    let tus: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let ps = db.post.with(owner_user.select(Ident::<User>::new().with(&tus))).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&ps).and(owner_user));
    rows(v.into_iter().map(|(p, (c, u))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title", "created", "activity"]));
        f.push(V::S(if accepted_answer_id.get(p).is_some() { "Yes" } else { "No" }));
        f.extend([V::I(score.get(p).unwrap()), V::I(c)]);
        f.extend(post_fields(db, p, &["views"]));
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, U.DisplayName AS OwnerDisplayName, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS RowNum
//     FROM Posts P LEFT JOIN Users U ON P.OwnerUserId = U.Id LEFT JOIN Comments C ON P.Id = C.PostId WHERE P.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'
//     GROUP BY P.Id, P.Title, U.DisplayName, P.CreationDate, P.Score, P.ViewCount, P.OwnerUserId),
// PostVoteData AS (SELECT V.PostId, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, COUNT(*) AS TotalVotes
//     FROM Votes V GROUP BY V.PostId)
// SELECT RP.PostId, RP.Title, RP.CreationDate, RP.Score, RP.ViewCount, RP.OwnerDisplayName, COALESCE(PVD.UpVotes, 0) AS UpVotes, COALESCE(PVD.DownVotes, 0) AS DownVotes, RP.CommentCount,
//        CASE WHEN RP.Score > 0 THEN 'Popular' WHEN RP.Score = 0 THEN 'Neutral' ELSE 'Unpopular' END AS PopularityCategory,
//        (SELECT COUNT(*) FROM Posts P WHERE P.ParentId = RP.PostId) AS AnswerCount
// FROM RecentPosts RP LEFT JOIN PostVoteData PVD ON RP.PostId = PVD.PostId WHERE RP.RowNum = 1 ORDER BY RP.Score DESC, RP.CommentCount DESC OFFSET 0 ROWS FETCH NEXT 10 ROWS ONLY;
fn q32294(db: &'static So) -> String {
    let Post { owner_user_id, creation_date, score, .. } = &db.post;
    let w = db.post.with(creation_date.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d): (Id<Post>, i64)| (Reverse(d), p), asc);
    let rp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let g = || (&rp).group_by(Ident::<Post>::new());
    let cc = g().select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ac = g().select(children_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let pvd = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let v = top_n(drain((&cc).and(&ac).and((&pvd).opt())), |&(p, ((c, _), _))| (Reverse(score.get(p).unwrap()), Reverse(c), p), 10);
    rows(v.into_iter().map(|(p, ((c, n), a))| {
        let s = score.get(p).unwrap();
        let a = a.unwrap_or([0, 0]);
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c), V::S(if s > 0 { "Popular" } else if s == 0 { "Neutral" } else { "Unpopular" }), V::I(n)]);
        row(f)
    }))
}

// WITH RecursivePostHistory AS (SELECT p.Id AS PostId, ph.CreationDate, ph.UserId, ph.Comment, ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY ph.CreationDate DESC) AS rn
//     FROM Posts p JOIN PostHistory ph ON p.Id = ph.PostId WHERE ph.PostHistoryTypeId IN (1, 4, 10)),
// PostMetrics AS (SELECT p.Id, p.Title, COUNT(DISTINCT c.Id) AS CommentsCount, COUNT(DISTINCT v.Id) AS VotesCount, AVG(u.Reputation) AS AvgUserReputation, MAX(ph.CreationDate) AS LastPostHistoryDate
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN RecursivePostHistory ph ON p.Id = ph.PostId
//     GROUP BY p.Id, p.Title),
// RankedPosts AS (SELECT pm.Id, pm.Title, pm.CommentsCount, pm.VotesCount, pm.AvgUserReputation, pm.LastPostHistoryDate, RANK() OVER (ORDER BY pm.VotesCount DESC, pm.CommentsCount DESC) AS PostRank
//     FROM PostMetrics pm)
// SELECT rp.Title, rp.CommentsCount, rp.VotesCount, COALESCE(rp.AvgUserReputation, 0) AS AvgUserReputation, CASE WHEN rp.LastPostHistoryDate IS NOT NULL THEN 'Active' ELSE 'Inactive' END AS PostStatus
// FROM RankedPosts rp WHERE rp.PostRank <= 10 ORDER BY rp.PostRank;
//
// Not recursive: the CTE never refers to itself, and its rn is never read. PostRank reads only the two distinct counts, which are one fold each, so the top posts
// are ranked first and the comments x votes x history product is driven for them alone.
fn q33577(db: &'static So) -> String {
    let g = || db.post.group_by(Ident::<Post>::new());
    let vc = g().select(votes_of(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let cc = g().select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let w = whole(&vc).select(Ident::<Post>::new().and(&vc).and(&cc)).window(rank, |((_, v), c): ((Id<Post>, i64), i64)| (Reverse(v), Reverse(c)), asc);
    let tp: MatSet<(Id<Post>, (i64, i64))> = (&w).filt(|(_, r)| r <= 10).map(|(((p, v), c), _)| (p, (v, c))).collect();
    let tps: MatSet<Id<Post>> = (&tp).map(|(p, _)| p).collect();
    let ph = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([1, 4, 10]))).select(&db.post_history.creation_date);
    let pm = (&tps)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).opt()).and((&db.post.owner_user).select(&db.user.reputation).opt()).and(ph.opt()))
        .fold((0i64, 0i64, i64::MIN), |(s, n, m), (((_, _), r), d)| (s + r.unwrap_or(0), n + r.is_some() as i64, m.max(d.unwrap_or(i64::MIN))));
    let v = drain(by_first(&tp).and(&pm));
    rows(v.into_iter().map(|(p, ((vn, cn), (s, n, m)))| {
        let mut f = post_fields(db, p, &["title"]);
        f.extend([V::I(cn), V::I(vn), V::F(if n == 0 { 0.0 } else { s as f64 / n as f64 }), V::S(if m == i64::MIN { "Inactive" } else { "Active" })]);
        row(f)
    }))
}

// WITH PostMetrics AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, COALESCE(COUNT(c.Id), 0) AS CommentCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount, DENSE_RANK() OVER (ORDER BY p.Score DESC) AS ScoreRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Score, p.ViewCount, p.CreationDate),
// TopPosts AS (SELECT PostId, Title, Score, ViewCount, CreationDate, CommentCount, UpVoteCount, DownVoteCount, ScoreRank FROM PostMetrics WHERE Score > (SELECT AVG(Score) FROM PostMetrics)
//     ORDER BY ScoreRank LIMIT 10)
// SELECT tp.PostId, tp.Title, tp.Score, tp.ViewCount, tp.CreationDate, tp.CommentCount, tp.UpVoteCount, tp.DownVoteCount, pt.Name AS PostType,
//        CASE WHEN tp.CommentCount > 5 THEN 'Highly Engaged' WHEN tp.Score > 100 THEN 'Popular' ELSE 'Standard' END AS EngagementLevel
// FROM TopPosts tp LEFT JOIN PostTypes pt ON pt.Id = (SELECT DISTINCT p.PostTypeId FROM Posts p WHERE p.Id = tp.PostId)
// WHERE tp.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' ORDER BY tp.Score DESC;
//
// ScoreRank and the AVG read only Score, so the ten questions are picked first (ties at the LIMIT broken by post id) and the comments x votes product is driven
// for them alone. The correlated subquery looks a post up by its primary key, so the type is the post's own. The answer is empty on this data.
fn q3167(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, .. } = &db.post;
    let qs = || db.post.with(post_type_id.eq(1));
    let (s, n) = qs().select(score).fold_flat((0i64, 0i64), |(s, n), x| (s + x, n + 1));
    let v = top_n(drain(qs().select(score).filt(move |x: i64| (x as i128) * (n as i128) > s as i128)), |&(p, x)| (Reverse(x), p), 10);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pm = (&tp)
        .with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = drain(&pm);
    rows(v.into_iter().map(|(p, a)| {
        let sc = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "created"]);
        f.extend(a.map(V::I));
        f.extend(post_fields(db, p, &["type"]));
        f.push(V::S(if a[0] > 5 { "Highly Engaged" } else if sc > 100 { "Popular" } else { "Standard" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COUNT(DISTINCT a.Id) AS AnswerCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes, COUNT(c.Id) AS CommentCount, ROW_NUMBER() OVER (ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Posts a ON p.Id = a.ParentId AND p.PostTypeId = 1 LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.CreationDate),
// TopPosts AS (SELECT PostId, Title, CreationDate, AnswerCount, Upvotes, Downvotes, CommentCount FROM RankedPosts WHERE Rank <= 10)
// SELECT u.DisplayName AS Author, tp.Title, tp.CreationDate, tp.AnswerCount, tp.Upvotes, tp.Downvotes, tp.CommentCount, COALESCE(b.Name, 'No Badge') AS UserBadge, COUNT(l.Id) AS RelatedLinksCount
// FROM TopPosts tp JOIN Users u ON tp.PostId = u.Id LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN PostLinks l ON tp.PostId = l.PostId
// GROUP BY u.DisplayName, tp.Title, tp.CreationDate, tp.AnswerCount, tp.Upvotes, tp.Downvotes, tp.CommentCount, b.Name ORDER BY tp.Upvotes DESC;
//
// Rank reads only CreationDate, so the ten questions are picked first (ties broken by post id) and the answers x votes x comments product is driven for them
// alone. `tp.PostId = u.Id` joins a post id to a user id, so it goes through the raw ids. The final GROUP BY names b.Name, so the (post, badge) rows are
// materialised and grouped. The answer is empty on this data.
fn q8231(db: &'static So) -> String {
    let Post { post_type_id, creation_date, origid, title, .. } = &db.post;
    let v = top_n(drain(db.post.with(post_type_id.eq(1)).with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(creation_date)), |&(p, d)| (Reverse(d), p), 10);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let rp = (&tp)
        .group_by(Ident::<Post>::new())
        .select(children_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(comments_of(db).opt()))
        .fold([0i64; 3], |a, ((_, t), c)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64]);
    let ac = (&tp).group_by(Ident::<Post>::new()).select(children_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    type K = (Option<Str>, i64, i64, [i64; 3]);
    type J = ((Id<Post>, K), (Str, Option<(Id<Badge>, Str)>));
    let cols = title.opt().and(creation_date).and(&ac).and(&rp).map(|(((t, d), k), a)| (t, d, k, a));
    let ub = origid.select(&uidx).select((&db.user.display_name).and(badges_of(db).select(Ident::<Badge>::new().and(&db.badge.name)).opt()));
    let j: MatSet<J> = (&tp).select(Ident::<Post>::new().and(cols).and(ub)).collect();
    let g = (&j)
        .group_by(Same::<J>::new().map(|((_, k), (n, b)): J| (n, k, b.map(|b| b.1))))
        .select(Same::<J>::new().map(|((p, _), _): J| p).select(links_of(db).opt()))
        .fold(0i64, |n, l| n + l.is_some() as i64);
    rows(drain(&g).into_iter().map(|((n, (t, d, k, a), b), l)| row(vec![V::S(n), ostr(t), V::T(d), V::I(k), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::S(b.unwrap_or("No Badge")), V::I(l)])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// FilteredUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation
//     HAVING AVG(u.Reputation) > 1000 OR COUNT(b.Id) > 5),
// RecentComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c WHERE c.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' GROUP BY c.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, COALESCE(rc.CommentCount, 0) AS CommentCount, f.UserId, f.DisplayName, f.Reputation, f.BadgeCount,
//        CASE WHEN rp.Score IS NULL THEN 'No Score' ELSE CASE WHEN rp.Score > 10 THEN 'Highly Rated' ELSE 'Moderately Rated' END END AS RatingStatus
// FROM RankedPosts rp LEFT JOIN RecentComments rc ON rp.PostId = rc.PostId JOIN FilteredUsers f ON f.UserId = rp.PostId WHERE rp.Rank <= 5 ORDER BY rp.ViewCount DESC, rp.CreationDate DESC LIMIT 50;
//
// `f.UserId = rp.PostId` joins a user id to a post id, so it goes through the raw ids. AVG(u.Reputation) is the fold's sum over its row count, compared exactly.
// The answer is empty on this data.
fn q4363(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, origid, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let w = db.post.with(creation_date.ge(add_years(t0, -1))).group_by(post_type_id).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d): (Id<Post>, i64)| (Reverse(d), p), asc);
    let rp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let fu = db
        .user
        .group_by(Ident::<User>::new())
        .select((&db.user.reputation).and(badges_of(db).opt()))
        .fold([0i64; 3], |a, (r, b)| [a[0] + r, a[1] + 1, a[2] + b.is_some() as i64]);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let fuj = Ident::<User>::new().and((&fu).filt(|a: [i64; 3]| (a[0] as i128) > 1000 * a[1] as i128 || a[2] > 5));
    let rc = db.comment.with((&db.comment.creation_date).ge(add_days(t0, -30))).group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&rp).select(origid.select(&uidx).select(fuj).and((&rc).opt())));
    let v = top_n(v, |&(p, _)| (Reverse(view_count.get(p)), Reverse(creation_date.get(p).unwrap()), p), 50);
    rows(v.into_iter().map(|(p, ((u, a), c))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.push(V::I(c.unwrap_or(0)));
        f.extend(ucols(db, u, &["uid", "name", "rep"]));
        f.extend([V::I(a[2]), V::S(if score.get(p).unwrap() > 10 { "Highly Rated" } else { "Moderately Rated" })]);
        row(f)
    }))
}

// WITH UserBadgeCounts AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// UserActivity AS (SELECT U.Id AS UserId, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN C.Id IS NOT NULL THEN 1 ELSE 0 END) AS CommentCount FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId GROUP BY U.Id),
// UserPostStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COALESCE(UBC.BadgeCount, 0) AS BadgeCount, COALESCE(UA.PostCount, 0) AS PostCount, COALESCE(UA.QuestionCount, 0) AS QuestionCount,
//        COALESCE(UA.AnswerCount, 0) AS AnswerCount, COALESCE(UA.CommentCount, 0) AS CommentCount FROM Users U LEFT JOIN UserBadgeCounts UBC ON U.Id = UBC.UserId LEFT JOIN UserActivity UA ON U.Id = UA.UserId)
// SELECT UserId, DisplayName, Reputation, BadgeCount, PostCount, QuestionCount, AnswerCount, CommentCount, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank
// FROM UserPostStats WHERE Reputation > 1000 ORDER BY ReputationRank, Reputation DESC;
//
// ReputationRank is taken after the WHERE, over the users with Reputation > 1000.
fn q9609(db: &'static So) -> String {
    let users = || db.user.with((&db.user.reputation).gt(1000));
    let bc = users().group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let ua = users()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(comments_of(db).opt())).opt())
        .fold([0i64; 3], |a, p| match p {
            Some((t, c)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + c.is_some() as i64],
            None => a,
        });
    let pc = users().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let w = whole(users()).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r): (Id<User>, i64)| Reverse(r), asc);
    let rk: MatSet<(Id<User>, i64)> = (&w).map(|((u, _), r)| (u, r)).collect();
    let v = drain((&bc).and(&pc).and(&ua).and(by_first(&rk)));
    rows(v.into_iter().map(|(u, (((b, n), a), r))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(b), V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 YEAR'),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(DISTINCT b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostVoteCounts AS (SELECT p.Id AS PostId, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id)
// SELECT rp.PostId, rp.Title, rp.Score, pv.UpVoteCount, pv.DownVoteCount, ub.BadgeCount, CASE WHEN rp.Score > 0 THEN 'Positive' WHEN rp.Score < 0 THEN 'Negative' ELSE 'Neutral' END AS ScoreCategory,
//        pht.Name AS PostHistoryType
// FROM RankedPosts rp LEFT JOIN PostVoteCounts pv ON rp.PostId = pv.PostId JOIN Users u ON rp.PostId = u.Id LEFT JOIN UserBadges ub ON u.Id = ub.UserId
// LEFT JOIN PostHistory ph ON rp.PostId = ph.PostId LEFT JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id WHERE rp.Rank = 1 ORDER BY rp.Score DESC, ub.BadgeCount DESC;
//
// `rp.PostId = u.Id` joins a post id to a user id, so it goes through the raw ids. The ROW_NUMBER breaks Score ties by post id (the SQL leaves them open; the
// answer is empty on this data).
fn q4850(db: &'static So) -> String {
    let Post { owner_user_id, creation_date, score, origid, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s): (Id<Post>, i64)| (Reverse(s), p), asc);
    let rp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let pv = (&rp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = drain((&pv).and(origid.select(&uidx).select(&ub)).and(history_of(db).select(htype_name(db)).opt()));
    rows(v.into_iter().map(|(p, ((a, b), h))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "score"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(b), V::S(if s > 0 { "Positive" } else if s < 0 { "Negative" } else { "Neutral" }), ostr(h)]);
        row(f)
    }))
}

// WITH RankedUsers AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U WHERE U.Reputation IS NOT NULL),
// UserPostStats AS (SELECT U.Id AS UserId, COUNT(P.Id) AS TotalPosts, COUNT(CASE WHEN P.ParentId IS NULL THEN 1 END) AS TotalQuestions, COUNT(CASE WHEN P.ParentId IS NOT NULL THEN 1 END) AS TotalAnswers
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id)
// SELECT RU.DisplayName, RU.Reputation, UPS.TotalPosts, UPS.TotalQuestions, UPS.TotalAnswers, COALESCE(PHT.PostHistoryCount, 0) AS PostHistoryCount, COALESCE(PVC.ViewCount, 0) AS MostViewedPost,
//        P.Title AS MostViewedPostTitle
// FROM RankedUsers RU JOIN UserPostStats UPS ON RU.UserId = UPS.UserId
// LEFT JOIN (SELECT PH.UserId, COUNT(PH.Id) AS PostHistoryCount FROM PostHistory PH WHERE PH.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year' GROUP BY PH.UserId) PHT ON RU.UserId = PHT.UserId
// LEFT JOIN (SELECT P.OwnerUserId, MAX(P.ViewCount) AS ViewCount FROM Posts P GROUP BY P.OwnerUserId) PVC ON RU.UserId = PVC.OwnerUserId
// LEFT JOIN Posts P ON P.OwnerUserId = RU.UserId AND P.ViewCount = PVC.ViewCount WHERE RU.ReputationRank <= 10 ORDER BY RU.Reputation DESC LIMIT 10;
fn q24651(db: &'static So) -> String {
    let Post { owner_user, parent_id, view_count, .. } = &db.post;
    let w = whole(&db.user.reputation).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r): (Id<User>, i64)| Reverse(r), asc);
    let tu: MatSet<Id<User>> = (&w).filt(|(_, r)| r <= 10).map(|((u, _), _)| u).collect();
    let g = || (&tu).group_by(Ident::<User>::new());
    let ups = g().select(posts_of(db).select(parent_id.opt()).opt()).fold([0i64; 3], |a, p| match p {
        Some(x) => [a[0] + 1, a[1] + x.is_none() as i64, a[2] + x.is_some() as i64],
        None => a,
    });
    let PostHistory { user, creation_date: hd, .. } = &db.post_history;
    let pht = db.post_history.with(hd.ge(add_years(date(2024, 10, 1), -1))).group_by(user).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let pvc = db.post.with(view_count).group_by(owner_user).select(view_count).fold(i64::MIN, |m, w| m.max(w));
    let at: HashIdx<(Id<User>, i64), Id<Post>> = db.post.select(owner_user.and(view_count)).inv().collect();
    let v = drain((&ups).and((&pht).opt()).and(Ident::<User>::new().and(&pvc).select(&at).opt()).and((&pvc).opt()));
    let v = top_n(v, |&(u, (((_, _), p), _))| (Reverse(db.user.reputation.get(u).unwrap()), u, p), 10);
    rows(v.into_iter().map(|(u, (((a, h), p), m))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(h.unwrap_or(0)), V::I(m.unwrap_or(0))]);
        f.push(p.map_or(V::Null, |p| ostr(db.post.title.get(p))));
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN p.PostTypeId = 2 AND p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswerCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COALESCE(SUM(c.Score), 0) AS CommentScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, AcceptedAnswerCount, UpVotes, DownVotes, CommentScore, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank
//     FROM UserStats)
// SELECT t.DisplayName, t.Reputation, t.PostCount, t.QuestionCount, t.AnswerCount, t.AcceptedAnswerCount, t.UpVotes, t.DownVotes, t.CommentScore, (SELECT AVG(Reputation) FROM UserStats) AS AvgReputation
// FROM TopUsers t WHERE t.ReputationRank <= 10 ORDER BY t.Reputation DESC;
//
// ReputationRank reads only Reputation, so the top users are picked first and the posts x votes x comments product is driven for them alone. UserStats has one
// row per user, so its AVG(Reputation) is over Users.
fn q9328(db: &'static So) -> String {
    let w = whole(&db.user.reputation).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r): (Id<User>, i64)| Reverse(r), asc);
    let tu: MatSet<Id<User>> = (&w).filt(|(_, r)| r <= 10).map(|((u, _), _)| u).collect();
    let Post { post_type_id, accepted_answer_id, .. } = &db.post;
    let s = (&tu)
        .group_by(Ident::<User>::new())
        .select(
            posts_of(db)
                .select(post_type_id.and(accepted_answer_id.opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(comments_of(db).select(&db.comment.score).opt()))
                .opt(),
        )
        .fold([0i64; 6], |a, p| match p {
            Some((((t, x), v), c)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (t == 2 && x.is_some()) as i64, a[3] + (v == Some(2)) as i64, a[4] + (v == Some(3)) as i64, a[5] + c.unwrap_or(0)],
            None => a,
        });
    let pc = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let (rs, rn) = db.user.select(&db.user.reputation).fold_flat((0i64, 0i64), |(s, n), r| (s + r, n + 1));
    let v = drain((&pc).and(&s));
    rows(v.into_iter().map(|(u, (n, a))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.push(avg(rs, rn));
        row(f)
    }))
}

// WITH UserVoteStats AS (SELECT U.Id AS UserId, U.DisplayName, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesCount, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesCount,
//        COUNT(DISTINCT P.Id) AS PostsCount FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId LEFT JOIN Posts P ON P.OwnerUserId = U.Id GROUP BY U.Id, U.DisplayName),
// PostStatistics AS (SELECT P.Id AS PostId, P.Title, P.Score, P.AnswerCount, P.ViewCount, COALESCE(AVG(CASE WHEN C.Score IS NOT NULL THEN C.Score ELSE 0 END), 0) AS AvgCommentScore
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId GROUP BY P.Id, P.Title, P.Score, P.AnswerCount, P.ViewCount),
// TopQuestions AS (SELECT PS.PostId, PS.Title, PS.Score, PS.AnswerCount, PS.ViewCount, RANK() OVER (ORDER BY PS.Score DESC) AS RankScore FROM PostStatistics PS WHERE PS.AnswerCount > 0)
// SELECT U.UserId, U.DisplayName, U.UpVotesCount, U.DownVotesCount, U.PostsCount, TQ.Title AS TopQuestionTitle, TQ.Score AS TopQuestionScore, TQ.AnswerCount AS TopQuestionAnswers,
//        TQ.ViewCount AS TopQuestionViews
// FROM UserVoteStats U LEFT JOIN TopQuestions TQ ON U.PostsCount > 0 AND TQ.RankScore <= 5 WHERE U.UpVotesCount IS NOT NULL ORDER BY U.UpVotesCount DESC, U.DisplayName;
//
// PostStatistics has one row per post and its AvgCommentScore is never read. The ON clause names TQ only through RankScore, so users with posts are crossed
// with the top-ranked questions and the others keep one NULL row.
fn q2793(db: &'static So) -> String {
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt().and(posts_of(db).opt()))
        .fold([0i64; 2], |a, (t, _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let tq = db.post.with((&db.post.answer_count).gt(0));
    let w = whole(&tq).select(Ident::<Post>::new().and(&db.post.score)).window(rank, |(_, s): (Id<Post>, i64)| Reverse(s), asc);
    let top: HashIdx<(), Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let us = (&s).and(&pc);
    let on = (&us).filt(|(_, n)| n > 0).map(|_| ()).select(&top);
    let v = drain((&us).and(on.opt()));
    rows(v.into_iter().map(|(u, ((a, n), p))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(n)]);
        f.extend(match p {
            Some(p) => post_fields(db, p, &["title", "score", "answers", "views"]),
            None => vec![V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH PostMetrics AS (SELECT p.Id AS PostId, p.Title, p.Score, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
//        COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT ph.Id) AS EditCount
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.Score),
// RankedPosts AS (SELECT PostId, Title, Score, UpVotes, DownVotes, CommentCount, EditCount, RANK() OVER (ORDER BY Score DESC, UpVotes DESC) AS ScoreRank FROM PostMetrics)
// SELECT rp.PostId, rp.Title, rp.Score, rp.UpVotes, rp.DownVotes, rp.CommentCount, rp.EditCount,
//        CASE WHEN rp.EditCount > 5 THEN 'Highly Edited' WHEN rp.EditCount BETWEEN 3 AND 5 THEN 'Moderately Edited' ELSE 'Slightly Edited' END AS EditStatus,
//        CASE WHEN rp.DownVotes > rp.UpVotes THEN 'More Downvotes than Upvotes' WHEN rp.UpVotes > rp.DownVotes THEN 'More Upvotes than Downvotes' ELSE 'Equal Votes' END AS VoteSummary
// FROM RankedPosts rp WHERE rp.ScoreRank <= 10 ORDER BY rp.Score DESC, rp.UpVotes DESC;
fn q2459(db: &'static So) -> String {
    let base = || db.post.with((&db.post.creation_date).ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let recent = || base().group_by(Ident::<Post>::new());
    let pm = recent()
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()).and(history_of(db).opt()))
        .fold([0i64; 2], |a, ((t, _), _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = recent().select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ec = recent().select(history_of(db).opt()).fold(0i64, |n, h| n + h.is_some() as i64);
    let w = whole(base())
        .select(Ident::<Post>::new().and(&db.post.score).and((&pm).and(&cc).and(&ec)))
        .window(rank, |((_, s), ((a, _), _)): ((Id<Post>, i64), (([i64; 2], i64), i64))| (Reverse(s), Reverse(a[0])), asc);
    let v = drain((&w).filt(|(_, r)| r <= 10));
    rows(v.into_iter().map(|(_, (((p, _), ((a, c), e)), _))| {
        let mut f = post_fields(db, p, &["id", "title", "score"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c), V::I(e)]);
        f.push(V::S(if e > 5 { "Highly Edited" } else if e >= 3 { "Moderately Edited" } else { "Slightly Edited" }));
        f.push(V::S(if a[1] > a[0] { "More Downvotes than Upvotes" } else if a[0] > a[1] { "More Upvotes than Downvotes" } else { "Equal Votes" }));
        row(f)
    }))
}

// WITH UserScoreSummary AS (SELECT U.Id AS UserId, U.DisplayName, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COALESCE(SUM(CASE WHEN V.VoteTypeId IN (8, 9) THEN V.BountyAmount ELSE 0 END), 0) AS TotalBounty,
//        COUNT(DISTINCT P.Id) AS PostCount, COUNT(DISTINCT C.Id) AS CommentCount
//     FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId GROUP BY U.Id, U.DisplayName),
// RankedUsers AS (SELECT UserId, DisplayName, UpVotes, DownVotes, TotalBounty, PostCount, CommentCount, RANK() OVER (ORDER BY (UpVotes - DownVotes) + TotalBounty DESC) AS UserRank FROM UserScoreSummary),
// ActivePosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.LastActivityDate, COALESCE((SELECT COUNT(*) FROM Comments WHERE PostId = P.Id), 0) AS CommentCount FROM Posts P
//     WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year')
// SELECT RU.UserRank, RU.DisplayName, RU.PostCount, RU.CommentCount, AP.PostId, AP.Title, AP.CreationDate, AP.LastActivityDate, AP.CommentCount AS PostCommentCount
// FROM RankedUsers RU JOIN ActivePosts AP ON RU.PostCount > 0 ORDER BY RU.UserRank, AP.LastActivityDate DESC LIMIT 25;
//
// The ON clause names only RU, so ranked users with posts are crossed with the active posts. A row reaches the first 25 only if fewer than 25 rows of its own
// side order strictly before it, so each side is cut at a second RANK <= 25 before the product.
fn q2525(db: &'static So) -> String {
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(vote_type_id.and(bounty_amount.opt())).opt().and(posts_of(db).select(comments_of(db).opt()).opt()))
        .fold(0i64, |n, (v, _)| match v {
            Some((t, b)) => n + (t == 2) as i64 - (t == 3) as i64 + if matches!(t, 8 | 9) { b.unwrap_or(0) } else { 0 },
            None => n,
        });
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let cc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let w = whole(&s).select(Ident::<User>::new().and(&s)).window(rank, |(_, n): (Id<User>, i64)| Reverse(n), asc);
    let rk: MatSet<(Id<User>, i64)> = (&w).map(|((u, _), r)| (u, r)).collect();
    let rk = by_first(&rk);
    let ru = (&pc).filt(|n| n > 0).and(&cc).and(&rk);
    let ap = db
        .post
        .with((&db.post.creation_date).ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt())
        .fold(0i64, |n, c| n + c.is_some() as i64);
    let la = &db.post.last_activity_date;
    let uw = whole(&ru).select(Ident::<User>::new().and(&ru)).window(rank, |(_, (_, r)): (Id<User>, ((i64, i64), i64))| r, asc);
    let aw = whole(&ap).select(Ident::<Post>::new().and(&ap).and(la)).window(rank, |(_, d): ((Id<Post>, i64), i64)| Reverse(d), asc);
    let uc = (&uw).filt(|(_, k)| k <= 25).map(|(x, _)| x);
    let pcand = (&aw).filt(|(_, k)| k <= 25).map(|(x, _)| x);
    let v = top_n(drain(uc.and(pcand)), |&(_, ((u, (_, r)), ((p, _), d)))| (r, Reverse(d), u, p), 25);
    rows(v.into_iter().map(|(_, ((u, ((n, c), r)), ((p, k), _)))| {
        let mut f = vec![V::I(r), user_col(db, u, "name"), V::I(n), V::I(c)];
        f.extend(post_fields(db, p, &["id", "title", "created", "activity"]));
        f.push(V::I(k));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.Tags, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate ASC) AS PostRank
//     FROM Posts p WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '1 year'),
// ClosedPosts AS (SELECT ph.PostId, COUNT(ph.Id) AS CloseCount, MAX(ph.CreationDate) AS LastCloseDate FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(v.Id) AS VoteCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName)
// SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.Tags, COALESCE(cp.CloseCount, 0) AS CloseCount, cp.LastCloseDate, ua.UserId, ua.DisplayName, ua.VoteCount, ua.UpVotes, ua.DownVotes,
//        CASE WHEN ua.UserId IS NULL THEN 'No Activity' ELSE 'Active User' END AS UserStatus
// FROM RankedPosts rp LEFT JOIN ClosedPosts cp ON rp.PostId = cp.PostId LEFT JOIN UserActivity ua ON rp.Score > 5 AND ua.VoteCount > 10 WHERE rp.PostRank <= 5 ORDER BY rp.CreationDate DESC;
//
// The second ON clause names ua only through VoteCount, so each ranked post with Score > 5 is crossed with the active voters (or keeps one NULL row when there
// are none), and every other post keeps one NULL row. The PostRank ties are broken by post id. The answer is empty on this data.
fn q30505(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(current_date(), -1)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d): ((Id<Post>, i64), i64)| (Reverse(s), d, p), asc);
    let rp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|(((p, _), _), _)| p).collect();
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let cp = db.post_history.with(post_history_type_id.eq(10)).group_by(post).select(hd).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let ua = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64],
        None => a,
    });
    let act = (&ua).filt(|a| a[0] > 10);
    let actv: HashIdx<(), (Id<User>, [i64; 3])> = whole(&act).select(Ident::<User>::new().and(&act)).collect();
    let on = (&rp).with(score.gt(5)).map(|_| ()).select(&actv);
    let v = drain((&rp).select(Ident::<Post>::new().and((&cp).opt()).and(on.opt())));
    rows(v.into_iter().map(|(_, ((p, c), a))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "tags"]);
        f.extend(match c {
            Some((n, m)) => [V::I(n), V::T(m)],
            None => [V::I(0), V::Null],
        });
        f.extend(match a {
            Some((u, a)) => [user_col(db, u, "uid"), user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::S("Active User")],
            None => [V::Null, V::Null, V::Null, V::Null, V::Null, V::S("No Activity")],
        });
        row(f)
    }))
}

// Rewritten (rewrites/2496.sql): the RecentPostRank ROW_NUMBER is tie-broken on p.Id.
// WITH UserMetrics AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS TotalPosts, COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS TotalQuestions,
//        COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS TotalAnswers, SUM(COALESCE(p.Score, 0)) AS TotalScore, SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounty
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 8 GROUP BY u.Id, u.DisplayName, u.Reputation),
// RankedUsers AS (SELECT *, ROW_NUMBER() OVER (ORDER BY Reputation DESC, TotalScore DESC) AS Rank FROM UserMetrics),
// RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC, p.Id) AS RecentPostRank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year')
// SELECT ru.Rank, ru.DisplayName, ru.Reputation, ru.TotalPosts, ru.TotalQuestions, ru.TotalAnswers, ru.TotalScore, ru.TotalBounty, rp.PostId, rp.Title AS RecentPostTitle, rp.CreationDate AS RecentPostDate
// FROM RankedUsers ru LEFT JOIN RecentPosts rp ON ru.UserId = rp.OwnerUserId AND rp.RecentPostRank = 1 WHERE ru.TotalPosts > 10 ORDER BY ru.Rank LIMIT 10;
//
// The Rank ties are broken by user id. The distinct question and answer counts are over one row per post.
fn q2496(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, creation_date, .. } = &db.post;
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let um = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(score.and(bounty.opt())).opt()).fold([0i64; 2], |a, p| match p {
        Some((s, b)) => [a[0] + s, a[1] + b.flatten().unwrap_or(0)],
        None => a,
    });
    let pq = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id).opt()).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64],
        None => a,
    });
    let w = whole(&um)
        .select(Ident::<User>::new().and(&db.user.reputation).and((&um).and(&pq)))
        .window(row_number, |((u, r), (a, _)): ((Id<User>, i64), ([i64; 2], [i64; 3]))| (Reverse(r), Reverse(a[0]), u), asc);
    let ru = top_n(drain((&w).filt(|((_, (_, c)), _)| c[0] > 10)), |&(_, (_, r))| r, 10);
    let tr: MatSet<(Id<User>, (([i64; 2], [i64; 3]), i64))> = rel(ru.into_iter().map(|(_, (((u, _), x), r))| (u, (x, r))).collect()).map(|x| x).collect();
    let tus: MatSet<Id<User>> = (&tr).map(|(u, _)| u).collect();
    let rw = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(owner_user.select(Ident::<User>::new().with(&tus)))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d): (Id<Post>, i64)| (Reverse(d), p), asc);
    let last = (&rw).filt(|(_, r)| r == 1).map(|((p, _), _)| p);
    let v = drain(by_first(&tr).and(last.opt()));
    rows(v.into_iter().map(|(u, (((a, c), r), p))| {
        let mut f = vec![V::I(r)];
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::I(c[0]), V::I(c[1]), V::I(c[2]), V::I(a[0]), V::I(a[1])]);
        f.extend(match p {
            Some(p) => post_fields(db, p, &["id", "title", "created"]),
            None => vec![V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.Score, p.AnswerCount, ROW_NUMBER() OVER(PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(DISTINCT b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation),
// PostStats AS (SELECT rp.OwnerUserId, SUM(rp.Score) AS TotalScore, SUM(rp.AnswerCount) AS TotalAnswers, COALESCE(u.Reputation, 0) AS UserReputation, COALESCE(ub.BadgeCount, 0) AS UserBadgeCount
//     FROM RankedPosts rp LEFT JOIN UserReputation u ON rp.OwnerUserId = u.UserId LEFT JOIN (SELECT UserId, COUNT(DISTINCT Id) AS BadgeCount FROM Badges GROUP BY UserId) ub ON rp.OwnerUserId = ub.UserId
//     GROUP BY rp.OwnerUserId, u.Reputation, ub.BadgeCount)
// SELECT ps.OwnerUserId, ps.TotalScore, ps.TotalAnswers, ps.UserReputation, ps.UserBadgeCount,
//        CASE WHEN ps.UserReputation >= 1000 THEN 'Gold' WHEN ps.UserReputation >= 500 THEN 'Silver' ELSE 'Bronze' END AS ReputationTier
// FROM PostStats ps WHERE ps.TotalAnswers > 5 ORDER BY ps.TotalScore DESC FETCH FIRST 10 ROWS ONLY;
//
// The ownerless posts form their own NULL group, as SQL's GROUP BY does. rn is never read.
fn q1470(db: &'static So) -> String {
    let Post { owner_user, owner_user_id, creation_date, score, answer_count, .. } = &db.post;
    let ps = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(owner_user_id.opt())
        .select(score.and(answer_count.opt()).and(owner_user.opt()))
        .fold((0i64, 0i64, 0i64, None::<Id<User>>), |(s, n, a, _), ((x, c), u)| (s + x, n + c.is_some() as i64, a + c.unwrap_or(0), u));
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = top_n(drain((&ps).filt(|(_, n, a, _)| n > 0 && a > 5)), |&(k, (s, _, _, _))| (Reverse(s), k), 10);
    type R = (Option<i64>, (i64, i64, i64, Option<Id<User>>));
    let v = drain(rel(v).select(Same::<R>::new().and(Same::<R>::new().flat_map(|(_, (_, _, _, u))| u).select(&bc).opt())));
    rows(v.into_iter().map(|(_, ((k, (s, _, a, u)), b))| {
        let r = u.map_or(0, |u| db.user.reputation.get(u).unwrap());
        row(vec![oint(k), V::I(s), V::I(a), V::I(r), V::I(b.unwrap_or(0)), V::S(if r >= 1000 { "Gold" } else if r >= 500 { "Silver" } else { "Bronze" })])
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U WHERE U.Reputation IS NOT NULL),
// PostStats AS (SELECT P.OwnerUserId, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
//        AVG(P.Score) AS AvgScore FROM Posts P WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY P.OwnerUserId),
// TopUsers AS (SELECT UR.UserId, UR.DisplayName, UR.Reputation, PS.TotalPosts, PS.TotalQuestions, PS.TotalAnswers, PS.AvgScore FROM UserReputation UR JOIN PostStats PS ON UR.UserId = PS.OwnerUserId
//     WHERE UR.ReputationRank <= 10)
// SELECT TU.DisplayName, TU.Reputation, TU.TotalPosts, TU.TotalQuestions, COALESCE(TU.TotalAnswers, 0) AS TotalAnswers, COALESCE(TU.AvgScore, 0) AS AvgScore,
//        CASE WHEN TU.TotalPosts > 50 THEN 'Active Contributor' WHEN TU.TotalPosts BETWEEN 20 AND 50 THEN 'Regular Contributor' ELSE 'New Contributor' END AS ContributorType
// FROM TopUsers TU LEFT JOIN Badges B ON TU.UserId = B.UserId AND B.Class = 1 WHERE B.Id IS NULL ORDER BY TU.Reputation DESC, TU.TotalPosts DESC;
fn q4232(db: &'static So) -> String {
    let Post { owner_user, creation_date, post_type_id, score, .. } = &db.post;
    let w = whole(&db.user.reputation).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r): (Id<User>, i64)| Reverse(r), asc);
    let tu: MatSet<Id<User>> = (&w).filt(|(_, r)| r <= 10).map(|((u, _), _)| u).collect();
    let ps = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(owner_user)
        .select(post_type_id.and(score))
        .fold([0i64; 4], |a, (t, s)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s]);
    let gold: MatSet<Id<User>> = db.badge.with((&db.badge.class).eq(1)).select(&db.badge.user).collect();
    let v = drain((&tu).minus(&gold).select(&ps));
    rows(v.into_iter().map(|(u, a)| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[3], a[0])]);
        f.push(V::S(if a[0] > 50 { "Active Contributor" } else if a[0] >= 20 { "Regular Contributor" } else { "New Contributor" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, COUNT(c.Id) AS CommentCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p LEFT JOIN Comments c ON c.PostId = p.Id WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.OwnerUserId),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COALESCE(SUM(b.Class), 0) AS TotalBadges, MAX(p.CreationDate) AS LastPostDate
//     FROM Users u LEFT JOIN Badges b ON b.UserId = u.Id LEFT JOIN Posts p ON p.OwnerUserId = u.Id GROUP BY u.Id, u.Reputation),
// ClosedPosts AS (SELECT ph.PostId, ph.CreationDate, ph.UserDisplayName, ph.Comment FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.CommentCount, ur.Reputation, COALESCE(ur.TotalBadges, 0) AS BadgeCount, cp.UserDisplayName AS CloseBy, cp.CreationDate AS CloseDate,
//        CASE WHEN cp.UserDisplayName IS NOT NULL THEN 'Closed' ELSE 'Open' END AS PostStatus
// FROM RankedPosts rp LEFT JOIN UserReputation ur ON rp.PostId = ur.UserId LEFT JOIN ClosedPosts cp ON rp.PostId = cp.PostId
// WHERE rp.CommentCount > 5 AND ur.Reputation > 100 AND (rp.PostRank = 1 OR rp.Score > 10) ORDER BY rp.CreationDate DESC LIMIT 50;
//
// `rp.PostId = ur.UserId` joins a post id to a user id, so it goes through the raw ids; UserReputation is computed for the users that join. The answer is empty
// on this data.
fn q4203(db: &'static So) -> String {
    let Post { owner_user_id, creation_date, score, origid, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let w = recent().group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d): (Id<Post>, i64)| (Reverse(d), p), asc);
    let first: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let cc = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let uidx: HashIdx<i64, Id<User>> = db.user.with((&db.user.reputation).gt(100)).select(&db.user.origid).inv().collect();
    let joined: MatSet<Id<User>> = recent().select(origid.select(&uidx)).collect();
    let ur = (&joined).group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt().and(posts_of(db).opt())).fold(0i64, |n, (b, _)| n + b.unwrap_or(0));
    let keep = Ident::<Post>::new().with(&first).opt().and(score).filt(|(f, s): (Option<Id<Post>>, i64)| f.is_some() || s > 10);
    let closed = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10)));
    let cand: MatSet<Id<Post>> = recent().with(keep).collect();
    let v = drain((&cand).select((&cc).filt(|n| n > 5)).and(origid.select(&uidx).select(Ident::<User>::new().and(&ur))).and(closed.opt()));
    let v = top_n(v, |&(p, (_, h))| (Reverse(creation_date.get(p).unwrap()), p, h), 50);
    rows(v.into_iter().map(|(p, ((c, (u, b)), h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend([V::I(c), user_col(db, u, "rep"), V::I(b)]);
        let by = h.and_then(|h| db.post_history.user_display_name.get(h));
        f.extend([ostr(by), h.map_or(V::Null, |h| V::T(db.post_history.creation_date.get(h).unwrap())), V::S(if by.is_some() { "Closed" } else { "Open" })]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
//        COALESCE(COUNT(DISTINCT p.Id), 0) AS PostCount, COALESCE(SUM(CASE WHEN c.Id IS NOT NULL THEN 1 ELSE 0 END), 0) AS CommentCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, UpVotes, DownVotes, CommentCount, RANK() OVER (ORDER BY PostCount DESC, UpVotes DESC) AS Rank FROM UserActivity),
// PostStats AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, COUNT(c.Id) AS CommentCount, AVG(COALESCE(p2.Score, 0)) AS AvgScore
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Posts p2 ON p.AcceptedAnswerId = p2.Id WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.Score)
// SELECT tu.DisplayName, tu.PostCount, tu.UpVotes, tu.DownVotes, tu.CommentCount, ps.Title, ps.CreationDate, ps.Score AS PostScore, ps.CommentCount AS TotalComments, ps.AvgScore AS AcceptedAnswerAvgScore
// FROM TopUsers tu JOIN PostStats ps ON tu.UserId = ps.PostId WHERE tu.Rank <= 10 ORDER BY tu.Rank, ps.Score DESC;
//
// `tu.UserId = ps.PostId` joins a user id to a post id, so it goes through the raw ids.
fn q525(db: &'static So) -> String {
    let Post { post_type_id, accepted_answer, score, origid, .. } = &db.post;
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt())).opt())
        .fold([0i64; 3], |a, p| match p {
            Some((t, c)) => [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64],
            None => a,
        });
    let w = whole(&s).select(Ident::<User>::new().and((&s).and(&pc))).window(rank, |(_, (a, n)): (Id<User>, ([i64; 3], i64))| (Reverse(n), Reverse(a[0])), asc);
    let tu: MatSet<(Id<User>, (([i64; 3], i64), i64))> = (&w).filt(|(_, r)| r <= 10).map(|((u, x), r)| (u, (x, r))).collect();
    let pidx: HashIdx<i64, Id<Post>> = db.post.with(post_type_id.eq(1)).select(origid).inv().collect();
    let ps = db
        .post
        .with(post_type_id.eq(1))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(accepted_answer.select(score).opt()))
        .fold([0i64; 3], |a, (c, s)| [a[0] + c.is_some() as i64, a[1] + s.unwrap_or(0), a[2] + 1]);
    let v = drain(by_first(&tu).and((&db.user.origid).select(&pidx).select(Ident::<Post>::new().and(&ps))));
    let v = top_n(v, |&(_, ((_, r), (p, _)))| (r, Reverse(score.get(p).unwrap()), p), 0);
    rows(v.into_iter().map(|(u, (((a, n), _), (p, b)))| {
        let mut f = vec![user_col(db, u, "name"), V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2])];
        f.extend(post_fields(db, p, &["title", "created", "score"]));
        f.extend([V::I(b[0]), avg(b[1], b[2])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn,
//        COALESCE(c.CommentCount, 0) AS CommentCount, COALESCE(b.BadgeCount, 0) AS BadgeCount
//     FROM Posts p LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
//     LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId) b ON p.OwnerUserId = b.UserId WHERE p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.AnswerCount, rp.CommentCount, rp.BadgeCount FROM RankedPosts rp
//     WHERE rp.rn = 1 AND rp.Score > 10 AND (rp.CommentCount IS NOT NULL OR rp.BadgeCount > 0))
// SELECT fp.PostId, fp.Title, fp.CreationDate, fp.Score, fp.AnswerCount, fp.CommentCount, fp.BadgeCount,
//        CASE WHEN fp.CommentCount > 5 THEN 'Highly Discussed' WHEN fp.BadgeCount > 0 THEN 'Recognized Contributor' ELSE 'Regular Post' END AS PostCategory
// FROM FilteredPosts fp ORDER BY fp.Score DESC LIMIT 50;
//
// CommentCount is a COALESCE, so it is never NULL and the OR always holds.
fn q4858(db: &'static So) -> String {
    let Post { owner_user_id, creation_date, score, .. } = &db.post;
    let w = db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d): (Id<Post>, i64)| (Reverse(d), p), asc);
    let rp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let by_uid: HashIdx<i64, Id<Badge>> = (&db.badge.user_id).inv().collect();
    let fp = || (&rp).with(score.gt(10)).group_by(Ident::<Post>::new());
    let cc = fp().select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let bc = fp().select(owner_user_id.select(&by_uid).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = top_n(drain((&cc).and(&bc)), |&(p, _)| (Reverse(score.get(p).unwrap()), p), 50);
    rows(v.into_iter().map(|(p, (c, b))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "answers"]);
        f.extend([V::I(c), V::I(b), V::S(if c > 5 { "Highly Discussed" } else if b > 0 { "Recognized Contributor" } else { "Regular Post" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.ViewCount DESC) AS ViewRank,
//        COUNT(DISTINCT c.Id) AS CommentCount, p.OwnerUserId FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '365 days' AND p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.OwnerUserId),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews, COUNT(DISTINCT p.Id) AS TotalPosts, COALESCE(SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges,
//        COALESCE(SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadges, COALESCE(SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName)
// SELECT us.DisplayName, us.TotalPosts, us.TotalViews, us.GoldBadges, us.SilverBadges, us.BronzeBadges, rp.Title, rp.CreationDate, rp.ViewCount, rp.CommentCount
// FROM UserStats us JOIN RankedPosts rp ON us.UserId = rp.OwnerUserId WHERE rp.ViewRank <= 3 ORDER BY us.TotalViews DESC, rp.ViewCount DESC;
//
// The ViewRank ties are broken by post id.
fn q5416(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, view_count, .. } = &db.post;
    let w = db
        .post
        .with(post_type_id.eq(1))
        .with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -365)))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(view_count.opt()))
        .window(row_number, |(p, w): (Id<Post>, Option<i64>)| (w.is_none(), Reverse(w), p), asc);
    let rp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 3).map(|((p, _), _)| p).collect();
    let owners: MatSet<Id<User>> = (&rp).select(owner_user).collect();
    let us = (&owners)
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(view_count.opt()).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 4], |a, (w, b)| [a[0] + w.flatten().unwrap_or(0), a[1] + (b == Some(1)) as i64, a[2] + (b == Some(2)) as i64, a[3] + (b == Some(3)) as i64]);
    let pc = (&owners).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let cc = (&rp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&cc).and(owner_user.select(Ident::<User>::new().and(&us).and(&pc))));
    rows(v.into_iter().map(|(p, (c, ((u, a), n)))| {
        let mut f = vec![user_col(db, u, "name"), V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3])];
        f.extend(post_fields(db, p, &["title", "created", "views"]));
        f.push(V::I(c));
        row(f)
    }))
}

// WITH UserSummary AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COALESCE(SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END), 0) AS QuestionCount,
//        COALESCE(SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS AnswerCount, COALESCE(SUM(V.BountyAmount), 0) AS TotalBounties, ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS UserRank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, QuestionCount, AnswerCount, TotalBounties, UserRank FROM UserSummary WHERE UserRank <= 10),
// PostStats AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, COALESCE(COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END), 0) AS CommentCount, COUNT(V.Id) AS VoteCount, MAX(PH.CreationDate) AS LastHistoryDate
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN PostHistory PH ON P.Id = PH.PostId GROUP BY P.Id, P.Title, P.CreationDate)
// SELECT U.DisplayName, U.Reputation, T.QuestionCount, T.AnswerCount, T.TotalBounties, P.Title, P.CommentCount, P.VoteCount, P.LastHistoryDate
// FROM TopUsers T JOIN PostStats P ON P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' LEFT JOIN UserSummary U ON T.UserId = U.UserId
// ORDER BY T.Reputation DESC, P.VoteCount DESC;
//
// UserRank reads only Reputation, so the ten users are picked first (ties broken by user id) and the posts x votes product is driven for them alone. The ON
// clause names only P, so the top users are crossed with the recent posts. `LEFT JOIN UserSummary U ON T.UserId = U.UserId` is the user's own row.
fn q4546(db: &'static So) -> String {
    let tu = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10);
    let tus: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let s = (&tus)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(&db.post.post_type_id).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold([0i64; 3], |a, (t, b)| [a[0] + (t == Some(1)) as i64, a[1] + (t == Some(2)) as i64, a[2] + b.flatten().unwrap_or(0)]);
    let ps = db
        .post
        .with((&db.post.creation_date).ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).opt()).and(history_of(db).select(&db.post_history.creation_date).opt()))
        .fold([0i64, 0, i64::MIN], |a, ((c, v), d)| [a[0] + c.is_some() as i64, a[1] + v.is_some() as i64, a[2].max(d.unwrap_or(i64::MIN))]);
    let mut v = Vec::new();
    (&s).cross(&ps).drive(|(u, p), (a, b)| v.push((u, a, p, b)));
    rows(v.into_iter().map(|(u, a, p, b)| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(a.map(V::I));
        f.extend(post_fields(db, p, &["title"]));
        f.extend([V::I(b[0]), V::I(b[1]), tmax(b[2])]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// MostActiveUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, AnswerCount, QuestionCount, AcceptedAnswers, RANK() OVER (ORDER BY PostCount DESC) AS RankByPosts FROM UserStats WHERE PostCount > 0),
// BadgeSummary AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// CombinedStats AS (SELECT mau.UserId, mau.DisplayName, mau.Reputation, mau.PostCount, mau.AnswerCount, mau.QuestionCount, mau.AcceptedAnswers, COALESCE(bs.BadgeCount, 0) AS BadgeCount, mau.RankByPosts
//     FROM MostActiveUsers mau LEFT JOIN BadgeSummary bs ON mau.UserId = bs.UserId)
// SELECT UserId, DisplayName, Reputation, PostCount, AnswerCount, QuestionCount, AcceptedAnswers, BadgeCount, RankByPosts FROM CombinedStats WHERE RankByPosts <= 10 ORDER BY Reputation DESC, PostCount DESC;
fn q7208(db: &'static So) -> String {
    let Post { post_type_id, accepted_answer_id, .. } = &db.post;
    let us = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(accepted_answer_id.opt())).opt()).fold([0i64; 4], |a, p| match p {
        Some((t, x)) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 1) as i64, a[3] + x.is_some() as i64],
        None => a,
    });
    let act = (&us).filt(|a| a[0] > 0);
    let w = whole(&act).select(Ident::<User>::new().and(&act)).window(rank, |(_, a): (Id<User>, [i64; 4])| Reverse(a[0]), asc);
    let tr: MatSet<(Id<User>, ([i64; 4], i64))> = (&w).filt(|(_, r)| r <= 10).map(|((u, a), r)| (u, (a, r))).collect();
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = drain(by_first(&tr).and(&bc));
    rows(v.into_iter().map(|(u, ((a, r), b))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(a.map(V::I));
        f.extend([V::I(b), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostID, p.Title, p.ViewCount, p.Score, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.Score IS NOT NULL),
// TopPosts AS (SELECT rp.PostID, rp.Title, rp.ViewCount, rp.Score, COALESCE(c.CommentCount, 0) AS TotalComments, U.DisplayName AS Author
//     FROM RankedPosts rp LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON rp.PostID = c.PostId
//     INNER JOIN Users U ON U.Id = (SELECT OwnerUserId FROM Posts WHERE Id = rp.PostID) WHERE rp.Rank <= 5),
// PostEngagement AS (SELECT tp.PostID, tp.Title, tp.ViewCount, tp.Score, tp.TotalComments, tp.Author, (tp.ViewCount + tp.TotalComments) AS EngagementScore FROM TopPosts tp)
// SELECT pe.PostID, pe.Title, pe.ViewCount, pe.Score, pe.TotalComments, pe.Author, pe.EngagementScore,
//        CASE WHEN pe.EngagementScore > 100 THEN 'High Engagement' WHEN pe.EngagementScore BETWEEN 50 AND 100 THEN 'Moderate Engagement' ELSE 'Low Engagement' END AS EngagementLevel
// FROM PostEngagement pe WHERE pe.ViewCount > (SELECT AVG(ViewCount) FROM Posts) ORDER BY pe.EngagementScore DESC;
//
// The correlated subquery looks a post up by its primary key, so it is the post's own OwnerUserId. The Rank ties are broken by post id.
fn q1255(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, owner_user, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(post_type_id).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s): (Id<Post>, i64)| (Reverse(s), p), asc);
    let rp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let (s, n) = db.post.select(view_count).fold_flat((0i64, 0i64), |(s, n), w| (s + w, n + 1));
    let above = view_count.filt(move |w: i64| (w as i128) * (n as i128) > s as i128);
    let cc = (&rp).with(owner_user).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |k, c| k + c.is_some() as i64);
    let v = drain((&cc).and(above));
    rows(v.into_iter().map(|(p, (c, w))| {
        let e = w + c;
        let mut f = post_fields(db, p, &["id", "title", "views", "score"]);
        f.extend([V::I(c)]);
        f.extend(post_fields(db, p, &["owner"]));
        f.extend([V::I(e), V::S(if e > 100 { "High Engagement" } else if e >= 50 { "Moderate Engagement" } else { "Low Engagement" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 AND p.CreationDate > (TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '3 months') GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.OwnerUserId),
// RecentPosts AS (SELECT PostId, Title, CreationDate, Score, CommentCount, UpVoteCount FROM RankedPosts WHERE rn <= 5),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, ROW_NUMBER() OVER (ORDER BY u.Reputation DESC) AS ReputationRank FROM Users u)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.CommentCount, rp.UpVoteCount, CASE WHEN ur.Reputation IS NULL THEN 'No Reputation Info' ELSE CAST(ur.Reputation AS VARCHAR) END AS UserReputation,
//        COALESCE(t.TagName, 'Unlabeled') AS TagName
// FROM RecentPosts rp LEFT JOIN Users u ON rp.PostId = u.Id LEFT JOIN Tags t ON t.ExcerptPostId = rp.PostId LEFT JOIN UserReputation ur ON u.Id = ur.UserId ORDER BY rp.Score DESC, rp.CreationDate DESC;
//
// rn reads only base columns, so each owner's five newest questions are picked first and the comments x votes product is driven for those alone. `rp.PostId = u.Id`
// joins a post id to a user id, so it goes through the raw ids; ReputationRank is never read.
fn q2698(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user_id, origid, .. } = &db.post;
    let w = db
        .post
        .with(post_type_id.eq(1))
        .with(creation_date.gt(add_months(ts(2024, 10, 1, 12, 34, 56), -3)))
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d): (Id<Post>, i64)| (Reverse(d), p), asc);
    let rp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let s = (&rp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64]);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let excerpt: HashIdx<Id<Post>, Id<Tag>> = (&db.tag.excerpt_post).inv().collect();
    let v = drain((&s).and(origid.select(&uidx).select(&db.user.reputation).opt()).and((&excerpt).select(&db.tag.tag_name).opt()));
    rows(v.into_iter().map(|(p, ((a, r), t))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.push(match r {
            Some(r) => V::Owned(r.to_string()),
            None => V::S("No Reputation Info"),
        });
        f.push(V::S(t.unwrap_or("Unlabeled")));
        row(f)
    }))
}

// WITH RecursiveTopUsers AS (SELECT U.Id, U.DisplayName, U.Reputation, RANK() OVER (ORDER BY U.Reputation DESC) AS UserRank FROM Users U WHERE U.Reputation > 1000),
// TopPostTypes AS (SELECT P.PostTypeId, COUNT(*) AS PostCount FROM Posts P GROUP BY P.PostTypeId HAVING COUNT(*) > 100),
// RecentPostHistory AS (SELECT PH.PostId, PH.PostHistoryTypeId, PH.CreationDate, PH.UserId, U.DisplayName AS EditorName FROM PostHistory PH JOIN Users U ON PH.UserId = U.Id
//     WHERE PH.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days')
// SELECT U.DisplayName AS UserName, U.Reputation AS UserReputation, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN PH.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS TotalClosedPosts,
//        SUM(CASE WHEN PH.PostHistoryTypeId = 12 THEN 1 ELSE 0 END) AS TotalDeletedPosts, COUNT(DISTINCT PH.PostId) AS UniquePostHistoryEntries, AVG(P.Score) AS AveragePostScore
// FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN RecentPostHistory PH ON P.Id = PH.PostId
// WHERE U.Reputation IS NOT NULL AND U.Id IN (SELECT Id FROM RecursiveTopUsers WHERE UserRank <= 10) AND EXISTS (SELECT 1 FROM TopPostTypes TPT WHERE P.PostTypeId = TPT.PostTypeId)
// GROUP BY U.Id, U.DisplayName, U.Reputation ORDER BY TotalPosts DESC, UserReputation DESC LIMIT 20;
//
// Not recursive: no CTE refers to itself. The EXISTS reads P.PostTypeId, so it drops the users' NULL post rows and keeps posts of the busy types only.
fn q32777(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let rich = db.user.with((&db.user.reputation).gt(1000));
    let w = whole(rich).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r): (Id<User>, i64)| Reverse(r), asc);
    let tu: MatSet<Id<User>> = (&w).filt(|(_, r)| r <= 10).map(|((u, _), _)| u).collect();
    let tpt = db.post.group_by(post_type_id).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let busy = || Ident::<Post>::new().with(post_type_id.select((&tpt).filt(|n| n > 100)));
    let PostHistory { creation_date: hd, user, post_history_type_id, .. } = &db.post_history;
    let rph = history_of(db).select(Ident::<PostHistory>::new().with(hd.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).with(user)).select(post_history_type_id);
    let g = || (&tu).group_by(Ident::<User>::new());
    let s = g().select(posts_of(db).select(busy()).select(score.and(rph.opt()))).fold([0i64; 4], |a, (s, t)| [a[0] + (t == Some(10)) as i64, a[1] + (t == Some(12)) as i64, a[2] + s, a[3] + 1]);
    let pc = g().select(posts_of(db).select(busy())).fold(0i64, |n, _| n + 1);
    let hp = g().select(posts_of(db).select(busy()).with(history_of(db).select(Ident::<PostHistory>::new().with(hd.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).with(user)))).fold(0i64, |n, _| n + 1);
    let v = drain((&s).and(&pc).and((&hp).opt()));
    let v = top_n(v, |&(u, ((_, n), _))| (Reverse(n), Reverse(db.user.reputation.get(u).unwrap()), u), 20);
    rows(v.into_iter().map(|(u, ((a, n), h))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(h.unwrap_or(0)), avg(a[2], a[3])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.LastActivityDate, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.ViewCount DESC) AS RankByViews,
//        COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.ViewCount > 100),
// RecentUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.CreationDate, DENSE_RANK() OVER (ORDER BY u.CreationDate DESC) AS RecentRank FROM Users u WHERE u.Reputation IS NOT NULL),
// VoteSummary AS (SELECT PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes v GROUP BY PostId)
// SELECT rp.PostId, rp.Title, rp.ViewCount, rp.LastActivityDate, rp.RankByViews, ru.DisplayName AS LatestUser, ru.Reputation, COALESCE(vs.UpVotes, 0) AS UpVotes, COALESCE(vs.DownVotes, 0) AS DownVotes,
//        CASE WHEN rp.CommentCount > 0 THEN 'Has Comments' ELSE 'No Comments' END AS CommentStatus,
//        CASE WHEN rp.CreationDate < (cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year') THEN 'Old' ELSE 'New' END AS PostAgeStatus
// FROM RankedPosts rp LEFT JOIN RecentUsers ru ON ru.RecentRank = 1 LEFT JOIN VoteSummary vs ON rp.PostId = vs.PostId WHERE rp.RankByViews <= 5 ORDER BY rp.ViewCount DESC LIMIT 10;
//
// The window numbers the post x comment rows, so those rows are materialised and ranked (ties broken by post and comment id). The ON clause names only ru, so
// the ranked rows are crossed with the newest users.
fn q22813(db: &'static So) -> String {
    let Post { post_type_id, view_count, creation_date, .. } = &db.post;
    let w = db
        .post
        .with(view_count.gt(100))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(view_count).and(comments_of(db).opt()))
        .window(row_number, |((p, w), c): ((Id<Post>, i64), Option<Id<Comment>>)| (Reverse(w), p, c), asc);
    let top: MatSet<(Id<Post>, i64)> = (&w).filt(|(_, r)| r <= 5).map(|(((p, _), _), r)| (p, r)).collect();
    let cc = db.post.with(view_count.gt(100)).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let uw = whole(&db.user.reputation).select(Ident::<User>::new().and(&db.user.creation_date)).window(dense_rank, |(_, d): (Id<User>, i64)| Reverse(d), asc);
    let ru: HashIdx<(), Id<User>> = (&uw).filt(|(_, r)| r == 1).map(|((u, _), _)| u).collect();
    let vs = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let v = drain(by_first(&top).and(&cc).and((&vs).opt()).and(Ident::<Post>::new().map(|_| ()).select(&ru).opt()));
    let v = top_n(v, |&(p, (((r, _), _), u))| (Reverse(view_count.get(p)), p, r, u), 10);
    let old = add_years(ts(2024, 10, 1, 12, 34, 56), -1);
    rows(v.into_iter().map(|(p, (((r, c), a), u))| {
        let a = a.unwrap_or([0, 0]);
        let mut f = post_fields(db, p, &["id", "title", "views", "activity"]);
        f.push(V::I(r));
        f.extend(match u {
            Some(u) => ucols(db, u, &["name", "rep"]),
            None => vec![V::Null, V::Null],
        });
        f.extend([V::I(a[0]), V::I(a[1]), V::S(if c > 0 { "Has Comments" } else { "No Comments" }), V::S(if creation_date.get(p).unwrap() < old { "Old" } else { "New" })]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS TotalPosts, COALESCE(SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END), 0) AS QuestionsCount,
//        COALESCE(SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS AnswersCount, COALESCE(SUM(CASE WHEN C.Id IS NOT NULL THEN 1 ELSE 0 END), 0) AS CommentsCount,
//        COUNT(DISTINCT B.Id) AS BadgesCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        DENSE_RANK() OVER (ORDER BY COALESCE(SUM(CASE WHEN P.PostTypeId = 1 THEN P.Score ELSE 0 END), 0) DESC) AS RankByQuestionScore
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Badges B ON U.Id = B.UserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName),
// RankedUsers AS (SELECT UserId, DisplayName, TotalPosts, QuestionsCount, AnswersCount, CommentsCount, BadgesCount, UpVotes, DownVotes, RankByQuestionScore,
//        ROW_NUMBER() OVER (ORDER BY TotalPosts DESC) AS OverallRank FROM UserActivity)
// SELECT RU.DisplayName, RU.TotalPosts, RU.QuestionsCount, RU.AnswersCount, RU.CommentsCount, RU.BadgesCount, RU.UpVotes, RU.DownVotes, RU.RankByQuestionScore, RU.OverallRank
// FROM RankedUsers RU WHERE RU.OverallRank <= 10 ORDER BY RU.RankByQuestionScore DESC;
//
// RankByQuestionScore ranks every user, so the whole posts x comments x badges x votes product is driven. OverallRank ties are broken by user id.
fn q9774(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(comments_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 7], |a, (p, _)| match p {
            Some((((t, s), c), v)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + c.is_some() as i64, a[4] + (v == Some(2)) as i64, a[5] + (v == Some(3)) as i64, a[6] + if t == 1 { s } else { 0 }],
            None => a,
        });
    let w = whole(&s).select(Ident::<User>::new().and(&s)).window(dense_rank, |(_, a): (Id<User>, [i64; 7])| Reverse(a[6]), asc);
    let w = (&w).window(row_number, |((u, a), _): ((Id<User>, [i64; 7]), i64)| (Reverse(a[0]), u), asc);
    let tr: MatSet<(Id<User>, ([i64; 7], i64, i64))> = (&w).filt(|(_, o)| o <= 10).map(|(((u, a), q), o)| (u, (a, q, o))).collect();
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = drain(by_first(&tr).and(&bc));
    rows(v.into_iter().map(|(u, ((a, q, o), b))| row(vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(b), V::I(a[4]), V::I(a[5]), V::I(q), V::I(o)])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.ViewCount DESC) AS UserPostRank
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounties
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 8 GROUP BY u.Id, u.DisplayName),
// UserBadges AS (SELECT b.UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverBadges, COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeBadges
//     FROM Badges b GROUP BY b.UserId)
// SELECT us.DisplayName, us.TotalPosts, us.TotalBounties, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges, rp.PostId, rp.Title, rp.ViewCount,
//        CASE WHEN rp.UserPostRank <= 3 THEN 'Top Post' ELSE 'Regular Post' END AS PostRanking
// FROM UserStats us LEFT JOIN UserBadges ub ON us.UserId = ub.UserId LEFT JOIN RankedPosts rp ON us.UserId = rp.PostId WHERE us.TotalPosts > 0
// ORDER BY us.TotalPosts DESC, us.TotalBounties DESC, rp.ViewCount DESC LIMIT 100;
//
// `us.UserId = rp.PostId` joins a user id to a post id, so it goes through the raw ids. UserPostRank ties are broken by post id.
fn q1697(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user_id, view_count, origid, .. } = &db.post;
    let w = db
        .post
        .with(post_type_id.eq(1))
        .with(creation_date.ge(add_years(date(2024, 10, 1), -1)))
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(view_count.opt()))
        .window(row_number, |(p, w): (Id<Post>, Option<i64>)| (w.is_none(), Reverse(w), p), asc);
    let rk: MatSet<(Id<Post>, i64)> = (&w).map(|((p, _), r)| (p, r)).collect();
    let by_id: HashIdx<i64, (Id<Post>, i64)> = (&rk).map(|(p, _)| p).select(origid).inv().collect();
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let us = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(bounty.opt())).fold(0i64, |n, b| n + b.flatten().unwrap_or(0));
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let v = drain((&pc).and(&us).and((&ub).opt()).and((&db.user.origid).select(&by_id).opt()));
    let v = top_n(v, |&(u, (((n, b), _), p))| (Reverse(n), Reverse(b), p.map_or(true, |(p, _)| view_count.get(p).is_none()), Reverse(p.map(|(p, _)| view_count.get(p))), u), 100);
    rows(v.into_iter().map(|(u, (((n, b), g), p))| {
        let mut f = vec![user_col(db, u, "name"), V::I(n), V::I(b)];
        f.extend(match g {
            Some(g) => g.into_iter().map(V::I).collect(),
            None => vec![V::Null, V::Null, V::Null],
        });
        f.extend(match p {
            Some((p, _)) => post_fields(db, p, &["id", "title", "views"]),
            None => vec![V::Null, V::Null, V::Null],
        });
        f.push(V::S(if p.map_or(false, |(_, r)| r <= 3) { "Top Post" } else { "Regular Post" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AcceptedAnswerId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS Rank,
//        p.OwnerUserId FROM Posts p),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, MAX(b.Class) AS HighestBadgeClass FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// AnswerInfo AS (SELECT a.OwnerUserId, COUNT(a.Id) AS TotalAnswers, SUM(CASE WHEN a.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers FROM Posts a WHERE a.PostTypeId = 2
//     GROUP BY a.OwnerUserId)
// SELECT u.DisplayName, u.Reputation, ub.BadgeCount, ub.HighestBadgeClass, COALESCE(ai.TotalAnswers, 0) AS TotalAnswers, COALESCE(ai.AcceptedAnswers, 0) AS AcceptedAnswers,
//        COUNT(DISTINCT rp.PostId) AS TotalPosts, AVG(rp.Score) AS AveragePostScore, MAX(rp.CreationDate) AS MostRecentPostDate
// FROM Users u LEFT JOIN UserBadges ub ON u.Id = ub.UserId LEFT JOIN AnswerInfo ai ON u.Id = ai.OwnerUserId LEFT JOIN RankedPosts rp ON u.Id = rp.OwnerUserId
// WHERE u.Reputation > 1000 AND (ub.BadgeCount > 0 OR ai.TotalAnswers > 0)
// GROUP BY u.DisplayName, u.Reputation, ub.BadgeCount, ub.HighestBadgeClass, ai.TotalAnswers, ai.AcceptedAnswers ORDER BY u.Reputation DESC, TotalPosts DESC;
//
// The GROUP BY names no user id, so users agreeing on every key column form one group. Rank is never read.
fn q33143(db: &'static So) -> String {
    let Post { post_type_id, accepted_answer_id, owner_user, score, creation_date, .. } = &db.post;
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold((0i64, None::<i64>), |(n, m), c| match c {
        Some(c) => (n + 1, Some(m.map_or(c, |m: i64| m.max(c)))),
        None => (n, m),
    });
    let ai = db.post.with(post_type_id.eq(2)).group_by(owner_user).select(accepted_answer_id.opt()).fold([0i64; 2], |a, x| [a[0] + 1, a[1] + x.is_some() as i64]);
    let keep = (&ub).and((&ai).opt()).filt(|((n, _), a): ((i64, Option<i64>), Option<[i64; 2]>)| n > 0 || a.map_or(false, |a| a[0] > 0));
    let key = (&db.user.display_name).and(&db.user.reputation).and(&keep);
    let g = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(key)
        .select(posts_of(db).select(score.and(creation_date)).opt())
        .fold((0i64, 0i64, i64::MIN), |(n, s, m), p| match p {
            Some((x, d)) => (n + 1, s + x, m.max(d)),
            None => (n, s, m),
        });
    let v = drain(&g);
    rows(v.into_iter().map(|(((name, rep), ((b, c), a)), (n, s, m))| {
        let a = a.unwrap_or([0, 0]);
        row(vec![V::S(name), V::I(rep), V::I(b), oint(c), V::I(a[0]), V::I(a[1]), V::I(n), avg(s, n), tmax(m)])
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END) AS PositiveScorePosts,
//        SUM(CASE WHEN P.ClosedDate IS NOT NULL THEN 1 ELSE 0 END) AS ClosedPosts, SUM(COALESCE(CommentCounts.CommentCount, 0)) AS TotalComments
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN (SELECT C.PostId, COUNT(C.Id) AS CommentCount FROM Comments C GROUP BY C.PostId) AS CommentCounts ON P.Id = CommentCounts.PostId
//     WHERE U.Reputation > 1000 GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, TotalQuestions, TotalAnswers, PositiveScorePosts, ClosedPosts, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserStats)
// SELECT TU.Rank, TU.DisplayName, TU.Reputation, TU.TotalPosts, TU.TotalQuestions, TU.TotalAnswers, TU.PositiveScorePosts, TU.ClosedPosts, B.Name AS BadgeName, B.Class AS BadgeClass
// FROM TopUsers TU LEFT JOIN Badges B ON TU.UserId = B.UserId WHERE TU.Rank <= 10 ORDER BY TU.Rank, B.Class DESC;
//
// Rank reads only Reputation, so the ten users are picked first (ties broken by user id). TotalComments is never read.
fn q5911(db: &'static So) -> String {
    let Post { post_type_id, score, closed_date, .. } = &db.post;
    let rich = db.user.with((&db.user.reputation).gt(1000));
    let w = whole(rich).select(Ident::<User>::new().and(&db.user.reputation)).window(row_number, |(u, r): (Id<User>, i64)| (Reverse(r), u), asc);
    let tr: MatSet<(Id<User>, i64)> = (&w).filt(|(_, r)| r <= 10).map(|((u, _), r)| (u, r)).collect();
    let tus: MatSet<Id<User>> = (&tr).map(|(u, _)| u).collect();
    let us = (&tus).group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(score).and(closed_date.opt())).opt()).fold([0i64; 5], |a, p| match p {
        Some(((t, s), c)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (s > 0) as i64, a[4] + c.is_some() as i64],
        None => a,
    });
    let v = drain(by_first(&tr).and(&us).and(badges_of(db).select((&db.badge.name).and(&db.badge.class)).opt()));
    rows(v.into_iter().map(|(u, ((r, a), b))| {
        let mut f = vec![V::I(r)];
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend(a.map(V::I));
        f.extend(match b {
            Some((n, c)) => [V::S(n), V::I(c)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.OwnerUserId, u.Reputation, u.DisplayName, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS RankScore
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.Score > 0),
// UserBadges AS (SELECT b.UserId, COUNT(*) FILTER (WHERE b.Class = 1) AS GoldBadges, COUNT(*) FILTER (WHERE b.Class = 2) AS SilverBadges, COUNT(*) FILTER (WHERE b.Class = 3) AS BronzeBadges
//     FROM Badges b GROUP BY b.UserId),
// PostInteraction AS (SELECT ct.PostId, COUNT(*) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount FROM Comments ct LEFT JOIN Votes v ON ct.PostId = v.PostId GROUP BY ct.PostId)
// SELECT rp.PostId, rp.Title, rp.Score, rp.DisplayName, rp.Reputation, COALESCE(ub.GoldBadges, 0) AS GoldBadges, COALESCE(ub.SilverBadges, 0) AS SilverBadges, COALESCE(ub.BronzeBadges, 0) AS BronzeBadges,
//        pi.CommentCount, pi.UpvoteCount
// FROM RankedPosts rp LEFT JOIN UserBadges ub ON rp.OwnerUserId = ub.UserId LEFT JOIN PostInteraction pi ON rp.PostId = pi.PostId
// WHERE rp.RankScore = 1 AND (rp.Reputation > 1000 OR rp.DisplayName IS NOT NULL) ORDER BY rp.Score DESC, ub.GoldBadges DESC;
//
// DisplayName is never NULL, so the OR always holds.
fn q1783(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(score.gt(0))
        .with(owner_user)
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(score))
        .window(rank, |(_, s): (Id<Post>, i64)| Reverse(s), asc);
    let rp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let pi = (&rp).group_by(Ident::<Post>::new()).select(comments_of(db).and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 2], |a, (_, t)| [a[0] + 1, a[1] + (t == Some(2)) as i64]);
    let v = drain((&rp).select(owner_user.select((&ub).opt()).and((&pi).opt())));
    rows(v.into_iter().map(|(p, (b, i))| {
        let b = b.unwrap_or([0; 3]);
        let mut f = post_fields(db, p, &["id", "title", "score", "owner", "rep"]);
        f.extend(b.map(V::I));
        f.extend(match i {
            Some(a) => [V::I(a[0]), V::I(a[1])],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserVoteStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT v.PostId) AS TotalVotes, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// PostAnalytics AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, COALESCE(ph.Comment, 'No history') AS PostHistory, (SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RankByOwner FROM Posts p LEFT JOIN PostHistory ph ON p.Id = ph.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND (p.Score > 10 OR (SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id) > 5)),
// TopUsers AS (SELECT uvs.DisplayName, uvs.Reputation, RANK() OVER (ORDER BY uvs.TotalVotes DESC) AS VoteRank FROM UserVoteStats uvs WHERE uvs.TotalVotes > 5)
// SELECT pa.Title, pa.CreationDate, pa.Score, pa.CommentCount, tu.DisplayName AS TopVoter, tu.Reputation AS VoterReputation
// FROM PostAnalytics pa JOIN TopUsers tu ON tu.VoteRank <= 5 WHERE pa.RankByOwner = 1 ORDER BY pa.Score DESC, pa.CommentCount DESC LIMIT 10;
//
// RankByOwner numbers the post x history rows; the rows of one post share every projected column, so the number is taken over the posts (ties broken by post id).
// The ON clause names only tu, so the first posts are crossed with the top voters.
fn q1956(db: &'static So) -> String {
    let Post { owner_user_id, creation_date, score, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let cc = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let hot = score.and(&cc).filt(|(s, c): (i64, i64)| s > 10 || c > 5);
    let w = recent().with(hot).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d): (Id<Post>, i64)| (Reverse(d), p), asc);
    let first: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let tv = db.vote.group_by(&db.vote.user).select(&db.vote.post_id).count_distinct();
    let act = (&tv).filt(|n| n > 5);
    let uw = whole(&act).select(Ident::<User>::new().and(&act)).window(rank, |(_, n): (Id<User>, i64)| Reverse(n), asc);
    let tu = (&uw).filt(|(_, r)| r <= 5).map(|((u, _), _)| u);
    let v = drain(whole(&first).select(Ident::<Post>::new().and(&cc)).and(tu));
    let v = top_n(v, |&(_, ((p, c), u))| (Reverse(score.get(p).unwrap()), Reverse(c), p, u), 10);
    rows(v.into_iter().map(|(_, ((p, c), u))| {
        let mut f = post_fields(db, p, &["title", "created", "score"]);
        f.push(V::I(c));
        f.extend(ucols(db, u, &["name", "rep"]));
        row(f)
    }))
}

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

// WITH RankedQuestions AS (SELECT p.Id AS QuestionId, p.Title, p.Tags, p.CreationDate, p.Score, u.DisplayName AS OwnerDisplayName, RANK() OVER (ORDER BY p.Score DESC) AS ScoreRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.AcceptedAnswerId IS NOT NULL),
// TopQuestions AS (SELECT Q.QuestionId, Q.Title, Q.Tags, Q.CreationDate, Q.Score, Q.OwnerDisplayName FROM RankedQuestions Q WHERE Q.ScoreRank <= 10),
// QuestionTags AS (SELECT T.TagName, COUNT(Q.QuestionId) AS QuestionCount FROM TopQuestions Q CROSS JOIN UNNEST(string_to_array(Q.Tags, '><')) AS T(TagName) GROUP BY T.TagName),
// MostPopularTags AS (SELECT TagName, QuestionCount, RANK() OVER (ORDER BY QuestionCount DESC) AS PopularityRank FROM QuestionTags)
// SELECT MPT.TagName, MPT.QuestionCount, (SELECT COUNT(*) FROM Posts WHERE Tags LIKE '%' || MPT.TagName || '%') AS TotalQuestionsWithTag,
//        (SELECT COUNT(*) FROM Badges B JOIN Users U ON B.UserId = U.Id WHERE U.Reputation > 1000) AS ActiveUsersWithBadges,
//        (SELECT COUNT(*) FROM Votes V WHERE V.VoteTypeId = 2 AND EXISTS (SELECT 1 FROM Posts P WHERE P.Id = V.PostId AND P.Tags LIKE '%' || MPT.TagName || '%')) AS UpvotesForTag
// FROM MostPopularTags MPT WHERE MPT.PopularityRank <= 5 ORDER BY MPT.PopularityRank;
//
// The split is on the raw Tags text, so the first and last elements keep their '<' and '>'. The LIKE runs over the distinct Tags strings (`select_where`),
// with LIKE's own '_' wildcard, and the posts and upvotes join back through that index.
fn q27675(db: &'static So) -> String {
    let Post { post_type_id, accepted_answer_id, owner_user, score, tags_str, .. } = &db.post;
    let qs = db.post.with(post_type_id.eq(1)).with(accepted_answer_id).with(owner_user);
    let w = whole(qs).select(Ident::<Post>::new().and(score)).window(rank, |(_, s): (Id<Post>, i64)| Reverse(s), asc);
    let tq: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|((p, _), _)| p).collect();
    let qt = (&tq).select(tags_str.flat_map(|t: Str| t.split("><"))).group_by(Same::<Str>::new()).select(Same::<Str>::new()).fold(0i64, |n, _| n + 1);
    let mw = whole(&qt).select(Same::<Str>::new().and(&qt)).window(rank, |(_, n): (Str, i64)| Reverse(n), asc);
    let mp: MatSet<(Str, i64, i64)> = (&mw).filt(|(_, r)| r <= 5).map(|((t, n), r)| (t, n, r)).collect();
    let pat: HashIdx<Str, (Str, i64, i64)> = (&mp).map(|(t, _, _)| t).inv().collect();
    let strs: MatSet<Str> = (&db.post.tags_str).collect();
    let hit: HashIdx<Str, (Str, i64, i64)> = (&strs).select_where(&pat, |s: Str, t: Str| like(s, &format!("%{t}%"))).collect();
    type R = (Str, i64, i64);
    let tp = db.post.select(tags_str.select(&hit)).group_by(Same::<R>::new()).select(Same::<R>::new()).fold(0i64, |n, _| n + 1);
    let tv = db
        .vote
        .with((&db.vote.vote_type_id).eq(2))
        .select((&db.vote.post).select(tags_str).select(&hit))
        .group_by(Same::<R>::new())
        .select(Same::<R>::new())
        .fold(0i64, |n, _| n + 1);
    let active = count(db.badge.select((&db.badge.user).select(Ident::<User>::new().with((&db.user.reputation).gt(1000)))));
    let v = drain((&mp).select(Same::<R>::new().and(Same::<R>::new().select((&tp).opt())).and(Same::<R>::new().select((&tv).opt()))));
    rows(v.into_iter().map(|(_, (((t, n, _), p), u))| row(vec![V::S(t), V::I(n), V::I(p.unwrap_or(0)), V::I(active), V::I(u.unwrap_or(0))])))
}

// Rewritten (rewrites/99.sql): the ROW_NUMBER is tie-broken on p.Id, b.Id (the oracle did not change).
// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC, p.Id, b.Id) AS Rank,
//        COALESCE(b.Class, 0) AS BadgeClass, COALESCE(u.UpVotes, 0) AS UserUpVotes, COALESCE(u.DownVotes, 0) AS UserDownVotes
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Badges b ON u.Id = b.UserId AND b.Date = (SELECT MAX(Date) FROM Badges WHERE UserId = u.Id)
//     WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'),
// TopPosts AS (SELECT rp.*, GREATEST(rp.Score + rp.UserUpVotes - rp.UserDownVotes, 0) AS AdjustedScore FROM RankedPosts rp WHERE rp.Rank = 1)
// SELECT tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.AdjustedScore,
//        CASE WHEN tp.BadgeClass = 1 THEN 'Gold' WHEN tp.BadgeClass = 2 THEN 'Silver' WHEN tp.BadgeClass = 3 THEN 'Bronze' ELSE 'No Badge' END AS BadgeType, COUNT(c.Id) AS CommentCount,
//        (SELECT COUNT(DISTINCT pl.RelatedPostId) FROM PostLinks pl WHERE pl.PostId = tp.Id AND pl.LinkTypeId = 3) AS DuplicateCount
// FROM TopPosts tp LEFT JOIN Comments c ON tp.Id = c.PostId GROUP BY tp.Id, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.AdjustedScore, tp.BadgeClass
// ORDER BY tp.AdjustedScore DESC, tp.CreationDate DESC LIMIT 10;
//
// The correlated MAX is a per-user fold; the window numbers the post x latest-badge rows, so those rows are materialised and numbered.
fn q99(db: &'static So) -> String {
    let Post { owner_user, owner_user_id, creation_date, score, .. } = &db.post;
    let Badge { user, date: bd, .. } = &db.badge;
    let md = db.badge.group_by(user).select(bd).fold(i64::MIN, |m, d| m.max(d));
    let at: HashIdx<(Id<User>, i64), Id<Badge>> = db.badge.select(user.and(bd)).inv().collect();
    let latest = Ident::<User>::new().and(&md).select(&at).select(Ident::<Badge>::new().and(&db.badge.class));
    let w = db
        .post
        .with(creation_date.ge(add_years(date(2024, 10, 1), -1)))
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(creation_date).and(owner_user.select(latest).opt()))
        .window(row_number, |((p, d), b): ((Id<Post>, i64), Option<(Id<Badge>, i64)>)| (Reverse(d), p, b.is_none(), b), asc);
    let tp: MatSet<(Id<Post>, Option<i64>)> = (&w).filt(|(_, r)| r == 1).map(|(((p, _), b), _)| (p, b.map(|b| b.1))).collect();
    let ps: MatSet<Id<Post>> = (&tp).map(|(p, _)| p).collect();
    let cc = (&ps).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let dc = (&ps)
        .group_by(Ident::<Post>::new())
        .select(links_of(db).select(Ident::<PostLink>::new().with((&db.post_link.link_type_id).eq(3))).select(&db.post_link.related_post_id))
        .count_distinct();
    let adj = score.and(owner_user.select((&db.user.up_votes).and(&db.user.down_votes)).opt()).map(|(s, u): (i64, Option<(i64, i64)>)| (s + u.map_or(0, |(a, b)| a - b)).max(0));
    let v = drain(by_first(&tp).and(adj).and(&cc).and((&dc).opt()));
    let v = top_n(v, |&(p, (((_, a), _), _))| (Reverse(a), Reverse(creation_date.get(p).unwrap()), p), 10);
    rows(v.into_iter().map(|(p, (((b, a), c), d))| {
        let mut f = post_fields(db, p, &["title", "created", "score", "views"]);
        f.push(V::I(a));
        f.push(V::S(match b {
            Some(1) => "Gold",
            Some(2) => "Silver",
            Some(3) => "Bronze",
            _ => "No Badge",
        }));
        f.extend([V::I(c), V::I(d.unwrap_or(0))]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U WHERE U.Reputation > 0),
// PostsSummary AS (SELECT P.OwnerUserId, COUNT(*) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersCount,
//        SUM(P.ViewCount) AS TotalViews FROM Posts P GROUP BY P.OwnerUserId),
// VoteCounts AS (SELECT V.UserId, COUNT(*) AS TotalVotes, SUM(CASE WHEN V.VoteTypeId IN (2, 3) THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId IN (3) THEN 1 ELSE 0 END) AS DownVotes
//     FROM Votes V GROUP BY V.UserId)
// SELECT U.DisplayName, COALESCE(UR.Reputation, 0) AS Reputation, COALESCE(PS.TotalPosts, 0) AS PostsCount, COALESCE(PS.QuestionsCount, 0) AS QuestionsCount, COALESCE(PS.AnswersCount, 0) AS AnswersCount,
//        COALESCE(PS.TotalViews, 0) AS TotalViews, COALESCE(VC.TotalVotes, 0) AS TotalVotes, COALESCE(VC.UpVotes, 0) AS UpVotes, COALESCE(VC.DownVotes, 0) AS DownVotes
// FROM Users U LEFT JOIN UserReputation UR ON U.Id = UR.UserId LEFT JOIN PostsSummary PS ON U.Id = PS.OwnerUserId LEFT JOIN VoteCounts VC ON U.Id = VC.UserId
// WHERE U.Location IS NOT NULL AND U.CreationDate < cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' ORDER BY Reputation DESC, TotalPosts DESC FETCH FIRST 10 ROWS ONLY;
fn q2010(db: &'static So) -> String {
    let Post { owner_user, post_type_id, view_count, .. } = &db.post;
    let ps = db.post.group_by(owner_user).select(post_type_id.and(view_count.opt())).fold([0i64; 5], |a, (t, w)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.unwrap_or(0), a[4] + w.is_some() as i64]);
    let vc = db.vote.group_by(&db.vote.user).select(&db.vote.vote_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + matches!(t, 2 | 3) as i64, a[2] + (t == 3) as i64]);
    let users = db.user.with(&db.user.location).with((&db.user.creation_date).lt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let rep = (&db.user.reputation).filt(|r| r > 0);
    let v = drain(users.select(rep.opt().and((&ps).opt()).and((&vc).opt())));
    let v = top_n(v, |&(u, ((r, p), _))| (Reverse(r.unwrap_or(0)), p.is_none(), Reverse(p.map(|a| a[0])), u), 10);
    rows(v.into_iter().map(|(u, ((r, p), c))| {
        let p = p.unwrap_or([0; 5]);
        let c = c.unwrap_or([0; 3]);
        row(vec![user_col(db, u, "name"), V::I(r.unwrap_or(0)), V::I(p[0]), V::I(p[1]), V::I(p[2]), V::I(p[3]), V::I(c[0]), V::I(c[1]), V::I(c[2])])
    }))
}

// WITH TagCounts AS (SELECT Tags.TagName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.ViewCount > 100 THEN 1 ELSE 0 END) AS PopularPostCount, AVG(COALESCE(p.Score, 0)) AS AverageScore
//     FROM Tags JOIN Posts p ON p.Tags LIKE '%' || Tags.TagName || '%' GROUP BY Tags.TagName),
// TopTags AS (SELECT TagName, PostCount, PopularPostCount, AverageScore, RANK() OVER (ORDER BY PostCount DESC) AS TagRank FROM TagCounts WHERE PostCount > 1),
// UserActivity AS (SELECT u.Id AS UserId, COUNT(DISTINCT b.Id) AS BadgeCount, SUM(CASE WHEN p.LastActivityDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 YEAR' THEN 1 ELSE 0 END) AS RecentActivity,
//        SUM(CASE WHEN p.Score > 0 THEN p.Score ELSE 0 END) AS TotalScore FROM Users u LEFT JOIN Badges b ON b.UserId = u.Id LEFT JOIN Posts p ON p.OwnerUserId = u.Id GROUP BY u.Id),
// ActiveUsers AS (SELECT ua.UserId, ua.BadgeCount, ua.RecentActivity, ua.TotalScore, RANK() OVER (ORDER BY ua.RecentActivity DESC, ua.TotalScore DESC, ua.BadgeCount DESC) AS UserRank
//     FROM UserActivity ua WHERE ua.RecentActivity > 0)
// SELECT tt.TagName, tt.PostCount, tt.PopularPostCount, tt.AverageScore, au.UserId, au.BadgeCount, au.RecentActivity, au.TotalScore, au.UserRank
// FROM TopTags tt JOIN ActiveUsers au ON tt.PostCount > 5 WHERE tt.TagRank <= 10 ORDER BY tt.PostCount DESC, au.RecentActivity DESC;
//
// The ON clause names only tt, so the top tags are crossed with the active users. The LIKE join is `tag_mentions`.
fn q26252(db: &'static So) -> String {
    let Post { view_count, score, last_activity_date, .. } = &db.post;
    let lt = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&lt).map(|(_, t)| t).inv().collect();
    let tc = db
        .tag
        .group_by(Ident::<Tag>::new())
        .select((&by_tag).map(|(p, _)| p).select(view_count.opt().and(score)))
        .fold([0i64; 3], |a, (w, s)| [a[0] + 1, a[1] + (w.unwrap_or(0) > 100) as i64, a[2] + s]);
    let tc2 = (&tc).filt(|a| a[0] > 1);
    let tw = whole(&tc2).select(Ident::<Tag>::new().and(&tc2)).window(rank, |(_, a): (Id<Tag>, [i64; 3])| Reverse(a[0]), asc);
    let tt = (&tw).filt(|((_, a), r)| r <= 10 && a[0] > 5).map(|(x, _)| x);
    let since = add_years(ts(2024, 10, 1, 12, 34, 56), -1);
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(posts_of(db).select(last_activity_date.and(score)).opt()))
        .fold([0i64; 2], |a, (_, p)| match p {
            Some((d, s)) => [a[0] + (d > since) as i64, a[1] + s.max(0)],
            None => a,
        });
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let au = (&ua).filt(|a| a[0] > 0).and(&bc);
    let aw = whole(&au).select(Ident::<User>::new().and(&au)).window(rank, |(_, (a, b)): (Id<User>, ([i64; 2], i64))| (Reverse(a[0]), Reverse(a[1]), Reverse(b)), asc);
    let v = drain(tt.and(&aw));
    rows(v.into_iter().map(|(_, ((t, a), ((u, (x, b)), r)))| {
        row(vec![V::S(db.tag.tag_name.get(t).unwrap()), V::I(a[0]), V::I(a[1]), avg(a[2], a[0]), user_col(db, u, "uid"), V::I(b), V::I(x[0]), V::I(x[1]), V::I(r)])
    }))
}

// WITH UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName, u.Reputation),
// HighestScoringPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.OwnerUserId, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS Rank FROM Posts p WHERE p.PostTypeId IN (1, 2)),
// AcceptedAnswers AS (SELECT p.Id AS AnswerId, p.AcceptedAnswerId, p.OwnerUserId, p.Score AS AnswerScore FROM Posts p WHERE p.PostTypeId = 2 AND p.AcceptedAnswerId IS NOT NULL)
// SELECT ur.DisplayName, ur.Reputation, ur.PostCount, ur.TotalUpVotes, ur.TotalDownVotes, hp.PostId, hp.Title, CASE WHEN aa.AnswerId IS NOT NULL THEN aa.AnswerScore ELSE 0 END AS AcceptedAnswerScore,
//        COALESCE(hp.Score, 0) AS HighestScore
// FROM UserReputation ur LEFT JOIN HighestScoringPosts hp ON ur.UserId = hp.OwnerUserId AND hp.Rank = 1 LEFT JOIN AcceptedAnswers aa ON hp.PostId = aa.AcceptedAnswerId
// WHERE ur.Reputation > 500 ORDER BY ur.Reputation DESC, HighestScore DESC LIMIT 50;
fn q598(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, accepted_answer, .. } = &db.post;
    let users = || db.user.with((&db.user.reputation).gt(500));
    let ur = users().group_by(Ident::<User>::new()).select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt()).fold([0i64; 2], |a, t| {
        let t = t.flatten();
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
    });
    let pc = users().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let w = db.post.with(post_type_id.is_in([1, 2])).group_by(owner_user).select(Ident::<Post>::new().and(score)).window(rank, |(_, s): (Id<Post>, i64)| Reverse(s), asc);
    let best = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p);
    let aa: HashIdx<Id<Post>, Id<Post>> = db.post.with(post_type_id.eq(2)).with(accepted_answer).select(accepted_answer).inv().collect();
    let v = drain((&ur).and(&pc).and(best.select(Ident::<Post>::new().and((&aa).opt())).opt()));
    let v = top_n(v, |&(u, (_, p))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(p.map_or(0, |(p, _)| score.get(p).unwrap())), u, p.map(|(p, a)| (p, a))), 50);
    rows(v.into_iter().map(|(u, ((a, n), p))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1])]);
        f.extend(match p {
            Some((p, x)) => {
                let mut g = post_fields(db, p, &["id", "title"]);
                g.extend([V::I(x.map_or(0, |x| score.get(x).unwrap())), V::I(score.get(p).unwrap())]);
                g
            }
            None => vec![V::Null, V::Null, V::I(0), V::I(0)],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.ViewCount, COALESCE(p.AcceptedAnswerId, 0) AS AcceptedAnswerId,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn, p.OwnerUserId FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserStats AS (SELECT u.Id AS UserId, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(COALESCE(b.Class, 0)) AS TotalBadges, MAX(u.LastAccessDate) AS LastAccess
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount, MAX(c.CreationDate) AS LastCommentDate FROM Comments c GROUP BY c.PostId)
// SELECT up.UserId, up.Reputation, up.PostCount, up.TotalBadges, rp.Title, rp.Score, rp.ViewCount, COALESCE(pc.CommentCount, 0) AS CommentCount, pc.LastCommentDate,
//        CASE WHEN rp.AcceptedAnswerId = 0 THEN 'No Accepted Answer' ELSE 'Has Accepted Answer' END AS AnswerStatus
// FROM UserStats up INNER JOIN RankedPosts rp ON up.UserId = rp.OwnerUserId LEFT JOIN PostComments pc ON rp.PostId = pc.PostId WHERE up.Reputation > 1000 AND rp.rn <= 5
// ORDER BY up.Reputation DESC, rp.ViewCount DESC;
fn q3164(db: &'static So) -> String {
    let Post { owner_user, creation_date, accepted_answer_id, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d): (Id<Post>, i64)| (Reverse(d), p), asc);
    let rp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let rich: MatSet<Id<User>> = (&rp).select(owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(1000)))).collect();
    let us = (&rich).group_by(Ident::<User>::new()).select(posts_of(db).opt().and(badges_of(db).select(&db.badge.class).opt())).fold(0i64, |n, (_, c)| n + c.unwrap_or(0));
    let pc = (&rich).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let cm = (&rp).group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.creation_date)).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let v = drain((&rp).select(owner_user.select(Ident::<User>::new().and(&us).and(&pc)).and((&cm).opt())));
    rows(v.into_iter().map(|(p, (((u, b), n), c))| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend([V::I(n), V::I(b)]);
        f.extend(post_fields(db, p, &["title", "score", "views"]));
        f.extend(match c {
            Some((k, m)) => [V::I(k), V::T(m)],
            None => [V::I(0), V::Null],
        });
        f.push(V::S(if accepted_answer_id.get(p).unwrap_or(0) == 0 { "No Accepted Answer" } else { "Has Accepted Answer" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// RecentComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c WHERE c.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 month' GROUP BY c.PostId),
// PostVoteSummary AS (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes FROM Votes v GROUP BY v.PostId),
// UserBadges AS (SELECT b.UserId, COUNT(DISTINCT b.Id) AS BadgeCount FROM Badges b GROUP BY b.UserId)
// SELECT up.DisplayName, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, COALESCE(rc.CommentCount, 0) AS RecentComments, COALESCE(pvs.Upvotes, 0) AS TotalUpvotes,
//        COALESCE(pvs.Downvotes, 0) AS TotalDownvotes, ub.BadgeCount
// FROM RankedPosts rp JOIN Users up ON rp.OwnerUserId = up.Id LEFT JOIN RecentComments rc ON rp.Id = rc.PostId LEFT JOIN PostVoteSummary pvs ON rp.Id = pvs.PostId
// LEFT JOIN UserBadges ub ON up.Id = ub.UserId WHERE rp.Rank = 1 ORDER BY rp.Score DESC LIMIT 10 OFFSET 0;
//
// The Rank ties are broken by post id.
fn q178(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let w = db.post.with(creation_date.ge(add_years(t0, -1))).with(owner_user).group_by(owner_user).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s): (Id<Post>, i64)| (Reverse(s), p), asc);
    let rp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let rc = db.comment.with((&db.comment.creation_date).ge(add_months(t0, -1))).group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let pvs = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let ub = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&rp).select((&rc).opt().and((&pvs).opt()).and(owner_user.select((&ub).opt()))));
    let v = top_n(v, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 10);
    rows(v.into_iter().map(|(p, ((c, a), b))| {
        let a = a.unwrap_or([0, 0]);
        let mut f = post_fields(db, p, &["owner", "title", "created", "score", "views"]);
        f.extend([V::I(c.unwrap_or(0)), V::I(a[0]), V::I(a[1]), oint(b)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn, p.OwnerUserId
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.Score > 0),
// UserEngagement AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounty, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// RecentComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c WHERE c.CreationDate >= CURRENT_TIMESTAMP - INTERVAL '30 days' GROUP BY c.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, ue.DisplayName, ue.TotalPosts, ue.TotalBounty, ue.UpVotes, ue.DownVotes, COALESCE(rc.CommentCount, 0) AS RecentCommentCount
// FROM RankedPosts rp JOIN Users u ON rp.OwnerUserId = u.Id JOIN UserEngagement ue ON u.Id = ue.UserId LEFT JOIN RecentComments rc ON rp.PostId = rc.PostId
// WHERE rp.rn = 1 AND rp.Score > 5 ORDER BY rp.CreationDate DESC, ue.TotalPosts DESC OFFSET 10 ROWS FETCH NEXT 20 ROWS ONLY;
//
// CURRENT_TIMESTAMP is a TIMESTAMPTZ, so the comments' CreationDate is compared as a New York wall time. UserEngagement is computed for the owners that join.
fn q2323(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, post_type_id, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1)).with(score.gt(0)).with(owner_user).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d): (Id<Post>, i64)| (Reverse(d), p), asc);
    let rp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let hot = || (&rp).with(score.gt(5));
    let owners: MatSet<Id<User>> = hot().select(owner_user).collect();
    let ue = (&owners)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select((&db.vote.vote_type_id).and((&db.vote.bounty_amount).opt())).opt()).opt())
        .fold([0i64; 3], |a, x| match x.flatten() {
            Some((t, b)) => [a[0] + b.unwrap_or(0), a[1] + (t == 2) as i64, a[2] + (t == 3) as i64],
            None => a,
        });
    let pc = (&owners).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let rc = db.comment.with((&db.comment.creation_date).ge(add_days(utc_to_ny(now_utc()), -30))).group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let v = drain(hot().select(owner_user.select(Ident::<User>::new().and(&pc).and(&ue)).and((&rc).opt())));
    let v = top_n(v, |&(p, ((_, n), _))| (Reverse(creation_date.get(p).unwrap()), Reverse(n), p), 30);
    rows(v.into_iter().skip(10).map(|(p, (((u, n), a), c))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([user_col(db, u, "name"), V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(c.unwrap_or(0))]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes, COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes,
//        COUNT(DISTINCT P.Id) AS PostsCount, COUNT(DISTINCT C.Id) AS CommentsCount, AVG(EXTRACT(EPOCH FROM (TIMESTAMP '2024-10-01 12:34:56' - U.CreationDate) / 86400)) AS DaysSinceCreation
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Comments C ON P.Id = C.PostId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, Upvotes, Downvotes, PostsCount, CommentsCount, DaysSinceCreation, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserActivity)
// SELECT U.DisplayName, U.Reputation, U.Upvotes, U.Downvotes, U.PostsCount, U.CommentsCount,
//        CASE WHEN U.DaysSinceCreation < 30 THEN 'New User' WHEN U.DaysSinceCreation BETWEEN 30 AND 365 THEN 'Active User' ELSE 'Veteran User' END AS UserCategory,
//        CASE WHEN U.Reputation >= 1000 THEN 'Gold User' WHEN U.Reputation BETWEEN 500 AND 999 THEN 'Silver User' ELSE 'Bronze User' END AS BadgeCategory
// FROM TopUsers U WHERE U.ReputationRank <= 10 ORDER BY U.Reputation DESC;
//
// ReputationRank reads only Reputation, so the top users are picked first and the posts x votes x comments product is driven for them alone.
fn q2838(db: &'static So) -> String {
    let w = whole(&db.user.reputation).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r): (Id<User>, i64)| Reverse(r), asc);
    let tu: MatSet<Id<User>> = (&w).filt(|(_, r)| r <= 10).map(|((u, _), _)| u).collect();
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let g = || (&tu).group_by(Ident::<User>::new());
    let s = g()
        .select((&db.user.creation_date).and(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt())).opt()))
        .fold(([0i64; 2], 0.0f64, 0i64), |(a, d, n), (c, p)| {
            let t = p.and_then(|(t, _)| t);
            ([a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64], d + secs(t0 - c) / 86400.0, n + 1)
        });
    let pc = g().select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let cc = g().select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&s).and(&pc).and(&cc).and(&db.user.reputation));
    rows(v.into_iter().map(|(u, ((((a, d, n), p), c), r))| {
        let days = d / n as f64;
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(p), V::I(c)]);
        f.push(V::S(if days < 30.0 { "New User" } else if days <= 365.0 { "Active User" } else { "Veteran User" }));
        f.push(V::S(if r >= 1000 { "Gold User" } else if r >= 500 { "Silver User" } else { "Bronze User" }));
        row(f)
    }))
}

// WITH RECURSIVE PostRankings AS (SELECT p.Id AS PostId, p.Title, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
//        RANK() OVER (ORDER BY COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) - COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) DESC) AS Rank
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName HAVING COUNT(DISTINCT p.Id) > 10)
// SELECT p.PostId, p.Title, p.UpVotes, p.DownVotes, p.Rank, u.DisplayName, u.PostCount, u.QuestionCount, u.AnswerCount, CASE WHEN p.Rank <= 10 THEN 'Top Post' ELSE 'Regular Post' END AS PostCategory
// FROM PostRankings p JOIN TopUsers u ON p.PostId IN (SELECT p.Id FROM Posts p WHERE p.OwnerUserId = u.UserId) WHERE EXISTS (SELECT 1 FROM Comments c WHERE c.PostId = p.PostId AND c.Score > 0)
// ORDER BY p.Rank, u.PostCount DESC LIMIT 50;
//
// WITH RECURSIVE, but no CTE refers to itself. The IN subquery pairs a post with its owner.
fn q31275(db: &'static So) -> String {
    let Post { creation_date, owner_user, post_type_id, .. } = &db.post;
    let pr = db
        .post
        .with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt())
        .fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let w = whole(&pr).select(Ident::<Post>::new().and(&pr)).window(rank, |(_, a): (Id<Post>, [i64; 2])| Reverse(a[0] - a[1]), asc);
    let rk: MatSet<(Id<Post>, ([i64; 2], i64))> = (&w).map(|((p, a), r)| (p, (a, r))).collect();
    let tu = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id)).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64]);
    let liked = comments_of(db).select(Ident::<Comment>::new().with((&db.comment.score).gt(0)));
    let v = drain(by_first(&rk).and(Ident::<Post>::new().with(liked).select(owner_user.select(Ident::<User>::new().and((&tu).filt(|a| a[0] > 10))))));
    let v = top_n(v, |&(p, ((_, r), (_, a)))| (r, Reverse(a[0]), p), 50);
    rows(v.into_iter().map(|(p, ((a, r), (u, t)))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(r), user_col(db, u, "name"), V::I(t[0]), V::I(t[1]), V::I(t[2]), V::S(if r <= 10 { "Top Post" } else { "Regular Post" })]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U),
// PostSummary AS (SELECT P.OwnerUserId, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(CASE WHEN P.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers,
//        AVG(P.Score) AS AverageScore FROM Posts P WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY P.OwnerUserId),
// VoteSummary AS (SELECT V.UserId, COUNT(V.Id) AS TotalVotes, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes V GROUP BY V.UserId)
// SELECT UR.DisplayName, UR.Reputation, PS.TotalPosts, PS.TotalQuestions, PS.AcceptedAnswers, PS.AverageScore, COALESCE(VS.TotalVotes, 0) AS TotalVotes, COALESCE(VS.UpVotes, 0) AS UpVotes,
//        COALESCE(VS.DownVotes, 0) AS DownVotes, CASE WHEN UR.ReputationRank <= 10 THEN 'Top Contributor' WHEN UR.ReputationRank <= 50 THEN 'Regular Contributor' ELSE 'New Contributor' END AS ContributorLevel
// FROM UserReputation UR LEFT JOIN PostSummary PS ON UR.UserId = PS.OwnerUserId LEFT JOIN VoteSummary VS ON UR.UserId = VS.UserId
// WHERE UR.Reputation > 1000 AND (PS.TotalPosts > 5 OR VS.TotalVotes > 10) ORDER BY UR.Reputation DESC LIMIT 100;
fn q2611(db: &'static So) -> String {
    let Post { owner_user, creation_date, post_type_id, accepted_answer_id, score, .. } = &db.post;
    let w = whole(&db.user.reputation).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r): (Id<User>, i64)| Reverse(r), asc);
    let rk: MatSet<(Id<User>, i64)> = (&w).map(|((u, _), r)| (u, r)).collect();
    let ps = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(owner_user)
        .select(post_type_id.and(accepted_answer_id.opt()).and(score))
        .fold([0i64; 4], |a, ((t, x), s)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + x.is_some() as i64, a[3] + s]);
    let vs = db.vote.group_by(&db.vote.user).select(&db.vote.vote_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64]);
    let keep = |(p, v): (Option<[i64; 4]>, Option<[i64; 3]>)| p.map_or(false, |a| a[0] > 5) || v.map_or(false, |a| a[0] > 10);
    let v = drain(db.user.with((&db.user.reputation).gt(1000)).select(by_first(&rk).and((&ps).opt().and((&vs).opt()).filt(keep))));
    let v = top_n(v, |&(u, _)| (Reverse(db.user.reputation.get(u).unwrap()), u), 100);
    rows(v.into_iter().map(|(u, (r, (p, c)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(match p {
            Some(a) => [V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[3], a[0])],
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        let c = c.unwrap_or([0; 3]);
        f.extend([V::I(c[0]), V::I(c[1]), V::I(c[2])]);
        f.push(V::S(if r <= 10 { "Top Contributor" } else if r <= 50 { "Regular Contributor" } else { "New Contributor" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS RankByScore,
//        COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount, COUNT(v.Id) OVER (PARTITION BY p.Id, v.VoteTypeId) AS UpvoteCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 2 WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.RankByScore, rp.CommentCount, COALESCE(rp.UpvoteCount, 0) AS UpvoteCount, p.OwnerUserId
//     FROM RankedPosts rp JOIN Posts p ON rp.PostId = p.Id WHERE rp.RankByScore <= 3),
// UserBadges AS (SELECT b.UserId, b.Class, COUNT(*) AS BadgeCount FROM Badges b GROUP BY b.UserId, b.Class)
// SELECT u.DisplayName, COUNT(DISTINCT tp.PostId) AS PostsCount, SUM(CASE WHEN ub.Class = 1 THEN 1 ELSE 0 END) AS GoldBadgeCount, SUM(CASE WHEN ub.Class = 2 THEN 1 ELSE 0 END) AS SilverBadgeCount,
//        SUM(CASE WHEN ub.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadgeCount
// FROM Users u LEFT JOIN TopPosts tp ON u.Id = tp.OwnerUserId LEFT JOIN UserBadges ub ON u.Id = ub.UserId WHERE u.Reputation > 1000 GROUP BY u.DisplayName
// HAVING COUNT(DISTINCT tp.PostId) > 5 ORDER BY PostsCount DESC, u.DisplayName ASC LIMIT 10;
//
// RankByScore numbers the post x comment x upvote rows, so those rows are materialised and numbered (ties broken by post, comment and vote id; the answer is
// empty on this data). The (user, top row) and (user, class) pairs are grouped by DisplayName.
fn q813(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, .. } = &db.post;
    let up = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2)));
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(owner_user)
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(score).and(comments_of(db).opt()).and(up.opt()))
        .window(row_number, |(((p, s), c), v): (((Id<Post>, i64), Option<Id<Comment>>), Option<Id<Vote>>)| (Reverse(s), p, c, v), asc);
    let tp = || (&w).filt(|(_, r)| r <= 3).map(|((((p, _), _), _), _)| p);
    let pairs: MatSet<(Id<User>, i64)> = db.badge.select((&db.badge.user).and(&db.badge.class)).collect();
    let classes = by_first(&pairs);
    let rich = || db.user.with((&db.user.reputation).gt(1000));
    let pc = rich().group_by(&db.user.display_name).select(tp()).count_distinct();
    let bc = rich()
        .group_by(&db.user.display_name)
        .select(tp().opt().and((&classes).opt()))
        .fold([0i64; 3], |a, (_, c)| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    let v = top_n(drain((&pc).filt(|n| n > 5).and(&bc)), |&(n, (k, _))| (Reverse(k), n), 10);
    rows(v.into_iter().map(|(n, (k, a))| row(vec![V::S(n), V::I(k), V::I(a[0]), V::I(a[1]), V::I(a[2])])))
}

// Rewritten (rewrites/1207.sql): the RankedPosts ROW_NUMBER is tie-broken on p.Id.
// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.Id) AS Rank
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.Score > 0),
// PostStatistics AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalQuestions, COALESCE(SUM(CASE WHEN b.Class = 1 THEN 1 END), 0) AS GoldBadges,
//        COALESCE(SUM(CASE WHEN b.Class = 2 THEN 1 END), 0) AS SilverBadges, COALESCE(SUM(CASE WHEN b.Class = 3 THEN 1 END), 0) AS BronzeBadges,
//        COALESCE(AVG(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 END), 0) AS AverageCloseReasons, COALESCE(AVG(p.ViewCount), 0) AS AverageViews
//     FROM Users u LEFT JOIN Posts p ON p.OwnerUserId = u.Id LEFT JOIN Badges b ON b.UserId = u.Id LEFT JOIN PostHistory ph ON ph.UserId = u.Id AND ph.PostId = p.Id
//     WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year' GROUP BY u.Id, u.DisplayName)
// SELECT ps.DisplayName, ps.TotalQuestions, ps.GoldBadges, ps.SilverBadges, ps.BronzeBadges, ps.AverageCloseReasons, ps.AverageViews, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount
// FROM PostStatistics ps LEFT JOIN RankedPosts rp ON ps.UserId = rp.PostId WHERE ps.TotalQuestions > 10 AND rp.Rank <= 5 ORDER BY ps.TotalQuestions DESC, ps.DisplayName LIMIT 50 OFFSET 0;
//
// The WHERE on p.CreationDate makes the posts join inner. `ph.UserId = u.Id AND ph.PostId = p.Id` is the post's history rows written by its owner.
// `ps.UserId = rp.PostId` joins a user id to a post id, so it goes through the raw ids.
fn q1207(db: &'static So) -> String {
    let Post { owner_user, owner_user_id, creation_date, score, view_count, post_type_id, origid, .. } = &db.post;
    let PostHistory { user: hu, post: hp, post_history_type_id, .. } = &db.post_history;
    let own_hist: HashIdx<Id<Post>, Id<PostHistory>> = db.post_history.with(hu.and(hp.select(owner_user)).filt(|(a, b)| a == b)).select(hp).inv().collect();
    let recent = || Ident::<Post>::new().with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let g = || db.user.group_by(Ident::<User>::new());
    let s = g()
        .select(posts_of(db).select(recent()).select(view_count.opt().and((&own_hist).select(post_history_type_id).opt())).and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 6], |a, ((w, t), b)| [a[0] + (b == Some(1)) as i64, a[1] + (b == Some(2)) as i64, a[2] + (b == Some(3)) as i64, a[3] + (t == Some(10)) as i64, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0)]);
    let tq = g().select(posts_of(db).select(recent())).fold(0i64, |n, _| n + 1);
    let w = db.post.with(post_type_id.eq(1)).with(score.gt(0)).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s): (Id<Post>, i64)| (Reverse(s), p), asc);
    let top: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let rp: HashIdx<i64, Id<Post>> = (&top).select(origid).inv().collect();
    let v = drain((&tq).filt(|n| n > 10).and(&s).and((&db.user.origid).select(&rp)));
    let v = top_n(v, |&(u, ((n, _), _))| (Reverse(n), db.user.display_name.get(u).unwrap(), u), 50);
    rows(v.into_iter().map(|(u, ((n, a), p))| {
        let mut f = vec![user_col(db, u, "name"), V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2])];
        f.push(V::F(if a[3] > 0 { 1.0 } else { 0.0 }));
        f.push(V::F(if a[4] == 0 { 0.0 } else { a[5] as f64 / a[4] as f64 }));
        f.extend(post_fields(db, p, &["title", "created", "score", "views"]));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END), 0) AS QuestionsAsked,
//        COALESCE(SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS AnswersGiven, COALESCE(SUM(CASE WHEN c.Id IS NOT NULL THEN 1 ELSE 0 END), 0) AS CommentsMade,
//        COALESCE(SUM(CASE WHEN b.Id IS NOT NULL THEN 1 ELSE 0 END), 0) AS BadgesReceived
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// UserScore AS (SELECT UserId, DisplayName, Reputation + (QuestionsAsked * 5) + (AnswersGiven * 10) + (CommentsMade * 3) + (BadgesReceived * 20) AS TotalScore FROM UserActivity),
// TopUsers AS (SELECT UserId, DisplayName, TotalScore, RANK() OVER (ORDER BY TotalScore DESC) AS Rank FROM UserScore)
// SELECT u.DisplayName, u.Reputation, ua.QuestionsAsked, ua.AnswersGiven, ua.CommentsMade, ua.BadgesReceived, t.TotalScore, t.Rank,
//        CASE WHEN t.Rank <= 10 THEN 'Top Contributor' ELSE 'Regular Contributor' END AS ContributorType
// FROM UserActivity ua JOIN TopUsers t ON ua.UserId = t.UserId JOIN Users u ON ua.UserId = u.Id WHERE u.LastAccessDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' ORDER BY t.Rank;
//
// The arithmetic is the query's own TotalScore formula over the product's sums.
fn q3401(db: &'static So) -> String {
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(comments_of(db).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 4], |a, (p, b)| {
            let (t, c) = p.map_or((0, false), |(t, c)| (t, c.is_some()));
            [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + c as i64, a[3] + b.is_some() as i64]
        });
    let total = (&s).and(&db.user.reputation).map(|(a, r): ([i64; 4], i64)| (a, r + a[0] * 5 + a[1] * 10 + a[2] * 3 + a[3] * 20));
    let w = whole(&s).select(Ident::<User>::new().and(total)).window(rank, |(_, (_, t)): (Id<User>, ([i64; 4], i64))| Reverse(t), asc);
    let rk: MatSet<(Id<User>, ([i64; 4], i64, i64))> = (&w).map(|((u, (a, t)), r)| (u, (a, t, r))).collect();
    let since = add_years(ts(2024, 10, 1, 12, 34, 56), -1);
    let v = drain(db.user.with((&db.user.last_access_date).ge(since)).select(by_first(&rk)));
    rows(v.into_iter().map(|(u, (a, t, r))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(a.map(V::I));
        f.extend([V::I(t), V::I(r), V::S(if r <= 10 { "Top Contributor" } else { "Regular Contributor" })]);
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.OwnerUserId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, SUM(COALESCE(b.Class, 0)) AS TotalBadges, AVG(COALESCE(v.BountyAmount, 0)) AS AverageBounty
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName),
// PostInteractions AS (SELECT p.Id AS PostId, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT pl.RelatedPostId) AS RelatedPosts FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId
//     LEFT JOIN PostLinks pl ON p.Id = pl.PostId GROUP BY p.Id)
// SELECT rp.Title, u.DisplayName, us.TotalBadges, us.AverageBounty, pi.CommentCount, pi.RelatedPosts, CASE WHEN (pi.CommentCount > 5) THEN 'High Activity' ELSE 'Low Activity' END AS ActivityLevel
// FROM RecentPosts rp JOIN Users u ON rp.OwnerUserId = u.Id JOIN UserStats us ON u.Id = us.UserId JOIN PostInteractions pi ON rp.PostId = pi.PostId
// WHERE us.AverageBounty > 0 AND rp.AnswerCount > 2 ORDER BY rp.CreationDate DESC LIMIT 100;
//
// rn is never read. UserStats is computed for the owners that join; AverageBounty is compared as its exact sum over row count.
fn q3812(db: &'static So) -> String {
    let Post { owner_user, creation_date, answer_count, .. } = &db.post;
    let rp = || db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(answer_count.gt(2)).with(owner_user);
    let owners: MatSet<Id<User>> = rp().select(owner_user).collect();
    let us = (&owners)
        .group_by(Ident::<User>::new())
        .select(badges_of(db).select(&db.badge.class).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold([0i64; 3], |a, (c, b)| [a[0] + c.unwrap_or(0), a[1] + b.flatten().unwrap_or(0), a[2] + 1]);
    let g = || rp().group_by(Ident::<Post>::new());
    let pi = g().select(comments_of(db).opt().and(links_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let rl = g().select(links_of(db).select(&db.post_link.related_post_id)).count_distinct();
    let v = drain((&pi).and((&rl).opt()).and(owner_user.select(Ident::<User>::new().and((&us).filt(|a| a[1] > 0)))));
    let v = top_n(v, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 100);
    rows(v.into_iter().map(|(p, ((c, r), (u, a)))| {
        let mut f = post_fields(db, p, &["title"]);
        f.extend([user_col(db, u, "name"), V::I(a[0]), avg(a[1], a[2]), V::I(c), V::I(r.unwrap_or(0)), V::S(if c > 5 { "High Activity" } else { "Low Activity" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS OwnerName, COUNT(c.Id) AS CommentCount, ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate > (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year')
//     GROUP BY p.Id, p.Title, p.CreationDate, u.DisplayName),
// TopPosts AS (SELECT PostId, Title, OwnerName, CreationDate, CommentCount FROM RankedPosts WHERE rn = 1),
// PostVoteSummary AS (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes v GROUP BY v.PostId)
// SELECT t.PostId, t.Title, t.OwnerName, t.CreationDate, COALESCE(pv.UpVotes, 0) AS TotalUpVotes, COALESCE(pv.DownVotes, 0) AS TotalDownVotes, t.CommentCount,
//        CASE WHEN t.CommentCount = 0 THEN 'No Comments' WHEN t.CommentCount > 0 AND t.CommentCount <= 5 THEN 'Few Comments' ELSE 'Many Comments' END AS CommentStatus
// FROM TopPosts t LEFT JOIN PostVoteSummary pv ON t.PostId = pv.PostId WHERE EXISTS (SELECT 1 FROM Posts p WHERE p.AcceptedAnswerId IS NOT NULL AND p.Id = t.PostId)
// ORDER BY t.CreationDate DESC LIMIT 10;
//
// rn partitions by the post itself, so every post is its own first row.
fn q4608(db: &'static So) -> String {
    let Post { creation_date, accepted_answer_id, .. } = &db.post;
    let tp = || db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(accepted_answer_id);
    let cc = tp().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let pv = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let v = top_n(drain((&cc).and((&pv).opt())), |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 10);
    rows(v.into_iter().map(|(p, (c, a))| {
        let a = a.unwrap_or([0, 0]);
        let mut f = post_fields(db, p, &["id", "title", "owner", "created"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c), V::S(if c == 0 { "No Comments" } else if c <= 5 { "Few Comments" } else { "Many Comments" })]);
        row(f)
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("3056", q3056),
    ("9276", q9276),
    ("1021", q1021),
    ("1463", q1463),
    ("28881", q28881),
    ("3247", q3247),
    ("831", q831),
    ("952", q952),
    ("4904", q4904),
    ("3359", q3359),
    ("5928", q5928),
    ("1927", q1927),
    ("939", q939),
    ("5754", q5754),
    ("4377", q4377),
    ("2038", q2038),
    ("5308", q5308),
    ("2542", q2542),
    ("6486", q6486),
    ("29645", q29645),
    ("7046", q7046),
    ("5541", q5541),
    ("23985", q23985),
    ("1912", q1912),
    ("2348", q2348),
    ("205", q205),
    ("4643", q4643),
    ("22677", q22677),
    ("32", q32),
    ("2666", q2666),
    ("2521", q2521),
    ("6510", q6510),
    ("9189", q9189),
    ("34864", q34864),
    ("28566", q28566),
    ("4442", q4442),
    ("54", q54),
    ("1919", q1919),
    ("2084", q2084),
    ("1651", q1651),
    ("20119", q20119),
    ("1743", q1743),
    ("149", q149),
    ("2399", q2399),
    ("32294", q32294),
    ("33577", q33577),
    ("3167", q3167),
    ("8231", q8231),
    ("4363", q4363),
    ("9609", q9609),
    ("4850", q4850),
    ("24651", q24651),
    ("9328", q9328),
    ("2793", q2793),
    ("2459", q2459),
    ("2525", q2525),
    ("30505", q30505),
    ("2496", q2496),
    ("1470", q1470),
    ("4232", q4232),
    ("4203", q4203),
    ("525", q525),
    ("4858", q4858),
    ("5416", q5416),
    ("4546", q4546),
    ("7208", q7208),
    ("1255", q1255),
    ("2698", q2698),
    ("32777", q32777),
    ("22813", q22813),
    ("9774", q9774),
    ("1697", q1697),
    ("33143", q33143),
    ("5911", q5911),
    ("1783", q1783),
    ("1956", q1956),
    ("27675", q27675),
    ("99", q99),
    ("2010", q2010),
    ("26252", q26252),
    ("598", q598),
    ("3164", q3164),
    ("178", q178),
    ("2323", q2323),
    ("2838", q2838),
    ("31275", q31275),
    ("2611", q2611),
    ("813", q813),
    ("1207", q1207),
    ("3401", q3401),
    ("3812", q3812),
    ("4608", q4608),
];
