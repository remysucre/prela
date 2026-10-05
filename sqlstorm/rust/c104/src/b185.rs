use harness::prelude::*;
use std::cmp::Reverse;

fn by_first<A: Copy + Eq + std::hash::Hash, B: Copy + Eq + std::hash::Hash>(m: &MatSet<(A, B)>) -> HashIdx<A, B> {
    m.map(|(a, _)| a).inv().map(|(_, b): (A, B)| b).collect()
}

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
    let w = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).group_by(post_type_id).select(Ident::<Post>::new().and(score)).window(rank, |(_, s)| Reverse(s), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
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
    let w = db.post.group_by(post_type_id).select(Ident::<Post>::new().and(view_count.opt()).and(ph.opt())).window(rank, |((_, w), _)| (w.is_none(), Reverse(w)), asc);
    type R = (Id<Post>, Option<i64>, Option<Id<PostHistory>>, i64);
    let rv: MatSet<R> = (&w).map(|(((p, w), h), k)| (p, w, h, k)).filt(|x: R| x.1.map_or(false, |w| w > 100)).collect();
    let by_id: HashIdx<i64, R> = (&rv).map(|x: R| x.0).select(&db.post.origid).inv().collect();
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
    let w = whole(pa()).select(Ident::<Post>::new().and(score)).window(rank, |(_, s)| Reverse(s), asc);
    type R = (Id<Post>, i64);
    let rv: MatSet<R> = (&w).map(|((p, _), k)| (p, k)).collect();
    let by_id: HashIdx<i64, R> = (&rv).map(|x: R| x.0).select(&db.post.origid).inv().collect();
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
        .and(creation_date)
        .and(owner_user.select(badges_of(db)).select(Ident::<Badge>::new().and(&db.badge.date)))
        .filt(|((_, cd), (_, bd)): ((Id<Post>, i64), (Id<Badge>, i64))| bd <= cd)
        .map(|(_, (b, _)): ((Id<Post>, i64), (Id<Badge>, i64))| b);
    let pd = recent()
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(history_of(db).select(post_history_type_id.and(hd)).opt()).and(early.opt()))
        .fold((i64::MIN, 0i64), |(m, n), ((_, h), b)| (h.map_or(m, |(t, d)| if t == 10 { m.max(d) } else { m }), n + b.is_some() as i64));
    let cc = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let w = recent().group_by(owner_user).select(Ident::<Post>::new().and(owner_user).and(score)).window(rank, |(_, s)| Reverse(s), asc);
    type R = (Id<Post>, Id<User>, i64);
    let rv: MatSet<R> = (&w).map(|(((p, u), _), k)| (p, u, k)).collect();
    let uvc = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + matches!(t, 2 | 8) as i64, a[2] + (t == 3) as i64],
        None => a,
    });
    type X = (R, ((((i64, i64), i64), [i64; 3]), i64));
    let v = drain(
        (&rv)
            .select(Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0).select((&pd).and(&cc)).and(Same::<R>::new().map(|x: R| x.1).select(&uvc)).and(Same::<R>::new().map(|x: R| x.0).select(creation_date))))
            .filt(|((_, _, k), ((((m, b), c), a), cd)): X| ((k == 1 && a[1] > a[2]) || (c > 10 && b >= 1)) && m != i64::MIN && m.max(cd) > cd)
            .map(|(r, (x, _)): X| (r, x)),
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
    let w = whole(recent()).select(Ident::<Post>::new().and(score).and(view_count.opt()).and(&map)).window(row_number, |(((p, s), w), _)| (Reverse(s), w.is_none(), Reverse(w), p), asc);
    let posts: MatSet<(Id<Post>, [i64; 2])> = (&w).filt(|((_, a), k)| k <= 5 || a[1] > 0).map(|((((p, _), _), b), _)| (p, b)).collect();
    let mut v = Vec::new();
    (&users).cross(&posts).drive(|_, ((u, (a, n)), (p, b))| v.push((u, a, n, p, b)));
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
    let w = db.post.with(post_type_id.eq(1).and(score.gt(0))).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let first = drain((&w).filt(|(_, r)| r == 1).map(|((p, d), _)| (p, d)));
    let top = top_n(first, |&(_, (p, d))| (Reverse(d), p), 100);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.1 .0).collect()).map(|p| p).collect();
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
    let by_id: HashIdx<i64, R> = (&rv).map(|x: R| x.0).select(&db.post.origid).inv().select(&rv).collect();
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
    let w = whole(db.post.with(&fp)).select(Ident::<Post>::new().and(view_count).and(&fp)).window(rank, |((_, w), c)| (Reverse(w), Reverse(c)), asc);
    let v = drain(rel(ru).cross(&w));
    let v = top_n(v, |&(_, ((_, k), (((p, _), _), r)))| (k, r, p), 10);
    rows(v.into_iter().map(|(_, ((u, _), (((p, _), c), k)))| {
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

// WITH RecursivePostStats AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.CreationDate, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT v.Id) FILTER (WHERE v.VoteTypeId = 2) AS UpVoteCount,
//        COUNT(DISTINCT v.Id) FILTER (WHERE v.VoteTypeId = 3) AS DownVoteCount, MAX(b.Date) AS LastBadgeDate
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON p.OwnerUserId = b.UserId WHERE p.PostTypeId IN (1, 2)
//     GROUP BY p.Id, p.Title, p.OwnerUserId, p.CreationDate),
// RankedPosts AS (SELECT ps.PostId, ps.Title, ps.OwnerUserId, ps.CreationDate, ps.CommentCount, ps.UpVoteCount, ps.DownVoteCount, ps.LastBadgeDate,
//        RANK() OVER (PARTITION BY ps.OwnerUserId ORDER BY ps.CommentCount DESC, ps.UpVoteCount DESC) AS PostRank FROM RecursivePostStats ps),
// PostLinksSummary AS (SELECT pl.PostId, COUNT(DISTINCT pl.RelatedPostId) AS RelatedPostCount FROM PostLinks pl GROUP BY pl.PostId)
// SELECT rp.PostId, rp.Title, u.DisplayName AS OwnerDisplayName, rp.CommentCount, rp.UpVoteCount, rp.DownVoteCount, COALESCE(pl.RelatedPostCount, 0) AS RelatedPostCount, rp.LastBadgeDate,
//        CASE WHEN rp.LastBadgeDate IS NOT NULL AND rp.LastBadgeDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' THEN 'Active' ELSE 'InActive' END AS UserActivityStatus
// FROM RankedPosts rp JOIN Users u ON rp.OwnerUserId = u.Id LEFT JOIN PostLinksSummary pl ON rp.PostId = pl.PostId
// WHERE rp.PostRank = 1 AND (rp.CommentCount > 5 OR rp.UpVoteCount > 10) ORDER BY UserActivityStatus DESC, rp.PostId;
//
// Every aggregate is DISTINCT or a MAX, so each child is folded on its own: the comment and vote counts per post, the latest badge per owner.
// The ownerless partition never joins a user, so only owned posts are ranked.
fn q31171(db: &'static So) -> String {
    let Post { post_type_id, owner_user, .. } = &db.post;
    let ps = || db.post.with(post_type_id.is_in([1, 2]));
    let vt = ps().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = ps().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let bm = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.date)).fold(i64::MIN, |m, d| m.max(d));
    let pl = db.post_link.group_by(&db.post_link.post).select(&db.post_link.related_post_id).count_distinct();
    let w = ps().group_by(owner_user).select(Ident::<Post>::new().and((&vt).and(&cc).and(owner_user))).window(rank, |(_, ((a, c), _))| (Reverse(c), Reverse(a[0])), asc);
    type R = ((Id<Post>, (([i64; 2], i64), Id<User>)), i64);
    let rv: MatSet<R> = (&w).filt(|((_, ((a, c), _)), k)| k == 1 && (c > 5 || a[0] > 10)).collect();
    let v = drain(
        (&rv)
            .select(Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0 .0).select(&pl).opt()).and(Same::<R>::new().map(|x: R| x.0 .1 .1).select(&bm).opt())),
    );
    let since = add_years(ts(2024, 10, 1, 12, 34, 56), -1);
    rows(v.into_iter().map(|(_, ((((p, ((a, c), u)), _), l), b))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([user_col(db, u, "name"), V::I(c), V::I(a[0]), V::I(a[1]), V::I(l.unwrap_or(0)), ots(b)]);
        f.push(V::S(if b.map_or(false, |b| b > since) { "Active" } else { "InActive" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.PostTypeId = 1),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(v.BountyAmount) AS TotalBounty, COUNT(DISTINCT p.Id) AS QuestionCount
//     FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId AND p.PostTypeId = 1 LEFT JOIN Votes v ON v.UserId = u.Id AND v.PostId IN (SELECT PostId FROM RankedPosts)
//     GROUP BY u.Id, u.DisplayName HAVING COUNT(DISTINCT p.Id) > 5),
// UserBadges AS (SELECT ub.UserId, COUNT(CASE WHEN ub.Class = 1 THEN 1 END) AS GoldCount, COUNT(CASE WHEN ub.Class = 2 THEN 1 END) AS SilverCount, COUNT(CASE WHEN ub.Class = 3 THEN 1 END) AS BronzeCount
//     FROM Badges ub GROUP BY ub.UserId),
// CombinedData AS (SELECT tu.UserId, tu.DisplayName, tu.TotalBounty, ub.GoldCount, ub.SilverCount, ub.BronzeCount, COALESCE(rp.PostId, 0) AS LatestPostId, COALESCE(rp.Title, 'No posts') AS LatestPostTitle
//     FROM TopUsers tu LEFT JOIN UserBadges ub ON tu.UserId = ub.UserId LEFT JOIN RankedPosts rp ON tu.UserId = rp.PostId)
// SELECT c.UserId, c.DisplayName, c.TotalBounty, c.GoldCount, c.SilverCount, c.BronzeCount, CASE WHEN c.TotalBounty IS NOT NULL THEN 'Has Bounty' ELSE 'No Bounty' END AS Bounty_Status,
//        c.LatestPostTitle, c.LatestPostId FROM CombinedData c ORDER BY c.TotalBounty DESC NULLS LAST, c.DisplayName ASC;
//
// rn is never read. `tu.UserId = rp.PostId` joins a user id to a post id, so it goes through the raw ids.
fn q30128(db: &'static So) -> String {
    let Post { creation_date, post_type_id, .. } = &db.post;
    let recent = || Ident::<Post>::new().with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.eq(1)));
    let vsel = votes_by(db).select(Ident::<Vote>::new().with((&db.vote.post).select(recent()))).select((&db.vote.bounty_amount).opt());
    let qs = || posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1)));
    let tb = db.user.group_by(Ident::<User>::new()).select(qs().and(vsel.opt())).fold((0i64, false), |(s, any), (_, b)| {
        let b = b.flatten();
        (s + b.unwrap_or(0), any || b.is_some())
    });
    let qc = db.user.group_by(Ident::<User>::new()).select(qs()).fold(0i64, |n, _| n + 1);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class)).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let ridx: HashIdx<i64, Id<Post>> = db.post.select(recent()).select(&db.post.origid).inv().collect();
    let v = drain((&tb).and((&qc).filt(|n| n > 5)).and((&ub).opt()).and((&db.user.origid).select(&ridx).opt()));
    rows(v.into_iter().map(|(u, (((b, _), g), p))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.push(if b.1 { V::I(b.0) } else { V::Null });
        f.extend(match g {
            Some(g) => g.map(V::I),
            None => [V::Null, V::Null, V::Null],
        });
        f.push(V::S(if b.1 { "Has Bounty" } else { "No Bounty" }));
        f.push(V::S(p.and_then(|p| db.post.title.get(p)).unwrap_or("No posts")));
        f.push(V::I(p.map_or(0, |p| db.post.origid.get(p).unwrap())));
        row(f)
    }))
}

// WITH RECURSIVE PostActivity AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.PostTypeId, p.Score, p.OwnerUserId, p.AnswerCount, p.CommentCount, p.ViewCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS ActivityRank FROM Posts p WHERE p.CreationDate >= DATE('2024-10-01') - INTERVAL '1 year'
//     UNION ALL SELECT pa.PostId, pa.Title, pa.CreationDate, pa.PostTypeId, pa.Score, pa.OwnerUserId, pa.AnswerCount, pa.CommentCount, pa.ViewCount,
//        ROW_NUMBER() OVER (PARTITION BY pa.OwnerUserId ORDER BY pa.CreationDate DESC) AS ActivityRank FROM PostActivity pa JOIN Votes v ON v.PostId = pa.PostId
//     WHERE v.CreationDate >= DATE('2024-10-01') - INTERVAL '6 months'),
// UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(COALESCE(p.Score, 0)) AS TotalScore, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews,
//        SUM(COALESCE(p.AnswerCount, 0)) AS TotalAnswers, SUM(COALESCE(p.CommentCount, 0)) AS TotalComments, MAX(p.CreationDate) AS LastPostDate
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName)
// SELECT ups.UserId, ups.DisplayName, ups.TotalPosts, ups.TotalScore, ups.TotalViews, ups.TotalAnswers, ups.TotalComments, ups.LastPostDate, ht.Name AS HighScoresType
// FROM UserPostStats ups LEFT JOIN PostHistory ph ON ups.UserId = ph.UserId INNER JOIN PostHistoryTypes ht ON ph.PostHistoryTypeId = ht.Id
// WHERE ups.TotalScore > 100 AND ups.LastPostDate >= DATE('2024-10-01') - INTERVAL '6 months' ORDER BY ups.TotalScore DESC;
//
// The recursive PostActivity is never read.
fn q31091(db: &'static So) -> String {
    let Post { score, view_count, answer_count, comment_count, creation_date, .. } = &db.post;
    let ups = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score.and(view_count.opt()).and(answer_count.opt()).and(comment_count).and(creation_date)))
        .fold([0i64, 0, 0, 0, 0, i64::MIN], |a, ((((s, w), n), c), d)| [a[0] + 1, a[1] + s, a[2] + w.unwrap_or(0), a[3] + n.unwrap_or(0), a[4] + c, a[5].max(d)]);
    let hidx: HashIdx<Id<User>, Id<PostHistory>> = (&db.post_history.user).inv().collect();
    let since = add_months(date(2024, 10, 1), -6);
    let v = drain((&ups).filt(move |a| a[1] > 100 && a[5] >= since).and((&hidx).select(htype_name(db))));
    rows(v.into_iter().map(|(u, (a, t))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4]), V::T(a[5]), V::S(t)]);
        row(f)
    }))
}

// WITH UserEngagement AS (SELECT U.Id AS UserId, U.DisplayName, COALESCE(SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotesCount,
//        COALESCE(SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotesCount, COUNT(DISTINCT P.Id) AS AnswerCount,
//        AVG(EXTRACT(EPOCH FROM (P.LastActivityDate - P.CreationDate)) / 3600.0) AS AvgResponseTime
//     FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId LEFT JOIN Posts P ON U.Id = P.OwnerUserId AND P.PostTypeId = 2 GROUP BY U.Id, U.DisplayName),
// PostStatistics AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.AcceptedAnswerId, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
//        COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVoteCount, COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVoteCount, ROW_NUMBER() OVER (ORDER BY P.CreationDate DESC) AS PostRank
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY P.Id, P.Title, P.CreationDate, P.AcceptedAnswerId),
// TopPosts AS (SELECT PS.*, U.DisplayName AS TopUser, U.Reputation AS UserReputation, COALESCE(UE.UpVotesCount, 0) - COALESCE(UE.DownVotesCount, 0) AS UserScore
//     FROM PostStatistics PS LEFT JOIN Users U ON PS.AcceptedAnswerId = U.Id LEFT JOIN UserEngagement UE ON U.Id = UE.UserId WHERE PS.PostRank <= 10)
// SELECT TP.PostId, TP.Title, TP.CommentCount, TP.UpVoteCount, TP.DownVoteCount, TP.TopUser, TP.UserReputation, TP.UserScore,
//        CASE WHEN TP.UserScore > 5 THEN 'Highly Engaged' WHEN TP.UserScore BETWEEN 0 AND 5 THEN 'Moderately Engaged' ELSE 'Needs Improvement' END AS EngagementLevel
// FROM TopPosts TP ORDER BY TP.UserScore DESC, TP.CommentCount DESC;
//
// PostRank reads only CreationDate, so the ten newest posts are picked first. `PS.AcceptedAnswerId = U.Id` joins a post id to a user id, so it goes
// through the raw ids, and the votes x answers product is driven for those users alone.
fn q4709(db: &'static So) -> String {
    let Post { creation_date, accepted_answer_id, post_type_id, .. } = &db.post;
    let top = top_n(drain(creation_date), |&(p, d)| (Reverse(d), p), 10);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ps = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 3], |a, (c, t)| {
        [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let mu: MatSet<Id<User>> = (&tp).select(accepted_answer_id.select(&uidx)).collect();
    let ue = (&mu)
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt().and(posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(2))).opt()))
        .fold([0i64; 2], |a, (t, _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = drain((&ps).and(accepted_answer_id.select(&uidx).select(Ident::<User>::new().and(&ue)).opt()));
    rows(v.into_iter().map(|(p, (a, u))| {
        let s = u.map_or(0, |(_, e)| e[0] - e[1]);
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        f.extend(match u {
            Some((u, _)) => ucols(db, u, &["name", "rep"]),
            None => vec![V::Null, V::Null],
        });
        f.push(V::I(s));
        f.push(V::S(if s > 5 { "Highly Engaged" } else if s >= 0 { "Moderately Engaged" } else { "Needs Improvement" }));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, COALESCE(SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END), 0) AS QuestionCount,
//        COALESCE(SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS AnswerCount, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount, COALESCE(SUM(CASE WHEN B.Id IS NOT NULL THEN 1 ELSE 0 END), 0) AS BadgeCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopContributors AS (SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, UpVoteCount, DownVoteCount, BadgeCount, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserStats),
// ActivePosts AS (SELECT P.Id AS PostId, P.Title, P.OwnerUserId, P.CreationDate, P.LastActivityDate, P.ViewCount, COALESCE(P.AnswerCount, 0) AS AnswerCount, COALESCE(P.CommentCount, 0) AS CommentCount
//     FROM Posts P WHERE P.LastActivityDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days'))
// SELECT TC.DisplayName, TC.Reputation, TC.PostCount, TC.QuestionCount, TC.AnswerCount, TC.UpVoteCount, TC.DownVoteCount, TC.BadgeCount, AP.PostId, AP.Title, AP.CreationDate,
//        AP.LastActivityDate, AP.ViewCount, AP.AnswerCount, AP.CommentCount
// FROM TopContributors TC JOIN ActivePosts AP ON TC.UserId = AP.OwnerUserId WHERE TC.Rank <= 10 ORDER BY TC.Reputation DESC, AP.LastActivityDate DESC;
//
// Rank reads only Reputation, so the ten users are picked first (ties broken by user id) and the posts x votes x badges product is driven for them alone.
fn q5116(db: &'static So) -> String {
    let Post { post_type_id, last_activity_date, answer_count, .. } = &db.post;
    let top = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(top.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let us = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 5], |a, (p, b)| {
            let (t, v) = p.map_or((None, None), |(t, v)| (Some(t), v));
            [a[0] + (t == Some(1)) as i64, a[1] + (t == Some(2)) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64, a[4] + b.is_some() as i64]
        });
    let pc = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let ap = posts_of(db).select(Ident::<Post>::new().with(last_activity_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))));
    let v = drain((&us).and(&pc).and(ap));
    rows(v.into_iter().map(|(u, ((a, n), p))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.extend(post_fields(db, p, &["id", "title", "created", "activity", "views"]));
        f.extend([V::I(answer_count.get(p).unwrap_or(0)), V::I(db.post.comment_count.get(p).unwrap())]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.PostTypeId, p.Score, p.ViewCount, p.OwnerUserId,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank FROM Posts p WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'),
// UserActivity AS (SELECT u.Id AS UserId, COALESCE(SUM(v.BountyAmount), 0) AS TotalBountyAmount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpvotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownvotes, COUNT(DISTINCT c.Id) AS CommentCount
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Comments c ON u.Id = c.UserId GROUP BY u.Id),
// PostMetrics AS (SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, ua.TotalUpvotes, ua.TotalDownvotes, ua.CommentCount FROM RankedPosts rp LEFT JOIN UserActivity ua ON rp.OwnerUserId = ua.UserId
//     WHERE rp.Rank <= 5)
// SELECT pm.PostId, pm.Title, pm.Score, pm.ViewCount, COALESCE(pm.TotalUpvotes, 0) AS TotalUpvotes, COALESCE(pm.TotalDownvotes, 0) AS TotalDownvotes, COALESCE(pm.CommentCount, 0) AS CommentCount,
//        CASE WHEN pm.Score IS NULL THEN 'No Score' WHEN pm.Score >= 100 THEN 'Highly Rated' ELSE 'Moderately Rated' END AS RatingClassification, SUBSTRING(pm.Title, 1, 50) || '...' AS ShortenedTitle,
//        CASE WHEN pm.ViewCount IS NULL THEN 'Unknown Views' WHEN pm.ViewCount < 100 THEN 'Low Traffic' ELSE 'High Traffic' END AS TrafficStatus
// FROM PostMetrics pm ORDER BY pm.Score DESC, pm.ViewCount DESC;
//
// Rank reads only base columns, so the posts are picked first (ties broken by post id) and the votes x comments product is driven for their owners alone.
fn q24522(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, owner_user, view_count, title, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1))).group_by(post_type_id).select(Ident::<Post>::new().and(score).and(creation_date)).window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|(((p, _), _), _)| p).collect();
    let owners: MatSet<Id<User>> = (&tp).select(owner_user).collect();
    let ua = (&owners)
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt().and(comments_by(db).opt()))
        .fold([0i64; 2], |a, (t, _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = (&owners).group_by(Ident::<User>::new()).select(comments_by(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&tp).select(Ident::<Post>::new().and(owner_user.select((&ua).and(&cc)).opt())));
    rows(v.into_iter().map(|(_, (p, u))| {
        let (a, c) = u.unwrap_or(([0, 0], 0));
        let s = score.get(p).unwrap();
        let w = view_count.get(p);
        let mut f = post_fields(db, p, &["id", "title", "score", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c)]);
        f.push(V::S(if s >= 100 { "Highly Rated" } else { "Moderately Rated" }));
        f.push(match title.get(p) {
            Some(t) => V::Owned(format!("{}...", t.chars().take(50).collect::<String>())),
            None => V::Null,
        });
        f.push(V::S(match w {
            None => "Unknown Views",
            Some(w) if w < 100 => "Low Traffic",
            _ => "High Traffic",
        }));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownVotes, COUNT(DISTINCT p.Id) AS PostCount,
//        DENSE_RANK() OVER (ORDER BY COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) - COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) DESC) AS RankScore
//     FROM Users u LEFT JOIN Posts p ON p.OwnerUserId = u.Id LEFT JOIN Votes v ON v.PostId = p.Id WHERE u.Reputation > 100 GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalUpVotes, TotalDownVotes, PostCount, RankScore FROM UserActivity WHERE PostCount > 5),
// RecentVotes AS (SELECT v.UserId, COUNT(*) AS VoteCount FROM Votes v WHERE v.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days' GROUP BY v.UserId),
// UserVotePerformance AS (SELECT u.UserId, u.DisplayName, COALESCE(ru.VoteCount, 0) AS RecentVoteCount, u.TotalUpVotes - u.TotalDownVotes AS NetVotes, u.RankScore
//     FROM TopUsers u LEFT JOIN RecentVotes ru ON ru.UserId = u.UserId)
// SELECT u.DisplayName, u.RecentVoteCount, u.NetVotes, RANK() OVER (ORDER BY u.NetVotes DESC, u.RecentVoteCount DESC) AS VoteRanking,
//        CASE WHEN u.NetVotes > 0 THEN 'Positive Contributor' WHEN u.NetVotes < 0 THEN 'Negative Contributor' ELSE 'Neutral Contributor' END AS ContributionType
// FROM UserVotePerformance u WHERE u.NetVotes IS NOT NULL ORDER BY u.NetVotes DESC, u.RecentVoteCount DESC LIMIT 10;
//
// RankScore is never read.
fn q24013(db: &'static So) -> String {
    let users = || db.user.with((&db.user.reputation).gt(100));
    let ua = users()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt())
        .fold([0i64; 2], |a, t| [a[0] + (t.flatten() == Some(2)) as i64, a[1] + (t.flatten() == Some(3)) as i64]);
    let pc = users().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let rv = db.vote.with((&db.vote.creation_date).ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).group_by(&db.vote.user).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let uvp: HashIdx<Id<User>, (([i64; 2], i64), Option<i64>)> = (&ua).and((&pc).filt(|n| n > 5)).and((&rv).opt()).collect();
    let key = |a: [i64; 2], r: Option<i64>| (Reverse(a[0] - a[1]), Reverse(r.unwrap_or(0)));
    let w = whole(db.user.with(&uvp)).select(Ident::<User>::new().and(&uvp)).window(rank, move |(_, ((a, _), r))| key(a, r), asc);
    let r = top_n(drain(&w).into_iter().map(|x| x.1).collect(), |&((u, ((a, _), r)), _)| (key(a, r), u), 10);
    rows(r.into_iter().map(|((u, ((a, _), r)), k)| {
        let n = a[0] - a[1];
        let mut f = ucols(db, u, &["name"]);
        f.extend([V::I(r.unwrap_or(0)), V::I(n), V::I(k)]);
        f.push(V::S(if n > 0 { "Positive Contributor" } else if n < 0 { "Negative Contributor" } else { "Neutral Contributor" }));
        row(f)
    }))
}

// WITH RECURSIVE PopularQuestions AS (SELECT p.Id AS QuestionId, p.Title, p.Score, p.CreationDate, p.OwnerUserId, ROW_NUMBER() OVER (ORDER BY p.Score DESC) AS Rank
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.AcceptedAnswerId IS NOT NULL),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews, SUM(COALESCE(p.AnswerCount, 0)) AS TotalAnswers, AVG(p.Score) AS AverageScore
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName),
// RecentActivity AS (SELECT DISTINCT p.OwnerUserId, COUNT(c.Id) AS TotalComments, COUNT(v.Id) AS TotalVotes FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' GROUP BY p.OwnerUserId)
// SELECT q.Title, q.Score, u.DisplayName, COALESCE(us.BadgeCount, 0) AS BadgeCount, COALESCE(us.TotalViews, 0) AS TotalViews, COALESCE(us.TotalAnswers, 0) AS TotalAnswers,
//        COALESCE(ra.TotalComments, 0) AS TotalComments, COALESCE(ra.TotalVotes, 0) AS TotalVotes,
//        CASE WHEN us.AverageScore > 10 THEN 'High Engager' WHEN us.AverageScore BETWEEN 5 AND 10 THEN 'Moderate Engager' ELSE 'Low Engager' END AS EngagementLevel
// FROM PopularQuestions q JOIN Users u ON q.OwnerUserId = u.Id LEFT JOIN UserStats us ON u.Id = us.UserId LEFT JOIN RecentActivity ra ON u.Id = ra.OwnerUserId
// WHERE q.Rank <= 10 ORDER BY q.Score DESC;
//
// No CTE refers to itself. Rank reads only Score, so the ten questions are picked first (ties broken by post id) and the badges x posts product is driven
// for their owners alone.
fn q30187(db: &'static So) -> String {
    let Post { post_type_id, accepted_answer_id, score, owner_user, view_count, answer_count, creation_date, .. } = &db.post;
    let top = top_n(drain(db.post.with(post_type_id.eq(1)).with(accepted_answer_id).select(score)), |&(p, s)| (Reverse(s), p), 10);
    let tq: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let owners: MatSet<Id<User>> = (&tq).select(owner_user).collect();
    let us = (&owners)
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(posts_of(db).select(score.and(view_count.opt()).and(answer_count.opt())).opt()))
        .fold([0i64; 5], |a, (b, p)| match p {
            Some(((s, w), n)) => [a[0] + b.is_some() as i64, a[1] + w.unwrap_or(0), a[2] + n.unwrap_or(0), a[3] + s, a[4] + 1],
            None => [a[0] + b.is_some() as i64, a[1], a[2], a[3], a[4]],
        });
    let ra = (&owners)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(Ident::<Post>::new().with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))).select(comments_of(db).opt().and(votes_of(db).opt())))
        .fold([0i64; 2], |a, (c, v)| [a[0] + c.is_some() as i64, a[1] + v.is_some() as i64]);
    let v = drain((&tq).select(owner_user.select(Ident::<User>::new().and((&us).opt()).and((&ra).opt()))));
    rows(v.into_iter().map(|(q, ((u, s), r))| {
        let s = s.unwrap_or([0; 5]);
        let r = r.unwrap_or([0; 2]);
        let mut f = post_fields(db, q, &["title", "score"]);
        f.extend([user_col(db, u, "name"), V::I(s[0]), V::I(s[1]), V::I(s[2]), V::I(r[0]), V::I(r[1])]);
        let m = if s[4] == 0 { None } else { Some(s[3] as f64 / s[4] as f64) };
        f.push(V::S(match m {
            Some(m) if m > 10.0 => "High Engager",
            Some(m) if (5.0..=10.0).contains(&m) => "Moderate Engager",
            _ => "Low Engager",
        }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate >= (DATE '2024-10-01' - INTERVAL '30 days')),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount FROM RankedPosts rp WHERE rp.Rank <= 5),
// UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, COUNT(DISTINCT c.Id) AS TotalComments, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounties
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 8 GROUP BY u.Id, u.DisplayName),
// EnhancedPostDetails AS (SELECT tp.PostId, tp.Title, tp.Score, tp.ViewCount, ups.TotalPosts, ups.TotalComments, ups.TotalBounties,
//        CASE WHEN ups.TotalPosts IS NULL THEN 'No User' ELSE ups.DisplayName END AS UserDisplayName FROM TopPosts tp LEFT JOIN UserPostStats ups ON tp.PostId = ups.UserId)
// SELECT epd.PostId, epd.Title, epd.Score, epd.ViewCount, epd.TotalPosts, epd.TotalComments, epd.TotalBounties, epd.UserDisplayName FROM EnhancedPostDetails epd
// WHERE epd.Score IS NOT NULL AND epd.ViewCount > (SELECT AVG(ViewCount) FROM Posts)
//   AND (SELECT COUNT(*) FROM Comments c WHERE c.PostId = epd.PostId) > (SELECT AVG((SELECT COUNT(*) FROM Comments WHERE PostId = p.Id)) FROM Posts p WHERE p.PostTypeId = 1)
// ORDER BY epd.Score DESC, epd.ViewCount DESC;
//
// Rank breaks Score ties by post id. `tp.PostId = ups.UserId` joins a post id to a user id, so it goes through the raw ids, and the posts x comments x votes
// product is driven for those users alone. The two averages are separate queries.
fn q24223(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, view_count, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_days(date(2024, 10, 1), -30))).group_by(post_type_id).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let (ws, wn) = view_count.fold_flat((0i64, 0i64), |(s, n), w| (s + w, n + 1));
    let wavg = ws as f64 / wn as f64;
    let qc = db.post.with(post_type_id.eq(1)).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let (cs, cn) = (&qc).fold_flat((0i64, 0i64), |(s, n), c| (s + c, n + 1));
    let cavg = cs as f64 / cn as f64;
    let cc = (&tp).with(view_count.filt(move |w| w as f64 > wavg)).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let keep = (&cc).filt(move |c| c as f64 > cavg);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let mu: MatSet<Id<User>> = (&tp).select((&db.post.origid).select(&uidx)).collect();
    let bv = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let ups = (&mu).group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db).opt().and(bv.opt())).opt()).fold(0i64, |s, x| {
        s + x.and_then(|(_, b)| b.flatten()).unwrap_or(0)
    });
    let upc = (&mu).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let ucc = (&mu).group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain(keep.and((&db.post.origid).select(&uidx).select(Ident::<User>::new().and(&ups).and(&upc).and(&ucc)).opt()));
    rows(v.into_iter().map(|(p, (_, u))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views"]);
        f.extend(match u {
            Some((((u, b), n), c)) => vec![V::I(n), V::I(c), V::I(b), user_col(db, u, "name")],
            None => vec![V::Null, V::Null, V::Null, V::S("No User")],
        });
        row(f)
    }))
}

// WITH UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.CreationDate, DENSE_RANK() OVER (ORDER BY u.Reputation DESC) AS ReputationRank FROM Users u WHERE u.Reputation > 0),
// PostStats AS (SELECT p.OwnerUserId, COUNT(DISTINCT p.Id) AS PostCount, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews, AVG(p.Score) AS AverageScore FROM Posts p
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.OwnerUserId),
// UserPostDetails AS (SELECT ur.UserId, ur.DisplayName, ur.Reputation, ps.PostCount, ps.TotalViews, ps.AverageScore, ROW_NUMBER() OVER (PARTITION BY ur.ReputationRank ORDER BY ur.Reputation DESC) AS RankWithinTier
//     FROM UserReputation ur LEFT JOIN PostStats ps ON ur.UserId = ps.OwnerUserId),
// ClosedPosts AS (SELECT p.OwnerUserId, COUNT(ph.Id) AS ClosedCount FROM Posts p JOIN PostHistory ph ON p.Id = ph.PostId WHERE ph.PostHistoryTypeId = 10
//     AND ph.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.OwnerUserId)
// SELECT upd.DisplayName, upd.Reputation, COALESCE(upd.PostCount, 0) AS PostCount, COALESCE(upd.TotalViews, 0) AS TotalViews, COALESCE(upd.AverageScore, 0) AS AverageScore,
//        COALESCE(cp.ClosedCount, 0) AS ClosedPostCount, CASE WHEN upd.RankWithinTier IS NULL THEN 'Unranked' ELSE CAST(upd.RankWithinTier AS VARCHAR) END AS RankInTier
// FROM UserPostDetails upd LEFT JOIN ClosedPosts cp ON upd.UserId = cp.OwnerUserId WHERE (upd.Reputation > 5000 OR cp.ClosedCount > 0)
// ORDER BY upd.Reputation DESC, upd.PostCount DESC LIMIT 100;
//
// A tier is one Reputation value, so PARTITION BY ReputationRank is PARTITION BY Reputation, and RankWithinTier numbers the users tied on it; the port
// breaks that tie by user id (the SQL leaves it open).
fn q4147(db: &'static So) -> String {
    let Post { creation_date, owner_user, view_count, score, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let ur = db.user.with((&db.user.reputation).gt(0));
    let w = ur.group_by(&db.user.reputation).select(Ident::<User>::new().and(&db.user.reputation)).window(row_number, |(u, _)| u, asc);
    let tr: MatSet<(Id<User>, i64, i64)> = (&w).map(|((u, r), k)| (u, r, k)).collect();
    let ps = db.post.with(creation_date.ge(add_years(t0, -1))).group_by(owner_user).select(view_count.opt().and(score)).fold([0i64; 3], |a, (w, s)| [a[0] + 1, a[1] + w.unwrap_or(0), a[2] + s]);
    let PostHistory { post_history_type_id, creation_date: hd, post, .. } = &db.post_history;
    let cp = db.post_history.with(post_history_type_id.eq(10)).with(hd.ge(add_years(t0, -1))).group_by(post.select(owner_user)).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    type T = (Id<User>, i64, i64);
    let v = drain(
        (&tr)
            .select(Same::<T>::new().and(Same::<T>::new().map(|x: T| x.0).select(&ps).opt()).and(Same::<T>::new().map(|x: T| x.0).select(&cp).opt()))
            .filt(|(((_, r, _), _), c): ((T, Option<[i64; 3]>), Option<i64>)| r > 5000 || c.map_or(false, |c| c > 0)),
    );
    let v = top_n(v, |&(_, (((u, r, _), p), _))| (Reverse(r), Reverse(p.map_or(0, |p| p[0])), u), 100);
    rows(v.into_iter().map(|(_, (((u, _, k), p), c))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(match p {
            Some(a) => [V::I(a[0]), V::I(a[1]), V::F(a[2] as f64 / a[0] as f64)],
            None => [V::I(0), V::I(0), V::F(0.0)],
        });
        f.extend([V::I(c.unwrap_or(0)), V::Owned(k.to_string())]);
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT U.Id AS UserId, U.Reputation, U.CreationDate, COALESCE(SUM(V.BountyAmount), 0) AS TotalBounties, COUNT(DISTINCT B.Id) AS BadgeCount
//     FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.Reputation, U.CreationDate),
// PostEngagement AS (SELECT P.Id AS PostId, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, AVG(P.ViewCount) AS AvgViewCount FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY P.Id),
// TopEngagedPosts AS (SELECT PE.PostId, PE.CommentCount, PE.UpVotes, PE.DownVotes, PE.AvgViewCount, RANK() OVER (ORDER BY PE.CommentCount DESC, PE.UpVotes DESC) AS EngagementRank
//     FROM PostEngagement PE WHERE PE.CommentCount > 0)
// SELECT U.Id AS UserId, U.DisplayName, U.Reputation, S.TotalBounties, S.BadgeCount, P.Title, P.CreationDate, E.CommentCount, E.UpVotes, E.DownVotes, E.AvgViewCount,
//        COALESCE(CAST(E.AvgViewCount * 1.0 / NULLIF(E.CommentCount, 0) AS DECIMAL(10, 2)), 0) AS ViewPerComment
// FROM Users U LEFT JOIN UserStatistics S ON U.Id = S.UserId LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN TopEngagedPosts E ON P.Id = E.PostId
// WHERE (U.Reputation > 500 OR S.BadgeCount > 5) AND (E.UpVotes > 0 OR E.CommentCount IS NULL) AND P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// ORDER BY S.TotalBounties DESC, U.Reputation DESC, E.CommentCount DESC;
//
// EngagementRank is never read. TotalBounties sums over the votes x badges rows. The WHERE on P.CreationDate makes the Posts join inner, so only recent posts are aggregated.
fn q24209(db: &'static So) -> String {
    let Post { creation_date, view_count, .. } = &db.post;
    let us = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select((&db.vote.bounty_amount).opt()).opt().and(badges_of(db).opt())).fold(0i64, |s, (b, _)| s + b.flatten().unwrap_or(0));
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let pe = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 3], |a, (c, t)| {
        [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let te = (&pe).filt(|a| a[0] > 0);
    let pp = posts_of(db).select(Ident::<Post>::new().with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).and(te.opt()));
    type X = (((Id<User>, i64), (i64, i64)), (Id<Post>, Option<[i64; 3]>));
    let v = drain(
        db.user
            .select(Ident::<User>::new().and(&db.user.reputation).and((&us).and(&bc)).and(pp))
            .filt(|(((_, r), (_, b)), (_, e)): X| (r > 500 || b > 5) && e.map_or(true, |e| e[1] > 0)),
    );
    rows(v.into_iter().map(|(_, (((u, _), (s, b)), (p, e)))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(s), V::I(b)]);
        f.extend(post_fields(db, p, &["title", "created"]));
        let w = view_count.get(p);
        match e {
            Some(e) => {
                f.extend([V::I(e[0]), V::I(e[1]), V::I(e[2]), ofloat(w.map(|w| w as f64))]);
                f.push(V::F(w.map_or(0.0, |w| (w as f64 / e[0] as f64 * 100.0).round() / 100.0)));
            }
            None => f.extend([V::Null, V::Null, V::Null, V::Null, V::F(0.0)]),
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, COUNT(c.Id) AS CommentCount, MAX(v.CreationDate) AS LastVoteDate,
//        DENSE_RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= '2024-10-01 12:34:56'::timestamp - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.PostTypeId, p.Score),
// PostHistoryWithDetails AS (SELECT ph.PostId, ph.PostHistoryTypeId, ph.CreationDate, php.Name AS PostHistoryTypeName, CASE WHEN ph.Comment IS NOT NULL THEN 'Comment: ' || ph.Comment ELSE 'No Comment' END AS CommentDetails
//     FROM PostHistory ph JOIN PostHistoryTypes php ON ph.PostHistoryTypeId = php.Id WHERE ph.CreationDate >= '2024-10-01 12:34:56'::timestamp - INTERVAL '6 months'),
// UserStats AS (SELECT u.Id AS UserId, COUNT(DISTINCT b.Id) AS BadgeCount, SUM(v.BountyAmount) AS TotalBounty FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON u.Id = v.UserId
//     WHERE u.Reputation > 1000 GROUP BY u.Id)
// SELECT rp.Title, rp.ViewCount, rp.CommentCount, rp.Rank, ph.PostHistoryTypeName, ph.CommentDetails, COALESCE(us.BadgeCount, 0) AS UserBadgeCount, COALESCE(us.TotalBounty, 0) AS UserTotalBounty
// FROM RankedPosts rp LEFT JOIN PostHistoryWithDetails ph ON rp.PostId = ph.PostId LEFT JOIN Users u ON rp.PostId = u.Id LEFT JOIN UserStats us ON u.Id = us.UserId
// WHERE ph.PostHistoryTypeId IS NOT NULL OR rp.Rank <= 5 ORDER BY rp.Rank, rp.ViewCount DESC;
//
// `rp.PostId = u.Id` joins a post id to a user id, so it goes through the raw ids. TotalBounty sums over the badges x votes rows.
fn q22130(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let recent = || db.post.with(creation_date.ge(add_years(t0, -1)));
    let cc = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let w = recent().group_by(post_type_id).select(Ident::<Post>::new().and(score).and(&cc)).window(dense_rank, |((_, s), _)| Reverse(s), asc);
    type R = ((Id<Post>, i64), i64);
    let rv: MatSet<R> = (&w).map(|(((p, _), c), k)| ((p, c), k)).collect();
    let PostHistory { creation_date: hd, .. } = &db.post_history;
    let ph = history_of(db).select(Ident::<PostHistory>::new().with(hd.ge(add_months(t0, -6))).with(htype_name(db)));
    let users = || db.user.with((&db.user.reputation).gt(1000));
    let ub = users().group_by(Ident::<User>::new()).select(badges_of(db).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt())).fold(0i64, |s, (_, b)| s + b.flatten().unwrap_or(0));
    let ubc = users().group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let pu = (&db.post.origid).select(&uidx).select((&ub).and(&ubc));
    let v = drain(
        (&rv)
            .select(Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0 .0).select(ph.opt().and(pu.opt()))))
            .filt(|((_, k), (h, _)): (R, (Option<Id<PostHistory>>, Option<(i64, i64)>))| h.is_some() || k <= 5),
    );
    rows(v.into_iter().map(|(_, (((p, c), k), (h, u)))| {
        let mut f = post_fields(db, p, &["title", "views"]);
        f.extend([V::I(c), V::I(k)]);
        f.extend(match h {
            Some(h) => [V::S(htype_name(db).get(h).unwrap()), V::Owned(db.post_history.comment.get(h).map_or("No Comment".to_string(), |c| format!("Comment: {c}")))],
            None => [V::Null, V::Null],
        });
        let (b, n) = u.unwrap_or((0, 0));
        f.extend([V::I(n), V::I(b)]);
        row(f)
    }))
}

// WITH RecursiveCTE AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.OwnerUserId, p.AcceptedAnswerId, p.ViewCount, COALESCE(a.Score, 0) AS AcceptedAnswerScore,
//        ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS RowNum FROM Posts p LEFT JOIN Posts a ON p.AcceptedAnswerId = a.Id WHERE p.PostTypeId = 1),
// UserMetrics AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(COALESCE(p.Score, 0)) AS TotalScore, AVG(COALESCE(p.Score, 0)) AS AverageScore,
//        SUM(COALESCE(b.Class, 0)) AS TotalBadges, MAX(u.Reputation) AS MaxReputation FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName)
// SELECT p.PostId, p.Title, p.CreationDate, p.ViewCount, p.AcceptedAnswerScore, um.DisplayName AS Author, um.TotalPosts, um.TotalScore, um.AverageScore,
//        CASE WHEN p.AcceptedAnswerId IS NULL THEN 'No Accepted Answer' ELSE 'Accepted Answer Exists' END AS AnswerStatus, COUNT(c.Id) AS CommentCount,
//        (SELECT COUNT(v.Id) FROM Votes v WHERE v.PostId = p.PostId AND v.VoteTypeId = 2) AS TotalUpVotes, (SELECT COUNT(v.Id) FROM Votes v WHERE v.PostId = p.PostId AND v.VoteTypeId = 3) AS TotalDownVotes
// FROM RecursiveCTE p JOIN UserMetrics um ON p.OwnerUserId = um.UserId LEFT JOIN Comments c ON p.PostId = c.PostId WHERE p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 MONTH'
// GROUP BY p.PostId, p.Title, p.CreationDate, p.ViewCount, p.AcceptedAnswerScore, um.DisplayName, um.TotalPosts, um.TotalScore, um.AverageScore, p.AcceptedAnswerId
// ORDER BY p.ViewCount DESC, p.PostId LIMIT 50;
//
// Uses rewrites/30155.sql, which adds p.PostId to the ORDER BY (ViewCount ties straddle the LIMIT). No CTE refers to itself; RowNum is never read.
// UserMetrics is driven only for the owners of the recent questions; its score sums and mean run over the posts x badges rows.
fn q30155(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, accepted_answer, accepted_answer_id, score, view_count, .. } = &db.post;
    let rq = || db.post.with(post_type_id.eq(1)).with(creation_date.gt(add_months(ts(2024, 10, 1, 12, 34, 56), -1)));
    let owners: MatSet<Id<User>> = rq().select(owner_user).collect();
    let um = (&owners).group_by(Ident::<User>::new()).select(posts_of(db).select(score).opt().and(badges_of(db).select(&db.badge.class).opt())).fold([0i64; 3], |a, (s, b)| {
        [a[0] + s.unwrap_or(0), a[1] + 1, a[2] + b.unwrap_or(0)]
    });
    let pc = (&owners).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let cc = rq().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = rq().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = drain((&cc).and(&vc).and(owner_user.select(Ident::<User>::new().and(&um).and(&pc))).and(accepted_answer.select(score).opt()));
    let v = top_n(v, |&(p, _)| { let w = view_count.get(p); (w.is_none(), Reverse(w), db.post.origid.get(p).unwrap()) }, 50);
    rows(v.into_iter().map(|(p, (((c, a), ((u, m), n)), s))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views"]);
        f.extend([V::I(s.unwrap_or(0)), user_col(db, u, "name"), V::I(n), V::I(m[0]), V::F(m[0] as f64 / m[1] as f64)]);
        f.push(V::S(if accepted_answer_id.get(p).is_none() { "No Accepted Answer" } else { "Accepted Answer Exists" }));
        f.extend([V::I(c), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.PostTypeId, p.Score, p.AnswerCount, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// UserAverages AS (SELECT u.Id AS UserId, AVG(u.Reputation) AS AvgReputation, COUNT(DISTINCT b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// TopRankedPosts AS (SELECT rp.Id, rp.Title, rp.Score, rp.OwnerUserId, COALESCE(ua.AvgReputation, 0) AS OwnerAverageReputation FROM RankedPosts rp LEFT JOIN UserAverages ua ON rp.OwnerUserId = ua.UserId
//     WHERE rp.Rank <= 5),
// PostCommentStats AS (SELECT p.Id AS PostId, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN c.Score > 0 THEN 1 ELSE 0 END) AS PositiveComments, SUM(CASE WHEN c.Score < 0 THEN 1 ELSE 0 END) AS NegativeComments
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY p.Id)
// SELECT trp.Title AS TopPostTitle, trp.Score AS TopPostScore, pc.CommentCount AS TotalComments, pc.PositiveComments AS PositiveCommentCount, pc.NegativeComments AS NegativeCommentCount,
//        CASE WHEN trp.OwnerAverageReputation IS NULL THEN 'Unknown' WHEN trp.OwnerAverageReputation > 1000 THEN 'Elite' ELSE 'Novice' END AS OwnerReputationCategory
// FROM TopRankedPosts trp JOIN PostCommentStats pc ON trp.Id = pc.PostId WHERE EXISTS (SELECT 1 FROM Votes v WHERE v.PostId = trp.Id AND v.VoteTypeId = 2 HAVING COUNT(v.Id) > 5)
// ORDER BY trp.Score DESC, pc.CommentCount DESC;
//
// AvgReputation averages one user's Reputation over their badge rows, so it is the Reputation itself; an owner not found is COALESCEd to 0.
fn q21627(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, owner_user, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(post_type_id).select(Ident::<Post>::new().and(score)).window(rank, |(_, s)| Reverse(s), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let up = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2)))).fold(0i64, |n, _| n + 1);
    let pc = (&tp).with((&up).filt(|n| n > 5)).group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.score).opt()).fold([0i64; 3], |a, s| match s {
        Some(s) => [a[0] + 1, a[1] + (s > 0) as i64, a[2] + (s < 0) as i64],
        None => a,
    });
    let v = drain((&pc).and(owner_user.select(&db.user.reputation).opt()));
    rows(v.into_iter().map(|(p, (a, r))| {
        let mut f = post_fields(db, p, &["title", "score"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        f.push(V::S(if r.unwrap_or(0) > 1000 { "Elite" } else { "Novice" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, p.ViewCount, p.AnswerCount, p.ClosedDate, DENSE_RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank,
//        COALESCE(NULLIF(b.Reputation, 0), 1) AS UserReputation
//     FROM Posts p LEFT JOIN Users b ON p.OwnerUserId = b.Id WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// UserActivity AS (SELECT u.Id AS UserId, COUNT(DISTINCT p.Id) AS PostsCreated, COUNT(DISTINCT c.Id) AS CommentsMade FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId
//     LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY u.Id HAVING COUNT(DISTINCT p.Id) > 5),
// VotingHistory AS (SELECT v.PostId, COUNT(CASE WHEN vt.Name = 'UpMod' THEN 1 END) AS UpVotes, COUNT(CASE WHEN vt.Name = 'DownMod' THEN 1 END) AS DownVotes FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY v.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, u.DisplayName AS OwnerDisplayName, rp.ViewCount, COALESCE(uh.PostsCreated, 0) AS UserPostsCreated, COALESCE(uh.CommentsMade, 0) AS UserCommentsMade,
//        COALESCE(vh.UpVotes, 0) AS UpVotes, COALESCE(vh.DownVotes, 0) AS DownVotes, CASE WHEN rp.ClosedDate IS NOT NULL THEN 'Closed' ELSE 'Open' END AS Status,
//        CASE WHEN rp.Rank = 1 THEN 'Most Recent' ELSE 'Earlier Post' END AS PostRank
// FROM RankedPosts rp JOIN Users u ON rp.OwnerUserId = u.Id LEFT JOIN UserActivity uh ON u.Id = uh.UserId LEFT JOIN VotingHistory vh ON rp.PostId = vh.PostId WHERE rp.Rank <= 3 ORDER BY rp.CreationDate DESC;
//
// The ownerless partition never joins a user, so only owned questions are ranked.
fn q22419(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, closed_date, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1)).with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(owner_user).select(Ident::<Post>::new().and(owner_user).and(creation_date)).window(dense_rank, |(_, d)| Reverse(d), asc);
    type R = ((Id<Post>, Id<User>), i64);
    let top: MatSet<R> = (&w).filt(|(_, k)| k <= 3).map(|((pu, _), k)| (pu, k)).collect();
    let pcnt = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let cm = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db))).fold(0i64, |n, _| n + 1);
    let ua = (&pcnt).filt(|n| n > 5).and((&cm).opt());
    let vh = votes_of(db).select(vtype_name(db));
    let vhf = (&db.post.id).group_by(Ident::<Post>::new()).select(vh).fold([0i64; 2], |a, n| [a[0] + (n == "UpMod") as i64, a[1] + (n == "DownMod") as i64]);
    let v = drain((&top).select(Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0 .1).select(&ua).opt()).and(Same::<R>::new().map(|x: R| x.0 .0).select(&vhf).opt())));
    rows(v.into_iter().map(|(_, ((((p, u), k), a), vv))| {
        let (n, c) = a.map_or((0, 0), |(n, c)| (n, c.unwrap_or(0)));
        let vv = vv.unwrap_or([0, 0]);
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.push(user_col(db, u, "name"));
        f.extend(post_fields(db, p, &["views"]));
        f.extend([V::I(n), V::I(c), V::I(vv[0]), V::I(vv[1])]);
        f.push(V::S(if closed_date.get(p).is_some() { "Closed" } else { "Open" }));
        f.push(V::S(if k == 1 { "Most Recent" } else { "Earlier Post" }));
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COALESCE(p.ParentId, 0) AS ParentPostId, (SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.ParentId ORDER BY p.CreationDate DESC) AS rn FROM Posts p WHERE p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// AggregatedUserData AS (SELECT u.Id AS UserId, AVG(u.Reputation) AS AvgReputation, SUM(COALESCE(b.Class, 0)) AS TotalBadges, COUNT(DISTINCT p.Id) AS TotalPosts
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id),
// FilteredComments AS (SELECT c.Id AS CommentId, c.PostId, c.Text, c.CreationDate, CASE WHEN c.UserId IS NULL THEN 'Anonymous' ELSE (SELECT DisplayName FROM Users u WHERE u.Id = c.UserId) END AS UserName
//     FROM Comments c WHERE c.CreationDate >= (cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '7 days'))
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.CommentCount, au.UserId, au.AvgReputation, au.TotalBadges, au.TotalPosts, fc.CommentId, fc.Text AS CommentText,
//        fc.CreationDate AS CommentCreationDate, fc.UserName
// FROM RecentPosts rp LEFT JOIN AggregatedUserData au ON rp.ParentPostId = au.UserId LEFT JOIN FilteredComments fc ON rp.PostId = fc.PostId
// WHERE (rp.Score > 10 OR rp.ViewCount > 100) AND (fc.CreationDate IS NULL OR fc.CreationDate >= (cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 week'))
// ORDER BY rp.CreationDate DESC, rp.Score DESC, au.AvgReputation DESC LIMIT 100;
//
// rn is never read. `rp.ParentPostId = au.UserId` joins a post id to a user id, so it goes through the raw ids, and the badges x posts product is driven
// for those users alone. AvgReputation averages one user's Reputation, so it is the Reputation.
fn q20303(db: &'static So) -> String {
    let Post { creation_date, score, view_count, parent_id, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let rp = || db.post.with(creation_date.gt(add_days(t0, -30))).with(score.gt(10).or(view_count.gt(100)));
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let pu = || parent_id.opt().map(|x: Option<i64>| x.unwrap_or(0)).select(&uidx);
    let mu: MatSet<Id<User>> = rp().select(pu()).collect();
    let au = (&mu).group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt().and(posts_of(db).opt())).fold(0i64, |s, (b, _)| s + b.unwrap_or(0));
    let apc = (&mu).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let cc = rp().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let fc = comments_of(db).select(Ident::<Comment>::new().with((&db.comment.creation_date).ge(add_days(t0, -7))));
    let v = drain((&cc).and(pu().select(Ident::<User>::new().and(&au).and(&apc)).opt()).and(fc.opt()));
    let v = top_n(v, |&(p, ((_, u), c))| (Reverse(creation_date.get(p).unwrap()), Reverse(score.get(p).unwrap()), u.map(|((u, _), _)| Reverse(db.user.reputation.get(u).unwrap())).is_none(), u.map(|((u, _), _)| Reverse(db.user.reputation.get(u).unwrap())), p, c), 100);
    rows(v.into_iter().map(|(p, ((n, u), c))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.push(V::I(n));
        f.extend(match u {
            Some(((u, b), k)) => vec![user_col(db, u, "uid"), V::F(db.user.reputation.get(u).unwrap() as f64), V::I(b), V::I(k)],
            None => vec![V::Null, V::Null, V::Null, V::Null],
        });
        f.extend(match c {
            Some(c) => {
                let Comment { origid, text, creation_date: cd, user_id, user, .. } = &db.comment;
                let name = if user_id.get(c).is_none() { V::S("Anonymous") } else { ostr(user.get(c).map(|u| db.user.display_name.get(u).unwrap())) };
                vec![V::I(origid.get(c).unwrap()), V::S(text.get(c).unwrap()), V::T(cd.get(c).unwrap()), name]
            }
            None => vec![V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts, SUM(P.ViewCount) AS TotalViews
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, PositivePosts, TotalViews, DENSE_RANK() OVER (ORDER BY TotalViews DESC) AS RankByViews FROM UserStats),
// RecentPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.ViewCount, U.DisplayName AS OwnerName, COALESCE(COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END), 0) AS CommentCount,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpvoteCount, COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownvoteCount
//     FROM Posts P LEFT JOIN Users U ON P.OwnerUserId = U.Id LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId
//     WHERE P.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days') GROUP BY P.Id, P.Title, P.CreationDate, P.ViewCount, U.DisplayName)
// SELECT TU.DisplayName AS TopUser, TU.TotalViews AS UserTotalViews, R.PostId, R.Title AS RecentPostTitle, R.CreationDate AS RecentPostDate, R.ViewCount AS RecentPostViews, R.CommentCount,
//        R.UpvoteCount, R.DownvoteCount
// FROM TopUsers TU JOIN RecentPosts R ON R.OwnerName = TU.DisplayName WHERE TU.RankByViews <= 5 ORDER BY TU.RankByViews, R.CreationDate DESC;
//
// The join is on DisplayName, so the recent posts are indexed by their owner's name.
fn q4974(db: &'static So) -> String {
    let Post { creation_date, owner_user, view_count, .. } = &db.post;
    let tv = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(view_count.opt()).opt()).fold((0i64, false), |(s, any), w| {
        let w = w.flatten();
        (s + w.unwrap_or(0), any || w.is_some())
    });
    let w = whole(db.user.with(&tv)).select(Ident::<User>::new().and(&tv)).window(dense_rank, |(_, (s, any))| (!any, Reverse(s)), asc);
    type R = ((Id<User>, (i64, bool)), i64);
    let rv: MatSet<R> = (&w).map(|x: R| x).collect();
    let recent = || db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)));
    let byname: HashIdx<Str, Id<Post>> = recent().select(owner_user.select(&db.user.display_name)).inv().collect();
    let rpa = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 3], |a, (c, t)| {
        [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let v = drain((&rv).filt(|(_, k): R| k <= 5).select(Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0 .0).select(&db.user.display_name).select(&byname).select(Ident::<Post>::new().and(&rpa)))));
    rows(v.into_iter().map(|(_, (((u, (s, any)), _), (p, a)))| {
        let mut f = vec![user_col(db, u, "name"), if any { V::I(s) } else { V::Null }];
        f.extend(post_fields(db, p, &["id", "title", "created", "views"]));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RankedVotes AS (SELECT P.Id AS PostId, V.VoteTypeId, COUNT(*) OVER (PARTITION BY P.Id, V.VoteTypeId) AS VoteCount, ROW_NUMBER() OVER (PARTITION BY P.Id ORDER BY COUNT(*) DESC) as VoteRank
//     FROM Votes V JOIN Posts P ON P.Id = V.PostId GROUP BY P.Id, V.VoteTypeId),
// RecentPostHistory AS (SELECT PH.PostId, PH.PostHistoryTypeId, PH.CreationDate, ROW_NUMBER() OVER (PARTITION BY PH.PostId ORDER BY PH.CreationDate DESC) AS RowNum FROM PostHistory PH
//     WHERE PH.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// FilteredPosts AS (SELECT P.Id, P.Title, COALESCE(RV.VoteCount, 0) AS Upvotes, COUNT(DISTINCT C.Id) AS CommentCount, COUNT(DISTINCT B.Id) AS BadgeCount, MAX(RPH.CreationDate) AS LastEditDate
//     FROM Posts P LEFT JOIN RankedVotes RV ON P.Id = RV.PostId AND RV.VoteTypeId = 2 LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Badges B ON P.OwnerUserId = B.UserId
//     LEFT JOIN RecentPostHistory RPH ON P.Id = RPH.PostId AND RPH.RowNum = 1
//     WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '2 years' AND (P.ClosedDate IS NULL OR P.AcceptedAnswerId IS NOT NULL) AND P.ViewCount > 100
//     GROUP BY P.Id, P.Title, RV.VoteCount),
// ExtendedAnalytics AS (SELECT FP.*, DENSE_RANK() OVER (ORDER BY FP.Upvotes DESC) AS RankByUpvotes, COUNT(*) OVER () AS TotalPosts FROM FilteredPosts FP)
// SELECT EA.Title, EA.Upvotes, EA.CommentCount, EA.BadgeCount, EA.LastEditDate,
//        CASE WHEN EA.RankByUpvotes <= (0.1 * EA.TotalPosts) THEN 'Top 10%' WHEN EA.RankByUpvotes <= (0.25 * EA.TotalPosts) THEN 'Top 25%' ELSE 'Below Top 25%' END AS PerformanceCategory
// FROM ExtendedAnalytics EA WHERE EA.BadgeCount > 0 ORDER BY EA.Upvotes DESC, EA.CommentCount DESC;
//
// RankedVotes is grouped by (post, vote type) before its window runs, so each partition is one row and VoteCount is 1: Upvotes is 1 for a post with an
// up vote and 0 otherwise. The row RowNum = 1 carries the latest recent history date, which is all MAX reads.
fn q20867(db: &'static So) -> String {
    let Post { creation_date, closed_date, accepted_answer_id, view_count, owner_user_id, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let fp = || {
        db.post
            .with(creation_date.ge(add_years(t0, -2)))
            .with(view_count.gt(100))
            .with(Ident::<Post>::new().minus(closed_date).or(accepted_answer_id))
    };
    let up = Ident::<Post>::new().with(votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2)))).opt();
    let cc = fp().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let bidx: HashIdx<i64, Id<Badge>> = (&db.badge.user_id).inv().collect();
    let bc = fp().group_by(Ident::<Post>::new()).select(owner_user_id.select(&bidx).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let hd = &db.post_history.creation_date;
    let le = fp().group_by(Ident::<Post>::new()).select(history_of(db).select(hd).filt(move |d| d >= add_years(t0, -1))).fold(i64::MIN, |m, d| m.max(d));
    let total = count(fp());
    let w = whole(fp()).select(Ident::<Post>::new().and(up)).window(dense_rank, |(_, u)| Reverse(u.is_some()), asc);
    let rk = by_first(&(&w).map(|((p, u), k)| (p, (u, k))).collect());
    let v = drain((&cc).and((&bc).filt(|b| b > 0)).and(&rk).and((&le).opt()));
    rows(v.into_iter().map(|(p, (((c, b), (u, k)), d))| {
        let k = k as f64;
        let mut f = post_fields(db, p, &["title"]);
        f.extend([V::I(u.is_some() as i64), V::I(c), V::I(b), ots(d)]);
        f.push(V::S(if k <= 0.1 * total as f64 { "Top 10%" } else if k <= 0.25 * total as f64 { "Top 25%" } else { "Below Top 25%" }));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(COALESCE(V.BountyAmount, 0)) AS TotalBounties, SUM(CASE WHEN V.Id IS NOT NULL THEN 1 ELSE 0 END) AS VoteCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, AnswerCount, QuestionCount, TotalBounties, VoteCount, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserActivity),
// RecentPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.ViewCount, U.DisplayName AS OwnerDisplayName FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id
//     WHERE P.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days')),
// PostStatistics AS (SELECT RP.PostId, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount, COUNT(DISTINCT L.RelatedPostId) AS LinkCount FROM RecentPosts RP
//     LEFT JOIN Comments C ON RP.PostId = C.PostId LEFT JOIN PostLinks L ON RP.PostId = L.PostId GROUP BY RP.PostId)
// SELECT TU.DisplayName, TU.Reputation, TU.PostCount, TU.TotalBounties, RP.Title, RP.CreationDate, RP.ViewCount, PS.CommentCount, PS.LinkCount
// FROM TopUsers TU JOIN RecentPosts RP ON TU.PostCount > 0 JOIN PostStatistics PS ON RP.PostId = PS.PostId WHERE TU.ReputationRank <= 10 ORDER BY TU.Reputation DESC, RP.ViewCount DESC;
//
// ReputationRank reads only Reputation, so the users are picked first and the posts x votes product is driven for them alone. The ON names only TU, so
// they are crossed with the recent posts.
fn q4937(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let w = whole(db.user.with(&db.user.reputation)).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r)| Reverse(r), asc);
    let tu: MatSet<Id<User>> = (&w).filt(|(_, k)| k <= 10).map(|((u, _), _)| u).collect();
    let ua = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(votes_of(db).select((&db.vote.bounty_amount).opt()).opt()).opt()).fold(0i64, |s, x| s + x.flatten().flatten().unwrap_or(0));
    let pc = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let users = rel(drain((&ua).and((&pc).filt(|n| n > 0))));
    let recent = || db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).with(owner_user);
    let lc = recent().group_by(Ident::<Post>::new()).select(links_of(db).select(&db.post_link.related_post_id)).count_distinct();
    let cc = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(links_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let ps = rel(drain((&cc).and((&lc).opt())));
    let mut v = Vec::new();
    (&users).cross(&ps).drive(|_, ((u, (b, n)), (p, (c, l)))| v.push((u, b, n, p, l.unwrap_or(0), c)));
    rows(v.into_iter().map(|(u, b, n, p, l, c)| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(n), V::I(b)]);
        f.extend(post_fields(db, p, &["title", "created", "views"]));
        f.extend([V::I(c), V::I(l)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS rn,
//        COUNT(c.Id) AS CommentCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate > CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.PostTypeId),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.CommentCount FROM RankedPosts rp WHERE rp.rn <= 10),
// PostVotes AS (SELECT p.Id AS PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id),
// PostBadges AS (SELECT b.UserId, COUNT(DISTINCT b.Id) AS BadgeCount FROM Badges b WHERE b.Class = 1 GROUP BY b.UserId),
// FinalReport AS (SELECT tp.PostId, tp.Title, tp.Score, tp.ViewCount, tp.CommentCount, pv.UpVotes, pv.DownVotes, COALESCE(pb.BadgeCount, 0) AS BadgeCount
//     FROM TopPosts tp LEFT JOIN PostVotes pv ON tp.PostId = pv.PostId LEFT JOIN PostBadges pb ON tp.PostId = pb.UserId)
// SELECT fr.PostId, fr.Title, fr.Score, fr.ViewCount, fr.CommentCount, fr.UpVotes, fr.DownVotes, fr.BadgeCount,
//        CASE WHEN fr.Score IS NULL THEN 'No Score' WHEN fr.Score > 10 THEN 'High Score' ELSE 'Moderate Score' END AS ScoreCategory
// FROM FinalReport fr WHERE fr.BadgeCount > 0 ORDER BY fr.Score DESC, fr.Title ASC;
//
// rn reads only base columns, so the posts are picked first (ties broken by post id). `tp.PostId = pb.UserId` joins a post id to a user id, so it goes
// through the raw ids.
fn q20034(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, .. } = &db.post;
    let w = db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(post_type_id).select(Ident::<Post>::new().and(score).and(creation_date)).window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|(((p, _), _), _)| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let pv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let pb = db.badge.with((&db.badge.class).eq(1)).group_by(&db.badge.user_id).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&cc).and(&pv).and((&db.post.origid).select((&pb).filt(|n| n > 0))));
    rows(v.into_iter().map(|(p, ((c, a), b))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "score", "views"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1]), V::I(b)]);
        f.push(V::S(if s > 10 { "High Score" } else { "Moderate Score" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.Score, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserEngagement AS (SELECT u.Id AS UserId, u.DisplayName, SUM(COALESCE(vb.BountyAmount, 0)) AS TotalBounty, SUM(COALESCE(c.Score, 0)) AS TotalComments,
//        AVG(CASE WHEN p.OwnerUserId IS NOT NULL THEN 1 ELSE 0 END) AS OwnershipRatio
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes vb ON p.Id = vb.PostId LEFT JOIN Comments c ON p.Id = c.PostId WHERE u.Reputation > 100 GROUP BY u.Id, u.DisplayName),
// CloseVoteCounts AS (SELECT ph.UserId, COUNT(DISTINCT ph.PostId) AS CloseVoteCount FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.UserId)
// SELECT ue.UserId, ue.DisplayName, ue.TotalBounty, ue.TotalComments, ue.OwnershipRatio, COALESCE(CCO.CloseVoteCount, 0) AS CloseVoteCount, RP.Title AS TopPostTitle, RP.ViewCount AS TopPostViewCount
// FROM UserEngagement ue LEFT JOIN CloseVoteCounts CCO ON ue.UserId = CCO.UserId
// LEFT JOIN RankedPosts RP ON RP.Rank = 1 AND RP.PostId IN (SELECT DISTINCT p.Id FROM Posts p JOIN Comments c ON p.Id = c.PostId WHERE c.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 week')
// WHERE COALESCE(ue.TotalBounty, 0) > (SELECT AVG(TotalBounty) FROM UserEngagement) AND COALESCE(ue.TotalComments, 0) > 5
// ORDER BY ue.TotalBounty DESC, ue.TotalComments DESC, COALESCE(CCO.CloseVoteCount, 0) DESC;
//
// The RankedPosts ON names only RP, so every kept user is crossed with the rank-1 posts that have a comment from the last week, or with one NULL row
// when there are none. Rank breaks (Score, ViewCount) ties by post id. The average is a separate query over the same fold.
fn q21609(db: &'static So) -> String {
    let Post { creation_date, score, view_count, post_type_id, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let ue = db
        .user
        .with((&db.user.reputation).gt(100))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select((&db.vote.bounty_amount).opt()).opt().and(comments_of(db).select(&db.comment.score).opt())).opt())
        .fold([0i64; 4], |a, x| match x {
            Some((b, c)) => [a[0] + b.flatten().unwrap_or(0), a[1] + c.unwrap_or(0), a[2] + 1, a[3] + 1],
            None => [a[0], a[1], a[2], a[3] + 1],
        });
    let (s, n) = (&ue).fold_flat((0i64, 0i64), |(s, n), a| (s + a[0], n + 1));
    let mean = s as f64 / n as f64;
    let PostHistory { post_history_type_id, user, post, .. } = &db.post_history;
    let cvc = db.post_history.with(post_history_type_id.eq(10)).group_by(user).select(post).count_distinct();
    let recent = Ident::<Post>::new().with(comments_of(db).select(Ident::<Comment>::new().with((&db.comment.creation_date).ge(add_days(t0, -7)))));
    let w = db.post.with(creation_date.ge(add_years(t0, -1))).group_by(post_type_id).select(Ident::<Post>::new().and(score).and(view_count.opt())).window(row_number, |((p, s), w)| (Reverse(s), w.is_none(), Reverse(w), p), asc);
    let rp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k == 1).map(|(((p, _), _), _)| p).select(recent).collect();
    let rpi: HashIdx<(), Id<Post>> = whole(&rp).collect();
    let rpo = rel(vec![()]).select((&rpi).opt());
    let users = rel(drain((&ue).filt(move |a| a[0] as f64 > mean && a[1] > 5).and((&cvc).opt())));
    let mut v = Vec::new();
    (&users).cross(&rpo).drive(|_, ((u, (a, c)), p)| v.push((u, a, c, p)));
    rows(v.into_iter().map(|(u, a, c, p)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::F(a[2] as f64 / a[3] as f64), V::I(c.unwrap_or(0))]);
        f.extend(match p {
            Some(p) => post_fields(db, p, &["title", "views"]),
            None => vec![V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserBadges AS (SELECT b.UserId, COUNT(*) AS BadgeCount, STRING_AGG(b.Name, ', ') AS BadgeNames FROM Badges b WHERE b.Class = 1 GROUP BY b.UserId),
// UserPostStats AS (SELECT p.OwnerUserId, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
//        MAX(p.CreationDate) AS LastPostDate FROM Posts p GROUP BY p.OwnerUserId),
// ActiveUsers AS (SELECT u.Id, u.DisplayName, u.Reputation, u.CreationDate, ub.BadgeCount, p.TotalPosts, p.Questions, p.Answers, p.LastPostDate FROM Users u
//     LEFT JOIN UserBadges ub ON u.Id = ub.UserId LEFT JOIN UserPostStats p ON u.Id = p.OwnerUserId WHERE u.Reputation > 50),
// UserActivity AS (SELECT au.Id, au.DisplayName, au.Reputation, au.BadgeCount, au.TotalPosts, au.Questions, au.Answers, au.LastPostDate, DENSE_RANK() OVER (ORDER BY au.Reputation DESC) AS UserRank FROM ActiveUsers au),
// PostHistoryStats AS (SELECT ph.UserId, COUNT(*) AS EditCount, SUM(CASE WHEN ph.PostHistoryTypeId IN (4, 5, 6) THEN 1 ELSE 0 END) AS Edits, SUM(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS ClosedPosts
//     FROM PostHistory ph GROUP BY ph.UserId)
// SELECT ua.Id AS UserId, ua.DisplayName, ua.Reputation, ua.BadgeCount, ua.TotalPosts, ua.Questions, ua.Answers, ua.LastPostDate, phs.EditCount, phs.Edits AS TotalEdits, phs.ClosedPosts, ua.UserRank
// FROM UserActivity ua LEFT JOIN PostHistoryStats phs ON ua.Id = phs.UserId WHERE ua.UserRank <= 10 ORDER BY ua.Reputation DESC, ua.LastPostDate DESC;
//
// BadgeNames is never read.
fn q33187(db: &'static So) -> String {
    let Post { owner_user, post_type_id, creation_date, .. } = &db.post;
    let w = whole(db.user.with((&db.user.reputation).gt(50))).select(Ident::<User>::new().and(&db.user.reputation)).window(dense_rank, |(_, r)| Reverse(r), asc);
    type R = ((Id<User>, i64), i64);
    let tu: MatSet<R> = (&w).map(|x: R| x).collect();
    let ub = db.badge.with((&db.badge.class).eq(1)).group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let ups = db.post.group_by(owner_user).select(post_type_id.and(creation_date)).fold([0i64, 0, 0, i64::MIN], |a, (t, d)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3].max(d)]);
    let pt = &db.post_history.post_history_type_id;
    let phs = db.post_history.group_by(&db.post_history.user).select(pt).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + matches!(t, 4 | 5 | 6) as i64, a[2] + (t == 10) as i64]);
    let u = || Same::<R>::new().map(|x: R| x.0 .0);
    let v = drain((&tu).filt(|(_, k): R| k <= 10).select(Same::<R>::new().and(u().select(&ub).opt()).and(u().select(&ups).opt()).and(u().select(&phs).opt())));
    rows(v.into_iter().map(|(_, (((((u, _), k), b), p), h))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.push(oint(b));
        f.extend(match p {
            Some(a) => [V::I(a[0]), V::I(a[1]), V::I(a[2]), V::T(a[3])],
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        f.extend(match h {
            Some(a) => a.map(V::I),
            None => [V::Null, V::Null, V::Null],
        });
        f.push(V::I(k));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.PostTypeId = 1),
// UserMetrics AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownVotes, COUNT(DISTINCT b.Id) AS BadgeCount
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PopularPosts AS (SELECT p.Id AS PostId, p.Title, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
//        COUNT(c.Id) AS CommentCount FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title
//     HAVING SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) > 10 ORDER BY UpVotes DESC LIMIT 10)
// SELECT um.UserId, um.DisplayName, um.TotalBounty, um.TotalUpVotes, um.TotalDownVotes, um.BadgeCount, pp.PostId, pp.Title, pp.UpVotes, pp.DownVotes, pp.CommentCount
// FROM UserMetrics um LEFT JOIN PopularPosts pp ON um.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = pp.PostId)
// WHERE um.TotalUpVotes > 50 AND (um.TotalDownVotes IS NULL OR um.TotalDownVotes < 5) ORDER BY um.TotalBounty DESC, um.TotalUpVotes DESC;
//
// RankedPosts is never read. The subquery looks a post up by its primary key, so it is the post's owner. PopularPosts breaks UpVotes ties by post id.
fn q63(db: &'static So) -> String {
    let Post { post_type_id, owner_user, .. } = &db.post;
    let um = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select((&db.vote.vote_type_id).and((&db.vote.bounty_amount).opt())).opt().and(badges_of(db).opt())).fold([0i64; 3], |a, (v, _)| match v {
        Some((t, b)) => [a[0] + b.unwrap_or(0), a[1] + (t == 2) as i64, a[2] + (t == 3) as i64],
        None => a,
    });
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let pp = db.post.with(post_type_id.eq(1)).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt())).fold([0i64; 3], |a, (t, c)| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64]
    });
    let top = top_n(drain((&pp).filt(|a| a[0] > 10)), |&(p, a)| (Reverse(a[0]), p), 10);
    type P = (Id<Post>, [i64; 3]);
    let tp = rel(top);
    let by_owner: HashIdx<Id<User>, P> = (&tp).map(|x: P| x.0).select(owner_user).inv().select(&tp).collect();
    let v = drain((&um).filt(|a| a[1] > 50 && a[2] < 5).and(&bc).and((&by_owner).opt()));
    rows(v.into_iter().map(|(u, ((a, b), p))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(b)]);
        f.extend(match p {
            Some((p, x)) => {
                let mut g = post_fields(db, p, &["id", "title"]);
                g.extend(x.map(V::I));
                g
            }
            None => vec![V::Null, V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS rn,
//        COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount, SUM(v.BountyAmount) OVER (PARTITION BY p.Id) AS TotalBounty
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (8, 9)
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.Score > (SELECT AVG(Score) FROM Posts)),
// CloseReasonStats AS (SELECT p.Id AS PostId, MAX(CASE WHEN ph.PostHistoryTypeId = 10 THEN CAST(ph.Comment AS INT) END) AS CloseReasonId, COUNT(DISTINCT ph.UserId) AS CloseVoteCount
//     FROM Posts p LEFT JOIN PostHistory ph ON p.Id = ph.PostId WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY p.Id),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, COALESCE(cos.CloseReasonId, 0) AS CloseReasonId, COALESCE(cos.CloseVoteCount, 0) AS CloseVoteCount, rp.CommentCount, rp.TotalBounty
//     FROM RankedPosts rp LEFT JOIN CloseReasonStats cos ON rp.PostId = cos.PostId WHERE rp.rn <= 5)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.ViewCount, tp.CommentCount, tp.TotalBounty,
//        CASE WHEN tp.CloseReasonId IS NULL THEN 'Not Closed' WHEN tp.CloseReasonId IS NOT NULL THEN (SELECT Name FROM CloseReasonTypes WHERE Id = tp.CloseReasonId) END AS CloseReason,
//        CASE WHEN tp.CloseVoteCount >= 5 THEN 'Highly Controversial' ELSE 'Regular' END AS ControversialStatus
// FROM TopPosts tp ORDER BY tp.ViewCount DESC, tp.CreationDate ASC;
//
// RankedPosts has one row per post x comment x bounty-vote, and rn numbers those rows, so the first five rows per type are numbered over the
// product. Rows of one post tie on CreationDate but project the same values. CloseReasonId is COALESCEd to 0, so it is never NULL.
fn q22793(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, .. } = &db.post;
    let (s, n) = score.fold_flat((0i64, 0i64), |(s, n), x| (s + x, n + 1));
    let mean = s as f64 / n as f64;
    let rp = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(score.filt(move |s| s as f64 > mean));
    let bv = || votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9])));
    let agg = rp().group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(bv().select((&db.vote.bounty_amount).opt()).opt())).fold((0i64, 0i64, false), |(c, s, any), (ci, b)| {
        let b = b.flatten();
        (c + ci.is_some() as i64, s + b.unwrap_or(0), any || b.is_some())
    });
    let w = rp()
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(creation_date).and(comments_of(db).opt()).and(bv().opt()))
        .window(row_number, |(((p, d), c), v)| (Reverse(d), p, c, v), asc);
    let PostHistory { post_history_type_id, comment, user_id, .. } = &db.post_history;
    let ch = || db.post_history.with(post_history_type_id.is_in([10, 11])).group_by(&db.post_history.post);
    let crs = ch().select(post_history_type_id.and(comment.opt())).fold(None::<i64>, |m, (t, c)| {
        let id = if t == 10 { c.and_then(|c: Str| c.trim().parse::<i64>().ok()) } else { None };
        m.max(id)
    });
    let cvc = ch().select(user_id).count_distinct();
    let crt: HashIdx<i64, Id<CloseReasonType>> = (&db.close_reason_type.origid).inv().collect();
    let rid = (&crs).opt().map(|r: Option<Option<i64>>| r.flatten().unwrap_or(0)).select(&crt).select(&db.close_reason_type.name);
    let v = drain((&w).filt(|(_, k)| k <= 5).map(|((((p, _), _), _), _)| p).select(Ident::<Post>::new().and(&agg).and((&cvc).opt()).and(rid.opt())));
    rows(v.into_iter().map(|(_, (((p, (c, s, any)), r), name))| {
        let cv = r.unwrap_or(0);
        let mut f = post_fields(db, p, &["id", "title", "created", "views"]);
        f.extend([V::I(c), if any { V::I(s) } else { V::Null }]);
        f.push(ostr(name));
        f.push(V::S(if cv >= 5 { "Highly Controversial" } else { "Regular" }));
        row(f)
    }))
}

// WITH PostStatistics AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT v.Id) FILTER (WHERE v.VoteTypeId IN (2, 3)) AS VoteCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC, p.Id) AS PostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId),
// TopPosts AS (SELECT ps.PostId, ps.Title, ps.CreationDate, ps.Score, ps.ViewCount, ps.CommentCount, ps.VoteCount, ps.UpVotes, ps.DownVotes, ps.PostRank, DENSE_RANK() OVER (ORDER BY ps.Score DESC) AS ScoreRank
//     FROM PostStatistics ps),
// FilteredPosts AS (SELECT tp.*, CASE WHEN tp.CommentCount = 0 THEN 'No Comments' WHEN tp.UpVotes > tp.DownVotes THEN 'Positive Feedback' ELSE 'Needs Improvement' END AS FeedbackStatus
//     FROM TopPosts tp WHERE tp.Score > 0 AND tp.PostRank <= 5 ORDER BY tp.ScoreRank)
// SELECT fp.PostId, fp.Title, fp.CreationDate, fp.Score, fp.ViewCount, fp.CommentCount, fp.VoteCount, fp.UpVotes, fp.DownVotes, fp.FeedbackStatus,
//        CASE WHEN fp.FeedbackStatus = 'No Comments' THEN 'Consider adding engaging content!' ELSE 'Great contribution!' END AS Advice
// FROM FilteredPosts fp WHERE fp.FeedbackStatus IS NOT NULL ORDER BY fp.Score DESC, fp.CreationDate DESC;
//
// Uses rewrites/23440.sql, which adds p.Id to PostRank's ORDER BY (an owner posted twice at the same instant). PostRank reads only base columns, so the posts
// are picked first; the NULL owner is its own partition. UpVotes and DownVotes sum over the comments x votes rows.
fn q23440(db: &'static So) -> String {
    let Post { creation_date, score, owner_user_id, origid, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date).and(origid)).window(row_number, |((_, d), o)| (Reverse(d), o), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|(((p, _), _), _)| p).collect();
    let fp = || (&tp).with(score.gt(0));
    let agg = fp().group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 2], |a, (_, t)| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
    });
    let cc = fp().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = fp().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold(0i64, |n, t| n + matches!(t, Some(2 | 3)) as i64);
    let v = drain((&agg).and(&cc).and(&vc));
    rows(v.into_iter().map(|(p, ((a, c), n))| {
        let st = if c == 0 { "No Comments" } else if a[0] > a[1] { "Positive Feedback" } else { "Needs Improvement" };
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(c), V::I(n), V::I(a[0]), V::I(a[1]), V::S(st)]);
        f.push(V::S(if c == 0 { "Consider adding engaging content!" } else { "Great contribution!" }));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, COUNT(DISTINCT C.Id) AS CommentCount, RANK() OVER (ORDER BY COUNT(DISTINCT P.Id) DESC) AS PostRank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId AND P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' LEFT JOIN Votes V ON P.Id = V.PostId
//     LEFT JOIN Comments C ON P.Id = C.PostId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UA.UserId, UA.DisplayName, UA.PostCount, UA.UpVotes, UA.DownVotes, UA.CommentCount FROM UserActivity UA WHERE UA.PostRank <= 10),
// PostWithVotes AS (SELECT P.Id AS PostId, P.Title, P.Score, P.ViewCount, P.CreationDate, COUNT(V.Id) FILTER (WHERE V.VoteTypeId = 2) AS UpVoteCount, COUNT(V.Id) FILTER (WHERE V.VoteTypeId = 3) AS DownVoteCount
//     FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY P.Id, P.Title, P.Score, P.ViewCount, P.CreationDate),
// PostStatistics AS (SELECT PW.PostId, PW.Title, PW.Score, PW.ViewCount, PW.CreationDate, COALESCE(PW.UpVoteCount, 0) AS TotalUpVotes, COALESCE(PW.DownVoteCount, 0) AS TotalDownVotes,
//        CASE WHEN PW.Score IS NULL THEN 'No Score' WHEN PW.Score > 0 THEN 'Positive Score' ELSE 'Negative Score' END AS ScoreCategory FROM PostWithVotes PW)
// SELECT TU.DisplayName, TU.PostCount, TU.UpVotes, TU.DownVotes, TU.CommentCount, PS.Title, PS.Score, PS.ViewCount, PS.CreationDate, PS.ScoreCategory
// FROM TopUsers TU JOIN PostStatistics PS ON TU.UserId = PS.PostId WHERE PS.TotalUpVotes > 5 ORDER BY TU.UpVotes DESC, PS.CreationDate DESC;
//
// PostRank reads only the post count, so the users are picked first and the posts x votes x comments product is driven for them alone.
// `TU.UserId = PS.PostId` joins a user id to a post id, so it goes through the raw ids.
fn q1090(db: &'static So) -> String {
    let Post { creation_date, score, .. } = &db.post;
    let recent = || posts_of(db).select(Ident::<Post>::new().with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))));
    let pc = db.user.group_by(Ident::<User>::new()).select(recent().opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let w = whole(db.user.with(&pc)).select(Ident::<User>::new().and(&pc)).window(rank, |(_, n)| Reverse(n), asc);
    let tu: MatSet<Id<User>> = (&w).filt(|(_, k)| k <= 10).map(|((u, _), _)| u).collect();
    let ua = (&tu).group_by(Ident::<User>::new()).select(recent().select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt())).opt()).fold([0i64; 2], |a, x| {
        let t = x.and_then(|(t, _)| t);
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
    });
    let cc = (&tu).group_by(Ident::<User>::new()).select(recent().select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let pw = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2)))).fold(0i64, |n, _| n + 1);
    let v = drain((&ua).and(&pc).and(&cc).and((&db.user.origid).select(&pidx).with((&pw).filt(|n| n > 5))));
    rows(v.into_iter().map(|(u, (((a, n), c), p))| {
        let s = score.get(p).unwrap();
        let mut f = vec![user_col(db, u, "name"), V::I(n), V::I(a[0]), V::I(a[1]), V::I(c)];
        f.extend(post_fields(db, p, &["title", "score", "views", "created"]));
        f.push(V::S(if s > 0 { "Positive Score" } else { "Negative Score" }));
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount, ROW_NUMBER() OVER (ORDER BY p.CreationDate DESC) AS RowNum
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(COALESCE(b.Class, 0)) AS BadgeCount, COUNT(DISTINCT p.Id) AS PostCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId
//     LEFT JOIN Posts p ON u.Id = p.OwnerUserId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName HAVING COUNT(DISTINCT p.Id) > 5),
// PostedLinks AS (SELECT pl.PostId, COUNT(pl.RelatedPostId) AS RelatedPostCount FROM PostLinks pl GROUP BY pl.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, u.DisplayName AS OwnerName, rp.CommentCount, rp.UpVoteCount, rp.DownVoteCount, COALESCE(pl.RelatedPostCount, 0) AS RelatedPosts,
//        CASE WHEN u.Location IS NOT NULL AND u.Location <> '' THEN u.Location ELSE 'Location not provided' END AS UserLocation,
//        CASE WHEN u.Reputation IS NULL THEN 'No Reputation' ELSE CAST(u.Reputation AS VARCHAR) END AS Reputation, t.BadgeCount, t.PostCount
// FROM RecentPosts rp JOIN Users u ON rp.OwnerUserId = u.Id LEFT JOIN PostedLinks pl ON rp.PostId = pl.PostId LEFT JOIN TopUsers t ON u.Id = t.UserId
// WHERE rp.RowNum <= 10 ORDER BY rp.UpVoteCount DESC, rp.CommentCount DESC;
//
// RowNum reads only CreationDate, so the ten newest posts are picked first (ties broken by post id) and TopUsers is driven for their owners alone.
fn q4406(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let top = top_n(drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(creation_date)), |&(p, d)| (Reverse(d), p), 10);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let agg = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 3], |a, (c, t)| {
        [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let pl = (&tp).group_by(Ident::<Post>::new()).select(links_of(db)).fold(0i64, |n, _| n + 1);
    let owners: MatSet<Id<User>> = (&tp).select(owner_user).with((&db.user.reputation).gt(1000)).collect();
    let tb = (&owners).group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt().and(posts_of(db).opt())).fold(0i64, |s, (b, _)| s + b.unwrap_or(0));
    let tpc = (&owners).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let v = drain((&agg).and((&pl).opt()).and(owner_user.select(Ident::<User>::new().and((&tb).and((&tpc).filt(|n| n > 5)).opt()))));
    rows(v.into_iter().map(|(p, ((a, l), (u, t)))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.push(user_col(db, u, "name"));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(l.unwrap_or(0))]);
        f.push(V::S(db.user.location.get(u).filter(|s| !s.is_empty()).unwrap_or("Location not provided")));
        f.push(V::Owned(db.user.reputation.get(u).unwrap().to_string()));
        f.extend(match t {
            Some((b, n)) => [V::I(b), V::I(n)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RECURSIVE UserReputation AS (SELECT U.Id AS UserId, U.Reputation, 1 AS Level, U.CreationDate FROM Users U WHERE U.Reputation > 1000 UNION ALL
//     SELECT U.Id AS UserId, U.Reputation, UR.Level + 1, U.CreationDate FROM Users U INNER JOIN UserReputation UR ON U.Id = UR.UserId WHERE UR.Reputation < U.Reputation),
// InactivePosts AS (SELECT P.Id AS PostId, P.Title, P.LastActivityDate, COALESCE(P.ViewCount, 0) AS ViewCount, DENSE_RANK() OVER (PARTITION BY P.OwnerUserId ORDER BY P.LastActivityDate DESC) AS RecentActivityRank
//     FROM Posts P WHERE P.LastActivityDate < TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '6 months'),
// ClosedPosts AS (SELECT PH.PostId, PH.CreationDate, C.Name AS CloseReason, COUNT(*) AS CloseCount FROM PostHistory PH INNER JOIN CloseReasonTypes C ON PH.PostHistoryTypeId = 10 AND PH.Comment = CAST(C.Id AS varchar)
//     GROUP BY PH.PostId, C.Name, PH.CreationDate),
// UserWithBadges AS (SELECT U.Id AS UserId, B.Name AS BadgeName, B.Class, COUNT(*) AS BadgeCount FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId AND B.Class = 1 GROUP BY U.Id, B.Name, B.Class)
// SELECT U.DisplayName AS UserName, U.Reputation, COALESCE(B.BadgeCount, 0) AS GoldBadgeCount, P.PostId, P.Title AS PostTitle, P.ViewCount, COALESCE(Closed.CloseCount, 0) AS CloseCount,
//        COALESCE(R.ColumnCount, 0) AS RecentPostCount
// FROM Users U LEFT JOIN UserWithBadges B ON U.Id = B.UserId LEFT JOIN InactivePosts P ON U.Id = P.PostId LEFT JOIN ClosedPosts Closed ON P.PostId = Closed.PostId
// LEFT JOIN (SELECT OwnerUserId, COUNT(*) AS ColumnCount FROM Posts GROUP BY OwnerUserId) R ON U.Id = R.OwnerUserId
// WHERE U.Reputation > (SELECT AVG(Reputation) FROM Users) ORDER BY U.Reputation DESC, GoldBadgeCount DESC, ViewCount DESC;
//
// The recursive UserReputation and RecentActivityRank are never read. A user with no gold badge keeps one UserWithBadges group, the NULL one, whose
// COUNT(*) is 1. `U.Id = P.PostId` joins a user id to a post id, so it goes through the raw ids.
fn q30052(db: &'static So) -> String {
    let Post { last_activity_date, owner_user, .. } = &db.post;
    let (s, n) = (&db.user.reputation).fold_flat((0i64, 0i64), |(s, n), r| (s + r, n + 1));
    let mean = s as f64 / n as f64;
    let users = db.user.with((&db.user.reputation).filt(move |r| r as f64 > mean));
    type G = (Id<User>, Str);
    let ub = db.badge.with((&db.badge.class).eq(1)).group_by((&db.badge.user).and(&db.badge.name)).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let ubr = rel(drain(&ub));
    let ubi: HashIdx<Id<User>, (G, i64)> = (&ubr).map(|x: (G, i64)| x.0 .0).inv().select(&ubr).collect();
    let inactive: HashIdx<i64, Id<Post>> = db.post.with(last_activity_date.lt(add_months(ts(2024, 10, 1, 12, 34, 56), -6))).select(&db.post.origid).inv().collect();
    let crt: HashIdx<Str, Id<CloseReasonType>> = (&db.close_reason_type.origid).map(|i: i64| &*Box::leak(i.to_string().into_boxed_str())).inv().collect();
    let PostHistory { post_history_type_id, comment, post, creation_date: hd, .. } = &db.post_history;
    type C = (((Id<Post>, Id<CloseReasonType>), i64), i64);
    let cp = db.post_history.with(post_history_type_id.eq(10)).group_by(post.and(comment.select(&crt)).and(hd)).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let cpr = rel(drain(&cp));
    let cpi: HashIdx<Id<Post>, i64> = (&cpr).map(|x: C| x.0 .0 .0).inv().select(&cpr).map(|x: C| x.1).collect();
    let rc = db.post.group_by(owner_user).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let v = drain(users.select(Ident::<User>::new().and((&ubi).opt()).and((&db.user.origid).select(&inactive).select(Ident::<Post>::new().and((&cpi).opt())).opt()).and((&rc).opt())));
    rows(v.into_iter().map(|(_, (((u, g), p), r))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(g.map_or(1, |g| g.1)));
        f.extend(match p {
            Some((p, c)) => {
                let mut g = post_fields(db, p, &["id", "title"]);
                g.extend([V::I(db.post.view_count.get(p).unwrap_or(0)), V::I(c.unwrap_or(0))]);
                g
            }
            None => vec![V::Null, V::Null, V::Null, V::I(0)],
        });
        f.push(V::I(r.unwrap_or(0)));
        row(f)
    }))
}

// WITH BadgesSummary AS (SELECT u.Id AS UserId, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges, COUNT(DISTINCT b.Id) AS TotalBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostActivity AS (SELECT p.OwnerUserId, COUNT(DISTINCT p.Id) AS PostCount, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews, SUM(CASE WHEN p.Score > 0 THEN p.Score ELSE 0 END) AS PositiveScore,
//        RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY SUM(p.ViewCount) DESC) AS ViewRank FROM Posts p WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year' GROUP BY p.OwnerUserId),
// UserVotingStats AS (SELECT v.UserId, SUM(CASE WHEN v.VoteTypeId IN (2, 4) THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes FROM Votes v GROUP BY v.UserId),
// CombinedResults AS (SELECT u.Id AS UserId, u.DisplayName, bs.GoldBadges, bs.SilverBadges, bs.BronzeBadges, pa.PostCount, pa.TotalViews, pa.PositiveScore, uv.TotalUpVotes, uv.TotalDownVotes,
//        GREATEST(COALESCE(bs.TotalBadges, 0), COALESCE(pa.PostCount, 0), COALESCE(uv.TotalUpVotes, 0) - COALESCE(uv.TotalDownVotes, 0)) AS PerformanceMetric
//     FROM Users u LEFT JOIN BadgesSummary bs ON u.Id = bs.UserId LEFT JOIN PostActivity pa ON u.Id = pa.OwnerUserId LEFT JOIN UserVotingStats uv ON u.Id = uv.UserId)
// SELECT UserId, DisplayName, GoldBadges, SilverBadges, BronzeBadges, PostCount, TotalViews, PositiveScore, TotalUpVotes, TotalDownVotes, PerformanceMetric
// FROM CombinedResults WHERE PerformanceMetric IS NOT NULL ORDER BY PerformanceMetric DESC LIMIT 10;
//
// ViewRank is never read.
fn q24072(db: &'static So) -> String {
    let Post { creation_date, owner_user, view_count, score, .. } = &db.post;
    let bs = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64, a[3] + 1],
        None => a,
    });
    let pa = db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1))).group_by(owner_user).select(view_count.opt().and(score)).fold([0i64; 3], |a, (w, s)| {
        [a[0] + 1, a[1] + w.unwrap_or(0), a[2] + s.max(0)]
    });
    let uv = db.vote.group_by(&db.vote.user).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + matches!(t, 2 | 4) as i64, a[1] + (t == 3) as i64]);
    let v = drain((&bs).and((&pa).opt()).and((&uv).opt()));
    let pm = |b: [i64; 4], p: Option<[i64; 3]>, u: Option<[i64; 2]>| b[3].max(p.map_or(0, |p| p[0])).max(u.map_or(0, |u| u[0] - u[1]));
    let v = top_n(v, |&(u, ((b, p), w))| (Reverse(pm(b, p, w)), u), 10);
    rows(v.into_iter().map(|(u, ((b, p), w))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(b[0]), V::I(b[1]), V::I(b[2])]);
        f.extend(match p {
            Some(p) => p.map(V::I),
            None => [V::Null, V::Null, V::Null],
        });
        f.extend(match w {
            Some(w) => w.map(V::I),
            None => [V::Null, V::Null],
        });
        f.push(V::I(pm(b, p, w)));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.Reputation, RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U),
// PostStats AS (SELECT P.Id AS PostId, P.PostTypeId, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount, COUNT(DISTINCT PL.RelatedPostId) AS RelatedPosts
//     FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN PostLinks PL ON P.Id = PL.PostId GROUP BY P.Id, P.PostTypeId),
// ClosedPosts AS (SELECT PH.PostId, MIN(PH.CreationDate) AS FirstClosed FROM PostHistory PH WHERE PH.PostHistoryTypeId = 10 GROUP BY PH.PostId),
// AcceptedAnswerCount AS (SELECT P.AcceptedAnswerId, COUNT(P.Id) AS AnswerCount FROM Posts P WHERE P.PostTypeId = 1 GROUP BY P.AcceptedAnswerId)
// SELECT U.DisplayName, U.Reputation, UP.ReputationRank, P.Title, ST.UpVotes, ST.DownVotes, ST.CommentCount, ST.RelatedPosts, CASE WHEN CP.FirstClosed IS NOT NULL THEN 'Closed' ELSE 'Open' END AS PostStatus,
//        COALESCE(AAC.AnswerCount, 0) AS AcceptedAnswers, CASE WHEN P.Body IS NULL THEN 'No Content' ELSE SUBSTRING(P.Body, 1, 200) || '...' END AS PreviewBody,
//        CAST(P.CreationDate AS DATE) AS PostCreationDate, CONCAT('https://example.com/posts/', P.Id) AS PostUrl
// FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id JOIN UserReputation UP ON U.Id = UP.UserId JOIN PostStats ST ON P.Id = ST.PostId LEFT JOIN ClosedPosts CP ON P.Id = CP.PostId
// LEFT JOIN AcceptedAnswerCount AAC ON P.AcceptedAnswerId = AAC.AcceptedAnswerId
// WHERE P.CreationDate >= (cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 YEAR') AND P.PostTypeId = 1 AND U.Reputation > 100 ORDER BY U.Reputation DESC, P.Title LIMIT 100;
//
// AcceptedAnswerCount's NULL group never joins, so it counts the questions sharing the post's accepted answer.
fn q20834(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, accepted_answer_id, title, body, .. } = &db.post;
    let w = whole(db.user.with(&db.user.reputation)).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r)| Reverse(r), asc);
    let urk = by_first(&(&w).map(|((u, _), k)| (u, k)).collect());
    let qs = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(post_type_id.eq(1)).with(owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(100))));
    let st = qs().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()).and(links_of(db).opt())).fold([0i64; 3], |a, ((t, c), _)| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64]
    });
    let rl = qs().group_by(Ident::<Post>::new()).select(links_of(db).select(&db.post_link.related_post_id)).count_distinct();
    let cp = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10)));
    let aac = db.post.with(post_type_id.eq(1)).group_by(accepted_answer_id).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&st).and((&rl).opt()).and(owner_user.select(Ident::<User>::new().and(&urk))).and(Ident::<Post>::new().with(cp).opt()).and(accepted_answer_id.select(&aac).opt()));
    let v = top_n(v, |&(p, ((((_, _), (u, _)), _), _))| (Reverse(db.user.reputation.get(u).unwrap()), title.get(p).is_none(), title.get(p), p), 100);
    rows(v.into_iter().map(|(p, ((((a, l), (u, k)), c), n))| {
        let l = l.unwrap_or(0);

        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(k));
        f.extend(post_fields(db, p, &["title"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(l)]);
        f.push(V::S(if c.is_some() { "Closed" } else { "Open" }));
        f.push(V::I(n.unwrap_or(0)));
        f.push(V::Owned(format!("{}...", body.get(p).unwrap().chars().take(200).collect::<String>())));
        f.push(V::D(trunc_day(creation_date.get(p).unwrap())));
        f.push(V::Owned(format!("https://example.com/posts/{}", db.post.origid.get(p).unwrap())));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.Score, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS ScoreRank,
//        COUNT(*) OVER (PARTITION BY p.OwnerUserId) AS TotalPosts FROM Posts p WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 YEAR' AND p.PostTypeId = 1),
// UserScores AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(CASE WHEN v.UserId IS NOT NULL AND vt.Id = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.UserId IS NOT NULL AND vt.Id = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COALESCE(SUM(CASE WHEN v.UserId IS NOT NULL AND vt.Id = 10 THEN 1 ELSE 0 END), 0) AS Deletes,
//        COALESCE(SUM(CASE WHEN v.UserId IS NOT NULL AND vt.Id = 11 THEN 1 ELSE 0 END), 0) AS Undeletes
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY u.Id, u.DisplayName),
// ClosedPostHistory AS (SELECT ph.PostId, ph.UserId, COUNT(*) FILTER (WHERE ph.PostHistoryTypeId = 10) AS CloseCount, COUNT(*) FILTER (WHERE ph.PostHistoryTypeId = 11) AS ReopenCount
//     FROM PostHistory ph GROUP BY ph.PostId, ph.UserId)
// SELECT DISTINCT u.DisplayName, p.Title, p.ViewCount, p.Score, p.ScoreRank, us.UpVotes, us.DownVotes, COALESCE(cph.CloseCount, 0) AS CloseCount, COALESCE(cph.ReopenCount, 0) AS ReopenCount,
//        COALESCE(NULLIF(us.UpVotes, 0) / NULLIF(us.DownVotes, 0), 0) AS VoteRatio
// FROM RankedPosts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN UserScores us ON u.Id = us.UserId LEFT JOIN ClosedPostHistory cph ON p.PostId = cph.PostId
// WHERE u.Reputation > 100 AND (p.Score > 10 OR (p.ViewCount > 1000 AND p.Score > 5)) ORDER BY VoteRatio DESC, p.Score DESC LIMIT 100;
//
// ScoreRank breaks Score ties by post id (the SQL leaves them open). The post fixes every other projected column but the history counts, so the DISTINCT
// is taken over (post, close count, reopen count), in prela.
fn q24062(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, score, view_count, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(post_type_id.eq(1))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(owner_user).and(score).and(view_count.opt()).and(owner_user.select(&db.user.reputation)))
        .window(row_number, |((((p, _), s), _), _)| (Reverse(s), p), asc);
    type R = ((Id<Post>, Id<User>), i64);
    let rp: MatSet<R> = (&w)
        .filt(|((((_, s), w), r), _)| r > 100 && (s > 10 || (w.map_or(false, |w| w > 1000) && s > 5)))
        .map(|((((pu, _), _), _), k)| (pu, k))
        .collect();
    let us = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let PostHistory { post, user_id, post_history_type_id, .. } = &db.post_history;
    type H = ((Id<Post>, Option<i64>), [i64; 2]);
    let cph = db.post_history.group_by(post.and(user_id.opt())).select(post_history_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 10) as i64, a[1] + (t == 11) as i64]);
    let cr = rel(drain(&cph));
    let cpi: HashIdx<Id<Post>, [i64; 2]> = (&cr).map(|x: H| x.0 .0).inv().select(&cr).map(|x: H| x.1).collect();
    type O = (R, ([i64; 2], [i64; 2]));
    let d: MatSet<O> = (&rp)
        .select(Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0 .1).select(&us).and(Same::<R>::new().map(|x: R| x.0 .0).select((&cpi).opt()).map(|c: Option<[i64; 2]>| c.unwrap_or([0, 0])))))
        .map(|x: O| x)
        .collect();
    let ratio = |a: [i64; 2]| if a[0] == 0 || a[1] == 0 { 0.0 } else { a[0] as f64 / a[1] as f64 };
    let v = top_n(drain(&d), |&(_, (((p, _), _), (a, c)))| (Reverse(fkey(ratio(a))), Reverse(score.get(p).unwrap()), p, c), 100);
    rows(v.into_iter().map(|(_, (((p, u), k), (a, c)))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title", "views", "score"]));
        f.extend([V::I(k), V::I(a[0]), V::I(a[1]), V::I(c[0]), V::I(c[1]), V::F(ratio(a))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, COALESCE(p.AcceptedAnswerId, -1) AS AcceptedAnswerId, p.LastActivityDate, p.Score, p.ViewCount,
//        LEAD(p.LastActivityDate) OVER (PARTITION BY p.OwnerUserId ORDER BY p.LastActivityDate) AS NextActivityDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS RankScore,
//        COUNT(ph.Id) AS EditCount, p.OwnerUserId
//     FROM Posts p LEFT JOIN PostHistory ph ON p.Id = ph.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.AcceptedAnswerId, p.LastActivityDate, p.Score, p.ViewCount, p.OwnerUserId),
// UserMetrics AS (SELECT u.Id AS UserId, u.Reputation, u.DisplayName, SUM(v.BountyAmount) AS TotalBounties, AVG(p.ViewCount) AS AvgViewCount, MAX(COALESCE(p.Score, 0)) AS MaxPostScore,
//        COUNT(DISTINCT p.Id) AS PostCount FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (8, 9)
//     WHERE u.Reputation > 100 AND (u.Location IS NOT NULL OR u.WebsiteUrl IS NOT NULL) GROUP BY u.Id, u.Reputation, u.DisplayName)
// SELECT u.UserId, u.DisplayName, u.TotalBounties, COALESCE(rp.PostId, -2) AS TopPostId, u.AvgViewCount, u.MaxPostScore,
//        CASE WHEN u.PostCount = 0 THEN 'No Posts' WHEN u.TotalBounties IS NULL THEN 'No Bounties' ELSE 'Active User' END AS UserStatus,
//        CASE WHEN rp.RankScore <= 3 THEN 'Top Contributor' ELSE 'Regular Contributor' END AS ContributorLevel
// FROM UserMetrics u LEFT JOIN RankedPosts rp ON u.UserId = rp.OwnerUserId WHERE u.Reputation BETWEEN 100 AND 5000
// ORDER BY u.TotalBounties DESC NULLS LAST, u.AvgViewCount DESC OFFSET 0 ROWS FETCH NEXT 10 ROWS ONLY;
//
// Only RankScore is read from RankedPosts, so its history join is not driven. RankScore breaks Score ties by post id. AvgViewCount averages over the
// posts x bounty-votes rows.
fn q22310(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, view_count, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(owner_user).select(Ident::<Post>::new().and(owner_user).and(score)).window(row_number, |((p, _), s)| (Reverse(s), p), asc);
    type R = ((Id<Post>, Id<User>), i64);
    let rp: MatSet<R> = (&w).map(|((pu, _), k)| (pu, k)).collect();
    let rpi: HashIdx<Id<User>, R> = (&rp).map(|x: R| x.0 .1).inv().select(&rp).collect();
    let users = db.user.with((&db.user.reputation).filt(|r| (100..=5000).contains(&r) && r > 100)).with((&db.user.location).or(&db.user.website_url));
    let bv = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt());
    let um = users.group_by(Ident::<User>::new()).select(posts_of(db).select(score.and(view_count.opt()).and(bv.opt())).opt()).fold((0i64, false, 0i64, 0i64, i64::MIN), |(b, any, ws, wn, m), x| match x {
        Some(((s, w), bb)) => {
            let bb = bb.flatten();
            (b + bb.unwrap_or(0), any || bb.is_some(), ws + w.unwrap_or(0), wn + w.is_some() as i64, m.max(s))
        }
        None => (b, any, ws, wn, m.max(0)),
    });
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let v = drain((&um).and(&pc).and((&rpi).opt()));
    let avgv = |a: (i64, bool, i64, i64, i64)| if a.3 == 0 { None } else { Some(a.2 as f64 / a.3 as f64) };
    let v = top_n(v, |&(u, ((a, _), r))| (!a.1, Reverse(if a.1 { a.0 } else { 0 }), avgv(a).is_none(), Reverse(avgv(a).map(fkey)), u, r.map(|r| r.0 .0)), 10);
    rows(v.into_iter().map(|(u, ((a, n), r))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.push(if a.1 { V::I(a.0) } else { V::Null });
        f.push(V::I(r.map_or(-2, |r| db.post.origid.get(r.0 .0).unwrap())));
        f.push(ofloat(avgv(a)));
        f.push(V::I(a.4));
        f.push(V::S(if n == 0 { "No Posts" } else if !a.1 { "No Bounties" } else { "Active User" }));
        f.push(V::S(if r.map_or(false, |r| r.1 <= 3) { "Top Contributor" } else { "Regular Contributor" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.PostTypeId, p.Score, p.ViewCount, p.CreationDate, U.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS rn, COALESCE(COUNT(c.Id) FILTER (WHERE c.Score > 0), 0) AS PositiveCommentCount
//     FROM Posts p LEFT JOIN Users U ON p.OwnerUserId = U.Id LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.PostTypeId, p.Score, p.ViewCount, p.CreationDate, U.DisplayName),
// PostHistoryStats AS (SELECT ph.PostId, COUNT(*) AS HistoryCount, MAX(CASE WHEN ph.PostHistoryTypeId = 10 THEN ph.CreationDate END) AS LastCloseDate,
//        MAX(CASE WHEN ph.PostHistoryTypeId = 11 THEN ph.CreationDate END) AS LastReopenDate FROM PostHistory ph GROUP BY ph.PostId),
// HighRankedPosts AS (SELECT rp.PostId, rp.Title, rp.OwnerDisplayName,
//        CASE WHEN phs.HistoryCount IS NULL THEN 'No History' WHEN phs.LastCloseDate IS NOT NULL AND phs.LastReopenDate IS NOT NULL AND phs.LastCloseDate > phs.LastReopenDate THEN 'Currently Closed'
//        WHEN phs.LastCloseDate IS NOT NULL THEN 'Closed' ELSE 'Open' END AS PostStatus, rp.PositiveCommentCount
//     FROM RankedPosts rp LEFT JOIN PostHistoryStats phs ON rp.PostId = phs.PostId WHERE rp.rn = 1)
// SELECT hp.PostId, hp.Title, hp.OwnerDisplayName, hp.PostStatus, hp.PositiveCommentCount,
//        (SELECT COUNT(DISTINCT v.Id) FROM Votes v WHERE v.PostId = hp.PostId AND v.VoteTypeId IN (2, 9)) AS TotalUpvotes
// FROM HighRankedPosts hp WHERE hp.PostStatus != 'No History' ORDER BY hp.PostStatus DESC, hp.PositiveCommentCount DESC;
//
// rn reads only base columns, so the newest post of each type is picked first (ties broken by post id).
fn q24792(db: &'static So) -> String {
    let Post { creation_date, post_type_id, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(post_type_id).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let pcc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.score).opt()).fold(0i64, |n, s| n + s.map_or(false, |s| s > 0) as i64);
    let PostHistory { post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let phs = (&tp).group_by(Ident::<Post>::new()).select(history_of(db).select(post_history_type_id.and(hd))).fold((i64::MIN, i64::MIN), |(c, r), (t, d)| {
        (if t == 10 { c.max(d) } else { c }, if t == 11 { r.max(d) } else { r })
    });
    let up = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([2, 9]))).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let v = drain((&pcc).and(&phs).and(&up));
    rows(v.into_iter().map(|(p, ((c, (lc, lr)), u))| {
        let st = if lc != i64::MIN && lr != i64::MIN && lc > lr { "Currently Closed" } else if lc != i64::MIN { "Closed" } else { "Open" };
        let mut f = post_fields(db, p, &["id", "title", "owner"]);
        f.extend([V::S(st), V::I(c), V::I(u)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, u.DisplayName AS OwnerDisplayName, p.CreationDate, p.Score, p.ViewCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopQuestions AS (SELECT rp.PostId, rp.Title, rp.OwnerDisplayName, rp.CreationDate, rp.Score, rp.ViewCount FROM RankedPosts rp WHERE rp.Rank <= 5 AND rp.PostId IN (SELECT PostId FROM Votes v WHERE v.VoteTypeId = 2)),
// TopAnswers AS (SELECT rp.PostId, rp.Title, rp.OwnerDisplayName, rp.CreationDate, rp.Score, rp.ViewCount FROM RankedPosts rp WHERE rp.Rank <= 5 AND rp.PostId IN (SELECT ParentId FROM Posts WHERE PostTypeId = 2)),
// TopPosts AS (SELECT tp.PostId, tp.Title, tp.OwnerDisplayName, 'Question' AS PostType, tp.CreationDate, tp.Score, tp.ViewCount FROM TopQuestions tp UNION ALL
//     SELECT ta.PostId, ta.Title, ta.OwnerDisplayName, 'Answer' AS PostType, ta.CreationDate, ta.Score, ta.ViewCount FROM TopAnswers ta)
// SELECT tp.PostId, tp.Title, tp.OwnerDisplayName, tp.PostType, tp.CreationDate, tp.Score, tp.ViewCount, COALESCE(c.CommentCount, 0) AS CommentCount, COALESCE(b.BadgeCount, 0) AS BadgeCount
// FROM TopPosts tp LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON tp.PostId = c.PostId
// LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId) b ON tp.OwnerDisplayName = (SELECT DisplayName FROM Users WHERE Id = b.UserId)
// ORDER BY tp.Score DESC, tp.ViewCount DESC;
//
// Rank reads only base columns, so the posts are picked first (ties broken by post id). The badge join is on DisplayName, so the per-user badge counts
// are indexed by their user's name; every user of that name contributes a row.
fn q7966(db: &'static So) -> String {
    let Post { creation_date, score, view_count, post_type_id, owner_user, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(row_number, |((p, s), w)| (Reverse(s), w.is_none(), Reverse(w), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|(((p, _), _), _)| p).collect();
    let up = Ident::<Post>::new().with(votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2))));
    let ans = Ident::<Post>::new().with(answers_of(db));
    let tq: MatSet<(Id<Post>, &'static str)> = (&tp).select(up).map(|p: Id<Post>| (p, "Question")).collect();
    let ta: MatSet<(Id<Post>, &'static str)> = (&tp).select(ans).map(|p: Id<Post>| (p, "Answer")).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db)).fold(0i64, |n, _| n + 1);
    let byname: HashIdx<Str, Id<User>> = db.user.with(&bc).select(&db.user.display_name).inv().collect();
    type T = (Id<Post>, &'static str);
    let side = || Same::<T>::new().and(Same::<T>::new().map(|x: T| x.0).select((&cc).opt().and(owner_user.select(&db.user.display_name).select(&byname).select(&bc).opt())));
    let v = drain((&tq).union(&ta).select(side()));
    rows(v.into_iter().map(|(_, ((p, t), (c, b)))| {
        let mut f = post_fields(db, p, &["id", "title", "owner"]);
        f.push(V::S(t));
        f.extend(post_fields(db, p, &["created", "score", "views"]));
        f.extend([V::I(c.unwrap_or(0)), V::I(b.unwrap_or(0))]);
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.CreationDate, COALESCE(SUM(CASE WHEN vote.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN vote.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COALESCE(COUNT(DISTINCT ph.PostId), 0) AS PostCount, MIN(u.CreationDate) OVER (PARTITION BY u.Id) AS FirstActivity
//     FROM Users u LEFT JOIN Votes vote ON u.Id = vote.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN PostHistory ph ON p.Id = ph.PostId GROUP BY u.Id, u.DisplayName, u.Reputation, u.CreationDate),
// ActivePosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, u.DisplayName AS OwnerDisplayName, EXTRACT(EPOCH FROM (TIMESTAMP '2024-10-01 12:34:56' - p.CreationDate)) / 3600 AS AgeInHours,
//        COUNT(c.Id) AS CommentCount FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, u.DisplayName),
// TrendingPosts AS (SELECT ap.PostId, ap.Title, ap.ViewCount, (ap.CommentCount * 0.5 + GREATEST(ap.ViewCount / NULLIF(EXTRACT(EPOCH FROM (TIMESTAMP '2024-10-01 12:34:56' - ap.CreationDate)) / 3600, 0), 1) * 0.5) AS EngagementScore
//     FROM ActivePosts ap)
// SELECT us.UserId, us.DisplayName, us.Reputation, us.PostCount, tp.Title, tp.ViewCount, tp.EngagementScore,
//        CASE WHEN us.FirstActivity IS NOT NULL AND us.FirstActivity < TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' THEN 'Veteran' WHEN us.Reputation < 100 THEN 'Newbie' ELSE 'Experienced' END AS UserTier
// FROM UserStatistics us LEFT JOIN TrendingPosts tp ON us.PostCount > 5 WHERE us.Reputation > 100 AND tp.EngagementScore > 1 ORDER BY tp.EngagementScore DESC, us.Reputation DESC LIMIT 10;
//
// The vote sums are never read, and COUNT(DISTINCT ph.PostId) is the user's posts that have history. The ON names only us, so it is a cross join;
// the WHERE drops the NULL row. GREATEST skips a NULL ViewCount. Only posts and users ranked <= 10 on their own sort key can reach the LIMIT, so each
// side is cut to RANK() <= 10 before the cross.
fn q24219(db: &'static So) -> String {
    let Post { creation_date, view_count, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let pc = db.user.with((&db.user.reputation).gt(100)).group_by(Ident::<User>::new()).select(posts_of(db).select(Ident::<Post>::new().with(history_of(db)))).fold(0i64, |n, _| n + 1);
    let uw = whole(db.user.with((&pc).filt(|n| n > 5))).select(Ident::<User>::new().and(&db.user.reputation).and(&pc)).window(rank, |((_, r), _)| Reverse(r), asc);
    let users: MatSet<(Id<User>, i64, i64)> = (&uw).filt(|(_, k)| k <= 10).map(|(((u, r), n), _)| (u, r, n)).collect();
    let cc = db.post.with(creation_date.ge(add_days(t0, -30))).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let es = move |d: i64, w: Option<i64>, c: i64| {
        let h = secs(t0 - d) / 3600.0;
        let g = w.and_then(|w| if h == 0.0 { None } else { Some(w as f64 / h) }).map_or(1.0, |x| x.max(1.0));
        c as f64 * 0.5 + g * 0.5
    };
    let tp = Ident::<Post>::new().and(creation_date).and(view_count.opt()).and(&cc).filt(move |(((_, d), w), c): (((Id<Post>, i64), Option<i64>), i64)| es(d, w, c) > 1.0);
    let pw = whole(db.post.with(&cc)).select(tp).window(rank, move |(((_, d), w), c)| Reverse(fkey(es(d, w, c))), asc);
    type T = (((Id<Post>, i64), Option<i64>), i64);
    let posts: MatSet<T> = (&pw).filt(|(_, k)| k <= 10).map(|(x, _)| x).collect();
    let v = top_n(drain((&posts).cross(&users)), |&(_, ((((p, d), w), c), (u, r, _)))| (Reverse(fkey(es(d, w, c))), Reverse(r), p, u), 10);
    rows(v.into_iter().map(|(_, ((((p, d), w), c), (u, _, n)))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.push(V::I(n));
        f.extend(post_fields(db, p, &["title", "views"]));
        f.push(V::F(es(d, w, c)));

        let r = db.user.reputation.get(u).unwrap();
        f.push(V::S(if db.user.creation_date.get(u).unwrap() < add_years(t0, -1) { "Veteran" } else if r < 100 { "Newbie" } else { "Experienced" }));
        row(f)
    }))
}

// WITH RecursiveUserVotes AS (SELECT U.Id AS UserId, COUNT(V.Id) AS TotalVotes, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id),
// TopUsers AS (SELECT UserId, TotalVotes, UpVotes, DownVotes, RANK() OVER (ORDER BY TotalVotes DESC) AS UserRank FROM RecursiveUserVotes WHERE TotalVotes > 0),
// PostStatistics AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, COALESCE(COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END), 0) AS CommentCount, COALESCE(COUNT(DISTINCT V.UserId), 0) AS VoteCount,
//        SUM(CASE WHEN PH.Comment IS NOT NULL THEN 1 ELSE 0 END) AS HistoryEditCount
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN PostHistory PH ON P.Id = PH.PostId WHERE P.ViewCount > 100 AND P.Score > 0
//     GROUP BY P.Id, P.Title, P.CreationDate, P.Score),
// RecentPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, PS.CommentCount, PS.VoteCount, PS.HistoryEditCount,
//        ROW_NUMBER() OVER (PARTITION BY EXTRACT(YEAR FROM P.CreationDate) ORDER BY P.CreationDate DESC) AS YearlyRanking
//     FROM Posts P JOIN PostStatistics PS ON P.Id = PS.PostId WHERE P.CreationDate >= CURRENT_DATE - INTERVAL '1 year')
// SELECT U.DisplayName AS TopUser, U.Reputation, RP.Title AS RecentPostTitle, RP.CreationDate AS RecentPostDate, RP.CommentCount, RP.VoteCount, RP.HistoryEditCount
// FROM TopUsers T JOIN Users U ON T.UserId = U.Id CROSS JOIN RecentPosts RP WHERE T.UserRank <= 5 AND RP.YearlyRanking <= 10 ORDER BY U.Reputation DESC, RP.VoteCount DESC;
//
// YearlyRanking reads only CreationDate, so the recent posts are picked first (ties broken by post id) and PostStatistics is driven for them alone.
fn q32493(db: &'static So) -> String {
    let Post { creation_date, view_count, score, .. } = &db.post;
    let tv = db.user.group_by(Ident::<User>::new()).select(votes_by(db)).fold(0i64, |n, _| n + 1);
    let uw = whole(db.user.with(&tv)).select(Ident::<User>::new().and(&tv)).window(rank, |(_, n)| Reverse(n), asc);
    let tu: MatSet<Id<User>> = (&uw).filt(|(_, k)| k <= 5).map(|((u, _), _)| u).collect();
    let since = add_years(current_date(), -1);
    let yw = db
        .post
        .with(view_count.gt(100).and(score.gt(0)))
        .with(creation_date.ge(since))
        .group_by(creation_date.map(|d: i64| year(d)))
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (Reverse(d), p), asc);
    let rp: MatSet<Id<Post>> = (&yw).filt(|(_, k)| k <= 10).map(|((p, _), _)| p).collect();
    let ps = (&rp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).opt()).and(history_of(db).select((&db.post_history.comment).opt()).opt()))
        .fold([0i64; 2], |a, ((c, _), h)| [a[0] + c.is_some() as i64, a[1] + h.flatten().is_some() as i64]);
    let vu = (&rp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.user_id)).count_distinct();
    let psr = rel(drain((&ps).and((&vu).opt())));
    let mut v = Vec::new();
    (&tu).cross(&psr).drive(|_, (u, (p, (a, n)))| v.push((u, p, (a[0], n.unwrap_or(0), a[1]))));
    rows(v.into_iter().map(|(u, p, (c, n, h))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["title", "created"]));
        f.extend([V::I(c), V::I(n), V::I(h)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.ViewCount IS NOT NULL THEN P.ViewCount ELSE 0 END) AS TotalViews,
//        SUM(CASE WHEN B.Id IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// PostDetail AS (SELECT P.Id AS PostId, P.Title, P.Score, P.CreationDate, P.LastActivityDate, COALESCE(V.VoteCount, 0) AS VoteCount, COALESCE(C.CommentCount, 0) AS CommentCount FROM Posts P
//     LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS VoteCount FROM Votes GROUP BY PostId) V ON P.Id = V.PostId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) C ON P.Id = C.PostId WHERE P.CreationDate >= CURRENT_DATE - INTERVAL '1 year' OR P.Score > 100),
// RankedPosts AS (SELECT PD.PostId, PD.Title, PD.Score, PD.CreationDate, PD.LastActivityDate, PD.VoteCount, PD.CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY CASE WHEN PD.Score > 50 THEN 'High Score' ELSE 'Low Score' END ORDER BY PD.Score DESC) AS PostRank FROM PostDetail PD)
// SELECT US.DisplayName, US.Reputation, US.PostCount, US.TotalViews, US.BadgeCount, RP.Title, RP.Score, RP.CreationDate, RP.LastActivityDate, RP.VoteCount, RP.CommentCount
// FROM UserStats US JOIN RankedPosts RP ON US.UserId = RP.PostId WHERE (US.Reputation > 1000 AND US.PostCount > 5) OR RP.PostRank <= 5 ORDER BY US.Reputation DESC, RP.Score DESC;
//
// PostRank breaks Score ties by post id (the SQL leaves them open). `US.UserId = RP.PostId` joins a user id to a post id, so it goes through the raw ids,
// and the posts x badges product is driven for those users alone.
fn q24555(db: &'static So) -> String {
    let Post { creation_date, score, view_count, .. } = &db.post;
    let since = add_years(current_date(), -1);
    let pd = || db.post.with(creation_date.ge(since).or(score.gt(100)));
    let w = pd().group_by(score.map(|s: i64| s > 50)).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    type R = ((Id<Post>, i64), i64);
    let rv: MatSet<R> = (&w).map(|x: R| x).collect();
    let by_id: HashIdx<i64, R> = (&rv).map(|x: R| x.0 .0).select(&db.post.origid).inv().collect();
    let users: MatSet<Id<User>> = db.user.with((&db.user.origid).select(&by_id)).collect();
    let us = (&users).group_by(Ident::<User>::new()).select(posts_of(db).select(view_count.opt()).opt().and(badges_of(db).opt())).fold([0i64; 2], |a, (w, b)| {
        [a[0] + w.flatten().unwrap_or(0), a[1] + b.is_some() as i64]
    });
    let pc = (&users).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let vc = pd().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold(0i64, |n, t| n + (t == 2) as i64);
    let cc = pd().group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    type X = ((Id<User>, i64), (([i64; 2], i64), R));
    let v = drain(
        (&users)
            .select(Ident::<User>::new().and(&db.user.reputation).and((&us).and(&pc).and((&db.user.origid).select(&by_id))))
            .filt(|((_, r), ((_, n), (_, k))): X| (r > 1000 && n > 5) || k <= 5)
            .select(Same::<X>::new().and(Same::<X>::new().map(|x: X| x.1 .1 .0 .0).select((&vc).opt().and((&cc).opt())))),
    );
    rows(v.into_iter().map(|(_, (((u, _), ((a, n), ((p, _), _))), (w, c)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1])]);
        f.extend(post_fields(db, p, &["title", "score", "created", "activity"]));
        f.extend([V::I(w.unwrap_or(0)), V::I(c.unwrap_or(0))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.OwnerUserId, p.CreationDate, p.ViewCount, p.Score, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS ScoreRank,
//        COUNT(c.Id) FILTER (WHERE c.UserId IS NOT NULL) AS CommentCount, COUNT(DISTINCT v.UserId) FILTER (WHERE v.VoteTypeId = 2) AS UpVoteCount,
//        COUNT(DISTINCT v.UserId) FILTER (WHERE v.VoteTypeId = 3) AS DownVoteCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.Title, p.OwnerUserId, p.CreationDate, p.ViewCount, p.Score, p.PostTypeId),
// RecentUsers AS (SELECT u.Id, u.DisplayName, CASE WHEN u.LastAccessDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days') THEN 'Active' ELSE 'Inactive' END AS UserStatus FROM Users u),
// PostHistoryCounts AS (SELECT ph.PostId, COUNT(*) AS EditCount FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (4, 5, 6) GROUP BY ph.PostId)
// SELECT rp.Title, ru.DisplayName AS Owner, rp.CreationDate, rp.ViewCount, rp.Score, rp.ScoreRank, COALESCE(rpc.EditCount, 0) AS EditCount, COALESCE(rp.CommentCount, 0) AS CommentCount,
//        COALESCE(rp.UpVoteCount, 0) AS UpVoteCount, COALESCE(rp.DownVoteCount, 0) AS DownVoteCount, COALESCE(DENSE_RANK() OVER (ORDER BY rp.ViewCount DESC), 0) AS ViewRank,
//        COALESCE(rp.UpVoteCount - rp.DownVoteCount, 0) AS NetVotes,
//        CASE WHEN rp.Score > 100 THEN 'Highly Popular' WHEN rp.Score BETWEEN 50 AND 100 THEN 'Moderately Popular' ELSE 'Less Popular' END AS PopularityTier
// FROM RankedPosts rp LEFT JOIN RecentUsers ru ON rp.OwnerUserId = ru.Id LEFT JOIN PostHistoryCounts rpc ON rp.Id = rpc.PostId
// WHERE rp.ScoreRank = 1 AND rp.ViewCount > 10 ORDER BY rp.CreationDate DESC LIMIT 100 OFFSET 0;
//
// ScoreRank reads only base columns, so the rank-1 posts are picked first. ViewRank ranks the rows left after the WHERE.
fn q920(db: &'static So) -> String {
    let Post { score, post_type_id, view_count, owner_user, creation_date, .. } = &db.post;
    let w = db.post.group_by(post_type_id).select(Ident::<Post>::new().and(score)).window(rank, |(_, s)| Reverse(s), asc);
    let sr = by_first(&(&w).filt(|(_, k)| k == 1).map(|((p, _), k)| (p, k)).collect());
    let tp: MatSet<Id<Post>> = db.post.with(&sr).with(view_count.gt(10)).collect();
    let cc = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).select((&db.comment.user_id).opt()).opt().and(votes_of(db).opt()))
        .fold(0i64, |n, (c, _)| n + c.flatten().is_some() as i64);
    let voters = |t: i64| (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(t))).select(&db.vote.user_id)).count_distinct();
    let (up, dn) = (voters(2), voters(3));
    let ec = (&tp).group_by(Ident::<Post>::new()).select(history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([4, 5, 6])))).fold(0i64, |n, _| n + 1);
    let vw = whole(&tp).select(Ident::<Post>::new().and(view_count)).window(dense_rank, |(_, w)| Reverse(w), asc);
    let vr = by_first(&(&vw).map(|((p, _), k)| (p, k)).collect());
    let v = drain((&cc).and((&up).opt()).and((&dn).opt()).and((&ec).opt()).and(owner_user.opt()).and(&vr).and(&sr));
    let v = top_n(v, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 100);
    rows(v.into_iter().map(|(p, ((((((c, u), d), e), o), k), r))| {
        let (u, d) = (u.unwrap_or(0), d.unwrap_or(0));
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["title"]);
        f.push(ostr(o.map(|o| db.user.display_name.get(o).unwrap())));
        f.extend(post_fields(db, p, &["created", "views", "score"]));
        f.extend([V::I(r), V::I(e.unwrap_or(0)), V::I(c), V::I(u), V::I(d), V::I(k), V::I(u - d)]);

        f.push(V::S(if s > 100 { "Highly Popular" } else if s >= 50 { "Moderately Popular" } else { "Less Popular" }));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.CreationDate, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COALESCE(SUM(CASE WHEN B.Id IS NOT NULL THEN 1 ELSE 0 END), 0) AS BadgeCount,
//        ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS Rank
//     FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName, U.Reputation, U.CreationDate),
// PostAnalysis AS (SELECT P.OwnerUserId, COUNT(DISTINCT P.Id) AS PostCount, SUM(P.Score) AS TotalScore, AVG(P.ViewCount) AS AverageViewCount, COUNT(DISTINCT PH.Id) AS EditCount,
//        COALESCE(SUM(CASE WHEN PH.PostHistoryTypeId = 10 THEN 1 ELSE 0 END), 0) AS ClosedPosts
//     FROM Posts P LEFT JOIN PostHistory PH ON P.Id = PH.PostId WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY P.OwnerUserId),
// UserPostStats AS (SELECT US.UserId, US.DisplayName, US.Reputation, UP.PostCount, UP.TotalScore, UP.AverageViewCount, UP.EditCount, UP.ClosedPosts FROM UserStats US
//     LEFT JOIN PostAnalysis UP ON US.UserId = UP.OwnerUserId)
// SELECT DISTINCT US.DisplayName, US.Reputation, UPS.PostCount, UPS.TotalScore, UPS.AverageViewCount, UPS.EditCount, UPS.ClosedPosts,
//        CASE WHEN UPS.Reputation >= 1000 THEN 'High' WHEN UPS.Reputation BETWEEN 500 AND 999 THEN 'Medium' ELSE 'Low' END AS ReputationCategory,
//        CASE WHEN UPS.PostCount IS NULL THEN 'No Posts' ELSE 'Has Posts' END AS PostStatus
// FROM UserStats US LEFT JOIN UserPostStats UPS ON US.UserId = UPS.UserId ORDER BY UPS.TotalScore DESC, US.Reputation DESC LIMIT 100 OFFSET 0;
//
// UserStats' aggregates are never read, so it is one row per user. The DISTINCT is taken in prela over the projected values.
fn q1512(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, view_count, .. } = &db.post;
    let pa1 = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(owner_user)
        .select(score.and(view_count.opt()).and(history_of(db).select(&db.post_history.post_history_type_id).opt()))
        .fold([0i64; 5], |a, ((s, w), t)| [a[0] + s, a[1] + w.unwrap_or(0), a[2] + w.is_some() as i64, a[3] + t.is_some() as i64, a[4] + (t == Some(10)) as i64]);
    let pa2 = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(owner_user).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    type O = (Str, i64, Option<(i64, [i64; 5])>);
    let d: MatSet<O> = db
        .user
        .select(Ident::<User>::new().and((&pa2).and(&pa1).opt()))
        .map(|(u, p): (Id<User>, Option<(i64, [i64; 5])>)| (db.user.display_name.get(u).unwrap(), db.user.reputation.get(u).unwrap(), p))
        .collect();
    let v = top_n(drain(&d), |&((n, r, p), _)| (p.is_none(), Reverse(p.map(|p| p.1[0])), Reverse(r), n, p), 100);
    rows(v.into_iter().map(|((n, r, p), _)| {
        let mut f = vec![V::S(n), V::I(r)];
        match p {
            Some((c, a)) => {
                f.extend([V::I(c), V::I(a[0]), if a[2] == 0 { V::Null } else { V::F(a[1] as f64 / a[2] as f64) }, V::I(a[3]), V::I(a[4])]);
                f.push(V::S(if r >= 1000 { "High" } else if (500..=999).contains(&r) { "Medium" } else { "Low" }));
                f.push(V::S("Has Posts"));
            }
            None => {
                f.extend([V::Null, V::Null, V::Null, V::Null, V::Null]);
                f.push(V::S(if r >= 1000 { "High" } else if (500..=999).contains(&r) { "Medium" } else { "Low" }));
                f.push(V::S("No Posts"));
            }
        }
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COALESCE(SUM(CASE WHEN V.VoteTypeId IN (2, 3) THEN 1 ELSE 0 END), 0) AS VoteCount,
//        COALESCE(SUM(CASE WHEN P.ViewCount IS NOT NULL THEN P.ViewCount ELSE 0 END), 0) AS TotalViews, COALESCE(SUM(CASE WHEN C.Id IS NOT NULL THEN 1 ELSE 0 END), 0) AS CommentCount,
//        COUNT(DISTINCT P.Id) AS PostCount
//     FROM Users U LEFT JOIN Posts P ON P.OwnerUserId = U.Id LEFT JOIN Votes V ON V.PostId = P.Id LEFT JOIN Comments C ON C.UserId = U.Id GROUP BY U.Id, U.DisplayName),
// PostStatistics AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.ViewCount, P.Score, P.AnswerCount, P.CommentCount, P.ClosedDate,
//        ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS RowNum, RANK() OVER (ORDER BY P.Score DESC) AS ScoreRank
//     FROM Posts P WHERE P.CreationDate >= CURRENT_TIMESTAMP - INTERVAL '1 year' AND P.Score IS NOT NULL),
// ClosedPosts AS (SELECT PH.PostId, PH.CreationDate, CTR.Name AS CloseReason FROM PostHistory PH JOIN CloseReasonTypes CTR ON PH.Comment IS NOT NULL AND PH.PostHistoryTypeId = 10
//     WHERE PH.CreationDate >= CURRENT_TIMESTAMP - INTERVAL '1 year')
// SELECT UA.UserId, UA.DisplayName, UA.VoteCount, UA.TotalViews, UA.CommentCount, UA.PostCount, PS.PostId, PS.Title, PS.CreationDate AS PostCreationDate, PS.ViewCount AS PostViewCount,
//        PS.Score AS PostScore, PS.AnswerCount, PS.CommentCount AS PostCommentCount, (SELECT COUNT(*) FROM ClosedPosts CP WHERE CP.PostId = PS.PostId) AS CloseCount, CP.CloseReason
// FROM UserActivity UA LEFT JOIN PostStatistics PS ON UA.UserId = PS.RowNum LEFT JOIN ClosedPosts CP ON PS.PostId = CP.PostId WHERE UA.PostCount > 0
// ORDER BY UA.TotalViews DESC, UA.VoteCount DESC LIMIT 50;
//
// The comments join on the user, not the post, so every user's posts x votes rows are crossed with their comments. `UA.UserId = PS.RowNum` joins a user
// id to a row number, raw. The ClosedPosts ON names only PH, so each close event is crossed with every close reason. CURRENT_TIMESTAMP is TIMESTAMPTZ,
// so the dates are read as New York local time. Every user yields at least one row, so the 50 users are cut before the PS join.
fn q23823(db: &'static So) -> String {
    let Post { creation_date, view_count, owner_user_id, .. } = &db.post;
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(view_count.opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(comments_by(db).opt()))
        .fold([0i64; 3], |a, (p, c)| {
            let (w, t) = p.map_or((None, None), |(w, t)| (w, t));
            [a[0] + matches!(t, Some(2 | 3)) as i64, a[1] + w.unwrap_or(0), a[2] + c.is_some() as i64]
        });
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let uw = whole(db.user.with(&pc)).select(Ident::<User>::new().and((&ua).and(&pc))).window(rank, |(_, (a, _))| (Reverse(a[1]), Reverse(a[0])), asc);
    type T = (Id<User>, ([i64; 3], i64));
    let tv: MatSet<T> = (&uw).filt(|(_, k)| k <= 50).map(|(x, _)| x).collect();
    let since = now_utc() - 365 * DAY_US;
    let w = db
        .post
        .with(creation_date.filt(move |d| ny_to_utc(d) >= since))
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (Reverse(d), p), asc);
    let by_rn = by_first(&(&w).map(|((p, _), k)| (k, p)).collect());
    let PostHistory { post_history_type_id, comment, creation_date: hd, .. } = &db.post_history;
    let cps = db.post_history.with(post_history_type_id.eq(10)).with(comment).with(hd.filt(move |d| ny_to_utc(d) >= since));
    let cp = cps.select(&db.post_history.post).cross(&db.close_reason_type.name);
    let cpi: HashIdx<Id<Post>, Str> = (&cp).map(|(p, _): (Id<Post>, Str)| p).inv().select(&cp).map(|(_, n): (Id<Post>, Str)| n).collect();
    let cpc = db.post.with(&cpi).group_by(Ident::<Post>::new()).select(&cpi).fold(0i64, |n, _| n + 1);
    let v = drain((&tv).select(Same::<T>::new().and(Same::<T>::new().map(|x: T| x.0).select(&db.user.origid).select(&by_rn).select(Ident::<Post>::new().and((&cpc).opt()).and((&cpi).opt())).opt())));
    let v = top_n(v, |&(_, ((u, (a, _)), p))| (Reverse(a[1]), Reverse(a[0]), u, p.map(|x| x.0 .0)), 50);
    rows(v.into_iter().map(|(_, ((u, (a, n)), p))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(n)]);
        match p {
            Some(((p, k), r)) => {
                f.extend(post_fields(db, p, &["id", "title", "created", "views", "score", "answers", "comments"]));
                f.extend([V::I(k.unwrap_or(0)), ostr(r)]);
            }
            None => f.extend([V::Null, V::Null, V::Null, V::Null, V::Null, V::Null, V::Null, V::I(0), V::Null]),
        }
        row(f)
    }))
}

// WITH PostDetails AS (SELECT p.Id AS PostId, p.Title, p.PostTypeId, p.CreationDate, p.Score, p.ViewCount, COALESCE(u.DisplayName, 'Community User') AS OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes, COUNT(DISTINCT b.Id) AS BadgeCount
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId
//     WHERE p.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year') GROUP BY p.Id, u.DisplayName, p.Title, p.PostTypeId, p.CreationDate, p.Score, p.ViewCount),
// RankedPosts AS (SELECT pd.PostId, pd.Title, pd.OwnerDisplayName, pd.CreationDate, pd.Score, pd.ViewCount, pd.CommentCount, pd.Upvotes, pd.Downvotes, RANK() OVER (ORDER BY pd.Score DESC, pd.ViewCount DESC) AS Rank
//     FROM PostDetails pd),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.OwnerDisplayName, rp.Rank FROM RankedPosts rp WHERE rp.Rank <= 10),
// PostHistoryCounts AS (SELECT ph.PostId, COUNT(ph.Id) AS HistoryCount FROM PostHistory ph GROUP BY ph.PostId),
// PostWithHistory AS (SELECT tp.*, COALESCE(phc.HistoryCount, 0) AS HistoryCount FROM TopPosts tp LEFT JOIN PostHistoryCounts phc ON tp.PostId = phc.PostId)
// SELECT pwh.Title, pwh.OwnerDisplayName, pwh.Rank, pwh.HistoryCount, CASE WHEN pwh.HistoryCount > 5 THEN 'Frequent Updates' WHEN pwh.HistoryCount = 0 THEN 'No History' ELSE 'Moderate Updates' END AS UpdateFrequency,
//        CONCAT('Post ', pwh.Title, ' (Rank: ', pwh.Rank, ') - Updated ', pwh.HistoryCount, ' times') AS Summary
// FROM PostWithHistory pwh WHERE pwh.OwnerDisplayName IS NOT NULL ORDER BY pwh.Rank;
//
// Only Rank and base columns reach the output, so PostDetails' aggregates are never computed and the ten posts are picked first. CONCAT reads a NULL
// Title as ''.
fn q21441(db: &'static So) -> String {
    let Post { creation_date, score, view_count, owner_user, title, .. } = &db.post;
    let w = whole(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))))
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(rank, |((_, s), w)| (Reverse(s), w.is_none(), Reverse(w)), asc);
    type R = (Id<Post>, i64);
    let rv: MatSet<R> = (&w).filt(|(_, k)| k <= 10).map(|(((p, _), _), k)| (p, k)).collect();
    let hc = db.post_history.group_by(&db.post_history.post).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&rv).select(Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0).select((&hc).opt().and(owner_user.opt())))));
    rows(v.into_iter().map(|(_, ((p, k), (h, u)))| {
        let h = h.unwrap_or(0);
        let mut f = post_fields(db, p, &["title"]);
        f.push(V::S(u.map_or("Community User", |u| db.user.display_name.get(u).unwrap())));
        f.extend([V::I(k), V::I(h)]);
        f.push(V::S(if h > 5 { "Frequent Updates" } else if h == 0 { "No History" } else { "Moderate Updates" }));
        f.push(V::Owned(format!("Post {} (Rank: {k}) - Updated {h} times", title.get(p).unwrap_or(""))));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U WHERE U.Reputation IS NOT NULL),
// PostSummary AS (SELECT P.Id AS PostId, P.PostTypeId, P.Score, COALESCE(P.AcceptedAnswerId, 0) AS AcceptedAnswer, COUNT(DISTINCT C.Id) AS CommentCount, COUNT(DISTINCT V.Id) AS VoteCount,
//        CASE WHEN P.PostTypeId = 1 THEN 'Question' WHEN P.PostTypeId = 2 THEN 'Answer' ELSE 'Other' END AS PostType
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY P.Id, P.PostTypeId, P.Score, P.AcceptedAnswerId),
// PostActivity AS (SELECT PS.PostId, PS.PostType, SUM(CASE WHEN PS.PostType = 'Question' THEN 1 ELSE 0 END) AS TotalQuestions, SUM(CASE WHEN PS.PostType = 'Answer' THEN 1 ELSE 0 END) AS TotalAnswers,
//        AVG(PS.Score) AS AverageScore FROM PostSummary PS GROUP BY PS.PostId, PS.PostType),
// RecentPostHistory AS (SELECT PH.PostId, PH.UserId, PH.CreationDate, PHT.Name AS HistoryType, PH.Comment FROM PostHistory PH JOIN PostHistoryTypes PHT ON PH.PostHistoryTypeId = PHT.Id
//     WHERE PH.CreationDate >= cast('2024-10-01' as date) - interval '30 days'),
// UserPosts AS (SELECT U.UserId, COUNT(P.Id) AS PostsCreated, SUM(COALESCE(P.Score, 0)) AS TotalScore FROM UserReputation U LEFT JOIN Posts P ON U.UserId = P.OwnerUserId GROUP BY U.UserId)
// SELECT UR.DisplayName, UR.Reputation, APR.TotalQuestions, APR.TotalAnswers, APR.AverageScore, UP.PostsCreated, UP.TotalScore, RPH.HistoryType, RPH.Comment
// FROM UserReputation UR JOIN PostActivity APR ON UR.UserId = APR.PostId JOIN UserPosts UP ON UR.UserId = UP.UserId LEFT JOIN RecentPostHistory RPH ON UR.UserId = RPH.UserId
// ORDER BY UR.Reputation DESC, APR.AverageScore DESC;
//
// ReputationRank and PostSummary's counts are never read; PostActivity has one row per post. `UR.UserId = APR.PostId` joins a user id to a post id, so it
// goes through the raw ids.
fn q3608(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let up = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(score).opt()).fold([0i64; 2], |a, s| match s {
        Some(s) => [a[0] + 1, a[1] + s],
        None => a,
    });
    let PostHistory { creation_date: hd, comment, .. } = &db.post_history;
    let hidx: HashIdx<Id<User>, Id<PostHistory>> = db.post_history.with(hd.ge(add_days(date(2024, 10, 1), -30))).with(htype_name(db)).select(&db.post_history.user).inv().collect();
    let v = drain((&db.user.origid).select(&pidx).and(&up).and((&hidx).opt()));
    rows(v.into_iter().map(|(u, ((p, a), h))| {
        let t = post_type_id.get(p).unwrap();
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I((t == 1) as i64), V::I((t == 2) as i64), V::F(score.get(p).unwrap() as f64), V::I(a[0]), V::I(a[1])]);
        f.extend(match h {
            Some(h) => [V::S(htype_name(db).get(h).unwrap()), ostr(comment.get(h))],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserVoteStatistics AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotesCount, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotesCount,
//        COUNT(CASE WHEN v.VoteTypeId = 6 THEN 1 END) AS CloseVotesCount, COUNT(CASE WHEN v.VoteTypeId = 11 THEN 1 END) AS UndeleteVotesCount, SUM(COALESCE(b.Class, 0)) AS TotalBadgeClass,
//        RANK() OVER (ORDER BY COUNT(v.Id) DESC) AS VoteRank FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PostDetails AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, COALESCE((SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id), 0) AS CommentCount,
//        COALESCE((SELECT COUNT(*) FROM Posts a WHERE a.ParentId = p.Id), 0) AS AnswerCount, MAX(CASE WHEN ph.PostHistoryTypeId = 10 THEN ph.CreationDate END) AS ClosedDate,
//        MAX(CASE WHEN ph.PostHistoryTypeId = 11 THEN ph.CreationDate END) AS ReopenedDate FROM Posts p LEFT JOIN PostHistory ph ON p.Id = ph.PostId GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount),
// RankedPosts AS (SELECT pd.*, ROW_NUMBER() OVER (ORDER BY pd.ViewCount DESC) AS ViewRank FROM PostDetails pd)
// SELECT uvs.UserId, uvs.DisplayName, uvs.UpVotesCount, uvs.DownVotesCount, uvs.CloseVotesCount, uvs.UndeleteVotesCount, rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.CommentCount,
//        rp.AnswerCount, rp.ClosedDate, rp.ReopenedDate, CASE WHEN rp.ClosedDate IS NOT NULL THEN 'Closed' WHEN rp.ReopenedDate IS NOT NULL THEN 'Reopened' ELSE 'Open' END AS PostStatus
// FROM UserVoteStatistics uvs JOIN RankedPosts rp ON uvs.UserId IN (SELECT DISTINCT p.OwnerUserId FROM Posts p WHERE p.CreationDate < NOW() - INTERVAL '30 days')
// WHERE uvs.VoteRank <= 10 ORDER BY uvs.UpVotesCount - uvs.DownVotesCount DESC, rp.ViewCount DESC;
//
// ViewRank is never read. The ON names only uvs, so the ranked users who own an old enough post are crossed with every post. NOW() is TIMESTAMPTZ, so
// CreationDate is read as New York local time. The vote counts run over the votes x badges rows.
fn q22389(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let uvs = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt().and(badges_of(db).opt())).fold([0i64; 5], |a, (t, _)| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64, a[3] + (t == 6) as i64, a[4] + (t == 11) as i64],
        None => a,
    });
    let w = whole(db.user.with(&uvs)).select(Ident::<User>::new().and(&uvs)).window(rank, |(_, a)| Reverse(a[0]), asc);
    type R = (Id<User>, [i64; 5]);
    let since = now_utc() - 30 * DAY_US;
    let old: MatSet<Id<User>> = db.post.with(creation_date.filt(move |d| ny_to_utc(d) < since)).select(owner_user).collect();
    let top: MatSet<R> = (&w).filt(|(_, k)| k <= 10).map(|(x, _)| x).collect();
    let users: MatSet<R> = (&top).select(Same::<R>::new().with(Same::<R>::new().map(|x: R| x.0).select(&old))).collect();
    let PostHistory { post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ac = db.post.group_by(Ident::<Post>::new()).select(children_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let hm = db.post.group_by(Ident::<Post>::new()).select(history_of(db).select(post_history_type_id.and(hd)).opt()).fold((i64::MIN, i64::MIN), |(c, r), h| match h {
        Some((t, d)) => (if t == 10 { c.max(d) } else { c }, if t == 11 { r.max(d) } else { r }),
        None => (c, r),
    });
    let pd = rel(drain((&cc).and(&ac).and(&hm)));
    let mut v = Vec::new();
    (&users).cross(&pd).drive(|_, ((u, a), (p, ((c, n), (cl, ro))))| v.push((u, a, p, c, n, cl, ro)));
    rows(v.into_iter().map(|(u, a, p, c, n, cl, ro)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4])]);
        f.extend(post_fields(db, p, &["id", "title", "created", "views"]));
        f.extend([V::I(c), V::I(n), tmax(cl), tmax(ro)]);
        f.push(V::S(if cl != i64::MIN { "Closed" } else if ro != i64::MIN { "Reopened" } else { "Open" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'),
// PostsWithVotes AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COUNT(DISTINCT CASE WHEN v.UserId IS NOT NULL THEN v.UserId END) AS UniqueVoterCount
//     FROM RankedPosts rp LEFT JOIN Votes v ON rp.PostId = v.PostId GROUP BY rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score),
// PostsWithComments AS (SELECT pwv.PostId, pwv.Title, pwv.CreationDate, pwv.ViewCount, pwv.Score, pwv.UpVotes, pwv.DownVotes, pwv.UniqueVoterCount, COALESCE(COUNT(c.Id), 0) AS CommentCount
//     FROM PostsWithVotes pwv LEFT JOIN Comments c ON pwv.PostId = c.PostId GROUP BY pwv.PostId, pwv.Title, pwv.CreationDate, pwv.ViewCount, pwv.Score, pwv.UpVotes, pwv.DownVotes, pwv.UniqueVoterCount),
// ClosedPosts AS (SELECT DISTINCT p.Id AS ClosedPostId, ph.Comment AS CloseReason FROM Posts p JOIN PostHistory ph ON p.Id = ph.PostId WHERE ph.PostHistoryTypeId = 10 AND ph.Comment IS NOT NULL)
// SELECT pwc.PostId, pwc.Title, pwc.CreationDate, pwc.ViewCount, pwc.Score, pwc.UpVotes, pwc.DownVotes, pwc.UniqueVoterCount, pwc.CommentCount, cp.CloseReason
// FROM PostsWithComments pwc LEFT JOIN ClosedPosts cp ON pwc.PostId = cp.ClosedPostId WHERE (pwc.CommentCount > 0 OR cp.CloseReason IS NOT NULL)
// ORDER BY pwc.Score DESC, pwc.CreationDate ASC LIMIT 100;
//
// Rank is never read. ClosedPosts' DISTINCT is a set of (post, comment) pairs, built in prela.
fn q24801(db: &'static So) -> String {
    let Post { creation_date, score, .. } = &db.post;
    let rp = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let pv = rp().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold((0i64, 0i64), |(u, d), t| (u + (t == Some(2)) as i64, d + (t == Some(3)) as i64));
    let uv = rp().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.user_id)).count_distinct();
    let cc = rp().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let PostHistory { post_history_type_id, comment, post, .. } = &db.post_history;
    let cps: MatSet<(Id<Post>, Str)> = db.post_history.with(post_history_type_id.eq(10)).select(post.and(comment)).collect();
    type C = (Id<Post>, Str);
    let cpi: HashIdx<Id<Post>, Str> = (&cps).map(|x: C| x.0).inv().select(&cps).map(|x: C| x.1).collect();
    type X = (Id<Post>, ((((i64, i64), Option<i64>), i64), Option<Str>));
    let v = drain(rp().select(Ident::<Post>::new().and((&pv).and((&uv).opt()).and(&cc).and((&cpi).opt()))).filt(|(_, ((_, c), r)): X| c > 0 || r.is_some()));
    let v = top_n(v, |&(_, (p, (_, r)))| (Reverse(score.get(p).unwrap()), creation_date.get(p).unwrap(), p, r), 100);
    rows(v.into_iter().map(|(_, (p, ((((u, d), n), c), r)))| {
        let n = n.unwrap_or(0);
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([V::I(u), V::I(d), V::I(n), V::I(c), ostr(r)]);
        row(f)
    }))
}

// WITH UserScores AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 WHEN v.VoteTypeId = 3 THEN -1 ELSE 0 END), 0) AS Score,
//        COUNT(DISTINCT p.Id) AS TotalPosts, COUNT(DISTINCT b.Id) AS TotalBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.UserId = u.Id LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, Score, TotalPosts, TotalBadges, RANK() OVER (ORDER BY Score DESC, Reputation DESC) AS UserRank FROM UserScores),
// PostStatistics AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.AnswerCount, COALESCE(AVG(CASE WHEN c.UserId IS NOT NULL THEN c.Score END), 0) AS AverageCommentScore,
//        COUNT(DISTINCT c.Id) AS TotalComments FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate > CURRENT_TIMESTAMP - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.AnswerCount),
// RecentActivities AS (SELECT ph.PostId, ph.UserId, ph.PostHistoryTypeId, COUNT(*) AS ChangeCount FROM PostHistory ph WHERE ph.CreationDate > CURRENT_TIMESTAMP - INTERVAL '30 days'
//     GROUP BY ph.PostId, ph.UserId, ph.PostHistoryTypeId)
// SELECT tu.UserId, tu.DisplayName, tu.Reputation, tu.Score AS UserScore, ps.PostId, ps.Title, ps.ViewCount, ps.AnswerCount, ps.AverageCommentScore, ps.TotalComments, ra.ChangeCount AS RecentChanges
// FROM TopUsers tu LEFT JOIN PostStatistics ps ON tu.UserId = ps.PostId LEFT JOIN RecentActivities ra ON ps.PostId = ra.PostId AND tu.UserId = ra.UserId
// WHERE tu.UserRank <= 10 ORDER BY tu.UserRank, ps.ViewCount DESC;
//
// The vote join also asks v.UserId = u.Id, so it is driven over (user, post) pairs and filtered on the vote's voter. `tu.UserId = ps.PostId` joins a user
// id to a post id, so it goes through the raw ids. CURRENT_TIMESTAMP is TIMESTAMPTZ, so the dates are read as New York local time.
fn q3447(db: &'static So) -> String {
    type X = (Id<User>, Option<Id<Post>>);
    let pairs = rel(drain(db.user.select(Ident::<User>::new().and(posts_of(db).opt()))).into_iter().map(|x| x.1).collect::<Vec<X>>());
    type VU = (Id<Vote>, Id<User>);
    let own = Same::<X>::new()
        .and(Same::<X>::new().flat_map(|x: X| x.1).select(votes_of(db)).select(Ident::<Vote>::new().and(&db.vote.user)).select(Same::<VU>::new().and(Same::<VU>::new().map(|x: VU| x.0).select(&db.vote.vote_type_id))))
        .filt(|(x, ((_, u), _)): (X, (VU, i64))| u == x.0)
        .map(|(_, (_, t)): (X, (VU, i64))| t);
    let us = (&pairs).group_by(Same::<X>::new().map(|x: X| x.0)).select(own.opt().and(Same::<X>::new().map(|x: X| x.0).select(badges_of(db)).opt())).fold(0i64, |s, (t, _)| {
        s + match t {
            Some(2) => 1,
            Some(3) => -1,
            _ => 0,
        }
    });
    let uw = whole(db.user.with(&us)).select(Ident::<User>::new().and(&us).and(&db.user.reputation)).window(rank, |((_, s), r)| (Reverse(s), Reverse(r)), asc);
    type R = ((Id<User>, i64), i64);
    let tu: MatSet<R> = (&uw).filt(|(_, k)| k <= 10).map(|(((u, s), _), k)| ((u, s), k)).collect();
    let since = now_utc() - 365 * DAY_US;
    let Post { creation_date, .. } = &db.post;
    let ps = db.post.with(creation_date.filt(move |d| ny_to_utc(d) > since)).group_by(Ident::<Post>::new()).select(comments_of(db).select((&db.comment.user_id).opt().and(&db.comment.score)).opt()).fold([0i64; 3], |a, c| match c {
        Some((u, s)) => [a[0] + u.is_some() as i64 * s, a[1] + u.is_some() as i64, a[2] + 1],
        None => a,
    });
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let since30 = now_utc() - 30 * DAY_US;
    let PostHistory { post, user, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    type H = (((Id<Post>, Id<User>), i64), i64);
    let ra = db.post_history.with(hd.filt(move |d| ny_to_utc(d) > since30)).group_by(post.and(user).and(post_history_type_id)).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let rar = rel(drain(&ra));
    let rai: HashIdx<(Id<Post>, Id<User>), i64> = (&rar).map(|x: H| x.0 .0).inv().select(&rar).map(|x: H| x.1).collect();
    type Y = (Id<User>, (Id<Post>, [i64; 3]));
    let pj = Same::<R>::new()
        .map(|x: R| x.0 .0)
        .select(Ident::<User>::new().and((&db.user.origid).select(&pidx).select(Ident::<Post>::new().and(&ps))))
        .select(Same::<Y>::new().and(Same::<Y>::new().map(|(u, (p, _)): Y| (p, u)).select(&rai).opt()));
    let v = drain((&tu).select(Same::<R>::new().and(pj.opt())));
    let v = top_n(v, |&(_, ((_, k), p))| { let w = p.and_then(|((_, (p, _)), _)| db.post.view_count.get(p)); (k, w.is_none(), Reverse(w)) }, 0);
    rows(v.into_iter().map(|(_, (((u, s), _), x))| {
        let (p, c) = match x {
            Some((y, c)) => (Some(y), c),
            None => (None, None),
        };
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.push(V::I(s));
        match p {
            Some((_, (p, a))) => {
                f.extend(post_fields(db, p, &["id", "title", "views", "answers"]));
                f.extend([V::F(if a[1] == 0 { 0.0 } else { a[0] as f64 / a[1] as f64 }), V::I(a[2])]);
            }
            None => f.extend([V::Null, V::Null, V::Null, V::Null, V::Null, V::Null]),
        }
        f.push(oint(c));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, p.Score, COUNT(DISTINCT c.Id) AS CommentCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.PostTypeId IN (1, 2)
//     GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.Score),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.OwnerUserId, rp.Score, rp.CommentCount FROM RankedPosts rp WHERE rp.Rank = 1),
// UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.Views, u.UpVotes, u.DownVotes, RANK() OVER (ORDER BY u.Reputation DESC) AS ReputationRank FROM Users u WHERE u.Reputation > 1000),
// CommentsStatistics AS (SELECT p.Id AS PostId, AVG(c.Score) AS AvgCommentScore, COUNT(c.Id) AS TotalComments FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY p.Id)
// SELECT fp.PostId, fp.Title, fp.CreationDate, ur.DisplayName AS OwnerDisplayName, COALESCE(ur.Reputation, 0) AS OwnerReputation, COALESCE(cs.AvgCommentScore, 0) AS AvgCommentScore, cs.TotalComments,
//        CASE WHEN ur.Reputation IS NULL THEN 'Unknown' ELSE CASE WHEN ur.Reputation < 2000 THEN 'New Contributor' WHEN ur.Reputation < 5000 THEN 'Regular Contributor' ELSE 'Top Contributor' END END AS ContributorLevel
// FROM FilteredPosts fp LEFT JOIN UserReputation ur ON fp.OwnerUserId = ur.UserId LEFT JOIN CommentsStatistics cs ON fp.PostId = cs.PostId
// WHERE (fp.CommentCount > 0 OR ur.Reputation IS NOT NULL) ORDER BY fp.CreationDate DESC LIMIT 100;
//
// Rank reads only base columns, so the newest post per owner is picked first (the NULL owner is its own partition; ties broken by post id).
fn q20590(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user_id, owner_user, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(post_type_id.is_in([1, 2]))
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (Reverse(d), p), asc);
    let fp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k == 1).map(|((p, _), _)| p).collect();
    let cs = (&fp).group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.score).opt()).fold([0i64; 2], |a, s| match s {
        Some(s) => [a[0] + s, a[1] + 1],
        None => a,
    });
    let ur = owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(1000)));
    type X = (Id<Post>, ([i64; 2], Option<Id<User>>));
    let v = drain((&cs).and(ur.opt()).filt(|(a, u): ([i64; 2], Option<Id<User>>)| a[1] > 0 || u.is_some()));
    let v: Vec<X> = v;
    let v = top_n(v, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 100);
    rows(v.into_iter().map(|(p, (a, u))| {
        let r = u.map(|u| db.user.reputation.get(u).unwrap());
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.push(ostr(u.map(|u| db.user.display_name.get(u).unwrap())));
        f.push(V::I(r.unwrap_or(0)));
        f.push(V::F(if a[1] == 0 { 0.0 } else { a[0] as f64 / a[1] as f64 }));
        f.push(V::I(a[1]));
        f.push(V::S(match r {
            None => "Unknown",
            Some(r) if r < 2000 => "New Contributor",
            Some(r) if r < 5000 => "Regular Contributor",
            _ => "Top Contributor",
        }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, RANK() OVER (PARTITION BY pt.Name ORDER BY p.Score DESC) AS ScoreRank,
//        DENSE_RANK() OVER (PARTITION BY pt.Name ORDER BY p.ViewCount DESC) AS ViewCountRank
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.Score IS NOT NULL),
// UserInteractions AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT v.PostId) AS VoteCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount, COUNT(DISTINCT b.Id) AS BadgeCount FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PostStatistics AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, COALESCE(ui.VoteCount, 0) AS TotalVotes, ui.UpvoteCount, ui.DownvoteCount,
//        CASE WHEN ui.UpvoteCount IS NULL AND ui.DownvoteCount IS NULL THEN 'No Votes' ELSE 'Votes Present' END AS VoteStatus, rp.ScoreRank, rp.ViewCountRank
//     FROM RankedPosts rp LEFT JOIN UserInteractions ui ON rp.OwnerUserId = ui.UserId)
// SELECT ps.PostId, ps.Title, ps.CreationDate, ps.Score, ps.ViewCount, ps.TotalVotes, ps.UpvoteCount, ps.DownvoteCount, ps.VoteStatus,
//        CASE WHEN ps.ScoreRank <= 5 THEN 'Top 5 Posts' WHEN ps.ViewCountRank <= 5 THEN 'Top 5 Viewed Posts' ELSE 'Regular Post' END AS PostCategory
// FROM PostStatistics ps WHERE ps.Score > (SELECT AVG(Score) FROM Posts WHERE Score IS NOT NULL) AND ps.VoteStatus = 'Votes Present' ORDER BY ps.Score DESC, ps.ViewCount DESC FETCH FIRST 10 ROWS ONLY;
//
// UpvoteCount is a SUM over a LEFT JOIN, so it is NULL only when the owner has no UserInteractions row: VoteStatus is 'Votes Present' exactly when the
// owner exists. The vote sums run over the votes x badges rows.
fn q22348(db: &'static So) -> String {
    let Post { creation_date, score, view_count, owner_user, .. } = &db.post;
    let w1 = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(ptype_name(db))
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(rank, |((_, s), _)| Reverse(s), asc);
    let w2 = (&w1).window(dense_rank, |(((_, _), w), _)| (w.is_none(), Reverse(w)), asc);
    type R = ((((Id<Post>, i64), Option<i64>), i64), i64);
    let (s, n) = score.fold_flat((0i64, 0i64), |(s, n), x| (s + x, n + 1));
    let mean = s as f64 / n as f64;
    let ui = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt().and(badges_of(db).opt())).fold([0i64; 2], |a, (t, _)| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
    });
    let uvc = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.post_id)).count_distinct();
    let rv: MatSet<R> = (&w2).filt(move |((((_, s), _), _), _)| s as f64 > mean).map(|x: R| x).collect();
    let v = drain((&rv).select(Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0 .0 .0 .0).select(owner_user.select((&ui).and((&uvc).opt()))))));
    let v = top_n(v, |&(_, (((((p, s), w), _), _), _))| (Reverse(s), w.is_none(), Reverse(w), p), 10);
    rows(v.into_iter().map(|(_, (((((p, _), _), sr), vr), (a, n)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(n.unwrap_or(0)), V::I(a[0]), V::I(a[1]), V::S("Votes Present")]);
        f.push(V::S(if sr <= 5 { "Top 5 Posts" } else if vr <= 5 { "Top 5 Viewed Posts" } else { "Regular Post" }));
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, AnswerCount, QuestionCount, UpVotes, DownVotes, RANK() OVER (ORDER BY Reputation DESC) AS RankReputation,
//        RANK() OVER (ORDER BY UpVotes DESC) AS RankUpVotes FROM UserStats),
// RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, COUNT(c.Id) AS CommentCount, MAX(ph.CreationDate) AS LastEditDate FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId
//     LEFT JOIN PostHistory ph ON p.Id = ph.PostId AND ph.PostHistoryTypeId IN (4, 5, 6) WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId)
// SELECT tu.DisplayName, tu.Reputation, tu.PostCount, tu.AnswerCount, rp.Title, rp.CreationDate, rp.CommentCount, CASE WHEN rp.LastEditDate IS NOT NULL THEN 'Edited' ELSE 'Not Edited' END AS EditStatus,
//        COALESCE(rp.OwnerUserId, -1) AS OwnerUserId, CASE WHEN tu.RankReputation = 1 THEN 'Top Reputation' WHEN tu.RankUpVotes = 1 THEN 'Most Upvotes' ELSE 'Regular User' END AS UserType
// FROM TopUsers tu JOIN RecentPosts rp ON tu.UserId = rp.OwnerUserId ORDER BY tu.Reputation DESC, rp.CreationDate DESC LIMIT 50;
//
// AnswerCount and UpVotes sum over the posts x votes rows.
fn q4941(db: &'static So) -> String {
    let Post { creation_date, owner_user, post_type_id, .. } = &db.post;
    let us = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt()).fold([0i64; 2], |a, p| match p {
        Some((t, v)) => [a[0] + (t == 2) as i64, a[1] + (v == Some(2)) as i64],
        None => a,
    });
    let rw = whole(db.user.with(&us)).select(Ident::<User>::new().and(&db.user.reputation).and(&us)).window(rank, |((_, r), _)| Reverse(r), asc);
    let rw2 = (&rw).window(rank, |(((_, _), a), _)| Reverse(a[1]), asc);
    let rk = by_first(&(&rw2).map(|((((u, _), _), rr), ru)| (u, (rr, ru))).collect());
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let recent = || db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)));
    let rp = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([4, 5, 6]))).opt())).fold((0i64, false), |(n, e), (c, h)| (n + c.is_some() as i64, e || h.is_some()));
    let v = drain((&rp).and(owner_user.select(Ident::<User>::new().and(&us).and(&pc).and(&rk))));
    let v = top_n(v, |&(p, (_, (((u, _), _), _)))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 50);
    rows(v.into_iter().map(|(p, ((c, e), (((u, a), n), (rr, ru))))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(n), V::I(a[0])]);
        f.extend(post_fields(db, p, &["title", "created"]));
        f.extend([V::I(c), V::S(if e { "Edited" } else { "Not Edited" }), user_col(db, u, "uid")]);
        f.push(V::S(if rr == 1 { "Top Reputation" } else if ru == 1 { "Most Upvotes" } else { "Regular User" }));
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, p.ParentId, p.AcceptedAnswerId, COALESCE(Users.DisplayName, 'Community User') AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn FROM Posts p LEFT JOIN Users ON p.OwnerUserId = Users.Id
//     WHERE p.CreationDate > (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days')),
// ActiveUsers AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(u.UpVotes) AS TotalUpVotes, SUM(u.DownVotes) AS TotalDownVotes, RANK() OVER (ORDER BY COUNT(DISTINCT p.Id) DESC) AS Rank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// ClosedPosts AS (SELECT ph.PostId, ph.UserId, ph.CreationDate, COUNT(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 END) AS CloseCount FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11)
//     GROUP BY ph.PostId, ph.UserId, ph.CreationDate),
// TopClosedPosts AS (SELECT cp.PostId, p.Title, cp.CloseCount, COALESCE(u.DisplayName, 'Unknown User') AS CloserUser FROM ClosedPosts cp JOIN Posts p ON cp.PostId = p.Id
//     LEFT JOIN Users u ON cp.UserId = u.Id WHERE cp.CloseCount > 0 ORDER BY cp.CloseCount DESC, cp.PostId, cp.UserId, cp.CreationDate LIMIT 10)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.OwnerDisplayName, au.PostCount AS UserPostCount, au.TotalUpVotes, au.TotalDownVotes, tk.CloseCount AS UserClosedPostCount, tk.CloserUser
// FROM RecentPosts rp LEFT JOIN ActiveUsers au ON rp.OwnerUserId = au.UserId LEFT JOIN TopClosedPosts tk ON rp.PostId = tk.PostId
// WHERE (rp.ParentId IS NULL OR rp.AcceptedAnswerId IS NOT NULL) AND (au.PostCount > 5 OR au.TotalUpVotes > 100) ORDER BY rp.CreationDate DESC;
//
// Uses rewrites/22664.sql, which adds (PostId, UserId, CreationDate) to TopClosedPosts' ORDER BY (every CloseCount is 1). rn and Rank are never read.
// TotalUpVotes sums the user's UpVotes once per post row.
fn q22664(db: &'static So) -> String {
    let Post { creation_date, owner_user, parent_id, accepted_answer_id, .. } = &db.post;
    let au = db
        .user
        .group_by(Ident::<User>::new())
        .select((&db.user.up_votes).and(&db.user.down_votes).and(posts_of(db).opt()))
        .fold([0i64; 3], |a, ((u, d), p)| [a[0] + p.is_some() as i64, a[1] + u, a[2] + d]);
    let PostHistory { post_history_type_id, post, user_id, creation_date: hd, .. } = &db.post_history;
    type K = ((Id<Post>, Option<i64>), i64);
    let cp = db.post_history.with(post_history_type_id.is_in([10, 11])).group_by(post.and(user_id.opt()).and(hd)).select(post_history_type_id).fold(0i64, |n, t| n + (t == 10) as i64);
    let top = top_n(drain((&cp).filt(|n| n > 0)), |&(((p, u), d), n)| (Reverse(n), db.post.origid.get(p).unwrap(), u.is_none(), u, d), 10);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    type T = (K, i64);
    let tk = rel(top);
    let tki: HashIdx<Id<Post>, (T, Option<Str>)> =
        (&tk).map(|x: T| x.0 .0 .0).inv().select(&tk).select(Same::<T>::new().and(Same::<T>::new().flat_map(|x: T| x.0 .0 .1).select(&uidx).select(&db.user.display_name).opt())).collect();
    let rp = db
        .post
        .with(creation_date.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .with(Ident::<Post>::new().minus(parent_id).or(accepted_answer_id));
    let v = drain(rp.select(Ident::<Post>::new().and(owner_user.select(Ident::<User>::new().and((&au).filt(|a| a[0] > 5 || a[1] > 100))).map(|x: (Id<User>, [i64; 3])| Some(x))).and((&tki).opt())));
    rows(v.into_iter().map(|(_, ((p, u), t))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.push(V::S(owner_user.get(p).map_or("Community User", |u| db.user.display_name.get(u).unwrap())));
        f.extend(match u {
            Some((_, a)) => [V::I(a[0]), V::I(a[1]), V::I(a[2])],
            None => [V::Null, V::Null, V::Null],
        });
        f.extend(match t {
            Some(((_, n), name)) => [V::I(n), V::S(name.unwrap_or("Unknown User"))],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.Score, p.AnswerCount, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS RankScore,
//        DENSE_RANK() OVER (ORDER BY p.CreationDate ASC) AS CreationRank FROM Posts p WHERE p.CreationDate >= (CAST('2024-10-01' AS DATE) - INTERVAL '365 days')),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.ViewCount, rp.Score, rp.AnswerCount, COUNT(c.Id) FILTER (WHERE c.Score >= 0) AS PositiveComments, COUNT(DISTINCT v.UserId) AS UniqueUpvoters
//     FROM RankedPosts rp LEFT JOIN Comments c ON c.PostId = rp.PostId LEFT JOIN Votes v ON v.PostId = rp.PostId AND v.VoteTypeId = 2 WHERE rp.RankScore <= 5
//     GROUP BY rp.PostId, rp.Title, rp.ViewCount, rp.Score, rp.AnswerCount),
// PostDetails AS (SELECT tp.PostId, tp.Title, tp.ViewCount, tp.Score, tp.AnswerCount, tp.PositiveComments, tp.UniqueUpvoters,
//        COALESCE((SELECT SUM(b.Class) FROM Badges b WHERE b.UserId IN (SELECT DISTINCT OwnerUserId FROM Posts WHERE Id = tp.PostId)), 0) AS TotalBadgeClass,
//        (SELECT COUNT(DISTINCT pl.RelatedPostId) FROM PostLinks pl WHERE pl.PostId = tp.PostId) AS RelatedPostCount FROM TopPosts tp)
// SELECT pd.PostId, pd.Title, pd.ViewCount, pd.Score, pd.AnswerCount, pd.PositiveComments, pd.UniqueUpvoters, pd.TotalBadgeClass, pd.RelatedPostCount,
//        CASE WHEN pd.Score IS NULL THEN 'No Score' WHEN pd.Score > 10 THEN 'High Score' ELSE 'Moderate Score' END AS ScoreCategory,
//        CASE WHEN pd.ViewCount IS NULL THEN 'Unseen' WHEN pd.ViewCount > 1000 THEN 'Trending' ELSE 'Normal' END AS TrendStatus
// FROM PostDetails pd WHERE (pd.TotalBadgeClass > 10 OR pd.ViewCount > 500) AND pd.UniqueUpvoters > 2 ORDER BY pd.Score DESC NULLS LAST, pd.ViewCount DESC;
//
// RankScore reads only base columns, so the posts are picked first (ties broken by post id). CreationRank is never read.
fn q20277(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, owner_user_id, view_count, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_days(date(2024, 10, 1), -365))).group_by(post_type_id).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 5).map(|((p, _), _)| p).collect();
    let uv = || votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2)));
    let pcm = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.score).opt().and(uv().opt())).fold(0i64, |n, (s, _)| n + s.map_or(false, |s| s >= 0) as i64);
    let nuv = (&tp).group_by(Ident::<Post>::new()).select(uv().select(&db.vote.user_id)).count_distinct();
    let tbc = db.badge.group_by(&db.badge.user_id).select(&db.badge.class).fold(0i64, |s, c| s + c);
    let rl = (&tp).group_by(Ident::<Post>::new()).select(links_of(db).select(&db.post_link.related_post_id)).count_distinct();
    type X = ((((i64, i64), Option<i64>), Option<i64>), Option<i64>);
    let v = drain(
        (&pcm)
            .and((&nuv).opt().map(|n: Option<i64>| n.unwrap_or(0)).filt(|n| n > 2))
            .and(owner_user_id.select(&tbc).opt())
            .and((&rl).opt())
            .and(view_count.opt())
            .filt(|((((_, _), b), _), w): X| b.unwrap_or(0) > 10 || w.map_or(false, |w| w > 500))
            .map(|(x, _): X| x),
    );
    rows(v.into_iter().map(|(p, (((c, n), b), l))| {
        let l = l.unwrap_or(0);

        let s = score.get(p).unwrap();
        let w = view_count.get(p);
        let mut f = post_fields(db, p, &["id", "title", "views", "score", "answers"]);
        f.extend([V::I(c), V::I(n), V::I(b.unwrap_or(0)), V::I(l)]);
        f.push(V::S(if s > 10 { "High Score" } else { "Moderate Score" }));
        f.push(V::S(match w {
            None => "Unseen",
            Some(w) if w > 1000 => "Trending",
            _ => "Normal",
        }));
        row(f)
    }))
}

// WITH PostStats AS (SELECT p.Id AS PostId, p.OwnerUserId, p.Title, p.CreationDate, p.Score, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount, COUNT(CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN 1 END) AS ClosingActions,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId
//     WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year' GROUP BY p.Id, p.OwnerUserId, p.Title, p.CreationDate, p.Score),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(SUM(ps.CommentCount), 0) AS TotalComments, COALESCE(SUM(ps.UpVoteCount), 0) AS TotalUpVotes,
//        COALESCE(SUM(ps.DownVoteCount), 0) AS TotalDownVotes, COUNT(DISTINCT ps.PostId) AS TotalPosts, MAX(ps.PostRank) AS HighestPostRank
//     FROM Users u LEFT JOIN PostStats ps ON u.Id = ps.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation)
// SELECT u.UserId, u.DisplayName, u.Reputation, u.TotalComments, u.TotalUpVotes, u.TotalDownVotes, u.TotalPosts,
//        CASE WHEN u.TotalPosts = 0 THEN NULL ELSE ROUND((CAST(u.TotalUpVotes AS FLOAT) / NULLIF(u.TotalPosts, 0)) * 100, 2) END AS UpVotePercentage,
//        CASE WHEN u.Reputation >= 1000 THEN 'Veteran' WHEN u.Reputation >= 100 THEN 'Intermediate' ELSE 'Novice' END AS UserLevel,
//        CASE WHEN EXISTS (SELECT 1 FROM PostHistory ph WHERE ph.UserId = u.UserId AND ph.PostHistoryTypeId IN (10, 11)) THEN 'Has closed posts' ELSE 'No closing action' END AS ClosingActionStatus
// FROM UserStats u WHERE u.Reputation IS NOT NULL ORDER BY u.TotalUpVotes DESC, u.TotalComments DESC, u.TotalPosts DESC;
//
// HighestPostRank is never read, so PostRank is not computed. The per-post sums run over the comments x votes x history rows. FLOAT is a 32-bit REAL.
fn q20471(db: &'static So) -> String {
    let Post { creation_date, .. } = &db.post;
    let since = add_years(ts(2024, 10, 1, 12, 34, 56), -1);
    let recent = || posts_of(db).select(Ident::<Post>::new().with(creation_date.ge(since)));
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(recent().select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(history_of(db).opt())).opt())
        .fold([0i64; 3], |a, x| match x {
            Some(((c, t), _)) => [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64],
            None => a,
        });
    let pc = db.user.group_by(Ident::<User>::new()).select(recent().opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let PostHistory { user, post_history_type_id, .. } = &db.post_history;
    let closer: MatSet<Id<User>> = db.post_history.with(post_history_type_id.is_in([10, 11])).select(user).collect();
    let v = drain((&us).and(&pc).and(Ident::<User>::new().with(&closer).opt()));
    rows(v.into_iter().map(|(u, ((a, n), c))| {
        let r = db.user.reputation.get(u).unwrap();
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(n)]);
        f.push(if n == 0 { V::Null } else {
            let x = a[1] as f32 / n as f32 * 100.0f32;
            V::F(((x as f64 * 100.0).round() / 100.0) as f32 as f64)
        });
        f.push(V::S(if r >= 1000 { "Veteran" } else if r >= 100 { "Intermediate" } else { "Novice" }));
        f.push(V::S(if c.is_some() { "Has closed posts" } else { "No closing action" }));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COALESCE(P.QuestionCount, 0) AS QuestionCount, COALESCE(A.AnswerCount, 0) AS AnswerCount, COALESCE(C.CommentCount, 0) AS CommentCount
//     FROM Users U LEFT JOIN (SELECT OwnerUserId, COUNT(*) AS QuestionCount FROM Posts WHERE PostTypeId = 1 GROUP BY OwnerUserId) P ON U.Id = P.OwnerUserId
//     LEFT JOIN (SELECT OwnerUserId, COUNT(*) AS AnswerCount FROM Posts WHERE PostTypeId = 2 GROUP BY OwnerUserId) A ON U.Id = A.OwnerUserId
//     LEFT JOIN (SELECT UserId, COUNT(*) AS CommentCount FROM Comments GROUP BY UserId) C ON U.Id = C.UserId),
// PostStatistics AS (SELECT P.Id AS PostId, P.Title, COUNT(DISTINCT C.Id) AS TotalComments, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes, SUM(CASE WHEN P.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS IsAccepted
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY P.Id, P.Title),
// RankedPosts AS (SELECT PS.*, RANK() OVER (ORDER BY PS.TotalUpvotes DESC, PS.TotalDownvotes ASC) AS PostRank FROM PostStatistics PS)
// SELECT UA.DisplayName AS UserName, UA.Reputation, RP.Title, RP.TotalComments, RP.TotalUpvotes, RP.TotalDownvotes, RP.IsAccepted, RP.PostRank
// FROM UserActivity UA JOIN Posts P ON P.OwnerUserId = UA.UserId JOIN RankedPosts RP ON P.Id = RP.PostId WHERE UA.Reputation > 1000 AND (RP.TotalComments > 5 OR RP.TotalUpvotes > 10)
// ORDER BY UA.Reputation DESC, RP.PostRank;
//
// UserActivity's counts are never read. The vote sums and IsAccepted run over the comments x votes rows.
fn q4454(db: &'static So) -> String {
    let Post { creation_date, owner_user, accepted_answer_id, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let ps = recent().group_by(Ident::<Post>::new()).select(accepted_answer_id.opt().and(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))).fold([0i64; 3], |a, (x, (_, t))| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + x.is_some() as i64]
    });
    let cc = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let w = whole(recent()).select(Ident::<Post>::new().and((&ps).and(&cc))).window(rank, |(_, (a, _))| (Reverse(a[0]), a[1]), asc);
    type R = ((Id<Post>, ([i64; 3], i64)), i64);
    let rv: MatSet<R> = (&w).filt(|((_, (a, c)), _)| c > 5 || a[0] > 10).collect();
    let v = drain(
        (&rv)
            .select(Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0 .0).select(owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(1000)))))),
    );
    rows(v.into_iter().map(|(_, (((p, (a, c)), k), u))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["title"]));
        f.extend([V::I(c), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(k)]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN P.PostTypeId = 1 THEN P.ViewCount ELSE 0 END) AS TotalViews,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN P.Score ELSE 0 END) AS TotalAnswerScore FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId WHERE U.Reputation > 1000 GROUP BY U.Id, U.DisplayName),
// PostStatistics AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, EXTRACT(YEAR FROM P.CreationDate) AS PostYear, PT.Name AS PostTypeName,
//        COALESCE(COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END), 0) AS CommentCount, COALESCE(SUM(V.BountyAmount), 0) AS TotalBounty
//     FROM Posts P JOIN PostTypes PT ON P.PostTypeId = PT.Id LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId = 8
//     GROUP BY P.Id, P.Title, P.CreationDate, P.Score, P.ViewCount, PT.Name),
// UserPostActivity AS (SELECT UA.UserId, UA.DisplayName, UA.PostCount, UA.QuestionCount, UA.AnswerCount, UA.TotalViews, UA.TotalAnswerScore, PS.PostId, PS.Title, PS.CreationDate, PS.Score, PS.ViewCount,
//        PS.PostYear, PS.PostTypeName, PS.CommentCount, PS.TotalBounty FROM UserActivity UA JOIN PostStatistics PS ON UA.UserId = PS.PostId)
// SELECT U.DisplayName, UP.PostId, UP.Title, UP.CreationDate, UP.Score, UP.ViewCount, UP.PostTypeName, UP.CommentCount, UP.TotalBounty, UP.TotalViews, UP.QuestionCount, UP.AnswerCount,
//        RANK() OVER (ORDER BY UP.TotalViews DESC) AS ViewRank
// FROM UserPostActivity UP JOIN Users U ON UP.UserId = U.Id WHERE UP.PostCount > 5 ORDER BY UP.TotalViews DESC, UP.CreationDate ASC;
//
// `UA.UserId = PS.PostId` joins a user id to a post id, so it goes through the raw ids, and PostStatistics is driven for those posts alone.
// ViewRank ranks the rows left after the WHERE.
fn q5946(db: &'static So) -> String {
    let Post { post_type_id, view_count, score, .. } = &db.post;
    let ua = db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(view_count.opt()).and(score)).opt()).fold([0i64; 5], |a, p| match p {
        Some(((t, w), s)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + if t == 1 { w.unwrap_or(0) } else { 0 }, a[4] + if t == 2 { s } else { 0 }],
        None => a,
    });
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let mp: MatSet<Id<Post>> = db.user.with((&ua).filt(|a| a[0] > 5)).select(&db.user.origid).select(&pidx).collect();
    let bv = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let ps = (&mp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(bv.opt())).fold([0i64; 2], |a, (c, b)| [a[0] + c.is_some() as i64, a[1] + b.flatten().unwrap_or(0)]);
    let up = || (&ua).filt(|a| a[0] > 5).and((&db.user.origid).select(&pidx).select(Ident::<Post>::new().and(&ps)));
    let w = whole(db.user.with(up())).select(Ident::<User>::new().and(up())).window(rank, |(_, (a, _))| Reverse(a[3]), asc);
    let v = drain(&w).into_iter().map(|x| x.1);
    rows(v.into_iter().map(|((u, (a, (p, b))), k)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["id", "title", "created", "score", "views", "type"]));
        f.extend([V::I(b[0]), V::I(b[1]), V::I(a[3]), V::I(a[1]), V::I(a[2]), V::I(k)]);
        row(f)
    }))
}

// WITH RecursivePostHistory AS (SELECT ph.PostId, ph.CreationDate, ph.UserId, ph.PostHistoryTypeId, ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS rn FROM PostHistory ph),
// PostDetails AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, u.DisplayName AS OwnerName, u.Reputation AS OwnerReputation,
//        MAX(CASE WHEN ph.PostHistoryTypeId = 10 THEN ph.CreationDate END) AS LastClosedDate, MAX(CASE WHEN ph.PostHistoryTypeId = 11 THEN ph.CreationDate END) AS LastReopenedDate,
//        COUNT(DISTINCT v.Id) AS VoteCount
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN PostHistory ph ON p.Id = ph.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year') GROUP BY p.Id, p.Title, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, u.DisplayName, u.Reputation
//     HAVING COUNT(DISTINCT v.Id) > 5),
// ClosedPosts AS (SELECT pd.PostId, pd.Title, pd.OwnerName, pd.OwnerReputation, pd.LastClosedDate, pd.LastReopenedDate, (SELECT COUNT(*) FROM Comments c WHERE c.PostId = pd.PostId) AS CommentCount
//     FROM PostDetails pd WHERE pd.LastClosedDate IS NOT NULL),
// ReopenedPosts AS (SELECT pd.PostId, pd.Title, pd.OwnerName, pd.OwnerReputation, pd.LastClosedDate, pd.LastReopenedDate FROM ClosedPosts cp JOIN PostDetails pd ON cp.PostId = pd.PostId
//     WHERE pd.LastReopenedDate IS NOT NULL)
// SELECT r.PostId, r.Title, r.OwnerName, r.OwnerReputation, cp.LastClosedDate, r.LastReopenedDate, COALESCE(cp.CommentCount, 0) AS ClosedComments,
//        CASE WHEN r.LastReopenedDate IS NOT NULL THEN 'Reopened' ELSE 'Still Closed' END AS Status,
//        DENSE_RANK() OVER (PARTITION BY r.OwnerName ORDER BY r.LastReopenedDate DESC NULLS LAST) AS OwnerReopenRank
// FROM ReopenedPosts r LEFT JOIN ClosedPosts cp ON r.PostId = cp.PostId ORDER BY r.LastReopenedDate DESC;
//
// RecursivePostHistory is never read.
fn q30174(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let pd = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user);
    let PostHistory { post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let dates = pd().group_by(Ident::<Post>::new()).select(history_of(db).select(post_history_type_id.and(hd))).fold((i64::MIN, i64::MIN), |(c, r), (t, d)| {
        (if t == 10 { c.max(d) } else { c }, if t == 11 { r.max(d) } else { r })
    });
    let vc = pd().group_by(Ident::<Post>::new()).select(votes_of(db)).fold(0i64, |n, _| n + 1);
    let cc = pd().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let pdx: HashIdx<Id<Post>, (((i64, i64), i64), i64)> = (&dates).filt(|(c, r)| c != i64::MIN && r != i64::MIN).and((&vc).filt(|n| n > 5)).and(&cc).collect();
    let w = pd().with(&pdx).group_by(owner_user.select(&db.user.display_name)).select(Ident::<Post>::new().and(&pdx).and(owner_user)).window(dense_rank, |((_, (((_, r), _), _)), _)| Reverse(r), asc);
    let v = drain(&w);
    rows(v.into_iter().map(|(_, (((p, (((c, r), _), n)), u), k))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::T(c), V::T(r), V::I(n), V::S("Reopened"), V::I(k)]);
        row(f)
    }))
}

// WITH PostStats AS (SELECT p.Id AS PostId, COUNT(CASE WHEN c.Id IS NOT NULL THEN 1 END) AS CommentCount, COUNT(DISTINCT CASE WHEN v.Id IS NOT NULL THEN v.Id END) AS VoteCount,
//        COUNT(DISTINCT CASE WHEN b.Id IS NOT NULL THEN b.Id END) AS BadgeCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount, MAX(p.CreationDate) AS LastActivityDate
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON b.UserId = p.OwnerUserId GROUP BY p.Id),
// UserStats AS (SELECT u.Id AS UserId, SUM(CASE WHEN p.Id IS NOT NULL THEN 1 ELSE 0 END) AS PostCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadgeCount,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadgeCount, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadgeCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostPerformance AS (SELECT ps.PostId, ps.CommentCount, ps.VoteCount, ps.BadgeCount, ps.UpVoteCount, ps.DownVoteCount, us.UserId, us.PostCount, us.GoldBadgeCount, us.SilverBadgeCount,
//        us.BronzeBadgeCount, ps.LastActivityDate, RANK() OVER (ORDER BY ps.UpVoteCount DESC, ps.CommentCount DESC) AS OverallRank
//     FROM PostStats ps JOIN Posts p ON ps.PostId = p.Id JOIN UserStats us ON p.OwnerUserId = us.UserId WHERE ps.LastActivityDate > (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'))
// SELECT p.PostId, p.CommentCount, p.VoteCount, p.UpVoteCount, p.DownVoteCount, p.PostCount AS UserPostCount, p.GoldBadgeCount, p.SilverBadgeCount, p.BronzeBadgeCount, p.LastActivityDate, p.OverallRank
// FROM PostPerformance p WHERE p.OverallRank <= 100 ORDER BY p.OverallRank;
//
// LastActivityDate is the post's CreationDate, so only owned posts created in the last year are ranked. The counts run over the comments x votes x
// owner-badges rows, and UserStats over the posts x badges rows of the owners of the ranked posts.
fn q5385(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let pp = || db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user);
    let ps = pp().group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(owner_user.select(badges_of(db)).opt())).fold([0i64; 3], |a, ((c, t), _)| {
        [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let vc = pp().group_by(Ident::<Post>::new()).select(votes_of(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let w = whole(pp()).select(Ident::<Post>::new().and((&ps).and(&vc))).window(rank, |(_, (a, _))| (Reverse(a[1]), Reverse(a[0])), asc);
    type R = ((Id<Post>, ([i64; 3], i64)), i64);
    let top: MatSet<R> = (&w).filt(|(_, k)| k <= 100).collect();
    let owners: MatSet<Id<User>> = (&top).map(|x: R| x.0 .0).select(owner_user).collect();
    let us = (&owners).group_by(Ident::<User>::new()).select(posts_of(db).opt().and(badges_of(db).select(&db.badge.class).opt())).fold([0i64; 4], |a, (p, c)| {
        [a[0] + p.is_some() as i64, a[1] + (c == Some(1)) as i64, a[2] + (c == Some(2)) as i64, a[3] + (c == Some(3)) as i64]
    });
    let v = drain((&top).select(Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0 .0).select(owner_user.select(&us)))));
    rows(v.into_iter().map(|(_, (((p, (a, n)), k), u))| {
        let mut f = post_fields(db, p, &["id"]);
        f.extend([V::I(a[0]), V::I(n), V::I(a[1]), V::I(a[2])]);
        f.extend(u.map(V::I));
        f.extend(post_fields(db, p, &["created"]));
        f.push(V::I(k));
        row(f)
    }))
}

// WITH UserBadgeCounts AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostScore AS (SELECT p.Id AS PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, COUNT(c.Id) AS CommentCount,
//        COUNT(DISTINCT CASE WHEN v.VoteTypeId IN (2, 3) THEN v.UserId END) AS DistinctVoters FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY p.Id),
// RecentActivity AS (SELECT u.Id AS UserId, MAX(ph.CreationDate) AS LastActivityDate FROM Users u LEFT JOIN PostHistory ph ON u.Id = ph.UserId GROUP BY u.Id),
// FancyPosts AS (SELECT p.Id, p.Title, p.Score, ps.UpVotes, ps.DownVotes, COALESCE(BadgeCounts.BadgeCount, 0) AS BadgeCount, COALESCE(RA.LastActivityDate, '1900-01-01') AS LastActivityDate,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank
//     FROM Posts p LEFT JOIN PostScore ps ON p.Id = ps.PostId LEFT JOIN UserBadgeCounts BadgeCounts ON p.OwnerUserId = BadgeCounts.UserId LEFT JOIN RecentActivity RA ON p.OwnerUserId = RA.UserId
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND (ps.UpVotes - ps.DownVotes) > 10 AND (p.Tags IS NOT NULL AND p.Tags <> ''))
// SELECT fp.Title, fp.Score, fp.UpVotes, fp.DownVotes, fp.BadgeCount, fp.LastActivityDate, CASE WHEN fp.UserPostRank = 1 THEN 'Top Post' ELSE 'Regular Post' END AS PostStatus
// FROM FancyPosts fp ORDER BY fp.Score DESC, fp.BadgeCount DESC, fp.LastActivityDate DESC;
//
// UpVotes and DownVotes sum over the votes x comments rows. UserPostRank numbers the rows left after the WHERE (the NULL owner is its own partition;
// ties broken by post id).
fn q24664(db: &'static So) -> String {
    let Post { creation_date, owner_user, owner_user_id, tags_str, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(tags_str.filt(|t: Str| !t.is_empty()));
    let ps = recent().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt())).fold([0i64; 2], |a, (t, _)| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
    });
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db)).fold(0i64, |n, _| n + 1);
    let ra = db.post_history.group_by(&db.post_history.user).select(&db.post_history.creation_date).fold(i64::MIN, |m, d| m.max(d));
    let fpx: HashIdx<Id<Post>, (([i64; 2], Option<i64>), Option<i64>)> = (&ps).filt(|a| a[0] - a[1] > 10).and(owner_user.select(&bc).opt()).and(owner_user.select(&ra).opt()).collect();
    let w = recent().with(&fpx).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let first: MatSet<Id<Post>> = (&w).filt(|(_, k)| k == 1).map(|((p, _), _)| p).collect();
    let v = drain((&fpx).and(Ident::<Post>::new().with(&first).opt()));
    rows(v.into_iter().map(|(p, (((a, b), d), f1))| {
        let mut f = post_fields(db, p, &["title", "score"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(b.unwrap_or(0)), V::T(d.unwrap_or(date(1900, 1, 1)))]);
        f.push(V::S(if f1.is_some() { "Top Post" } else { "Regular Post" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, COUNT(c.Id) AS CommentCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate > cast('2024-10-01' as date) - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.PostTypeId),
// ActiveUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId
//     WHERE u.Reputation > 1000 AND u.LastAccessDate >= cast('2024-10-01' as date) - INTERVAL '90 days' GROUP BY u.Id, u.DisplayName),
// CombinedData AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.CommentCount, au.UserId, au.DisplayName, (au.GoldBadges + au.SilverBadges + au.BronzeBadges) AS TotalBadges
//     FROM RankedPosts rp CROSS JOIN ActiveUsers au WHERE rp.Rank <= 5 AND au.GoldBadges > 0),
// FilteredPosts AS (SELECT DISTINCT cd.PostId, cd.Title, cd.CreationDate, cd.Score, cd.CommentCount, cd.UserId, cd.DisplayName, cd.TotalBadges FROM CombinedData cd
//     WHERE cd.Score IS NOT NULL AND cd.TotalBadges > 1 ORDER BY cd.Score DESC)
// SELECT fp.Title, fp.Score, fp.CommentCount, fp.DisplayName, fp.TotalBadges,
//        CASE WHEN fp.CommentCount > 10 THEN 'Highly Discussed' WHEN fp.CommentCount BETWEEN 5 AND 10 THEN 'Moderately Discussed' ELSE 'Less Discussed' END AS DiscussionStatus,
//        COALESCE((SELECT COUNT(*) FROM Votes v WHERE v.PostId = fp.PostId AND v.VoteTypeId = 2), 0) AS UpvoteCount, COALESCE((SELECT AVG(Score) FROM Posts p2 WHERE p2.Id = fp.PostId), 0) AS AverageScore
// FROM FilteredPosts fp WHERE fp.CreationDate < cast('2024-10-01' as date) - INTERVAL '7 days' ORDER BY DiscussionStatus, fp.Score DESC;
//
// Rank reads only base columns, so the posts are picked first (ties broken by post id). (PostId, UserId) is unique in CombinedData, so the DISTINCT
// changes nothing. The AVG subquery reads one post by its key, so it is the post's Score.
fn q22804(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, .. } = &db.post;
    let d0 = date(2024, 10, 1);
    let w = db.post.with(creation_date.gt(add_days(d0, -30))).group_by(post_type_id).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 5).map(|((p, _), _)| p).select(Ident::<Post>::new().with(creation_date.lt(add_days(d0, -7)))).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let up = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2))).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let au = db
        .user
        .with((&db.user.reputation).gt(1000))
        .with((&db.user.last_access_date).ge(add_days(d0, -90)))
        .group_by(Ident::<User>::new())
        .select(badges_of(db).select(&db.badge.class).opt())
        .fold([0i64; 3], |a, c| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    let users = rel(drain((&au).filt(|a| a[0] > 0 && a[0] + a[1] + a[2] > 1)));
    let posts = rel(drain((&cc).and(&up)));
    let mut v = Vec::new();
    (&posts).cross(&users).drive(|_, ((p, (c, n)), (u, a))| v.push((p, c, n, u, a)));
    rows(v.into_iter().map(|(p, c, n, u, a)| {
        let mut f = post_fields(db, p, &["title", "score"]);
        f.extend([V::I(c), user_col(db, u, "name"), V::I(a[0] + a[1] + a[2])]);
        f.push(V::S(if c > 10 { "Highly Discussed" } else if (5..=10).contains(&c) { "Moderately Discussed" } else { "Less Discussed" }));
        f.extend([V::I(n), V::F(score.get(p).unwrap() as f64)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.Reputation, U.CreationDate, U.LastAccessDate, COUNT(DISTINCT B.Id) AS TotalBadges, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes, ROW_NUMBER() OVER (PARTITION BY U.Id ORDER BY U.LastAccessDate DESC) AS UserRank
//     FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.Reputation, U.CreationDate, U.LastAccessDate),
// RecentPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.OwnerUserId, P.Score, P.ViewCount, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS RecentPostRank
//     FROM Posts P WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// TopUsers AS (SELECT US.UserId, US.Reputation, US.TotalBadges, US.TotalUpVotes, US.TotalDownVotes, RANK() OVER (ORDER BY US.Reputation DESC) AS ReputationRank FROM UserStats US WHERE US.TotalBadges > 5)
// SELECT TU.UserId, TU.Reputation, TU.TotalBadges, P.Title AS RecentPostTitle, P.CreationDate AS RecentPostDate, COALESCE(P.Score, 0) AS PostScore, COALESCE(P.ViewCount, 0) AS PostViewCount,
//        (SELECT COUNT(Comment.Id) FROM Comments Comment WHERE Comment.PostId = P.PostId) AS TotalComments,
//        CASE WHEN P.CreationDate IS NOT NULL THEN CASE WHEN P.CreationDate < cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '14 days' THEN 'Old Post'
//        WHEN P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '3 days' THEN 'New Post' ELSE 'Somewhat Recent Post' END ELSE 'No Recent Posts' END AS PostAgeCategory
// FROM TopUsers TU LEFT JOIN RecentPosts P ON TU.UserId = P.OwnerUserId AND P.RecentPostRank = 1 WHERE TU.ReputationRank <= 10 ORDER BY TU.Reputation DESC, RecentPostDate DESC;
//
// The vote sums are never read. RecentPostRank breaks CreationDate ties by post id (the SQL leaves them open).
fn q23847(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, view_count, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db)).fold(0i64, |n, _| n + 1);
    let uw = whole(db.user.with((&bc).filt(|n| n > 5))).select(Ident::<User>::new().and(&db.user.reputation).and(&bc)).window(rank, |((_, r), _)| Reverse(r), asc);
    type R = ((Id<User>, i64), i64);
    let tu: MatSet<R> = (&uw).filt(|(_, k)| k <= 10).map(|(((u, _), b), k)| ((u, b), k)).collect();
    let pw = db.post.with(creation_date.ge(add_days(t0, -30))).group_by(owner_user).select(Ident::<Post>::new().and(owner_user).and(creation_date)).window(row_number, |((p, _), d)| (Reverse(d), p), asc);
    let fi = by_first(&(&pw).filt(|(_, k)| k == 1).map(|(((p, u), _), _)| (u, p)).collect());
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&tu).select(Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0 .0).select((&fi).select(Ident::<Post>::new().and(&cc))).opt())));
    rows(v.into_iter().map(|(_, (((u, b), _), p))| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.push(V::I(b));
        match p {
            Some((p, c)) => {
                let d = creation_date.get(p).unwrap();
                f.extend(post_fields(db, p, &["title", "created"]));
                f.extend([V::I(score.get(p).unwrap()), V::I(view_count.get(p).unwrap_or(0)), V::I(c)]);
                f.push(V::S(if d < add_days(t0, -14) { "Old Post" } else if d >= add_days(t0, -3) { "New Post" } else { "Somewhat Recent Post" }));
            }
            None => f.extend([V::Null, V::Null, V::I(0), V::I(0), V::I(0), V::S("No Recent Posts")]),
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.Body, p.CreationDate, p.Score, p.ViewCount, p.Tags, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.Id) AS VoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (2, 3) WHERE p.PostTypeId = 1
//     GROUP BY p.Id, p.Title, p.Body, p.CreationDate, p.Score, p.ViewCount, p.Tags, u.DisplayName),
// TopQuestions AS (SELECT Id, Title, Body, CreationDate, Score, ViewCount, Tags, OwnerDisplayName, CommentCount, VoteCount FROM RankedPosts WHERE PostRank = 1 ORDER BY Score DESC, ViewCount DESC LIMIT 10),
// PostHistoryDetails AS (SELECT ph.PostId, ph.PostHistoryTypeId, p.Title AS OldTitle, p.Body AS OldBody, STRING_AGG(cht.Name, ', ') AS CloseReasons FROM PostHistory ph LEFT JOIN Posts p ON ph.PostId = p.Id
//     LEFT JOIN CloseReasonTypes cht ON CAST(ph.Comment AS INT) = cht.Id WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.PostId, ph.PostHistoryTypeId, p.Title, p.Body),
// FinalOutput AS (SELECT tq.Title AS QuestionTitle, tq.Body AS QuestionBody, tq.CreationDate AS QuestionDate, tq.Score AS QuestionScore, tq.ViewCount AS QuestionViewCount, tq.Tags AS QuestionTags,
//        tq.OwnerDisplayName AS QuestionOwner, tq.CommentCount AS QuestionCommentCount, tq.VoteCount AS QuestionVoteCount, phd.OldTitle AS PostOldTitle, phd.OldBody AS PostOldBody, phd.CloseReasons AS PostCloseReasons
//     FROM TopQuestions tq LEFT JOIN PostHistoryDetails phd ON tq.Id = phd.PostId)
// SELECT * FROM FinalOutput ORDER BY QuestionScore DESC, QuestionViewCount DESC;
//
// PostRank partitions by the post itself, so it is always 1, and the LIMIT reads only base columns: the ten questions are picked first (ties broken by
// post id). The STRING_AGG has no ORDER BY; it is built in history id order.
fn q25196(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let top = top_n(drain(db.post.with(post_type_id.eq(1)).select(score)), |&(p, s)| { let w = view_count.get(p); (Reverse(s), w.is_none(), Reverse(w), p) }, 10);
    let tq: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let v23 = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([2, 3])));
    let cc = (&tq).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([2, 3]))).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let vc = (&tq).group_by(Ident::<Post>::new()).select(v23.opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let PostHistory { post_history_type_id, comment, post, .. } = &db.post_history;
    let crt: HashIdx<i64, Id<CloseReasonType>> = (&db.close_reason_type.origid).inv().collect();
    let name = comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&crt).select(&db.close_reason_type.name);
    type K = (Id<Post>, i64);
    let phd = db.post_history.with(post_history_type_id.is_in([10, 11])).group_by(post.and(post_history_type_id)).select(name.opt()).buf_fold(|v| {
        let t: Vec<&str> = v.iter().flatten().copied().collect();
        if t.is_empty() { None } else { Some(leak(t, ", ")) }
    });
    let pr = rel(drain(&phd));
    let phi: HashIdx<Id<Post>, (K, Option<Str>)> = (&pr).map(|x: (K, Option<Str>)| x.0 .0).inv().select(&pr).collect();
    let v = drain((&cc).and(&vc).and((&phi).opt()));
    rows(v.into_iter().map(|(p, ((c, n), h))| {
        let mut f = post_fields(db, p, &["title", "body", "created", "score", "views", "tags", "owner"]);
        f.extend([V::I(c), V::I(n)]);
        match h {
            Some((_, r)) => {
                f.extend(post_fields(db, p, &["title", "body"]));
                f.push(ostr(r));
            }
            None => f.extend([V::Null, V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, SUM(CASE WHEN b.Id IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId WHERE u.Reputation >= 100 GROUP BY u.Id, u.DisplayName, u.Reputation),
// PostStatistics AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 END), 0) AS TotalUpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 END), 0) AS TotalDownVotes, COUNT(c.Id) AS CommentCount, PERCENT_RANK() OVER (ORDER BY p.Score DESC) AS ScoreRank
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score),
// ClosedPosts AS (SELECT ph.PostId, ph.CreationDate, ph.UserId, ph.Comment, RANK() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS ClosingRank FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11)),
// UserScore AS (SELECT ua.UserId, ua.DisplayName, ua.Reputation, ps.PostId, ps.Title, ps.ScoreRank, (ua.UpVotes - ua.DownVotes) AS NetVotes FROM UserActivity ua JOIN PostStatistics ps ON ua.PostCount > 0)
// SELECT us.DisplayName, us.Reputation, us.NetVotes, ps.Title, ps.ScoreRank, cp.Comment AS ClosureComment, CASE WHEN cp.ClosingRank IS NOT NULL THEN 'Closed' ELSE 'Active' END AS PostStatus
// FROM UserScore us LEFT JOIN ClosedPosts cp ON us.PostId = cp.PostId JOIN PostStatistics ps ON us.PostId = ps.PostId WHERE us.NetVotes > 0
// ORDER BY us.Reputation DESC, ps.ScoreRank ASC OFFSET 0 ROWS FETCH NEXT 10 ROWS ONLY;
//
// The UserActivity ON names only ua, so it is a cross join. NetVotes sums over the posts x votes x badges rows. ScoreRank is (RANK - 1) / (n - 1).
fn q20733(db: &'static So) -> String {
    let Post { creation_date, score, .. } = &db.post;
    let ua = db
        .user
        .with((&db.user.reputation).ge(100))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt().and(badges_of(db).opt()))
        .fold(0i64, |s, (t, _)| s + match t.flatten() {
            Some(2) => 1,
            Some(3) => -1,
            _ => 0,
        });
    let pc = db.user.with((&db.user.reputation).ge(100)).group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let uw = whole(db.user.with((&ua).filt(|n| n > 0).and(&pc))).select(Ident::<User>::new().and(&db.user.reputation).and((&ua).and(&pc))).window(rank, |((_, r), _)| Reverse(r), asc);
    type U = ((Id<User>, i64), (i64, i64));
    let users: MatSet<U> = (&uw).filt(|(_, k)| k <= 10).map(|(x, _)| x).collect();
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let n = count(recent()) as f64;
    let pr = |k: i64| if n <= 1.0 { 0.0 } else { (k - 1) as f64 / (n - 1.0) };
    let cp = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([10, 11])));
    let sw = whole(recent()).select(Ident::<Post>::new().and(score)).window(rank, |(_, s)| Reverse(s), asc);
    type R = ((Id<Post>, i64), i64);
    type PS = (R, Option<Id<PostHistory>>);
    let psm: MatSet<PS> = (&sw).select(Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0 .0).select(cp.opt()))).collect();
    let pw = whole(&psm).window(rank, |(r, _): PS| r.1, asc);
    let ps: MatSet<PS> = (&pw).filt(|(_, k)| k <= 10).map(|(x, _)| x).collect();
    let v = drain((&users).cross(&ps));
    let v = top_n(v, |&(_, (((u, r), _), (((p, _), k), h)))| (Reverse(r), u, k, p, h), 10);
    rows(v.into_iter().map(|(_, (((u, _), (s, _)), (((p, _), k), h)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(s));
        f.extend(post_fields(db, p, &["title"]));
        f.push(V::F(pr(k)));
        f.push(ostr(h.and_then(|h| db.post_history.comment.get(h))));
        f.push(V::S(if h.is_some() { "Closed" } else { "Active" }));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.Score IS NULL THEN 0 ELSE p.Score END) AS TotalScore,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, RANK() OVER (ORDER BY COUNT(p.Id) DESC) AS PostRank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON v.PostId = p.Id WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName, u.Reputation),
// ProminentUsers AS (SELECT ua.UserId, ua.DisplayName, ua.Reputation, ua.PostCount, ua.TotalScore, ua.UpVotes, ua.DownVotes FROM UserActivity ua WHERE ua.PostRank <= 10),
// PostEngagement AS (SELECT p.Id AS PostId, p.Title, c.Id AS CommentId, c.Text AS CommentText, ph.UserId AS EditorId, ph.Comment AS EditComment, COUNT(DISTINCT c.Id) AS TotalComments
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId AND ph.PostHistoryTypeId IN (4, 5, 6)
//     WHERE p.CreationDate >= (cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year') GROUP BY p.Id, p.Title, c.Id, c.Text, ph.UserId, ph.Comment),
// PostSummary AS (SELECT pe.PostId, pe.Title, COALESCE(SUM(pe.TotalComments), 0) AS CommentCount, COALESCE(MIN(pe.EditComment), 'No edits') AS LatestEditComment FROM PostEngagement pe GROUP BY pe.PostId, pe.Title)
// SELECT pu.DisplayName, pu.Reputation, ps.Title, ps.CommentCount, ps.LatestEditComment,
//        CASE WHEN ps.CommentCount >= 5 THEN 'Highly Engaged' WHEN ps.CommentCount BETWEEN 1 AND 4 THEN 'Moderately Engaged' ELSE 'No Engagement' END AS EngagementLevel,
//        CONCAT('User: ', pu.DisplayName, ' has a total reputation of ', pu.Reputation, ' and posts titled "', ps.Title, '" with comment count: ', ps.CommentCount) AS EngagementSummary
// FROM ProminentUsers pu JOIN PostSummary ps ON pu.UserId = ps.PostId ORDER BY pu.Reputation DESC, ps.CommentCount DESC;
//
// PostRank counts the posts x votes rows. PostEngagement's groups are the distinct (post, comment, editor, edit comment) tuples, built as a set in prela.
// `pu.UserId = ps.PostId` joins a user id to a post id, so it goes through the raw ids.
fn q24635(db: &'static So) -> String {
    let Post { creation_date, .. } = &db.post;
    let ua = db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new()).select(posts_of(db).select(votes_of(db).opt()).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let uw = whole(db.user.with(&ua)).select(Ident::<User>::new().and(&ua)).window(rank, |(_, n)| Reverse(n), asc);
    let pu: MatSet<Id<User>> = (&uw).filt(|(_, k)| k <= 10).map(|((u, _), _)| u).collect();
    let PostHistory { post_history_type_id, user_id, comment, .. } = &db.post_history;
    let edits = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.is_in([4, 5, 6]))).select(user_id.opt().and(comment.opt()));
    type G = (Id<Post>, (Option<Id<Comment>>, (Option<i64>, Option<Str>)));
    let pe: MatSet<G> = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .select(Ident::<Post>::new().and(comments_of(db).opt().and(edits.opt().map(|e: Option<(Option<i64>, Option<Str>)>| e.unwrap_or((None, None))))))
        .map(|x: G| x)
        .collect();
    let ps = (&pe).group_by(Same::<G>::new().map(|x: G| x.0)).select(Same::<G>::new()).fold((0i64, None::<Str>), |(n, m), (_, (c, (_, e)))| {
        (n + c.is_some() as i64, match (m, e) {
            (Some(a), Some(b)) => Some(if b < a { b } else { a }),
            (a, b) => a.or(b),
        })
    });
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let v = drain((&pu).select(Ident::<User>::new().and((&db.user.origid).select(&pidx).select(Ident::<Post>::new().and(&ps)))));
    rows(v.into_iter().map(|(_, (u, (p, (n, e))))| {
        let r = db.user.reputation.get(u).unwrap();
        let name = db.user.display_name.get(u).unwrap();
        let mut f = vec![V::S(name), V::I(r)];
        f.extend(post_fields(db, p, &["title"]));
        f.extend([V::I(n), V::S(e.unwrap_or("No edits"))]);
        f.push(V::S(if n >= 5 { "Highly Engaged" } else if (1..=4).contains(&n) { "Moderately Engaged" } else { "No Engagement" }));
        f.push(V::Owned(format!("User: {name} has a total reputation of {r} and posts titled \"{}\" with comment count: {n}", db.post.title.get(p).unwrap_or(""))));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.ParentId, ROW_NUMBER() OVER (PARTITION BY EXTRACT(YEAR FROM p.CreationDate) ORDER BY p.Score DESC, p.ViewCount DESC) AS RankInYear,
//        COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '5 years'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.ParentId),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, ParentId, RankInYear, CommentCount, UpVotesCount, DownVotesCount FROM RankedPosts WHERE RankInYear <= 10),
// UserEngagement AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, COALESCE(SUM(b.Class), 0) AS TotalBadges, RANK() OVER (ORDER BY COUNT(DISTINCT p.Id) DESC) AS PostRank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId WHERE u.CreationDate < CURRENT_DATE GROUP BY u.Id, u.DisplayName),
// FinalOutput AS (SELECT tp.Title, tp.Score, tp.ViewCount, u.DisplayName AS Owner, tp.CommentCount, tp.UpVotesCount, tp.DownVotesCount FROM TopPosts tp
//     JOIN UserEngagement u ON tp.ParentId = u.UserId OR tp.ParentId IS NULL WHERE tp.Score > 10)
// SELECT FO.Title, FO.Owner, FO.Score, FO.ViewCount, FO.CommentCount, (FO.UpVotesCount - FO.DownVotesCount) AS NetVotes,
//        CASE WHEN FO.Score > 100 THEN 'Highly Rated' WHEN FO.Score BETWEEN 50 AND 100 THEN 'Moderately Rated' ELSE 'Low Rated' END AS RatingCategory
// FROM FinalOutput FO ORDER BY FO.Score DESC, FO.ViewCount DESC;
//
// UserEngagement's aggregates are never read, so it is the users created before today. RankInYear reads only base columns, so the posts are picked first
// (ties broken by post id). The ON is a disjunction whose branches cannot both hold: a post with no parent is crossed with every user, and one with a
// parent joins the user whose raw id equals it.
fn q22180(db: &'static So) -> String {
    let Post { creation_date, score, view_count, parent_id, .. } = &db.post;
    let today = current_date();
    let w = db
        .post
        .with(creation_date.ge(add_years(today, -5)))
        .group_by(creation_date.map(|d: i64| year(d)))
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(row_number, |((p, s), w)| (Reverse(s), w.is_none(), Reverse(w), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 10).map(|(((p, _), _), _)| p).select(Ident::<Post>::new().with(score.gt(10))).collect();
    let agg = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 2], |a, (c, t)| {
        [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64 - (t == Some(3)) as i64]
    });
    let uu: MatSet<Id<User>> = db.user.with((&db.user.creation_date).lt(today)).collect();
    let uall: HashIdx<(), Id<User>> = whole(&uu).collect();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let roots = (&agg).minus(parent_id).and(Ident::<Post>::new().map(|_: Id<Post>| ()).select(&uall));
    let kids = (&agg).and(parent_id.select(&uidx).with(&uu));
    let v = drain(roots.union(kids));
    rows(v.into_iter().map(|(p, (a, u))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["title"]);
        f.push(user_col(db, u, "name"));
        f.extend(post_fields(db, p, &["score", "views"]));
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.push(V::S(if s > 100 { "Highly Rated" } else if s >= 50 { "Moderately Rated" } else { "Low Rated" }));
        row(f)
    }))
}

// WITH UserScores AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes, COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes,
//        COUNT(DISTINCT P.Id) AS PostsCount, RANK() OVER (ORDER BY U.Reputation DESC) AS UserRank FROM Users U LEFT JOIN Votes V ON V.UserId = U.Id LEFT JOIN Posts P ON P.OwnerUserId = U.Id
//     GROUP BY U.Id, U.DisplayName, U.Reputation),
// QualifiedUsers AS (SELECT Us.UserId, Us.DisplayName, Us.Reputation, Us.PostsCount, Us.Upvotes, Us.Downvotes, CASE WHEN Us.Reputation > 5000 THEN 'Master' WHEN Us.Reputation BETWEEN 1000 AND 5000 THEN 'Skilled' ELSE 'Novice' END AS ExpertiseLevel
//     FROM UserScores Us WHERE Us.PostsCount > 10),
// PopularPosts AS (SELECT P.Id AS PostId, P.Title, P.ViewCount, P.Score, P.AcceptedAnswerId FROM Posts P WHERE P.ViewCount > (SELECT AVG(ViewCount) FROM Posts) ORDER BY P.ViewCount DESC LIMIT 5),
// PostComments AS (SELECT C.PostId, COUNT(C.Id) AS CommentCount, MAX(C.CreationDate) AS LastCommentDate FROM Comments C GROUP BY C.PostId)
// SELECT Pu.UserId, Pu.DisplayName, Pu.ExpertiseLevel, Po.PostId, Po.Title, Po.ViewCount, Po.Score, COALESCE(PC.CommentCount, 0) AS CommentCount,
//        CASE WHEN Po.AcceptedAnswerId IS NULL THEN 'Not Accepted' ELSE 'Accepted' END AS AnswerStatus,
//        CASE WHEN PC.LastCommentDate < cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' THEN 'Inactive Discussion' ELSE 'Active Discussion' END AS DiscussionStatus
// FROM QualifiedUsers Pu CROSS JOIN PopularPosts Po LEFT JOIN PostComments PC ON Po.PostId = PC.PostId WHERE Pu.Reputation IS NOT NULL AND Pu.ExpertiseLevel != 'Novice'
// ORDER BY Pu.Reputation DESC, Po.ViewCount DESC;
//
// The vote sums and UserRank are never read, so PostsCount is the user's posts. The average is a separate query.
fn q21633(db: &'static So) -> String {
    let Post { view_count, accepted_answer_id, .. } = &db.post;
    let pc = db.user.with((&db.user.reputation).ge(1000)).group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let users = rel(drain((&pc).filt(|n| n > 10)));
    let (s, n) = view_count.fold_flat((0i64, 0i64), |(s, n), w| (s + w, n + 1));
    let mean = s as f64 / n as f64;
    let top = top_n(drain(view_count.filt(move |w| w as f64 > mean)), |&(p, w)| (Reverse(w), p), 5);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pcs = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.creation_date)).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let posts = rel(drain((&tp).select(Ident::<Post>::new().and((&pcs).opt()))));
    let mut v = Vec::new();
    (&users).cross(&posts).drive(|_, ((u, _), (_, (p, c)))| v.push((u, p, c)));
    let cut = add_days(ts(2024, 10, 1, 12, 34, 56), -30);
    rows(v.into_iter().map(|(u, p, c)| {
        let r = db.user.reputation.get(u).unwrap();
        let mut f = ucols(db, u, &["uid", "name"]);
        f.push(V::S(if r > 5000 { "Master" } else { "Skilled" }));
        f.extend(post_fields(db, p, &["id", "title", "views", "score"]));
        f.push(V::I(c.map_or(0, |c| c.0)));
        f.push(V::S(if accepted_answer_id.get(p).is_none() { "Not Accepted" } else { "Accepted" }));
        f.push(V::S(if c.map_or(false, |c| c.1 < cut) { "Inactive Discussion" } else { "Active Discussion" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.CreationDate > CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges, COALESCE(SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadges,
//        COALESCE(SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadges, COUNT(DISTINCT CASE WHEN ph.PostHistoryTypeId = 10 THEN ph.PostId END) AS ClosedQuestions,
//        COUNT(DISTINCT CASE WHEN ph.PostHistoryTypeId = 11 THEN ph.PostId END) AS ReopenedQuestions, COUNT(DISTINCT CASE WHEN v.VoteTypeId = 2 THEN v.Id END) AS UpvotesReceived
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN PostHistory ph ON p.Id = ph.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     GROUP BY u.Id, u.DisplayName, u.Reputation),
// FilteredPosts AS (SELECT rp.*, us.Reputation, us.GoldBadges, us.SilverBadges, us.BronzeBadges,
//        CASE WHEN us.Reputation > 1000 THEN 'Experienced' WHEN us.Reputation BETWEEN 500 AND 1000 THEN 'Moderate' ELSE 'Novice' END AS UserExperienceLevel
//     FROM RankedPosts rp JOIN UserStats us ON rp.OwnerUserId = us.UserId WHERE rp.UserPostRank <= 3)
// SELECT fp.PostId, fp.Title, fp.CreationDate, fp.Score, fp.ViewCount, fp.Reputation, fp.UserExperienceLevel, COALESCE(SUM(CASE WHEN ph.PostId IS NOT NULL THEN 1 ELSE 0 END), 0) AS TotalEdits,
//        (SELECT COUNT(*) FROM Comments c WHERE c.PostId = fp.PostId) AS CommentCount
// FROM FilteredPosts fp LEFT JOIN PostHistory ph ON fp.PostId = ph.PostId GROUP BY fp.PostId, fp.Title, fp.CreationDate, fp.Score, fp.ViewCount, fp.Reputation, fp.UserExperienceLevel
// ORDER BY fp.Score DESC, fp.ViewCount DESC LIMIT 10;
//
// Only Reputation is read from UserStats, so it is the users. UserPostRank reads only base columns, so the posts are picked first (ties broken by post id).
fn q21849(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, score, view_count, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1)).with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let fp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 3).map(|((p, _), _)| p).collect();
    let te = (&fp).group_by(Ident::<Post>::new()).select(history_of(db).opt()).fold(0i64, |n, h| n + h.is_some() as i64);
    let cc = (&fp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&te).and(&cc).and(owner_user));
    let v = top_n(v, |&(p, _)| { let w = view_count.get(p); (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p) }, 10);
    rows(v.into_iter().map(|(p, ((e, c), u))| {
        let r = db.user.reputation.get(u).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(r), V::S(if r > 1000 { "Experienced" } else if r >= 500 { "Moderate" } else { "Novice" }), V::I(e), V::I(c)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS Rank, COUNT(c.Id) AS CommentCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.Score IS NOT NULL AND p.Score > 0 GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.PostTypeId),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverBadges, COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostHistoryDetails AS (SELECT ph.PostId, ph.UserId, ph.PostHistoryTypeId, ph.CreationDate, ph.Comment, COUNT(*) OVER (PARTITION BY ph.PostId) AS HistoryCount FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11, 12))
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, COALESCE(rb.GoldBadges, 0) AS GoldBadges, COALESCE(rb.SilverBadges, 0) AS SilverBadges, COALESCE(rb.BronzeBadges, 0) AS BronzeBadges,
//        COUNT(DISTINCT ph.UserId) AS UniqueUsersInHistory, MAX(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS IsClosed, MAX(CASE WHEN ph.PostHistoryTypeId = 11 THEN 1 ELSE 0 END) AS IsReopened,
//        MAX(CASE WHEN ph.PostHistoryTypeId = 12 THEN 1 ELSE 0 END) AS IsDeleted, rp.CommentCount, EXTRACT(DOW FROM rp.CreationDate) AS DayOfWeek,
//        CASE WHEN rp.ViewCount > 1000 THEN 'High Views' WHEN rp.ViewCount BETWEEN 500 AND 1000 THEN 'Medium Views' ELSE 'Low Views' END AS ViewCountCategory
// FROM RankedPosts rp LEFT JOIN UserBadges rb ON rp.PostId = rb.UserId LEFT JOIN PostHistoryDetails ph ON rp.PostId = ph.PostId WHERE rp.Rank <= 5
// GROUP BY rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rb.GoldBadges, rb.SilverBadges, rb.BronzeBadges, rp.CommentCount ORDER BY rp.ViewCount DESC, rp.CreationDate DESC;
//
// Rank reads only base columns, so the posts are picked first (ties broken by post id). `rp.PostId = rb.UserId` joins a post id to a user id, so it goes
// through the raw ids.
fn q21499(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, view_count, .. } = &db.post;
    let w = db.post.with(score.gt(0)).group_by(post_type_id).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 5).map(|((p, _), _)| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let PostHistory { post_history_type_id, user_id, .. } = &db.post_history;
    let phx = || history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.is_in([10, 11, 12])));
    let fl = (&tp).group_by(Ident::<Post>::new()).select(phx().select(post_history_type_id).opt()).fold([0i64; 3], |a, t| {
        [a[0].max((t == Some(10)) as i64), a[1].max((t == Some(11)) as i64), a[2].max((t == Some(12)) as i64)]
    });
    let nu = (&tp).group_by(Ident::<Post>::new()).select(phx().select(user_id)).count_distinct();
    let ph = (&fl).and((&nu).opt()).map(|(a, n): ([i64; 3], Option<i64>)| (n.unwrap_or(0), a[0], a[1], a[2]));
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class)).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let v = drain((&cc).and(&ph).and((&db.post.origid).select(&uidx).select(&ub).opt()));
    rows(v.into_iter().map(|(p, ((c, (n, a, b, d)), g))| {
        let g = g.unwrap_or([0; 3]);
        let w = view_count.get(p);
        let mut f = post_fields(db, p, &["id", "title", "created", "views"]);
        f.extend(g.map(V::I));
        f.extend([V::I(n), V::I(a), V::I(b), V::I(d), V::I(c), V::I(dow(creation_date.get(p).unwrap()))]);
        f.push(V::S(match w {
            Some(w) if w > 1000 => "High Views",
            Some(w) if w >= 500 => "Medium Views",
            _ => "Low Views",
        }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY pt.Name ORDER BY p.CreationDate DESC) AS Rank, pt.Name AS PostTypeName,
//        u.DisplayName AS OwnerDisplayName FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id LEFT JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// ClosedPosts AS (SELECT DISTINCT ph.PostId, cr.Name AS CloseReason FROM PostHistory ph JOIN CloseReasonTypes cr ON ph.Comment::INTEGER = cr.Id WHERE ph.PostHistoryTypeId IN (10, 11)),
// PostMetrics AS (SELECT rp.PostId, COUNT(c.Id) AS CommentCount, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounties, COALESCE(AVG(u.Reputation), 0) AS AvgUserReputation
//     FROM RankedPosts rp LEFT JOIN Comments c ON c.PostId = rp.PostId LEFT JOIN Votes v ON v.PostId = rp.PostId LEFT JOIN Users u ON v.UserId = u.Id GROUP BY rp.PostId),
// FinalReport AS (SELECT pm.PostId, rp.Title, rp.CreationDate, pm.CommentCount, pm.TotalBounties, pm.AvgUserReputation, CASE WHEN cp.CloseReason IS NOT NULL THEN cp.CloseReason ELSE 'Not Closed' END AS ClosureStatus,
//        CASE WHEN pm.CommentCount IS NULL THEN 'No Comments' ELSE 'Has Comments' END AS CommentStatus, rp.Rank
//     FROM PostMetrics pm JOIN RankedPosts rp ON pm.PostId = rp.PostId LEFT JOIN ClosedPosts cp ON pm.PostId = cp.PostId)
// SELECT f.PostId, f.Title, f.CreationDate, f.CommentCount, f.TotalBounties, f.AvgUserReputation, f.ClosureStatus, f.CommentStatus,
//        CASE WHEN f.AvgUserReputation > 1000 THEN 'Elite User Engagement' WHEN f.CommentCount > 50 THEN 'Very Active Discussion' ELSE 'Standard Post Quality' END AS EngagementLevel
// FROM FinalReport f WHERE f.Rank <= 5 ORDER BY f.CreationDate DESC;
//
// Rank reads only base columns, so the posts are picked first (ties broken by post id) and PostMetrics is driven for them alone. ClosedPosts' DISTINCT
// is a set of (post, reason) pairs, built in prela.
fn q23449(db: &'static So) -> String {
    let Post { creation_date, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(ptype_name(db)).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 5).map(|((p, _), _)| p).collect();
    let pm = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select((&db.vote.bounty_amount).opt().and((&db.vote.user).select(&db.user.reputation).opt())).opt()))
        .fold([0i64; 4], |a, (c, v)| {
            let (b, r) = v.map_or((None, None), |(b, r)| (b, r));
            [a[0] + c.is_some() as i64, a[1] + b.unwrap_or(0), a[2] + r.unwrap_or(0), a[3] + r.is_some() as i64]
        });
    let PostHistory { post_history_type_id, comment, post, .. } = &db.post_history;
    let crt: HashIdx<i64, Id<CloseReasonType>> = (&db.close_reason_type.origid).inv().collect();
    type C = (Id<Post>, Str);
    let cps: MatSet<C> = db
        .post_history
        .with(post_history_type_id.is_in([10, 11]))
        .select(post.and(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&crt).select(&db.close_reason_type.name)))
        .collect();
    let cpi: HashIdx<Id<Post>, Str> = (&cps).map(|x: C| x.0).inv().select(&cps).map(|x: C| x.1).collect();
    let v = drain((&pm).and((&cpi).opt()));
    rows(v.into_iter().map(|(p, (a, r))| {
        let avg = if a[3] == 0 { 0.0 } else { a[2] as f64 / a[3] as f64 };
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::F(avg), V::S(r.unwrap_or("Not Closed")), V::S("Has Comments")]);
        f.push(V::S(if avg > 1000.0 { "Elite User Engagement" } else if a[0] > 50 { "Very Active Discussion" } else { "Standard Post Quality" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS RankByScore,
//        COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount, SUBSTRING(p.Body, 1, 100) AS PreviewBody, ARRAY_LENGTH(string_to_array(p.Tags, ','), 1) AS TagCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// UserMetrics AS (SELECT u.Id AS UserId, u.Reputation, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty, COUNT(b.Id) AS BadgeCount, COUNT(DISTINCT p.Id) FILTER (WHERE p.PostTypeId = 1) AS QuestionCount,
//        COUNT(DISTINCT p.Id) FILTER (WHERE p.PostTypeId IN (2, 3)) AS AnswerCount FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId
//     GROUP BY u.Id, u.Reputation),
// HighRankedPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, um.UserId, um.Reputation, um.TotalBounty, um.BadgeCount, um.QuestionCount, um.AnswerCount
//     FROM RankedPosts rp JOIN UserMetrics um ON rp.RankByScore = 1 AND rp.PostId IN (SELECT PostId FROM Votes WHERE VoteTypeId = 2 GROUP BY PostId HAVING COUNT(*) > 5) WHERE rp.Score IS NOT NULL)
// SELECT h.Title, h.CreationDate, h.Score, h.UserId, h.Reputation, h.TotalBounty, h.BadgeCount, h.QuestionCount, h.AnswerCount,
//        CASE WHEN h.Score > 100 THEN 'Highly Engaging' WHEN h.Score BETWEEN 50 AND 100 THEN 'Moderately Engaging' ELSE 'Low Engagement' END AS EngagementLevel,
//        CASE WHEN h.TotalBounty = 0 THEN 'No Bounty Offered' ELSE 'Bounty Offered' END AS BountyStatus
// FROM HighRankedPosts h WHERE h.Reputation > 500 ORDER BY h.Score DESC, h.Reputation DESC FETCH FIRST 10 ROWS ONLY;
//
// RankedPosts has one row per post x comment, and RankByScore numbers those rows within an owner (the NULL owner is its own partition; ties broken by post
// and comment id). The ON names no UserMetrics column, so the rank-1 rows are crossed with every user; only rows and users ranked <= 10 on Score and
// Reputation can reach the LIMIT, so both sides are cut by RANK first, and the ten rows are cut before UserMetrics' votes x badges x posts product is
// driven for their users alone.
fn q22403(db: &'static So) -> String {
    let Post { creation_date, score, owner_user_id, post_type_id, .. } = &db.post;
    type J = ((Id<Post>, i64), Option<Id<Comment>>);
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(score).and(comments_of(db).opt()))
        .window(row_number, |((p, s), c)| (Reverse(s), p, c), asc);
    let up = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2)))).fold(0i64, |n, _| n + 1);
    let hr: MatSet<J> = (&w).filt(|(_, k)| k == 1).map(|(x, _)| x).select(Same::<J>::new().with(Same::<J>::new().map(|x: J| x.0 .0).select((&up).filt(|n| n > 5)))).collect();
    let hw = whole(&hr).window(rank, |((_, s), _): J| Reverse(s), asc);
    let hr10: MatSet<J> = (&hw).filt(|(_, k)| k <= 10).map(|(x, _)| x).collect();
    let uw = whole(db.user.with((&db.user.reputation).gt(500))).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r)| Reverse(r), asc);
    let users: MatSet<(Id<User>, i64)> = (&uw).filt(|(_, k)| k <= 10).map(|(x, _)| x).collect();
    let v = top_n(drain((&hr10).cross(&users)), |&(_, (((p, s), c), (u, r)))| (Reverse(s), Reverse(r), p, c, u), 10);
    let v: Vec<((Id<Post>, Option<Id<Comment>>), Id<User>)> = v.into_iter().map(|(_, (((p, _), c), (u, _)))| ((p, c), u)).collect();
    let mu: MatSet<Id<User>> = rel(v.iter().map(|x| x.1).collect()).map(|u| u).collect();
    let um = (&mu)
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select((&db.vote.bounty_amount).opt()).opt().and(badges_of(db).opt()).and(posts_of(db).opt()))
        .fold([0i64; 2], |a, ((v, b), _)| [a[0] + v.flatten().unwrap_or(0), a[1] + b.is_some() as i64]);
    let qa = (&mu).group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(1)) as i64, a[1] + matches!(t, Some(2 | 3)) as i64]);
    let x = rel(v);
    type X = ((Id<Post>, Option<Id<Comment>>), Id<User>);
    let v = drain((&x).select(Same::<X>::new().and(Same::<X>::new().map(|x: X| x.1).select((&um).and(&qa)))));
    let v = top_n(v, |&(_, (((p, c), u), _))| (Reverse(score.get(p).unwrap()), Reverse(db.user.reputation.get(u).unwrap()), p, c, u), 10);
    rows(v.into_iter().map(|(_, (((p, _), u), (a, q)))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["title", "created", "score"]);
        f.extend(ucols(db, u, &["uid", "rep"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(q[0]), V::I(q[1])]);
        f.push(V::S(if s > 100 { "Highly Engaging" } else if s >= 50 { "Moderately Engaging" } else { "Low Engagement" }));
        f.push(V::S(if a[0] == 0 { "No Bounty Offered" } else { "Bounty Offered" }));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT Id AS UserId, Reputation, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank,
//        CASE WHEN Reputation >= 10000 THEN 'High Repute' WHEN Reputation >= 1000 THEN 'Medium Repute' ELSE 'Low Repute' END AS ReputationCategory FROM Users),
// PostDetails AS (SELECT p.Id AS PostId, p.OwnerUserId, p.Title, p.CreationDate, COALESCE(p.ClosedDate, CAST('1970-01-01' AS TIMESTAMP)) AS ClosureDate,
//        DENSE_RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank, ARRAY_LENGTH(string_to_array(p.Tags, ','), 1) AS TagCount, EXTRACT(YEAR FROM p.CreationDate) AS CreationYear FROM Posts p),
// PostHistoryCounts AS (SELECT PostId, COUNT(*) AS EditCount FROM PostHistory WHERE PostHistoryTypeId IN (4, 5, 6) GROUP BY PostId),
// ActiveUsers AS (SELECT u.DisplayName, COUNT(DISTINCT p.Id) AS ActivePostCount, SUM(COALESCE(ph.EditCount, 0)) AS TotalEdits, AVG(COALESCE(uv.Rank, 0)) AS AvgPostRank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN PostHistoryCounts ph ON p.Id = ph.PostId LEFT JOIN UserReputation uv ON u.Id = uv.UserId
//     WHERE u.LastAccessDate >= CAST('2024-10-01' AS DATE) - INTERVAL '1 year' GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT DisplayName, ActivePostCount, TotalEdits, AvgPostRank, ROW_NUMBER() OVER (ORDER BY ActivePostCount DESC, TotalEdits DESC) AS UserRank FROM ActiveUsers)
// SELECT tu.DisplayName, tu.ActivePostCount, tu.TotalEdits, CASE WHEN tu.AvgPostRank IS NULL THEN 'Unknown' WHEN tu.AvgPostRank < 1 THEN 'No Posts' ELSE 'Active Contributor' END AS ContributorStatus,
//        pd.Title, pd.TagCount, CASE WHEN pd.UserPostRank = 1 THEN 'Latest' ELSE 'Earlier Post' END AS PostStatus, COALESCE(pts.Name, 'No Type') AS PostType
// FROM TopUsers tu LEFT JOIN PostDetails pd ON tu.ActivePostCount = pd.OwnerUserId LEFT JOIN PostTypes pts ON pd.PostId = pts.Id WHERE tu.UserRank <= 10 ORDER BY tu.ActivePostCount DESC, tu.TotalEdits DESC;
//
// Every user has a Rank of at least 1, so AvgPostRank is never below 1 and ContributorStatus is always 'Active Contributor'. UserRank breaks ties by user id.
// `tu.ActivePostCount = pd.OwnerUserId` and `pd.PostId = pts.Id` join raw ids.
fn q20102(db: &'static So) -> String {
    let Post { creation_date, owner_user_id, tags_str, .. } = &db.post;
    let pt = &db.post_history.post_history_type_id;
    let ec = db.post_history.with(pt.is_in([4, 5, 6])).group_by(&db.post_history.post).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let au = db
        .user
        .with((&db.user.last_access_date).ge(add_years(date(2024, 10, 1), -1)))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&ec).opt()).opt())
        .fold([0i64; 2], |a, p| match p {
            Some(e) => [a[0] + 1, a[1] + e.unwrap_or(0)],
            None => a,
        });
    let top = top_n(drain(&au), |&(u, a)| (Reverse(a[0]), Reverse(a[1]), u), 10);
    let w = db.post.group_by(owner_user_id).select(Ident::<Post>::new().and(owner_user_id).and(creation_date)).window(dense_rank, |(_, d)| Reverse(d), asc);
    type R = ((Id<Post>, i64), i64);
    let rv: MatSet<R> = (&w).map(|((x, _), k)| (x, k)).collect();
    let by_owner: HashIdx<i64, R> = (&rv).map(|x: R| x.0 .1).inv().collect();
    let ptidx: HashIdx<i64, Id<PostType>> = (&db.post_type.origid).inv().collect();
    type T = (Id<User>, [i64; 2]);
    let tv = rel(top);
    let v = drain((&tv).select(Same::<T>::new().and(Same::<T>::new().map(|x: T| x.1[0]).select(&by_owner).select(Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0 .0).select(&db.post.origid).select(&ptidx).select(&db.post_type.name).opt())).opt())));
    rows(v.into_iter().map(|(_, ((u, a), p))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::S("Active Contributor")];
        match p {
            Some((((p, _), k), t)) => {
                f.extend(post_fields(db, p, &["title"]));
                f.push(oint(tags_str.get(p).map(|t| t.split(',').count() as i64)));
                f.push(V::S(if k == 1 { "Latest" } else { "Earlier Post" }));
                f.push(V::S(t.unwrap_or("No Type")));
            }
            None => f.extend([V::Null, V::Null, V::S("Earlier Post"), V::S("No Type")]),
        }
        row(f)
    }))
}

// WITH UserBadges AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// ClosedPosts AS (SELECT p.Id AS PostId, p.Title, ph.UserId AS CloserId, COUNT(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 END) AS CloseCount FROM Posts p LEFT JOIN PostHistory ph ON p.Id = ph.PostId
//     WHERE ph.PostHistoryTypeId = 10 GROUP BY p.Id, p.Title, ph.UserId),
// PostScores AS (SELECT p.Id AS PostId, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) - SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Score,
//        ROW_NUMBER() OVER (ORDER BY COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) - SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) DESC) AS Rank FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id),
// TopUsers AS (SELECT ub.UserId, ub.DisplayName, ub.BadgeCount, cp.PostId, ps.Score FROM UserBadges ub INNER JOIN ClosedPosts cp ON cp.CloserId = ub.UserId LEFT JOIN PostScores ps ON ps.PostId = cp.PostId
//     WHERE ub.BadgeCount > 0 AND ps.Score IS NOT NULL)
// SELECT tu.DisplayName, tu.BadgeCount, COALESCE(SUM(ps.Score), 0) AS TotalScore,
//        (SELECT COUNT(*) FROM Posts p WHERE p.OwnerUserId = tu.UserId AND p.CreationDate < (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '6 months')) AS OldPostCount,
//        CASE WHEN SUM(ps.Score) IS NULL THEN 'No Score Yet' WHEN SUM(ps.Score) > 100 THEN 'Highly Rated' ELSE 'Emerging Contributor' END AS ContributionLevel
// FROM TopUsers tu LEFT JOIN PostScores ps ON ps.PostId = tu.PostId GROUP BY tu.UserId, tu.DisplayName, tu.BadgeCount HAVING COUNT(DISTINCT tu.PostId) > 1
// ORDER BY TotalScore DESC, tu.BadgeCount DESC, tu.DisplayName LIMIT 10;
//
// Rank is never read. A TopUsers row is one (post, closer) group, so each closer's rows are the distinct posts they closed; PostScores is one row per post.
fn q22400(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db)).fold(0i64, |n, _| n + 1);
    let PostHistory { post_history_type_id, post, user, .. } = &db.post_history;
    let closed: MatSet<(Id<User>, Id<Post>)> = db.post_history.with(post_history_type_id.eq(10)).select(user.and(post)).collect();
    let ps = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold(0i64, |s, t| s + (t == Some(2)) as i64 - (t == Some(3)) as i64);
    type K = (Id<User>, Id<Post>);
    let tu = (&closed).group_by(Same::<K>::new().map(|x: K| x.0)).select(Same::<K>::new().map(|x: K| x.1).select(&ps)).fold((0i64, 0i64), |(n, s), x| (n + 1, s + x));
    let old = db.post.with(creation_date.lt(add_months(ts(2024, 10, 1, 12, 34, 56), -6))).group_by(owner_user).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&tu).filt(|(n, _)| n > 1).and(&bc).and((&old).opt()));
    let v = top_n(v, |&(u, (((_, s), b), _))| (Reverse(s), Reverse(b), db.user.display_name.get(u).unwrap(), u), 10);
    rows(v.into_iter().map(|(u, (((_, s), b), o))| {
        row(vec![user_col(db, u, "name"), V::I(b), V::I(s), V::I(o.unwrap_or(0)), V::S(if s > 100 { "Highly Rated" } else { "Emerging Contributor" })])
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId WHERE U.Reputation > 1000 GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopActiveUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, Questions, Answers, ROW_NUMBER() OVER (ORDER BY TotalPosts DESC, Reputation DESC) AS Rank FROM UserReputation WHERE TotalPosts > 5),
// RecentPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, U.DisplayName AS Author, P.ViewCount, P.Score, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount, MAX(PH.CreationDate) AS LastEditDate
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN PostHistory PH ON P.Id = PH.PostId
//     WHERE P.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' GROUP BY P.Id, P.Title, P.CreationDate, U.DisplayName, P.ViewCount, P.Score),
// UserPostDetails AS (SELECT U.Id AS UserId, U.DisplayName, RP.PostId, RP.Title, RP.ViewCount, RP.Score, COALESCE(RP.CommentCount, 0) AS CommentCount,
//        RANK() OVER (PARTITION BY U.Id ORDER BY RP.CreationDate DESC) AS PostRank FROM Users U JOIN RecentPosts RP ON U.DisplayName = RP.Author)
// SELECT UPD.UserId, UPD.DisplayName, COALESCE(SUM(CASE WHEN UPD.PostRank = 1 THEN UPD.ViewCount END), 0) AS MostViewedPostCount, COALESCE(MIN(UPD.Score), 0) AS LowestScore,
//        COALESCE(AVG(UPD.CommentCount), 0) AS AvgComments, MAX(TAU.Reputation) AS Reputation
// FROM UserPostDetails UPD JOIN TopActiveUsers TAU ON UPD.UserId = TAU.UserId GROUP BY UPD.UserId, UPD.DisplayName HAVING COUNT(UPD.PostId) >= 3
// ORDER BY MAX(TAU.Reputation) DESC, MostViewedPostCount DESC;
//
// The join is on DisplayName, so the recent posts are indexed by their owner's name and every user of that name gets them. CommentCount runs over the
// comments x history rows.
fn q1963(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, view_count, .. } = &db.post;
    let recent = || db.post.with(creation_date.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).with(owner_user);
    let cc = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(history_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let byname: HashIdx<Str, Id<Post>> = recent().select(owner_user.select(&db.user.display_name)).inv().collect();
    let tp = db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let tau: MatSet<Id<User>> = db.user.with((&tp).filt(|n| n > 5)).collect();
    let mine = || (&db.user.display_name).select(&byname);
    let agg = (&tau).group_by(Ident::<User>::new()).select(mine().select(score.and(&cc))).fold((0i64, i64::MAX, 0i64), |(n, m, c), (s, k)| (n + 1, m.min(s), c + k));
    let w = (&tau).group_by(Ident::<User>::new()).select(mine().select(creation_date.and(view_count.opt()))).window(rank, |(d, _)| Reverse(d), asc);
    let mv = (&w).filt(|(_, k)| k == 1).fold(0i64, |s, ((_, w), _)| s + w.unwrap_or(0));
    let v = drain((&agg).filt(|(n, _, _)| n >= 3).and((&mv).opt()));
    rows(v.into_iter().map(|(u, ((n, m, c), w))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(w.unwrap_or(0)), V::I(m), V::F(c as f64 / n as f64)]);
        f.extend(ucols(db, u, &["rep"]));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.OwnerUserId, p.Title, p.CreationDate, p.Score, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p WHERE p.CreationDate >= '2024-10-01 12:34:56'::timestamp - INTERVAL '1 year'),
// UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(SUM(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END), 0) AS AcceptedAnswers,
//        COALESCE(COUNT(DISTINCT b.Id), 0) AS BadgeCount, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 8 GROUP BY u.Id, u.DisplayName, u.Reputation),
// RecentCloseVotes AS (SELECT ph.UserId, ph.PostId, p.Title, ph.CreationDate, COUNT(*) AS CloseVoteCount FROM PostHistory ph JOIN Posts p ON ph.PostId = p.Id
//     WHERE ph.PostHistoryTypeId = 10 AND ph.CreationDate >= '2024-10-01 12:34:56'::timestamp - INTERVAL '3 months' GROUP BY ph.UserId, ph.PostId, p.Title, ph.CreationDate),
// TopUsers AS (SELECT us.UserId, us.DisplayName, us.Reputation, us.AcceptedAnswers, us.BadgeCount, us.TotalBounty, ROW_NUMBER() OVER (ORDER BY us.Reputation DESC) AS UserRank FROM UserStatistics us WHERE us.Reputation > 0),
// MaxCloseVote AS (SELECT UserId, MAX(CloseVoteCount) AS MaxVotes FROM RecentCloseVotes GROUP BY UserId)
// SELECT tu.DisplayName, tu.Reputation, tu.AcceptedAnswers, tu.BadgeCount, tu.TotalBounty, COALESCE(mc.MaxVotes, 0) AS MaxCloseVotesByUser, COUNT(rp.PostId) AS RecentPostsCount
// FROM TopUsers tu LEFT JOIN MaxCloseVote mc ON tu.UserId = mc.UserId LEFT JOIN RankedPosts rp ON rp.OwnerUserId = tu.UserId WHERE tu.UserRank <= 10
// GROUP BY tu.DisplayName, tu.Reputation, tu.AcceptedAnswers, tu.BadgeCount, tu.TotalBounty, mc.MaxVotes ORDER BY tu.Reputation DESC;
//
// UserRank reads only Reputation, so the ten users are picked first (ties broken by user id) and the posts x badges x bounty-votes product is driven for
// them alone. PostRank is never read.
fn q33821(db: &'static So) -> String {
    let Post { creation_date, accepted_answer_id, .. } = &db.post;
    let top = top_n(drain(db.user.with((&db.user.reputation).gt(0)).select(&db.user.reputation)), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(top.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let bv = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let us = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(accepted_answer_id.opt().and(bv.opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 2], |a, (p, _)| match p {
            Some((x, b)) => [a[0] + x.is_some() as i64, a[1] + b.flatten().unwrap_or(0)],
            None => a,
        });
    let bc = (&tu).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let PostHistory { post_history_type_id, creation_date: hd, user, post, .. } = &db.post_history;
    type G = ((Id<User>, Id<Post>), i64);
    let rcv = db.post_history.with(post_history_type_id.eq(10)).with(hd.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -3))).group_by(user.and(post).and(hd)).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let rr = rel(drain(&rcv));
    let mc = (&rr).group_by(Same::<(G, i64)>::new().map(|x: (G, i64)| x.0 .0 .0)).select(Same::<(G, i64)>::new().map(|x: (G, i64)| x.1)).fold(0i64, |m, n| m.max(n));
    let rp = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(Ident::<Post>::new().with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let v = drain((&us).and(&bc).and((&mc).opt()).and(&rp));
    rows(v.into_iter().map(|(u, (((a, b), m), n))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(b), V::I(a[1]), V::I(m.unwrap_or(0)), V::I(n)]);
        row(f)
    }))
}

// WITH UserScores AS (SELECT U.Id AS UserId, U.Reputation, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 END), 0) AS DownVotes,
//        COUNT(DISTINCT P.Id) AS PostCount, COUNT(DISTINCT CASE WHEN P.PostTypeId = 1 THEN P.Id END) AS QuestionCount, COUNT(DISTINCT CASE WHEN P.PostTypeId = 2 AND P.AcceptedAnswerId IS NOT NULL THEN P.Id END) AS AcceptedAnswerCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.Reputation),
// PostSummary AS (SELECT P.Id AS PostId, P.Title, P.ViewCount, COALESCE(PC.CommentCount, 0) AS CommentCount, (SELECT COUNT(1) FROM Votes V WHERE V.PostId = P.Id AND V.VoteTypeId = 2) AS UpVoteCount,
//        (SELECT COUNT(1) FROM Votes V WHERE V.PostId = P.Id AND V.VoteTypeId = 3) AS DownVoteCount, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.LastActivityDate DESC) AS PostRank
//     FROM Posts P LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) PC ON P.Id = PC.PostId WHERE P.CreationDate >= (cast('2024-10-01' as date) - INTERVAL '1 year')),
// UserPostDetails AS (SELECT US.UserId, US.Reputation, PS.PostId, PS.Title, PS.ViewCount, PS.CommentCount, PS.UpVoteCount, PS.DownVoteCount FROM UserScores US JOIN PostSummary PS ON US.UserId = PS.PostId)
// SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.Views, UPS.PostId, UPS.Title, UPS.ViewCount, UPS.CommentCount, UPS.UpVoteCount, UPS.DownVoteCount,
//        CASE WHEN UPS.UpVoteCount > UPS.DownVoteCount THEN 'Positive' WHEN UPS.UpVoteCount < UPS.DownVoteCount THEN 'Negative' ELSE 'Neutral' END AS VoteSentiment, RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank
// FROM Users U LEFT JOIN UserPostDetails UPS ON U.Id = UPS.UserId WHERE (UPS.CommentCount > 0 OR UPS.ViewCount > 100) AND (UPS.Reputation IS NOT NULL OR U.Reputation > 0)
// ORDER BY VoteSentiment DESC, U.Reputation DESC LIMIT 100;
//
// Only Reputation is read from UserScores, so it is the users. `US.UserId = PS.PostId` joins a user id to a post id, so it goes through the raw ids; the
// WHERE drops the unmatched users. PostRank is never read. ReputationRank ranks the rows left after the WHERE.
fn q21518(db: &'static So) -> String {
    let Post { creation_date, view_count, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1)));
    let cc = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = recent().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    type X = (Id<Post>, (i64, [i64; 2]));
    let ps: MatSet<X> = recent()
        .select(Ident::<Post>::new().and((&cc).and(&vc)).and(view_count.opt()))
        .filt(|((_, (c, _)), w): (X, Option<i64>)| c > 0 || w.map_or(false, |w| w > 100))
        .map(|(x, _): (X, Option<i64>)| x)
        .collect();
    let by_id: HashIdx<i64, X> = (&ps).map(|x: X| x.0).select(&db.post.origid).inv().collect();
    let ux: HashIdx<Id<User>, X> = (&db.user.origid).select(&by_id).collect();
    let snt = |a: [i64; 2]| if a[0] > a[1] { "Positive" } else if a[0] < a[1] { "Negative" } else { "Neutral" };
    let w = whole(db.user.with(&ux)).select(Ident::<User>::new().and(&db.user.reputation).and(&ux)).window(rank, |((_, r), _)| Reverse(r), asc);
    let v: Vec<((Id<User>, X), i64)> = drain(&w).into_iter().map(|(_, (((u, _), x), k))| ((u, x), k)).collect();
    let v = top_n(v, |&((u, (p, (_, a))), _)| (Reverse(snt(a)), Reverse(db.user.reputation.get(u).unwrap()), u, p), 100);
    rows(v.into_iter().map(|((u, (p, (c, a))), k)| {
        let mut f = ucols(db, u, &["uid", "name", "rep", "uviews"]);
        f.extend(post_fields(db, p, &["id", "title", "views"]));
        f.extend([V::I(c), V::I(a[0]), V::I(a[1]), V::S(snt(a)), V::I(k)]);
        row(f)
    }))
}

// WITH UserVoteCounts AS (SELECT U.Id AS UserId, COUNT(V.Id) AS TotalVotes, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        SUM(CASE WHEN V.VoteTypeId IN (10, 12) THEN 1 ELSE 0 END) AS DeleteVotes FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id),
// PostStatistics AS (SELECT P.Id AS PostId, P.Title, P.OwnerUserId, COUNT(C.Id) AS CommentCount, COUNT(DISTINCT PL.RelatedPostId) AS RelatedPostCount, MAX(COALESCE(PH.CreationDate, '1970-01-01'::timestamp)) AS LastHistoryDate,
//        RANK() OVER (PARTITION BY P.OwnerUserId ORDER BY COUNT(DISTINCT C.Id) DESC) AS RankByComments
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN PostLinks PL ON P.Id = PL.PostId LEFT JOIN PostHistory PH ON P.Id = PH.PostId
//     WHERE P.CreationDate < TIMESTAMP '2024-10-01 12:34:56' AND (P.ClosedDate IS NULL OR P.ClosedDate > TIMESTAMP '2024-10-01 12:34:56') GROUP BY P.Id, P.Title, P.OwnerUserId),
// UserPostPerformance AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, UP.TotalVotes, UP.UpVotes, UP.DownVotes, BP.PostId, BP.Title, BP.CommentCount, BP.RelatedPostCount, BP.LastHistoryDate,
//        CASE WHEN BP.RankByComments <= 5 THEN 'High Engagement' WHEN BP.RankByComments > 5 AND BP.RankByComments <= 10 THEN 'Moderate Engagement' ELSE 'Low Engagement' END AS EngagementLevel
//     FROM Users U JOIN UserVoteCounts UP ON U.Id = UP.UserId JOIN PostStatistics BP ON U.Id = BP.OwnerUserId WHERE U.Reputation > 1000)
// SELECT U.DisplayName, U.Reputation, U.TotalVotes, U.UpVotes, U.DownVotes, U.CommentCount, U.RelatedPostCount, U.LastHistoryDate, U.EngagementLevel,
//        CASE WHEN U.LastHistoryDate IS NULL THEN 'No activity' WHEN U.LastHistoryDate < TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' THEN 'Inactive' ELSE 'Active' END AS ActivityStatus
// FROM UserPostPerformance U WHERE U.TotalVotes > 0 AND U.EngagementLevel = 'High Engagement' ORDER BY U.Reputation DESC, U.TotalVotes DESC;
//
// Only owners with Reputation > 1000 and a vote reach the output, and RankByComments partitions by owner, so PostStatistics is driven for those owners'
// posts alone. CommentCount runs over the comments x links x history rows.
fn q23689(db: &'static So) -> String {
    let Post { creation_date, closed_date, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let uv = db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id)).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64]);
    let owners: MatSet<Id<User>> = db.user.with(&uv).collect();
    let ps = || (&owners).select(posts_of(db)).with(creation_date.lt(t0)).with(Ident::<Post>::new().minus(closed_date).or(closed_date.gt(t0)));
    let hd = &db.post_history.creation_date;
    let st = ps().group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(links_of(db).opt()).and(history_of(db).select(hd).opt())).fold((0i64, i64::MIN), |(n, m), ((c, _), h)| {
        (n + c.is_some() as i64, m.max(h.unwrap_or(date(1970, 1, 1))))
    });
    let dc = ps().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let rl = ps().group_by(Ident::<Post>::new()).select(links_of(db).select(&db.post_link.related_post_id)).count_distinct();
    let w = ps().select(Ident::<Post>::new().and((&st).and(&dc).and((&rl).opt()))).window(rank, |(_, ((_, d), _))| Reverse(d), asc);
    let v = drain((&w).filt(|(_, k)| k <= 5).and(&uv));
    rows(v.into_iter().map(|(u, (((_, (((n, m), _), l)), _), a))| {
        let l = l.unwrap_or(0);
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(n), V::I(l), V::T(m), V::S("High Engagement")]);
        f.push(V::S(if m < add_years(t0, -1) { "Inactive" } else { "Active" }));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, COUNT(DISTINCT c.Id) AS TotalComments, SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounty,
//        SUM(COALESCE(p.Score, 0)) AS TotalScore, RANK() OVER (ORDER BY COUNT(DISTINCT p.Id) DESC) AS PostRank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON u.Id = c.UserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 8 GROUP BY u.Id, u.DisplayName),
// RecentPostHistory AS (SELECT ph.PostId, ph.PostHistoryTypeId, ph.CreationDate, ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS rn FROM PostHistory ph
//     WHERE ph.CreationDate > (TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year') AND ph.PostHistoryTypeId IN (10, 11, 12)),
// PostStats AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.AnswerCount, (SELECT COUNT(*) FROM PostHistory ph WHERE ph.PostId = p.Id AND ph.PostHistoryTypeId = 6) AS EditTagCount,
//        (SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id) AS CommentCount, (SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 3) AS DownvoteCount,
//        (SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 2) AS UpvoteCount
//     FROM Posts p WHERE p.CreationDate BETWEEN (TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '2 year') AND TIMESTAMP '2024-10-01 12:34:56')
// SELECT ua.DisplayName, ua.TotalPosts, ua.TotalComments, ua.TotalBounty, ps.PostId, ps.Title, ps.ViewCount, ps.AnswerCount, ps.EditTagCount, ps.CommentCount, ps.DownvoteCount, ps.UpvoteCount,
//        CASE WHEN rp.PostId IS NOT NULL THEN 'Post has recently changed state' ELSE 'No recent changes' END AS RecentChangeStatus
// FROM UserActivity ua JOIN PostStats ps ON ua.UserId = ps.PostId LEFT JOIN RecentPostHistory rp ON ps.PostId = rp.PostId AND rp.rn = 1
// WHERE ua.TotalPosts > 5 AND ua.TotalBounty > 0 AND EXISTS (SELECT 1 FROM Badges b WHERE b.UserId = ua.UserId AND b.Class = 1) ORDER BY ua.TotalBounty DESC, ua.TotalPosts DESC;
//
// `ua.UserId = ps.PostId` joins a user id to a post id, so it goes through the raw ids, and the posts x comments x bounty-votes product is driven for the
// matched users alone. The rn = 1 row exists exactly when the post has such history. PostRank and TotalScore are never read.
fn q24793(db: &'static So) -> String {
    let Post { creation_date, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let pidx: HashIdx<i64, Id<Post>> = db.post.with(creation_date.ge(add_years(t0, -2)).and(creation_date.le(t0))).select(&db.post.origid).inv().collect();
    let gold = Ident::<User>::new().with(badges_of(db).select(Ident::<Badge>::new().with((&db.badge.class).eq(1))));
    let mu: MatSet<Id<User>> = db.user.with((&db.user.origid).select(&pidx)).select(gold).collect();
    let bv = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let ub = (&mu).group_by(Ident::<User>::new()).select(posts_of(db).select(bv.opt()).opt().and(comments_by(db).opt())).fold(0i64, |s, (b, _)| s + b.flatten().flatten().unwrap_or(0));
    let pc = (&mu).group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let ccu = (&mu).group_by(Ident::<User>::new()).select(comments_by(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let PostHistory { post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let et = db.post_history.with(post_history_type_id.eq(6)).group_by(&db.post_history.post).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let vc = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 3) as i64, a[1] + (t == 2) as i64]);
    let rph = Ident::<Post>::new().with(history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.is_in([10, 11, 12])).with(hd.gt(add_years(t0, -1)))));
    let v = drain((&ub).filt(|b| b > 0).and((&pc).filt(|n| n > 5)).and(&ccu).and((&db.user.origid).select(&pidx).select(Ident::<Post>::new().and((&et).opt()).and((&cc).opt()).and((&vc).opt()).and(rph.opt()))));
    rows(v.into_iter().map(|(u, (((b, n), c), ((((p, e), k), a), r)))| {
        let a = a.unwrap_or([0, 0]);
        let mut f = vec![user_col(db, u, "name"), V::I(n), V::I(c), V::I(b)];
        f.extend(post_fields(db, p, &["id", "title", "views", "answers"]));
        f.extend([V::I(e.unwrap_or(0)), V::I(k.unwrap_or(0)), V::I(a[0]), V::I(a[1])]);
        f.push(V::S(if r.is_some() { "Post has recently changed state" } else { "No recent changes" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.Score, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate ASC) AS RankScore,
//        COALESCE((SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 2), 0) AS UpVotes, COALESCE((SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 3), 0) AS DownVotes
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// PostDetails AS (SELECT rp.PostId, rp.Title, rp.ViewCount, rp.Score, rp.RankScore, rp.UpVotes, rp.DownVotes,
//        CASE WHEN rp.UpVotes + rp.DownVotes > 0 THEN (CAST(rp.UpVotes AS FLOAT) / (rp.UpVotes + rp.DownVotes)) * 100 ELSE NULL END AS UpVotePercentage,
//        CASE WHEN rp.ViewCount > 1000 THEN 'Popular' ELSE 'Less Popular' END AS Popularity FROM RankedPosts rp WHERE rp.RankScore <= 10),
// PostHistories AS (SELECT ph.PostId, ph.PostHistoryTypeId, ph.CreationDate, COALESCE(COUNT(DISTINCT ph.Id), 0) AS EditCount FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (4, 5, 6)
//     GROUP BY ph.PostId, ph.PostHistoryTypeId, ph.CreationDate)
// SELECT pd.PostId, pd.Title, pd.ViewCount, pd.Score, pd.UpVotePercentage, pd.Popularity, ph.EditCount, CASE WHEN ph.EditCount IS NULL THEN 'No Edits' ELSE 'Edited' END AS EditStatus,
//        CASE WHEN pd.Score > 10 AND pd.Popularity = 'Popular' THEN 'High Engagement' WHEN pd.Score <= 10 AND pd.Popularity = 'Less Popular' THEN 'Low Engagement' ELSE 'Moderate Engagement' END AS EngagementLevel
// FROM PostDetails pd LEFT JOIN PostHistories ph ON pd.PostId = ph.PostId WHERE pd.UpVotePercentage IS NOT NULL ORDER BY pd.ViewCount DESC, pd.Score DESC;
//
// RankScore reads only base columns, so the posts are picked first (ties broken by post id). FLOAT is a 32-bit REAL. Each (type, date) group of a post's
// edits is its own row.
fn q23776(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, view_count, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(post_type_id).select(Ident::<Post>::new().and(score).and(creation_date)).window(row_number, |((p, s), d)| (Reverse(s), d, p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 10).map(|(((p, _), _), _)| p).collect();
    let ud = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let PostHistory { post_history_type_id, post, creation_date: hd, .. } = &db.post_history;
    type G = ((Id<Post>, i64), i64);
    let phs = db.post_history.with(post_history_type_id.is_in([4, 5, 6])).group_by(post.and(post_history_type_id).and(hd)).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let pr = rel(drain(&phs));
    let phi: HashIdx<Id<Post>, i64> = (&pr).map(|x: (G, i64)| x.0 .0 .0).inv().select(&pr).map(|x: (G, i64)| x.1).collect();
    let v = drain((&ud).filt(|a| a[0] + a[1] > 0).and((&phi).opt()));
    rows(v.into_iter().map(|(p, (a, e))| {
        let s = score.get(p).unwrap();
        let pop = view_count.get(p).map_or(false, |w| w > 1000);
        let pct = a[0] as f32 / (a[0] + a[1]) as f32 * 100.0f32;
        let mut f = post_fields(db, p, &["id", "title", "views", "score"]);
        f.extend([V::F(pct as f64), V::S(if pop { "Popular" } else { "Less Popular" }), oint(e), V::S(if e.is_none() { "No Edits" } else { "Edited" })]);
        f.push(V::S(if s > 10 && pop { "High Engagement" } else if s <= 10 && !pop { "Low Engagement" } else { "Moderate Engagement" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS UserRank, p.OwnerUserId
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.Score IS NOT NULL),
// UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
//        COUNT(DISTINCT ph.Id) FILTER (WHERE ph.PostHistoryTypeId IN (10, 11)) AS CloseReopenCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId GROUP BY u.Id, u.DisplayName, u.Reputation),
// PostsWithBadges AS (SELECT rp.PostId, rp.Title, rp.CreationDate, us.UserId, us.DisplayName, us.Reputation, us.UpVotes, us.DownVotes, COUNT(b.Id) AS BadgeCount FROM RankedPosts rp
//     JOIN UserStatistics us ON rp.OwnerUserId = us.UserId LEFT JOIN Badges b ON us.UserId = b.UserId GROUP BY rp.PostId, rp.Title, rp.CreationDate, us.UserId, us.DisplayName, us.Reputation, us.UpVotes, us.DownVotes),
// FinalResults AS (SELECT p.Title, p.CreationDate, p.UpVotes, p.DownVotes, p.BadgeCount, CASE WHEN p.BadgeCount = 0 THEN 'No Badges' WHEN p.BadgeCount < 3 THEN 'Few Badges' ELSE 'Many Badges' END AS BadgeCategory
//     FROM PostsWithBadges p WHERE p.UpVotes - p.DownVotes > 10)
// SELECT fr.Title, fr.CreationDate, fr.UpVotes, fr.DownVotes, fr.BadgeCategory FROM FinalResults fr ORDER BY fr.UpVotes DESC, fr.CreationDate DESC LIMIT 10;
//
// UserRank and CloseReopenCount are never read. The vote sums run over the owner's posts x votes x history rows, driven for the owners of recent posts alone.
fn q3136(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let rp = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let owners: MatSet<Id<User>> = rp().select(owner_user).collect();
    let us = (&owners)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt().and(history_of(db).opt())).opt())
        .fold([0i64; 2], |a, x| {
            let t = x.and_then(|(t, _)| t);
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
        });
    let bc = (&owners).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = drain(rp().select(Ident::<Post>::new().and(owner_user.select((&us).filt(|a| a[0] - a[1] > 10).and(&bc)))));
    let v = top_n(v, |&(_, (p, (a, _)))| (Reverse(a[0]), Reverse(creation_date.get(p).unwrap()), p), 10);
    rows(v.into_iter().map(|(_, (p, (a, b)))| {
        let mut f = post_fields(db, p, &["title", "created"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::S(if b == 0 { "No Badges" } else if b < 3 { "Few Badges" } else { "Many Badges" })]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.Score IS NOT NULL THEN p.Score ELSE 0 END) AS TotalScore, MAX(p.CreationDate) AS LastPostDate
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId WHERE u.CreationDate < cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY u.Id, u.DisplayName, u.Reputation),
// ActiveUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, TotalScore, LastPostDate, ROW_NUMBER() OVER (ORDER BY Reputation DESC, PostCount DESC) AS Rank FROM UserStats WHERE PostCount > 0),
// RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.OwnerUserId, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RecentRank FROM Posts p
//     WHERE p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// PostVoteStats AS (SELECT v.PostId, SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpvoteCount, SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownvoteCount FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY v.PostId)
// SELECT au.DisplayName, au.Reputation, p.Title, p.ViewCount, COALESCE(pv.UpvoteCount, 0) AS Upvotes, COALESCE(pv.DownvoteCount, 0) AS Downvotes, CASE WHEN au.Rank <= 10 THEN 'Top Contributor' ELSE 'Active Contributor' END AS ContributorType,
//        CASE WHEN r.RecentRank = 1 THEN 'Most Recent Post' ELSE 'Previous Post' END AS PostRecency
// FROM ActiveUsers au JOIN RecentPosts r ON au.UserId = r.OwnerUserId JOIN Posts p ON r.PostId = p.Id LEFT JOIN PostVoteStats pv ON p.Id = pv.PostId WHERE (p.ViewCount > 100 OR pv.UpvoteCount > 10)
// ORDER BY au.Reputation DESC, p.ViewCount DESC;
//
// Rank breaks (Reputation, PostCount) ties by user id and RecentRank breaks CreationDate ties by post id (the SQL leaves both open).
fn q21817(db: &'static So) -> String {
    let Post { creation_date, owner_user, view_count, .. } = &db.post;
    let pc = db.user.with((&db.user.creation_date).lt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let uw = whole(db.user.with(&pc)).select(Ident::<User>::new().and(&db.user.reputation).and(&pc)).window(row_number, |((u, r), n)| (Reverse(r), Reverse(n), u), asc);
    let au = by_first(&(&uw).map(|(((u, _), _), k)| (u, k)).collect());
    let rp: MatSet<Id<Post>> = db.post.with(creation_date.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).with(owner_user).collect();
    let rw = (&rp).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let first: MatSet<Id<Post>> = (&rw).filt(|(_, k)| k == 1).map(|((p, _), _)| p).collect();
    let pv = (&rp).group_by(Ident::<Post>::new()).select(votes_of(db).select(vtype_name(db))).fold([0i64; 2], |a, n| [a[0] + (n == "UpMod") as i64, a[1] + (n == "DownMod") as i64]);
    type X = (Id<Post>, ((Option<[i64; 2]>, (Id<User>, i64)), Option<Id<Post>>));
    let v = drain(
        (&rp)
            .select(Ident::<Post>::new().and((&pv).opt().and(owner_user.select(Ident::<User>::new().and(&au))).and(Ident::<Post>::new().with(&first).opt())).and(view_count.opt()))
            .filt(|((_, ((a, _), _)), w): (X, Option<i64>)| w.map_or(false, |w| w > 100) || a.map_or(false, |a| a[0] > 10))
            .map(|(x, _): (X, Option<i64>)| x),
    );
    rows(v.into_iter().map(|(_, (p, ((a, (u, k)), f1)))| {
        let a = a.unwrap_or([0, 0]);
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["title", "views"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::S(if k <= 10 { "Top Contributor" } else { "Active Contributor" })]);
        f.push(V::S(if f1.is_some() { "Most Recent Post" } else { "Previous Post" }));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(COALESCE(P.ViewCount, 0)) AS TotalViews, SUM(COALESCE(P.Score, 0)) AS TotalScore, MAX(P.CreationDate) AS LastPostDate
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId WHERE U.Reputation > 100 GROUP BY U.Id, U.DisplayName),
// RecentActiveUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalViews, TotalScore, LastPostDate, RANK() OVER (ORDER BY LastPostDate DESC) AS ActivityRank FROM UserActivity),
// TopPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.ViewCount, P.Score, P.OwnerUserId, U.DisplayName AS OwnerDisplayName, RANK() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC) AS PostRank
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' AND P.Score IS NOT NULL),
// RecentComments AS (SELECT C.PostId, COUNT(C.Id) AS CommentCount, SUM(CASE WHEN C.UserId IS NOT NULL THEN 1 ELSE 0 END) AS UserComments FROM Comments C GROUP BY C.PostId),
// PostLinkStats AS (SELECT PL.PostId, COUNT(PL.RelatedPostId) AS RelatedLinks FROM PostLinks PL GROUP BY PL.PostId)
// SELECT U.UserId, U.DisplayName, U.TotalPosts, U.TotalScore, U.TotalViews, U.LastPostDate, COALESCE(PP.Title, 'No Posts') AS PopularPostTitle, PP.ViewCount AS PopularPostViews,
//        COALESCE(PC.CommentCount, 0) AS CommentCount, COALESCE(PL.RelatedLinks, 0) AS RelatedLinks
// FROM RecentActiveUsers U LEFT JOIN TopPosts PP ON U.UserId = PP.OwnerUserId AND PP.PostRank = 1 LEFT JOIN RecentComments PC ON PP.PostId = PC.PostId LEFT JOIN PostLinkStats PL ON PP.PostId = PL.PostId
// WHERE U.ActivityRank <= 10 ORDER BY U.LastPostDate DESC;
//
// ActivityRank sorts a missing LastPostDate last.
fn q20696(db: &'static So) -> String {
    let Post { creation_date, owner_user, view_count, score, post_type_id, title, .. } = &db.post;
    let ua = db.user.with((&db.user.reputation).gt(100)).group_by(Ident::<User>::new()).select(posts_of(db).select(view_count.opt().and(score).and(creation_date)).opt()).fold([0i64, 0, 0, i64::MIN], |a, p| match p {
        Some(((w, s), d)) => [a[0] + 1, a[1] + w.unwrap_or(0), a[2] + s, a[3].max(d)],
        None => a,
    });
    let uw = whole(db.user.with(&ua)).select(Ident::<User>::new().and(&ua)).window(rank, |(_, a)| (a[3] == i64::MIN, Reverse(a[3])), asc);
    type R = ((Id<User>, [i64; 4]), i64);
    let tu: MatSet<R> = (&uw).filt(|(_, k)| k <= 10).collect();
    let pw = db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).with(owner_user).group_by(post_type_id).select(Ident::<Post>::new().and(score)).window(rank, |(_, s)| Reverse(s), asc);
    let tp: MatSet<Id<Post>> = (&pw).filt(|(_, k)| k == 1).map(|((p, _), _)| p).collect();
    let ti: HashIdx<Id<User>, Id<Post>> = (&tp).select(owner_user).inv().collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let lc = (&tp).group_by(Ident::<Post>::new()).select(links_of(db)).fold(0i64, |n, _| n + 1);
    let v = drain((&tu).select(Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0 .0).select((&ti).select(Ident::<Post>::new().and((&cc).opt()).and((&lc).opt()))).opt())));
    rows(v.into_iter().map(|(_, (((u, a), _), p))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[2]), V::I(a[1]), tmax(a[3])]);
        match p {
            Some(((p, c), l)) => {
                f.push(V::S(title.get(p).unwrap_or("No Posts")));
                f.extend(post_fields(db, p, &["views"]));
                f.extend([V::I(c.unwrap_or(0)), V::I(l.unwrap_or(0))]);
            }
            None => f.extend([V::S("No Posts"), V::Null, V::I(0), V::I(0)]),
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.Tags, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS PostRank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' AND p.Score IS NOT NULL),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, u.DisplayName, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes,
//        COUNT(DISTINCT b.Id) AS BadgeCount FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation, u.DisplayName),
// PostWithVoteCounts AS (SELECT p.Id AS PostId, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpvoteCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownvoteCount
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, rp.CommentCount, rp.Tags, ur.DisplayName AS Author, ur.Reputation AS AuthorReputation, pwc.UpvoteCount, pwc.DownvoteCount,
//        CASE WHEN rp.Score > 10 AND ur.Reputation >= 1000 THEN 'High Engagement' WHEN rp.Score <= 0 AND ur.Reputation < 100 THEN 'Low Engagement' ELSE 'Moderate Engagement' END AS EngagementLevel,
//        CASE WHEN rp.PostRank <= 5 THEN 'Top Post' ELSE 'Regular Post' END AS PostCategory
// FROM RankedPosts rp JOIN Users ur ON rp.PostId IN (SELECT AcceptedAnswerId FROM Posts WHERE Id = rp.PostId) LEFT JOIN PostWithVoteCounts pwc ON rp.PostId = pwc.PostId
// WHERE rp.PostRank <= 10 AND ur.Reputation IS NOT NULL ORDER BY rp.Score DESC, ur.Reputation DESC;
//
// The ON names no column of ur and asks that the post be its own accepted answer, so the qualifying posts are crossed with every user. UserReputation
// is never read.
fn q21561(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, accepted_answer_id, origid, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(accepted_answer_id.opt()).and(origid))
        .window(rank, |(((_, s), _), _)| Reverse(s), asc);
    type R = (Id<Post>, i64);
    let pw: MatSet<R> = (&w).filt(|(((_, a), o), k)| k <= 10 && a == Some(o)).map(|((((p, _), _), _), k)| (p, k)).collect();
    let pwc = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let pp: MatSet<(R, [i64; 2])> = (&pw).select(Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0).select(&pwc))).collect();
    let mut v = Vec::new();
    (&pp).cross(&db.user.reputation).drive(|(_, u), (((p, k), a), _)| v.push((p, k, a, u)));

    rows(v.into_iter().map(|(p, k, a, u)| {
        let s = score.get(p).unwrap();
        let rep = db.user.reputation.get(u).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "tags"]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.push(V::S(if s > 10 && rep >= 1000 { "High Engagement" } else if s <= 0 && rep < 100 { "Low Engagement" } else { "Moderate Engagement" }));
        f.push(V::S(if k <= 5 { "Top Post" } else { "Regular Post" }));
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpVotes, COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownVotes,
//        COUNT(DISTINCT P.Id) AS TotalPosts, COUNT(DISTINCT C.Id) AS TotalComments
//     FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, ROW_NUMBER() OVER (ORDER BY Reputation DESC, TotalUpVotes DESC) AS Rank FROM UserStatistics),
// RecentPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.OwnerUserId, COUNT(C.Id) AS CommentCount FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId
//     WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' GROUP BY P.Id, P.Title, P.CreationDate, P.OwnerUserId),
// UserPostComments AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS UserPostCount, SUM(COALESCE(C.Score, 0)) AS TotalCommentScore FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId
//     LEFT JOIN Comments C ON P.Id = C.PostId GROUP BY U.Id, U.DisplayName)
// SELECT U.DisplayName, U.Reputation, U.TotalPosts, U.TotalComments, RP.Title AS RecentPostTitle, RP.CommentCount, CASE WHEN U.Reputation IS NULL THEN 'Unknown' WHEN U.Reputation >= 1000 THEN 'Elite' ELSE 'Regular' END AS UserTier,
//        (SELECT COUNT(*) FROM UserPostComments) AS TotalUserComments, U.TotalUpVotes - U.TotalDownVotes AS NetVotes
// FROM UserStatistics U LEFT JOIN TopUsers TU ON U.UserId = TU.UserId LEFT JOIN RecentPosts RP ON U.UserId = RP.OwnerUserId WHERE TU.Rank <= 10 ORDER BY U.Reputation DESC;
//
// The vote sums run over the votes x posts x comments rows. UserPostComments has one row per user, so its COUNT(*) is the user count. Rank breaks
// (Reputation, TotalUpVotes) ties by user id.
fn q2788(db: &'static So) -> String {
    let Post { creation_date, .. } = &db.post;
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt().and(posts_of(db).select(comments_of(db).opt()).opt()))
        .fold([0i64; 2], |a, (t, _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let top = top_n(drain(&us), |&(u, a)| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(a[0]), u), 10);
    let tu = rel(top);
    let tuu: MatSet<Id<User>> = (&tu).map(|x: (Id<User>, [i64; 2])| x.0).collect();
    let pc = (&tuu).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let ccu = (&tuu).group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let recent = posts_of(db).select(Ident::<Post>::new().with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))));
    let rc = db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let nu = count(&db.user.origid);
    type T = (Id<User>, [i64; 2]);
    let v = drain((&tu).select(Same::<T>::new().and(Same::<T>::new().map(|x: T| x.0).select((&pc).and(&ccu).and(recent.select(Ident::<Post>::new().and(&rc)).opt())))));
    rows(v.into_iter().map(|(_, ((u, a), ((n, c), p)))| {
        let r = db.user.reputation.get(u).unwrap();
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(n), V::I(c)]);
        match p {
            Some((p, k)) => {
                f.extend(post_fields(db, p, &["title"]));
                f.push(V::I(k));
            }
            None => f.extend([V::Null, V::Null]),
        }
        f.extend([V::S(if r >= 1000 { "Elite" } else { "Regular" }), V::I(nu), V::I(a[0] - a[1])]);
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
    ("31171", q31171),
    ("30128", q30128),
    ("31091", q31091),
    ("4709", q4709),
    ("5116", q5116),
    ("24522", q24522),
    ("24013", q24013),
    ("30187", q30187),
    ("24223", q24223),
    ("4147", q4147),
    ("24209", q24209),
    ("22130", q22130),
    ("30155", q30155),
    ("21627", q21627),
    ("22419", q22419),
    ("20303", q20303),
    ("4974", q4974),
    ("20867", q20867),
    ("4937", q4937),
    ("20034", q20034),
    ("21609", q21609),
    ("33187", q33187),
    ("63", q63),
    ("22793", q22793),
    ("23440", q23440),
    ("1090", q1090),
    ("4406", q4406),
    ("30052", q30052),
    ("24072", q24072),
    ("20834", q20834),
    ("24062", q24062),
    ("22310", q22310),
    ("24792", q24792),
    ("7966", q7966),
    ("24219", q24219),
    ("32493", q32493),
    ("24555", q24555),
    ("920", q920),
    ("1512", q1512),
    ("23823", q23823),
    ("21441", q21441),
    ("3608", q3608),
    ("22389", q22389),
    ("24801", q24801),
    ("3447", q3447),
    ("20590", q20590),
    ("22348", q22348),
    ("4941", q4941),
    ("22664", q22664),
    ("20277", q20277),
    ("20471", q20471),
    ("4454", q4454),
    ("5946", q5946),
    ("30174", q30174),
    ("5385", q5385),
    ("24664", q24664),
    ("22804", q22804),
    ("23847", q23847),
    ("25196", q25196),
    ("20733", q20733),
    ("24635", q24635),
    ("22180", q22180),
    ("21633", q21633),
    ("21849", q21849),
    ("21499", q21499),
    ("23449", q23449),
    ("22403", q22403),
    ("20102", q20102),
    ("22400", q22400),
    ("1963", q1963),
    ("33821", q33821),
    ("21518", q21518),
    ("23689", q23689),
    ("24793", q24793),
    ("23776", q23776),
    ("3136", q3136),
    ("21817", q21817),
    ("20696", q20696),
    ("21561", q21561),
    ("2788", q2788),
];
