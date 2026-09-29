use harness::prelude::*;
use std::cmp::Reverse;

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS OwnerDisplayName, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount,
//        RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS ScoreRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, u.DisplayName, p.Score, p.ViewCount, p.PostTypeId),
// TopRankedPosts AS (SELECT PostId, Title, CreationDate, OwnerDisplayName, Score, ViewCount, CommentCount FROM RankedPosts WHERE ScoreRank <= 5),
// PostsWithBadges AS (SELECT tr.PostId, tr.Title, tr.CreationDate, tr.OwnerDisplayName, tr.Score, tr.ViewCount, tr.CommentCount, COUNT(b.Id) AS BadgeCount
//     FROM TopRankedPosts tr LEFT JOIN Badges b ON tr.OwnerDisplayName = (SELECT DisplayName FROM Users WHERE Id = b.UserId)
//     GROUP BY tr.PostId, tr.Title, tr.CreationDate, tr.OwnerDisplayName, tr.Score, tr.ViewCount, tr.CommentCount)
// SELECT p.PostId, p.Title, p.CreationDate, p.OwnerDisplayName, p.Score, p.ViewCount, p.CommentCount, p.BadgeCount, STRING_AGG(pt.Name, ', ') AS PostTypeNames
// FROM PostsWithBadges p JOIN PostTypes pt ON pt.Id = (SELECT PostTypeId FROM Posts WHERE Id = p.PostId)
// GROUP BY p.PostId, p.Title, p.CreationDate, p.OwnerDisplayName, p.Score, p.ViewCount, p.CommentCount, p.BadgeCount ORDER BY p.Score DESC, p.ViewCount DESC;
//
// ScoreRank reads only base columns, so the posts are picked first. The badge join is on the badge owner's DisplayName, so badges are indexed by that name.
// Each post has one type, so the STRING_AGG is that type's name.
fn q6333(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| Reverse(score.get(p).unwrap()), 5, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let bidx: HashIdx<Str, Id<Badge>> = (&db.badge.user).select(&db.user.display_name).inv().collect();
    let bc = (&tp).group_by(Ident::<Post>::new()).select(owner_user.select(&db.user.display_name).select(&bidx).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = drain((&cc).and(&bc));
    rows(v.into_iter().map(|(p, (c, b))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner", "score", "views"]);
        f.extend([V::I(c), V::I(b)]);
        f.extend(post_fields(db, p, &["type"]));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(COALESCE(v.BountyAmount, 0)) AS TotalBountyEarned,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId WHERE u.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
//     GROUP BY u.Id, u.DisplayName, u.Reputation),
// PostStats AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.AnswerCount, p.CommentCount, COALESCE(ph.CreationDate, p.CreationDate) AS LastActiveDate,
//        RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.ViewCount DESC) AS RankByViews
//     FROM Posts p LEFT JOIN PostHistory ph ON p.Id = ph.PostId AND ph.PostHistoryTypeId IN (10, 11))
// SELECT u.DisplayName AS User, u.Reputation, ps.Title AS PostTitle, ps.ViewCount, ps.AnswerCount, ps.CommentCount, ps.LastActiveDate, (u.TotalBountyEarned - u.TotalDownvotes) AS EffectiveBounty,
//        CASE WHEN EXISTS (SELECT 1 FROM Badges b WHERE b.UserId = u.UserId AND b.Class = 1) THEN 'Gold' WHEN EXISTS (SELECT 1 FROM Badges b WHERE b.UserId = u.UserId AND b.Class = 2) THEN 'Silver'
//        ELSE 'Bronze or No Badge' END AS BadgeClass,
//        CASE WHEN ps.RankByViews <= 5 THEN 'Top 5 Posts' WHEN ps.RankByViews IS NULL THEN 'No Views' ELSE 'Regular Post' END AS PostRank
// FROM UserReputation u JOIN PostStats ps ON u.UserId = ps.PostId WHERE u.Reputation > (SELECT AVG(Reputation) FROM Users) AND ps.ViewCount > 100
// ORDER BY u.Reputation DESC, ps.ViewCount DESC LIMIT 50;
//
// `u.UserId = ps.PostId` joins a user id to a post id, so it goes through the raw ids. RankByViews ranks the post x close-history rows.
fn q23466(db: &'static So) -> String {
    let Post { post_type_id, view_count, creation_date, .. } = &db.post;
    let PostHistory { post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let ph = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.is_in([10, 11])));
    type J = (Id<Post>, (Option<i64>, Option<Id<PostHistory>>));
    let j: Vec<J> = drain(db.post.select(Ident::<Post>::new().and(view_count.opt().and(ph.opt())))).into_iter().map(|x| x.1).collect();
    let r = per_group(ranked(j, |&(p, (w, _))| (post_type_id.get(p).unwrap(), w.is_none(), Reverse(w)), false), |x| post_type_id.get(x.0).unwrap());
    type R = (Id<Post>, Option<i64>, Option<Id<PostHistory>>, i64);
    let rv = rel(r.into_iter().map(|((p, (w, h)), k)| (p, w, h, k)).collect::<Vec<R>>());
    let by_id: HashIdx<i64, R> = (&rv).filt(|x: R| x.1.map_or(false, |w| w > 100)).map(|x: R| db.post.origid.get(x.0).unwrap()).inv().select(&rv).collect();
    let (s, n) = (&db.user.reputation).fold_flat((0i64, 0i64), |(s, n), r| (s + r, n + 1));
    let mean = s as f64 / n as f64;
    let users = || db.user.with((&db.user.creation_date).ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with((&db.user.reputation).filt(move |r| r as f64 > mean));
    let ur = users()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select((&db.vote.vote_type_id).and((&db.vote.bounty_amount).opt())).opt()).opt())
        .fold([0i64; 2], |a, x| match x.flatten() {
            Some((t, b)) => [a[0] + b.unwrap_or(0), a[1] + (t == 3) as i64],
            None => a,
        });
    let class = |c: i64| Ident::<User>::new().with(badges_of(db).select(Ident::<Badge>::new().with((&db.badge.class).eq(c)))).opt();
    let v = drain((&ur).and((&db.user.origid).select(&by_id)).and(class(1)).and(class(2)));
    let v = top_n(v, |&(u, (((_, (p, w, h, _)), _), _))| (Reverse(db.user.reputation.get(u).unwrap()), w.is_none(), Reverse(w), u, p, h), 50);
    rows(v.into_iter().map(|(u, (((a, (p, _, h, k)), g), s))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["title", "views", "answers", "comments"]));
        f.push(V::T(h.map_or(creation_date.get(p).unwrap(), |h| hd.get(h).unwrap())));
        f.push(V::I(a[0] - a[1]));
        f.push(V::S(if g.is_some() { "Gold" } else if s.is_some() { "Silver" } else { "Bronze or No Badge" }));
        f.push(V::S(if k <= 5 { "Top 5 Posts" } else { "Regular Post" }));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.CreationDate, U.LastAccessDate, COUNT(DISTINCT P.Id) AS TotalPosts,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
//        SUM(CASE WHEN P.PostTypeId = 1 AND P.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers, SUM(COALESCE(B.Class, 0)) AS TotalBadges
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName, U.Reputation, U.CreationDate, U.LastAccessDate),
// PostActivity AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.LastActivityDate, P.Score, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount, COUNT(V.Id) AS VoteCount
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId WHERE P.CreationDate >= CURRENT_TIMESTAMP - INTERVAL '1 year'
//     GROUP BY P.Id, P.Title, P.CreationDate, P.LastActivityDate, P.Score),
// RankedPosts AS (SELECT PA.*, RANK() OVER (ORDER BY PA.Score DESC) AS PostRank FROM PostActivity PA)
// SELECT US.UserId, US.DisplayName, US.Reputation, US.TotalPosts, US.Questions, US.Answers, US.AcceptedAnswers, US.TotalBadges, RP.PostId, RP.Title, RP.CreationDate,
//        RP.LastActivityDate, RP.Score, RP.CommentCount, RP.VoteCount, RP.PostRank
// FROM UserStats US JOIN RankedPosts RP ON US.UserId = RP.PostId WHERE US.Reputation > 1000 ORDER BY US.Reputation DESC, RP.Score DESC LIMIT 10;
//
// CURRENT_TIMESTAMP is TIMESTAMPTZ, so CreationDate is read as New York local time. `US.UserId = RP.PostId` joins a user id to a post id, so it goes
// through the raw ids, and the posts x badges product is driven only for the users that match.
fn q6879(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, accepted_answer_id, .. } = &db.post;
    let since = ny_to_utc(add_years(utc_to_ny(now_utc()), -1));
    let pa = || db.post.with(creation_date.filt(move |d| ny_to_utc(d) >= since));
    let agg = pa().group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold([0i64; 2], |a, (c, v)| [a[0] + c.is_some() as i64, a[1] + v.is_some() as i64]);
    let r = ranked(drain(pa().select(score)), |&(_, s)| Reverse(s), false);
    type R = (Id<Post>, i64);
    let rv = rel(r.into_iter().map(|((p, _), k)| (p, k)).collect::<Vec<R>>());
    let by_id: HashIdx<i64, R> = (&rv).map(|x: R| db.post.origid.get(x.0).unwrap()).inv().select(&rv).collect();
    let users: MatSet<Id<User>> = db.user.with((&db.user.reputation).gt(1000)).with((&db.user.origid).select(&by_id)).collect();
    let us = (&users)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(accepted_answer_id.opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 4], |a, (p, b)| {
            let (q, an, ac) = p.map_or((false, false, false), |(t, x)| (t == 1, t == 2, t == 1 && x.is_some()));
            [a[0] + q as i64, a[1] + an as i64, a[2] + ac as i64, a[3] + b.unwrap_or(0)]
        });
    let pc = (&users).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let v = drain((&us).and(&pc).and((&db.user.origid).select(&by_id).select(Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0).select(&agg)))));
    let v = top_n(v, |&(u, (_, ((p, _), _)))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(score.get(p).unwrap()), u, p), 10);
    rows(v.into_iter().map(|(u, ((a, n), ((p, k), c)))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3])]);
        f.extend(post_fields(db, p, &["id", "title", "created", "activity", "score"]));
        f.extend([V::I(c[0]), V::I(c[1]), V::I(k)]);
        row(f)
    }))
}

// WITH UserVoteCounts AS (SELECT u.Id AS UserId, COUNT(v.Id) AS TotalVotes, SUM(CASE WHEN v.VoteTypeId IN (2, 8) THEN 1 ELSE 0 END) AS Upvotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id),
// PostDetails AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, p.Score, COALESCE(MAX(CASE WHEN ph.PostHistoryTypeId = 10 THEN ph.CreationDate END), p.CreationDate) AS ClosedDate,
//        COUNT(DISTINCT c.Id) AS CommentCount, SUM(CASE WHEN b.Id IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId LEFT JOIN Badges b ON p.OwnerUserId = b.UserId AND b.Date <= p.CreationDate
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.Score),
// RankedPosts AS (SELECT pd.*, RANK() OVER (PARTITION BY pd.OwnerUserId ORDER BY pd.Score DESC) AS OwnerPostRank FROM PostDetails pd)
// SELECT up.UserId, u.DisplayName, rp.PostId, rp.Title, rp.CreationDate, rp.ClosedDate, rp.CommentCount, rp.BadgeCount, up.TotalVotes, up.Upvotes, up.Downvotes
// FROM UserVoteCounts up JOIN RankedPosts rp ON rp.OwnerUserId = up.UserId JOIN Users u ON up.UserId = u.Id
// WHERE ((rp.OwnerPostRank = 1 AND up.Upvotes > up.Downvotes) OR (rp.CommentCount > 10 AND rp.BadgeCount >= 1)) AND (rp.ClosedDate IS NULL OR rp.ClosedDate > rp.CreationDate)
// ORDER BY rp.Score DESC, up.TotalVotes DESC LIMIT 50;
//
// BadgeCount counts the comments x history x earlier-badges rows. The ownerless partition never joins a user, so only owned posts are ranked.
fn q21947(db: &'static So) -> String {
    let Post { creation_date, score, owner_user, .. } = &db.post;
    let PostHistory { post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let early = Ident::<Post>::new()
        .and(owner_user.select(badges_of(db)))
        .filt(move |(p, b): (Id<Post>, Id<Badge>)| db.badge.date.get(b).unwrap() <= creation_date.get(p).unwrap())
        .map(|(_, b): (Id<Post>, Id<Badge>)| b);
    let pd = recent()
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(history_of(db).select(post_history_type_id.and(hd)).opt()).and(early.opt()))
        .fold((i64::MIN, 0i64), |(m, n), ((_, h), b)| (h.map_or(m, |(t, d)| if t == 10 { m.max(d) } else { m }), n + b.is_some() as i64));
    let cc = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let r = per_group(ranked(drain(recent().select(owner_user)), |&(p, u)| (u, Reverse(score.get(p).unwrap())), false), |x| x.1);
    type R = (Id<Post>, Id<User>, i64);
    let rv = rel(r.into_iter().map(|((p, u), k)| (p, u, k)).collect::<Vec<R>>());
    let uvc = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + matches!(t, 2 | 8) as i64, a[2] + (t == 3) as i64],
        None => a,
    });
    type X = (R, (((i64, i64), i64), [i64; 3]));
    let v = drain(
        (&rv)
            .select(Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0).select((&pd).and(&cc)).and(Same::<R>::new().map(|x: R| x.1).select(&uvc))))
            .filt(move |((p, _, k), (((m, b), c), a)): X| ((k == 1 && a[1] > a[2]) || (c > 10 && b >= 1)) && m != i64::MIN && m.max(creation_date.get(p).unwrap()) > creation_date.get(p).unwrap()),
    );
    let v = top_n(v, |&(_, ((p, _, _), (_, a)))| (Reverse(score.get(p).unwrap()), Reverse(a[0]), p), 50);
    rows(v.into_iter().map(|(_, ((p, u, _), (((m, b), c), a)))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(post_fields(db, p, &["id", "title", "created"]));
        f.extend([V::T(m), V::I(c), V::I(b), V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        row(f)
    }))
}

fn leak(parts: Vec<&str>, sep: &str) -> Str {
    Box::leak(parts.join(sep).into_boxed_str())
}

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

// WITH UserEngagement AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVotes,
//        COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVotes, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount, SUM(COALESCE(P.ViewCount, 0)) AS TotalViews,
//        ROW_NUMBER() OVER (ORDER BY COUNT(DISTINCT P.Id) DESC) AS EngagementRank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Comments C ON P.Id = C.PostId GROUP BY U.Id, U.DisplayName, U.Reputation),
// MostActivePosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate AS PostDate, P.Score, P.ViewCount, COALESCE(COUNT(C.Id), 0) AS CommentCount, COALESCE(SUM(V.BountyAmount), 0) AS TotalBounties,
//        ROW_NUMBER() OVER (ORDER BY P.Score DESC, P.ViewCount DESC) AS ActivityRank
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId = 8
//     WHERE P.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY P.Id, P.Title, P.CreationDate, P.Score, P.ViewCount)
// SELECT U.DisplayName AS User, U.PostCount, U.UpVotes, U.DownVotes, U.CommentCount, U.TotalViews, P.Title AS ActivePost, P.PostDate, P.Score AS PostScore,
//        P.CommentCount AS ActivePostCommentCount, P.TotalBounties
// FROM UserEngagement U LEFT JOIN MostActivePosts P ON U.PostCount > 0 WHERE U.EngagementRank <= 10 AND (P.ActivityRank <= 5 OR P.TotalBounties > 0)
// ORDER BY U.Reputation DESC, U.TotalViews DESC;
//
// EngagementRank reads only the post count, so the ten users are picked first and the posts x votes x comments product is driven for them alone.
// The ON names only U, so they are crossed with the posts; the WHERE drops the NULL row. ActivityRank breaks (Score, ViewCount) ties by post id.
fn q3656(db: &'static So) -> String {
    let Post { creation_date, score, view_count, .. } = &db.post;
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let top = top_n(drain(&pc), |&(u, n)| (Reverse(n), u), 10);
    let tu: MatSet<Id<User>> = rel(top.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let ue = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(view_count.opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(comments_of(db).opt())).opt())
        .fold([0i64; 4], |a, x| match x {
            Some(((w, t), c)) => [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64, a[3] + w.unwrap_or(0)],
            None => a,
        });
    let users = rel(drain((&ue).and((&pc).filt(|n| n > 0))));
    let recent = || db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let bv = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let map = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(bv.opt())).fold([0i64; 2], |a, (c, b)| [a[0] + c.is_some() as i64, a[1] + b.flatten().unwrap_or(0)]);
    let r = ranked(drain(&map), |&(p, _)| { let w = view_count.get(p); (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p) }, false);
    type R = ((Id<Post>, [i64; 2]), i64);
    let posts = rel(drain(rel(r).filt(|((_, a), k): R| k <= 5 || a[1] > 0)).into_iter().map(|x| x.1).collect::<Vec<R>>());
    let mut v = Vec::new();
    (&users).cross(&posts).drive(|_, ((u, (a, n)), ((p, b), _))| v.push((u, a, n, p, b)));
    rows(v.into_iter().map(|(u, a, n, p, b)| {
        let mut f = ucols(db, u, &["name"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3])]);
        f.extend(post_fields(db, p, &["title", "created", "score"]));
        f.extend([V::I(b[0]), V::I(b[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.AnswerCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY u.Id ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.Score > 0),
// PostVoteSummary AS (SELECT p.Id AS PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId IN (1, 2) GROUP BY p.Id),
// PostCommentAggregates AS (SELECT p.Id AS PostId, COUNT(c.Id) AS CommentCount, STRING_AGG(c.Text, ' | ') AS Comments FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY p.Id),
// ClosedPostDetails AS (SELECT ph.PostId, MAX(ph.CreationDate) AS LastClosedDate, STRING_AGG(pr.Name, ', ') AS ClosureReasons
//     FROM PostHistory ph JOIN CloseReasonTypes pr ON CAST(ph.Comment AS INT) = pr.Id WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, ps.UpVotes, ps.DownVotes, pca.CommentCount, pca.Comments, cp.LastClosedDate, cp.ClosureReasons, rp.OwnerDisplayName
// FROM RankedPosts rp JOIN PostVoteSummary ps ON rp.PostId = ps.PostId LEFT JOIN PostCommentAggregates pca ON rp.PostId = pca.PostId LEFT JOIN ClosedPostDetails cp ON rp.PostId = cp.PostId
// WHERE rp.PostRank = 1 ORDER BY rp.CreationDate DESC LIMIT 100 OFFSET 0;
//
// PostRank and the order read only base columns, so the hundred posts are picked first. PostRank breaks CreationDate ties by post id. The STRING_AGGs have
// no ORDER BY; they are built in comment and history id order.
fn q31484(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, owner_user, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(score.gt(0))).select(owner_user));
    let first = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let top = top_n(first, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 100);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ps = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let pca = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.text).opt()).buf_fold(|v| {
        let t: Vec<&str> = v.iter().flatten().copied().collect();
        (t.len() as i64, if t.is_empty() { None } else { Some(leak(t, " | ")) })
    });
    let PostHistory { post_history_type_id, comment, creation_date: hd, .. } = &db.post_history;
    let crt: HashIdx<i64, Id<CloseReasonType>> = (&db.close_reason_type.origid).inv().collect();
    let cp = db
        .post_history
        .with(post_history_type_id.is_in([10, 11]))
        .group_by(&db.post_history.post)
        .select(hd.and(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&crt).select(&db.close_reason_type.name)))
        .buf_fold(|v| (v.iter().map(|x| x.0).max().unwrap(), leak(v.iter().map(|x| x.1).collect(), ", ")));
    let v = drain((&ps).and(&pca).and((&cp).opt()));
    let v = top_n(v, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 100);
    rows(v.into_iter().map(|(p, ((a, (n, t)), c))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(n), ostr(t)]);
        f.extend(match c {
            Some((d, r)) => [V::T(d), V::S(r)],
            None => [V::Null, V::Null],
        });
        f.extend(post_fields(db, p, &["owner"]));
        row(f)
    }))
}

// WITH UserVoteStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(V.Id) AS TotalVotes, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, COUNT(DISTINCT P.Id) AS TotalPosts
//     FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId LEFT JOIN Posts P ON V.PostId = P.Id GROUP BY U.Id, U.DisplayName),
// PostStatistics AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, COALESCE(COUNT(CM.Id), 0) AS CommentCount, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount
//     FROM Posts P LEFT JOIN Comments CM ON P.Id = CM.PostId LEFT JOIN Votes V ON P.Id = V.PostId WHERE P.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days'
//     GROUP BY P.Id, P.Title, P.CreationDate, P.Score),
// RankedPosts AS (SELECT PS.*, ROW_NUMBER() OVER (ORDER BY PS.Score DESC, PS.CommentCount DESC) AS PostRank FROM PostStatistics PS)
// SELECT UPS.UserId, UPS.DisplayName, PPS.PostId, PPS.Title, PPS.CreationDate, PPS.Score, PPS.CommentCount, PPS.UpVoteCount, PPS.DownVoteCount,
//        CASE WHEN PPS.Score IS NULL THEN 'No Score' ELSE CASE WHEN PPS.Score > 10 THEN 'High Score' WHEN PPS.Score BETWEEN 1 AND 10 THEN 'Moderate Score' ELSE 'Low Score' END END AS ScoreCategory,
//        (SELECT COUNT(*) FROM Votes V WHERE V.PostId = PPS.PostId AND V.VoteTypeId = 4) AS NominationCount
// FROM UserVoteStats UPS JOIN RankedPosts PPS ON UPS.TotalVotes > 5 AND UPS.UserId = PPS.PostId WHERE PPS.PostRank <= 10 ORDER BY UPS.TotalVotes DESC, PPS.Score DESC;
//
// `UPS.UserId = PPS.PostId` joins a user id to a post id, so it goes through the raw ids. PostRank breaks (Score, CommentCount) ties by post id.
fn q3784(db: &'static So) -> String {
    let Post { creation_date, score, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_days(date(2024, 10, 1), -30)));
    let st = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 3], |a, (c, t)| {
        [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let r = top_n(drain(&st), |&(p, a)| (Reverse(score.get(p).unwrap()), Reverse(a[0]), p), 10);
    type R = (Id<Post>, [i64; 3]);
    let rv = rel(r);
    let by_id: HashIdx<i64, R> = (&rv).map(|x: R| db.post.origid.get(x.0).unwrap()).inv().select(&rv).collect();
    let uvs = db.user.group_by(Ident::<User>::new()).select(votes_by(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let nom = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(4)));
    let ncnt = recent().group_by(Ident::<Post>::new()).select(nom.opt()).fold(0i64, |n, x| n + x.is_some() as i64);
    let v = drain((&uvs).filt(|n| n > 5).and((&db.user.origid).select(&by_id).select(Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0).select(&ncnt)))));
    rows(v.into_iter().map(|(u, (_, ((p, a), nc)))| {
        let s = score.get(p).unwrap();
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(post_fields(db, p, &["id", "title", "created", "score"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        f.push(V::S(if s > 10 { "High Score" } else if (1..=10).contains(&s) { "Moderate Score" } else { "Low Score" }));
        f.push(V::I(nc));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// RankedUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserStats),
// PopularPosts AS (SELECT P.Id AS PostId, P.Title, P.ViewCount, P.Score, COALESCE(PH.RevisionGUID, 'N/A') AS LastGUID, RANK() OVER (ORDER BY P.ViewCount DESC) AS PopularityRank
//     FROM Posts P LEFT JOIN PostHistory PH ON P.Id = PH.PostId AND PH.PostHistoryTypeId IN (4, 5, 10) WHERE P.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'),
// MostCommentedPosts AS (SELECT PostId, COUNT(C.Id) AS CommentCount FROM Comments C GROUP BY PostId),
// FilteredPosts AS (SELECT P.Title, P.Body, P.ViewCount, COALESCE(MC.CommentCount, 0) AS CommentCount, U.DisplayName AS OwnerDisplayName,
//        RANK() OVER (ORDER BY P.ViewCount DESC, COALESCE(MC.CommentCount, 0) DESC) AS CombinedRank
//     FROM Posts P LEFT JOIN Users U ON P.OwnerUserId = U.Id LEFT JOIN MostCommentedPosts MC ON P.Id = MC.PostId WHERE P.ClosedDate IS NULL AND P.ViewCount > 100)
// SELECT U.DisplayName, U.Reputation, FP.Title, FP.ViewCount, FP.CommentCount, FP.CombinedRank
// FROM RankedUsers U JOIN FilteredPosts FP ON U.QuestionCount > 5 WHERE U.Rank <= 10 ORDER BY U.Rank, FP.CombinedRank LIMIT 10;
//
// PopularPosts is never read. The ON names only U, so it is a cross join. Rank breaks Reputation ties by user id.
fn q24647(db: &'static So) -> String {
    let Post { view_count, closed_date, post_type_id, .. } = &db.post;
    let top = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10);
    let ru = rel(top.into_iter().enumerate().map(|(i, (u, _))| (u, i as i64 + 1)).collect::<Vec<(Id<User>, i64)>>());
    let qc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id)).fold(0i64, |n, t| n + (t == 1) as i64);
    let ru = drain((&ru).select(Same::<(Id<User>, i64)>::new().and(Same::<(Id<User>, i64)>::new().map(|x: (Id<User>, i64)| x.0).select((&qc).filt(|n| n > 5)))));
    let ru: Vec<(Id<User>, i64)> = ru.into_iter().map(|x| x.1 .0).collect();
    let fp = db.post.minus(closed_date).with(view_count.gt(100)).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let r = ranked(drain(&fp), |&(p, c)| (Reverse(view_count.get(p).unwrap()), Reverse(c)), false);
    let v = cross_top(ru, |&(_, k)| k, r, |&((p, _), k)| (k, p), 10);
    rows(v.into_iter().map(|((u, _), ((p, c), k))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["title", "views"]));
        f.extend([V::I(c), V::I(k)]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS Rank FROM Users U),
// TopPosts AS (SELECT P.Id AS PostId, P.Title, P.Score, P.ViewCount, P.CreationDate, COALESCE(SUM(VB.BountyAmount), 0) AS TotalBounties, COUNT(CA.Id) AS AnswerCount, COUNT(C.Id) AS CommentCount
//     FROM Posts P LEFT JOIN Votes VB ON P.Id = VB.PostId AND VB.VoteTypeId = 8 LEFT JOIN Posts CA ON P.Id = CA.ParentId AND CA.PostTypeId = 2 LEFT JOIN Comments C ON P.Id = C.PostId
//     WHERE P.PostTypeId = 1 GROUP BY P.Id, P.Title, P.Score, P.ViewCount, P.CreationDate),
// RecentEdits AS (SELECT PH.PostId, PH.UserId, PH.CreationDate, PH.Comment FROM PostHistory PH WHERE PH.PostHistoryTypeId IN (4, 5, 6) AND PH.CreationDate >= CURRENT_TIMESTAMP - INTERVAL '30 days'),
// UserScore AS (SELECT UserId, SUM(Score) AS TotalScore FROM (SELECT P.OwnerUserId AS UserId, P.Score FROM Posts P WHERE P.PostTypeId = 1 UNION ALL
//     SELECT A.OwnerUserId AS UserId, A.Score FROM Posts A WHERE A.PostTypeId = 2) AS UserPosts GROUP BY UserId)
// SELECT UR.UserId, UR.DisplayName, UR.Reputation, TP.PostId, TP.Title, TP.Score, TP.TotalBounties, TP.AnswerCount, TP.CommentCount, RE.UserId AS LastEditedBy,
//        RE.CreationDate AS LastEditDate, RE.Comment AS LastEditComment
// FROM UserReputation UR JOIN TopPosts TP ON UR.Rank <= 10 LEFT JOIN RecentEdits RE ON TP.PostId = RE.PostId LEFT JOIN UserScore US ON UR.UserId = US.UserId
// WHERE COALESCE(US.TotalScore, 0) > 1000 ORDER BY UR.Reputation DESC, TP.Score DESC;
//
// The ON names only UR, so the ten users are crossed with the questions. CURRENT_TIMESTAMP is TIMESTAMPTZ, so PostHistory.CreationDate is read as New York local time.
// Rank breaks Reputation ties by user id.
fn q4662(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, .. } = &db.post;
    let top = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(top.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let us = db.post.with(post_type_id.is_in([1, 2])).group_by(owner_user).select(score).fold(0i64, |s, x| s + x);
    let users = rel(drain((&tu).select(Ident::<User>::new().with((&us).filt(|s| s > 1000)))).into_iter().map(|x| x.0).collect::<Vec<Id<User>>>());
    let bv = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let tp = db
        .post
        .with(post_type_id.eq(1))
        .group_by(Ident::<Post>::new())
        .select(bv.opt().and(answers_of(db).opt()).and(comments_of(db).opt()))
        .fold([0i64; 3], |a, ((b, x), c)| [a[0] + b.flatten().unwrap_or(0), a[1] + x.is_some() as i64, a[2] + c.is_some() as i64]);
    let since = now_utc() - 30 * DAY_US;
    let PostHistory { post_history_type_id, creation_date: hd, user_id, comment, .. } = &db.post_history;
    let re = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.is_in([4, 5, 6])).with(hd.filt(move |d| ny_to_utc(d) >= since)));
    let posts = rel(drain((&tp).and(re.opt())));
    let mut v = Vec::new();
    (&users).cross(&posts).drive(|_, (u, (p, (a, e)))| v.push((u, p, a, e)));
    rows(v.into_iter().map(|(u, p, a, e)| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(post_fields(db, p, &["id", "title", "score"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        f.extend(match e {
            Some(h) => [oint(user_id.get(h)), V::T(hd.get(h).unwrap()), ostr(comment.get(h))],
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("6333", q6333),
    ("23466", q23466),
    ("6879", q6879),
    ("21947", q21947),
    ("3656", q3656),
    ("31484", q31484),
    ("3784", q3784),
    ("24647", q24647),
    ("4662", q4662),
];
