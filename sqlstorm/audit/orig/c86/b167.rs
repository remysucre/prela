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


// WITH UserReputation AS (SELECT Id, DisplayName, Reputation, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM Users),
// PostStats AS (SELECT p.Id AS PostId, p.OwnerUserId, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) - SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Score
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.Id, p.OwnerUserId),
// TopUsers AS (SELECT ur.DisplayName, ur.ReputationRank, ps.PostId, ps.CommentCount, ps.UpVotes, ps.DownVotes, ps.Score
//     FROM UserReputation ur LEFT JOIN PostStats ps ON ur.Id = ps.OwnerUserId WHERE ur.ReputationRank <= 10),
// ClosedPosts AS (SELECT ph.PostId, COUNT(*) AS CloseCount FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.PostId)
// SELECT tu.DisplayName, tu.ReputationRank, tu.PostId, COALESCE(cp.CloseCount, 0) AS CloseCount, tu.CommentCount, tu.UpVotes, tu.DownVotes, tu.Score
// FROM TopUsers tu LEFT JOIN ClosedPosts cp ON tu.PostId = cp.PostId ORDER BY tu.ReputationRank, tu.Score DESC;
fn q356(db: &'static So) -> String {
    let rr = rel(ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false).into_iter().map(|((u, _), r)| (u, r)).collect());
    let rank: HashIdx<Id<User>, i64> = (&rr).map(|(u, _)| u).inv().select((&rr).map(|(_, r)| r)).collect();
    let top: MatSet<Id<User>> = db.user.with((&rank).le(10)).collect();
    let recent = Ident::<Post>::new().with((&db.post.creation_date).ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let tp: MatSet<Id<Post>> = (&top).select(posts_of(db).select(recent)).collect();
    let ps = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).in_v(vec![10, 11])));
    let cc = (&tp).group_by(Ident::<Post>::new()).select(closes.opt()).fold(0i64, |n, h| n + h.is_some() as i64);
    let v = drain((&top).select((&rank).and(posts_of(db).select(Ident::<Post>::new().and(&ps).and(&cc)).opt())));
    rows(v.into_iter().map(|(u, (r, p))| {
        let mut f = vec![user_col(db, u, "name"), V::I(r)];
        f.extend(match p {
            Some(((p, a), c)) => [post_fields(db, p, &["id"]).pop().unwrap(), V::I(c), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[1] - a[2])],
            None => [V::Null, V::I(0), V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, u.DisplayName AS OwnerName, p.CreationDate, p.ViewCount, p.Score,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate ASC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.OwnerName, rp.CreationDate, rp.ViewCount, rp.Score FROM RankedPosts rp WHERE rp.Rank <= 5),
// PostStats AS (SELECT tp.PostId, tp.Title, tp.OwnerName, tp.CreationDate, tp.ViewCount, tp.Score, COUNT(c.Id) AS CommentCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes
//     FROM TopPosts tp LEFT JOIN Comments c ON tp.PostId = c.PostId LEFT JOIN Votes v ON tp.PostId = v.PostId
//     GROUP BY tp.PostId, tp.Title, tp.OwnerName, tp.CreationDate, tp.ViewCount, tp.Score)
// SELECT ps.OwnerName, ps.Title, ps.CreationDate, ps.ViewCount, ps.Score, ps.CommentCount, ps.UpVotes, ps.DownVotes
// FROM PostStats ps ORDER BY ps.Score DESC, ps.ViewCount DESC FETCH FIRST 10 ROWS ONLY;
fn q7891(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), creation_date.get(p).unwrap(), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = top_n(drain(&s), |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    }, 10);
    rows(v.into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["owner", "title", "created", "views", "score"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RecursivePostHistory AS (SELECT ph.PostId, ph.UserId, ph.PostHistoryTypeId, ph.CreationDate, ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate) AS rn
//     FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11)),
// PostStats AS (SELECT p.Id AS PostId, COUNT(DISTINCT c.Id) AS CommentCount, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, MAX(b.Class) AS HighestBadgeClass FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id)
// SELECT ps.PostId, ps.CommentCount, ps.UpVotes, ps.DownVotes, ps.TotalBounty, RPH.UserId AS LastUserToClose, RPH.CreationDate AS LastCloseDate,
//        ub.BadgeCount AS UserBadgeCount, ub.HighestBadgeClass
// FROM PostStats ps LEFT JOIN RecursivePostHistory RPH ON ps.PostId = RPH.PostId AND RPH.rn = 1 LEFT JOIN Users u ON RPH.UserId = u.Id
// LEFT JOIN UserBadges ub ON ub.UserId = u.Id WHERE ps.TotalBounty > 0 OR ps.CommentCount > 5 ORDER BY ps.TotalBounty DESC, ps.CommentCount DESC;
//
// No CTE is recursive. The first close/reopen row of a post breaks a CreationDate tie on the history id.
fn q32048(db: &'static So) -> String {
    let ps = db
        .post
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select((&db.vote.vote_type_id).and((&db.vote.bounty_amount).opt())).opt()))
        .fold([0i64; 3], |a, (_, v)| {
            let (t, b) = v.map_or((0, None), |(t, b)| (t, b));
            [a[0] + b.unwrap_or(0), a[1] + (t == 2) as i64, a[2] + (t == 3) as i64]
        });
    let dc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let keep = (&ps).and(&dc).filt(|(a, c): ([i64; 3], i64)| a[0] > 0 || c > 5);
    let PostHistory { post, post_history_type_id, creation_date: hd, user, user_id, .. } = &db.post_history;
    let cl = drain(db.post_history.with(post_history_type_id.in_v(vec![10, 11])).select(post));
    let cl = top_per(cl, |&(_, p)| p, |&(h, _)| (hd.get(h).unwrap(), h), 1, false);
    let cl = rel(cl.into_iter().map(|(h, p)| (p, h)).collect());
    let first: HashIdx<Id<Post>, Id<PostHistory>> = (&cl).map(|(p, _)| p).inv().select((&cl).map(|(_, h)| h)).collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold((0i64, i64::MIN), |(n, m), c| match c {
        Some(c) => (n + 1, m.max(c)),
        None => (n, m),
    });
    let v = drain(keep.and((&first).select(Ident::<PostHistory>::new().and(user.select(&ub).opt())).opt()));
    rows(v.into_iter().map(|(p, ((a, c), h))| {
        let mut f = post_fields(db, p, &["id"]);
        f.extend([V::I(c), V::I(a[1]), V::I(a[2]), V::I(a[0])]);
        f.extend(match h {
            Some((h, b)) => [
                oint(user_id.get(h)),
                V::T(hd.get(h).unwrap()),
                oint(b.map(|b| b.0)),
                b.map_or(V::Null, |(n, m)| if n == 0 { V::Null } else { V::I(m) }),
            ],
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn, p.OwnerUserId
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.Score > 0 AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserStats AS (SELECT u.Id AS UserId, COUNT(DISTINCT p.Id) AS PostCount, SUM(v.BountyAmount) AS TotalBounties, COUNT(DISTINCT b.Id) AS BadgeCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 8 LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// TopUsers AS (SELECT us.UserId, us.PostCount, us.TotalBounties, us.BadgeCount, RANK() OVER (ORDER BY us.PostCount DESC, us.TotalBounties DESC) AS user_rank FROM UserStats us)
// SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.CreationDate, rp.OwnerDisplayName, tu.user_rank, tu.PostCount, tu.TotalBounties, tu.BadgeCount
// FROM RankedPosts rp LEFT JOIN TopUsers tu ON rp.OwnerUserId = tu.UserId WHERE tu.user_rank <= 10 OR tu.user_rank IS NULL ORDER BY rp.ViewCount DESC, rp.Score DESC;
//
// user_rank leads with the distinct post count, so only users with at least the tenth-highest count can rank in the top ten; the
// posts x bounty votes x badges product is driven for those alone. Every RankedPosts owner is a user, so user_rank is never NULL.
fn q200(db: &'static So) -> String {
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let tenth = top_n(drain(&pc), |&(u, n)| (Reverse(n), u), 10).last().unwrap().1;
    let cand: MatSet<Id<User>> = db.user.with((&pc).filt(|n| n >= tenth)).collect();
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let tb = (&cand)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(bounty.opt()).opt().and(badges_of(db).opt()))
        .fold([0i64; 2], |a, (b, _)| match b.flatten().flatten() {
            Some(x) => [a[0] + 1, a[1] + x],
            None => a,
        });
    let bc = (&cand).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let ur = ranked(drain((&tb).and(&pc).and(&bc)), |&(_, ((b, n), _))| (Reverse(n), b[0] == 0, Reverse(b[1])), false);
    let ur = rel(ur.into_iter().map(|((u, x), r)| (u, (x, r))).collect());
    let tu: HashIdx<Id<User>, ((([i64; 2], i64), i64), i64)> = (&ur).map(|(u, _)| u).inv().select((&ur).map(|(_, x)| x)).filt(|(_, r)| r <= 10).collect();
    let Post { post_type_id, score, creation_date, owner_user, .. } = &db.post;
    let rp = db.post.with(post_type_id.eq(1).and(score.gt(0)).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))));
    let v = drain(rp.select(owner_user.select(&tu)));
    rows(v.into_iter().map(|(p, (((b, n), c), r))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "created", "owner"]);
        f.extend([V::I(r), V::I(n), nullable(b[1], b[0]), V::I(c)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS UserRank
//     FROM Posts p WHERE p.PostTypeId = 1),
// ClosedPosts AS (SELECT ph.PostId, ph.PostHistoryTypeId, ph.CreationDate AS ClosedDate, cr.Name AS CloseReasonName
//     FROM PostHistory ph LEFT JOIN CloseReasonTypes cr ON CAST(ph.Comment AS INTEGER) = cr.Id WHERE ph.PostHistoryTypeId = 10),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpvoteCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownvoteCount, COUNT(DISTINCT b.Id) AS BadgeCount
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName)
// SELECT up.UserId, up.DisplayName, up.UpvoteCount, up.DownvoteCount, up.BadgeCount, rp.Title AS TopPostTitle, rp.CreationDate AS TopPostDate, cp.ClosedDate, cp.CloseReasonName
// FROM UserStats up LEFT JOIN RankedPosts rp ON up.UserId = rp.OwnerUserId AND rp.UserRank = 1 LEFT JOIN ClosedPosts cp ON rp.Id = cp.PostId
// WHERE up.UpvoteCount - up.DownvoteCount > 10 ORDER BY up.BadgeCount DESC, up.DisplayName ASC LIMIT 50;
//
// UserRank breaks a Score tie on the post id (the SQL leaves it open).
fn q3301(db: &'static So) -> String {
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt().and(badges_of(db).opt()))
        .fold([0i64; 2], |a, (t, _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let Post { post_type_id, owner_user, score, .. } = &db.post;
    let top = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 1, false);
    let tp = rel(top.into_iter().map(|(p, u)| (u, p)).collect());
    let first: HashIdx<Id<User>, Id<Post>> = (&tp).map(|(u, _)| u).inv().select((&tp).map(|(_, p)| p)).collect();
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post, post_history_type_id, comment, creation_date: hd, .. } = &db.post_history;
    let closed: HashIdx<Id<Post>, Id<PostHistory>> = db.post_history.with(post_history_type_id.eq(10)).select(post).inv().collect();
    let cr = comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason);
    let v = drain((&us).filt(|a| a[0] - a[1] > 10).and(&bc).and((&first).select(Ident::<Post>::new().and(closed.select(hd.and(cr.opt())).opt())).opt()));
    let v = top_n(v, |&(u, ((_, b), _))| (Reverse(b), db.user.display_name.get(u).unwrap(), u), 50);
    rows(v.into_iter().map(|(u, ((a, b), t))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(b)]);
        match t {
            Some((p, c)) => {
                f.extend(post_fields(db, p, &["title", "created"]));
                f.extend(match c {
                    Some((d, r)) => [V::T(d), ostr(r)],
                    None => [V::Null, V::Null],
                });
            }
            None => f.extend([V::Null, V::Null, V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.PostTypeId),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.CommentCount FROM RankedPosts rp WHERE rp.Rank <= 10),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes,
//        COUNT(DISTINCT b.Id) AS BadgeCount FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.CommentCount, us.UserId, us.DisplayName, us.Upvotes, us.Downvotes, us.BadgeCount
// FROM TopPosts tp JOIN Users u ON tp.PostId = u.Id JOIN UserStats us ON us.UserId = u.Id ORDER BY tp.Score DESC, tp.CommentCount DESC;
//
// Rank reads only base columns, so the top posts are picked first. `tp.PostId = u.Id` joins a post id to a user id, so it goes through the raw ids.
fn q6308(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, origid, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let tu: MatSet<Id<User>> = (&tp).select(origid.select(&uidx)).collect();
    let us = (&tu)
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt().and(badges_of(db).opt()))
        .fold([0i64; 2], |a, (t, _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let bc = (&tu).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = drain((&cc).and(origid.select(&uidx).select(Ident::<User>::new().and(&us).and(&bc))));
    rows(v.into_iter().map(|(p, (c, ((u, a), b)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.push(V::I(c));
        f.extend(ucols(db, u, &["uid", "name"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(b)]);
        row(f)
    }))
}

// WITH RecursivePostHistory AS (SELECT p.Id AS PostId, p.Title, ph.CreationDate, ph.PostHistoryTypeId, ph.Comment, ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY ph.CreationDate DESC) AS rn
//     FROM Posts p JOIN PostHistory ph ON p.Id = ph.PostId WHERE ph.PostHistoryTypeId IN (10, 11)),
// AggregatedVotes AS (SELECT PostId, COUNT(CASE WHEN VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN VoteTypeId = 3 THEN 1 END) AS DownVotes,
//        COUNT(CASE WHEN VoteTypeId = 1 THEN 1 END) AS AcceptedVotes FROM Votes GROUP BY PostId),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(u.Reputation) AS TotalReputation, RANK() OVER (ORDER BY SUM(u.Reputation) DESC) AS UserRank
//     FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName HAVING SUM(u.Reputation) > 1000)
// SELECT ph.PostId, p.Title, COALESCE(av.UpVotes, 0) AS UpVotes, COALESCE(av.DownVotes, 0) AS DownVotes, COALESCE(av.AcceptedVotes, 0) AS AcceptedVotes,
//        pu.DisplayName AS TopUser, pu.TotalReputation
// FROM Posts p LEFT JOIN RecursivePostHistory ph ON p.Id = ph.PostId AND ph.rn = 1 LEFT JOIN AggregatedVotes av ON p.Id = av.PostId
// LEFT JOIN TopUsers pu ON pu.UserRank <= 10 WHERE ph.PostHistoryTypeId IS NULL OR ph.PostHistoryTypeId IN (11, 10) ORDER BY p.CreationDate DESC;
//
// No CTE is recursive. The ON of the TopUsers join names only pu, so every post is crossed with the top ten; the WHERE always holds.
// `Votes.PostId` is read raw, since AggregatedVotes groups on it without joining Posts.
fn q32456(db: &'static So) -> String {
    let Post { owner_user, .. } = &db.post;
    let tr = db.post.group_by(owner_user).select(owner_user.select(&db.user.reputation)).fold(0i64, |s, r| s + r);
    let v = ranked(drain((&tr).filt(|s| s > 1000)), |&(_, s)| Reverse(s), false);
    let top = rel(v.into_iter().filter(|x| x.1 <= 10).map(|x| x.0).collect());
    let closed: MatSet<Id<Post>> = db.post_history.with((&db.post_history.post_history_type_id).in_v(vec![10, 11])).select(&db.post_history.post).collect();
    let Vote { post_id, vote_type_id, .. } = &db.vote;
    let av: HashIdx<i64, [i64; 3]> = db
        .vote
        .group_by(post_id)
        .select(vote_type_id)
        .fold([0i64; 3], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + (t == 1) as i64])
        .collect();
    let left = db.post.select(Ident::<Post>::new().with(&closed).opt().and((&db.post.origid).select(&av).opt()));
    let mut out = Vec::new();
    left.cross(&top).drive(|(p, _), ((c, a), (u, s))| out.push((p, c, a.unwrap_or([0; 3]), u, s)));
    rows(out.into_iter().map(|(p, c, a, u, s)| {
        let id = c.map_or(V::Null, |p| post_fields(db, p, &["id"]).pop().unwrap());
        row(vec![id, post_fields(db, p, &["title"]).pop().unwrap(), V::I(a[0]), V::I(a[1]), V::I(a[2]), user_col(db, u, "name"), V::I(s)])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn,
//        p.OwnerUserId FROM Posts p WHERE p.PostTypeId = 1 AND p.Score > 0),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverBadges,
//        COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges
//     FROM RankedPosts rp LEFT JOIN UserBadges ub ON rp.OwnerUserId = ub.UserId WHERE ub.GoldBadges IS NOT NULL OR ub.SilverBadges > 0 OR ub.BronzeBadges > 5)
// SELECT fp.Title, fp.CreationDate, fp.ViewCount, COALESCE(fp.Score, 0) AS Score,
//        CONCAT('Gold: ', COALESCE(fp.GoldBadges, 0), ', Silver: ', COALESCE(fp.SilverBadges, 0), ', Bronze: ', COALESCE(fp.BronzeBadges, 0)) AS BadgeCount
// FROM FilteredPosts fp WHERE fp.ViewCount > (SELECT AVG(ViewCount) FROM Posts) ORDER BY fp.ViewCount DESC LIMIT 10;
//
// GoldBadges is a COUNT, so it is NULL only where the LEFT JOIN found no user: the WHERE keeps exactly the posts with an owner.
fn q928(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, owner_user, .. } = &db.post;
    let (n, s) = db.post.select(view_count).fold_flat((0i64, 0i64), |(n, s), w| (n + 1, s + w));
    let mean = s as f64 / n as f64;
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| {
        [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]
    });
    let fp = db.post.with(post_type_id.eq(1).and(score.gt(0))).with(view_count.filt(move |w: i64| w as f64 > mean)).select(view_count.and(owner_user.select(&ub)));
    let v = top_n(drain(fp), |&(p, (w, _))| (Reverse(w), p), 10);
    rows(v.into_iter().map(|(p, (_, b))| {
        let mut f = post_fields(db, p, &["title", "created", "views", "score"]);
        f.push(V::Owned(format!("Gold: {}, Silver: {}, Bronze: {}", b[0], b[1], b[2])));
        row(f)
    }))
}

// WITH UserPostStatistics AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON v.UserId = u.Id GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalBounty, RANK() OVER (ORDER BY TotalPosts DESC) AS PostRank FROM UserPostStatistics),
// ClosedPostStatistics AS (SELECT ph.UserId, COUNT(ph.Id) AS TotalClosedPosts, MAX(ph.CreationDate) AS LastClosedDate FROM PostHistory ph
//     WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.UserId),
// FinalStatistics AS (SELECT tu.UserId, tu.DisplayName, tu.TotalPosts, tu.TotalQuestions, tu.TotalAnswers, tu.TotalBounty, COALESCE(cps.TotalClosedPosts, 0) AS TotalClosedPosts,
//        cps.LastClosedDate FROM TopUsers tu LEFT JOIN ClosedPostStatistics cps ON tu.UserId = cps.UserId)
// SELECT *, CASE WHEN TotalClosedPosts > 0 THEN 'Active Contributor' ELSE 'New Contributor' END AS ContributorStatus
// FROM FinalStatistics WHERE TotalPosts > 5 ORDER BY TotalPosts DESC, TotalBounty DESC;
//
// The WHERE reads only the distinct post count, so the posts x votes product is driven only for the users it keeps.
fn q3125(db: &'static So) -> String {
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let cand: MatSet<Id<User>> = db.user.with((&pc).filt(|n| n > 5)).collect();
    let s = (&cand)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(&db.post.post_type_id).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold([0i64; 3], |a, (t, b)| [a[0] + (t == Some(1)) as i64, a[1] + (t == Some(2)) as i64, a[2] + b.flatten().unwrap_or(0)]);
    let PostHistory { user, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let cps = db.post_history.with(post_history_type_id.in_v(vec![10, 11])).group_by(user).select(hd).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let v = drain((&s).and(&pc).and((&cps).opt()));
    rows(v.into_iter().map(|(u, ((a, n), c))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        let (k, d) = c.unwrap_or((0, i64::MIN));
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(k), tmax(d), V::S(if k > 0 { "Active Contributor" } else { "New Contributor" })]);
        row(f)
    }))
}

// Rewritten (rewrites/2697.sql): the RankScore window is tie-broken on p.Id.
// WITH RankedPosts AS (SELECT p.Id, p.Title, p.Score, p.CreationDate, u.DisplayName AS OwnerName,
//        ROW_NUMBER() OVER (PARTITION BY CAST(p.CreationDate AS DATE) ORDER BY p.Score DESC, p.Id) AS RankScore
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' AND p.PostTypeId = 1),
// TopScorers AS (SELECT rp.OwnerName, COUNT(rp.Id) AS QuestionsCount FROM RankedPosts rp WHERE rp.RankScore <= 5 GROUP BY rp.OwnerName),
// PostComments AS (SELECT p.Id AS PostId, COUNT(c.Id) AS CommentsCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY p.Id),
// PostDetails AS (SELECT rp.Id, rp.Title, rp.Score, pc.CommentsCount, ts.QuestionsCount FROM RankedPosts rp LEFT JOIN PostComments pc ON rp.Id = pc.PostId
//     LEFT JOIN TopScorers ts ON rp.OwnerName = ts.OwnerName)
// SELECT pd.Id, pd.Title, pd.Score, pd.CommentsCount, COALESCE(pd.QuestionsCount, 0) AS QuestionsCount,
//        CASE WHEN pd.Score >= 100 THEN 'High Score' WHEN pd.Score >= 50 THEN 'Medium Score' ELSE 'Low Score' END AS ScoreCategory
// FROM PostDetails pd WHERE pd.CommentsCount > 10 ORDER BY pd.Score DESC, pd.Title ASC;
fn q2697(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let rp = || db.post.with(post_type_id.eq(1).and(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))).with(owner_user);
    let top = top_per(drain(rp().select(creation_date.map(trunc_day))), |&(_, d)| d, |&(p, _)| (Reverse(score.get(p).unwrap()), db.post.origid.get(p).unwrap()), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let name = owner_user.select(&db.user.display_name);
    let ts = (&tp).group_by(&name).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let pc = rp().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&pc).filt(|n| n > 10).and(name.select(&ts).opt()));
    rows(v.into_iter().map(|(p, (c, q))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "score"]);
        f.extend([V::I(c), V::I(q.unwrap_or(0)), V::S(if s >= 100 { "High Score" } else if s >= 50 { "Medium Score" } else { "Low Score" })]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT Id, Reputation, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM Users),
// PostSummary AS (SELECT P.OwnerUserId, COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS Questions, COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS Answers,
//        COUNT(CASE WHEN P.PostTypeId IN (4, 5) THEN 1 END) AS TagWikis, SUM(V.BountyAmount) AS TotalBounty FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY P.OwnerUserId),
// ClosedPosts AS (SELECT Ph.UserId, COUNT(*) AS ClosedPostCount FROM PostHistory Ph WHERE Ph.PostHistoryTypeId = 10 GROUP BY Ph.UserId)
// SELECT U.DisplayName, COALESCE(UR.Reputation, 0) AS Reputation, COALESCE(PS.Questions, 0) AS TotalQuestions, COALESCE(PS.Answers, 0) AS TotalAnswers,
//        COALESCE(PS.TagWikis, 0) AS TotalTagWikis, COALESCE(CL.ClosedPostCount, 0) AS TotalClosedPosts,
//        CASE WHEN COALESCE(PS.TotalBounty, 0) > 0 THEN 'Has Bounty' ELSE 'No Bounty' END AS BountyStatus,
//        CASE WHEN COALESCE(UR.Reputation, 0) >= 1000 THEN 'Expert' WHEN COALESCE(UR.Reputation, 0) >= 500 THEN 'Intermediate' ELSE 'Novice' END AS UserLevel
// FROM Users U LEFT JOIN UserReputation UR ON U.Id = UR.Id LEFT JOIN PostSummary PS ON U.Id = PS.OwnerUserId LEFT JOIN ClosedPosts CL ON U.Id = CL.UserId
// WHERE U.Location IS NOT NULL ORDER BY Reputation DESC, TotalQuestions DESC LIMIT 100;
fn q1385(db: &'static So) -> String {
    let Post { owner_user, post_type_id, .. } = &db.post;
    let ps = db
        .post
        .group_by(owner_user)
        .select(post_type_id.and(votes_of(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold([0i64; 5], |a, (t, b)| {
            let b = b.flatten();
            [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + matches!(t, 4 | 5) as i64, a[3] + b.is_some() as i64, a[4] + b.unwrap_or(0)]
        });
    let PostHistory { user, post_history_type_id, .. } = &db.post_history;
    let cl = db.post_history.with(post_history_type_id.eq(10)).group_by(user).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let v = drain(db.user.with(&db.user.location).select((&db.user.reputation).and((&ps).opt()).and((&cl).opt())));
    let v = top_n(v, |&(u, ((r, a), _))| (Reverse(r), Reverse(a.map_or(0, |a| a[0])), u), 100);
    rows(v.into_iter().map(|(u, ((r, a), c))| {
        let a = a.unwrap_or([0; 5]);
        row(vec![
            user_col(db, u, "name"),
            V::I(r),
            V::I(a[0]),
            V::I(a[1]),
            V::I(a[2]),
            V::I(c.unwrap_or(0)),
            V::S(if a[3] > 0 && a[4] > 0 { "Has Bounty" } else { "No Bounty" }),
            V::S(if r >= 1000 { "Expert" } else if r >= 500 { "Intermediate" } else { "Novice" }),
        ])
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS Rank,
//        COALESCE(u.DisplayName, 'Deleted User') AS OwnerDisplayName, COALESCE(u.Reputation, 0) AS OwnerReputation
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT rp.*, CASE WHEN rp.Score >= 10 THEN 'High Score' WHEN rp.Score BETWEEN 5 AND 9 THEN 'Medium Score' ELSE 'Low Score' END AS ScoreCategory
//     FROM RankedPosts rp WHERE rp.Rank <= 3),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount, AVG(c.Score) AS AverageCommentScore FROM Comments c GROUP BY c.PostId)
// SELECT tp.Title, tp.OwnerDisplayName, tp.OwnerReputation, tp.CreationDate, tp.Score AS PostScore, tp.ScoreCategory, COALESCE(pc.CommentCount, 0) AS TotalComments,
//        COALESCE(pc.AverageCommentScore, 0) AS AvgCommentScore,
//        CASE WHEN EXISTS (SELECT 1 FROM Votes v WHERE v.PostId = tp.Id AND v.VoteTypeId = 2) THEN 'Has Upvotes' ELSE 'No Upvotes' END AS VoteStatus
// FROM TopPosts tp LEFT JOIN PostComments pc ON tp.Id = pc.PostId ORDER BY tp.CreationDate DESC LIMIT 10;
//
// The ownerless posts rank as one partition of their own. Rank breaks a Score tie on the post id (the SQL leaves it open).
fn q1014(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 3, false);
    let top = top_n(top, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 10);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.score).opt()).fold([0i64; 2], |a, s| match s {
        Some(s) => [a[0] + 1, a[1] + s],
        None => a,
    });
    let up = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2)));
    let has: MatSet<Id<Post>> = (&tp).with(up).collect();
    let v = drain((&pc).and(Ident::<Post>::new().with(&has).opt()));
    rows(v.into_iter().map(|(p, (a, h))| {
        let s = score.get(p).unwrap();
        let u = owner_user.get(p);
        row(vec![
            post_fields(db, p, &["title"]).pop().unwrap(),
            V::S(u.map_or("Deleted User", |u| db.user.display_name.get(u).unwrap())),
            V::I(u.map_or(0, |u| db.user.reputation.get(u).unwrap())),
            V::T(creation_date.get(p).unwrap()),
            V::I(s),
            V::S(if s >= 10 { "High Score" } else if (5..=9).contains(&s) { "Medium Score" } else { "Low Score" }),
            V::I(a[0]),
            if a[0] == 0 { V::F(0.0) } else { avg(a[1], a[0]) },
            V::S(if h.is_some() { "Has Upvotes" } else { "No Upvotes" }),
        ])
    }))
}

// WITH RecentPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.ViewCount, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// UserScores AS (SELECT u.Id AS UserId, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(COALESCE(p.Score, 0)) AS TotalScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.Reputation),
// HighReputationUsers AS (SELECT us.UserId, us.Reputation, us.PostCount, us.TotalScore FROM UserScores us WHERE us.Reputation > (SELECT AVG(Reputation) FROM Users)),
// TopViewCountPosts AS (SELECT rp.Title, rp.ViewCount, rp.OwnerUserId, u.DisplayName, ROW_NUMBER() OVER (ORDER BY rp.ViewCount DESC) AS rn
//     FROM RecentPosts rp JOIN Users u ON rp.OwnerUserId = u.Id WHERE rp.ViewCount > 100)
// SELECT COUNT(DISTINCT h.UserId) AS UniqueHighRepUsers, SUM(h.Reputation) AS TotalReputation, AVG(t.ViewCount) AS AverageViewCount
// FROM HighReputationUsers h LEFT JOIN TopViewCountPosts t ON h.UserId = t.OwnerUserId WHERE t.rn <= 5 GROUP BY t.OwnerUserId HAVING COUNT(t.Title) > 0
// ORDER BY TotalReputation DESC;
//
// The WHERE on t.rn makes the LEFT JOIN inner. UserScores' aggregates are never read.
fn q1380(db: &'static So) -> String {
    let (n, s) = (&db.user.reputation).fold_flat((0i64, 0i64), |(n, s), r| (n + 1, s + r));
    let high: MatSet<Id<User>> = db.user.with((&db.user.reputation).filt(move |r: i64| r as i128 * n as i128 > s as i128)).collect();
    let Post { creation_date, owner_user, view_count, title, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).with(owner_user).select(view_count.gt(100)));
    let top = top_n(v, |&(p, w)| (Reverse(w), p), 5);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let hu = || owner_user.select(&high);
    let tv = (&tp)
        .group_by(hu())
        .select(view_count.and(title.opt()).and(hu().select(&db.user.reputation)))
        .fold([0i64; 4], |a, ((w, t), r)| [a[0] + 1, a[1] + w, a[2] + t.is_some() as i64, a[3] + r]);
    let du = (&tp).group_by(hu()).select(hu()).count_distinct();
    let v = drain((&tv).filt(|a| a[2] > 0).and(&du));
    rows(v.into_iter().map(|(_, (a, d))| row(vec![V::I(d), V::I(a[3]), avg(a[1], a[0])])))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
//        COUNT(DISTINCT V.UserId) AS VoteCount FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId = 2
//     WHERE P.PostTypeId = 1 GROUP BY P.Id, P.Title, P.CreationDate, P.Score, P.ViewCount),
// ActiveUsers AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS PostsCount, SUM(U.UpVotes) AS TotalUpVotes, SUM(U.DownVotes) AS TotalDownVotes
//     FROM Users U JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName),
// TopPosts AS (SELECT RP.PostId, RP.Title, RP.CreationDate, RP.Score, RP.ViewCount, RP.CommentCount, RP.VoteCount, ROW_NUMBER() OVER (ORDER BY RP.Score DESC, RP.ViewCount DESC) AS Rank
//     FROM RankedPosts RP)
// SELECT TP.PostId, TP.Title, TP.CreationDate, TP.Score, TP.ViewCount, TP.CommentCount, TP.VoteCount, AU.DisplayName, AU.PostsCount, AU.TotalUpVotes, AU.TotalDownVotes
// FROM TopPosts TP JOIN ActiveUsers AU ON TP.PostId IN (SELECT DISTINCT P.Id FROM Posts P WHERE P.OwnerUserId = AU.UserId) WHERE TP.Rank <= 10
// ORDER BY TP.Score DESC, TP.ViewCount DESC;
//
// Rank reads only base columns, so the top ten questions are picked first and the comment x upvote product is driven for those alone.
// The IN subquery makes AU the post's owner.
fn q6044(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, owner_user, .. } = &db.post;
    let top = top_n(drain(db.post.with(post_type_id.eq(1)).select(score)), |&(p, s)| {
        let w = view_count.get(p);
        (Reverse(s), w.is_none(), Reverse(w), p)
    }, 10);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let up = || votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2)));
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(up().opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(up().select(&db.vote.user).opt()).buf_fold(distinct_some);
    let User { up_votes, down_votes, .. } = &db.user;
    let au = db.post.with(owner_user).group_by(owner_user).select(owner_user.select(up_votes.and(down_votes))).fold([0i64; 3], |a, (u, d)| [a[0] + 1, a[1] + u, a[2] + d]);
    let v = drain((&cc).and(&vc).and(owner_user.select(Ident::<User>::new().and(&au))));
    rows(v.into_iter().map(|(p, ((c, n), (u, a)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(c), V::I(n), user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.OwnerUserId, p.Title, p.CreationDate, p.Score, p.ViewCount, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS timestamp) - INTERVAL '1 year'),
// UserStatistics AS (SELECT u.Id AS UserId, u.Reputation, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts,
//        SUM(CASE WHEN p.Score < 0 THEN 1 ELSE 0 END) AS NegativePosts, AVG(p.ViewCount) AS AvgViews FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.Reputation),
// CloseReasons AS (SELECT ph.PostId, MAX(CASE WHEN ph.PostHistoryTypeId = 10 THEN cr.Name END) AS CloseReason
//     FROM PostHistory ph LEFT JOIN CloseReasonTypes cr ON ph.Comment = CAST(cr.Id AS VARCHAR) GROUP BY ph.PostId)
// SELECT u.DisplayName, us.TotalPosts, us.PositivePosts, us.NegativePosts, us.AvgViews, rp.Title, rp.Score, rp.ViewCount, cr.CloseReason
// FROM Users u JOIN UserStatistics us ON u.Id = us.UserId LEFT JOIN RankedPosts rp ON u.Id = rp.OwnerUserId AND rp.PostRank = 1 LEFT JOIN CloseReasons cr ON rp.PostId = cr.PostId
// WHERE u.Reputation > 1000 AND us.AvgViews > 50 ORDER BY us.Reputation DESC, us.TotalPosts DESC;
//
// RANK keeps every post tied at the newest CreationDate.
fn q1793(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, view_count, .. } = &db.post;
    let us = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(score.and(view_count.opt())).opt()).fold([0i64; 5], |a, p| match p {
        Some((s, w)) => [a[0] + 1, a[1] + (s > 0) as i64, a[2] + (s < 0) as i64, a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0)],
        None => a,
    });
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| Reverse(creation_date.get(p).unwrap()), 1, true);
    let tp = rel(top.into_iter().map(|(p, u)| (u, p)).collect());
    let first: HashIdx<Id<User>, Id<Post>> = (&tp).map(|(u, _)| u).inv().select((&tp).map(|(_, p)| p)).collect();
    let reason: HashIdx<Str, Str> = (&db.close_reason_type.origid).map(|i: i64| &*Box::leak(i.to_string().into_boxed_str())).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let cr = db
        .post_history
        .group_by(post)
        .select(post_history_type_id.and(comment.select(&reason).opt()))
        .fold(None::<Str>, |m, (t, r)| if t == 10 { m.max(r) } else { m });
    let hi = db.user.with((&db.user.reputation).gt(1000)).select((&us).filt(|a| a[3] > 0 && a[4] as i128 > 50 * a[3] as i128));
    let v = drain(hi.and((&first).select(Ident::<Post>::new().and((&cr).opt())).opt()));
    rows(v.into_iter().map(|(u, (a, p))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[4], a[3])];
        match p {
            Some((p, c)) => {
                f.extend(post_fields(db, p, &["title", "score", "views"]));
                f.push(ostr(c.flatten()));
            }
            None => f.extend([V::Null, V::Null, V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.Score IS NOT NULL),
// UserInsights AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounties
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (8, 9) GROUP BY u.Id, u.DisplayName, u.Reputation),
// ClosedPosts AS (SELECT ph.PostId, COUNT(*) AS CloseCount, MIN(ph.CreationDate) AS FirstClosedDate FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, ui.DisplayName AS UserDisplayName, ui.Reputation AS UserReputation, ui.PostCount AS UserPostCount,
//        ui.TotalBounties AS UserTotalBounties, cp.CloseCount AS TotalCloseVotes, cp.FirstClosedDate
// FROM RankedPosts rp LEFT JOIN UserInsights ui ON rp.PostId = ui.UserId LEFT JOIN ClosedPosts cp ON rp.PostId = cp.PostId
// WHERE (cp.CloseCount > 0 OR cp.CloseCount IS NULL) ORDER BY COALESCE(rp.Score, 0) DESC, rp.ViewCount DESC LIMIT 50;
//
// The ORDER BY reads only base columns, so the fifty questions are picked first. `rp.PostId = ui.UserId` joins a post id to a user id,
// so it goes through the raw ids. A ClosedPosts row always has CloseCount > 0, so the WHERE keeps every row.
fn q2612(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, origid, .. } = &db.post;
    let top = top_n(drain(db.post.with(post_type_id.eq(1)).select(score)), |&(p, s)| {
        let w = view_count.get(p);
        (Reverse(s), w.is_none(), Reverse(w), p)
    }, 50);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let tu: MatSet<Id<User>> = (&tp).select(origid.select(&uidx)).collect();
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).in_v(vec![8, 9]))).select((&db.vote.bounty_amount).opt());
    let ui = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(bounty.opt()).opt()).fold(0i64, |s, b| s + b.flatten().flatten().unwrap_or(0));
    let pc = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let cp = db.post_history.with(post_history_type_id.eq(10)).group_by(post).select(hd).fold((0i64, i64::MAX), |(n, m), d| (n + 1, m.min(d)));
    let v = drain((&tp).select(origid.select(&uidx).select(Ident::<User>::new().and(&ui).and(&pc)).opt().and((&cp).opt())));
    rows(v.into_iter().map(|(p, (u, c))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend(match u {
            Some(((u, b), n)) => {
                let mut g = ucols(db, u, &["name", "rep"]);
                g.extend([V::I(n), V::I(b)]);
                g
            }
            None => vec![V::Null, V::Null, V::Null, V::Null],
        });
        f.extend(match c {
            Some((n, d)) => [V::I(n), V::T(d)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COALESCE(u.DisplayName, 'Anonymous') AS Owner,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// PostStatistics AS (SELECT rp.PostId, rp.Title, rp.Owner, rp.Score, rp.ViewCount, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, SUM(CASE WHEN v.VoteTypeId = 10 THEN 1 ELSE 0 END) AS Deletions
//     FROM RankedPosts rp LEFT JOIN Comments c ON rp.PostId = c.PostId LEFT JOIN Votes v ON rp.PostId = v.PostId GROUP BY rp.PostId, rp.Title, rp.Owner, rp.Score, rp.ViewCount),
// TopPosts AS (SELECT p.*, CASE WHEN p.Rank <= 5 THEN 'Top 5' ELSE 'Other' END AS PostCategory FROM RankedPosts p)
// SELECT ps.*, tp.PostCategory, CASE WHEN ps.Deletions > 0 THEN 'Deleted' ELSE 'Active' END AS Status
// FROM PostStatistics ps JOIN TopPosts tp ON ps.PostId = tp.PostId WHERE (ps.CommentCount > 0 OR ps.UpVotes > 0) ORDER BY ps.Score DESC, ps.ViewCount DESC;
//
// Rank breaks a Score tie on the post id (the SQL leaves it open).
fn q888(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, .. } = &db.post;
    let rp = || db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)));
    let top = top_per(drain(rp().select(post_type_id)), |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false);
    let t5: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ps = rp()
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 4], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64, a[3] + (t == Some(10)) as i64]);
    let v = drain((&ps).filt(|a| a[0] > 0 || a[1] > 0).and(Ident::<Post>::new().with(&t5).opt()));
    rows(v.into_iter().map(|(p, (a, t))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.push(V::S(db.post.owner_user.get(p).map_or("Anonymous", |u| db.user.display_name.get(u).unwrap())));
        f.extend(post_fields(db, p, &["score", "views"]));
        f.extend(a.map(V::I));
        f.extend([V::S(if t.is_some() { "Top 5" } else { "Other" }), V::S(if a[3] > 0 { "Deleted" } else { "Active" })]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(COALESCE(P.ViewCount, 0)) AS TotalViews,
//        SUM(COALESCE(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END, 0)) AS TotalUpVotes, SUM(COALESCE(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END, 0)) AS TotalDownVotes,
//        ROW_NUMBER() OVER (PARTITION BY U.Id ORDER BY COUNT(DISTINCT P.Id) DESC) AS Rank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId WHERE U.Reputation > 1000 GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, TotalViews, TotalUpVotes, TotalDownVotes, DENSE_RANK() OVER (ORDER BY PostCount DESC) AS PostRank FROM UserActivity)
// SELECT TU.DisplayName, TU.Reputation, TU.PostCount, TU.TotalViews, TU.TotalUpVotes, TU.TotalDownVotes,
//        CASE WHEN TU.TotalDownVotes = 0 THEN 'No Negative Feedback' ELSE CONCAT('Negative Feedback: ', TU.TotalDownVotes) END AS FeedbackStatus,
//        (SELECT COUNT(*) FROM Badges B WHERE B.UserId = TU.UserId AND B.Class = 1) AS GoldBadges, (SELECT COUNT(*) FROM Badges B WHERE B.UserId = TU.UserId AND B.Class = 2) AS SilverBadges,
//        (SELECT COUNT(*) FROM Badges B WHERE B.UserId = TU.UserId AND B.Class = 3) AS BronzeBadges
// FROM TopUsers TU WHERE TU.PostRank <= 10 ORDER BY TU.PostCount DESC;
//
// PostRank reads only the distinct post count, so the ranked users are picked first and the posts x votes product is driven for those alone.
fn q3278(db: &'static So) -> String {
    let hi = || db.user.with((&db.user.reputation).gt(1000));
    let pc = hi().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let r = ranked(drain(&pc), |&(_, n)| Reverse(n), true);
    let tu: MatSet<Id<User>> = rel(r.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let s = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.view_count).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 3], |a, p| match p {
            Some((w, t)) => [a[0] + w.unwrap_or(0), a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64],
            None => a,
        });
    let bc = (&tu).group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| {
        [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]
    });
    let v = drain((&s).and(&pc).and(&bc));
    rows(v.into_iter().map(|(u, ((a, n), b))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        f.push(if a[2] == 0 { V::S("No Negative Feedback") } else { V::Owned(format!("Negative Feedback: {}", a[2])) });
        f.extend(b.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, u.DisplayName AS OwnerDisplayName, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.PostTypeId IN (1, 2)),
// PostMetrics AS (SELECT rp.PostId, rp.Title, rp.OwnerDisplayName, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, COALESCE(AVG(v.BountyAmount), 0) AS AverageBounty,
//        COUNT(c.Id) AS CommentCount FROM RankedPosts rp LEFT JOIN Votes v ON rp.PostId = v.PostId AND v.VoteTypeId = 8 LEFT JOIN Comments c ON rp.PostId = c.PostId
//     WHERE rp.Rank <= 10 GROUP BY rp.PostId, rp.Title, rp.OwnerDisplayName, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount)
// SELECT pm.PostId, pm.Title, pm.OwnerDisplayName, pm.CreationDate, pm.Score, pm.ViewCount, pm.AnswerCount, pm.AverageBounty, pm.CommentCount,
//        CASE WHEN pm.Score >= 100 THEN 'High Score' WHEN pm.Score BETWEEN 50 AND 99 THEN 'Medium Score' ELSE 'Low Score' END AS ScoreCategory
// FROM PostMetrics pm ORDER BY pm.Score DESC, pm.CreationDate DESC;
fn q5309(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.is_in([1, 2]).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let pm = (&tp).group_by(Ident::<Post>::new()).select(bounty.opt().and(comments_of(db).opt())).fold([0i64; 3], |a, (b, c)| {
        let b = b.flatten();
        [a[0] + b.is_some() as i64, a[1] + b.unwrap_or(0), a[2] + c.is_some() as i64]
    });
    rows(drain(&pm).into_iter().map(|(p, a)| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "score", "views", "answers"]);
        f.extend([if a[0] == 0 { V::F(0.0) } else { avg(a[1], a[0]) }, V::I(a[2])]);
        f.push(V::S(if s >= 100 { "High Score" } else if (50..=99).contains(&s) { "Medium Score" } else { "Low Score" }));
        row(f)
    }))
}

// WITH RankedUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(b.Id) AS BadgeCount, COALESCE(SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges,
//        COALESCE(SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadges, COALESCE(SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadges,
//        RANK() OVER (ORDER BY u.Reputation DESC) AS UserRank FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// PostStats AS (SELECT p.OwnerUserId, COUNT(DISTINCT p.Id) AS TotalPosts, COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS Questions,
//        COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS Answers, SUM(p.ViewCount) AS TotalViews, COUNT(DISTINCT c.Id) AS CommentCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY p.OwnerUserId)
// SELECT ru.UserId, ru.DisplayName, ru.Reputation, ru.BadgeCount, ru.GoldBadges, ru.SilverBadges, ru.BronzeBadges, ps.TotalPosts, ps.Questions, ps.Answers, ps.TotalViews, ps.CommentCount,
//        CASE WHEN ru.BadgeCount > 10 THEN 'Highly Acclaimed' WHEN ru.Reputation > 1000 THEN 'Expert Contributor' ELSE 'Regular Contributor' END AS ContributorLevel
// FROM RankedUsers ru LEFT JOIN PostStats ps ON ru.UserId = ps.OwnerUserId WHERE ru.UserRank <= 100 ORDER BY ru.Reputation DESC, ru.BadgeCount DESC;
//
// UserRank reads only Reputation, so the ranked users are picked first. TotalViews sums over posts x comments, as the SQL does.
fn q26754(db: &'static So) -> String {
    let r = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let tu: MatSet<Id<User>> = rel(r.into_iter().take_while(|x| x.1 <= 100).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let ub = (&tu).group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let Post { post_type_id, view_count, .. } = &db.post;
    let tv = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(view_count.opt().and(comments_of(db).opt())))
        .fold([0i64; 2], |a, (w, _)| [a[0] + w.is_some() as i64, a[1] + w.unwrap_or(0)]);
    let dp = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id)).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64]);
    let dc = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db).opt())).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&ub).and((&dp).and(&tv).and(&dc).opt()));
    rows(v.into_iter().map(|(u, (b, s))| {
        let rep = db.user.reputation.get(u).unwrap();
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(b.map(V::I));
        f.extend(match s {
            Some(((d, t), c)) => [V::I(d[0]), V::I(d[1]), V::I(d[2]), nullable(t[1], t[0]), V::I(c)],
            None => [V::Null, V::Null, V::Null, V::Null, V::Null],
        });
        f.push(V::S(if b[0] > 10 { "Highly Acclaimed" } else if rep > 1000 { "Expert Contributor" } else { "Regular Contributor" }));
        row(f)
    }))
}

// WITH RankedUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, RANK() OVER (ORDER BY u.Reputation DESC) AS ReputationRank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, AnswerCount, QuestionCount FROM RankedUsers WHERE ReputationRank <= 10),
// RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS OwnerName, COUNT(c.Id) AS CommentCount, AVG(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS AvgUpvotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId
//     LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.CreationDate, u.DisplayName)
// SELECT tu.DisplayName AS TopUser, tu.Reputation, rp.Title AS RecentPostTitle, rp.CreationDate AS PostDate, rp.OwnerName, rp.CommentCount, rp.AvgUpvotes, rp.DownvoteCount
// FROM TopUsers tu JOIN RecentPosts rp ON tu.DisplayName = rp.OwnerName ORDER BY tu.Reputation DESC, rp.CreationDate DESC;
fn q8474(db: &'static So) -> String {
    let r = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let tu: MatSet<Id<User>> = rel(r.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let by_name: HashIdx<Str, Id<User>> = (&tu).select(&db.user.display_name).inv().collect();
    let Post { creation_date, owner_user, .. } = &db.post;
    let rp = db
        .post
        .with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 4], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + 1, a[2] + (t == Some(2)) as i64, a[3] + (t == Some(3)) as i64]);
    let v = drain((&rp).and(owner_user.select(&db.user.display_name).select(&by_name)));
    rows(v.into_iter().map(|(p, (a, u))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["title", "created", "owner"]));
        f.extend([V::I(a[0]), avg(a[2], a[1]), V::I(a[3])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, RANK() OVER (ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'
//     GROUP BY p.Id, p.Title, p.Score, p.ViewCount),
// TopPosts AS (SELECT PostId, Title, Score, ViewCount, CommentCount, UpVotes, DownVotes FROM RankedPosts WHERE Rank <= 10),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, SUM(p.Score) AS TotalScore, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId
//     LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName)
// SELECT tp.Title, tp.Score, tp.ViewCount, tp.CommentCount, tp.UpVotes, tp.DownVotes, us.DisplayName AS TopUser, us.TotalScore, us.BadgeCount
// FROM TopPosts tp JOIN UserStats us ON tp.PostId IN (SELECT p.Id FROM Posts p WHERE p.OwnerUserId = us.UserId) ORDER BY tp.Score DESC, tp.ViewCount DESC;
//
// Rank reads only base columns, so the top posts are picked first and each product is driven for those (and their owners) alone.
// The IN subquery makes us the post's owner.
fn q5555(db: &'static So) -> String {
    let Post { creation_date, score, view_count, owner_user, .. } = &db.post;
    let key = |p: Id<Post>| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    };
    let r = ranked(drain(db.post.with(creation_date.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))), |&(p, _)| key(p), false);
    let tp: MatSet<Id<Post>> = rel(r.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let ps = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let owners: MatSet<Id<User>> = (&tp).select(owner_user).collect();
    let us = (&owners)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score).opt().and(badges_of(db).opt()))
        .fold([0i64; 3], |a, (s, b)| [a[0] + s.is_some() as i64, a[1] + s.unwrap_or(0), a[2] + b.is_some() as i64]);
    let v = drain((&ps).and(owner_user.select(Ident::<User>::new().and(&us))));
    rows(v.into_iter().map(|(p, (a, (u, s)))| {
        let mut f = post_fields(db, p, &["title", "score", "views"]);
        f.extend(a.map(V::I));
        f.extend([user_col(db, u, "name"), nullable(s[1], s[0]), V::I(s[2])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, u.DisplayName AS OwnerDisplayName, p.Score, p.ViewCount,
//        ROW_NUMBER() OVER (PARTITION BY EXTRACT(YEAR FROM p.CreationDate) ORDER BY p.CreationDate DESC) AS YearRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1),
// TopPosts AS (SELECT Id, Title, CreationDate, OwnerDisplayName, Score, ViewCount FROM RankedPosts WHERE YearRank <= 5),
// PostVoteSummary AS (SELECT p.Id AS PostId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes,
//        COUNT(CASE WHEN v.VoteTypeId = 10 THEN 1 END) AS Deletions FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id)
// SELECT tp.Title, tp.CreationDate, tp.OwnerDisplayName, tp.Score, tp.ViewCount, COALESCE(vs.UpVotes, 0) AS UpVotes, COALESCE(vs.DownVotes, 0) AS DownVotes,
//        COALESCE(vs.Deletions, 0) AS Deletions, CASE WHEN COALESCE(vs.Deletions, 0) > 0 THEN 'Deleted' ELSE 'Active' END AS PostStatus,
//        CASE WHEN tp.ViewCount > 1000 THEN 'Hot' ELSE 'Normal' END AS Popularity
// FROM TopPosts tp LEFT JOIN PostVoteSummary vs ON tp.Id = vs.PostId ORDER BY tp.CreationDate DESC;
fn q34765(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, view_count, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1)).with(owner_user).select(creation_date.map(year)));
    let top = top_per(v, |&(_, y)| y, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let vs = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + (t == Some(10)) as i64]
    });
    rows(drain(&vs).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["title", "created", "owner", "score", "views"]);
        f.extend(a.map(V::I));
        f.extend([V::S(if a[2] > 0 { "Deleted" } else { "Active" }), V::S(if view_count.get(p).map_or(false, |w| w > 1000) { "Hot" } else { "Normal" })]);
        row(f)
    }))
}

// WITH PostVoteSummary AS (SELECT p.Id AS PostId, p.PostTypeId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotesCount, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotesCount,
//        COUNT(v.Id) AS TotalVotesCount, MAX(v.CreationDate) AS LastVoteDate FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '6 months' GROUP BY p.Id, p.PostTypeId),
// ClosedPostHistory AS (SELECT ph.PostId, ph.CreationDate, ph.UserDisplayName, COALESCE(NULLIF(pr.Name, ''), 'Unknown Reason') AS CloseReason
//     FROM PostHistory ph JOIN CloseReasonTypes pr ON CAST(ph.Comment AS INT) = pr.Id WHERE ph.PostHistoryTypeId = 10),
// TopPosts AS (SELECT p.Id, p.Title, p.ViewCount, p.CreationDate, ps.UpVotesCount - ps.DownVotesCount AS NetVotes,
//        ROW_NUMBER() OVER (ORDER BY p.CreationDate DESC, ps.UpVotesCount - ps.DownVotesCount DESC) AS Rank FROM Posts p JOIN PostVoteSummary ps ON p.Id = ps.PostId WHERE ps.TotalVotesCount > 5)
// SELECT tp.Title, tp.ViewCount, tp.CreationDate, tp.NetVotes, ph.CreationDate AS CloseDate, ph.UserDisplayName AS ClosedBy, ph.CloseReason
// FROM TopPosts tp LEFT JOIN ClosedPostHistory ph ON tp.Id = ph.PostId WHERE tp.Rank <= 10 ORDER BY tp.NetVotes DESC, tp.CreationDate DESC;
fn q24052(db: &'static So) -> String {
    let Post { creation_date, .. } = &db.post;
    let ps = db
        .post
        .with(creation_date.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -6)))
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt())
        .fold([0i64; 3], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + t.is_some() as i64]);
    let top = top_n(drain((&ps).filt(|a| a[2] > 5)), |&(p, a)| (Reverse(creation_date.get(p).unwrap()), Reverse(a[0] - a[1]), p), 10);
    let tp = rel(top);
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post, post_history_type_id, comment, creation_date: hd, user_display_name, .. } = &db.post_history;
    let cr = || comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason);
    let closes: HashIdx<Id<Post>, Id<PostHistory>> = db.post_history.with(post_history_type_id.eq(10)).with(cr()).select(post).inv().collect();
    type R = (Id<Post>, [i64; 3]);
    let v = drain((&tp).select(Same::<R>::new().and(Same::<R>::new().map(|(p, _): R| p).select(closes.select(Ident::<PostHistory>::new().and(cr())).opt()))));
    rows(v.into_iter().map(|(_, ((p, a), h))| {
        let mut f = post_fields(db, p, &["title", "views", "created"]);
        f.push(V::I(a[0] - a[1]));
        f.extend(match h {
            Some((h, r)) => [V::T(hd.get(h).unwrap()), ostr(user_display_name.get(h)), V::S(if r.is_empty() { "Unknown Reason" } else { r })],
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COALESCE(COUNT(c.Id), 0) AS CommentCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount,
//        ROW_NUMBER() OVER (PARTITION BY pt.Name ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= timestamp '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, pt.Name),
// RankedUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(p.Score) AS TotalScore, RANK() OVER (ORDER BY SUM(p.Score) DESC) AS UserRank
//     FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId WHERE p.CreationDate >= timestamp '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY u.Id, u.DisplayName)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.CommentCount, rp.UpVoteCount, rp.DownVoteCount, ru.DisplayName AS TopUser,
//        ru.TotalScore AS UserTotalScore, ru.UserRank
// FROM RankedPosts rp JOIN RankedUsers ru ON rp.Rank = 1 WHERE rp.Rank <= 5 ORDER BY rp.Score DESC;
//
// The ON names only rp, so each type's top post is crossed with every ranked user. Rank reads only base columns, so the top posts are picked
// first and the comment x vote product is driven for those alone.
fn q6177(db: &'static So) -> String {
    let Post { post_type, creation_date, score, owner_user, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let top = top_per(drain(recent().select(post_type.select(&db.post_type.name))), |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let rp = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let ts_ = recent().with(owner_user).group_by(owner_user).select(score).fold(0i64, |s, x| s + x);
    let ru = rel(ranked(drain(&ts_), |&(_, s)| Reverse(s), false));
    let mut out = Vec::new();
    (&rp).cross(&ru).drive(|(p, _), (a, ((u, s), r))| out.push((p, a, u, s, r)));
    rows(out.into_iter().map(|(p, a, u, s, r)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend(a.map(V::I));
        f.extend([user_col(db, u, "name"), V::I(s), V::I(r)]);
        row(f)
    }))
}

// WITH RECURSIVE UserBadges AS (SELECT U.Id AS UserId, U.DisplayName, B.Class, COUNT(B.Id) AS BadgeCount FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId
//     WHERE U.Reputation > 1000 GROUP BY U.Id, U.DisplayName, B.Class),
// RankedUsers AS (SELECT UserId, DisplayName, BadgeCount, RANK() OVER (PARTITION BY Class ORDER BY BadgeCount DESC) AS BadgeRank FROM UserBadges),
// PostStatistics AS (SELECT P.Id AS PostId, P.Title, COUNT(V.Id) AS TotalVotes, AVG(V.BountyAmount) AS AverageBountyAmount, SUM(COALESCE(V.BountyAmount, 0)) as TotalBounty,
//        MAX(P.CreationDate) as LatestPostDate FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
//     GROUP BY P.Id, P.Title),
// CombinedStats AS (SELECT RU.DisplayName, RU.BadgeCount, PS.PostId, PS.Title, PS.TotalVotes, PS.TotalBounty, PS.LatestPostDate FROM RankedUsers RU JOIN PostStatistics PS ON RU.BadgeRank = 1)
// SELECT CS.DisplayName, CS.BadgeCount, CS.PostId, CS.Title, CS.TotalVotes, CS.TotalBounty,
//        CASE WHEN CS.LatestPostDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 month' THEN 'Active' ELSE 'Inactive' END AS PostActivityStatus
// FROM CombinedStats CS ORDER BY CS.BadgeCount DESC, CS.TotalVotes DESC LIMIT 10;
//
// WITH RECURSIVE, but no CTE refers to itself. The ON names only RU, so the rank-1 (user, class) groups are crossed with every recent post;
// `lex_top` takes the ORDER BY ... LIMIT over that product without building it.
fn q30657(db: &'static So) -> String {
    let hi = db.user.with((&db.user.reputation).gt(1000));
    let ub = rel(drain(hi.select(badges_of(db).select(Ident::<Badge>::new().and(&db.badge.class)).opt())));
    type R = (Id<User>, Option<(Id<Badge>, i64)>);
    let g = (&ub)
        .group_by(Same::<R>::new().map(|(u, b): R| (u, b.map(|x| x.1))))
        .select(Same::<R>::new())
        .fold(0i64, |n, (_, b)| n + b.is_some() as i64);
    let r1 = top_per(drain(&g), |&((_, c), _)| c, |&(_, n)| Reverse(n), 1, true);
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let ps = db
        .post
        .with((&db.post.creation_date).ge(add_years(t0, -1)))
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select((&db.vote.bounty_amount).opt()).opt())
        .fold([0i64; 2], |a, v| [a[0] + v.is_some() as i64, a[1] + v.flatten().unwrap_or(0)]);
    let v = cross_top(r1, |&((u, _), n)| (Reverse(n), u), drain(&ps), |&(p, a)| (Reverse(a[0]), p), 10);
    rows(v.into_iter().map(|(((u, _), n), (p, a))| {
        let mut f = vec![user_col(db, u, "name"), V::I(n)];
        f.extend(post_fields(db, p, &["id", "title"]));
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.push(V::S(if db.post.creation_date.get(p).unwrap() >= add_months(t0, -1) { "Active" } else { "Inactive" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.Score, p.ViewCount, p.AnswerCount, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS Rank,
//        COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount, p.OwnerUserId FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(v.BountyAmount) AS TotalBounty
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 8 GROUP BY u.Id, u.DisplayName)
// SELECT us.UserId, us.DisplayName, us.GoldBadges, us.SilverBadges, us.BronzeBadges, us.TotalPosts, us.TotalBounty, rp.Title, rp.Score, rp.ViewCount, rp.AnswerCount, rp.CreationDate,
//        rp.CommentCount
// FROM UserStats us JOIN RankedPosts rp ON us.UserId = rp.OwnerUserId WHERE us.TotalPosts > 10 ORDER BY rp.Score DESC, us.UserId ASC LIMIT 50 OFFSET 0;
//
// RankedPosts has one row per post x comment (the window counts them). The ORDER BY and WHERE read only base columns and the distinct post
// count, so the fifty rows are picked first and the badges x posts x bounty product is driven for their users alone.
fn q1078(db: &'static So) -> String {
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let Post { post_type_id, creation_date, owner_user, score, .. } = &db.post;
    let many = Ident::<User>::new().with((&pc).filt(|n| n > 10));
    let rp = db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(owner_user.select(many).and(comments_of(db).opt()));
    let top = top_n(drain(rp), |&(p, (u, c))| (Reverse(score.get(p).unwrap()), db.user.origid.get(u).unwrap(), p, c), 50);
    let tu: MatSet<Id<User>> = rel(top.iter().map(|x| x.1 .0).collect()).map(|u| u).collect();
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let us = (&tu)
        .group_by(Ident::<User>::new())
        .select(badges_of(db).select(&db.badge.class).opt().and(posts_of(db).select(bounty.opt()).opt()))
        .fold([0i64; 5], |a, (c, b)| {
            let b = b.flatten().flatten();
            [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64, a[3] + b.is_some() as i64, a[4] + b.unwrap_or(0)]
        });
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let tv = rel(top);
    type R = (Id<Post>, (Id<User>, Option<Id<Comment>>));
    let v = drain((&tv).select(Same::<R>::new().and(Same::<R>::new().map(|(_, (u, _)): R| u).select(&us)).and(Same::<R>::new().map(|(p, _): R| p).select(&cc))));
    rows(v.into_iter().map(|(_, (((p, (u, _)), a), c))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        f.extend(match pc.get(u) {
            Some(n) => [V::I(n)],
            None => [V::Null],
        });
        f.push(nullable(a[4], a[3]));
        f.extend(post_fields(db, p, &["title", "score", "views", "answers", "created"]));
        f.push(V::I(c));
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COALESCE(SUM(V.BountyAmount), 0) AS TotalBounty,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpvotes, COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownvotes,
//        COUNT(DISTINCT P.Id) AS TotalPosts, COUNT(DISTINCT C.Id) AS TotalComments
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, TotalBounty, TotalUpvotes, TotalDownvotes, TotalPosts, TotalComments,
//        ROW_NUMBER() OVER (ORDER BY Reputation DESC, TotalBounty DESC) AS Rank FROM UserStatistics)
// SELECT TU.DisplayName, TU.Reputation, TU.TotalBounty, TU.TotalUpvotes, TU.TotalDownvotes, TU.TotalPosts, TU.TotalComments,
//        CASE WHEN TU.Reputation > 10000 THEN 'High Reputation' WHEN TU.Reputation BETWEEN 5000 AND 10000 THEN 'Medium Reputation' ELSE 'Low Reputation' END AS ReputationCategory,
//        CASE WHEN TU.TotalPosts > 50 THEN 'Active User' ELSE 'Less Active User' END AS ActivityLevel
// FROM TopUsers TU WHERE TU.Rank <= 10 ORDER BY TU.Reputation DESC, TU.TotalBounty DESC;
//
// Rank leads with Reputation, so only users with at least the tenth-highest reputation can rank in the top ten; the posts x comments x votes
// product is driven for those alone.
fn q2562(db: &'static So) -> String {
    let tenth = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10).last().unwrap().1;
    let cand: MatSet<Id<User>> = db.user.with((&db.user.reputation).ge(tenth)).collect();
    let s = (&cand)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(comments_of(db).opt()).opt().and(votes_by(db).select((&db.vote.vote_type_id).and((&db.vote.bounty_amount).opt())).opt()))
        .fold([0i64; 3], |a, (_, v)| match v {
            Some((t, b)) => [a[0] + b.unwrap_or(0), a[1] + (t == 2) as i64, a[2] + (t == 3) as i64],
            None => a,
        });
    let dp = (&cand).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let dc = (&cand).group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = top_n(drain((&s).and(&dp).and(&dc)), |&(u, ((a, _), _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(a[0]), u), 10);
    rows(v.into_iter().map(|(u, ((a, p), c))| {
        let r = db.user.reputation.get(u).unwrap();
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(p), V::I(c)]);
        f.push(V::S(if r > 10000 { "High Reputation" } else if (5000..=10000).contains(&r) { "Medium Reputation" } else { "Low Reputation" }));
        f.push(V::S(if p > 50 { "Active User" } else { "Less Active User" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.OwnerUserId, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RankByUser
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, AVG(COALESCE(p.ViewCount, 0)) AS AvgPostViewCount, SUM(CASE WHEN bh.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN bh.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN bh.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges bh ON u.Id = bh.UserId GROUP BY u.Id, u.DisplayName)
// SELECT ua.UserId, ua.DisplayName, ua.AvgPostViewCount, ua.GoldBadges, ua.SilverBadges, ua.BronzeBadges, rp.Title, rp.CreationDate, rp.CommentCount, rp.UpVoteCount, rp.DownVoteCount
// FROM UserActivity ua JOIN RankedPosts rp ON ua.UserId = rp.OwnerUserId WHERE ua.AvgPostViewCount > 50
// ORDER BY ua.GoldBadges DESC, ua.SilverBadges DESC, ua.BronzeBadges DESC, rp.UpVoteCount DESC LIMIT 10 OFFSET 0;
//
// UserActivity is driven only for the owners of recent posts, the only users the join keeps.
fn q622(db: &'static So) -> String {
    let Post { creation_date, owner_user, view_count, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let owners: MatSet<Id<User>> = recent().select(owner_user).collect();
    let ua = (&owners)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(view_count.opt()).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 5], |a, (w, c)| [a[0] + 1, a[1] + w.flatten().unwrap_or(0), a[2] + (c == Some(1)) as i64, a[3] + (c == Some(2)) as i64, a[4] + (c == Some(3)) as i64]);
    let rp = recent()
        .with(owner_user)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = drain((&rp).and(owner_user.select(Ident::<User>::new().and((&ua).filt(|a| a[1] as i128 > 50 * a[0] as i128)))));
    let v = top_n(v, |&(p, (r, (_, a)))| (Reverse(a[2]), Reverse(a[3]), Reverse(a[4]), Reverse(r[1]), p), 10);
    rows(v.into_iter().map(|(p, (r, (u, a)))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([avg(a[1], a[0]), V::I(a[2]), V::I(a[3]), V::I(a[4])]);
        f.extend(post_fields(db, p, &["title", "created"]));
        f.extend(r.map(V::I));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.CreationDate, U.LastAccessDate, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpVotes,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownVotes, COUNT(DISTINCT P.Id) AS TotalPosts, COUNT(DISTINCT C.Id) AS TotalComments
//     FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId
//     GROUP BY U.Id, U.DisplayName, U.Reputation, U.CreationDate, U.LastAccessDate),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserStats)
// SELECT U.UserId, U.DisplayName, U.Reputation, U.ReputationRank, COALESCE(B.BadgeCount, 0) AS TotalBadges, COALESCE(PL.LinkedPosts, 0) AS TotalLinks
// FROM TopUsers U LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId) B ON U.UserId = B.UserId
// LEFT JOIN (SELECT P.OwnerUserId, COUNT(DISTINCT PL.RelatedPostId) AS LinkedPosts FROM PostLinks PL JOIN Posts P ON PL.PostId = P.Id GROUP BY P.OwnerUserId) PL ON U.UserId = PL.OwnerUserId
// WHERE U.ReputationRank <= 10 ORDER BY U.Reputation DESC FETCH FIRST 10 ROWS ONLY;
//
// TopUsers reads only Reputation from UserStats, whose aggregates are never projected.
fn q1201(db: &'static So) -> String {
    let r = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let r = rel(r.into_iter().take_while(|x| x.1 <= 10).map(|((u, _), r)| (u, r)).collect());
    let tu: HashIdx<Id<User>, i64> = (&r).map(|(u, _)| u).inv().select((&r).map(|(_, r)| r)).collect();
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let PostLink { post, related_post_id, .. } = &db.post_link;
    let pl = db.post_link.with(post.select(&db.post.owner_user)).group_by(post.select(&db.post.owner_user)).select(related_post_id).count_distinct();
    let v = top_n(drain((&tu).and((&bc).opt()).and((&pl).opt())), |&(u, _)| (Reverse(db.user.reputation.get(u).unwrap()), u), 10);
    rows(v.into_iter().map(|(u, ((r, b), l))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(r), V::I(b.unwrap_or(0)), V::I(l.unwrap_or(0))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.Score > 0 AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, u.DisplayName, CASE WHEN u.Reputation >= 1000 THEN 'High' WHEN u.Reputation >= 100 THEN 'Medium' ELSE 'Low' END AS ReputationLevel FROM Users u),
// PostStatistics AS (SELECT p.Id AS PostId, COUNT(c.Id) AS CommentCount, SUM(v.BountyAmount) AS TotalBounty FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId
//     LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (8, 9) GROUP BY p.Id)
// SELECT r.PostId, r.Title, r.CreationDate, u.DisplayName, u.ReputationLevel, COALESCE(s.CommentCount, 0) AS CommentCount, COALESCE(s.TotalBounty, 0) AS TotalBounty,
//        r.Score AS PostScore, r.ViewCount, CASE WHEN r.rn = 1 THEN 'Latest Post' ELSE NULL END AS Tag
// FROM RankedPosts r JOIN UserReputation u ON r.OwnerUserId = u.UserId LEFT JOIN PostStatistics s ON r.PostId = s.PostId
// WHERE u.ReputationLevel IN ('High', 'Medium') ORDER BY r.CreationDate DESC LIMIT 50;
//
// The ORDER BY reads only CreationDate, so the fifty posts are picked first and the comment x bounty product is driven for those alone.
fn q1937(db: &'static So) -> String {
    let Post { score, creation_date, owner_user, .. } = &db.post;
    let rp = || db.post.with(score.gt(0).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))));
    let latest = top_per(drain(rp().select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let latest: MatSet<Id<Post>> = rel(latest.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let v = drain(rp().with(owner_user.select(Ident::<User>::new().with((&db.user.reputation).ge(100)))));
    let top = top_n(v, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 50);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).in_v(vec![8, 9]))).select((&db.vote.bounty_amount).opt());
    let s = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(bounty.opt())).fold([0i64; 2], |a, (c, b)| [a[0] + c.is_some() as i64, a[1] + b.flatten().unwrap_or(0)]);
    let v = drain((&s).and(Ident::<Post>::new().with(&latest).opt()));
    rows(v.into_iter().map(|(p, (a, l))| {
        let r = db.user.reputation.get(owner_user.get(p).unwrap()).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend([V::S(if r >= 1000 { "High" } else { "Medium" }), V::I(a[0]), V::I(a[1])]);
        f.extend(post_fields(db, p, &["score", "views"]));
        f.push(if l.is_some() { V::S("Latest Post") } else { V::Null });
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS UpvotedPosts,
//        SUM(CASE WHEN p.Score < 0 THEN 1 ELSE 0 END) AS DownvotedPosts, SUM(v.BountyAmount) AS TotalBountiesEarned
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON v.UserId = u.Id AND v.PostId = p.Id GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, UpvotedPosts, DownvotedPosts, TotalBountiesEarned, RANK() OVER (ORDER BY Reputation DESC) AS Rank FROM UserStats),
// TopUsersPosts AS (SELECT tu.UserId, tu.DisplayName, p.Title, p.CreationDate, p.Score, COALESCE(c.CommentCount, 0) AS CommentCount
//     FROM TopUsers tu INNER JOIN Posts p ON tu.UserId = p.OwnerUserId LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
//     WHERE tu.Rank <= 10)
// SELECT t.DisplayName, COUNT(t.Title) AS TotalPosts, SUM(CASE WHEN t.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts, SUM(t.CommentCount) AS TotalComments, AVG(u.Reputation) AS AvgReputation
// FROM TopUsersPosts t JOIN Users u ON t.UserId = u.Id GROUP BY t.DisplayName HAVING SUM(t.CommentCount) > 5 ORDER BY TotalPosts DESC LIMIT 5;
//
// TopUsers reads only Reputation from UserStats, whose aggregates are never projected.
fn q2916(db: &'static So) -> String {
    let r = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let tu: MatSet<Id<User>> = rel(r.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let Post { owner_user, title, score, .. } = &db.post;
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let tp = db.post.with(owner_user.select(&tu));
    let g = tp
        .group_by(owner_user.select(&db.user.display_name))
        .select(title.opt().and(score).and(&cc).and(owner_user.select(&db.user.reputation)))
        .fold([0i64; 5], |a, (((t, s), c), r)| [a[0] + t.is_some() as i64, a[1] + (s > 0) as i64, a[2] + c, a[3] + r, a[4] + 1]);
    let v = top_n(drain((&g).filt(|a| a[2] > 5)), |&(n, a)| (Reverse(a[0]), n), 5);
    rows(v.into_iter().map(|(n, a)| row(vec![V::S(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[3], a[4])])))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.CreationDate, ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U WHERE U.Reputation > 1000),
// RecentPosts AS (SELECT P.Id AS PostId, P.OwnerUserId, P.CreationDate, P.Title, P.Score, P.ViewCount, COUNT(C.Id) AS CommentCount FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId
//     WHERE P.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' GROUP BY P.Id, P.OwnerUserId, P.CreationDate, P.Title, P.Score, P.ViewCount),
// TopPosts AS (SELECT RP.PostId, RP.Title, RP.Score, RP.ViewCount, UR.UserId, UR.DisplayName, UR.ReputationRank FROM RecentPosts RP JOIN UserReputation UR ON RP.OwnerUserId = UR.UserId
//     WHERE RP.Score > 10),
// PostHistories AS (SELECT PH.PostId, COUNT(*) AS EditCount, MAX(PH.CreationDate) AS LastEditedDate FROM PostHistory PH WHERE PH.PostHistoryTypeId IN (4, 5, 6) GROUP BY PH.PostId)
// SELECT TP.Title, TP.Score, TP.ViewCount, TP.DisplayName, TP.ReputationRank, PH.EditCount, PH.LastEditedDate, CASE WHEN PH.EditCount IS NULL THEN 'No Edits' ELSE 'Edited' END AS EditStatus
// FROM TopPosts TP LEFT JOIN PostHistories PH ON TP.PostId = PH.PostId WHERE TP.ReputationRank <= 10 ORDER BY TP.Score DESC, TP.ViewCount DESC;
//
// ReputationRank breaks a Reputation tie on the user id (the SQL leaves it open).
fn q4888(db: &'static So) -> String {
    let top = top_n(drain(db.user.with((&db.user.reputation).gt(1000)).select(&db.user.reputation)), |&(u, r)| (Reverse(r), u), 10);
    let ur = rel(top.into_iter().enumerate().map(|(i, (u, _))| (u, i as i64 + 1)).collect());
    let rank: HashIdx<Id<User>, i64> = (&ur).map(|(u, _)| u).inv().select((&ur).map(|(_, r)| r)).collect();
    let Post { creation_date, score, owner_user, .. } = &db.post;
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let ph = db.post_history.with(post_history_type_id.is_in([4, 5, 6])).group_by(post).select(hd).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let v = drain(db.post.with(creation_date.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30)).and(score.gt(10))).select(owner_user.select(Ident::<User>::new().and(&rank)).and((&ph).opt())));
    rows(v.into_iter().map(|(p, ((u, r), h))| {
        let mut f = post_fields(db, p, &["title", "score", "views"]);
        f.extend([user_col(db, u, "name"), V::I(r)]);
        f.extend(match h {
            Some((n, d)) => [V::I(n), V::T(d), V::S("Edited")],
            None => [V::Null, V::Null, V::S("No Edits")],
        });
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges,
//        COALESCE(SUM(p.ViewCount), 0) AS TotalViews
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, AnswerCount, QuestionCount, UpVotes, DownVotes, GoldBadges, SilverBadges, BronzeBadges, TotalViews,
//        RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserStatistics)
// SELECT u.UserId, u.DisplayName, u.Reputation, u.PostCount, u.AnswerCount, u.QuestionCount, u.UpVotes, u.DownVotes, u.GoldBadges, u.SilverBadges, u.BronzeBadges, u.TotalViews
// FROM TopUsers u WHERE u.ReputationRank <= 10 ORDER BY u.Reputation DESC;
//
// ReputationRank reads only Reputation, so the ranked users are picked first and the posts x votes x badges product is driven for those alone.
fn q8197(db: &'static So) -> String {
    let r = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let tu: MatSet<Id<User>> = rel(r.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let Post { post_type_id, view_count, .. } = &db.post;
    let s = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(view_count.opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 8], |a, (p, c)| {
            let ((t, w), v) = p.map_or(((0, None), None), |x| x);
            [
                a[0] + (t == 2) as i64,
                a[1] + (t == 1) as i64,
                a[2] + (v == Some(2)) as i64,
                a[3] + (v == Some(3)) as i64,
                a[4] + (c == Some(1)) as i64,
                a[5] + (c == Some(2)) as i64,
                a[6] + (c == Some(3)) as i64,
                a[7] + w.unwrap_or(0),
            ]
        });
    let pc = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    rows(drain((&pc).and(&s)).into_iter().map(|(u, (n, a))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT v.Id) FILTER (WHERE v.VoteTypeId = 2) AS UpVoteCount,
//        COUNT(DISTINCT v.Id) FILTER (WHERE v.VoteTypeId = 3) AS DownVoteCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 month'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.PostTypeId),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, CommentCount, UpVoteCount, DownVoteCount FROM RankedPosts WHERE Rank <= 5)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.CommentCount, tp.UpVoteCount, tp.DownVoteCount, COALESCE(b.Name, 'No Badge') AS TopUserBadge,
//        AVG(u.Reputation) AS AverageReputation
// FROM TopPosts tp LEFT JOIN Users u ON u.Id = (SELECT OwnerUserId FROM Posts WHERE Id = tp.PostId) LEFT JOIN Badges b ON b.UserId = u.Id AND b.Class = 1
// GROUP BY tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.CommentCount, tp.UpVoteCount, tp.DownVoteCount, b.Name ORDER BY tp.Score DESC;
//
// Rank reads only Score, so the top posts are picked first; it breaks a Score tie on the post id (the SQL leaves it open). The joined rows
// are grouped by (post, badge name).
fn q8812(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let gold = badges_of(db).select(Ident::<Badge>::new().with((&db.badge.class).eq(1)));
    let j = rel(drain((&tp).select(owner_user.select(Ident::<User>::new().and(gold.select(&db.badge.name).opt())).opt())));
    type R = (Id<Post>, Option<(Id<User>, Option<Str>)>);
    let g = (&j)
        .group_by(Same::<R>::new().map(|(p, u): R| (p, u.and_then(|x| x.1))))
        .select(Same::<R>::new().map(|(_, u): R| u.map(|x| x.0)).flat_map(|u: Option<Id<User>>| u).select(&db.user.reputation).opt())
        .fold([0i64; 2], |a, r| match r {
            Some(r) => [a[0] + 1, a[1] + r],
            None => a,
        });
    let gv = rel(drain(&g));
    type G = ((Id<Post>, Option<Str>), [i64; 2]);
    let v = drain((&gv).select(Same::<G>::new().and(Same::<G>::new().map(|((p, _), _): G| p).select((&cc).and(&vc)))));
    rows(v.into_iter().map(|(_, (((p, b), a), (c, u)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(c), V::I(u[0]), V::I(u[1]), V::S(b.unwrap_or("No Badge")), avg(a[1], a[0])]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COALESCE(SUM(P.ViewCount), 0) AS TotalViews, COALESCE(COUNT(DISTINCT P.Id), 0) AS TotalPosts,
//        COALESCE(SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END), 0) AS TotalQuestions, COALESCE(SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalAnswers,
//        COALESCE(SUM(CASE WHEN PH.PostHistoryTypeId = 10 THEN 1 ELSE 0 END), 0) AS TotalClosures, COALESCE(SUM(CASE WHEN PH.PostHistoryTypeId = 24 THEN 1 ELSE 0 END), 0) AS TotalEdits
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN PostHistory PH ON P.Id = PH.PostId GROUP BY U.Id, U.DisplayName, U.Reputation),
// AggregatedData AS (SELECT UserId, DisplayName, Reputation, TotalViews, TotalPosts, TotalQuestions, TotalAnswers, TotalClosures, TotalEdits,
//        RANK() OVER (ORDER BY TotalViews DESC, Reputation DESC) AS ViewRank, RANK() OVER (ORDER BY TotalPosts DESC) AS PostRank FROM UserActivity),
// MostActiveUsers AS (SELECT *, GREATEST(ViewRank, PostRank) AS CombinedRank FROM AggregatedData)
// SELECT UserId, DisplayName, Reputation, TotalViews, TotalPosts, TotalQuestions, TotalAnswers, TotalClosures, TotalEdits, CombinedRank
// FROM MostActiveUsers WHERE CombinedRank <= 10 ORDER BY CombinedRank;
fn q29968(db: &'static So) -> String {
    let Post { post_type_id, view_count, .. } = &db.post;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(view_count.opt()).and(history_of(db).select(&db.post_history.post_history_type_id).opt())).opt())
        .fold([0i64; 5], |a, p| match p {
            Some(((t, w), h)) => [a[0] + w.unwrap_or(0), a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (h == Some(10)) as i64, a[4] + (h == Some(24)) as i64],
            None => a,
        });
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let v = ranked(drain((&s).and(&pc)), |&(u, (a, _))| (Reverse(a[0]), Reverse(db.user.reputation.get(u).unwrap())), false);
    let v = ranked(v, |&((_, (_, n)), _)| Reverse(n), false);
    let v = rel(v.into_iter().map(|(((u, (a, n)), r1), r2)| (u, a, n, r1.max(r2))).collect());
    type R = (Id<User>, [i64; 5], i64, i64);
    let v = drain((&v).filt(|(_, _, _, c): R| c <= 10));
    rows(v.into_iter().map(|(_, (u, a, n, c))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[0]), V::I(n), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4]), V::I(c)]);
        row(f)
    }))
}

// WITH RecursivePostStatistics AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, COALESCE(V.TotalVotes, 0) AS TotalVotes,
//        ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS RN, P.OwnerUserId
//     FROM Posts P LEFT JOIN (SELECT PostId, COUNT(*) AS TotalVotes FROM Votes WHERE VoteTypeId IN (2, 3) GROUP BY PostId) V ON P.Id = V.PostId),
// UserBadges AS (SELECT U.Id AS UserId, COUNT(B.Id) AS BadgeCount FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id),
// TopPosts AS (SELECT RPS.PostId, RPS.Title, RPS.CreationDate, RPS.TotalVotes, U.DisplayName, UB.BadgeCount FROM RecursivePostStatistics RPS
//     JOIN Users U ON RPS.RN = 1 AND RPS.OwnerUserId = U.Id JOIN UserBadges UB ON U.Id = UB.UserId WHERE UB.BadgeCount > 0 ORDER BY RPS.TotalVotes DESC LIMIT 10)
// SELECT TP.PostId, TP.Title, TP.CreationDate, TP.TotalVotes, TP.DisplayName, CASE WHEN TP.BadgeCount > 0 THEN 'User has badges' ELSE 'No badges' END AS BadgeStatus
// FROM TopPosts TP LEFT JOIN PostHistory PH ON TP.PostId = PH.PostId AND PH.PostHistoryTypeId = 10 WHERE PH.Id IS NULL ORDER BY TP.TotalVotes DESC;
//
// Not recursive. The newest post per owner breaks a CreationDate tie on the post id (the SQL leaves it open).
fn q34698(db: &'static So) -> String {
    let Post { owner_user, creation_date, .. } = &db.post;
    let first = top_per(drain(db.post.select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let first: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let tv = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([2, 3])));
    let tc = (&first).group_by(Ident::<Post>::new()).select(tv.opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let v = drain((&first).with(owner_user.select(Ident::<User>::new().with(badges_of(db)))).select(&tc));
    let top = top_n(v, |&(p, n)| (Reverse(n), p), 10);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10)));
    let v = drain((&tp).minus(closes).select(&tc));
    rows(v.into_iter().map(|(p, n)| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(n), post_fields(db, p, &["owner"]).pop().unwrap(), V::S("User has badges")]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U),
// RecentPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.ViewCount, P.Score, U.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS PostRank
//     FROM Posts P LEFT JOIN Users U ON P.OwnerUserId = U.Id WHERE P.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 YEAR'),
// TopPosts AS (SELECT RP.*, UR.ReputationRank FROM RecentPosts RP JOIN UserReputation UR ON RP.OwnerDisplayName = UR.DisplayName WHERE RP.PostRank = 1),
// VotesSummary AS (SELECT V.PostId, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes V GROUP BY V.PostId),
// FinalSummary AS (SELECT TP.*, VS.UpVotes, VS.DownVotes, (TP.Score + COALESCE(VS.UpVotes, 0) - COALESCE(VS.DownVotes, 0)) AS NetScore FROM TopPosts TP LEFT JOIN VotesSummary VS ON TP.PostId = VS.PostId)
// SELECT FS.OwnerDisplayName, FS.Title, FS.CreationDate, FS.ViewCount, FS.Score, FS.UpVotes, FS.DownVotes, FS.NetScore, FS.ReputationRank
// FROM FinalSummary FS WHERE FS.ReputationRank <= 10 ORDER BY FS.NetScore DESC, FS.ViewCount DESC LIMIT 10;
//
// The newest post per owner breaks a CreationDate tie on the post id (the SQL leaves it open).
fn q398(db: &'static So) -> String {
    let r = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let r = rel(r.into_iter().take_while(|x| x.1 <= 10).map(|((u, _), r)| (u, r)).collect());
    let by_name: HashIdx<Str, (Id<User>, i64)> = (&r).map(|(u, _)| u).select(&db.user.display_name).inv().select(&r).collect();
    let Post { creation_date, owner_user, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user));
    let first = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let first: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let vs = (&first).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let v = drain((&first).select(owner_user.select(&db.user.display_name).select(&by_name).and((&vs).opt())));
    let v = top_n(v, |&(p, ((u, _), a))| {
        let w = view_count.get(p);
        let a = a.unwrap_or([0; 2]);
        (Reverse(score.get(p).unwrap() + a[0] - a[1]), w.is_none(), Reverse(w), p, u)
    }, 10);
    rows(v.into_iter().map(|(p, ((_, r), a))| {
        let mut f = post_fields(db, p, &["owner", "title", "created", "views", "score"]);
        let s = score.get(p).unwrap();
        f.extend(match a {
            Some(a) => [V::I(a[0]), V::I(a[1]), V::I(s + a[0] - a[1])],
            None => [V::Null, V::Null, V::I(s)],
        });
        f.push(V::I(r));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.OwnerUserId, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// PostStatistics AS (SELECT rp.OwnerUserId, COUNT(rp.PostId) AS TotalPosts, SUM(rp.Score) AS TotalScore, AVG(rp.Score) AS AvgScore FROM RankedPosts rp GROUP BY rp.OwnerUserId),
// CloseReasonCounts AS (SELECT ph.UserId, COUNT(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 END) AS CloseVoteCount, COUNT(CASE WHEN ph.PostHistoryTypeId = 11 THEN 1 END) AS ReopenVoteCount
//     FROM PostHistory ph GROUP BY ph.UserId)
// SELECT us.DisplayName AS UserName, ps.TotalPosts, ps.TotalScore, ps.AvgScore, COALESCE(cr.CloseVoteCount, 0) AS CloseVotes, COALESCE(cr.ReopenVoteCount, 0) AS ReopenVotes,
//        CASE WHEN ps.AvgScore > 50 THEN 'High Performer' WHEN ps.AvgScore BETWEEN 20 AND 50 THEN 'Average Performer' ELSE 'Low Performer' END AS PerformerStatus
// FROM PostStatistics ps JOIN Users us ON ps.OwnerUserId = us.Id LEFT JOIN CloseReasonCounts cr ON cr.UserId = us.Id WHERE ps.TotalPosts > 10 ORDER BY ps.TotalScore DESC LIMIT 100;
fn q3234(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let ps = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(owner_user).select(score).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let PostHistory { user, post_history_type_id, .. } = &db.post_history;
    let cr = db.post_history.group_by(user).select(post_history_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 10) as i64, a[1] + (t == 11) as i64]);
    let v = top_n(drain((&ps).filt(|a| a[0] > 10).and((&cr).opt())), |&(u, (a, _))| (Reverse(a[1]), u), 100);
    rows(v.into_iter().map(|(u, (a, c))| {
        let c = c.unwrap_or([0; 2]);
        let (s, n) = (a[1] as i128, a[0] as i128);
        let cat = if s > 50 * n { "High Performer" } else if s >= 20 * n { "Average Performer" } else { "Low Performer" };
        row(vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), avg(a[1], a[0]), V::I(c[0]), V::I(c[1]), V::S(cat)])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, u.DisplayName AS OwnerDisplayName, COUNT(co.Id) AS CommentCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes,
//        RANK() OVER (ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments co ON p.Id = co.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= CURRENT_DATE - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, u.DisplayName),
// TopRankedPosts AS (SELECT PostId, Title, CreationDate, Score, OwnerDisplayName, CommentCount, Upvotes, Downvotes FROM RankedPosts WHERE Rank <= 10)
// SELECT trp.PostId, trp.Title, trp.CreationDate, trp.Score, trp.OwnerDisplayName, trp.CommentCount, trp.Upvotes, trp.Downvotes, (trp.Upvotes - trp.Downvotes) AS NetVotes,
//        COUNT(ph.Id) AS PostHistoryCount
// FROM TopRankedPosts trp LEFT JOIN PostHistory ph ON trp.PostId = ph.PostId
// GROUP BY trp.PostId, trp.Title, trp.CreationDate, trp.Score, trp.OwnerDisplayName, trp.CommentCount, trp.Upvotes, trp.Downvotes ORDER BY trp.Score DESC, trp.CreationDate DESC;
//
// Rank reads only base columns, so the top questions are picked first and the comment x vote product is driven for those alone.
fn q6865(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, owner_user, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_days(current_date(), -30)))).with(owner_user));
    let r = ranked(v, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())), false);
    let tp: MatSet<Id<Post>> = rel(r.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let hc = (&tp).group_by(Ident::<Post>::new()).select(history_of(db).opt()).fold(0i64, |n, h| n + h.is_some() as i64);
    rows(drain((&s).and(&hc)).into_iter().map(|(p, (a, h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[1] - a[2]), V::I(h)]);
        row(f)
    }))
}

// WITH TagCounts AS (SELECT T.TagName, COUNT(DISTINCT P.Id) AS PostCount, SUM(COALESCE(P.ViewCount, 0)) AS TotalViews, SUM(COALESCE(P.AnswerCount, 0)) AS TotalAnswers,
//        SUM(COALESCE(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END)) AS Upvotes, SUM(COALESCE(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END)) AS Downvotes
//     FROM Tags T LEFT JOIN Posts P ON P.Tags LIKE '%' || T.TagName || '%' LEFT JOIN Votes V ON V.PostId = P.Id WHERE T.IsModeratorOnly IS NOT TRUE GROUP BY T.TagName),
// RankedTags AS (SELECT TagName, PostCount, TotalViews, TotalAnswers, Upvotes, Downvotes, RANK() OVER (ORDER BY TotalViews DESC) AS ViewRank,
//        RANK() OVER (ORDER BY TotalAnswers DESC) AS AnswerRank, RANK() OVER (ORDER BY Upvotes - Downvotes DESC) AS VoteRank FROM TagCounts),
// BenchmarkResults AS (SELECT TagName, PostCount, TotalViews, TotalAnswers, Upvotes, Downvotes, ViewRank, AnswerRank, VoteRank,
//        (PostCount * 0.4) + (TotalViews * 0.3) + (TotalAnswers * 0.2) + ((Upvotes - Downvotes) * 0.1) AS BenchmarkScore FROM RankedTags)
// SELECT TagName, PostCount, TotalViews, TotalAnswers, Upvotes, Downvotes, BenchmarkScore, DENSE_RANK() OVER (ORDER BY BenchmarkScore DESC) AS OverallRank
// FROM BenchmarkResults ORDER BY OverallRank LIMIT 10;
//
// The DECIMAL score is computed exactly in tenths. The Rank columns of RankedTags are never read.
fn q29291(db: &'static So) -> String {
    let lt = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, Id<Post>> = (&lt).map(|(_, t)| t).inv().select((&lt).map(|(p, _)| p)).collect();
    let Post { view_count, answer_count, .. } = &db.post;
    let tags = || db.tag.minus((&db.tag.is_moderator_only).eq(1));
    let tc = tags()
        .group_by(&db.tag.tag_name)
        .select((&by_tag).select(view_count.opt().and(answer_count.opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some(((w, n), t)) => [a[0] + w.unwrap_or(0), a[1] + n.unwrap_or(0), a[2] + (t == Some(2)) as i64, a[3] + (t == Some(3)) as i64],
            None => a,
        });
    let pc = tags().group_by(&db.tag.tag_name).select((&by_tag).opt()).buf_fold(distinct_some);
    let v = drain((&tc).and(&pc));
    let tenths = |&(_, (a, n)): &(Str, ([i64; 4], i64))| n as i128 * 4 + a[0] as i128 * 3 + a[1] as i128 * 2 + (a[2] - a[3]) as i128;
    let v = ranked(v, |x| Reverse(tenths(x)), true);
    let v = top_n(v, |&(x, r)| (r, x.0), 10);
    rows(v.into_iter().map(|(x, r)| {
        let (t, (a, n)) = x;
        row(vec![V::S(t), V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::F(tenths(&x) as f64 / 10.0), V::I(r)])
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS PostCount, SUM(V.BountyAmount) AS TotalBounties, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UA.UserId, UA.DisplayName, UA.PostCount, UA.TotalBounties, ROW_NUMBER() OVER (ORDER BY UA.PostCount DESC) AS Rank FROM UserActivity UA WHERE UA.PostCount > 0),
// PostDetails AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, COALESCE(B.Count, 0) AS TagCount, COUNT(C.Id) AS CommentCount
//     FROM Posts P LEFT JOIN Tags B ON P.Tags LIKE '%' || B.TagName || '%' LEFT JOIN Comments C ON P.Id = C.PostId GROUP BY P.Id, P.Title, P.CreationDate, P.Score, B.Count)
// SELECT TU.DisplayName, TU.PostCount, TU.TotalBounties, PD.Title, PD.CreationDate, PD.Score, PD.TagCount, PD.CommentCount,
//        RANK() OVER (PARTITION BY TU.UserId ORDER BY PD.Score DESC) AS PostRank
// FROM TopUsers TU JOIN PostDetails PD ON TU.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = PD.PostId LIMIT 1) WHERE TU.Rank <= 10 ORDER BY TU.Rank, PD.Score DESC;
//
// Rank reads only the distinct post count, so the ten users are picked first (a PostCount tie broken on the user id; the SQL leaves it open)
// and the posts x votes product is driven for them alone. PostDetails groups the post x tag x comment rows by (post, tag Count), so one
// group can hold several tags; it is built only for those users' posts, the only ones the join keeps. The subquery is the post's owner.
fn q1318(db: &'static So) -> String {
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let tu = top_n(drain((&pc).filt(|n| n > 0)), |&(u, n)| (Reverse(n), u), 10);
    let tu = rel(tu.into_iter().enumerate().map(|(i, (u, _))| (u, i as i64 + 1)).collect());
    let rank: HashIdx<Id<User>, i64> = (&tu).map(|(u, _)| u).inv().select((&tu).map(|(_, r)| r)).collect();
    let bounty = votes_of(db).select((&db.vote.bounty_amount).opt());
    let tb = db.user.with(&rank).group_by(Ident::<User>::new()).select(posts_of(db).select(bounty.opt())).fold([0i64; 2], |a, b| match b.flatten() {
        Some(x) => [a[0] + 1, a[1] + x],
        None => a,
    });
    let lt = tag_mentions(db);
    let tags_of: HashIdx<Id<Post>, Id<Tag>> = (&lt).map(|(p, _)| p).inv().select((&lt).map(|(_, t)| t)).collect();
    let Post { owner_user, score, .. } = &db.post;
    let j = rel(drain(db.post.with(owner_user.select(&rank)).select((&tags_of).select(Ident::<Tag>::new().and(&db.tag.count)).opt())));
    type R = (Id<Post>, Option<(Id<Tag>, i64)>);
    let pd = (&j)
        .group_by(Same::<R>::new().map(|(p, t): R| (p, t.map(|x| x.1))))
        .select(Same::<R>::new().map(|(p, _): R| p).select(comments_of(db).opt()))
        .fold(0i64, |n, c| n + c.is_some() as i64);
    let rows_ = drain(&pd);
    let v = ranked(rows_, |&((p, _), _)| (owner_user.get(p).unwrap(), Reverse(score.get(p).unwrap())), false);
    let v = per_group(v, |&((p, _), _)| owner_user.get(p).unwrap());
    let v = rel(v);
    type G = (((Id<Post>, Option<i64>), i64), i64);
    let v = drain((&v).select(Same::<G>::new().and(Same::<G>::new().map(|(((p, _), _), _): G| p).select(owner_user.select(Ident::<User>::new().and(&pc).and(&tb))))));
    rows(v.into_iter().map(|(_, ((((p, t), c), r), ((u, n), b)))| {
        let mut f = vec![user_col(db, u, "name"), V::I(n), nullable(b[1], b[0])];
        f.extend(post_fields(db, p, &["title", "created", "score"]));
        f.extend([V::I(t.unwrap_or(0)), V::I(c), V::I(r)]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT Id AS UserId, Reputation, CASE WHEN Reputation >= 10000 THEN 'Expert' WHEN Reputation >= 1000 THEN 'Experienced' WHEN Reputation >= 100 THEN 'Novice'
//        ELSE 'Beginner' END AS ReputationTier FROM Users),
// PostStatistics AS (SELECT p.Id AS PostId, p.OwnerUserId, p.PostTypeId, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount, COUNT(DISTINCT ph.UserId) FILTER (WHERE ph.PostHistoryTypeId IN (10, 11)) AS CloseOpenCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.OwnerUserId, p.PostTypeId),
// RankedPosts AS (SELECT ps.PostId, ps.OwnerUserId, ps.CommentCount, ps.UpVoteCount, ps.DownVoteCount, ur.ReputationTier,
//        RANK() OVER (PARTITION BY ur.ReputationTier ORDER BY ps.UpVoteCount DESC) AS PostRank FROM PostStatistics ps JOIN UserReputation ur ON ps.OwnerUserId = ur.UserId)
// SELECT rp.PostId, rp.CommentCount, rp.UpVoteCount, rp.DownVoteCount, rp.ReputationTier, rp.PostRank FROM RankedPosts rp WHERE rp.PostRank <= 5 ORDER BY rp.ReputationTier, rp.PostRank;
//
// CloseOpenCount is never read.
fn q4153(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let ps = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(owner_user)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(history_of(db).opt()))
        .fold([0i64; 3], |a, ((c, t), _)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let tier = |r: i64| if r >= 10000 { "Expert" } else if r >= 1000 { "Experienced" } else if r >= 100 { "Novice" } else { "Beginner" };
    let v = drain((&ps).and(owner_user.select((&db.user.reputation).map(tier))));
    let v = ranked(v, |&(_, (a, t))| (t, Reverse(a[1])), false);
    let v = per_group(v, |&(_, (_, t))| t);
    let v = drain(rel(v).filt(|(_, r): ((Id<Post>, ([i64; 3], &'static str)), i64)| r <= 5));
    rows(v.into_iter().map(|(_, ((p, (a, t)), r))| {
        let mut f = post_fields(db, p, &["id"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::S(t), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS rn
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.PostTypeId IN (1, 2)),
// PostMetrics AS (SELECT rp.PostId, rp.Title, rp.OwnerDisplayName, rp.CreationDate, rp.Score, rp.ViewCount, COALESCE(COUNT(c.Id), 0) AS CommentCount, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty
//     FROM RankedPosts rp LEFT JOIN Comments c ON c.PostId = rp.PostId LEFT JOIN Votes v ON v.PostId = rp.PostId AND v.VoteTypeId IN (8, 9) WHERE rp.rn <= 5
//     GROUP BY rp.PostId, rp.Title, rp.OwnerDisplayName, rp.CreationDate, rp.Score, rp.ViewCount),
// FinalOutput AS (SELECT pm.*, CASE WHEN pm.Score > 100 THEN 'Hot' WHEN pm.Score > 50 THEN 'Popular' ELSE 'Normal' END AS Popularity FROM PostMetrics pm)
// SELECT fo.PostId, fo.Title, fo.OwnerDisplayName, fo.CreationDate, fo.Score, fo.ViewCount, fo.CommentCount, fo.TotalBounty, fo.Popularity
// FROM FinalOutput fo ORDER BY fo.Popularity DESC, fo.Score DESC LIMIT 50;
fn q8173(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.is_in([1, 2]).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).in_v(vec![8, 9]))).select((&db.vote.bounty_amount).opt());
    let pm = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(bounty.opt())).fold([0i64; 2], |a, (c, b)| [a[0] + c.is_some() as i64, a[1] + b.flatten().unwrap_or(0)]);
    let pop = |s: i64| if s > 100 { "Hot" } else if s > 50 { "Popular" } else { "Normal" };
    let v = top_n(drain(&pm), |&(p, _)| {
        let s = score.get(p).unwrap();
        (Reverse(pop(s)), Reverse(s), p)
    }, 50);
    rows(v.into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "score", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::S(pop(score.get(p).unwrap()))]);
        row(f)
    }))
}

// Rewritten (rewrites/2513.sql): the NTILE order is tie-broken on Id and TopPosts' ORDER BY on PS.Id.
// WITH UserReputation AS (SELECT Id, Reputation, CreationDate, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS ReputationRank,
//        NTILE(5) OVER (ORDER BY Reputation, Id) AS ReputationTier FROM Users),
// PostStats AS (SELECT P.Id, P.Title, P.OwnerUserId, P.CreationDate, COALESCE(P.AcceptedAnswerId, 0) AS AcceptedAnswerId, COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVoteCount,
//        COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVoteCount, COUNT(CASE WHEN V.VoteTypeId = 1 THEN 1 END) AS AcceptedVoteCount
//     FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY P.Id, P.Title, P.OwnerUserId, P.CreationDate, P.AcceptedAnswerId),
// TopPosts AS (SELECT PS.Id, PS.Title, PS.UpVoteCount, PS.DownVoteCount, (PS.UpVoteCount - PS.DownVoteCount) AS NetScore, UR.ReputationTier
//     FROM PostStats PS INNER JOIN UserReputation UR ON PS.OwnerUserId = UR.Id WHERE PS.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'
//     ORDER BY NetScore DESC, PS.Id LIMIT 10)
// SELECT T.Id, T.Title, T.UpVoteCount, T.DownVoteCount, T.NetScore, T.ReputationTier, COALESCE(CH.CommentCount, 0) AS CommentsMade,
//        (SELECT COUNT(*) FROM PostHistory PH WHERE PH.PostId = T.Id AND PH.PostHistoryTypeId IN (10, 11)) AS CloseReopenCount
// FROM TopPosts T LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) CH ON T.Id = CH.PostId ORDER BY T.NetScore DESC, T.ReputationTier ASC;
//
// NTILE(5) is taken over the users sorted by (Reputation, Id): the first n % 5 tiles hold one row more.
fn q2513(db: &'static So) -> String {
    let mut us = drain(&db.user.reputation);
    us.sort_by_key(|&(u, r)| (r, db.user.origid.get(u).unwrap()));
    let n = us.len();
    let (q, extra) = (n / 5, n % 5);
    let tile = |i: usize| if i < extra * (q + 1) { i / (q + 1) + 1 } else { extra + (i - extra * (q + 1)) / q + 1 };
    let ur = rel(us.into_iter().enumerate().map(|(i, (u, _))| (u, tile(i) as i64)).collect());
    let tier: HashIdx<Id<User>, i64> = (&ur).map(|(u, _)| u).inv().select((&ur).map(|(_, t)| t)).collect();
    let Post { creation_date, owner_user, origid, .. } = &db.post;
    let ps = db
        .post
        .with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .with(owner_user)
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt())
        .fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let top = top_n(drain(&ps), |&(p, a)| (Reverse(a[0] - a[1]), origid.get(p).unwrap()), 10);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).in_v(vec![10, 11])));
    let hc = (&tp).group_by(Ident::<Post>::new()).select(closes.opt()).fold(0i64, |n, h| n + h.is_some() as i64);
    let v = drain((&ps).and(owner_user.select(&tier)).and(&cc).and(&hc));
    rows(v.into_iter().map(|(p, (((a, t), c), h))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[0] - a[1]), V::I(t), V::I(c), V::I(h)]);
        row(f)
    }))
}

// WITH ranked_posts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.ViewCount > 100),
// top_posts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.OwnerDisplayName FROM ranked_posts rp WHERE rp.Rank <= 10),
// post_comments AS (SELECT pc.PostId, COUNT(pc.Id) AS CommentCount FROM Comments pc GROUP BY pc.PostId),
// post_votes AS (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes FROM Votes v GROUP BY v.PostId)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.OwnerDisplayName, COALESCE(pc.CommentCount, 0) AS CommentCount, COALESCE(pv.Upvotes, 0) AS Upvotes,
//        COALESCE(pv.Downvotes, 0) AS Downvotes
// FROM top_posts tp LEFT JOIN post_comments pc ON tp.PostId = pc.PostId LEFT JOIN post_votes pv ON tp.PostId = pv.PostId ORDER BY tp.Score DESC, tp.ViewCount DESC;
fn q6215(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(view_count.gt(100))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    rows(drain((&cc).and(&vc)).into_iter().map(|(p, (c, a))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS rn
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'),
// TopPosts AS (SELECT rp.* FROM RankedPosts rp WHERE rp.rn = 1),
// VoteSummary AS (SELECT PostId, COUNT(CASE WHEN VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN VoteTypeId = 3 THEN 1 END) AS DownVotes FROM Votes GROUP BY PostId),
// PostStatistics AS (SELECT tp.Id, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, COALESCE(vs.UpVotes, 0) AS UpVotes, COALESCE(vs.DownVotes, 0) AS DownVotes, tp.OwnerDisplayName
//     FROM TopPosts tp LEFT JOIN VoteSummary vs ON tp.Id = vs.PostId)
// SELECT ps.Id, ps.Title, ps.CreationDate, ps.Score, ps.ViewCount, ps.UpVotes, ps.DownVotes, (ps.UpVotes - ps.DownVotes) AS NetVotes, COUNT(c.Id) AS CommentCount
// FROM PostStatistics ps LEFT JOIN Comments c ON ps.Id = c.PostId WHERE ps.Score > 5 GROUP BY ps.Id, ps.Title, ps.CreationDate, ps.Score, ps.ViewCount, ps.UpVotes, ps.DownVotes
// ORDER BY NetVotes DESC, ps.CreationDate DESC LIMIT 10;
//
// rn breaks a Score tie on the post id (the SQL leaves it open).
fn q2764(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(date(2024, 10, 1), -1)))).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let good = || (&tp).with(score.gt(5));
    let vs = good().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = good().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = top_n(drain((&vs).and(&cc)), |&(p, (a, _))| (Reverse(a[0] - a[1]), Reverse(creation_date.get(p).unwrap()), p), 10);
    rows(v.into_iter().map(|(p, (a, c))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[0] - a[1]), V::I(c)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 YEAR'),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.OwnerName FROM RankedPosts rp WHERE rp.Rank <= 10),
// PostAnalysis AS (SELECT tp.PostId, tp.Title, tp.OwnerName, tp.CreationDate, tp.Score, tp.ViewCount, COALESCE(COUNT(c.Id), 0) AS CommentCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes
//     FROM TopPosts tp LEFT JOIN Comments c ON tp.PostId = c.PostId LEFT JOIN Votes v ON tp.PostId = v.PostId GROUP BY tp.PostId, tp.Title, tp.OwnerName, tp.CreationDate, tp.Score, tp.ViewCount)
// SELECT pa.PostId, pa.Title, pa.OwnerName, pa.CreationDate, pa.Score, pa.ViewCount, pa.CommentCount, pa.UpVotes, pa.DownVotes, (pa.UpVotes - pa.DownVotes) AS NetVotes
// FROM PostAnalysis pa ORDER BY pa.Score DESC, pa.CommentCount DESC;
fn q6069(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain(&s).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "score", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[1] - a[2])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS PostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.Score > 0),
// RecentVotes AS (SELECT v.PostId, COUNT(*) AS VoteCount FROM Votes v WHERE v.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' GROUP BY v.PostId),
// PostDetails AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.OwnerDisplayName, COALESCE(rv.VoteCount, 0) AS RecentVoteCount
//     FROM RankedPosts rp LEFT JOIN RecentVotes rv ON rp.PostId = rv.PostId WHERE rp.PostRank <= 5)
// SELECT pd.Title, pd.CreationDate, pd.Score, pd.ViewCount, pd.OwnerDisplayName, pd.RecentVoteCount, pt.Name AS PostTypeName, COUNT(DISTINCT c.Id) AS CommentCount
// FROM PostDetails pd JOIN PostTypes pt ON pd.PostId IN (SELECT Id FROM Posts WHERE PostTypeId = pt.Id) LEFT JOIN Comments c ON pd.PostId = c.PostId
// GROUP BY pd.Title, pd.CreationDate, pd.Score, pd.ViewCount, pd.OwnerDisplayName, pd.RecentVoteCount, pt.Name ORDER BY pd.Score DESC, pd.RecentVoteCount DESC, pd.CreationDate DESC;
//
// The IN subquery makes pt the post's own type. The final GROUP BY does not name the post, so posts that agree on every grouped column
// share a group; the rows are grouped by that tuple.
fn q9381(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, title, view_count, post_type, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(score.gt(0))).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let recent = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.creation_date).gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30))));
    let rv = (&tp).group_by(Ident::<Post>::new()).select(recent.opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let key = title.opt().and(creation_date).and(score).and(view_count.opt()).and(owner_user.select(&db.user.display_name)).and(&rv).and(post_type.select(&db.post_type.name));
    let g = (&tp).group_by(key).select(comments_of(db).opt()).buf_fold(distinct_some);
    rows(drain(&g).into_iter().map(|(((((((t, d), s), w), o), r), n), c)| {
        row(vec![ostr(t), V::T(d), V::I(s), oint(w), V::S(o), V::I(r), V::S(n), V::I(c)])
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN p.PostTypeId = 1 AND p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedQuestions,
//        AVG(p.Score) AS AvgScore FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, AcceptedQuestions, AvgScore, RANK() OVER (ORDER BY AvgScore DESC) AS ScoreRank
//     FROM UserPostStats WHERE TotalPosts > 0),
// PostVotes AS (SELECT p.Id AS PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id)
// SELECT u.DisplayName, t.TotalPosts, t.TotalQuestions, t.TotalAnswers, t.AcceptedQuestions, t.AvgScore, pv.Upvotes, pv.Downvotes, COALESCE(pv.Upvotes - pv.Downvotes, 0) AS NetVotes
// FROM TopUsers t JOIN PostVotes pv ON pv.PostId IN (SELECT Id FROM Posts WHERE OwnerUserId = t.UserId) LEFT JOIN Users u ON u.Id = t.UserId WHERE t.ScoreRank <= 10
// ORDER BY t.AvgScore DESC;
//
// The IN subquery pairs each ranked user with each of their posts. AvgScore is ranked as an exact fraction.
fn q2596(db: &'static So) -> String {
    let Post { post_type_id, accepted_answer, score, .. } = &db.post;
    let us = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(accepted_answer.opt()).and(score))).fold([0i64; 5], |a, ((t, x), s)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (t == 1 && x.is_some()) as i64, a[4] + s]
    });
    #[derive(PartialEq, Eq)]
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
    let r = ranked(drain(&us), |&(_, a)| Reverse(Frac(a[4], a[0])), false);
    let tu: MatSet<Id<User>> = rel(r.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let pv = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = drain((&tu).select((&us).and(posts_of(db).select(&pv))));
    rows(v.into_iter().map(|(u, (a, pv))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), avg(a[4], a[0]), V::I(pv[0]), V::I(pv[1]), V::I(pv[0] - pv[1])]);
        row(f)
    }))
}

// WITH RankedUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, RANK() OVER (ORDER BY u.Reputation DESC) AS ReputationRank FROM Users u),
// PostStatistics AS (SELECT p.OwnerUserId, COALESCE(SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END), 0) AS QuestionCount, COALESCE(SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS AnswerCount,
//        COALESCE(SUM(CASE WHEN p.ClosedDate IS NOT NULL THEN 1 ELSE 0 END), 0) AS ClosedPostCount, COALESCE(SUM(p.Score), 0) AS TotalScore FROM Posts p GROUP BY p.OwnerUserId),
// UserBadges AS (SELECT b.UserId, COUNT(*) AS BadgeCount FROM Badges b GROUP BY b.UserId),
// UserPosts AS (SELECT ps.OwnerUserId, COUNT(*) AS TotalPosts FROM Posts ps GROUP BY ps.OwnerUserId)
// SELECT ru.UserId, ru.DisplayName, ru.Reputation, ps.QuestionCount, ps.AnswerCount, ps.ClosedPostCount, ps.TotalScore, COALESCE(ub.BadgeCount, 0) AS BadgeCount,
//        COALESCE(up.TotalPosts, 0) AS TotalPosts, CASE WHEN ru.Reputation >= 10000 THEN 'Gold' WHEN ru.Reputation >= 1000 THEN 'Silver' ELSE 'Bronze' END AS ReputationTier
// FROM RankedUsers ru LEFT JOIN PostStatistics ps ON ru.UserId = ps.OwnerUserId LEFT JOIN UserBadges ub ON ru.UserId = ub.UserId LEFT JOIN UserPosts up ON ru.UserId = up.OwnerUserId
// WHERE ru.ReputationRank <= 100 ORDER BY ru.Reputation DESC;
fn q8950(db: &'static So) -> String {
    let r = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let tu: MatSet<Id<User>> = rel(r.into_iter().take_while(|x| x.1 <= 100).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let Post { owner_user, post_type_id, closed_date, score, .. } = &db.post;
    let ps = db.post.group_by(owner_user).select(post_type_id.and(closed_date.opt()).and(score)).fold([0i64; 5], |a, ((t, c), s)| {
        [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + c.is_some() as i64, a[3] + s, a[4] + 1]
    });
    let ub = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&tu).select((&ps).opt().and((&ub).opt())));
    rows(v.into_iter().map(|(u, (p, b))| {
        let rep = db.user.reputation.get(u).unwrap();
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(match p {
            Some(a) => [V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3])],
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        f.extend([V::I(b.unwrap_or(0)), V::I(p.map_or(0, |a| a[4])), V::S(if rep >= 10000 { "Gold" } else if rep >= 1000 { "Silver" } else { "Bronze" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.OwnerUserId,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.CreationDate >= (cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year')),
// UserEngagement AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(distinct p.Id) AS QuestionsAsked, SUM(COALESCE(b.Class, 0)) AS TotalBadges, SUM(v.BountyAmount) AS TotalBounty
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId AND p.PostTypeId = 1 LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName),
// TopEngagedUsers AS (SELECT ue.UserId, ue.DisplayName, ue.QuestionsAsked, ue.TotalBadges, ue.TotalBounty, ROW_NUMBER() OVER (ORDER BY ue.QuestionsAsked DESC, ue.TotalBounty DESC) AS EngagementRank
//     FROM UserEngagement ue WHERE ue.QuestionsAsked > 0)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, ue.DisplayName AS OwnerDisplayName, ue.QuestionsAsked, ue.TotalBadges, ue.TotalBounty
// FROM RankedPosts rp JOIN TopEngagedUsers ue ON rp.OwnerUserId = ue.UserId WHERE ue.EngagementRank <= 10 ORDER BY rp.CreationDate DESC;
//
// EngagementRank leads with the distinct question count, so only users with at least the tenth-highest count can rank in the top ten;
// the questions x badges x votes product is driven for those alone. DESC puts a NULL TotalBounty last.
fn q26758(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let q = || posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1)));
    let qc = db.user.group_by(Ident::<User>::new()).select(q().opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let tenth = top_n(drain(&qc), |&(u, n)| (Reverse(n), u), 10).last().unwrap().1;
    let cand: MatSet<Id<User>> = db.user.with((&qc).filt(move |n| n >= tenth && n > 0)).collect();
    let ue = (&cand)
        .group_by(Ident::<User>::new())
        .select(q().opt().and(badges_of(db).select(&db.badge.class).opt()).and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold([0i64; 3], |a, ((_, c), b)| {
            let b = b.flatten();
            [a[0] + c.unwrap_or(0), a[1] + b.is_some() as i64, a[2] + b.unwrap_or(0)]
        });
    let top = top_n(drain((&ue).and(&qc)), |&(u, (a, n))| (Reverse(n), a[1] == 0, Reverse(a[2]), u), 10);
    let tu = rel(top);
    let by_user: HashIdx<Id<User>, (Id<User>, ([i64; 3], i64))> = (&tu).map(|(u, _)| u).inv().select(&tu).collect();
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(owner_user.select(&by_user)));
    rows(v.into_iter().map(|(p, (u, (a, n)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([user_col(db, u, "name"), V::I(n), V::I(a[0]), nullable(a[2], a[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostInteractions AS (SELECT p.Id AS PostId, COALESCE(COUNT(c.Id), 0) AS CommentCount, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (8, 9) GROUP BY p.Id)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, ub.BadgeCount AS UserBadgeCount, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges, pi.CommentCount, pi.TotalBounty
// FROM RankedPosts rp JOIN Users u ON rp.PostId = u.Id JOIN UserBadges ub ON u.Id = ub.UserId LEFT JOIN PostInteractions pi ON rp.PostId = pi.PostId
// WHERE rp.PostRank <= 5 ORDER BY rp.ViewCount DESC, rp.Score DESC;
//
// `rp.PostId = u.Id` joins a post id to a user id, so it goes through the raw ids. The ownerless posts rank as one partition; PostRank
// breaks a Score tie on the post id (the SQL leaves it open).
fn q3879(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, origid, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class)).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let hit: MatSet<Id<Post>> = (&tp).with(origid.select(&uidx).select(&ub)).collect();
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).in_v(vec![8, 9]))).select((&db.vote.bounty_amount).opt());
    let pi = (&hit).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(bounty.opt())).fold([0i64; 2], |a, (c, b)| [a[0] + c.is_some() as i64, a[1] + b.flatten().unwrap_or(0)]);
    let v = drain((&pi).and(origid.select(&uidx).select(&ub)));
    rows(v.into_iter().map(|(p, (a, b))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend(b.map(V::I));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.Tags, p.CreationDate, p.ViewCount, u.DisplayName AS OwnerDisplayName, u.Reputation AS OwnerReputation,
//        COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT a.Id) AS AnswerCount, MAX(ph.CreationDate) AS LastEditDate
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2
//     LEFT JOIN PostHistory ph ON p.Id = ph.PostId WHERE p.PostTypeId = 1 AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'
//     GROUP BY p.Id, p.Title, p.Body, p.Tags, p.CreationDate, p.ViewCount, u.DisplayName, u.Reputation),
// RankedPosts AS (SELECT rp.*, RANK() OVER (ORDER BY rp.ViewCount DESC) AS ViewRank, RANK() OVER (ORDER BY rp.AnswerCount DESC) AS AnswerRank FROM RecentPosts rp)
// SELECT rp.PostId, rp.Title, rp.Body, rp.Tags, rp.CreationDate, rp.OwnerDisplayName, rp.OwnerReputation, rp.CommentCount, rp.AnswerCount, rp.ViewCount, rp.LastEditDate,
//        CASE WHEN rp.ViewRank <= 10 THEN 'Top Viewed' WHEN rp.AnswerRank <= 10 THEN 'Top Answered' ELSE 'Moderate' END AS PostStatus
// FROM RankedPosts rp WHERE rp.CommentCount > 0 OR rp.AnswerCount > 0 ORDER BY rp.ViewRank, rp.AnswerRank;
//
// Each distinct count is its own fold over the post's children.
fn q28105(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, view_count, .. } = &db.post;
    let rp = || db.post.with(post_type_id.eq(1).and(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))).with(owner_user);
    let cc = rp().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ac = rp().group_by(Ident::<Post>::new()).select(answers_of(db).opt()).fold(0i64, |n, a| n + a.is_some() as i64);
    let le = rp().group_by(Ident::<Post>::new()).select(history_of(db).select(&db.post_history.creation_date).opt()).fold(i64::MIN, |m, d| d.map_or(m, |d| m.max(d)));
    let v = ranked(drain((&cc).and(&ac).and(&le)), |&(p, _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w))
    }, false);
    let v = ranked(v, |&((_, ((_, a), _)), _)| Reverse(a), false);
    let v = drain(rel(v).filt(|(((_, ((c, a), _)), _), _): (((Id<Post>, ((i64, i64), i64)), i64), i64)| c > 0 || a > 0));
    rows(v.into_iter().map(|(_, (((p, ((c, a), d)), vr), ar))| {
        let mut f = post_fields(db, p, &["id", "title", "body", "tags", "created", "owner", "rep"]);
        f.extend([V::I(c), V::I(a), post_fields(db, p, &["views"]).pop().unwrap(), tmax(d)]);
        f.push(V::S(if vr <= 10 { "Top Viewed" } else if ar <= 10 { "Top Answered" } else { "Moderate" }));
        row(f)
    }))
}

// WITH UserEngagement AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges, SUM(COALESCE(COM.Score, 0)) AS CommentScore
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId AND V.UserId = U.Id LEFT JOIN Badges B ON U.Id = B.UserId
//     LEFT JOIN Comments COM ON P.Id = COM.PostId WHERE U.Reputation > 1000 GROUP BY U.Id, U.DisplayName),
// UserRanking AS (SELECT UserId, DisplayName, PostCount, AnswerCount, UpVotes, DownVotes, GoldBadges, SilverBadges, BronzeBadges, CommentScore,
//        DENSE_RANK() OVER (ORDER BY PostCount DESC, UpVotes - DownVotes DESC) AS EngagementRank FROM UserEngagement)
// SELECT UserId, DisplayName, PostCount, AnswerCount, UpVotes, DownVotes, GoldBadges, SilverBadges, BronzeBadges, CommentScore, EngagementRank
// FROM UserRanking WHERE EngagementRank <= 10 ORDER BY EngagementRank;
//
// EngagementRank leads with the distinct post count, so only users whose count is among the ten highest distinct counts can reach dense
// rank 10; the posts x own votes x badges x comments product is driven for those alone.
fn q5772(db: &'static So) -> String {
    let hi = || db.user.with((&db.user.reputation).gt(1000));
    let pc = hi().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let mut counts: Vec<i64> = drain(&pc).into_iter().map(|x| x.1).collect();
    counts.sort_unstable_by(|a, b| b.cmp(a));
    counts.dedup();
    let tenth = counts[counts.len().min(10) - 1];
    let cand: MatSet<Id<User>> = db.user.with((&pc).filt(move |n| n >= tenth)).collect();
    let ov = own_votes(db);
    let s = (&cand)
        .group_by(Ident::<User>::new())
        .select(
            posts_of(db)
                .select((&db.post.post_type_id).and((&ov).select(&db.vote.vote_type_id).opt()).and(comments_of(db).select(&db.comment.score).opt()))
                .opt()
                .and(badges_of(db).select(&db.badge.class).opt()),
        )
        .fold([0i64; 7], |a, (p, c)| {
            let ((t, v), s) = p.map_or(((0, None), None), |x| x);
            [
                a[0] + (t == 2) as i64,
                a[1] + (v == Some(2)) as i64,
                a[2] + (v == Some(3)) as i64,
                a[3] + (c == Some(1)) as i64,
                a[4] + (c == Some(2)) as i64,
                a[5] + (c == Some(3)) as i64,
                a[6] + s.unwrap_or(0),
            ]
        });
    let v = ranked(drain((&s).and(&pc)), |&(_, (a, n))| (Reverse(n), Reverse(a[1] - a[2])), true);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, (a, n)), r)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.OwnerUserId, COUNT(c.Id) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS OwnerRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.OwnerUserId),
// UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.Views, u.UpVotes, u.DownVotes, COALESCE(b.Name, 'No Badge') AS BadgeName, COUNT(DISTINCT p.Id) AS PostsCount,
//        SUM(p.Score) AS TotalScore FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId
//     GROUP BY u.Id, u.DisplayName, u.Reputation, u.Views, u.UpVotes, u.DownVotes, b.Name)
// SELECT up.DisplayName, up.Reputation, up.Views, up.PostsCount, up.TotalScore, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.CommentCount
// FROM UserStatistics up JOIN RecentPosts rp ON up.UserId = rp.OwnerUserId
// WHERE up.Reputation > 1000 AND rp.CommentCount > 5
//   AND EXISTS (SELECT 1 FROM Votes v WHERE v.PostId = rp.PostId AND v.VoteTypeId = 2 AND v.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 week')
// ORDER BY up.Reputation DESC, rp.Score DESC LIMIT 50;
//
// UserStatistics has one group per (user, badge name), built over the badge rows and only for the owners the join keeps.
fn q1068(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let rc = db.post.with(creation_date.gt(add_days(t0, -30))).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let Vote { vote_type_id, creation_date: vd, .. } = &db.vote;
    let up = votes_of(db).select(Ident::<Vote>::new().with(vote_type_id.eq(2).and(vd.gt(add_days(t0, -7)))));
    let hi = Ident::<User>::new().with((&db.user.reputation).gt(1000));
    let rp: MatSet<Id<Post>> = db.post.with((&rc).filt(|n| n > 5)).with(up).with(owner_user.select(hi)).collect();
    let owners: MatSet<Id<User>> = (&rp).select(owner_user).collect();
    let j = rel(drain((&owners).select(badges_of(db).select(Ident::<Badge>::new().and(&db.badge.name)).opt())));
    type R = (Id<User>, Option<(Id<Badge>, Str)>);
    let us = (&j)
        .group_by(Same::<R>::new().map(|(u, b): R| (u, b.map(|x| x.1))))
        .select(Same::<R>::new().map(|(u, _): R| u).select(posts_of(db).select(score).opt()))
        .fold([0i64; 2], |a, s| [a[0] + s.is_some() as i64, a[1] + s.unwrap_or(0)]);
    let pcnt = (&owners).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let usv = rel(drain(&us));
    let by_user: HashIdx<Id<User>, ((Id<User>, Option<Str>), [i64; 2])> = (&usv).map(|((u, _), _)| u).inv().select(&usv).collect();
    let v = drain((&rp).select(owner_user.select((&by_user).and(&pcnt))).and(&rc));
    let v = top_n(v, |&(p, ((((u, b), _), _), _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(score.get(p).unwrap()), p, b), 50);
    rows(v.into_iter().map(|(p, ((((u, _), a), n), c))| {
        let mut f = ucols(db, u, &["name", "rep", "uviews"]);
        f.extend([V::I(n), nullable(a[1], a[0])]);
        f.extend(post_fields(db, p, &["title", "created", "views", "score"]));
        f.push(V::I(c));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId
//     GROUP BY U.Id, U.DisplayName, U.Reputation),
// RecentPosts AS (SELECT P.OwnerUserId, P.Id AS PostId, P.Title, P.CreationDate, P.ViewCount, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS RecentRank
//     FROM Posts P WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// TopPosts AS (SELECT RP.OwnerUserId, RP.PostId, RP.Title, RP.ViewCount, RANK() OVER (PARTITION BY RP.OwnerUserId ORDER BY RP.ViewCount DESC) AS RankByViews FROM RecentPosts RP)
// SELECT U.DisplayName, U.Reputation, UR.BadgeCount, UR.GoldBadges, UR.SilverBadges, UR.BronzeBadges, TP.Title, TP.ViewCount
// FROM UserReputation UR LEFT JOIN TopPosts TP ON UR.UserId = TP.OwnerUserId AND TP.RankByViews = 1 INNER JOIN Users U ON U.Id = UR.UserId
// WHERE UR.Reputation > (SELECT AVG(Reputation) FROM Users) OR TP.ViewCount IS NOT NULL ORDER BY U.Reputation DESC, TP.ViewCount DESC NULLS LAST LIMIT 100;
//
// RankByViews = 1 keeps every post tied at the owner's highest view count (NULL counts sort last and tie among themselves).
fn q4626(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let Post { creation_date, owner_user, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w))
    }, 1, true);
    let tp = rel(top.into_iter().map(|(p, u)| (u, p)).collect());
    let first: HashIdx<Id<User>, Id<Post>> = (&tp).map(|(u, _)| u).inv().select((&tp).map(|(_, p)| p)).collect();
    let (n, s) = (&db.user.reputation).fold_flat((0i64, 0i64), |(n, s), r| (n + 1, s + r));
    type R = (i64, ([i64; 4], Option<(Id<Post>, Option<i64>)>));
    let keep = (&db.user.reputation)
        .and((&ub).and((&first).select(Ident::<Post>::new().and(view_count.opt())).opt()))
        .filt(move |(r, (_, p)): R| r as i128 * n as i128 > s as i128 || p.map_or(false, |x| x.1.is_some()));
    let v = top_n(drain(keep), |&(u, (r, (_, p)))| {
        let w = p.and_then(|x| x.1);
        (Reverse(r), w.is_none(), Reverse(w), u, p.map(|x| x.0))
    }, 100);
    rows(v.into_iter().map(|(u, (_, (b, p)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(b.map(V::I));
        f.extend(match p {
            Some((p, w)) => [post_fields(db, p, &["title"]).pop().unwrap(), oint(w)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId IN (2, 6) THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, u.DisplayName, p.PostTypeId),
// TopRankedPosts AS (SELECT * FROM RankedPosts WHERE Rank <= 10),
// PostDetails AS (SELECT trp.PostId, trp.Title, trp.CreationDate, trp.Score, trp.OwnerDisplayName, trp.CommentCount, trp.UpVotes, trp.DownVotes, COALESCE(ht.Name, 'No History') AS PostHistoryType
//     FROM TopRankedPosts trp LEFT JOIN PostHistory ph ON trp.PostId = ph.PostId LEFT JOIN PostHistoryTypes ht ON ph.PostHistoryTypeId = ht.Id)
// SELECT pd.PostId, pd.Title, pd.CreationDate, pd.Score, pd.OwnerDisplayName, pd.CommentCount, pd.UpVotes, pd.DownVotes, pd.PostHistoryType FROM PostDetails pd ORDER BY pd.Score DESC, pd.CreationDate DESC;
//
// Rank reads only base columns, so the top posts are picked first and the comment x vote product is driven for those alone.
fn q8104(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + matches!(t, Some(2 | 6)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = drain((&s).and(history_of(db).select(htype_name(db)).opt()));
    rows(v.into_iter().map(|(p, (a, h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "owner"]);
        f.extend(a.map(V::I));
        f.push(V::S(h.unwrap_or("No History")));
        row(f)
    }))
}

// Rewritten (rewrites/1820.sql): TopUsers' ORDER BY is tie-broken on U.Id.
// WITH RecentPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.OwnerUserId, P.Score, P.ViewCount, P.AnswerCount, P.CommentCount, P.ClosedDate,
//        ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS rn FROM Posts P WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// UserScores AS (SELECT U.Id AS UserId, COALESCE(SUM(VB.BountyAmount), 0) AS TotalBounties, COUNT(V.Id) FILTER (WHERE V.VoteTypeId = 2) AS UpVotes,
//        COUNT(V.Id) FILTER (WHERE V.VoteTypeId = 3) AS DownVotes FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId LEFT JOIN Votes VB ON U.Id = VB.UserId AND VB.VoteTypeId = 8 GROUP BY U.Id),
// TopUsers AS (SELECT U.Id, U.DisplayName, US.TotalBounties, (US.UpVotes - US.DownVotes) AS NetVotes FROM Users U JOIN UserScores US ON U.Id = US.UserId
//     WHERE U.Reputation > 100 ORDER BY NetVotes DESC, U.Id LIMIT 10)
// SELECT RP.Title, RP.CreationDate, RP.ViewCount, T.DisplayName AS OwnerName, TU.TotalBounties, TU.NetVotes
// FROM RecentPosts RP LEFT JOIN Users T ON RP.OwnerUserId = T.Id LEFT JOIN TopUsers TU ON T.Id = TU.Id
// WHERE RP.ClosedDate IS NULL OR RP.ClosedDate < cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '7 days' ORDER BY RP.Score DESC, RP.ViewCount DESC;
fn q1820(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let vb = votes_by(db).select(Ident::<Vote>::new().with(vote_type_id.eq(8))).select(bounty_amount.opt());
    let us = db
        .user
        .with((&db.user.reputation).gt(100))
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(vote_type_id).opt().and(vb.opt()))
        .fold([0i64; 3], |a, (t, b)| [a[0] + b.flatten().unwrap_or(0), a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let top = top_n(drain(&us), |&(u, a)| (Reverse(a[1] - a[2]), db.user.origid.get(u).unwrap()), 10);
    let tv = rel(top);
    let tu: HashIdx<Id<User>, [i64; 3]> = (&tv).map(|(u, _)| u).inv().select((&tv).map(|(_, a)| a)).collect();
    let Post { creation_date, closed_date, owner_user, .. } = &db.post;
    let open = db.post.with(creation_date.ge(add_days(t0, -30))).minus(closed_date.ge(add_days(t0, -7)));
    let v = drain(open.select(owner_user.opt().and(owner_user.select(&tu).opt())));
    rows(v.into_iter().map(|(p, (u, a))| {
        let mut f = post_fields(db, p, &["title", "created", "views"]);
        f.push(u.map_or(V::Null, |u| user_col(db, u, "name")));
        f.extend(match a {
            Some(a) => [V::I(a[0]), V::I(a[1] - a[2])],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END) AS TotalUpvotedPosts,
//        SUM(CASE WHEN P.Score < 0 THEN 1 ELSE 0 END) AS TotalDownvotedPosts, SUM(CASE WHEN B.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalBadges
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUserPosts AS (SELECT PU.UserId, U.DisplayName, P.Title, P.Score, P.CreationDate, RANK() OVER (PARTITION BY PU.UserId ORDER BY P.Score DESC) AS PostRank
//     FROM Posts P JOIN UserStatistics U ON U.UserId = P.OwnerUserId JOIN (SELECT UserId FROM UserStatistics WHERE TotalPosts > 0) PU ON U.UserId = PU.UserId)
// SELECT U.DisplayName, U.Reputation, U.TotalPosts, U.TotalQuestions, U.TotalAnswers, U.TotalUpvotedPosts, U.TotalDownvotedPosts, U.TotalBadges, T.Title AS TopPostTitle,
//        T.Score AS TopPostScore, T.CreationDate AS TopPostDate
// FROM UserStatistics U LEFT JOIN TopUserPosts T ON U.UserId = T.UserId AND T.PostRank = 1 WHERE U.Reputation > 1000 ORDER BY U.TotalPosts DESC, U.Reputation DESC LIMIT 10;
//
// The ORDER BY reads only the distinct post count and Reputation, and each user yields at least one row, so the ten users are picked
// first and the posts x badges product is driven for those alone.
fn q5957(db: &'static So) -> String {
    let hi = || db.user.with((&db.user.reputation).gt(1000));
    let pc = hi().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let key = |u: Id<User>, n: i64| (Reverse(n), Reverse(db.user.reputation.get(u).unwrap()));
    let top = top_n(drain(&pc), |&(u, n)| (key(u, n), u), 10);
    let tu: MatSet<Id<User>> = rel(top.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let Post { post_type_id, score, .. } = &db.post;
    let s = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score)).opt().and(badges_of(db).opt()))
        .fold([0i64; 5], |a, (p, b)| {
            let (t, s) = p.map_or((0, 0), |x| x);
            [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (p.is_some() && s > 0) as i64, a[3] + (p.is_some() && s < 0) as i64, a[4] + b.is_some() as i64]
        });
    let best = top_per(drain((&tu).select(posts_of(db))), |&(u, _)| u, |&(_, p)| Reverse(score.get(p).unwrap()), 1, true);
    let bp = rel(best);
    let best_of: HashIdx<Id<User>, Id<Post>> = (&bp).map(|(u, _)| u).inv().select((&bp).map(|(_, p)| p)).collect();
    let v = drain((&s).and(&pc).and((&best_of).opt()));
    let v = top_n(v, |&(u, ((_, n), p))| (key(u, n), u, p), 10);
    rows(v.into_iter().map(|(u, ((a, n), p))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.extend(match p {
            Some(p) => post_fields(db, p, &["title", "score", "created"]),
            None => vec![V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// Rewritten (rewrites/253.sql): UserPostRank is tie-broken on P.Id.
// WITH PostStatistics AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, COALESCE((SELECT COUNT(*) FROM Comments C WHERE C.PostId = P.Id), 0) AS CommentCount,
//        COALESCE((SELECT COUNT(*) FROM Votes V WHERE V.PostId = P.Id AND V.VoteTypeId = 2), 0) AS Upvotes, COALESCE((SELECT COUNT(*) FROM Votes V WHERE V.PostId = P.Id AND V.VoteTypeId = 3), 0) AS Downvotes,
//        DENSE_RANK() OVER (ORDER BY P.Score DESC) AS ScoreRank, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC, P.Id) AS UserPostRank, P.OwnerUserId
//     FROM Posts P WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserBadges AS (SELECT U.Id AS UserId, COUNT(B.Id) AS TotalBadges, MAX(B.Class) AS HighestBadgeClass FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id)
// SELECT PS.PostId, PS.Title, PS.CreationDate, PS.Score, PS.CommentCount, PS.Upvotes, PS.Downvotes, PS.ScoreRank, UB.TotalBadges, UB.HighestBadgeClass,
//        CASE WHEN PS.Score > 10 THEN 'High' WHEN PS.Score BETWEEN 5 AND 10 THEN 'Medium' ELSE 'Low' END AS ScoreCategory,
//        CASE WHEN PS.UserPostRank = 1 THEN 'Most Recent Post' ELSE NULL END AS RecentPostIndicator
// FROM PostStatistics PS LEFT JOIN UserBadges UB ON PS.OwnerUserId = UB.UserId ORDER BY PS.Score DESC, PS.CreationDate DESC LIMIT 100;
//
// ScoreRank and UserPostRank read only base columns, so they are taken over all recent posts and the hundred posts are picked before any
// count is driven. The ownerless posts rank as one partition.
fn q253(db: &'static So) -> String {
    let Post { creation_date, score, owner_user, origid, .. } = &db.post;
    let recent = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user.opt()));
    let latest = top_per(recent.clone(), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), origid.get(p).unwrap()), 1, false);
    let latest: MatSet<Id<Post>> = rel(latest.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let r = ranked(recent, |&(p, _)| Reverse(score.get(p).unwrap()), true);
    let top = top_n(r, |&((p, _), _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 100);
    let tr = rel(top.into_iter().map(|((p, _), r)| (p, r)).collect());
    let rank: HashIdx<Id<Post>, i64> = (&tr).map(|(p, _)| p).inv().select((&tr).map(|(_, r)| r)).collect();
    let cc = db.post.with(&rank).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = db.post.with(&rank).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold((0i64, i64::MIN), |(n, m), c| match c {
        Some(c) => (n + 1, m.max(c)),
        None => (n, m),
    });
    let v = drain((&rank).and(&cc).and(&vc).and(owner_user.select(&ub).opt()).and(Ident::<Post>::new().with(&latest).opt()));
    rows(v.into_iter().map(|(p, ((((r, c), a), b), l))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1]), V::I(r)]);
        f.extend(match b {
            Some((n, m)) => [V::I(n), if n == 0 { V::Null } else { V::I(m) }],
            None => [V::Null, V::Null],
        });
        f.push(V::S(if s > 10 { "High" } else if (5..=10).contains(&s) { "Medium" } else { "Low" }));
        f.push(if l.is_some() { V::S("Most Recent Post") } else { V::Null });
        row(f)
    }))
}

// WITH UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT b.Id) AS BadgeCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
//     LEFT JOIN Comments c ON u.Id = c.UserId AND c.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
//     LEFT JOIN Badges b ON u.Id = b.UserId AND b.Date >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY u.Id, u.Reputation),
// TopUsers AS (SELECT UserId, Reputation, PostCount, CommentCount, BadgeCount, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserReputation)
// SELECT u.DisplayName, u.Reputation, pu.PostCount, pu.CommentCount, pu.BadgeCount, pt.Name AS PostTypeName, COUNT(DISTINCT p.Id) AS TotalPosts,
//        SUM(CASE WHEN pt.Id = 1 THEN 1 ELSE 0 END) AS Questions, SUM(CASE WHEN pt.Id = 2 THEN 1 ELSE 0 END) AS Answers, SUM(CASE WHEN pt.Id = 3 THEN 1 ELSE 0 END) AS Wikis
// FROM Users u JOIN TopUsers pu ON u.Id = pu.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN PostTypes pt ON p.PostTypeId = pt.Id WHERE pu.ReputationRank <= 10
// GROUP BY u.DisplayName, u.Reputation, pu.PostCount, pu.CommentCount, pu.BadgeCount, pt.Name, pu.ReputationRank ORDER BY pu.ReputationRank;
//
// ReputationRank reads only Reputation, so the ranked users are picked first; each distinct count is its own fold. The final groups are
// built over the (user, post) rows, keyed by the grouped columns.
fn q7319(db: &'static So) -> String {
    let t = add_years(ts(2024, 10, 1, 12, 34, 56), -1);
    let r = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let r = rel(r.into_iter().take_while(|x| x.1 <= 10).map(|((u, _), r)| (u, r)).collect());
    let rank: HashIdx<Id<User>, i64> = (&r).map(|(u, _)| u).inv().select((&r).map(|(_, r)| r)).collect();
    let tu = || db.user.with(&rank);
    let recent = |d: &'static Col<Post, i64>| Ident::<Post>::new().with(d.ge(t));
    let pc = tu().group_by(Ident::<User>::new()).select(posts_of(db).select(recent(&db.post.creation_date)).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let cc = tu().group_by(Ident::<User>::new()).select(comments_by(db).select(Ident::<Comment>::new().with((&db.comment.creation_date).ge(t))).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let bc = tu().group_by(Ident::<User>::new()).select(badges_of(db).select(Ident::<Badge>::new().with((&db.badge.date).ge(t))).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let Post { post_type, post_type_id, .. } = &db.post;
    let j = rel(drain(tu().select(
        (&db.user.display_name)
            .and(&db.user.reputation)
            .and(&pc)
            .and(&cc)
            .and(&bc)
            .and(&rank)
            .and(posts_of(db).select(Ident::<Post>::new().and(post_type.select(&db.post_type.name)).and(post_type_id)).opt()),
    )));
    type K = (((((Str, i64), i64), i64), i64), i64);
    type R = (Id<User>, (K, Option<((Id<Post>, Str), i64)>));
    let g = (&j)
        .group_by(Same::<R>::new().map(|(_, (k, p)): R| (k, p.map(|x| x.0 .1))))
        .select(Same::<R>::new().map(|(_, (_, p)): R| p))
        .fold([0i64; 4], |a, p| match p {
            Some((_, t)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (t == 3) as i64],
            None => a,
        });
    rows(drain(&g).into_iter().map(|((k, n), a)| {
        let (((((name, rep), p), c), b), _) = k;
        row(vec![V::S(name), V::I(rep), V::I(p), V::I(c), V::I(b), ostr(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3])])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.FavoriteCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId IN (1, 2) AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, rp.CommentCount, rp.FavoriteCount, rp.OwnerDisplayName FROM RankedPosts rp WHERE rp.Rank <= 5)
// SELECT tp.Title, tp.OwnerDisplayName, tp.CreationDate, tp.Score, tp.ViewCount, COALESCE(c.CommentCount, 0) AS TotalComments, COALESCE(v.UpVotes, 0) AS TotalUpVotes,
//        COALESCE(v.DownVotes, 0) AS TotalDownVotes
// FROM TopPosts tp LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON tp.PostId = c.PostId
// LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes GROUP BY PostId) v
//     ON tp.PostId = v.PostId ORDER BY tp.Score DESC, tp.CreationDate DESC;
fn q7501(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.is_in([1, 2]).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    rows(drain((&cc).and(&vc)).into_iter().map(|(p, (c, a))| {
        let mut f = post_fields(db, p, &["title", "owner", "created", "score", "views"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH PostTags AS (SELECT p.Id AS PostId, unnest(string_to_array(substring(p.Tags, 2, length(p.Tags)-2), '><')) AS Tag FROM Posts p WHERE p.PostTypeId = 1),
// TagMetrics AS (SELECT pt.Tag, COUNT(*) AS TagCount, COUNT(DISTINCT pt.PostId) AS PostCount, AVG(u.Reputation) AS AvgUserReputation
//     FROM PostTags pt JOIN Posts p ON pt.PostId = p.Id JOIN Users u ON p.OwnerUserId = u.Id GROUP BY pt.Tag),
// PopularTags AS (SELECT Tag, TagCount, PostCount, AvgUserReputation FROM TagMetrics WHERE TagCount > (SELECT AVG(TagCount) FROM TagMetrics)),
// TopQuestions AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.CreationDate, ARRAY_AGG(DISTINCT pt.Tag) AS Tags
//     FROM Posts p JOIN PostTags pt ON p.Id = pt.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.CreationDate
//     ORDER BY p.Score DESC LIMIT 10)
// SELECT tq.PostId, tq.Title, tq.Score, tq.ViewCount, tq.AnswerCount, tq.CommentCount, tq.CreationDate, pm.Tag AS PopularTag, pm.TagCount, pm.PostCount, pm.AvgUserReputation
// FROM TopQuestions tq JOIN PopularTags pm ON pm.Tag = ANY(tq.Tags) ORDER BY tq.Score DESC, pm.TagCount DESC;
//
// `pm.Tag = ANY(tq.Tags)` joins each question to the popular tags among its own.
fn q27313(db: &'static So) -> String {
    let Post { post_type_id, tags_str, owner_user, score, .. } = &db.post;
    let pt = || db.post.with(post_type_id.eq(1)).select(Ident::<Post>::new().and(tags_str.flat_map(tag_list)));
    type R = (Id<Post>, Str);
    let tm = pt()
        .with(Same::<R>::new().map(|(p, _): R| p).select(owner_user))
        .group_by(Same::<R>::new().map(|(_, t): R| t))
        .select(Same::<R>::new().map(|(p, _): R| p).select(owner_user.select(&db.user.reputation)))
        .fold([0i64; 2], |a, r| [a[0] + 1, a[1] + r]);
    let dp = pt().with(Same::<R>::new().map(|(p, _): R| p).select(owner_user)).group_by(Same::<R>::new().map(|(_, t): R| t)).select(Same::<R>::new().map(|(p, _): R| p)).count_distinct();
    let (n, s) = (&tm).fold_flat((0i64, 0i64), |(n, s), a| (n + 1, s + a[0]));
    let popular = (&tm).filt(move |a: [i64; 2]| a[0] as i128 * n as i128 > s as i128).and(&dp);
    let top = top_n(drain(db.post.with(post_type_id.eq(1)).with(tags_str.flat_map(tag_list)).select(score)), |&(p, s)| (Reverse(s), p), 10);
    let tq: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let v = drain((&tq).select(Ident::<Post>::new().and(tags_str.flat_map(tag_list).select(Same::<Str>::new().and(popular)))));
    rows(v.into_iter().map(|(_, (p, (t, (a, d))))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "answers", "comments", "created"]);
        f.extend([V::S(t), V::I(a[0]), V::I(d), avg(a[1], a[0])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS OwnerDisplayName, p.Score, p.ViewCount, p.AnswerCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT rp.* FROM RankedPosts rp WHERE rp.rn <= 10),
// PostComments AS (SELECT c.PostId, c.UserDisplayName, COUNT(c.Id) AS CommentCount, SUM(c.Score) AS TotalScore FROM Comments c GROUP BY c.PostId, c.UserDisplayName),
// PostsWithComments AS (SELECT tp.PostId, tp.Title, tp.CreationDate, tp.OwnerDisplayName, tp.Score, tp.ViewCount, tp.AnswerCount, COALESCE(pc.CommentCount, 0) AS CommentCount,
//        COALESCE(pc.TotalScore, 0) AS TotalScore FROM TopPosts tp LEFT JOIN PostComments pc ON tp.PostId = pc.PostId)
// SELECT p.Title, p.CreationDate, p.OwnerDisplayName, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.TotalScore,
//        CASE WHEN p.Score > 10 THEN 'High Score' WHEN p.Score BETWEEN 5 AND 10 THEN 'Medium Score' ELSE 'Low Score' END AS ScoreCategory
// FROM PostsWithComments p ORDER BY p.ViewCount DESC, p.Score DESC;
//
// PostComments has one group per (post, commenter name), NULL name included; each top post joins all of its groups.
fn q5254(db: &'static So) -> String {
    let Post { owner_user, creation_date, post_type_id, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let Comment { post, user_display_name, score: cs, .. } = &db.comment;
    let pc = db.comment.with(post.select(&tp)).group_by(post.and(user_display_name.opt())).select(cs).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let pv = rel(drain(&pc));
    let by_post: HashIdx<Id<Post>, [i64; 2]> = (&pv).map(|((p, _), _)| p).inv().select((&pv).map(|(_, a)| a)).collect();
    let v = drain((&tp).select((&by_post).opt()));
    rows(v.into_iter().map(|(p, a)| {
        let s = score.get(p).unwrap();
        let a = a.unwrap_or([0; 2]);
        let mut f = post_fields(db, p, &["title", "created", "owner", "score", "views", "answers"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::S(if s > 10 { "High Score" } else if (5..=10).contains(&s) { "Medium Score" } else { "Low Score" })]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS TotalPosts, COALESCE(SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalAnswers,
//        COALESCE(SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END), 0) AS TotalQuestions, COALESCE(SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END), 0) AS UpvotedPosts,
//        COALESCE(SUM(CASE WHEN P.Score < 0 THEN 1 ELSE 0 END), 0) AS DownvotedPosts, COALESCE(SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges,
//        COALESCE(SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadges, COALESCE(SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadges
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Badges B ON U.Id = B.UserId WHERE U.Reputation > 100 GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, TotalAnswers, TotalQuestions, UpvotedPosts, DownvotedPosts, GoldBadges, SilverBadges, BronzeBadges,
//        RANK() OVER (ORDER BY Reputation DESC) AS Rank FROM UserStats)
// SELECT Rank, DisplayName, Reputation, TotalPosts, TotalAnswers, TotalQuestions, UpvotedPosts, DownvotedPosts, GoldBadges, SilverBadges, BronzeBadges FROM TopUsers WHERE Rank <= 10 ORDER BY Rank;
//
// Rank reads only Reputation, so the ranked users are picked first and the posts x badges product is driven for those alone.
fn q6327(db: &'static So) -> String {
    let r = ranked(drain(db.user.with((&db.user.reputation).gt(100)).select(&db.user.reputation)), |&(_, r)| Reverse(r), false);
    let r = rel(r.into_iter().take_while(|x| x.1 <= 10).map(|((u, _), r)| (u, r)).collect());
    let rank: HashIdx<Id<User>, i64> = (&r).map(|(u, _)| u).inv().select((&r).map(|(_, r)| r)).collect();
    let tu = || db.user.with(&rank);
    let Post { post_type_id, score, .. } = &db.post;
    let s = tu()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score)).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 7], |a, (p, c)| {
            let (t, s) = p.map_or((0, 0), |x| x);
            [a[0] + (t == 2) as i64, a[1] + (t == 1) as i64, a[2] + (s > 0) as i64, a[3] + (s < 0) as i64, a[4] + (c == Some(1)) as i64, a[5] + (c == Some(2)) as i64, a[6] + (c == Some(3)) as i64]
        });
    let pc = tu().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    rows(drain((&rank).and(&pc).and(&s)).into_iter().map(|(u, ((r, n), a))| {
        let mut f = vec![V::I(r)];
        f.extend(ucols(db, u, &["name", "rep"]));
        f.push(V::I(n));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.AnswerCount, COALESCE(u.DisplayName, 'Community User') AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS rn, DATE_TRUNC('day', p.CreationDate) AS PostDate
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.PostTypeId = 1),
// TopPosts AS (SELECT PostId, Title, Score, ViewCount, AnswerCount, OwnerDisplayName, PostDate, rn FROM RankedPosts WHERE rn = 1),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsAsked, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersGiven,
//        AVG(p.Score) AS AvgScore, MAX(p.ViewCount) AS MaxViewCount FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName)
// SELECT tp.PostId, tp.Title, tp.Score, tp.ViewCount, tp.AnswerCount, tp.OwnerDisplayName, us.DisplayName AS UserDisplayName, us.QuestionsAsked, us.AnswersGiven, us.AvgScore, us.MaxViewCount
// FROM TopPosts tp JOIN UserStats us ON tp.OwnerDisplayName = us.DisplayName ORDER BY tp.Score DESC, tp.ViewCount DESC LIMIT 50;
//
// The ownerless questions rank as one partition, named 'Community User'. rn breaks a Score tie on the post id (the SQL leaves it open).
fn q7045(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(owner_user.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 1, false);
    let tp = rel(top.into_iter().map(|(p, u)| (p, u.map_or("Community User", |u| db.user.display_name.get(u).unwrap()))).collect());
    let us = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(score).and(view_count.opt())).opt()).fold([0, 0, 0, 0, i64::MIN], |a, p| match p {
        Some(((t, s), w)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, w.map_or(a[4], |w| a[4].max(w))],
        None => a,
    });
    let by_name: HashIdx<Str, Id<User>> = (&db.user.display_name).inv().collect();
    type R = (Id<Post>, Str);
    let v = drain((&tp).select(Same::<R>::new().and(Same::<R>::new().map(|(_, n): R| n).select((&by_name).select(Ident::<User>::new().and(&us))))));
    let v = top_n(v, |&(_, ((p, _), (u, _)))| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p, u)
    }, 50);
    rows(v.into_iter().map(|(_, ((p, n), (u, a)))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "answers"]);
        f.extend([V::S(n), user_col(db, u, "name")]);
        f.extend([V::I(a[1]), V::I(a[2]), avg(a[3], a[0]), if a[4] == i64::MIN { V::Null } else { V::I(a[4]) }]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.Score, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate > cast('2024-10-01' as date) - INTERVAL '1 year'),
// UserScores AS (SELECT u.Id AS UserId, u.Reputation, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 WHEN v.VoteTypeId = 3 THEN -1 ELSE 0 END) AS UserVoteScore
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.Reputation),
// PostDetails AS (SELECT rp.PostId, rp.Title, rp.ViewCount, rp.Score, us.UserVoteScore FROM RankedPosts rp LEFT JOIN Posts re ON rp.PostId = re.AcceptedAnswerId
//     LEFT JOIN UserScores us ON re.OwnerUserId = us.UserId WHERE rp.Rank <= 10),
// AggregatePosts AS (SELECT COUNT(*) AS TotalPosts, AVG(ViewCount) AS AvgViewCount, SUM(Score) AS TotalScore, SUM(UserVoteScore) AS TotalUserVoteScore FROM PostDetails)
// SELECT p.Title, COALESCE(p.ViewCount, 0) AS ViewCount, COALESCE(p.Score, 0) AS Score, ap.TotalPosts, ap.AvgViewCount, ap.TotalScore, ap.TotalUserVoteScore
// FROM PostDetails p CROSS JOIN AggregatePosts ap WHERE p.ViewCount > ap.AvgViewCount ORDER BY p.Score DESC, p.ViewCount DESC;
//
// AggregatePosts is one row, folded from PostDetails and crossed back; the WHERE compares ViewCount to the exact mean.
fn q33700(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, owner_user, accepted_answer, .. } = &db.post;
    let v = drain(db.post.with(creation_date.gt(add_years(date(2024, 10, 1), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let accepted_by: HashIdx<Id<Post>, Id<Post>> = accepted_answer.inv().collect();
    let us = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold(0i64, |s, t| s + match t {
        Some(2) => 1,
        Some(3) => -1,
        _ => 0,
    });
    let pd = rel(drain((&tp).select(view_count.opt().and(score).and((&accepted_by).select(owner_user.select(&us).opt()).opt()))));
    type R = (Id<Post>, ((Option<i64>, i64), Option<Option<i64>>));
    let ap = whole(&pd).select(&pd).fold([0i64; 6], |a, (_, ((w, s), u)): R| {
        let u = u.flatten();
        [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s, a[4] + u.is_some() as i64, a[5] + u.unwrap_or(0)]
    });
    let v = drain((&pd).cross(&ap).filt(|((_, ((w, _), _)), a): (R, [i64; 6])| w.map_or(false, |w| a[1] > 0 && w as i128 * a[1] as i128 > a[2] as i128)));
    rows(v.into_iter().map(|(_, ((p, ((w, s), _)), a))| {
        row(vec![post_fields(db, p, &["title"]).pop().unwrap(), V::I(w.unwrap_or(0)), V::I(s), V::I(a[0]), avg(a[2], a[1]), V::I(a[3]), nullable(a[5], a[4])])
    }))
}

// WITH RECURSIVE UserBadges AS (SELECT U.Id AS UserId, U.DisplayName, B.Class, B.Name, B.Date, ROW_NUMBER() OVER (PARTITION BY U.Id ORDER BY B.Date DESC) AS BadgeRank
//     FROM Users U JOIN Badges B ON U.Id = B.UserId),
// PostStats AS (SELECT P.OwnerUserId, COUNT(*) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
//        AVG(P.Score) AS AvgScore, AVG(P.ViewCount) AS AvgViews FROM Posts P GROUP BY P.OwnerUserId),
// PopularTags AS (SELECT T.TagName, T.Count, ROW_NUMBER() OVER (ORDER BY T.Count DESC) AS TagRank FROM Tags T WHERE T.Count > 100)
// SELECT U.Id AS UserId, U.DisplayName, COALESCE(B.BadgeCount, 0) AS TotalBadges, PS.TotalPosts, PS.Questions, PS.Answers, PS.AvgScore, PS.AvgViews,
//        PTag.TagName AS MostPopularTag, PTag.Count AS PopularityCount
// FROM Users U LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM UserBadges WHERE BadgeRank = 1 GROUP BY UserId) B ON U.Id = B.UserId
// LEFT JOIN PostStats PS ON U.Id = PS.OwnerUserId LEFT JOIN (SELECT TagName, Count FROM PopularTags WHERE TagRank = 1) PTag ON TRUE
// WHERE U.Reputation > 100 AND U.LastAccessDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' ORDER BY U.Reputation DESC, TotalPosts DESC;
//
// WITH RECURSIVE, but no CTE refers to itself. BadgeRank = 1 is one badge per user, so B counts the users with a badge; the ROW_NUMBER picks
// the newest (a Date tie broken on the badge id, the SQL leaves it open). PTag is one row, crossed onto every user.
fn q34387(db: &'static So) -> String {
    let Badge { user, date, .. } = &db.badge;
    let newest = top_per(drain(db.badge.select(user)), |&(_, u)| u, |&(b, _)| (Reverse(date.get(b).unwrap()), b), 1, false);
    let nb = rel(newest);
    type B = (Id<Badge>, Id<User>);
    let bc = (&nb).group_by(Same::<B>::new().map(|(_, u): B| u)).select(Same::<B>::new()).fold(0i64, |n, _| n + 1);
    let Post { owner_user, post_type_id, score, view_count, .. } = &db.post;
    let ps = db.post.group_by(owner_user).select(post_type_id.and(score).and(view_count.opt())).fold([0i64; 6], |a, ((t, s), w)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0)]
    });
    let tag = top_n(drain(db.tag.with((&db.tag.count).gt(100)).select(&db.tag.count)), |&(t, c)| (Reverse(c), t), 1);
    let ptag = left_all(tag);
    let users = db
        .user
        .with((&db.user.reputation).gt(100).and((&db.user.last_access_date).gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))))
        .select((&bc).opt().and((&ps).opt()));
    let v = drain(users.cross(&ptag));
    rows(v.into_iter().map(|((u, _), ((b, p), t))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.push(V::I(b.unwrap_or(0)));
        f.extend(match p {
            Some(a) => [V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[3], a[0]), avg(a[5], a[4])],
            None => [V::Null, V::Null, V::Null, V::Null, V::Null],
        });
        f.extend(match t {
            Some((t, c)) => [V::S(db.tag.tag_name.get(t).unwrap()), V::I(c)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserVoteSummary AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVotes,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN P.Score ELSE 0 END) AS TotalQuestionScore FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId LEFT JOIN Posts P ON V.PostId = P.Id
//     GROUP BY U.Id, U.DisplayName),
// ClosedPostDetails AS (SELECT P.Id AS PostId, P.Title, PH.CreationDate AS ClosedDate, (SELECT COUNT(*) FROM Comments C WHERE C.PostId = P.Id) AS TotalComments,
//        (SELECT COUNT(*) FROM Votes V WHERE V.PostId = P.Id AND V.VoteTypeId IN (6, 12)) AS CloseVotes FROM Posts P JOIN PostHistory PH ON P.Id = PH.PostId WHERE PH.PostHistoryTypeId = 10),
// RankedPosts AS (SELECT PD.PostId, PD.Title, PD.ClosedDate, PD.TotalComments, PD.CloseVotes, RANK() OVER (ORDER BY PD.CloseVotes DESC, PD.TotalComments DESC) AS PostRank FROM ClosedPostDetails PD)
// SELECT U.DisplayName, U.UpVotes, U.DownVotes, U.TotalQuestionScore, RP.Title, RP.ClosedDate, RP.TotalComments, RP.CloseVotes, RP.PostRank
// FROM UserVoteSummary U FULL OUTER JOIN RankedPosts RP ON U.UserId = RP.PostId
// WHERE U.UserId IS NOT NULL AND (UPPER(U.DisplayName) LIKE '%DAVID%' OR RP.TotalComments > 10) ORDER BY COALESCE(RP.CloseVotes, 0) DESC, U.UpVotes DESC;
//
// `U.UserId IS NOT NULL` drops the rows only RankedPosts has, so the FULL OUTER JOIN is a LEFT JOIN from users. `U.UserId = RP.PostId`
// joins a user id to a post id, so it goes through the raw ids.
fn q1077(db: &'static So) -> String {
    let Vote { vote_type_id, post, .. } = &db.vote;
    let Post { post_type_id, score, origid, .. } = &db.post;
    let uvs = db
        .user
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(vote_type_id.and(post.select(post_type_id.and(score)).opt())).opt())
        .fold([0i64; 3], |a, v| match v {
            Some((t, p)) => [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + p.map_or(0, |(t, s)| if t == 1 { s } else { 0 })],
            None => a,
        });
    let PostHistory { post: hp, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let closed: MatSet<Id<Post>> = db.post_history.with(post_history_type_id.eq(10)).select(hp).collect();
    let cv = votes_of(db).select(Ident::<Vote>::new().with(vote_type_id.is_in([6, 12])));
    let pc = (&closed).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let pv = (&closed).group_by(Ident::<Post>::new()).select(cv.opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let cpd = drain(db.post_history.with(post_history_type_id.eq(10)).select(hp.select((&pc).and(&pv))));
    let r = ranked(cpd, |&(_, (c, v))| (Reverse(v), Reverse(c)), false);
    let rp = rel(r.into_iter().map(|((h, (c, v)), r)| (h, (c, v, r))).collect());
    type P = (Id<PostHistory>, (i64, i64, i64));
    let by_post: HashIdx<i64, P> = (&rp).map(|(h, _): P| h).select(hp).select(origid).inv().select(&rp).collect();
    type R = ((Str, [i64; 3]), Option<P>);
    let v = drain(
        db.user
            .select((&db.user.display_name).and(&uvs).and((&db.user.origid).select(&by_post).opt()))
            .filt(|((n, _), x): R| n.to_uppercase().contains("DAVID") || x.map_or(false, |(_, (c, _, _))| c > 10)),
    );
    rows(v.into_iter().map(|(_, ((n, a), x))| {
        let mut f = vec![V::S(n), V::I(a[0]), V::I(a[1]), V::I(a[2])];
        f.extend(match x {
            Some((h, (c, cv, r))) => [post_fields(db, hp.get(h).unwrap(), &["title"]).pop().unwrap(), V::T(hd.get(h).unwrap()), V::I(c), V::I(cv), V::I(r)],
            None => [V::Null, V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount, COUNT(v.Id) AS VoteCount, ROW_NUMBER() OVER (ORDER BY p.Score DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.CommentCount, rp.VoteCount FROM RankedPosts rp WHERE rp.Rank <= 10),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostsCreated, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersCount,
//        SUM(CASE WHEN b.Id IS NOT NULL THEN 1 ELSE 0 END) AS BadgesEarned FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.CommentCount, tp.VoteCount, ua.UserId, ua.DisplayName, ua.PostsCreated, ua.AnswersCount, ua.BadgesEarned
// FROM TopPosts tp JOIN Users u ON tp.PostId = u.Id JOIN UserActivity ua ON u.Id = ua.UserId ORDER BY tp.Score DESC, tp.CreationDate DESC;
//
// Rank reads only Score, so the ten questions are picked first (a Score tie broken on the post id; the SQL leaves it open) and each product
// is driven for those alone. `tp.PostId = u.Id` joins a post id to a user id, so it goes through the raw ids.
fn q7313(db: &'static So) -> String {
    let Post { post_type_id, score, origid, .. } = &db.post;
    let top = top_n(drain(db.post.with(post_type_id.eq(1)).select(score)), |&(p, s)| (Reverse(s), p), 10);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold([0i64; 2], |a, (c, v)| [a[0] + c.is_some() as i64, a[1] + v.is_some() as i64]);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let tu: MatSet<Id<User>> = (&tp).select(origid.select(&uidx)).collect();
    let ua = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id).opt().and(badges_of(db).opt()))
        .fold([0i64; 2], |a, (t, b)| [a[0] + (t == Some(2)) as i64, a[1] + b.is_some() as i64]);
    let dp = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let v = drain((&pc).and(origid.select(&uidx).select(Ident::<User>::new().and(&dp).and(&ua))));
    rows(v.into_iter().map(|(p, (c, ((u, n), a)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(c[0]), V::I(c[1])]);
        f.extend(ucols(db, u, &["uid", "name"]));
        f.extend([V::I(n), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN b.Id IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, BadgeCount, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserStats),
// ActiveTags AS (SELECT t.TagName, COUNT(DISTINCT p.Id) AS ActivePostCount FROM Tags t JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' GROUP BY t.TagName HAVING COUNT(DISTINCT p.Id) > 10),
// TopTags AS (SELECT TagName, ActivePostCount, RANK() OVER (ORDER BY ActivePostCount DESC) AS TagRank FROM ActiveTags)
// SELECT tu.DisplayName AS TopUser, tu.Reputation, tu.PostCount, tu.QuestionCount, tu.AnswerCount, tu.BadgeCount, tt.TagName AS PopularTag, tt.ActivePostCount
// FROM TopUsers tu JOIN TopTags tt ON tu.ReputationRank < 11 AND tt.TagRank < 11 ORDER BY tu.Reputation DESC, tt.ActivePostCount DESC;
//
// The ON reads only the two ranks, so the top users are crossed with the top tags. ReputationRank reads only Reputation, so the users are
// picked first and the posts x badges product is driven for those alone.
fn q8031(db: &'static So) -> String {
    let r = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let tu: MatSet<Id<User>> = rel(r.into_iter().take_while(|x| x.1 < 11).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let us = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(&db.post.post_type_id).opt().and(badges_of(db).opt()))
        .fold([0i64; 3], |a, (t, b)| [a[0] + (t == Some(1)) as i64, a[1] + (t == Some(2)) as i64, a[2] + b.is_some() as i64]);
    let dp = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let lt = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, Id<Post>> = (&lt).map(|(_, t)| t).inv().select((&lt).map(|(p, _)| p)).collect();
    let at = db.tag.group_by(&db.tag.tag_name).select(&by_tag).count_distinct();
    let tr = ranked(drain((&at).filt(|n| n > 10)), |&(_, n)| Reverse(n), false);
    let tt = rel(tr.into_iter().take_while(|x| x.1 < 11).map(|x| x.0).collect());
    let v = drain((&dp).and(&us).cross(&tt));
    rows(v.into_iter().map(|((u, _), ((n, a), (t, c)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::S(t), V::I(c)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, COALESCE((SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 2), 0) AS upvotes,
//        COALESCE((SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 3), 0) AS downvotes, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate ASC) AS rn
//     FROM Posts p WHERE p.PostTypeId IN (1, 2)),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.upvotes, rp.downvotes FROM RankedPosts rp WHERE rp.rn <= 5),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostsCount, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes
//     FROM Users u LEFT JOIN Posts p ON p.OwnerUserId = u.Id LEFT JOIN Votes v ON v.UserId = u.Id GROUP BY u.Id, u.DisplayName)
// SELECT up.UserId, up.DisplayName, up.PostsCount, up.TotalViews, up.TotalUpvotes, up.TotalDownvotes, tp.PostId, tp.Title, tp.CreationDate, tp.ViewCount, tp.Score
// FROM UserStats up JOIN TopPosts tp ON up.PostsCount > 0 ORDER BY up.TotalViews DESC, tp.Score DESC;
//
// The ON reads only up, so every user with a post is crossed with the top posts.
fn q8873(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.is_in([1, 2])).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), creation_date.get(p).unwrap(), p), 5, false);
    let tp = rel(top.into_iter().map(|x| x.0).collect());
    let posters = || db.user.with(posts_of(db));
    let us = posters()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(view_count.opt()).and(votes_by(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (w, t)| [a[0] + w.unwrap_or(0), a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let pc = posters().group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let v = drain((&pc).and(&us).cross(&tp));
    rows(v.into_iter().map(|((u, _), ((n, a), p))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        f.extend(post_fields(db, p, &["id", "title", "created", "views", "score"]));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '1 year'),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, AnswerCount, CommentCount, OwnerDisplayName FROM RankedPosts WHERE Rank <= 10),
// PostStats AS (SELECT tp.Title, tp.ViewCount, tp.AnswerCount, tp.CommentCount, COUNT(c.Id) AS TotalComments, SUM(v.BountyAmount) AS TotalBounties
//     FROM TopPosts tp LEFT JOIN Comments c ON tp.PostId = c.PostId LEFT JOIN Votes v ON tp.PostId = v.PostId AND v.VoteTypeId = 8 GROUP BY tp.Title, tp.ViewCount, tp.AnswerCount, tp.CommentCount)
// SELECT ts.Title, ts.ViewCount, ts.AnswerCount, ts.CommentCount, ts.TotalComments, ts.TotalBounties, (CAST(ts.TotalComments AS decimal(10,2)) / NULLIF(ts.ViewCount, 0) * 100) AS CommentEngagementRate,
//        CASE WHEN ts.TotalBounties > 0 THEN 'Yes' ELSE 'No' END AS HasBounty
// FROM PostStats ts ORDER BY ts.ViewCount DESC, ts.AnswerCount DESC;
//
// PostStats groups by the four columns it names, not the post, so the top posts' rows are grouped by that tuple.
fn q9859(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, view_count, title, answer_count, comment_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(current_date(), -1))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let g = (&tp)
        .group_by(title.opt().and(view_count.opt()).and(answer_count.opt()).and(comment_count))
        .select(comments_of(db).opt().and(bounty.opt()))
        .fold([0i64; 3], |a, (c, b)| {
            let b = b.flatten();
            [a[0] + c.is_some() as i64, a[1] + b.is_some() as i64, a[2] + b.unwrap_or(0)]
        });
    rows(drain(&g).into_iter().map(|((((t, w), n), c), a)| {
        let rate = match w {
            Some(w) if w != 0 => V::F(a[0] as f64 / w as f64 * 100.0),
            _ => V::Null,
        };
        row(vec![ostr(t), oint(w), oint(n), V::I(c), V::I(a[0]), nullable(a[2], a[1]), rate, V::S(if a[1] > 0 && a[2] > 0 { "Yes" } else { "No" })])
    }))
}

// WITH UserBadgeCounts AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// PostStats AS (SELECT P.OwnerUserId, COUNT(P.Id) AS PostCount, SUM(P.Score) AS TotalScore, AVG(P.ViewCount) AS AvgViewCount, COUNT(DISTINCT P.Id) FILTER (WHERE P.PostTypeId = 1) AS QuestionCount,
//        COUNT(DISTINCT P.Id) FILTER (WHERE P.PostTypeId = 2) AS AnswerCount FROM Posts P GROUP BY P.OwnerUserId),
// UserRankings AS (SELECT UBC.UserId, UBC.DisplayName, COALESCE(PS.PostCount, 0) AS PostCount, COALESCE(PS.TotalScore, 0) AS TotalScore, COALESCE(PS.AvgViewCount, 0) AS AvgViewCount,
//        UBC.BadgeCount, UBC.GoldBadges, UBC.SilverBadges, UBC.BronzeBadges, RANK() OVER (ORDER BY COALESCE(PS.TotalScore, 0) DESC, UBC.BadgeCount DESC) AS Rank
//     FROM UserBadgeCounts UBC LEFT JOIN PostStats PS ON UBC.UserId = PS.OwnerUserId)
// SELECT UserId, DisplayName, PostCount, TotalScore, AvgViewCount, BadgeCount, GoldBadges, SilverBadges, BronzeBadges, Rank FROM UserRankings WHERE Rank <= 10 ORDER BY Rank;
fn q5935(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let Post { owner_user, score, view_count, .. } = &db.post;
    let ps = db.post.group_by(owner_user).select(score.and(view_count.opt())).fold([0i64; 4], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]);
    let r = ranked(drain((&ub).and((&ps).opt())), |&(_, (b, p))| (Reverse(p.map_or(0, |a| a[1])), Reverse(b[0])), false);
    rows(r.into_iter().take_while(|x| x.1 <= 10).map(|((u, (b, p)), r)| {
        let a = p.unwrap_or([0; 4]);
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), if a[2] == 0 { V::F(0.0) } else { avg(a[3], a[2]) }]);
        f.extend(b.map(V::I));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.ViewCount DESC) AS UserPostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE u.Reputation > 1000 AND p.CreationDate > (TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year')),
// PostStatistics AS (SELECT p.PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes FROM RankedPosts p LEFT JOIN Votes v ON p.PostId = v.PostId
//     GROUP BY p.PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount),
// TopRatedPosts AS (SELECT ps.PostId, ps.Title, ps.ViewCount, ps.Score, ps.AnswerCount, ps.Upvotes, ps.Downvotes, RANK() OVER (ORDER BY ps.Score DESC, ps.ViewCount DESC) AS PostRank FROM PostStatistics ps)
// SELECT t.Title, t.ViewCount, t.Score, t.AnswerCount, t.Upvotes, t.Downvotes, u.DisplayName, u.Reputation, u.CreationDate, u.Location
// FROM TopRatedPosts t JOIN Users u ON u.Id = (SELECT OwnerUserId FROM Posts WHERE Id = t.PostId) WHERE t.PostRank <= 10 ORDER BY t.PostRank;
//
// PostRank reads only base columns, so the ranked posts are picked first and their votes counted alone. The subquery is the post's owner.
fn q8060(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, view_count, .. } = &db.post;
    let hi = Ident::<User>::new().with((&db.user.reputation).gt(1000));
    let v = drain(db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user.select(hi)));
    let r = ranked(v, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    }, false);
    let tp: MatSet<Id<Post>> = rel(r.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let pv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = drain((&pv).and(owner_user));
    rows(v.into_iter().map(|(p, (a, u))| {
        let mut f = post_fields(db, p, &["title", "views", "score", "answers"]);
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.extend(ucols(db, u, &["name", "rep", "ucreated"]));
        f.push(ostr(db.user.location.get(u)));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.OwnerUserId, p.Title, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RN
//     FROM Posts p INNER JOIN Users u ON p.OwnerUserId = u.Id WHERE u.Reputation > 1000),
// RecentVotes AS (SELECT v.PostId, COUNT(v.Id) AS TotalVotes, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Votes v WHERE v.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' GROUP BY v.PostId),
// PostHistorySummary AS (SELECT ph.PostId, ph.UserId, ph.PostHistoryTypeId, COUNT(*) AS ChangeCount, MAX(ph.CreationDate) AS LastChangeDate FROM PostHistory ph
//     WHERE ph.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '6 months' GROUP BY ph.PostId, ph.UserId, ph.PostHistoryTypeId)
// SELECT rp.Title, rp.CreationDate, COALESCE(rv.TotalVotes, 0) AS TotalVotes, COALESCE(rv.UpVotes, 0) AS UpVotes, COALESCE(rv.DownVotes, 0) AS DownVotes,
//        COALESCE(phs.ChangeCount, 0) AS EditHistoryCount, phs.LastChangeDate
// FROM RankedPosts rp LEFT JOIN RecentVotes rv ON rp.PostId = rv.PostId LEFT JOIN PostHistorySummary phs ON rp.PostId = phs.PostId WHERE rp.RN = 1
// ORDER BY rp.CreationDate DESC LIMIT 10 OFFSET 0;
//
// RN breaks a CreationDate tie on the post id (the SQL leaves it open). A post joins one PostHistorySummary row per (user, type) group.
fn q2248(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let Post { owner_user, creation_date, .. } = &db.post;
    let hi = Ident::<User>::new().with((&db.user.reputation).gt(1000));
    let first = top_per(drain(db.post.select(owner_user.select(hi))), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let first = top_n(first, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 10);
    let fp: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let recent = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.creation_date).gt(add_days(t0, -30))));
    let rv = (&fp).group_by(Ident::<Post>::new()).select(recent.select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64],
        None => a,
    });
    let PostHistory { post, user_id, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let phs = db
        .post_history
        .with(hd.gt(add_months(t0, -6)))
        .with(post.select(&fp))
        .group_by(post.and(user_id.opt()).and(post_history_type_id))
        .select(hd)
        .fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let pv = rel(drain(&phs));
    type G = (((Id<Post>, Option<i64>), i64), (i64, i64));
    let by_post: HashIdx<Id<Post>, G> = (&pv).map(|(((p, _), _), _): G| p).inv().select(&pv).collect();
    let v = drain((&rv).and((&by_post).opt()));
    let v = top_n(v, |&(p, (_, g))| (Reverse(creation_date.get(p).unwrap()), p, g.map(|x| x.0)), 10);
    rows(v.into_iter().map(|(p, (a, g))| {
        let mut f = post_fields(db, p, &["title", "created"]);
        f.extend(a.map(V::I));
        f.extend(match g {
            Some((_, (n, d))) => [V::I(n), V::T(d)],
            None => [V::I(0), V::Null],
        });
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS TotalPosts, COUNT(DISTINCT C.Id) AS TotalComments,
//        SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON U.Id = C.UserId LEFT JOIN Votes V ON V.UserId = U.Id
//     WHERE U.CreationDate >= cast('2024-10-01' as date) - INTERVAL '2 years' GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, TotalComments, TotalUpVotes, TotalDownVotes, DENSE_RANK() OVER (ORDER BY Reputation DESC) AS RankByReputation,
//        DENSE_RANK() OVER (ORDER BY TotalPosts DESC) AS RankByPosts, DENSE_RANK() OVER (ORDER BY TotalUpVotes DESC) AS RankByUpVotes FROM UserActivity),
// CombinedRankings AS (SELECT UserId, DisplayName, Reputation, TotalPosts, TotalComments, TotalUpVotes, TotalDownVotes, RankByReputation, RankByPosts, RankByUpVotes,
//        LEAST(RankByReputation, RankByPosts, RankByUpVotes) AS OverallRank FROM TopUsers)
// SELECT UserId, DisplayName, Reputation, TotalPosts, TotalComments, TotalUpVotes, TotalDownVotes, OverallRank FROM CombinedRankings WHERE OverallRank <= 10 ORDER BY OverallRank;
fn q9819(db: &'static So) -> String {
    let nu = || db.user.with((&db.user.creation_date).ge(add_years(date(2024, 10, 1), -2)));
    let s = nu()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).opt().and(comments_by(db).opt()).and(votes_by(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let pc = nu().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let cc = nu().group_by(Ident::<User>::new()).select(comments_by(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&db.user.reputation).and(&s).and(&pc).and(&cc));
    let v = ranked(v, |&(_, (((r, _), _), _))| Reverse(r), true);
    let v = ranked(v, |&((_, ((_, n), _)), _)| Reverse(n), true);
    let v = ranked(v, |&(((_, (((_, a), _), _)), _), _)| Reverse(a[0]), true);
    let v = rel(v.into_iter().map(|(((x, r1), r2), r3)| (x, r1.min(r2).min(r3))).collect());
    type R = ((Id<User>, (((i64, [i64; 2]), i64), i64)), i64);
    let v = drain((&v).filt(|(_, o): R| o <= 10));
    rows(v.into_iter().map(|(_, ((u, (((r, a), n), c)), o))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(r), V::I(n), V::I(c), V::I(a[0]), V::I(a[1]), V::I(o)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, COALESCE(SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END), 0) AS PositivePosts,
//        COALESCE(SUM(CASE WHEN p.Score < 0 THEN 1 ELSE 0 END), 0) AS NegativePosts FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// RecentComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c WHERE c.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days' GROUP BY c.PostId)
// SELECT ps.PostId, ps.Title, ps.CreationDate, us.DisplayName AS Author, us.TotalPosts, us.PositivePosts, us.NegativePosts, COALESCE(rc.CommentCount, 0) AS RecentComments,
//        CASE WHEN ps.Score > 10 THEN 'High Score' WHEN ps.Score BETWEEN 1 AND 10 THEN 'Moderate Score' ELSE 'Low Score' END AS ScoreCategory
// FROM RankedPosts ps JOIN UserStats us ON ps.OwnerUserId = us.UserId LEFT JOIN RecentComments rc ON ps.PostId = rc.PostId WHERE ps.PostRank = 1 ORDER BY ps.CreationDate DESC LIMIT 50;
//
// PostRank breaks a CreationDate tie on the post id (the SQL leaves it open).
fn q1832(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let Post { owner_user, creation_date, score, .. } = &db.post;
    let first = top_per(drain(db.post.with(creation_date.ge(add_years(t0, -1))).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let first = top_n(first, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 50);
    let fp: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let owners: MatSet<Id<User>> = (&fp).select(owner_user).collect();
    let us = (&owners).group_by(Ident::<User>::new()).select(posts_of(db).select(score)).fold([0i64; 3], |a, s| [a[0] + 1, a[1] + (s > 0) as i64, a[2] + (s < 0) as i64]);
    let recent = comments_of(db).select(Ident::<Comment>::new().with((&db.comment.creation_date).ge(add_days(t0, -30))));
    let rc = (&fp).group_by(Ident::<Post>::new()).select(recent.opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&rc).and(owner_user.select(Ident::<User>::new().and(&us))));
    rows(v.into_iter().map(|(p, (c, (u, a)))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(c)]);
        f.push(V::S(if s > 10 { "High Score" } else if (1..=10).contains(&s) { "Moderate Score" } else { "Low Score" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.OwnerUserId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostActivity AS (SELECT p.Id AS PostId, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty, AVG(c.Score) AS AverageCommentScore
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (8, 9) LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY p.Id)
// SELECT u.DisplayName, up.BadgeCount, up.GoldBadges, up.SilverBadges, up.BronzeBadges, rp.Title, rp.Score, rp.ViewCount, pa.TotalBounty, pa.AverageCommentScore
// FROM Users u JOIN UserBadges up ON u.Id = up.UserId JOIN RankedPosts rp ON u.Id = rp.OwnerUserId AND rp.PostRank <= 5 JOIN PostActivity pa ON rp.PostId = pa.PostId
// WHERE up.BadgeCount > 0 ORDER BY up.BadgeCount DESC, rp.Score DESC LIMIT 10;
//
// PostRank breaks a Score tie on the post id (the SQL leaves it open). The ORDER BY reads the badge count and the post's Score only, so the
// ten rows are picked first and the bounty x comment product is driven for those posts alone.
fn q3219(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, .. } = &db.post;
    let rp = top_per(drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false);
    let rp: MatSet<Id<Post>> = rel(rp.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class)).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    let v = drain((&rp).select(owner_user.select(Ident::<User>::new().and(&ub))));
    let top = top_n(v, |&(p, (_, b))| (Reverse(b[0]), Reverse(score.get(p).unwrap()), p), 10);
    let tp: MatSet<Id<Post>> = rel(top.iter().map(|x| x.0).collect()).map(|p| p).collect();
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).in_v(vec![8, 9]))).select((&db.vote.bounty_amount).opt());
    let pa = (&tp).group_by(Ident::<Post>::new()).select(bounty.opt().and(comments_of(db).select(&db.comment.score).opt())).fold([0i64; 3], |a, (b, c)| {
        [a[0] + b.flatten().unwrap_or(0), a[1] + c.is_some() as i64, a[2] + c.unwrap_or(0)]
    });
    let v = drain((&pa).and(owner_user.select(Ident::<User>::new().and(&ub))));
    let v = top_n(v, |&(p, (_, (_, b)))| (Reverse(b[0]), Reverse(score.get(p).unwrap()), p), 10);
    rows(v.into_iter().map(|(p, (a, (u, b)))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(b.map(V::I));
        f.extend(post_fields(db, p, &["title", "score", "views"]));
        f.extend([V::I(a[0]), avg(a[2], a[1])]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COALESCE(SUM(CASE WHEN C.Id IS NOT NULL THEN 1 ELSE 0 END), 0) AS CommentsCount,
//        COALESCE(SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END), 0) AS QuestionsAsked
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Comments C ON P.Id = C.PostId GROUP BY U.Id, U.DisplayName),
// RankedUsers AS (SELECT UA.UserId, UA.DisplayName, UA.UpVotes, UA.DownVotes, UA.CommentsCount, UA.QuestionsAsked, RANK() OVER (ORDER BY UA.UpVotes - UA.DownVotes DESC, UA.CommentsCount DESC) AS ActivityRank
//     FROM UserActivity UA),
// FilteredUsers AS (SELECT R.DisplayName, R.ActivityRank, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM RankedUsers R LEFT JOIN Badges B ON R.UserId = B.UserId WHERE R.ActivityRank <= 10 GROUP BY R.DisplayName, R.ActivityRank)
// SELECT F.DisplayName, F.ActivityRank, F.GoldBadges, F.SilverBadges, F.BronzeBadges FROM FilteredUsers F ORDER BY F.ActivityRank;
//
// FilteredUsers groups the ranked users' badge rows by (DisplayName, ActivityRank).
fn q3407(db: &'static So) -> String {
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt())).opt())
        .fold([0i64; 3], |a, p| match p {
            Some((t, c)) => [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64],
            None => a,
        });
    let r = ranked(drain(&ua), |&(_, a)| (Reverse(a[0] - a[1]), Reverse(a[2])), false);
    let r = rel(r.into_iter().take_while(|x| x.1 <= 10).map(|((u, _), r)| (u, r)).collect());
    let rank: HashIdx<Id<User>, i64> = (&r).map(|(u, _)| u).inv().select((&r).map(|(_, r)| r)).collect();
    let fu = db
        .user
        .with(&rank)
        .group_by((&db.user.display_name).and(&rank))
        .select(badges_of(db).select(&db.badge.class).opt())
        .fold([0i64; 3], |a, c| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    rows(drain(&fu).into_iter().map(|((n, r), a)| row(vec![V::S(n), V::I(r), V::I(a[0]), V::I(a[1]), V::I(a[2])])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.OwnerUserId, u.DisplayName AS OwnerDisplayName, p.Tags,
//        ROW_NUMBER() OVER (PARTITION BY CASE WHEN u.Reputation >= 1000 THEN 1 ELSE 0 END ORDER BY p.Score DESC) AS Rnk
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.CreationDate >= CURRENT_DATE - INTERVAL '1 year'),
// DistinctTags AS (SELECT t.TagName, COUNT(DISTINCT p.Id) AS PostCount FROM Tags t JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' GROUP BY t.TagName),
// TopBadges AS (SELECT b.UserId, b.Name, COUNT(b.Id) AS BadgeCount FROM Badges b GROUP BY b.UserId, b.Name),
// BadgeDetails AS (SELECT ub.UserId, u.DisplayName, tb.Name AS BadgeName, tb.BadgeCount FROM (SELECT UserId, SUM(BadgeCount) AS TotalBadges FROM TopBadges GROUP BY UserId) ub
//     JOIN Users u ON ub.UserId = u.Id JOIN TopBadges tb ON ub.UserId = tb.UserId)
// SELECT rp.PostId, rp.Title, rp.Body, rp.CreationDate, rp.OwnerDisplayName, dt.TagName, bd.BadgeName, bd.BadgeCount
// FROM RankedPosts rp LEFT JOIN DistinctTags dt ON rp.Tags LIKE '%' || dt.TagName || '%' LEFT JOIN BadgeDetails bd ON rp.OwnerUserId = bd.UserId WHERE rp.Rnk = 1 ORDER BY rp.CreationDate DESC;
//
// Rnk breaks a Score tie on the post id (the SQL leaves it open). Every tag the LIKE matches on an rp post has that post, so it is in
// DistinctTags; the match is `tag_mentions`. BadgeDetails is one row per (user, badge name).
fn q26306(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(current_date(), -1)))).select(owner_user.select((&db.user.reputation).map(|r: i64| r >= 1000))));
    let top = top_per(v, |&(_, b)| b, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 1, false);
    let rp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let lt = tag_mentions(db);
    let tags_of: HashIdx<Id<Post>, Id<Tag>> = (&lt).map(|(p, _)| p).inv().select((&lt).map(|(_, t)| t)).collect();
    let Badge { user, name, .. } = &db.badge;
    let tb = db.badge.group_by(user.and(name)).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let tv = rel(drain(&tb));
    type B = ((Id<User>, Str), i64);
    let bd: HashIdx<Id<User>, B> = (&tv).map(|((u, _), _): B| u).inv().select(&tv).collect();
    let v = drain((&rp).select((&tags_of).select(&db.tag.tag_name).opt().and(owner_user.select(&bd).opt())));
    rows(v.into_iter().map(|(p, (t, b))| {
        let mut f = post_fields(db, p, &["id", "title", "body", "created", "owner"]);
        f.push(ostr(t));
        f.extend(match b {
            Some(((_, n), c)) => [V::S(n), V::I(c)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// UserBadgeCounts AS (SELECT u.Id AS UserId, COUNT(b.Id) FILTER (WHERE b.Class = 1) AS GoldBadges, COUNT(b.Id) FILTER (WHERE b.Class = 2) AS SilverBadges,
//        COUNT(b.Id) FILTER (WHERE b.Class = 3) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostVoteStats AS (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, COUNT(v.Id) AS TotalVotes
//     FROM Votes v GROUP BY v.PostId)
// SELECT up.DisplayName, rp.PostId, rp.Title, rp.ViewCount, rp.Score, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges, COALESCE(pvs.UpVotes, 0) AS UpVotes, COALESCE(pvs.DownVotes, 0) AS DownVotes,
//        CASE WHEN rp.Rank = 1 THEN 'Latest Post' ELSE 'Older Post' END AS PostStatus
// FROM RankedPosts rp JOIN Users up ON rp.PostId = up.Id LEFT JOIN UserBadgeCounts ub ON up.Id = ub.UserId LEFT JOIN PostVoteStats pvs ON rp.PostId = pvs.PostId
// WHERE rp.Rank <= 3 ORDER BY rp.Score DESC, rp.ViewCount DESC LIMIT 10;
//
// `rp.PostId = up.Id` joins a post id to a user id, so it goes through the raw ids. The ownerless posts rank as one partition; Rank breaks a
// CreationDate tie on the post id (the SQL leaves it open).
fn q995(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, view_count, origid, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 3, false);
    let r = rel(ranked(top, |&(p, u)| (u, Reverse(creation_date.get(p).unwrap()), p), false));
    let r = per_group(drain(&r).into_iter().map(|x| x.1).collect(), |&(_, u)| u);
    let rr = rel(r.into_iter().map(|((p, _), r)| (p, r)).collect());
    let rank: HashIdx<Id<Post>, i64> = (&rr).map(|(p, _)| p).inv().select((&rr).map(|(_, r)| r)).collect();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| {
        [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]
    });
    let hits: MatSet<Id<Post>> = db.post.with(&rank).with(origid.select(&uidx)).collect();
    let pv = (&hits).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = drain((&pv).and(&rank).and(origid.select(&uidx).select(Ident::<User>::new().and(&ub))));
    let v = top_n(v, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 10);
    rows(v.into_iter().map(|(p, ((a, r), (u, b)))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["id", "title", "views", "score"]));
        f.extend(b.map(V::I));
        f.extend([V::I(a[0]), V::I(a[1]), V::S(if r == 1 { "Latest Post" } else { "Older Post" })]);
        row(f)
    }))
}

// WITH RECURSIVE RecursiveTopUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.CreationDate, ROW_NUMBER() OVER (ORDER BY u.Reputation DESC) AS Rank FROM Users u WHERE u.Reputation > 1000),
// PostsRanked AS (SELECT p.Id AS PostId, p.OwnerUserId, p.Title, p.CreationDate, p.Score, COUNT(c.Id) AS CommentCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.OwnerUserId, p.Title, p.CreationDate, p.Score),
// UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.PostId) AS TotalPosts, SUM(p.Score) AS TotalScore, SUM(COALESCE(c.CommentCount, 0)) AS TotalComments
//     FROM Users u LEFT JOIN PostsRanked p ON u.Id = p.OwnerUserId LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.PostId = c.PostId
//     WHERE u.Reputation > 500 GROUP BY u.Id, u.DisplayName)
// SELECT t.UserId, t.DisplayName, t.Reputation, ups.TotalPosts, ups.TotalScore, ups.TotalComments FROM RecursiveTopUsers t JOIN UserPostStats ups ON t.UserId = ups.UserId
// WHERE ups.TotalPosts > 5 ORDER BY t.Rank, ups.TotalScore DESC LIMIT 10;
//
// WITH RECURSIVE, but no CTE refers to itself. Rank breaks a Reputation tie on the user id (the SQL leaves it open).
fn q31815(db: &'static So) -> String {
    let Post { creation_date, score, .. } = &db.post;
    let recent = posts_of(db).select(Ident::<Post>::new().with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))));
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ups = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(recent.select(score.and(&cc)))
        .fold([0i64; 3], |a, (s, c)| [a[0] + 1, a[1] + s, a[2] + c]);
    let v = top_n(drain((&ups).filt(|a| a[0] > 5)), |&(u, a)| (Reverse(db.user.reputation.get(u).unwrap()), db.user.origid.get(u).unwrap(), Reverse(a[1])), 10);
    rows(v.into_iter().map(|(u, a)| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS rn, p.OwnerUserId
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.Score > 0),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount, COALESCE(SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges,
//        COALESCE(SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadges, COALESCE(SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadges
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName)
// SELECT us.DisplayName, us.UpVoteCount, us.DownVoteCount, us.GoldBadges, us.SilverBadges, us.BronzeBadges, COUNT(DISTINCT rp.Id) AS PostCount, MAX(rp.Score) AS HighestPostScore,
//        COUNT(DISTINCT CASE WHEN rp.rn = 1 THEN rp.Id END) AS TopPostsCount
// FROM UserStats us LEFT JOIN RankedPosts rp ON us.UserId = rp.OwnerUserId GROUP BY us.DisplayName, us.UpVoteCount, us.DownVoteCount, us.GoldBadges, us.SilverBadges, us.BronzeBadges
// HAVING COUNT(DISTINCT rp.Id) > 5 OR SUM(us.UpVoteCount) > 100 ORDER BY us.UpVoteCount DESC, HighestPostScore DESC;
//
// The final GROUP BY does not name the user, so the (user, question) rows are grouped by the named columns. Each question has one owner,
// so a group's rows hold each question once and the distinct counts are counts of rows. rn = 1 marks each owner's best question (a Score
// tie broken on the post id; which one is picked does not change the count).
fn q3369(db: &'static So) -> String {
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 5], |a, (t, c)| {
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + (c == Some(1)) as i64, a[3] + (c == Some(2)) as i64, a[4] + (c == Some(3)) as i64]
        });
    let Post { post_type_id, score, owner_user, .. } = &db.post;
    let rp = || Ident::<Post>::new().with(post_type_id.eq(1).and(score.gt(0)));
    let best = top_per(drain(db.post.with(post_type_id.eq(1).and(score.gt(0))).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 1, false);
    let best: MatSet<Id<Post>> = rel(best.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let j = rel(drain(db.user.select((&db.user.display_name).and(&us).and(posts_of(db).select(rp().and(score).and(Ident::<Post>::new().with(&best).opt())).opt()))));
    type R = (Id<User>, ((Str, [i64; 5]), Option<((Id<Post>, i64), Option<Id<Post>>)>));
    let g = (&j)
        .group_by(Same::<R>::new().map(|(_, (k, _)): R| k))
        .select(Same::<R>::new().map(|(_, x): R| x))
        .fold([0, i64::MIN, 0, 0], |a, ((_, s), p)| match p {
            Some(((_, sc), b)) => [a[0] + 1, a[1].max(sc), a[2] + b.is_some() as i64, a[3] + s[0]],
            None => [a[0], a[1], a[2], a[3] + s[0]],
        });
    let v = drain((&g).filt(|a| a[0] > 5 || a[3] > 100));
    rows(v.into_iter().map(|((n, s), a)| {
        let mut f = vec![V::S(n)];
        f.extend(s.map(V::I));
        f.extend([V::I(a[0]), if a[0] == 0 { V::Null } else { V::I(a[1]) }, V::I(a[2])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, p.OwnerUserId, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// PostStats AS (SELECT rp.PostId, rp.Title, rp.OwnerDisplayName, rp.Score, rp.ViewCount, COUNT(c.Id) AS CommentCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY rp.PostId ORDER BY rp.Score DESC, rp.ViewCount DESC) AS Rank
//     FROM RankedPosts rp LEFT JOIN Comments c ON rp.PostId = c.PostId LEFT JOIN Votes v ON rp.PostId = v.PostId GROUP BY rp.PostId, rp.Title, rp.OwnerDisplayName, rp.Score, rp.ViewCount)
// SELECT ps.PostId, ps.Title, ps.OwnerDisplayName, ps.Score, ps.ViewCount, ps.CommentCount, ps.UpVotes, ps.DownVotes,
//        CASE WHEN ps.Rank <= 10 THEN 'Top 10%' WHEN ps.Rank <= 50 THEN 'Top 50%' ELSE 'Other' END AS PerformanceTier
// FROM PostStats ps JOIN RankedPosts rp ON ps.PostId = rp.PostId WHERE ps.CommentCount > 5 ORDER BY ps.Score DESC, ps.ViewCount DESC;
//
// PostStats' Rank partitions by PostId, one row per partition, so it is always 1 and every tier is 'Top 10%'.
fn q5519(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let ps = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(owner_user)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain((&ps).filt(|a| a[0] > 5)).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "score", "views"]);
        f.extend(a.map(V::I));
        f.push(V::S("Top 10%"));
        row(f)
    }))
}

// WITH RankedUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, ROW_NUMBER() OVER (ORDER BY u.Reputation DESC) AS UserRank FROM Users u WHERE u.Reputation > 0),
// PostStats AS (SELECT p.Id AS PostId, p.Title, COUNT(c.Id) AS CommentCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COUNT(DISTINCT CASE WHEN v.VoteTypeId = 2 THEN v.UserId END) AS UniqueUpVoters,
//        COUNT(DISTINCT CASE WHEN v.VoteTypeId = 3 THEN v.UserId END) AS UniqueDownVoters, COUNT(DISTINCT ph.Id) AS HistoryCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId GROUP BY p.Id, p.Title),
// TopPosts AS (SELECT ps.PostId, ps.Title, ps.CommentCount, ps.UpVotes, ps.DownVotes, ps.UniqueUpVoters, ps.UniqueDownVoters, ps.HistoryCount,
//        RANK() OVER (ORDER BY ps.UpVotes - ps.DownVotes DESC) AS PostRank FROM PostStats ps)
// SELECT ru.DisplayName AS TopUser, tp.Title AS PostTitle, tp.CommentCount, tp.UpVotes, tp.DownVotes, tp.UniqueUpVoters, tp.UniqueDownVoters, tp.HistoryCount
// FROM TopPosts tp JOIN RankedUsers ru ON ru.UserId IN (SELECT p.OwnerUserId FROM Posts p WHERE p.Id = tp.PostId) WHERE tp.PostRank <= 10 ORDER BY tp.PostRank, ru.Reputation DESC;
//
// The IN subquery makes ru the post's owner (with Reputation > 0). The distinct counts are separate folds over the ranked posts.
fn q5913(db: &'static So) -> String {
    let Vote { vote_type_id, user_id, .. } = &db.vote;
    let ps = db
        .post
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(vote_type_id).opt()).and(history_of(db).opt()))
        .fold([0i64; 3], |a, ((c, t), _)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let r = ranked(drain(&ps), |&(_, a)| Reverse(a[1] - a[2]), false);
    let r = rel(r.into_iter().take_while(|x| x.1 <= 10).map(|((p, a), r)| (p, (a, r))).collect());
    type T = (Id<Post>, ([i64; 3], i64));
    let tp: HashIdx<Id<Post>, ([i64; 3], i64)> = (&r).map(|(p, _): T| p).inv().select((&r).map(|(_, x): T| x)).collect();
    let voters = |k: i64| votes_of(db).select(Ident::<Vote>::new().with(vote_type_id.eq(k))).select(user_id).opt();
    let uu = db.post.with(&tp).group_by(Ident::<Post>::new()).select(voters(2)).buf_fold(distinct_some);
    let ud = db.post.with(&tp).group_by(Ident::<Post>::new()).select(voters(3)).buf_fold(distinct_some);
    let hc = db.post.with(&tp).group_by(Ident::<Post>::new()).select(history_of(db).opt()).fold(0i64, |n, h| n + h.is_some() as i64);
    let owner = (&db.post.owner_user).select(Ident::<User>::new().with((&db.user.reputation).gt(0)));
    let v = drain((&tp).and(&uu).and(&ud).and(&hc).and(owner));
    let v = top_n(v, |&(p, ((((( _, r), _), _), _), u))| (r, Reverse(db.user.reputation.get(u).unwrap()), p), 0);
    rows(v.into_iter().map(|(p, (((((a, _), uu), ud), h), u))| {
        let mut f = vec![user_col(db, u, "name"), post_fields(db, p, &["title"]).pop().unwrap()];
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(uu), V::I(ud), V::I(h)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount, COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 2) AS UpVoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, u.DisplayName, p.OwnerUserId),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges, SUM(CASE WHEN p.Title IS NOT NULL THEN 1 ELSE 0 END) AS TotalPosts
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName)
// SELECT r.OwnerDisplayName, r.Title, r.CommentCount, r.UpVoteCount, u.TotalPosts, u.GoldBadges + u.SilverBadges + u.BronzeBadges AS TotalBadges
// FROM RankedPosts r JOIN UserStats u ON r.OwnerDisplayName = u.DisplayName WHERE r.PostRank = 1 ORDER BY r.UpVoteCount DESC, r.CommentCount DESC LIMIT 10;
//
// PostRank breaks a CreationDate tie on the post id (the SQL leaves it open); the ownerless posts have no name and never join. UserStats is
// driven only for the users whose name matches a newest post's owner.
fn q7821(db: &'static So) -> String {
    let Post { creation_date, owner_user, title, .. } = &db.post;
    let first = top_per(drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let fp: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let rp = (&fp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64]);
    let names: MatSet<Str> = (&fp).select(owner_user.select(&db.user.display_name)).collect();
    let by_name: HashIdx<Str, Id<User>> = db.user.with((&db.user.display_name).select(&names)).select(&db.user.display_name).inv().collect();
    let cand: MatSet<Id<User>> = (&names).select(&by_name).collect();
    let us = (&cand)
        .group_by(Ident::<User>::new())
        .select(badges_of(db).select(&db.badge.class).opt().and(posts_of(db).select(title.opt()).opt()))
        .fold([0i64; 4], |a, (c, t)| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64, a[3] + t.flatten().is_some() as i64]);
    let v = drain((&rp).and(owner_user.select(&db.user.display_name).select((&by_name).select(Ident::<User>::new().and(&us)))));
    let v = top_n(v, |&(p, (a, (u, _)))| (Reverse(a[1]), Reverse(a[0]), p, u), 10);
    rows(v.into_iter().map(|(p, (a, (_, s)))| {
        let mut f = post_fields(db, p, &["owner", "title"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(s[3]), V::I(s[0] + s[1] + s[2])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank,
//        COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT v.Id) AS VoteCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate > (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 YEAR') GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.Score),
// UserBadges AS (SELECT b.UserId, COUNT(b.Id) FILTER (WHERE b.Class = 1) AS GoldBadges, COUNT(b.Id) FILTER (WHERE b.Class = 2) AS SilverBadges,
//        COUNT(b.Id) FILTER (WHERE b.Class = 3) AS BronzeBadges FROM Badges b GROUP BY b.UserId),
// TopPosts AS (SELECT rp.Id, rp.Title, rp.CreationDate, rp.OwnerUserId, rp.Score, rp.CommentCount, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges
//     FROM RankedPosts rp LEFT JOIN UserBadges ub ON rp.OwnerUserId = ub.UserId WHERE rp.PostRank = 1)
// SELECT tp.Title AS TopPostTitle, tp.Score, tp.CommentCount, COALESCE(tp.GoldBadges, 0) AS GoldBadges, COALESCE(tp.SilverBadges, 0) AS SilverBadges, COALESCE(tp.BronzeBadges, 0) AS BronzeBadges,
//        (SELECT AVG(Score) FROM RankedPosts) AS AveragePostScore, (SELECT COUNT(*) FROM Posts p WHERE p.ViewCount > 1000) AS HighViewCountPosts
// FROM TopPosts tp ORDER BY tp.Score DESC LIMIT 10;
//
// PostRank breaks a Score tie on the post id (the SQL leaves it open); the ownerless posts rank as one partition. The two scalar subqueries
// are one-row folds.
fn q3629(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, view_count, .. } = &db.post;
    let recent = || db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let (n, s) = recent().select(score).fold_flat((0i64, 0i64), |(n, t), x| (n + 1, t + x));
    let hv = count(db.post.with(view_count.gt(1000)));
    let best = top_per(drain(recent().select(owner_user.opt())), |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 1, false);
    let best = top_n(best, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 10);
    let tp: MatSet<Id<Post>> = rel(best.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |k, c| k + c.is_some() as i64);
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let v = drain((&cc).and(owner_user.select(&ub).opt()));
    rows(v.into_iter().map(|(p, (c, b))| {
        let mut f = post_fields(db, p, &["title", "score"]);
        f.push(V::I(c));
        f.extend(b.unwrap_or([0; 3]).map(V::I));
        f.extend([avg(s, n), V::I(hv)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS rn, COALESCE(u.Reputation, 0) AS UserReputation
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// RecentVotes AS (SELECT v.PostId, COUNT(v.Id) AS VoteCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Votes v WHERE v.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '6 months' GROUP BY v.PostId),
// PostWithVotes AS (SELECT rp.PostId, rp.Title, rp.Score, rp.UserReputation, COALESCE(rv.VoteCount, 0) AS TotalVotes, COALESCE(rv.UpVotes, 0) AS UpVotes, COALESCE(rv.DownVotes, 0) AS DownVotes
//     FROM RankedPosts rp LEFT JOIN RecentVotes rv ON rp.PostId = rv.PostId)
// SELECT pwv.PostId, pwv.Title, pwv.Score, pwv.UserReputation, pwv.TotalVotes, pwv.UpVotes, pwv.DownVotes,
//        CASE WHEN pwv.Score >= 10 THEN 'High Score' WHEN pwv.Score BETWEEN 5 AND 9 THEN 'Medium Score' ELSE 'Low Score' END AS ScoreCategory
// FROM PostWithVotes pwv WHERE pwv.UserReputation > 50 ORDER BY pwv.Score DESC NULLS LAST FETCH FIRST 10 ROWS ONLY;
//
// A post without an owner has UserReputation 0, so the WHERE keeps only owned posts. The ORDER BY reads only Score, so the ten posts are
// picked first.
fn q180(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let rep = owner_user.select(&db.user.reputation);
    let v = drain(db.post.with(creation_date.ge(add_years(t0, -1))).select(rep.gt(50)));
    let top = top_n(v, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 10);
    let tv = rel(top);
    type R = (Id<Post>, i64);
    let tp: HashIdx<Id<Post>, i64> = (&tv).map(|(p, _): R| p).inv().select((&tv).map(|(_, r): R| r)).collect();
    let recent = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.creation_date).ge(add_months(t0, -6))));
    let rv = db.post.with(&tp).group_by(Ident::<Post>::new()).select(recent.select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64],
        None => a,
    });
    let v = drain((&tp).and(&rv));
    rows(v.into_iter().map(|(p, (r, a))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "score"]);
        f.push(V::I(r));
        f.extend(a.map(V::I));
        f.push(V::S(if s >= 10 { "High Score" } else if (5..=9).contains(&s) { "Medium Score" } else { "Low Score" }));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS TotalPosts, COUNT(DISTINCT CASE WHEN P.PostTypeId = 1 THEN P.Id END) AS Questions,
//        COUNT(DISTINCT CASE WHEN P.PostTypeId = 2 THEN P.Id END) AS Answers, SUM(COALESCE(P.ViewCount, 0)) AS TotalViews, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation),
// RankedPosts AS (SELECT UserId, DisplayName, Reputation, TotalPosts, Questions, Answers, TotalViews, Upvotes, Downvotes, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserStats),
// TopUsers AS (SELECT *, CASE WHEN Reputation >= 1000 THEN 'Gold' WHEN Reputation >= 500 THEN 'Silver' ELSE 'Bronze' END AS Badge FROM RankedPosts WHERE TotalPosts > 10)
// SELECT U.DisplayName, U.Reputation, U.Badge, U.TotalPosts, U.Questions, U.Answers, U.TotalViews, U.Upvotes, U.Downvotes, (CAST(U.Upvotes AS FLOAT) / NULLIF(U.TotalPosts, 0)) * 100 AS UpvotePercentage
// FROM TopUsers U WHERE U.ReputationRank <= 50 ORDER BY U.Reputation DESC, U.DisplayName LIMIT 20;
//
// ReputationRank reads only Reputation and is taken over every user, so the fifty ranked users are picked first and the posts x votes product
// is driven for those alone. The percentage is FLOAT (f32) arithmetic.
fn q396(db: &'static So) -> String {
    let r = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let tu: MatSet<Id<User>> = rel(r.into_iter().take_while(|x| x.1 <= 50).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let Post { post_type_id, view_count, .. } = &db.post;
    let s = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(view_count.opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 3], |a, p| match p {
            Some((w, t)) => [a[0] + w.unwrap_or(0), a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64],
            None => a,
        });
    let dp = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id).opt()).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64],
        None => a,
    });
    let v = drain((&dp).filt(|a| a[0] > 10).and(&s));
    let v = top_n(v, |&(u, _)| (Reverse(db.user.reputation.get(u).unwrap()), db.user.display_name.get(u).unwrap(), u), 20);
    rows(v.into_iter().map(|(u, (d, a))| {
        let rep = db.user.reputation.get(u).unwrap();
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::S(if rep >= 1000 { "Gold" } else if rep >= 500 { "Silver" } else { "Bronze" }));
        f.extend([V::I(d[0]), V::I(d[1]), V::I(d[2]), V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        f.push(V::F(((a[1] as f32 / d[0] as f32) * 100.0f32) as f64));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U WHERE U.Reputation IS NOT NULL),
// PostStats AS (SELECT P.OwnerUserId, COUNT(P.Id) AS TotalPosts, COALESCE(SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END), 0) AS PositiveScores,
//        COALESCE(SUM(CASE WHEN P.Score < 0 THEN 1 ELSE 0 END), 0) AS NegativeScores, AVG(P.ViewCount) AS AverageViews FROM Posts P GROUP BY P.OwnerUserId),
// UserBadgeCounts AS (SELECT B.UserId, COUNT(B.Id) AS BadgeCount FROM Badges B GROUP BY B.UserId),
// CombinedStats AS (SELECT U.DisplayName, COALESCE(UR.Reputation, 0) AS Reputation, COALESCE(PS.TotalPosts, 0) AS TotalPosts, COALESCE(PS.PositiveScores, 0) AS PositiveScores,
//        COALESCE(PS.NegativeScores, 0) AS NegativeScores, COALESCE(UBC.BadgeCount, 0) AS BadgeCount, UR.ReputationRank
//     FROM Users U LEFT JOIN UserReputation UR ON U.Id = UR.UserId LEFT JOIN PostStats PS ON U.Id = PS.OwnerUserId LEFT JOIN UserBadgeCounts UBC ON U.Id = UBC.UserId WHERE U.Reputation > 0),
// FinalResults AS (SELECT *, RANK() OVER (ORDER BY Reputation DESC, BadgeCount DESC) AS OverallRank FROM CombinedStats)
// SELECT DisplayName, Reputation, TotalPosts, PositiveScores, NegativeScores, BadgeCount, OverallRank FROM FinalResults WHERE OverallRank <= 10 ORDER BY OverallRank;
fn q3748(db: &'static So) -> String {
    let Post { owner_user, score, .. } = &db.post;
    let ps = db.post.group_by(owner_user).select(score).fold([0i64; 3], |a, s| [a[0] + 1, a[1] + (s > 0) as i64, a[2] + (s < 0) as i64]);
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let v = drain(db.user.with((&db.user.reputation).gt(0)).select((&db.user.reputation).and((&ps).opt()).and((&bc).opt())));
    let r = ranked(v, |&(_, ((rep, _), b))| (Reverse(rep), Reverse(b.unwrap_or(0))), false);
    rows(r.into_iter().take_while(|x| x.1 <= 10).map(|((u, ((rep, p), b)), r)| {
        let p = p.unwrap_or([0; 3]);
        row(vec![user_col(db, u, "name"), V::I(rep), V::I(p[0]), V::I(p[1]), V::I(p[2]), V::I(b.unwrap_or(0)), V::I(r)])
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.CreationDate, ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U),
// PostStats AS (SELECT P.OwnerUserId, COUNT(P.Id) AS PostCount, SUM(P.ViewCount) AS TotalViews, AVG(P.Score) AS AverageScore FROM Posts P GROUP BY P.OwnerUserId),
// TopUsers AS (SELECT UR.UserId, UR.DisplayName, UR.Reputation, PS.PostCount, PS.TotalViews, PS.AverageScore, RANK() OVER (ORDER BY UR.Reputation DESC) AS UserRank
//     FROM UserReputation UR JOIN PostStats PS ON UR.UserId = PS.OwnerUserId),
// RecentPosts AS (SELECT P.Id, P.Title, P.CreationDate, P.OwnerUserId, COALESCE(COUNT(C.Id), 0) AS CommentCount FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId
//     WHERE P.CreationDate >= CURRENT_TIMESTAMP - INTERVAL '30 days' GROUP BY P.Id, P.Title, P.CreationDate, P.OwnerUserId)
// SELECT TU.UserId, TU.DisplayName, TU.Reputation, TU.PostCount, RANK() OVER (ORDER BY TU.Reputation DESC) AS GlobalRank, RP.Title, RP.CreationDate, RP.CommentCount,
//        CASE WHEN TU.Reputation > 1000 THEN 'Expert' WHEN TU.Reputation BETWEEN 500 AND 1000 THEN 'Intermediate' ELSE 'Novice' END AS UserLevel
// FROM TopUsers TU JOIN RecentPosts RP ON TU.UserId = RP.OwnerUserId WHERE TU.UserRank <= 10 ORDER BY TU.Reputation DESC, RP.CreationDate DESC LIMIT 5;
//
// CreationDate is compared as a TIMESTAMPTZ in the session zone against CURRENT_TIMESTAMP (see `ny_to_utc`).
fn q4752(db: &'static So) -> String {
    let since = now_utc() - 30 * DAY_US;
    let Post { owner_user, creation_date, .. } = &db.post;
    let pc = db.post.group_by(owner_user).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let r = ranked(drain(db.user.with(&pc).select(&db.user.reputation)), |&(_, r)| Reverse(r), false);
    let tu: MatSet<Id<User>> = rel(r.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let rp = db.post.with(creation_date.filt(move |d: i64| ny_to_utc(d) >= since)).with(owner_user.select(&tu));
    let cc = rp.group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&cc).and(owner_user.select(Ident::<User>::new().and(&pc))));
    let v = ranked(v, |&(_, (_, (u, _)))| Reverse(db.user.reputation.get(u).unwrap()), false);
    let v = top_n(v, |&((p, (_, (u, _))), _)| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 5);
    rows(v.into_iter().map(|((p, (c, (u, n))), g)| {
        let rep = db.user.reputation.get(u).unwrap();
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(n), V::I(g)]);
        f.extend(post_fields(db, p, &["title", "created"]));
        f.extend([V::I(c), V::S(if rep > 1000 { "Expert" } else if (500..=1000).contains(&rep) { "Intermediate" } else { "Novice" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostID, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.PostTypeId IN (1, 2)),
// TopPosts AS (SELECT PostID, Title, CreationDate, Score, ViewCount, OwnerName FROM RankedPosts WHERE Rank <= 5),
// PostDetails AS (SELECT tp.PostID, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.OwnerName, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount FROM TopPosts tp LEFT JOIN Comments c ON tp.PostID = c.PostId LEFT JOIN Votes v ON tp.PostID = v.PostId
//     GROUP BY tp.PostID, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.OwnerName)
// SELECT pd.PostID, pd.Title, pd.CreationDate, pd.Score, pd.ViewCount, pd.OwnerName, pd.CommentCount, pd.UpVoteCount, pd.DownVoteCount,
//        ROUND(COALESCE(NULLIF(pd.UpVoteCount, 0), 1) * 100.0 / NULLIF(pd.UpVoteCount + pd.DownVoteCount, 0), 2) AS UpVotePercentage
// FROM PostDetails pd ORDER BY pd.Score DESC, pd.ViewCount DESC;
fn q6011(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.is_in([1, 2]).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain(&s).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend(a.map(V::I));
        let num = if a[1] == 0 { 1 } else { a[1] };
        f.push(if a[1] + a[2] == 0 { V::Null } else { V::F((num as f64 * 100.0 / (a[1] + a[2]) as f64 * 100.0).round() / 100.0) });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.OwnerUserId, P.ViewCount, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS PostRank
//     FROM Posts P WHERE P.PostTypeId = 1),
// UserVotes AS (SELECT V.PostId, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes V GROUP BY V.PostId),
// ActiveUsers AS (SELECT U.Id, U.DisplayName, U.Reputation, U.LastAccessDate FROM Users U WHERE U.LastAccessDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// PostComments AS (SELECT C.PostId, COUNT(C.Id) AS CommentCount FROM Comments C GROUP BY C.PostId)
// SELECT RP.PostId, RP.Title, RP.CreationDate, AU.DisplayName AS Author, COALESCE(UP.UpVotes, 0) AS TotalUpVotes, COALESCE(DN.DownVotes, 0) AS TotalDownVotes,
//        COALESCE(PC.CommentCount, 0) AS TotalComments, CASE WHEN RP.PostRank = 1 THEN 'Latest Post' ELSE 'Older Post' END AS PostStatus
// FROM RankedPosts RP LEFT JOIN UserVotes UP ON RP.PostId = UP.PostId LEFT JOIN UserVotes DN ON RP.PostId = DN.PostId JOIN ActiveUsers AU ON RP.OwnerUserId = AU.Id
// LEFT JOIN PostComments PC ON RP.PostId = PC.PostId WHERE RP.PostRank <= 5 ORDER BY RP.OwnerUserId, RP.CreationDate DESC;
//
// PostRank breaks a CreationDate tie on the post id (the SQL leaves it open).
fn q4631(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let active = Ident::<User>::new().with((&db.user.last_access_date).ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)));
    let v = drain(db.post.with(post_type_id.eq(1)).select(owner_user.select(active)));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let first = top_per(top.clone(), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let first: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let uv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let pc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&uv).and(&pc).and(owner_user).and(Ident::<Post>::new().with(&first).opt()));
    rows(v.into_iter().map(|(p, (((a, c), u), l))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(c), V::S(if l.is_some() { "Latest Post" } else { "Older Post" })]);
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, RANK() OVER (ORDER BY U.Reputation DESC) AS Rank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, AnswerCount, QuestionCount, UpVotes, DownVotes, Rank FROM UserStatistics WHERE Rank <= 10),
// ClosedPosts AS (SELECT P.Id AS PostId, P.Title, PH.UserId AS ClosedBy, PH.CreationDate AS ClosedDate FROM Posts P JOIN PostHistory PH ON P.Id = PH.PostId WHERE PH.PostHistoryTypeId = 10)
// SELECT TU.DisplayName, TU.Reputation, TU.AnswerCount, TU.QuestionCount, CP.Title AS ClosedPostTitle, CP.ClosedDate, (COALESCE(TU.UpVotes, 0) - COALESCE(TU.DownVotes, 0)) AS NetVotes
// FROM TopUsers TU LEFT JOIN ClosedPosts CP ON TU.UserId = CP.ClosedBy ORDER BY TU.Reputation DESC, CP.ClosedDate DESC;
//
// Rank reads only Reputation, so the ranked users are picked first and the posts x votes product is driven for those alone.
fn q1915(db: &'static So) -> String {
    let r = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let tu: MatSet<Id<User>> = rel(r.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let s = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some((t, v)) => [a[0] + (t == 2) as i64, a[1] + (t == 1) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64],
            None => a,
        });
    let PostHistory { user, post_history_type_id, post, creation_date: hd, .. } = &db.post_history;
    let closed_by: HashIdx<Id<User>, Id<PostHistory>> = db.post_history.with(post_history_type_id.eq(10)).select(user).inv().collect();
    let v = drain((&s).and((&closed_by).opt()));
    rows(v.into_iter().map(|(u, (a, h))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.extend(match h {
            Some(h) => [post_fields(db, post.get(h).unwrap(), &["title"]).pop().unwrap(), V::T(hd.get(h).unwrap())],
            None => [V::Null, V::Null],
        });
        f.push(V::I(a[2] - a[3]));
        row(f)
    }))
}

// WITH UserBadges AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PopularPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.Score, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.Id) AS VoteCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId
//     LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.ViewCount, p.Score
//     HAVING COUNT(c.Id) > 5 OR COUNT(DISTINCT v.Id) > 10),
// TopUsers AS (SELECT ub.UserId, ub.DisplayName, ub.BadgeCount, ROW_NUMBER() OVER (ORDER BY ub.BadgeCount DESC) AS Rank FROM UserBadges ub WHERE ub.BadgeCount > 0),
// EngagementMetrics AS (SELECT p.Id AS PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id)
// SELECT t.UserId, t.DisplayName, t.BadgeCount, pp.Title, pp.ViewCount, pp.Score, emp.UpVotes, emp.DownVotes
// FROM TopUsers t JOIN PopularPosts pp ON pp.PostId IN (SELECT DISTINCT PostId FROM EngagementMetrics em WHERE em.UpVotes - em.DownVotes > 10)
// JOIN EngagementMetrics emp ON pp.PostId = emp.PostId WHERE t.Rank <= 10 ORDER BY t.BadgeCount DESC, pp.ViewCount DESC;
//
// The first ON reads only pp, so the top users are crossed with the popular posts it keeps. Rank breaks a BadgeCount tie on the user id.
fn q7707(db: &'static So) -> String {
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let tu = rel(top_n(drain((&bc).filt(|n| n > 0)), |&(u, n)| (Reverse(n), u), 10));
    let recent = || db.post.with((&db.post.creation_date).gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30)));
    let cc = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let vc = recent().group_by(Ident::<Post>::new()).select(votes_of(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let em = recent().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let pp = rel(drain((&cc).and(&vc).filt(|(c, v): (i64, i64)| c > 5 || v > 10).and((&em).filt(|a| a[0] - a[1] > 10))));
    let v = drain((&tu).cross(&pp));
    rows(v.into_iter().map(|(_, ((u, n), (p, (_, a))))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.push(V::I(n));
        f.extend(post_fields(db, p, &["title", "views", "score"]));
        f.extend([V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH UserEngagement AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS QuestionCount, SUM(COALESCE(VUp.VoteCount, 0)) AS TotalUpVotes,
//        SUM(COALESCE(VDown.VoteCount, 0)) AS TotalDownVotes, SUM(COALESCE(C.CommentCount, 0)) AS TotalComments
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId AND P.PostTypeId = 1
//     LEFT JOIN (SELECT PostId, COUNT(*) AS VoteCount FROM Votes WHERE VoteTypeId = 2 GROUP BY PostId) VUp ON P.Id = VUp.PostId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS VoteCount FROM Votes WHERE VoteTypeId = 3 GROUP BY PostId) VDown ON P.Id = VDown.PostId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) C ON P.Id = C.PostId GROUP BY U.Id, U.DisplayName, U.Reputation),
// UserRanked AS (SELECT *, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS UserRank FROM UserEngagement)
// SELECT U.*, COALESCE(B.BadgeCount, 0) AS BadgeCount, CASE WHEN UserRank <= 10 THEN 'Top Contributor' WHEN Reputation >= 1000 THEN 'Experienced User' ELSE 'Newcomer' END AS UserStatus
// FROM UserRanked U LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId) B ON U.UserId = B.UserId WHERE U.TotalUpVotes > U.TotalDownVotes
// ORDER BY U.Reputation DESC LIMIT 100;
//
// UserRank breaks a Reputation tie on the user id (the SQL leaves it open). The derived tables are one row per post, so each question
// contributes its own counts.
fn q2748(db: &'static So) -> String {
    let Post { post_type_id, .. } = &db.post;
    let per_q = db.post.with(post_type_id.eq(1)).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
    });
    let cq = db.post.with(post_type_id.eq(1)).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ue = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&per_q).and(&cq)).opt())
        .fold([0i64; 4], |a, q| match q {
            Some((v, c)) => [a[0] + 1, a[1] + v[0], a[2] + v[1], a[3] + c],
            None => a,
        });
    let r = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), db.user.origid.get(u).unwrap()), 0);
    let rr = rel(r.into_iter().enumerate().map(|(i, (u, _))| (u, i as i64 + 1)).collect());
    let rank: HashIdx<Id<User>, i64> = (&rr).map(|(u, _)| u).inv().select((&rr).map(|(_, r)| r)).collect();
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&ue).filt(|a| a[1] > a[2]).and(&rank).and((&bc).opt()));
    let v = top_n(v, |&(u, _)| (Reverse(db.user.reputation.get(u).unwrap()), db.user.origid.get(u).unwrap()), 100);
    rows(v.into_iter().map(|(u, ((a, r), b))| {
        let rep = db.user.reputation.get(u).unwrap();
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(a.map(V::I));
        f.extend([V::I(r), V::I(b.unwrap_or(0)), V::S(if r <= 10 { "Top Contributor" } else if rep >= 1000 { "Experienced User" } else { "Newcomer" })]);
        row(f)
    }))
}

// WITH UserBadges AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// ActivePosts AS (SELECT p.OwnerUserId, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts, SUM(CASE WHEN p.CommentCount > 0 THEN 1 ELSE 0 END) AS CommentedPosts
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.OwnerUserId),
// UserStats AS (SELECT ub.UserId, ub.DisplayName, COALESCE(ap.TotalPosts, 0) AS TotalPosts, COALESCE(ap.PositivePosts, 0) AS PositivePosts, COALESCE(ap.CommentedPosts, 0) AS CommentedPosts,
//        ub.BadgeCount, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges FROM UserBadges ub LEFT JOIN ActivePosts ap ON ub.UserId = ap.OwnerUserId)
// SELECT us.DisplayName, us.TotalPosts, us.PositivePosts, us.CommentedPosts, us.BadgeCount, us.GoldBadges, us.SilverBadges, us.BronzeBadges,
//        RANK() OVER (ORDER BY us.BadgeCount DESC, us.TotalPosts DESC) AS UserRank
// FROM UserStats us WHERE us.TotalPosts > 5 ORDER BY UserRank;
fn q7697(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let Post { owner_user, creation_date, score, comment_count, .. } = &db.post;
    let ap = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(owner_user).select(score.and(comment_count)).fold([0i64; 3], |a, (s, c)| {
        [a[0] + 1, a[1] + (s > 0) as i64, a[2] + (c > 0) as i64]
    });
    let v = drain((&ap).filt(|a| a[0] > 5).and(&ub));
    let r = ranked(v, |&(_, (a, b))| (Reverse(b[0]), Reverse(a[0])), false);
    rows(r.into_iter().map(|((u, (a, b)), r)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(a.map(V::I));
        f.extend(b.map(V::I));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH UserBadgeCounts AS (SELECT UserId, COUNT(CASE WHEN Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN Class = 2 THEN 1 END) AS SilverBadges, COUNT(CASE WHEN Class = 3 THEN 1 END) AS BronzeBadges
//     FROM Badges GROUP BY UserId),
// PostStatistics AS (SELECT p.OwnerUserId, COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS Questions, COUNT(CASE WHEN p.PostTypeId = 2 AND p.AcceptedAnswerId IS NOT NULL THEN 1 END) AS AcceptedAnswers,
//        SUM(p.ViewCount) AS TotalViews FROM Posts p GROUP BY p.OwnerUserId),
// UserActivity AS (SELECT u.Id AS UserId, u.Reputation, u.DisplayName, COALESCE(ub.GoldBadges, 0) AS GoldBadges, COALESCE(ub.SilverBadges, 0) AS SilverBadges,
//        COALESCE(ub.BronzeBadges, 0) AS BronzeBadges, COALESCE(ps.Questions, 0) AS Questions, COALESCE(ps.AcceptedAnswers, 0) AS AcceptedAnswers, COALESCE(ps.TotalViews, 0) AS TotalViews,
//        ROW_NUMBER() OVER (ORDER BY u.Reputation DESC) AS Rank FROM Users u LEFT JOIN UserBadgeCounts ub ON u.Id = ub.UserId LEFT JOIN PostStatistics ps ON u.Id = ps.OwnerUserId)
// SELECT ua.UserId, ua.DisplayName, ua.Reputation, ua.GoldBadges, ua.SilverBadges, ua.BronzeBadges, ua.Questions, ua.AcceptedAnswers, ua.TotalViews, ua.Rank
// FROM UserActivity ua WHERE (ua.Reputation > 50 OR ua.GoldBadges > 0) AND ua.Questions > (SELECT AVG(Questions) FROM PostStatistics) ORDER BY ua.Rank LIMIT 10;
//
// Rank breaks a Reputation tie on the user id (the SQL leaves it open). The AVG runs over every PostStatistics group, the ownerless one included.
fn q3646(db: &'static So) -> String {
    let Post { owner_user, post_type_id, accepted_answer, view_count, .. } = &db.post;
    let fold = |a: [i64; 4], ((t, x), w): ((i64, Option<Id<Post>>), Option<i64>)| [a[0] + (t == 1) as i64, a[1] + (t == 2 && x.is_some()) as i64, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)];
    let ps = db.post.group_by(owner_user).select(post_type_id.and(accepted_answer.opt()).and(view_count.opt())).fold([0i64; 4], fold);
    let all = db.post.group_by(owner_user.opt()).select(post_type_id.and(accepted_answer.opt()).and(view_count.opt())).fold([0i64; 4], fold);
    let (n, s) = (&all).fold_flat((0i64, 0i64), |(n, s), a| (n + 1, s + a[0]));
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let r = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), db.user.origid.get(u).unwrap()), 0);
    let rr = rel(r.into_iter().enumerate().map(|(i, (u, _))| (u, i as i64 + 1)).collect());
    let rank: HashIdx<Id<User>, i64> = (&rr).map(|(u, _)| u).inv().select((&rr).map(|(_, r)| r)).collect();
    type R = (((i64, Option<[i64; 3]>), Option<[i64; 4]>), i64);
    let v = drain(db.user.select((&db.user.reputation).and((&ub).opt()).and((&ps).opt()).and(&rank)).filt(move |(((r, b), p), _): R| {
        (r > 50 || b.map_or(false, |b| b[0] > 0)) && p.map_or(0, |a| a[0]) as i128 * n as i128 > s as i128
    }));
    let v = top_n(v, |&(_, (_, r))| r, 10);
    rows(v.into_iter().map(|(u, (((rep, b), p), r))| {
        let b = b.unwrap_or([0; 3]);
        let p = p.unwrap_or([0; 4]);
        let mut f = ucols(db, u, &["uid", "name"]);
        f.push(V::I(rep));
        f.extend(b.map(V::I));
        f.extend([V::I(p[0]), V::I(p[1]), V::I(p[3]), V::I(r)]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(COALESCE(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END, 0)) AS PositiveScoreCount,
//        SUM(COALESCE(CASE WHEN P.Score < 0 THEN 1 ELSE 0 END, 0)) AS NegativeScoreCount, SUM(COALESCE(V.BountyAmount, 0)) AS TotalBounty,
//        DENSE_RANK() OVER (ORDER BY COUNT(DISTINCT P.Id) DESC) AS ActivityRank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId IN (8, 9) WHERE U.Reputation > 1000 GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, PositiveScoreCount, NegativeScoreCount, TotalBounty, ActivityRank FROM UserActivity WHERE ActivityRank <= 10),
// TagSummary AS (SELECT T.TagName, COUNT(DISTINCT P.Id) AS TagPostCount, SUM(P.ViewCount) AS TotalViews FROM Tags T JOIN Posts P ON P.Tags LIKE '%' || T.TagName || '%' GROUP BY T.TagName)
// SELECT TU.DisplayName, TU.Reputation, TU.PostCount, TU.PositiveScoreCount, TU.NegativeScoreCount, TU.TotalBounty, TS.TagName, TS.TagPostCount, TS.TotalViews
// FROM TopUsers TU LEFT JOIN TagSummary TS ON TS.TagPostCount > 0 ORDER BY TU.Reputation DESC, TU.PostCount DESC, TS.TotalViews DESC LIMIT 20;
//
// ActivityRank reads only the distinct post count, so only users whose count is among the ten highest distinct counts can reach dense rank
// 10; the posts x bounty product is driven for those alone. The ON reads only TS, so every top user is crossed with every tag summary
// (all have a post); `lex_top` takes the ORDER BY ... LIMIT over that product.
fn q4654(db: &'static So) -> String {
    let hi = || db.user.with((&db.user.reputation).gt(1000));
    let pc = hi().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let d = ranked(drain(&pc), |&(_, n)| Reverse(n), true);
    let tu: MatSet<Id<User>> = rel(d.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).in_v(vec![8, 9]))).select((&db.vote.bounty_amount).opt());
    let ua = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.score).and(bounty.opt())).opt())
        .fold([0i64; 3], |a, p| match p {
            Some((s, b)) => [a[0] + (s > 0) as i64, a[1] + (s < 0) as i64, a[2] + b.flatten().unwrap_or(0)],
            None => a,
        });
    let users = drain((&ua).and(&pc));
    let lt = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, Id<Post>> = (&lt).map(|(_, t)| t).inv().select((&lt).map(|(p, _)| p)).collect();
    let ts = db.tag.group_by(&db.tag.tag_name).select((&by_tag).select((&db.post.view_count).opt())).fold([0i64; 3], |a, w| [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0)]);
    let tags = drain((&ts).filt(|a| a[0] > 0));
    let v = cross_top(users, |&(u, (_, n))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(n), u), tags, |&(t, a)| (a[1] == 0, Reverse(a[2]), t), 20);
    rows(v.into_iter().map(|((u, (a, n)), (t, s))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::S(t), V::I(s[0]), nullable(s[2], s[1])]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U WHERE U.Reputation > 0),
// RecentPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, U.DisplayName AS OwnerDisplayName FROM Posts P LEFT JOIN Users U ON P.OwnerUserId = U.Id
//     WHERE P.CreationDate >= DATE '2024-10-01' - INTERVAL '30 days'),
// ClosedPosts AS (SELECT PH.PostId, PH.CreationDate, C.Name AS CloseReason FROM PostHistory PH JOIN CloseReasonTypes C ON CAST(PH.Comment AS INTEGER) = C.Id WHERE PH.PostHistoryTypeId = 10),
// PostStats AS (SELECT RP.PostId, COUNT(C.Id) AS CommentCount, SUM(V.BountyAmount) AS TotalBounties, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, MAX(RP.Score) AS MaxScore FROM RecentPosts RP LEFT JOIN Comments C ON RP.PostId = C.PostId
//     LEFT JOIN Votes V ON RP.PostId = V.PostId GROUP BY RP.PostId)
// SELECT UR.DisplayName, UR.Reputation, PS.PostId, PS.CommentCount, PS.TotalBounties, PS.UpVotes, PS.DownVotes, PS.MaxScore, CP.CloseReason
// FROM UserReputation UR JOIN PostStats PS ON UR.UserId = PS.PostId LEFT JOIN ClosedPosts CP ON PS.PostId = CP.PostId
// WHERE UR.ReputationRank <= 10 AND CP.CloseReason IS NOT NULL ORDER BY UR.Reputation DESC, PS.TotalBounties DESC;
//
// `UR.UserId = PS.PostId` joins a user id to a post id, so it goes through the raw ids. ReputationRank breaks a Reputation tie on the user
// id (the SQL leaves it open). The WHERE on CloseReason makes the ClosedPosts join inner.
fn q1504(db: &'static So) -> String {
    let top = top_n(drain(db.user.with((&db.user.reputation).gt(0)).select(&db.user.reputation)), |&(u, r)| (Reverse(r), db.user.origid.get(u).unwrap()), 10);
    let tu: MatSet<Id<User>> = rel(top.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let recent = Ident::<Post>::new().with((&db.post.creation_date).ge(add_days(date(2024, 10, 1), -30)));
    let rp: MatSet<Id<Post>> = (&tu).select((&db.user.origid).select(&pidx).select(recent)).collect();
    let ps = (&rp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select((&db.vote.vote_type_id).and((&db.vote.bounty_amount).opt())).opt()))
        .fold([0i64; 4], |a, (_, v)| match v {
            Some((t, b)) => [a[0] + b.is_some() as i64, a[1] + b.unwrap_or(0), a[2] + (t == 2) as i64, a[3] + (t == 3) as i64],
            None => a,
        });
    let cc = (&rp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let cr = || comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason);
    let closes: HashIdx<Id<Post>, Id<PostHistory>> = db.post_history.with(post_history_type_id.eq(10)).with(cr()).select(post).inv().collect();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let v = drain((&ps).and(&cc).and((&db.post.origid).select(&uidx)).and((&closes).select(cr())));
    let v = top_n(v, |&(p, (((a, _), u), r))| (Reverse(db.user.reputation.get(u).unwrap()), a[0] == 0, Reverse(a[1]), p, r), 0);
    rows(v.into_iter().map(|(p, (((a, c), u), r))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["id"]));
        f.extend([V::I(c), nullable(a[1], a[0]), V::I(a[2]), V::I(a[3])]);
        f.extend(post_fields(db, p, &["score"]));
        f.push(V::S(r));
        row(f)
    }))
}

// WITH UserVoteStats AS (SELECT U.Id AS UserId, U.DisplayName, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.DisplayName),
// PostStatistics AS (SELECT P.Id AS PostId, P.Title, COALESCE(COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END), 0) AS CommentCount,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) - COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS NetVoteScore,
//        ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) - COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) DESC) AS rn
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY P.Id, P.Title, P.OwnerUserId),
// TopPosts AS (SELECT PS.PostId, PS.Title, PS.CommentCount, PS.NetVoteScore, ROW_NUMBER() OVER (ORDER BY PS.NetVoteScore DESC, PS.CommentCount DESC) AS Rank FROM PostStatistics PS)
// SELECT U.DisplayName, U.UpVoteCount, U.DownVoteCount, TP.Title, TP.CommentCount, TP.NetVoteScore FROM UserVoteStats U JOIN TopPosts TP ON U.UserId = TP.PostId
// WHERE TP.Rank <= 10 ORDER BY U.DisplayName, TP.NetVoteScore DESC;
//
// `U.UserId = TP.PostId` joins a user id to a post id, so it goes through the raw ids. Rank breaks a tie on the post id (the SQL leaves it open).
fn q2344(db: &'static So) -> String {
    let ps = db
        .post
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64 - (t == Some(3)) as i64]);
    let top = top_n(drain(&ps), |&(p, a)| (Reverse(a[1]), Reverse(a[0]), p), 10);
    let tv = rel(top);
    type T = (Id<Post>, [i64; 2]);
    let tp: HashIdx<Id<Post>, [i64; 2]> = (&tv).map(|(p, _): T| p).inv().select((&tv).map(|(_, a): T| a)).collect();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let tu: MatSet<Id<User>> = db.post.with(&tp).select((&db.post.origid).select(&uidx)).collect();
    let uv = (&tu).group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = drain((&tp).and((&db.post.origid).select(&uidx).select(Ident::<User>::new().and(&uv))));
    rows(v.into_iter().map(|(p, (a, (u, b)))| {
        let mut f = vec![user_col(db, u, "name"), V::I(b[0]), V::I(b[1])];
        f.extend(post_fields(db, p, &["title"]));
        f.extend([V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("356", q356),
    ("7891", q7891),
    ("32048", q32048),
    ("200", q200),
    ("3301", q3301),
    ("6308", q6308),
    ("32456", q32456),
    ("928", q928),
    ("3125", q3125),
    ("2697", q2697),
    ("1385", q1385),
    ("1014", q1014),
    ("1380", q1380),
    ("6044", q6044),
    ("1793", q1793),
    ("2612", q2612),
    ("888", q888),
    ("3278", q3278),
    ("5309", q5309),
    ("26754", q26754),
    ("8474", q8474),
    ("5555", q5555),
    ("34765", q34765),
    ("24052", q24052),
    ("6177", q6177),
    ("30657", q30657),
    ("1078", q1078),
    ("2562", q2562),
    ("622", q622),
    ("1201", q1201),
    ("1937", q1937),
    ("2916", q2916),
    ("4888", q4888),
    ("8197", q8197),
    ("8812", q8812),
    ("29968", q29968),
    ("34698", q34698),
    ("398", q398),
    ("3234", q3234),
    ("6865", q6865),
    ("29291", q29291),
    ("1318", q1318),
    ("4153", q4153),
    ("8173", q8173),
    ("2513", q2513),
    ("6215", q6215),
    ("2764", q2764),
    ("6069", q6069),
    ("9381", q9381),
    ("2596", q2596),
    ("8950", q8950),
    ("26758", q26758),
    ("3879", q3879),
    ("28105", q28105),
    ("5772", q5772),
    ("1068", q1068),
    ("4626", q4626),
    ("8104", q8104),
    ("1820", q1820),
    ("5957", q5957),
    ("253", q253),
    ("7319", q7319),
    ("7501", q7501),
    ("27313", q27313),
    ("5254", q5254),
    ("6327", q6327),
    ("7045", q7045),
    ("33700", q33700),
    ("34387", q34387),
    ("1077", q1077),
    ("7313", q7313),
    ("8031", q8031),
    ("8873", q8873),
    ("9859", q9859),
    ("5935", q5935),
    ("8060", q8060),
    ("2248", q2248),
    ("9819", q9819),
    ("1832", q1832),
    ("3219", q3219),
    ("3407", q3407),
    ("26306", q26306),
    ("995", q995),
    ("31815", q31815),
    ("3369", q3369),
    ("5519", q5519),
    ("5913", q5913),
    ("7821", q7821),
    ("3629", q3629),
    ("180", q180),
    ("396", q396),
    ("3748", q3748),
    ("4752", q4752),
    ("6011", q6011),
    ("4631", q4631),
    ("1915", q1915),
    ("7707", q7707),
    ("2748", q2748),
    ("7697", q7697),
    ("3646", q3646),
    ("4654", q4654),
    ("1504", q1504),
    ("2344", q2344),
];
