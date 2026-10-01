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


// WITH UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldCount,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverCount, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeCount
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// TopPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS PostRank
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c GROUP BY c.PostId),
// UserTopScore AS (SELECT u.Id AS UserId, SUM(p.Score) AS TotalScore FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id)
// SELECT u.DisplayName, ub.BadgeCount, ub.GoldCount, ub.SilverCount, ub.BronzeCount, COUNT(tp.PostId) AS TopPostsCount, SUM(COALESCE(pc.CommentCount, 0)) AS TotalComments, ups.TotalScore
// FROM Users u JOIN UserBadges ub ON u.Id = ub.UserId LEFT JOIN TopPosts tp ON u.Id = (SELECT p.OwnerUserId FROM Posts p WHERE p.Id = tp.PostId)
// LEFT JOIN PostComments pc ON tp.PostId = pc.PostId LEFT JOIN UserTopScore ups ON u.Id = ups.UserId
// WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName, ub.BadgeCount, ub.GoldCount, ub.SilverCount, ub.BronzeCount, ups.TotalScore
// ORDER BY ub.BadgeCount DESC, ups.TotalScore DESC LIMIT 10;
//
// PostRank is never read, so TopPosts is every post of the last year.
fn q5617(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let Post { creation_date, score, .. } = &db.post;
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let recent = Ident::<Post>::new().with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let rich = || db.user.with((&db.user.reputation).gt(1000));
    let tp = rich().group_by(Ident::<User>::new()).select(posts_of(db).select(recent).select((&cc).opt()).opt()).fold([0i64; 2], |a, p| match p {
        Some(c) => [a[0] + 1, a[1] + c.unwrap_or(0)],
        None => a,
    });
    let ups = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(score)).fold(0i64, |n, s| n + s);
    let v = drain(rich().select((&ub).and(&tp).and((&ups).opt())));
    let v = top_n(v, |&(u, ((b, _), s))| (Reverse(b[0]), s.is_none(), Reverse(s), u), 10);
    rows(v.into_iter().map(|(u, ((b, t), s))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(b.map(V::I));
        f.extend([V::I(t[0]), V::I(t[1]), oint(s)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, COALESCE(v.UpVoteCount, 0) AS UpVoteCount, COALESCE(v.DownVoteCount, 0) AS DownVoteCount,
//        COALESCE(c.CommentCount, 0) AS CommentCount, COALESCE(b.BadgeCount, 0) AS BadgeCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY COALESCE(v.UpVoteCount, 0) DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p
//     LEFT JOIN (SELECT PostId, COUNT(CASE WHEN VoteTypeId = 2 THEN 1 END) AS UpVoteCount, COUNT(CASE WHEN VoteTypeId = 3 THEN 1 END) AS DownVoteCount FROM Votes GROUP BY PostId) v ON p.Id = v.PostId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
//     LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId) b ON p.OwnerUserId = b.UserId
//     WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days')
// SELECT r.PostId, r.Title, r.Body, r.CreationDate, r.UpVoteCount, r.DownVoteCount, r.CommentCount, r.BadgeCount, p.PostTypeId,
//        CASE WHEN r.Rank <= 10 THEN 'Top' WHEN r.Rank <= 20 THEN 'Mid' ELSE 'Low' END AS RankingCategory
// FROM RankedPosts r JOIN Posts p ON r.PostId = p.Id WHERE p.PostTypeId IN (1, 2) ORDER BY r.Rank;
//
// Rank is partitioned by PostTypeId, so the PostTypeId filter is applied before ranking.
fn q25120(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, .. } = &db.post;
    let rp = || db.post.with(creation_date.ge(add_days(date(2024, 10, 1), -30)));
    let vc = rp().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let cc = rp().group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db)).fold(0i64, |n, _| n + 1);
    let v = drain(rp().with(post_type_id.is_in([1, 2])).select(post_type_id.and((&vc).opt()).and((&cc).opt()).and(owner_user.select(&bc).opt())));
    let key = |x: &(Id<Post>, (((i64, Option<[i64; 2]>), Option<i64>), Option<i64>))| (Reverse(x.1 .0 .0 .1.map_or(0, |a| a[0])), Reverse(creation_date.get(x.0).unwrap()), x.0);
    let v = per_group(ranked(v, |x| (x.1 .0 .0 .0, key(x)), false), |x| x.1 .0 .0 .0);
    rows(v.into_iter().map(|((p, (((t, a), c), b)), r)| {
        let a = a.unwrap_or([0, 0]);
        let mut f = post_fields(db, p, &["id", "title", "body", "created"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c.unwrap_or(0)), V::I(b.unwrap_or(0)), V::I(t)]);
        f.push(V::S(if r <= 10 { "Top" } else if r <= 20 { "Mid" } else { "Low" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.AnswerCount, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank, p.OwnerUserId
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON u.Id = v.UserId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName),
// PostDetails AS (SELECT rp.PostId, rp.Title, rp.CreationDate, us.UserId, us.DisplayName, COALESCE(us.BadgeCount, 0) AS BadgeCount, COALESCE(us.TotalBounty, 0) AS TotalBounty, rp.PostRank
//     FROM RankedPosts rp LEFT JOIN UserStats us ON rp.OwnerUserId = us.UserId)
// SELECT pd.Title, pd.CreationDate, pd.DisplayName, pd.BadgeCount, pd.TotalBounty, (SELECT COUNT(*) FROM Comments c WHERE c.PostId = pd.PostId) AS CommentCount,
//        (SELECT SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) FROM Votes v WHERE v.PostId = pd.PostId) AS UpVoteCount,
//        (SELECT SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) FROM Votes v WHERE v.PostId = pd.PostId) AS DownVoteCount
// FROM PostDetails pd WHERE pd.PostRank = 1 ORDER BY pd.TotalBounty DESC LIMIT 10;
//
// The rank-1 posts are picked first; UserStats (the badges x votes product) is driven only for their owners.
fn q4496(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(owner_user.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let owners: MatSet<Id<User>> = (&tp).select(owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(1000)))).collect();
    let us = (&owners)
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold([0i64; 2], |a, (b, v)| [a[0] + b.is_some() as i64, a[1] + v.flatten().unwrap_or(0)]);
    let pv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let pc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let v = drain((&tp).select(owner_user.select(Ident::<User>::new().and(&us)).opt().and((&pc).opt()).and((&pv).opt())));
    let v = top_n(v, |&(p, ((u, _), _))| (Reverse(u.map_or(0, |(_, a)| a[1])), p), 10);
    rows(v.into_iter().map(|(p, ((u, c), a))| {
        let mut f = post_fields(db, p, &["title", "created"]);
        match u {
            Some((u, s)) => f.extend([user_col(db, u, "name"), V::I(s[0]), V::I(s[1])]),
            None => f.extend([V::Null, V::I(0), V::I(0)]),
        }
        f.push(V::I(c.unwrap_or(0)));
        f.extend(match a {
            Some(a) => [V::I(a[0]), V::I(a[1])],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS OwnerName, p.Score, p.ViewCount,
//        DENSE_RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'),
// TopRankedPosts AS (SELECT rp.* FROM RankedPosts rp WHERE rp.Rank <= 5),
// PostDetails AS (SELECT trp.PostId, trp.Title, trp.OwnerName, COALESCE(pc.CommentCount, 0) AS CommentCount, COALESCE(ac.AnswerCount, 0) AS AnswerCount, COALESCE(b.BadgeCount, 0) AS BadgeCount
//     FROM TopRankedPosts trp
//     LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) pc ON trp.PostId = pc.PostId
//     LEFT JOIN (SELECT ParentId AS PostId, COUNT(*) AS AnswerCount FROM Posts WHERE PostTypeId = 2 GROUP BY ParentId) ac ON trp.PostId = ac.PostId
//     LEFT JOIN (SELECT UserId, COUNT(DISTINCT Id) AS BadgeCount FROM Badges GROUP BY UserId) b ON trp.OwnerName = (SELECT DisplayName FROM Users WHERE Id = b.UserId))
// SELECT PD.PostId, PD.Title, PD.OwnerName, PD.CommentCount, PD.AnswerCount, PD.BadgeCount, COUNT(DISTINCT v.Id) AS VoteCount
// FROM PostDetails PD LEFT JOIN Votes v ON PD.PostId = v.PostId GROUP BY PD.PostId, PD.Title, PD.OwnerName, PD.CommentCount, PD.AnswerCount, PD.BadgeCount
// ORDER BY PD.BadgeCount DESC, PD.CommentCount DESC;
//
// The badge join matches every user with the owner's display name, so a post has one PostDetails row per such user; the GROUP BY keeps one per distinct BadgeCount.
fn q6843(db: &'static So) -> String {
    let Post { creation_date, owner_user, post_type_id, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1))).with(owner_user).select(post_type_id.and(creation_date)));
    let v = per_group(ranked(v, |&(_, (t, d))| (t, Reverse(d)), true), |&(_, (t, _))| t);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().filter(|x| x.1 <= 5).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db)).fold(0i64, |n, _| n + 1);
    let by_name: HashIdx<Str, i64> = (&db.user.display_name).inv().select(&bc).collect();
    type R = (Id<Post>, Option<i64>);
    let pd = rel(drain((&tp).select(owner_user.select(&db.user.display_name).select(&by_name).opt())));
    let g = (&pd).group_by(Same::<R>::new()).select(Same::<R>::new()).fold(0i64, |n, _| n + 1);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let ac = (&tp).group_by(Ident::<Post>::new()).select(answers_of(db)).fold(0i64, |n, _| n + 1);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db)).fold(0i64, |n, _| n + 1);
    let v = drain(&g);
    rows(v.into_iter().map(|((p, b), _)| {
        let mut f = post_fields(db, p, &["id", "title", "owner"]);
        f.extend([V::I(cc.get(p).unwrap_or(0)), V::I(ac.get(p).unwrap_or(0)), V::I(b.unwrap_or(0)), V::I(vc.get(p).unwrap_or(0))]);
        row(f)
    }))
}

// WITH UserBadges AS (SELECT U.Id AS UserId, U.DisplayName, B.Class, COUNT(*) AS BadgeCount FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName, B.Class),
// PopularPosts AS (SELECT P.Id AS PostId, P.Title, P.ViewCount, RANK() OVER (ORDER BY P.ViewCount DESC) AS ViewRank FROM Posts P
//     WHERE P.CreationDate >= CURRENT_DATE - INTERVAL '6 months' AND P.Score > 0),
// RecentComments AS (SELECT C.PostId, COUNT(C.Id) AS CommentCount FROM Comments C WHERE C.CreationDate >= CURRENT_DATE - INTERVAL '30 days' GROUP BY C.PostId),
// Combined AS (SELECT U.DisplayName AS UserName, P.Title AS PostTitle, COALESCE(RC.CommentCount, 0) AS RecentComments, COALESCE(UB.BadgeCount, 0) AS TotalBadges,
//        PB.Class AS BadgeClass, PP.ViewRank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN RecentComments RC ON P.Id = RC.PostId LEFT JOIN UserBadges UB ON U.Id = UB.UserId
//     LEFT JOIN PopularPosts PP ON P.Id = PP.PostId LEFT JOIN (SELECT DISTINCT Class FROM UserBadges) PB ON U.Id = UB.UserId WHERE U.Reputation > 100)
// SELECT UserName, PostTitle, RecentComments, TotalBadges, COALESCE(BadgeClass, 0) AS BadgeClass, ViewRank FROM Combined
// WHERE ViewRank <= 10 OR TotalBadges > 5 ORDER BY TotalBadges DESC, RecentComments DESC NULLS LAST;
//
// The PB join names only the left side (every user has a UserBadges row), so it is a cross join with the distinct classes, NULL included.
fn q21195(db: &'static So) -> String {
    let j: MatSet<(Id<User>, Option<Id<Badge>>)> = db.user.select(Ident::<User>::new().and(badges_of(db).opt())).collect();
    let user_of = (&j).map(|(u, _)| u);
    let badge_of = (&j).flat_map(|(_, b)| b);
    let ub = (&j).group_by((&user_of).and((&badge_of).select(&db.badge.class).opt())).select(Same::<(Id<User>, Option<Id<Badge>>)>::new()).fold(0i64, |n, _| n + 1);
    let ubv = rel(drain(&ub));
    let pb: MatSet<Option<i64>> = (&ubv).map(|((_, c), _)| c).collect();
    let pb = rel(drain(&pb).into_iter().map(|x| x.1).collect());
    let ub_of: HashIdx<Id<User>, ((Id<User>, Option<i64>), i64)> = (&ubv).map(|((u, _), _)| u).inv().select(&ubv).collect();
    let Post { creation_date, score, view_count, .. } = &db.post;
    let today = current_date();
    let pp = ranked(drain(db.post.with(creation_date.ge(add_months(today, -6)).and(score.gt(0))).select(view_count.opt())), |&(_, w)| (w.is_none(), Reverse(w)), false);
    let pp = rel(pp.into_iter().map(|((p, _), r)| (p, r)).collect());
    let vr: HashIdx<Id<Post>, i64> = (&pp).map(|(p, _)| p).inv().select(&pp).map(|(_, r)| r).collect();
    let rc = db.comment.with((&db.comment.creation_date).ge(add_days(today, -30))).group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let comb: MatSet<(Id<User>, Option<Id<Post>>, Option<i64>, i64)> = db
        .user
        .with((&db.user.reputation).gt(100))
        .select(Ident::<User>::new().and(posts_of(db).opt()).and(&ub_of))
        .map(|((u, p), ((_, c), n))| (u, p, c, n))
        .filt(|(_, p, _, n): (Id<User>, Option<Id<Post>>, Option<i64>, i64)| n > 5 || p.and_then(|p| vr.get(p)).map_or(false, |r| r <= 10))
        .collect();
    let mut out = Vec::new();
    (&comb).cross(&pb).drive(|_, ((u, p, _, n), c)| out.push((u, p, n, c)));
    rows(out.into_iter().map(|(u, p, n, c)| {
        row(vec![
            user_col(db, u, "name"),
            p.map_or(V::Null, |p| ostr(db.post.title.get(p))),
            V::I(p.and_then(|p| rc.get(p)).unwrap_or(0)),
            V::I(n),
            V::I(c.unwrap_or(0)),
            oint(p.and_then(|p| vr.get(p))),
        ])
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, P.AnswerCount, P.CommentCount, U.DisplayName AS OwnerDisplayName,
//        RANK() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC, P.CreationDate DESC) AS RankScore
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id WHERE P.CreationDate >= CURRENT_DATE - INTERVAL '30 days'),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, AnswerCount, CommentCount, OwnerDisplayName FROM RankedPosts WHERE RankScore <= 5),
// PostVoteStats AS (SELECT PostId, COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVotes FROM Votes V GROUP BY PostId)
// SELECT TP.PostId, TP.Title, TP.CreationDate, TP.Score, TP.ViewCount, TP.AnswerCount, TP.CommentCount, TP.OwnerDisplayName, COALESCE(PVS.UpVotes, 0) AS UpVotes,
//        COALESCE(PVS.DownVotes, 0) AS DownVotes, (COALESCE(PVS.UpVotes, 0) - COALESCE(PVS.DownVotes, 0)) AS NetVotes,
//        CASE WHEN COALESCE(PVS.UpVotes, 0) + COALESCE(PVS.DownVotes, 0) > 0 THEN COALESCE(PVS.UpVotes, 0)::FLOAT / (COALESCE(PVS.UpVotes, 0) + COALESCE(PVS.DownVotes, 0)) * 100 ELSE NULL END AS VotePercentage
// FROM TopPosts TP LEFT JOIN PostVoteStats PVS ON TP.PostId = PVS.PostId ORDER BY TP.Score DESC, TP.CreationDate DESC;
fn q31099(db: &'static So) -> String {
    let Post { creation_date, owner_user, post_type_id, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_days(current_date(), -30))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())), 5, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let v = drain((&tp).select((&pv).opt()));
    rows(v.into_iter().map(|(p, a)| {
        let [u, d] = a.unwrap_or([0, 0]);
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "owner"]);
        f.extend([V::I(u), V::I(d), V::I(u - d), if u + d > 0 { V::F(u as f64 / (u + d) as f64 * 100.0) } else { V::Null }]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, u.DisplayName AS OwnerDisplayName, p.CreationDate, p.Score,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// UserReputation AS (SELECT Id AS UserId, Reputation, CASE WHEN Reputation >= 1000 THEN 'High' WHEN Reputation BETWEEN 100 AND 999 THEN 'Medium' ELSE 'Low' END AS ReputationCategory FROM Users),
// PostStats AS (SELECT rp.PostId, rp.Title, rp.OwnerDisplayName, rp.CreationDate, rp.Score, ur.ReputationCategory, COALESCE(SUM(CASE WHEN v.VoteTypeId IN (2, 3) THEN 1 ELSE 0 END), 0) AS VoteCount
//     FROM RankedPosts rp LEFT JOIN Votes v ON rp.PostId = v.PostId JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId
//     GROUP BY rp.PostId, rp.Title, rp.OwnerDisplayName, rp.CreationDate, rp.Score, ur.ReputationCategory)
// SELECT ps.PostId, ps.Title, ps.OwnerDisplayName, ps.CreationDate, ps.Score, ps.ReputationCategory, ps.VoteCount, CASE WHEN ps.VoteCount > 10 THEN 'Popular' ELSE 'Less Popular' END AS Popularity
// FROM PostStats ps WHERE ps.Score > 0 AND ps.ReputationCategory = 'High' ORDER BY ps.VoteCount DESC, ps.CreationDate DESC LIMIT 10;
fn q3281(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let high = Ident::<User>::new().with((&db.user.reputation).ge(1000));
    let ps = db
        .post
        .with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(score.gt(0)))
        .with(owner_user.select(high))
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt())
        .fold(0i64, |n, t| n + matches!(t, Some(2 | 3)) as i64);
    let v = top_n(drain(&ps), |&(p, n)| (Reverse(n), Reverse(creation_date.get(p).unwrap()), p), 10);
    rows(v.into_iter().map(|(p, n)| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "score"]);
        f.extend([V::S("High"), V::I(n), V::S(if n > 10 { "Popular" } else { "Less Popular" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, COUNT(pc.Id) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate ASC) AS Rank
//     FROM Posts p LEFT JOIN Comments pc ON p.Id = pc.PostId WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.PostTypeId),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, rp.CommentCount FROM RankedPosts rp WHERE rp.Rank <= 10),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes,
//        COUNT(DISTINCT b.Id) AS BadgeCount FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName)
// SELECT tp.Title, tp.Score, tp.ViewCount, tp.AnswerCount, tp.CommentCount, ua.DisplayName AS TopUser, ua.TotalUpVotes, ua.TotalDownVotes, ua.BadgeCount
// FROM TopPosts tp JOIN Votes v ON tp.PostId = v.PostId JOIN UserActivity ua ON v.UserId = ua.UserId WHERE v.VoteTypeId = 2 ORDER BY tp.Score DESC, ua.TotalUpVotes DESC;
//
// UserActivity is computed only for the users who upvoted a top post, the only ones it is joined to.
fn q5753(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), creation_date.get(p).unwrap(), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let up = || votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2)));
    let voters: MatSet<Id<User>> = (&tp).select(up().select(&db.vote.user)).collect();
    let ua = (&voters)
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt().and(badges_of(db).opt()))
        .fold([0i64; 2], |a, (t, _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let bc = (&voters).group_by(Ident::<User>::new()).select(badges_of(db).opt()).buf_fold(distinct_some);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&tp).select((&cc).and(up().select(&db.vote.user).select(Ident::<User>::new().and(&ua).and(&bc)))));
    rows(v.into_iter().map(|(p, (c, ((u, a), b)))| {
        let mut f = post_fields(db, p, &["title", "score", "views", "answers"]);
        f.extend([V::I(c), user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(b)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.CreationDate, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName, u.Reputation, u.CreationDate),
// BadgeStats AS (SELECT b.UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverBadges, COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeBadges
//     FROM Badges b GROUP BY b.UserId),
// TotalStats AS (SELECT us.UserId, us.DisplayName, us.Reputation, us.CreationDate, us.PostCount, us.QuestionCount, us.AnswerCount, us.UpVoteCount, us.DownVoteCount,
//        COALESCE(bs.GoldBadges, 0) AS GoldBadges, COALESCE(bs.SilverBadges, 0) AS SilverBadges, COALESCE(bs.BronzeBadges, 0) AS BronzeBadges FROM UserStats us LEFT JOIN BadgeStats bs ON us.UserId = bs.UserId)
// SELECT *, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM TotalStats WHERE PostCount > 10 ORDER BY Reputation DESC, AnswerCount DESC, QuestionCount DESC;
fn q7700(db: &'static So) -> String {
    let pc = user_distinct_posts(db);
    let us = db
        .user
        .with((&pc).filt(|n| n > 10))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, p| {
            let (t, v) = p.map_or((0, None), |(t, v)| (t, v));
            [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64]
        });
    let bs = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let v = drain((&us).and(&pc).and((&bs).opt()));
    let v = ranked(v, |&(u, _)| Reverse(db.user.reputation.get(u).unwrap()), false);
    rows(v.into_iter().map(|((u, ((a, n), b)), r)| {
        let mut f = ucols(db, u, &["uid", "name", "rep", "ucreated"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.extend(b.unwrap_or([0; 3]).map(V::I));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH UserVoteStats AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 5 THEN 1 ELSE 0 END), 0) AS Favorites
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName),
// PostStats AS (SELECT p.Id AS PostId, p.Title, p.PostTypeId, COALESCE(SUM(CASE WHEN c.Id IS NOT NULL THEN 1 ELSE 0 END), 0) AS CommentCount,
//        COALESCE(SUM(CASE WHEN ph.Id IS NOT NULL THEN 1 ELSE 0 END), 0) AS HistoryEditCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId GROUP BY p.Id, p.Title, p.PostTypeId),
// RankedPosts AS (SELECT ps.PostId, ps.Title, ps.CommentCount, ps.HistoryEditCount, ROW_NUMBER() OVER (PARTITION BY ps.PostTypeId ORDER BY ps.CommentCount DESC, ps.HistoryEditCount DESC) AS Rank FROM PostStats ps)
// SELECT ups.UserId, ups.DisplayName, ups.UpVotes, ups.DownVotes, ups.Favorites, rp.PostId, rp.Title, rp.CommentCount, rp.HistoryEditCount
// FROM UserVoteStats ups LEFT JOIN RankedPosts rp ON ups.UserId = (SELECT p.OwnerUserId FROM Posts p WHERE p.Id = rp.PostId)
// WHERE ups.UpVotes > ups.DownVotes AND EXISTS (SELECT 1 FROM Posts p WHERE p.OwnerUserId = ups.UserId AND p.PostTypeId IN (1, 2)) ORDER BY ups.UpVotes DESC, ups.Favorites DESC;
//
// Rank is never read, so RankedPosts is every post; the comment x history product is driven only for the posts of the qualifying users.
fn q3340(db: &'static So) -> String {
    let uv = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + (t == Some(5)) as i64]
    });
    let qa = posts_of(db).select(Ident::<Post>::new().with((&db.post.post_type_id).is_in([1, 2])));
    let users: MatSet<Id<User>> = db.user.with((&uv).filt(|a| a[0] > a[1])).with(qa).collect();
    let posts: MatSet<Id<Post>> = (&users).select(posts_of(db)).collect();
    let ps = (&posts).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(history_of(db).opt())).fold([0i64; 2], |a, (c, h)| [a[0] + c.is_some() as i64, a[1] + h.is_some() as i64]);
    let v = drain((&users).select((&uv).and(posts_of(db).select(Ident::<Post>::new().and(&ps)).opt())));
    rows(v.into_iter().map(|(u, (a, p))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        f.extend(match p {
            Some((p, s)) => [V::I(db.post.origid.get(p).unwrap()), ostr(db.post.title.get(p)), V::I(s[0]), V::I(s[1])],
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, u.DisplayName AS Author,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS RankByScore
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, AnswerCount, CommentCount, Author FROM RankedPosts WHERE RankByScore <= 10),
// PostVoteSummary AS (SELECT PostId, SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes
//     FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY PostId)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.AnswerCount, tp.CommentCount, tp.Author, COALESCE(ps.UpVotes, 0) AS TotalUpVotes, COALESCE(ps.DownVotes, 0) AS TotalDownVotes,
//        CASE WHEN COALESCE(ps.UpVotes, 0) + COALESCE(ps.DownVotes, 0) > 0 THEN (COALESCE(ps.UpVotes, 0) * 1.0 / (COALESCE(ps.UpVotes, 0) + COALESCE(ps.DownVotes, 0))) * 100 ELSE 0 END AS UpVotePercentage
// FROM TopPosts tp LEFT JOIN PostVoteSummary ps ON tp.PostId = ps.PostId ORDER BY tp.Score DESC, tp.CreationDate DESC;
fn q9993(db: &'static So) -> String {
    let Post { creation_date, owner_user, post_type_id, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(vtype_name(db))).fold([0i64; 2], |a, n| [a[0] + (n == "UpMod") as i64, a[1] + (n == "DownMod") as i64]);
    let v = drain((&tp).select((&pv).opt()));
    rows(v.into_iter().map(|(p, a)| {
        let [u, d] = a.unwrap_or([0, 0]);
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "owner"]);
        f.extend([V::I(u), V::I(d), V::F(if u + d > 0 { u as f64 / (u + d) as f64 * 100.0 } else { 0.0 })]);
        row(f)
    }))
}

// WITH RECURSIVE UserPostCounts AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, ROW_NUMBER() OVER (ORDER BY COUNT(P.Id) DESC) AS Rank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId WHERE U.Reputation > 1000 GROUP BY U.Id, U.DisplayName),
// RecentVotes AS (SELECT V.UserId, COUNT(V.Id) AS VoteCount, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes
//     FROM Votes V WHERE V.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 month' GROUP BY V.UserId),
// PostHistorySummary AS (SELECT PH.UserId, COUNT(PH.Id) AS EditCount, SUM(CASE WHEN PH.PostHistoryTypeId IN (4, 5) THEN 1 ELSE 0 END) AS TitleOrBodyEdits,
//        COUNT(DISTINCT PH.PostId) AS DistinctPostsEdited FROM PostHistory PH GROUP BY PH.UserId)
// SELECT UPC.DisplayName, UPC.PostCount, UPC.QuestionCount, UPC.AnswerCount, RV.VoteCount, RV.DownVotes, RV.UpVotes, PHS.EditCount, PHS.TitleOrBodyEdits, PHS.DistinctPostsEdited
// FROM UserPostCounts UPC LEFT JOIN RecentVotes RV ON UPC.UserId = RV.UserId LEFT JOIN PostHistorySummary PHS ON UPC.UserId = PHS.UserId
// WHERE UPC.Rank <= 50 ORDER BY UPC.PostCount DESC, UPC.QuestionCount DESC LIMIT 100;
//
// WITH RECURSIVE, but no CTE refers to itself.
fn q31779(db: &'static So) -> String {
    let ups = user_posts(db);
    let v = top_n(drain(db.user.with((&db.user.reputation).gt(1000)).select(&ups)), |&(u, a)| (Reverse(a[1]), u), 50);
    let top: MatSet<Id<User>> = rel(v.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let Vote { user, creation_date, vote_type_id, .. } = &db.vote;
    let rv = db.vote.with(creation_date.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(user).select(vote_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 3) as i64, a[2] + (t == 2) as i64]);
    let PostHistory { user: hu, post_history_type_id, post, .. } = &db.post_history;
    let phs = db.post_history.group_by(hu).select(post_history_type_id).fold([0i64; 2], |a, t| [a[0] + 1, a[1] + matches!(t, 4 | 5) as i64]);
    let phd = db.post_history.group_by(hu).select(post).count_distinct();
    let v = drain((&top).select((&ups).and((&rv).opt()).and((&phs).and(&phd).opt())));
    let v = top_n(v, |&(u, ((a, _), _))| (Reverse(a[1]), Reverse(a[2]), u), 100);
    rows(v.into_iter().map(|(u, ((a, r), h))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[1]), V::I(a[2]), V::I(a[3])];
        f.extend(match r {
            Some(r) => r.map(V::I),
            None => [V::Null, V::Null, V::Null],
        });
        f.extend(match h {
            Some((h, d)) => [V::I(h[0]), V::I(h[1]), V::I(d)],
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RECURSIVE UserBadgeCounts AS (SELECT U.Id AS UserId, COUNT(B.Id) AS BadgeCount FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id),
// HighScoringPosts AS (SELECT P.Id AS PostId, P.Title, P.Score, P.OwnerUserId, RANK() OVER (PARTITION BY P.OwnerUserId ORDER BY P.Score DESC) AS PostRank FROM Posts P WHERE P.Score > 10),
// UserPostStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(H.PostId) AS TotalEdits, COALESCE(SUM(CASE WHEN H.PostHistoryTypeId IN (2, 4) THEN 1 ELSE 0 END), 0) AS BodyEdits,
//        COALESCE(SUM(CASE WHEN H.PostHistoryTypeId IN (1, 6) THEN 1 ELSE 0 END), 0) AS TitleEdits FROM Users U LEFT JOIN PostHistory H ON U.Id = H.UserId WHERE U.Reputation > 1000 GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT U.Id, U.DisplayName, UB.BadgeCount, UPS.TotalEdits, UPS.BodyEdits, UPS.TitleEdits FROM Users U JOIN UserBadgeCounts UB ON U.Id = UB.UserId JOIN UserPostStats UPS ON U.Id = UPS.UserId
//     WHERE UB.BadgeCount > 5)
// SELECT U.DisplayName, U.BadgeCount, U.TotalEdits, U.BodyEdits, U.TitleEdits, COUNT(DISTINCT P.PostId) AS HighScorePostsCount
// FROM TopUsers U LEFT JOIN HighScoringPosts P ON U.Id = P.OwnerUserId GROUP BY U.DisplayName, U.BadgeCount, U.TotalEdits, U.BodyEdits, U.TitleEdits
// ORDER BY HighScorePostsCount DESC, U.BadgeCount DESC;
//
// WITH RECURSIVE, but no CTE refers to itself. The final GROUP BY is on the column tuple, not the user, so it is the group key.
fn q33563(db: &'static So) -> String {
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db)).fold(0i64, |n, _| n + 1);
    let PostHistory { user, post_history_type_id, .. } = &db.post_history;
    let hist: HashIdx<Id<User>, Id<PostHistory>> = db.post_history.select(user).inv().collect();
    let ups = db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new()).select((&hist).select(post_history_type_id).opt()).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + matches!(t, 2 | 4) as i64, a[2] + matches!(t, 1 | 6) as i64],
        None => a,
    });
    let high = posts_of(db).select(Ident::<Post>::new().with((&db.post.score).gt(10)));
    let g = db
        .user
        .with((&bc).filt(|n| n > 5))
        .group_by((&db.user.display_name).and(&bc).and(&ups))
        .select(high.opt())
        .buf_fold(distinct_some);
    let v = drain(&g);
    rows(v.into_iter().map(|(((n, b), a), c)| row(vec![V::S(n), V::I(b), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(c)])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS OwnerRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// PostVoteCounts AS (SELECT PostId, COUNT(CASE WHEN vt.Name = 'UpMod' THEN 1 END) AS UpvoteCount, COUNT(CASE WHEN vt.Name = 'DownMod' THEN 1 END) AS DownvoteCount
//     FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY PostId),
// PostHistoryCount AS (SELECT PostId, COUNT(*) AS EditCount FROM PostHistory WHERE PostHistoryTypeId IN (4, 5, 6) GROUP BY PostId)
// SELECT rp.PostId, rp.Title, rp.OwnerDisplayName, rp.Score, COALESCE(phc.EditCount, 0) AS EditCount, COALESCE(pvc.UpvoteCount, 0) AS UpvoteCount, COALESCE(pvc.DownvoteCount, 0) AS DownvoteCount,
//        CASE WHEN COALESCE(phc.EditCount, 0) > 10 THEN 'Highly Edited' WHEN COALESCE(phc.EditCount, 0) > 5 THEN 'Moderately Edited' ELSE 'Rarely Edited' END AS EditLevel,
//        CASE WHEN rp.OwnerRank = 1 THEN 'Top Post' ELSE 'Other Post' END AS PostStatus
// FROM RankedPosts rp LEFT JOIN PostHistoryCount phc ON rp.PostId = phc.PostId LEFT JOIN PostVoteCounts pvc ON rp.PostId = pvc.PostId WHERE rp.Score > 10 ORDER BY rp.Score DESC, rp.ViewCount DESC;
fn q1909(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 1, false);
    let first: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let rp = || db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(score.gt(10))).with(owner_user);
    let edit = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([4, 5, 6])));
    let ec = rp().group_by(Ident::<Post>::new()).select(edit.opt()).fold(0i64, |n, h| n + h.is_some() as i64);
    let vc = rp().group_by(Ident::<Post>::new()).select(votes_of(db).select(vtype_name(db)).opt()).fold([0i64; 2], |a, n| [a[0] + (n == Some("UpMod")) as i64, a[1] + (n == Some("DownMod")) as i64]);
    let v = drain((&ec).and(&vc).and(Ident::<Post>::new().with(&first).opt()));
    rows(v.into_iter().map(|(p, ((e, a), t))| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "score"]);
        f.extend([V::I(e), V::I(a[0]), V::I(a[1])]);
        f.push(V::S(if e > 10 { "Highly Edited" } else if e > 5 { "Moderately Edited" } else { "Rarely Edited" }));
        f.push(V::S(if t.is_some() { "Top Post" } else { "Other Post" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RankByUser
//     FROM Posts p WHERE p.PostTypeId = 1),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(DISTINCT p.Id) AS TotalQuestions FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId AND p.PostTypeId = 1 GROUP BY u.Id, u.Reputation),
// HighRepUsers AS (SELECT UserId, Reputation FROM UserReputation WHERE Reputation > 1000),
// PostActivity AS (SELECT p.Id AS PostId, p.ViewCount, COALESCE(c.CommentCount, 0) AS CommentCount FROM Posts p LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId)
// SELECT up.UserId, up.Reputation, rp.Title, rp.CreationDate, pa.ViewCount, pa.CommentCount, CASE WHEN pa.ViewCount > 100 THEN 'High Views' ELSE 'Low Views' END AS ViewStatus,
//        (SELECT MAX(Score) FROM Posts WHERE OwnerUserId = up.UserId AND PostTypeId = 2) AS MaxAnswerScore
// FROM HighRepUsers up JOIN RankedPosts rp ON up.UserId = rp.PostId JOIN PostActivity pa ON rp.PostId = pa.PostId WHERE rp.RankByUser <= 3
// ORDER BY up.Reputation DESC, rp.CreationDate ASC OFFSET 5 ROWS;
//
// `up.UserId = rp.PostId` joins a user id to a post id, so it goes through the raw ids.
fn q830(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, view_count, score, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1)).select(owner_user.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 3, false);
    let rp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let by_raw: HashIdx<i64, Id<Post>> = (&rp).select(&db.post.origid).inv().collect();
    let cc = (&rp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ma = db.post.with(post_type_id.eq(2)).group_by(owner_user).select(score).fold(i64::MIN, |m, s| m.max(s));
    let v = drain(db.user.with((&db.user.reputation).gt(1000)).select((&db.user.origid).select(&by_raw).select(Ident::<Post>::new().and(&cc)).and((&ma).opt())));
    let v = top_n(v, |&(u, ((p, _), _))| (Reverse(db.user.reputation.get(u).unwrap()), creation_date.get(p).unwrap(), u), 0);
    rows(v.into_iter().skip(5).map(|(u, ((p, c), m))| {
        let w = view_count.get(p);
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend(post_fields(db, p, &["title", "created", "views"]));
        f.extend([V::I(c), V::S(if w.map_or(false, |w| w > 100) { "High Views" } else { "Low Views" }), oint(m)]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS TotalQuestions,
//        COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS TotalAnswers, SUM(p.ViewCount) AS TotalViews, COUNT(DISTINCT b.Id) AS TotalBadges, COALESCE(ROUND(AVG(v.BountyAmount), 2), 0) AS AvgBounty
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (8, 9) GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalViews, TotalBadges, AvgBounty, RANK() OVER (ORDER BY TotalPosts DESC, TotalViews DESC) AS UserRank FROM UserPostStats)
// SELECT u.UserId, u.DisplayName, u.TotalPosts, u.TotalQuestions, u.TotalAnswers, u.TotalViews, u.TotalBadges, u.AvgBounty,
//        COALESCE(ROUND((CAST(u.TotalPosts AS decimal) / NULLIF(u.TotalViews, 0)) * 100, 2), 0) AS PostViewRatio,
//        CASE WHEN u.TotalBadges >= 5 THEN 'Experienced' WHEN u.TotalBadges >= 3 THEN 'Moderate' ELSE 'Novice' END AS UserExperience, p.CreationDate, pt.Name AS PostType
// FROM TopUsers u LEFT JOIN Posts p ON u.UserId = p.OwnerUserId LEFT JOIN PostTypes pt ON p.PostTypeId = pt.Id WHERE u.UserRank <= 10 ORDER BY u.UserRank, u.TotalViews DESC;
//
// UserRank leads with the distinct post count, so only users with at least the tenth-highest count can rank in the top ten; the posts x badges x votes product is driven for those alone.
fn q1966(db: &'static So) -> String {
    let pc = user_distinct_posts(db);
    let tenth = top_n(drain(&pc), |&(u, n)| (Reverse(n), u), 10).last().unwrap().1;
    let cand: MatSet<Id<User>> = db.user.with((&pc).filt(|n| n >= tenth)).collect();
    let Post { post_type_id, view_count, .. } = &db.post;
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt());
    let s = (&cand)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(view_count.opt().and(bounty.opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 4], |a, (p, _)| {
            let (w, b) = p.map_or((None, None), |(w, b)| (w, b.flatten()));
            [a[0] + w.is_some() as i64, a[1] + w.unwrap_or(0), a[2] + b.is_some() as i64, a[3] + b.unwrap_or(0)]
        });
    let qc = (&cand).group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(1)) as i64, a[1] + (t == Some(2)) as i64]);
    let bc = (&cand).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = ranked(drain((&s).and(&pc).and(&qc).and(&bc)), |&(_, (((a, n), _), _))| (Reverse(n), a[0] == 0, Reverse(a[1])), false);
    let top: Vec<_> = v.into_iter().take_while(|x| x.1 <= 10).collect();
    let tu = rel(top.into_iter().map(|((u, x), _)| (u, x)).collect());
    let v = drain((&tu).select(Same::<(Id<User>, ((([i64; 4], i64), [i64; 2]), i64))>::new().and(Same::<(Id<User>, ((([i64; 4], i64), [i64; 2]), i64))>::new().map(|(u, _)| u).select(posts_of(db).opt()))));
    rows(v.into_iter().map(|(_, ((u, (((a, n), q), b)), p))| {
        let views = if a[0] == 0 { None } else { Some(a[1]) };
        let ab = if a[2] == 0 { 0.0 } else { (a[3] as f64 / a[2] as f64 * 100.0).round() / 100.0 };
        let ratio = match views {
            Some(w) if w != 0 => (n as f64 / w as f64 * 100.0 * 100.0).round() / 100.0,
            _ => 0.0,
        };
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(q[0]), V::I(q[1]), oint(views), V::I(b), V::F(ab), V::F(ratio)]);
        f.push(V::S(if b >= 5 { "Experienced" } else if b >= 3 { "Moderate" } else { "Novice" }));
        f.extend(match p {
            Some(p) => post_fields(db, p, &["created", "type"]),
            None => vec![V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn,
//        COUNT(c.Id) AS CommentCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.OwnerUserId, p.CreationDate, p.Score, p.ViewCount),
// UserBadges AS (SELECT b.UserId, COUNT(b.Id) AS TotalBadges, MAX(b.Class) AS HighestBadgeClass FROM Badges b GROUP BY b.UserId),
// PostVoteStats AS (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes FROM Votes v GROUP BY v.PostId)
// SELECT up.DisplayName AS UserName, rp.PostId, rp.Title, rp.Score AS PostScore, rp.ViewCount, rp.CommentCount, ubs.TotalBadges, ubs.HighestBadgeClass, COALESCE(pvs.Upvotes, 0) AS TotalUpvotes,
//        COALESCE(pvs.Downvotes, 0) AS TotalDownvotes, CASE WHEN rp.Score > 10 THEN 'High Score' WHEN rp.Score BETWEEN 1 AND 10 THEN 'Medium Score' ELSE 'Low Score' END AS ScoreCategory
// FROM RankedPosts rp JOIN Users up ON rp.OwnerUserId = up.Id LEFT JOIN UserBadges ubs ON up.Id = ubs.UserId LEFT JOIN PostVoteStats pvs ON rp.PostId = pvs.PostId
// WHERE rp.rn = 1 ORDER BY rp.CreationDate DESC LIMIT 50 OFFSET 0;
fn q31629(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(current_date(), -1))).with(owner_user).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let pv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold((0i64, 0i64), |(n, m), c| (n + 1, m.max(c)));
    let v = drain((&cc).and(owner_user.select((&ub).opt())).and((&pv).opt()));
    let v = top_n(v, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 50);
    rows(v.into_iter().map(|(p, ((c, b), a))| {
        let s = score.get(p).unwrap();
        let [u, d] = a.unwrap_or([0, 0]);
        let mut f = post_fields(db, p, &["owner", "id", "title", "score", "views"]);
        f.push(V::I(c));
        f.extend(match b {
            Some((n, m)) => [V::I(n), V::I(m)],
            None => [V::Null, V::Null],
        });
        f.extend([V::I(u), V::I(d), V::S(if s > 10 { "High Score" } else if s >= 1 { "Medium Score" } else { "Low Score" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, U.DisplayName AS OwnerDisplayName, COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVoteCount,
//        COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVoteCount, ROW_NUMBER() OVER (PARTITION BY P.Id ORDER BY P.Score DESC) AS Rank
//     FROM Posts P LEFT JOIN Users U ON P.OwnerUserId = U.Id LEFT JOIN Votes V ON P.Id = V.PostId WHERE P.CreationDate >= timestamp '2024-10-01 12:34:56' - INTERVAL '1 year'
//     GROUP BY P.Id, P.Title, P.CreationDate, P.Score, P.ViewCount, U.DisplayName),
// ClosedPosts AS (SELECT PH.PostId, PH.CreationDate, C.Name AS CloseReason, ROW_NUMBER() OVER (PARTITION BY PH.PostId ORDER BY PH.CreationDate DESC) AS CloseRank
//     FROM PostHistory PH JOIN CloseReasonTypes C ON CAST(PH.Comment AS integer) = C.Id WHERE PH.PostHistoryTypeId IN (10, 11)),
// TopClosedPosts AS (SELECT RP.PostId, RP.Title, RP.OwnerDisplayName, COALESCE(CT.CloseReason, 'Not Closed') AS CloseReason FROM RankedPosts RP
//     LEFT JOIN ClosedPosts CT ON RP.PostId = CT.PostId AND CT.CloseRank = 1 WHERE RP.Rank <= 10)
// SELECT TCP.Title, TCP.OwnerDisplayName, COALESCE(TCP.CloseReason, 'Active') AS Status, RP.ViewCount, RP.UpVoteCount, RP.DownVoteCount
// FROM TopClosedPosts TCP JOIN RankedPosts RP ON TCP.PostId = RP.PostId ORDER BY RP.Score DESC, TCP.Title;
//
// Rank is partitioned by the post, so it is 1 for every post. A close tie on CreationDate goes to the larger history id.
fn q4387(db: &'static So) -> String {
    let rp = || db.post.with((&db.post.creation_date).ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let vc = rp().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post, post_history_type_id, comment, creation_date: hd, .. } = &db.post_history;
    let cl = drain(db.post_history.with(post_history_type_id.in_v(vec![10, 11])).select(post.and(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason))));
    let cl = top_per(cl, |&(_, (p, _))| p, |&(h, _)| (Reverse(hd.get(h).unwrap()), Reverse(h)), 1, false);
    let cl = rel(cl.into_iter().map(|(_, pr)| pr).collect());
    let last: HashIdx<Id<Post>, Str> = (&cl).map(|(p, _)| p).inv().select(&cl).map(|(_, r)| r).collect();
    let v = drain((&vc).and((&last).opt()));
    rows(v.into_iter().map(|(p, (a, r))| {
        let mut f = post_fields(db, p, &["title", "owner"]);
        f.extend([V::S(r.unwrap_or("Not Closed")), oint(db.post.view_count.get(p)), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT Id, DisplayName, Reputation, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Ranking FROM Users),
// PostStatistics AS (SELECT OwnerUserId, COUNT(CASE WHEN PostTypeId = 1 THEN 1 END) AS QuestionCount, COUNT(CASE WHEN PostTypeId = 2 THEN 1 END) AS AnswerCount, SUM(ViewCount) AS TotalViews,
//        SUM(Score) AS TotalScore FROM Posts GROUP BY OwnerUserId),
// RecentPosts AS (SELECT p.Id, p.Title, p.CreationDate, CASE WHEN p.ClosedDate IS NOT NULL THEN 'Closed' ELSE 'Open' END AS PostStatus FROM Posts p
//     WHERE p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// HighlyActiveUsers AS (SELECT u.Id, u.DisplayName, u.Reputation FROM UserReputation u JOIN PostStatistics ps ON u.Id = ps.OwnerUserId WHERE ps.QuestionCount + ps.AnswerCount > 10),
// ClosedPostReasons AS (SELECT ph.PostId, ph.Comment AS CloseReason, ph.CreationDate AS CloseDate, u.DisplayName AS ClosedBy FROM PostHistory ph JOIN Users u ON ph.UserId = u.Id WHERE ph.PostHistoryTypeId = 10)
// SELECT u.DisplayName AS UserName, ps.QuestionCount, ps.AnswerCount, ps.TotalViews, ps.TotalScore, rp.Title AS RecentPostTitle, rp.PostStatus, CPR.CloseReason, CPR.CloseDate, CPR.ClosedBy
// FROM HighlyActiveUsers u LEFT JOIN PostStatistics ps ON u.Id = ps.OwnerUserId LEFT JOIN RecentPosts rp ON ps.OwnerUserId = rp.Id LEFT JOIN ClosedPostReasons CPR ON CPR.PostId = rp.Id
// WHERE ps.TotalViews > 1000 ORDER BY ps.TotalScore DESC, ps.TotalViews DESC;
//
// `ps.OwnerUserId = rp.Id` joins a user id to a post id, so it goes through the raw ids.
fn q3878(db: &'static So) -> String {
    let Post { owner_user, post_type_id, view_count, score, creation_date, closed_date, .. } = &db.post;
    let ps = db.post.group_by(owner_user).select(post_type_id.and(view_count.opt()).and(score)).fold([0i64; 5], |a, ((t, w), s)| {
        [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + s]
    });
    let recent: HashIdx<i64, Id<Post>> = db.post.with(creation_date.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(&db.post.origid).inv().collect();
    let PostHistory { post, post_history_type_id, user, .. } = &db.post_history;
    let cpr: HashIdx<Id<Post>, Id<PostHistory>> = db.post_history.with(post_history_type_id.eq(10)).with(user).select(post).inv().collect();
    let v = drain(db.user.select((&ps).filt(|a| a[0] + a[1] > 10 && a[2] > 0 && a[3] > 1000).and((&db.user.origid).select(&recent).select(Ident::<Post>::new().and((&cpr).opt())).opt())));
    rows(v.into_iter().map(|(u, (a, r))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[3]), V::I(a[4])];
        match r {
            Some((p, h)) => {
                f.extend([ostr(db.post.title.get(p)), V::S(if closed_date.get(p).is_some() { "Closed" } else { "Open" })]);
                f.extend(match h {
                    Some(h) => [ostr(db.post_history.comment.get(h)), V::T(db.post_history.creation_date.get(h).unwrap()), user_col(db, user.get(h).unwrap(), "name")],
                    None => [V::Null, V::Null, V::Null],
                });
            }
            None => f.extend([V::Null, V::Null, V::Null, V::Null, V::Null]),
        }
        row(f)
    }))
}

// Rewritten (rewrites/9306.sql): the ROW_NUMBER order is tie-broken on p.Id.
// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC, p.Id) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.PostTypeId IN (1, 2)),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.OwnerDisplayName, rp.Score, rp.ViewCount FROM RankedPosts rp WHERE rp.Rank <= 10),
// PostVotes AS (SELECT p.Id AS PostId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN v.Id END) AS UpVotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN v.Id END) AS DownVotes FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id),
// PostBadges AS (SELECT p.OwnerUserId AS UserId, COUNT(b.Id) AS BadgeCount FROM Badges b JOIN Posts p ON b.UserId = p.OwnerUserId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.OwnerUserId)
// SELECT tp.Title, tp.OwnerDisplayName, tp.Score, tp.ViewCount, pv.UpVotes, pv.DownVotes, COALESCE(pb.BadgeCount, 0) AS BadgeCount
// FROM TopPosts tp LEFT JOIN PostVotes pv ON tp.PostId = pv.PostId LEFT JOIN PostBadges pb ON tp.OwnerDisplayName = (SELECT u.DisplayName FROM Users u WHERE u.Id = pb.UserId) ORDER BY tp.Score DESC;
//
// PostBadges counts each badge once per recent post of its owner (the JOIN Posts multiplies), and is matched by display name, so a post gets one row per user of that name.
fn q9306(db: &'static So) -> String {
    let Post { creation_date, owner_user, post_type_id, score, view_count, .. } = &db.post;
    let since = add_years(ts(2024, 10, 1, 12, 34, 56), -1);
    let v = drain(db.post.with(creation_date.ge(since).and(post_type_id.is_in([1, 2]))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let pb = db.post.with(creation_date.ge(since)).group_by(owner_user).select(owner_user.select(badges_of(db))).fold(0i64, |n, _| n + 1);
    let by_name: HashIdx<Str, i64> = (&db.user.display_name).inv().select(&pb).collect();
    let v = drain((&pv).and(owner_user.select(&db.user.display_name).select(&by_name).opt()));
    rows(v.into_iter().map(|(p, (a, b))| {
        let mut f = post_fields(db, p, &["title", "owner", "score", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(b.unwrap_or(0))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank,
//        COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount, MAX(b.Date) OVER (PARTITION BY p.OwnerUserId) AS LatestBadge
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Badges b ON p.OwnerUserId = b.UserId AND b.Class = 1
//     WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year' AND p.PostTypeId = 1),
// FilteredRankedPosts AS (SELECT rp.PostId, rp.Title, rp.ViewCount, rp.PostRank, rp.CommentCount, COALESCE(DATE_PART('epoch', rp.LatestBadge), 0) AS BadgeTimestamp FROM RankedPosts rp
//     WHERE rp.PostRank = 1 OR rp.CommentCount > 5),
// AggregatedVotes AS (SELECT p.Id AS PostId, COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 2) AS UpVoteCount, COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 3) AS DownVoteCount
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id)
// SELECT f.PostId, f.Title, f.ViewCount, v.UpVoteCount, v.DownVoteCount, CASE WHEN f.BadgeTimestamp > 0 THEN 'Gold Badge Owner' ELSE 'No Gold Badge' END AS BadgeStatus
// FROM FilteredRankedPosts f JOIN AggregatedVotes v ON f.PostId = v.PostId WHERE (v.UpVoteCount - v.DownVoteCount) > 10 ORDER BY f.ViewCount DESC OFFSET 0 ROWS FETCH NEXT 10 ROWS ONLY;
//
// The windows run over the joined rows (post x comment x owner's gold badge). PostRank = 1 is one joined row of the owner's newest question; which one is
// DuckDB's choice, but every joined row of a post projects the same columns, so the port takes the smallest.
fn q20231(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, view_count, .. } = &db.post;
    type J = (Id<Post>, Option<Id<Comment>>, Option<Id<Badge>>);
    let gold = owner_user.select(badges_of(db).select(Ident::<Badge>::new().with((&db.badge.class).eq(1))));
    let j: MatSet<J> = db
        .post
        .with(creation_date.ge(add_years(date(2024, 10, 1), -1)).and(post_type_id.eq(1)))
        .select(Ident::<Post>::new().and(comments_of(db).opt()).and(gold.opt()))
        .map(|((p, c), b)| (p, c, b))
        .collect();
    let post_of = (&j).map(|(p, _, _)| p);
    let cc = (&j).group_by(&post_of).select(Same::<J>::new()).fold(0i64, |n, (_, c, _)| n + c.is_some() as i64);
    let first = top_per(drain((&j).select((&post_of).select(owner_user.opt()))), |&(_, u)| u, |&(r, _)| (Reverse(creation_date.get(r.0).unwrap()), r), 1, false);
    let first: MatSet<J> = rel(first.into_iter().map(|x| x.0).collect()).map(|r| r).collect();
    let keep = (&j).select(Same::<J>::new().with(&first).opt().and((&post_of).select(&cc))).filt(|(r, n): (Option<J>, i64)| r.is_some() || n > 5);
    let av = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let has_gold: MatSet<Id<User>> = db.badge.with((&db.badge.class).eq(1)).select(&db.badge.user).collect();
    let v = drain((&j).with(keep).select((&post_of).select((&av).filt(|a| a[0] - a[1] > 10))));
    let v = top_n(v, |&((p, c, b), _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), p, c, b)
    }, 10);
    rows(v.into_iter().map(|((p, _, _), a)| {
        let g = owner_user.get(p).map_or(false, |u| has_gold.member(u));
        let mut f = post_fields(db, p, &["id", "title", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::S(if g { "Gold Badge Owner" } else { "No Gold Badge" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, u.DisplayName AS OwnerDisplayName, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '1 year' AND p.Score > 10),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.OwnerDisplayName, COALESCE(c.CommentCount, 0) AS CommentCount, COALESCE(v.UpVoteCount, 0) AS UpVoteCount FROM RankedPosts rp
//     LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON rp.PostId = c.PostId
//     LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount FROM Votes GROUP BY PostId) v ON rp.PostId = v.PostId WHERE rp.rn = 1),
// FinalResults AS (SELECT tp.Title, tp.CreationDate, tp.OwnerDisplayName, tp.CommentCount, tp.UpVoteCount, CASE WHEN tp.UpVoteCount > 20 THEN 'Hot Post' ELSE 'Regular Post' END AS PostStatus FROM TopPosts tp)
// SELECT f.Title, f.CreationDate, f.OwnerDisplayName, f.CommentCount, f.UpVoteCount, f.PostStatus FROM FinalResults f ORDER BY f.UpVoteCount DESC, f.CreationDate DESC LIMIT 10;
fn q1362(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(current_date(), -1)).and(score.gt(10))).with(owner_user).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let uv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold(0i64, |n, t| n + (t == 2) as i64);
    let v = drain((&tp).select((&cc).opt().and((&uv).opt())));
    let v = top_n(v, |&(p, (_, u))| (Reverse(u.unwrap_or(0)), Reverse(creation_date.get(p).unwrap()), p), 10);
    rows(v.into_iter().map(|(p, (c, u))| {
        let u = u.unwrap_or(0);
        let mut f = post_fields(db, p, &["title", "created", "owner"]);
        f.extend([V::I(c.unwrap_or(0)), V::I(u), V::S(if u > 20 { "Hot Post" } else { "Regular Post" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.AnswerCount, U.DisplayName AS OwnerName, RANK() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC) AS PostRank
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// RecentVotes AS (SELECT V.PostId, COUNT(*) AS VoteCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
//     FROM Votes V WHERE V.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 month' GROUP BY V.PostId),
// PostStatistics AS (SELECT RP.PostId, RP.Title, RP.CreationDate, RP.Score, RP.AnswerCount, RP.OwnerName, COALESCE(RV.VoteCount, 0) AS RecentVoteCount, COALESCE(RV.Upvotes, 0) AS Upvotes,
//        COALESCE(RV.Downvotes, 0) AS Downvotes, RP.PostRank FROM RankedPosts RP LEFT JOIN RecentVotes RV ON RP.PostId = RV.PostId)
// SELECT PS.*, CASE WHEN PS.AnswerCount > 0 THEN 'Answered' ELSE 'Unanswered' END AS PostStatus, (PS.Upvotes - PS.Downvotes) AS VoteBalance,
//        CASE WHEN PS.Score > 100 THEN 'High Score' WHEN PS.Score BETWEEN 50 AND 100 THEN 'Medium Score' ELSE 'Low Score' END AS ScoreCategory
// FROM PostStatistics PS WHERE PS.PostRank <= 5 ORDER BY PS.Score DESC, PS.RecentVoteCount DESC;
fn q1489(db: &'static So) -> String {
    let Post { creation_date, owner_user, post_type_id, score, answer_count, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let v = drain(db.post.with(creation_date.ge(add_years(t0, -1))).with(owner_user).select(post_type_id.and(score)));
    let v = per_group(ranked(v, |&(_, (t, s))| (t, Reverse(s)), false), |&(_, (t, _))| t);
    let tp = rel(v.into_iter().filter(|x| x.1 <= 5).map(|((p, _), r)| (p, r)).collect());
    let recent = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.creation_date).ge(add_months(t0, -1))));
    let rv = db.post.group_by(Ident::<Post>::new()).select(recent.select(&db.vote.vote_type_id)).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64]);
    type R = (Id<Post>, i64);
    let v = drain((&tp).select(Same::<R>::new().and(Same::<R>::new().map(|(p, _): R| p).select(&rv).opt())));
    rows(v.into_iter().map(|(_, ((p, r), a))| {
        let a = a.unwrap_or([0; 3]);
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "answers", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(r)]);
        f.push(V::S(if answer_count.get(p).map_or(false, |n| n > 0) { "Answered" } else { "Unanswered" }));
        f.push(V::I(a[1] - a[2]));
        f.push(V::S(if s > 100 { "High Score" } else if s >= 50 { "Medium Score" } else { "Low Score" }));
        row(f)
    }))
}

// WITH PostStats AS (SELECT p.Id AS PostId, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes,
//        SUM(CASE WHEN v.VoteTypeId IN (6, 7) THEN 1 ELSE 0 END) AS CloseVotes, AVG(COALESCE(p.Score, 0)) AS AverageScore
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.Id),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// TopPosts AS (SELECT ps.PostId, ps.CommentCount, ps.Upvotes, ps.Downvotes, ps.CloseVotes, ps.AverageScore, ROW_NUMBER() OVER (ORDER BY ps.AverageScore DESC, ps.Upvotes DESC) AS Rank FROM PostStats ps)
// SELECT tp.PostId, tp.CommentCount, tp.Upvotes, tp.Downvotes, tp.CloseVotes, tp.AverageScore, ub.BadgeCount, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges
// FROM TopPosts tp JOIN Users u ON u.Id = (SELECT OwnerUserId FROM Posts WHERE Id = tp.PostId) JOIN UserBadges ub ON ub.UserId = u.Id WHERE tp.Rank <= 10;
//
// AVG(COALESCE(p.Score, 0)) averages the post's own score over its joined rows, so it is the score.
fn q9325(db: &'static So) -> String {
    let Post { creation_date, score, owner_user, .. } = &db.post;
    let ps = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(Ident::<Post>::new())
        .select(score.and(comments_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 6], |a, ((s, c), t)| {
            [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64, a[3] + matches!(t, Some(6 | 7)) as i64, a[4] + s, a[5] + 1]
        });
    let v = top_n(drain(&ps), |&(_, a)| (Reverse(fkey(a[4] as f64 / a[5] as f64)), Reverse(a[1])), 10);
    let tp = rel(v);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    type R = (Id<Post>, [i64; 6]);
    let v = drain((&tp).select(Same::<R>::new().and(Same::<R>::new().map(|(p, _): R| p).select(owner_user.select(&ub)))));
    rows(v.into_iter().map(|(_, ((p, a), b))| {
        let mut f = vec![V::I(db.post.origid.get(p).unwrap()), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::F(a[4] as f64 / a[5] as f64)];
        f.extend(b.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount, COALESCE(NULLIF(u.Reputation, 0), 0) AS UserReputation
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND (p.ViewCount > 100 OR u.Reputation > 50)
//     GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, u.Reputation, p.OwnerUserId, p.Score),
// TopPosts AS (SELECT PostId, Title, CreationDate, ViewCount, PostRank, CommentCount, UpVoteCount, DownVoteCount, UserReputation FROM RankedPosts WHERE PostRank <= 5),
// FinalPosts AS (SELECT tp.*, pt.Name AS PostTypeName FROM TopPosts tp INNER JOIN PostTypes pt ON tp.PostId IN (SELECT p.Id FROM Posts p WHERE p.PostTypeId = pt.Id))
// SELECT fp.PostId, fp.Title, fp.CreationDate, fp.ViewCount, fp.CommentCount, fp.UpVoteCount, fp.DownVoteCount, fp.UserReputation, fp.PostTypeName
// FROM FinalPosts fp ORDER BY fp.ViewCount DESC, fp.UpVoteCount DESC FETCH FIRST 10 ROWS ONLY;
//
// PostRank reads only Score, so each owner's five best posts are picked first and the comment x vote product is driven for those alone.
fn q3405(db: &'static So) -> String {
    let Post { creation_date, owner_user, view_count, score, .. } = &db.post;
    let base = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(view_count.gt(100).or(owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(50)))));
    let top = top_per(drain(base.select(owner_user.opt())), |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = top_n(drain(&s), |&(p, a)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), Reverse(a[1]), p)
    }, 10);
    rows(v.into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views"]);
        f.extend(a.map(V::I));
        f.push(V::I(owner_user.get(p).map_or(0, |u| db.user.reputation.get(u).unwrap())));
        f.push(post_fields(db, p, &["type"]).remove(0));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, ROW_NUMBER() OVER (ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank FROM Posts p
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= DATE '2023-10-01'),
// UserEngagement AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS QuestionCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, SUM(CASE WHEN v.VoteTypeId = 4 THEN 1 ELSE 0 END) AS OffensiveVotes, SUM(CASE WHEN b.Id IS NOT NULL THEN 1 ELSE 0 END) AS BadgesEarned
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PostStatistics AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, ue.UserId, ue.DisplayName AS UserName, ue.QuestionCount, ue.UpVotes, ue.DownVotes, ue.OffensiveVotes,
//        ue.BadgesEarned, rp.Rank FROM RankedPosts rp JOIN UserEngagement ue ON rp.PostId = ue.QuestionCount)
// SELECT ps.PostId, ps.Title, ps.CreationDate, ps.ViewCount, ps.Score, ps.UserName, ps.QuestionCount, ps.UpVotes, ps.DownVotes, ps.OffensiveVotes, ps.BadgesEarned
// FROM PostStatistics ps ORDER BY ps.Rank LIMIT 10;
//
// `rp.PostId = ue.QuestionCount` joins a post id to a count, so it goes through the raw post ids. The join reads only the distinct post count,
// so the posts x votes x badges product is driven only for the users whose count matches a post id.
fn q5347(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, .. } = &db.post;
    let rp = ranked(drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(date(2023, 10, 1)))).select(score)), |&(p, s)| {
        let w = view_count.get(p);
        (Reverse(s), w.is_none(), Reverse(w), p)
    }, false);
    let rp = rel(rp.into_iter().map(|((p, _), r)| (db.post.origid.get(p).unwrap(), (p, r))).collect());
    let by_raw: HashIdx<i64, (i64, (Id<Post>, i64))> = (&rp).map(|(k, _)| k).inv().select(&rp).collect();
    let pc = user_distinct_posts(db);
    let cand: MatSet<Id<User>> = db.user.with((&pc).select(&by_raw)).collect();
    let ue = (&cand)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt().and(badges_of(db).opt()))
        .fold([0i64; 4], |a, (t, b)| {
            let t = t.flatten();
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + (t == Some(4)) as i64, a[3] + b.is_some() as i64]
        });
    let v = drain((&cand).select((&pc).and(&ue).and((&pc).select(&by_raw))));
    let v = top_n(v, |&(u, (_, (_, (p, r))))| (r, p, u), 10);
    rows(v.into_iter().map(|(u, ((n, a), (_, (p, _))))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([user_col(db, u, "name"), V::I(n)]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserScores AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS QuestionCount,
//        COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS AnswerCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, UpVotes, DownVotes, QuestionCount, AnswerCount, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserScores WHERE Reputation > 100),
// PostStatistics AS (SELECT pt.Name AS PostType, COUNT(p.Id) AS PostCount, COALESCE(SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END), 0) AS PositiveScores,
//        COALESCE(SUM(CASE WHEN p.CommentCount > 0 THEN 1 ELSE 0 END), 0) AS PostsWithComments FROM PostTypes pt LEFT JOIN Posts p ON p.PostTypeId = pt.Id GROUP BY pt.Name)
// SELECT tu.DisplayName, tu.Reputation, tu.UpVotes, tu.DownVotes, tu.QuestionCount, tu.AnswerCount, ps.PostType, ps.PostCount, ps.PositiveScores, ps.PostsWithComments
// FROM TopUsers tu JOIN PostStatistics ps ON tu.AnswerCount > 10 ORDER BY tu.ReputationRank, ps.PostCount DESC FETCH FIRST 50 ROWS ONLY;
//
// The ON clause names only tu, so qualifying users are crossed with every post type. The rows come in reputation order, so the posts x votes
// product is driven only for the users ranked at or above the last one that can reach the first 50 rows (each contributes one row per post type).
fn q6514(db: &'static So) -> String {
    let Post { post_type_id, score, comment_count, .. } = &db.post;
    let qa = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(1)) as i64, a[1] + (t == Some(2)) as i64]);
    let tr = ranked(drain(db.user.with((&db.user.reputation).gt(100)).select(&db.user.reputation)), |&(_, r)| Reverse(r), false);
    let tr = rel(tr.into_iter().map(|((u, _), r)| (u, r)).collect());
    let rank_of: HashIdx<Id<User>, i64> = (&tr).map(|(u, _)| u).inv().select(&tr).map(|(_, r)| r).collect();
    let ntypes = count(&db.post_type.name) as usize;
    let tu = top_n(drain(db.user.with((&qa).filt(|a| a[1] > 10)).select(&rank_of)), |&(_, r)| r, 0);
    let last = tu[50usize.div_ceil(ntypes).min(tu.len()) - 1].1;
    let tu: MatSet<Id<User>> = rel(tu.into_iter().take_while(|x| x.1 <= last).map(|x| x.0).collect()).map(|u| u).collect();
    let us = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt()).fold([0i64; 2], |a, t| {
        let t = t.flatten();
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
    });
    let of_type: HashIdx<Id<PostType>, Id<Post>> = (&db.post.post_type).inv().collect();
    let ps = db
        .post_type
        .group_by(&db.post_type.name)
        .select(of_type.select(score.and(comment_count)).opt())
        .fold([0i64; 3], |a, p| match p {
            Some((s, c)) => [a[0] + 1, a[1] + (s > 0) as i64, a[2] + (c > 0) as i64],
            None => a,
        });
    let urows = (&tu).select((&us).and(&qa).and(&rank_of));
    let mut v = Vec::new();
    urows.cross(&ps).drive(|(u, t), (((a, q), r), s)| v.push((u, t, a, q, r, s)));
    let v = top_n(v, |&(u, t, _, _, r, s)| (r, Reverse(s[0]), u, t), 50);
    rows(v.into_iter().map(|(u, t, a, q, _, s)| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(q[0]), V::I(q[1]), V::S(t), V::I(s[0]), V::I(s[1]), V::I(s[2])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, u.DisplayName AS Author, p.Tags,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year' AND p.PostTypeId = 1),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.CreationDate, rp.Author, rp.Tags FROM RankedPosts rp WHERE rp.Rank <= 10),
// PostDetails AS (SELECT tp.PostId, tp.Title, tp.Score, tp.ViewCount, tp.CreationDate, tp.Author, tp.Tags, COUNT(c.Id) AS CommentCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpvoteCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownvoteCount
//     FROM TopPosts tp LEFT JOIN Comments c ON tp.PostId = c.PostId LEFT JOIN Votes v ON tp.PostId = v.PostId GROUP BY tp.PostId, tp.Title, tp.Score, tp.ViewCount, tp.CreationDate, tp.Author, tp.Tags)
// SELECT pd.PostId, pd.Title, pd.Score, pd.ViewCount, pd.CreationDate, pd.Author, pd.Tags, pd.CommentCount, pd.UpvoteCount, pd.DownvoteCount, (pd.UpvoteCount - pd.DownvoteCount) AS NetVote
// FROM PostDetails pd ORDER BY pd.Score DESC, pd.ViewCount DESC;
fn q9444(db: &'static So) -> String {
    let Post { creation_date, owner_user, post_type_id, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.eq(1))).with(owner_user).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain(&s).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "created", "owner", "tags"]);
        f.extend(a.map(V::I));
        f.push(V::I(a[1] - a[2]));
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, AnswerCount, QuestionCount, UpVotes, DownVotes, GoldBadges, SilverBadges, BronzeBadges, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserStatistics)
// SELECT tu.Rank, tu.DisplayName, tu.Reputation, tu.PostCount, tu.AnswerCount, tu.QuestionCount, tu.UpVotes, tu.DownVotes, tu.GoldBadges, tu.SilverBadges, tu.BronzeBadges,
//        ROUND((CAST(tu.UpVotes AS decimal) / NULLIF(tu.UpVotes + tu.DownVotes, 0)) * 100, 2) AS UpVotePercentage
// FROM TopUsers tu WHERE tu.Rank <= 10 ORDER BY tu.Rank;
//
// Rank reads only Reputation, so the ten users are picked first and the posts x votes x badges product is driven for them alone.
fn q26959(db: &'static So) -> String {
    let v = top_n(drain(db.user.with((&db.user.reputation).gt(1000)).select(&db.user.reputation)), |&(u, r)| (Reverse(r), u), 10);
    let tu = rel(v.into_iter().enumerate().map(|(i, (u, _))| (u, i as i64 + 1)).collect());
    let cand: MatSet<Id<User>> = (&tu).map(|(u, _)| u).collect();
    let s = (&cand)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 7], |a, (p, c)| {
            let (t, v) = p.map_or((0, None), |(t, v)| (t, v));
            [a[0] + (t == 2) as i64, a[1] + (t == 1) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64, a[4] + (c == Some(1)) as i64, a[5] + (c == Some(2)) as i64, a[6] + (c == Some(3)) as i64]
        });
    let pc = user_distinct_posts(db);
    type R = (Id<User>, i64);
    let v = drain((&tu).select(Same::<R>::new().and(Same::<R>::new().map(|(u, _): R| u).select((&s).and(&pc)))));
    rows(v.into_iter().map(|(_, ((u, r), (a, n)))| {
        let mut f = vec![V::I(r)];
        f.extend(ucols(db, u, &["name", "rep"]));
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.push(if a[2] + a[3] == 0 { V::Null } else { V::F((a[2] as f64 / (a[2] + a[3]) as f64 * 100.0 * 100.0).round() / 100.0) });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, U.DisplayName AS OwnerDisplayName,
//        RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS ScoreRank
//     FROM Posts p JOIN Users U ON p.OwnerUserId = U.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// RecentVotes AS (SELECT v.PostId, COUNT(v.Id) AS VoteCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Votes v WHERE v.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY v.PostId),
// PostsWithVotes AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, rp.CommentCount, rp.OwnerDisplayName, COALESCE(rv.VoteCount, 0) AS TotalVotes,
//        COALESCE(rv.UpVotes, 0) AS UpVoteCount, COALESCE(rv.DownVotes, 0) AS DownVoteCount, rp.ScoreRank FROM RankedPosts rp LEFT JOIN RecentVotes rv ON rp.PostId = rv.PostId)
// SELECT p.PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.OwnerDisplayName, p.TotalVotes, p.UpVoteCount, p.DownVoteCount, p.ScoreRank
// FROM PostsWithVotes p WHERE p.ScoreRank <= 5 ORDER BY p.Score DESC, p.ViewCount DESC;
fn q9355(db: &'static So) -> String {
    let Post { creation_date, owner_user, post_type_id, score, .. } = &db.post;
    let t0 = add_years(ts(2024, 10, 1, 12, 34, 56), -1);
    let v = drain(db.post.with(creation_date.ge(t0)).with(owner_user).select(post_type_id.and(score)));
    let v = per_group(ranked(v, |&(_, (t, s))| (t, Reverse(s)), false), |&(_, (t, _))| t);
    let tp = rel(v.into_iter().filter(|x| x.1 <= 5).map(|((p, _), r)| (p, r)).collect());
    let recent = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.creation_date).ge(t0)));
    let rv = db.post.group_by(Ident::<Post>::new()).select(recent.select(&db.vote.vote_type_id)).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64]);
    type R = (Id<Post>, i64);
    let v = drain((&tp).select(Same::<R>::new().and(Same::<R>::new().map(|(p, _): R| p).select(&rv).opt())));
    rows(v.into_iter().map(|(_, ((p, r), a))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "owner"]);
        f.extend(a.unwrap_or([0; 3]).map(V::I));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, COUNT(DISTINCT c.Id) AS CommentsCount, COUNT(DISTINCT v.Id) FILTER (WHERE v.VoteTypeId = 2) AS UpVotes,
//        COUNT(DISTINCT v.Id) FILTER (WHERE v.VoteTypeId = 3) AS DownVotes, SUM(COALESCE(b.Class, 0)) AS TotalBadgeClass, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RecentPostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON p.OwnerUserId = b.UserId
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId),
// TopUserPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, u.DisplayName AS OwnerDisplayName, rp.CommentsCount, rp.UpVotes, rp.DownVotes, rp.TotalBadgeClass FROM RankedPosts rp
//     JOIN Users u ON rp.OwnerUserId = u.Id WHERE rp.RecentPostRank = 1 ORDER BY rp.UpVotes - rp.DownVotes DESC, rp.CommentsCount DESC LIMIT 10)
// SELECT tup.Title, tup.CreationDate, tup.OwnerDisplayName, tup.CommentsCount, tup.UpVotes, tup.DownVotes, tup.TotalBadgeClass FROM TopUserPosts tup JOIN PostHistory ph ON tup.PostId = ph.PostId
// WHERE ph.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '6 months' AND ph.PostHistoryTypeId = 10 ORDER BY tup.UpVotes DESC;
//
// The distinct counts need no product, so the ten posts are picked from them first and the badge product (for TotalBadgeClass) is driven for those alone.
// The LIMIT 10 is tie-broken on the post id.
fn q9572(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let v = drain(db.post.with(creation_date.ge(add_years(t0, -1))).select(owner_user.opt()));
    let first = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let first: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cnt = (&first)
        .with(owner_user)
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt())
        .fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = (&first).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let top = top_n(drain((&cnt).and(&cc)), |&(p, (a, c))| (Reverse(a[0] - a[1]), Reverse(c), p), 10);
    let top: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let bs = (&top)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).opt()).and(owner_user.select(badges_of(db).select(&db.badge.class)).opt()))
        .fold(0i64, |n, (_, c)| n + c.unwrap_or(0));
    let PostHistory { creation_date: hd, post_history_type_id, .. } = &db.post_history;
    let closes = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.eq(10).and(hd.ge(add_months(t0, -6)))));
    let v = drain((&top).select((&cnt).and(&cc).and(&bs)).and(closes));
    rows(v.into_iter().map(|(p, (((a, c), b), _))| {
        let mut f = post_fields(db, p, &["title", "created", "owner"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1]), V::I(b)]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 8 GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalBounty, ROW_NUMBER() OVER (ORDER BY TotalPosts DESC, TotalBounty DESC) AS rn FROM UserPostStats)
// SELECT tu.DisplayName, tu.TotalPosts, tu.TotalQuestions, tu.TotalAnswers, tu.TotalBounty, CASE WHEN tu.TotalBounty > 0 THEN 'Has Bounty' ELSE 'No Bounty' END AS BountyStatus,
//        COALESCE(ti.Title, 'N/A') AS TopPostTitle, COUNT(DISTINCT cm.Id) AS TotalComments
// FROM TopUsers tu LEFT JOIN Posts p ON tu.UserId = p.OwnerUserId LEFT JOIN Comments cm ON p.Id = cm.PostId
// LEFT JOIN (SELECT p.Id, p.Title, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn FROM Posts p WHERE p.PostTypeId = 1) ti ON p.Id = ti.Id AND ti.rn = 1
// WHERE tu.rn <= 10 GROUP BY tu.UserId, tu.DisplayName, tu.TotalPosts, tu.TotalQuestions, tu.TotalAnswers, tu.TotalBounty, ti.Title ORDER BY tu.TotalPosts DESC, tu.TotalBounty DESC;
//
// Each user's rows group on ti.Title: the newest question's title, and NULL for every other post (a NULL title joins that group).
fn q1996(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, title, .. } = &db.post;
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let ups = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(bounty.opt())).opt()).fold([0i64; 4], |a, p| match p {
        Some((t, b)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + b.flatten().unwrap_or(0)],
        None => a,
    });
    let top = top_n(drain(&ups), |&(u, a)| (Reverse(a[0]), Reverse(a[3]), u), 10);
    let tu: MatSet<Id<User>> = rel(top.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let newest = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user.opt())), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let ti: MatSet<Id<Post>> = rel(newest.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    type J = (Id<User>, Option<Id<Post>>, Option<Id<Comment>>);
    let j: MatSet<J> = (&tu).select(Ident::<User>::new().and(posts_of(db).select(Ident::<Post>::new().and(comments_of(db).opt())).opt())).map(|(u, p)| (u, p.map(|x| x.0), p.and_then(|x| x.1))).collect();
    let key = (&j).map(|(u, _, _): J| u).and((&j).flat_map(|(_, p, _): J| p).select(Ident::<Post>::new().with(&ti)).select(title).opt());
    let g = (&j).group_by(key).select(Same::<J>::new().map(|(_, _, c): J| c)).buf_fold(distinct_some);
    let v = drain(&g);
    rows(v.into_iter().map(|((u, t), n)| {
        let a = ups.get(u).unwrap();
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::S(if a[3] > 0 { "Has Bounty" } else { "No Bounty" })];
        f.extend([V::S(t.unwrap_or("N/A")), V::I(n)]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.CreationDate, ROW_NUMBER() OVER (ORDER BY u.Reputation DESC) AS ReputationRank FROM Users u),
// PostDetails AS (SELECT p.Id AS PostId, p.Title, p.CreationDate AS PostCreationDate, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes, COALESCE(SUM(CASE WHEN v.VoteTypeId IN (1, 6) THEN 1 ELSE 0 END), 0) AS AcceptedCount
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.Title, p.CreationDate),
// RankedPosts AS (SELECT pd.PostId, pd.Title, pd.PostCreationDate, pd.Upvotes, pd.Downvotes, pd.AcceptedCount, RANK() OVER (ORDER BY pd.Upvotes - pd.Downvotes DESC) AS PostRank FROM PostDetails pd)
// SELECT ur.DisplayName, ur.Reputation, rp.Title, rp.Upvotes, rp.Downvotes, rp.AcceptedCount, CASE WHEN rp.PostRank = 1 THEN 'Top Post' ELSE 'Regular Post' END AS PostCategory,
//        CASE WHEN EXISTS (SELECT 1 FROM Posts p WHERE p.OwnerUserId = ur.UserId AND p.CreationDate < CURRENT_DATE - INTERVAL '6 MONTH') THEN 'Has Older Posts' ELSE 'No Older Posts' END AS OldPostsFlag
// FROM UserReputation ur LEFT JOIN RankedPosts rp ON ur.UserId = rp.PostId WHERE ur.Reputation > 100 ORDER BY ur.Reputation DESC, rp.Upvotes DESC;
//
// `ur.UserId = rp.PostId` joins a user id to a post id, so it goes through the raw ids.
fn q61(db: &'static So) -> String {
    let pd = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + matches!(t, Some(1 | 6)) as i64]
    });
    let rp = ranked(drain(&pd), |&(_, a)| Reverse(a[0] - a[1]), false);
    let rp = rel(rp.into_iter().map(|((p, a), r)| (db.post.origid.get(p).unwrap(), (p, a, r))).collect());
    let by_raw: HashIdx<i64, (i64, (Id<Post>, [i64; 3], i64))> = (&rp).map(|(k, _)| k).inv().select(&rp).collect();
    let old: MatSet<Id<User>> = db.post.with((&db.post.creation_date).lt(add_months(current_date(), -6))).select(&db.post.owner_user).collect();
    let v = drain(db.user.with((&db.user.reputation).gt(100)).select((&db.user.origid).select(&by_raw).opt().and(Ident::<User>::new().with(&old).opt())));
    rows(v.into_iter().map(|(u, (r, o))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        match r {
            Some((_, (p, a, k))) => {
                f.push(ostr(db.post.title.get(p)));
                f.extend(a.map(V::I));
                f.push(V::S(if k == 1 { "Top Post" } else { "Regular Post" }));
            }
            None => f.extend([V::Null, V::Null, V::Null, V::Null, V::S("Regular Post")]),
        }
        f.push(V::S(if o.is_some() { "Has Older Posts" } else { "No Older Posts" }));
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.Views, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty, ROW_NUMBER() OVER (ORDER BY u.Reputation DESC) AS UserRank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName, u.Reputation, u.Views),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, Views, PostCount, QuestionCount, AnswerCount, TotalBounty, UserRank, (PostCount * 1.0 / NULLIF(QuestionCount, 0)) AS QuestionRatio,
//        (PostCount * 1.0 / NULLIF(AnswerCount, 0)) AS AnswerRatio FROM UserStats WHERE Reputation > 100),
// ClosedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, ph.UserId AS ClosedByUserId, ph.CreationDate AS CloseDate, ph.Comment AS CloseReason FROM Posts p JOIN PostHistory ph ON p.Id = ph.PostId
//     WHERE ph.PostHistoryTypeId = 10)
// SELECT tu.DisplayName, tu.Reputation, tu.QuestionCount, tu.AnswerCount, tu.TotalBounty, cp.Title AS ClosedPostTitle, cp.CloseDate, cp.CloseReason
// FROM TopUsers tu LEFT JOIN ClosedPosts cp ON tu.UserId = cp.ClosedByUserId ORDER BY tu.UserRank, cp.CloseDate DESC FETCH FIRST 10 ROWS ONLY;
//
// UserRank reads only Reputation and every user has at least one output row, so the ten best-ranked users are picked first and the posts x votes product is driven for them alone.
fn q2598(db: &'static So) -> String {
    let v = top_n(drain(db.user.with((&db.user.reputation).gt(100)).select(&db.user.reputation)), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(v.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let s = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(&db.post.post_type_id).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold([0i64; 3], |a, (t, b)| [a[0] + (t == Some(1)) as i64, a[1] + (t == Some(2)) as i64, a[2] + b.flatten().unwrap_or(0)]);
    let PostHistory { user, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let closes: HashIdx<Id<User>, Id<PostHistory>> = db.post_history.with(post_history_type_id.eq(10)).select(user).inv().collect();
    let v = drain((&s).and((&closes).opt()));
    let v = top_n(v, |&(u, (_, h))| (Reverse(db.user.reputation.get(u).unwrap()), u, Reverse(h.map(|h| hd.get(h).unwrap()))), 10);
    rows(v.into_iter().map(|(u, (a, h))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(a.map(V::I));
        f.extend(match h {
            Some(h) => [ostr(db.post.title.get(db.post_history.post.get(h).unwrap())), V::T(hd.get(h).unwrap()), ostr(db.post_history.comment.get(h))],
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserEngagement AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN C.Id IS NOT NULL THEN 1 ELSE 0 END) AS CommentCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount, SUM(B.Class) AS BadgeCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Badges B ON U.Id = B.UserId
//     GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, AnswerCount, QuestionCount, CommentCount, UpvoteCount, DownvoteCount, BadgeCount, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserEngagement)
// SELECT TU.DisplayName, TU.Reputation, TU.PostCount, TU.AnswerCount, TU.QuestionCount, TU.CommentCount, TU.UpvoteCount, TU.DownvoteCount, TU.BadgeCount, PH.UserDisplayName AS LastEditedBy,
//        PH.CreationDate AS LastEditedDate, COUNT(*) OVER (PARTITION BY TU.UserId) AS EditCount
// FROM TopUsers TU LEFT JOIN PostHistory PH ON TU.UserId = PH.UserId WHERE TU.ReputationRank <= 10 ORDER BY TU.Reputation DESC, EditCount DESC;
//
// ReputationRank reads only Reputation, so the top-ranked users are picked first and the posts x comments x votes x badges product is driven for them alone.
fn q26263(db: &'static So) -> String {
    let v = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let tu: MatSet<Id<User>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let Post { post_type_id, .. } = &db.post;
    let s = (&tu)
        .group_by(Ident::<User>::new())
        .select(
            posts_of(db)
                .select(post_type_id.and(comments_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()))
                .opt()
                .and(badges_of(db).select(&db.badge.class).opt()),
        )
        .fold([0i64; 7], |a, (p, b)| {
            let (t, c, v) = p.map_or((0, false, None), |((t, c), v)| (t, c.is_some(), v));
            [a[0] + (t == 2) as i64, a[1] + (t == 1) as i64, a[2] + c as i64, a[3] + (v == Some(2)) as i64, a[4] + (v == Some(3)) as i64, a[5] + b.is_some() as i64, a[6] + b.unwrap_or(0)]
        });
    let pc = user_distinct_posts(db);
    let PostHistory { user, user_display_name, creation_date: hd, .. } = &db.post_history;
    let hist: HashIdx<Id<User>, Id<PostHistory>> = db.post_history.select(user).inv().collect();
    let ec = (&tu).group_by(Ident::<User>::new()).select((&hist).opt()).fold(0i64, |n, _| n + 1);
    let v = drain((&s).and(&pc).and(&ec).and((&hist).opt()));
    rows(v.into_iter().map(|(u, (((a, n), e), h))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4]), nullable(a[6], a[5])]);
        f.extend(match h {
            Some(h) => [ostr(user_display_name.get(h)), V::T(hd.get(h).unwrap())],
            None => [V::Null, V::Null],
        });
        f.push(V::I(e));
        row(f)
    }))
}

// WITH User_reputation AS (SELECT u.Id, u.DisplayName, u.Reputation, (SELECT COUNT(*) FROM Posts p WHERE p.OwnerUserId = u.Id) AS PostCount,
//        COALESCE((SELECT AVG(v.BountyAmount) FROM Votes v WHERE v.UserId = u.Id AND v.VoteTypeId IN (8, 9)), 0) AS AverageBounty, ROW_NUMBER() OVER (ORDER BY u.Reputation DESC) AS UserRank FROM Users u),
// PostMetrics AS (SELECT p.Id AS PostId, p.Title, p.Body, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, COUNT(c.Id) FILTER(WHERE c.Text IS NOT NULL) AS NonNullCommentCount,
//        MAX(ph.CreationDate) AS LastEditDate FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId
//     GROUP BY p.Id, p.Title, p.Body, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount),
// TopPosts AS (SELECT pm.*, ROW_NUMBER() OVER (ORDER BY pm.Score DESC, pm.ViewCount DESC) AS PostRank FROM PostMetrics pm)
// SELECT u.DisplayName AS UserName, u.Reputation, COALESCE(tp.Title, 'No Posts') AS PostTitle, tp.Score AS PostScore, tp.ViewCount AS PostViews, tp.LastEditDate,
//        CASE WHEN tp.PostRank <= 10 THEN 'Top Performers' ELSE 'Others' END AS UserGroup
// FROM User_reputation u LEFT JOIN TopPosts tp ON u.Id = tp.PostId WHERE u.Reputation > (SELECT AVG(Reputation) FROM Users) AND (tp.PostId IS NOT NULL OR (tp.PostId IS NULL AND tp.NonNullCommentCount > 0))
// ORDER BY u.Reputation DESC, tp.Score DESC NULLS LAST;
//
// `u.Id = tp.PostId` joins a user id to a post id, so it goes through the raw ids. The second disjunct can never hold (a NULL PostId has a NULL count),
// so only matched posts are kept, and the comment x history product is driven for those alone. PostRank's ties go to the smaller id.
fn q3274(db: &'static So) -> String {
    let Post { score, view_count, .. } = &db.post;
    let top = top_n(drain(&db.post.origid), |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 10);
    let top: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let by_raw: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let (rs, rn) = db.user.select(&db.user.reputation).fold_flat((0i64, 0i64), |(s, n), r| (s + r, n + 1));
    let above = || Ident::<User>::new().with((&db.user.reputation).filt(|r| (r as i128) * (rn as i128) > rs as i128));
    let matched: MatSet<Id<Post>> = db.user.select(above()).select((&db.user.origid).select(&by_raw)).collect();
    let pm = (&matched)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(history_of(db).select(&db.post_history.creation_date).opt()))
        .fold((0i64, i64::MIN), |(n, m), (c, d)| (n + c.is_some() as i64, d.map_or(m, |d| m.max(d))));
    let v = drain(db.user.select(above()).select((&db.user.origid).select(&by_raw).select(Ident::<Post>::new().and(&pm).and(Ident::<Post>::new().with(&top).opt()))));
    rows(v.into_iter().map(|(u, ((p, (_, m)), t))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::S(db.post.title.get(p).unwrap_or("No Posts")));
        f.extend(post_fields(db, p, &["score", "views"]));
        f.push(tmax(m));
        f.push(V::S(if t.is_some() { "Top Performers" } else { "Others" }));
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(CASE WHEN p.PostTypeId = 1 AND p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges, SUM(CASE WHEN p.ClosedDate IS NOT NULL THEN 1 ELSE 0 END) AS ClosedPosts
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, Questions, Answers, AcceptedAnswers, UpVotes, DownVotes, GoldBadges, SilverBadges, BronzeBadges, ClosedPosts,
//        ROW_NUMBER() OVER (ORDER BY TotalPosts DESC) AS Rank FROM UserStatistics)
// SELECT Rank, DisplayName, TotalPosts, Questions, Answers, AcceptedAnswers, UpVotes, DownVotes, GoldBadges, SilverBadges, BronzeBadges, ClosedPosts FROM TopUsers WHERE Rank <= 10 ORDER BY Rank;
//
// Rank reads only the distinct post count, so the ten users are picked first and the posts x votes x badges product is driven for them alone.
fn q6022(db: &'static So) -> String {
    let pc = user_distinct_posts(db);
    let v = top_n(drain(&pc), |&(u, n)| (Reverse(n), u), 10);
    let tu = rel(v.into_iter().enumerate().map(|(i, (u, n))| (u, (i as i64 + 1, n))).collect());
    let cand: MatSet<Id<User>> = (&tu).map(|(u, _)| u).collect();
    let Post { post_type_id, accepted_answer, closed_date, .. } = &db.post;
    let s = (&cand)
        .group_by(Ident::<User>::new())
        .select(
            posts_of(db)
                .select(post_type_id.and(accepted_answer.opt()).and(closed_date.opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()))
                .opt()
                .and(badges_of(db).select(&db.badge.class).opt()),
        )
        .fold([0i64; 9], |a, (p, c)| {
            let (t, acc, cl, v) = p.map_or((0, false, false, None), |(((t, acc), cl), v)| (t, acc.is_some(), cl.is_some(), v));
            [
                a[0] + (t == 1) as i64,
                a[1] + (t == 2) as i64,
                a[2] + (t == 1 && acc) as i64,
                a[3] + (v == Some(2)) as i64,
                a[4] + (v == Some(3)) as i64,
                a[5] + (c == Some(1)) as i64,
                a[6] + (c == Some(2)) as i64,
                a[7] + (c == Some(3)) as i64,
                a[8] + cl as i64,
            ]
        });
    type R = (Id<User>, (i64, i64));
    let v = drain((&tu).select(Same::<R>::new().and(Same::<R>::new().map(|(u, _): R| u).select(&s))));
    rows(v.into_iter().map(|(_, ((u, (r, n)), a))| {
        let mut f = vec![V::I(r), user_col(db, u, "name"), V::I(n)];
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS ScoreRank,
//        COUNT(*) OVER (PARTITION BY p.OwnerUserId) AS TotalPosts FROM Posts p WHERE p.PostTypeId = 1),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS TotalQuestions, SUM(p.Score) AS TotalScore, SUM(COALESCE(c.Score, 0)) AS TotalCommentsScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId AND p.PostTypeId = 1 LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY u.Id, u.DisplayName, u.Reputation),
// RecentVotes AS (SELECT v.UserId AS VoterId, COUNT(v.Id) AS VoteCount, SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes
//     FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY v.UserId)
// SELECT ua.UserId, ua.DisplayName, ua.Reputation, ua.TotalQuestions, ua.TotalScore, ra.PostId, ra.Title, ra.Score, ra.ScoreRank, ra.TotalPosts, rv.VoteCount, rv.UpVotes, rv.DownVotes
// FROM UserActivity ua LEFT JOIN RankedPosts ra ON ua.UserId = ra.PostId LEFT JOIN RecentVotes rv ON ua.UserId = rv.VoterId
// WHERE ua.Reputation > 1000 AND (COALESCE(rv.UpVotes, 0) - COALESCE(rv.DownVotes, 0)) > 5 ORDER BY ua.Reputation DESC, ra.Score DESC;
//
// `ua.UserId = ra.PostId` joins a user id to a post id, so it goes through the raw ids. UserActivity is computed only for the users the WHERE keeps.
fn q30459(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, .. } = &db.post;
    let rv = db.vote.group_by(&db.vote.user).select(vtype_name(db)).fold([0i64; 3], |a, n| [a[0] + 1, a[1] + (n == "UpMod") as i64, a[2] + (n == "DownMod") as i64]);
    let users: MatSet<Id<User>> = db.user.with((&db.user.reputation).gt(1000)).with((&rv).filt(|a| a[1] - a[2] > 5)).collect();
    let qs = || posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1)));
    let ua = (&users).group_by(Ident::<User>::new()).select(qs().select(score.and(comments_of(db).select(&db.comment.score).opt())).opt()).fold([0i64; 2], |a, p| match p {
        Some((s, _)) => [a[0] + 1, a[1] + s],
        None => a,
    });
    let tq = (&users).group_by(Ident::<User>::new()).select(qs().opt()).buf_fold(distinct_some);
    let v = drain(db.post.with(post_type_id.eq(1)).select(owner_user.opt()));
    let rk = per_group(ranked(v, |&(p, u)| (u, Reverse(score.get(p).unwrap()), p), false), |&(_, u)| u);
    let tot = db.post.with(post_type_id.eq(1)).group_by(owner_user.opt()).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let rk = rel(rk.into_iter().map(|((p, u), r)| (db.post.origid.get(p).unwrap(), (p, r, tot.get(u).unwrap()))).collect());
    let by_raw: HashIdx<i64, (i64, (Id<Post>, i64, i64))> = (&rk).map(|(k, _)| k).inv().select(&rk).collect();
    let v = drain((&users).select((&ua).and(&tq).and((&db.user.origid).select(&by_raw).opt()).and(&rv)));
    rows(v.into_iter().map(|(u, (((a, q), r), b))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(q), nullable(a[1], a[0])]);
        f.extend(match r {
            Some((_, (p, k, t))) => {
                let mut g = post_fields(db, p, &["id", "title", "score"]);
                g.extend([V::I(k), V::I(t)]);
                g
            }
            None => vec![V::Null, V::Null, V::Null, V::Null, V::Null],
        });
        f.extend(b.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty, COUNT(DISTINCT b.Id) AS BadgeCount
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// EligibleUsers AS (SELECT us.UserId, us.DisplayName, us.Reputation, us.TotalBounty, us.BadgeCount FROM UserStats us WHERE us.Reputation > 1000 AND us.BadgeCount >= 5)
// SELECT eu.DisplayName, eu.Reputation, eu.TotalBounty, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, (SELECT COUNT(*) FROM Comments c WHERE c.PostId = rp.PostId) AS CommentCount,
//        (SELECT COUNT(*) FROM Votes v WHERE v.PostId = rp.PostId AND v.VoteTypeId = 2) AS UpVotes, (SELECT COUNT(*) FROM Votes v WHERE v.PostId = rp.PostId AND v.VoteTypeId = 3) AS DownVotes
// FROM RankedPosts rp JOIN EligibleUsers eu ON rp.PostId IN (SELECT p.Id FROM Posts p WHERE p.OwnerUserId = eu.UserId AND p.ViewCount > 50)
// WHERE rp.PostRank = 1 ORDER BY eu.Reputation DESC, rp.ViewCount DESC LIMIT 10;
//
// The newest post of each owner is picked first; UserStats (the votes x badges product) is driven only for the owners it can join.
fn q2632(db: &'static So) -> String {
    let Post { creation_date, owner_user, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user.opt()));
    let first = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let rp: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cand: MatSet<Id<User>> = (&rp).with(view_count.gt(50)).select(owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(1000)))).collect();
    let us = (&cand)
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select((&db.vote.bounty_amount).opt()).opt().and(badges_of(db).opt()))
        .fold(0i64, |n, (b, _)| n + b.flatten().unwrap_or(0));
    let bc = (&cand).group_by(Ident::<User>::new()).select(badges_of(db).opt()).buf_fold(distinct_some);
    let eu = Ident::<User>::new().and(&us).and((&bc).filt(|n| n >= 5));
    let v = drain((&rp).with(view_count.gt(50)).select(owner_user.select(eu)));
    let v = top_n(v, |&(p, ((u, _), _))| {
        let w = view_count.get(p);
        (Reverse(db.user.reputation.get(u).unwrap()), w.is_none(), Reverse(w), p)
    }, 10);
    let top = rel(v);
    type R = (Id<Post>, ((Id<User>, i64), i64));
    let pid = || Same::<R>::new().map(|(p, _): R| p);
    let cnt = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let ccount = db.post.group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let v = drain((&top).select(Same::<R>::new().and(pid().select(&ccount).opt()).and(pid().select(&cnt).opt())));
    rows(v.into_iter().map(|(_, (((p, ((u, b), _)), c), a))| {
        let a = a.unwrap_or([0, 0]);
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(b));
        f.extend(post_fields(db, p, &["title", "created", "views", "score"]));
        f.extend([V::I(c.unwrap_or(0)), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounty, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews
//     FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName),
// PostHistoryStats AS (SELECT ph.PostId, COUNT(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 END) AS CloseCount, COUNT(CASE WHEN ph.PostHistoryTypeId = 11 THEN 1 END) AS ReopenCount,
//        COUNT(CASE WHEN ph.PostHistoryTypeId = 12 THEN 1 END) AS DeleteCount FROM PostHistory ph GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, rp.CommentCount, tu.DisplayName AS TopUser, tu.TotalBounty, tu.TotalViews, phs.CloseCount, phs.ReopenCount, phs.DeleteCount
// FROM RankedPosts rp JOIN TopUsers tu ON rp.PostId = (SELECT p.Id FROM Posts p WHERE p.OwnerUserId = tu.UserId ORDER BY p.Score DESC LIMIT 1) JOIN PostHistoryStats phs ON rp.PostId = phs.PostId
// WHERE rp.Rank <= 5 ORDER BY rp.Score DESC, tu.TotalViews DESC;
//
// Both the Rank and the correlated `ORDER BY p.Score DESC LIMIT 1` leave score ties open; the port gives them to the smaller id.
// TopUsers (the posts x votes product) is driven only for the owners of the ranked posts.
fn q5024(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false);
    let rp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let best = top_per(drain(db.post.select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 1, false);
    let best: MatSet<Id<Post>> = rel(best.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cand: MatSet<Id<User>> = (&rp).with(&best).select(owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(1000)))).collect();
    let tu = (&cand)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(view_count.opt().and(votes_of(db).select((&db.vote.bounty_amount).opt()).opt())))
        .fold([0i64; 2], |a, (w, b)| [a[0] + b.flatten().unwrap_or(0), a[1] + w.unwrap_or(0)]);
    let phs = db.post_history.group_by(&db.post_history.post).select(&db.post_history.post_history_type_id).fold([0i64; 3], |a, t| [a[0] + (t == 10) as i64, a[1] + (t == 11) as i64, a[2] + (t == 12) as i64]);
    let v = drain((&rp).with(&best).select(owner_user.select(Ident::<User>::new().and(&tu))).and(&phs));
    rows(v.into_iter().map(|(p, ((u, t), h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments"]);
        f.extend([user_col(db, u, "name"), V::I(t[0]), V::I(t[1])]);
        f.extend(h.map(V::I));
        row(f)
    }))
}

// Rewritten (rewrites/1622.sql): the final ORDER BY is tie-broken on FR.QuestionId.
// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS Rank FROM Users U),
// ActiveQuestions AS (SELECT P.Id AS QuestionId, P.Title, P.CreationDate, COALESCE(P.AcceptedAnswerId, 0) AS AcceptedAnswerId, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
//        AVG(CASE WHEN V.VoteTypeId = 2 THEN 1.0 ELSE 0 END) AS AvgUpVotes FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId = 2
//     WHERE P.PostTypeId = 1 GROUP BY P.Id, P.Title, P.AcceptedAnswerId, P.CreationDate),
// ClosedQuestions AS (SELECT PH.PostId, COUNT(*) AS CloseCount FROM PostHistory PH WHERE PH.PostHistoryTypeId = 10 GROUP BY PH.PostId),
// FinalResults AS (SELECT AQ.QuestionId, AQ.Title, AQ.AcceptedAnswerId, AQ.CommentCount, AQ.AvgUpVotes, COALESCE(CQ.CloseCount, 0) AS CloseCount, UR.Reputation FROM ActiveQuestions AQ
//     LEFT JOIN ClosedQuestions CQ ON AQ.QuestionId = CQ.PostId JOIN UserReputation UR ON AQ.AcceptedAnswerId = UR.UserId)
// SELECT FR.QuestionId, FR.Title, FR.CommentCount, FR.AvgUpVotes, FR.CloseCount, CASE WHEN FR.Reputation > 1000 THEN 'High Reputation' WHEN FR.Reputation BETWEEN 500 AND 1000 THEN 'Medium Reputation'
//        ELSE 'Low Reputation' END AS ReputationCategory
// FROM FinalResults FR WHERE FR.CloseCount = 0 ORDER BY FR.AvgUpVotes DESC, FR.CommentCount DESC, FR.QuestionId FETCH FIRST 100 ROWS ONLY;
//
// `AQ.AcceptedAnswerId = UR.UserId` joins a post id to a user id, so it goes through the raw ids.
fn q1622(db: &'static So) -> String {
    let Post { post_type_id, accepted_answer_id, .. } = &db.post;
    let up = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2)));
    let closed: MatSet<Id<Post>> = db.post_history.with((&db.post_history.post_history_type_id).eq(10)).select(&db.post_history.post).collect();
    let by_raw: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let aq = db
        .post
        .with(post_type_id.eq(1))
        .minus(&closed)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(up.opt()))
        .fold([0i64; 3], |a, (c, v)| [a[0] + c.is_some() as i64, a[1] + v.is_some() as i64, a[2] + 1]);
    let v = drain((&aq).and(accepted_answer_id.opt().map(|x: Option<i64>| x.unwrap_or(0)).select(&by_raw)));
    let v = top_n(v, |&(p, (a, _))| (Reverse(fkey(a[1] as f64 / a[2] as f64)), Reverse(a[0]), p), 100);
    rows(v.into_iter().map(|(p, (a, u))| {
        let r = db.user.reputation.get(u).unwrap();
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([V::I(a[0]), V::F(a[1] as f64 / a[2] as f64), V::I(0)]);
        f.push(V::S(if r > 1000 { "High Reputation" } else if r >= 500 { "Medium Reputation" } else { "Low Reputation" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.AnswerCount, COALESCE(u.DisplayName, 'Anonymous') AS OwnerDisplayName, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// PostVoteStats AS (SELECT v.PostId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS Upvotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS Downvotes FROM Votes v GROUP BY v.PostId),
// CommentStats AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c GROUP BY c.PostId),
// CombinedStats AS (SELECT rp.PostId, rp.Title, rp.Score, rp.AnswerCount, rp.OwnerDisplayName, COALESCE(pvs.Upvotes, 0) AS Upvotes, COALESCE(pvs.Downvotes, 0) AS Downvotes,
//        COALESCE(cs.CommentCount, 0) AS CommentCount, rp.Rank FROM RankedPosts rp LEFT JOIN PostVoteStats pvs ON rp.PostId = pvs.PostId LEFT JOIN CommentStats cs ON rp.PostId = cs.PostId)
// SELECT cs.PostId, cs.Title, cs.Score, cs.AnswerCount, cs.OwnerDisplayName, cs.Upvotes, cs.Downvotes, cs.CommentCount, CASE WHEN cs.Rank = 1 THEN 'Top Post' ELSE 'Regular Post' END AS PostCategory
// FROM CombinedStats cs WHERE cs.Upvotes > cs.Downvotes ORDER BY cs.Score DESC, cs.Upvotes DESC LIMIT 10;
fn q2746(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let rp = || db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let first = top_per(drain(rp().select(owner_user.opt())), |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 1, false);
    let first: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pv = rp().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = rp().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&pv).filt(|a| a[0] > a[1]).and(&cc).and(Ident::<Post>::new().with(&first).opt()));
    let v = top_n(v, |&(p, ((a, _), _))| (Reverse(score.get(p).unwrap()), Reverse(a[0])), 10);
    rows(v.into_iter().map(|(p, ((a, c), t))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "answers"]);
        f.push(V::S(owner_user.get(p).map_or("Anonymous", |u| db.user.display_name.get(u).unwrap())));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c), V::S(if t.is_some() { "Top Post" } else { "Regular Post" })]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostsCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersCount,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsCount, SUM(COALESCE(co.CommentCount, 0)) AS CommentsCount, SUM(COALESCE(v.VoteCount, 0)) AS VotesCount,
//        COALESCE(MAX(p.CreationDate), '1970-01-01') AS LastActivityDate
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) co ON p.Id = co.PostId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS VoteCount FROM Votes GROUP BY PostId) v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// ActiveUsers AS (SELECT UserId, DisplayName, PostsCount, AnswersCount, QuestionsCount, CommentsCount, VotesCount, LastActivityDate, RANK() OVER (ORDER BY PostsCount DESC) AS RankPosts,
//        RANK() OVER (ORDER BY AnswersCount DESC) AS RankAnswers FROM UserActivity WHERE LastActivityDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 YEAR')
// SELECT au.DisplayName, au.PostsCount, au.AnswersCount, au.QuestionsCount, au.CommentsCount, au.VotesCount, au.RankPosts, au.RankAnswers FROM ActiveUsers au
// WHERE au.RankPosts <= 10 OR au.RankAnswers <= 10 ORDER BY au.RankPosts, au.RankAnswers;
fn q5103(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let vc = db.post.group_by(Ident::<Post>::new()).select(votes_of(db)).fold(0i64, |n, _| n + 1);
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and((&cc).opt()).and((&vc).opt()).and(creation_date)).opt())
        .fold([0, 0, 0, 0, 0, 0], |a, p| match p {
            Some((((t, c), v), d)) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 1) as i64, a[3] + c.unwrap_or(0), a[4] + v.unwrap_or(0), a[5].max(d)],
            None => a,
        });
    let v = drain((&ua).filt(|a| a[0] > 0 && a[5] >= add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let v = ranked(v, |&(_, a)| Reverse(a[0]), false);
    let v = ranked(v, |&((_, a), _)| Reverse(a[1]), false);
    let v = drain(rel(v).filt(|(((_, _), p), a): (((Id<User>, [i64; 6]), i64), i64)| p <= 10 || a <= 10)).into_iter().map(|x| x.1);
    rows(v.map(|(((u, a), p), r)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(a[..5].iter().map(|&x| V::I(x)));
        f.extend([V::I(p), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p INNER JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName, p.PostTypeId),
// TopRankedPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, OwnerDisplayName FROM RankedPosts WHERE Rank <= 10)
// SELECT trp.PostId, trp.Title, trp.CreationDate, trp.Score, trp.ViewCount, trp.OwnerDisplayName, COALESCE(b.UserCount, 0) AS BadgeCount, COALESCE(v.UpVoteCount, 0) AS UpVoteCount,
//        COALESCE(v.DownVoteCount, 0) AS DownVoteCount
// FROM TopRankedPosts trp LEFT JOIN (SELECT UserId, COUNT(*) AS UserCount FROM Badges GROUP BY UserId) b ON trp.OwnerDisplayName = (SELECT DisplayName FROM Users WHERE Id = b.UserId)
// LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount FROM Votes GROUP BY PostId) v ON trp.PostId = v.PostId
// ORDER BY trp.Score DESC, trp.CreationDate DESC;
//
// The badge join matches every user with the owner's display name, so a post has one row per such user with badges.
fn q8846(db: &'static So) -> String {
    let Post { creation_date, owner_user, post_type_id, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db)).fold(0i64, |n, _| n + 1);
    let by_name: HashIdx<Str, i64> = (&db.user.display_name).inv().select(&bc).collect();
    let pv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let v = drain((&tp).select(owner_user.select(&db.user.display_name).select(&by_name).opt().and((&pv).opt())));
    rows(v.into_iter().map(|(p, (b, a))| {
        let a = a.unwrap_or([0, 0]);
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend([V::I(b.unwrap_or(0)), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.Score, p.AnswerCount, p.CommentCount, u.DisplayName AS OwnerDisplayName,
//        RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS PostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.PostTypeId IN (1, 2)),
// PostStatistics AS (SELECT p.PostId, p.Title, p.ViewCount, p.Score, p.AnswerCount, p.CommentCount, p.OwnerDisplayName, p.PostRank, ph.UserId AS LastEditorId, ph.UserDisplayName AS LastEditorDisplayName,
//        ph.CreationDate AS LastEditDate FROM RankedPosts p LEFT JOIN PostHistory ph ON p.PostId = ph.PostId AND ph.CreationDate IS NOT NULL WHERE p.PostRank <= 10),
// VoteCounts AS (SELECT v.PostId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes, COUNT(v.Id) AS TotalVotes FROM Votes v GROUP BY v.PostId)
// SELECT ps.PostId, ps.Title, ps.ViewCount, ps.Score, ps.AnswerCount, ps.CommentCount, ps.OwnerDisplayName, ps.LastEditorId, ps.LastEditorDisplayName, ps.LastEditDate, vc.UpVotes, vc.DownVotes, vc.TotalVotes
// FROM PostStatistics ps LEFT JOIN VoteCounts vc ON ps.PostId = vc.PostId ORDER BY ps.Score DESC, ps.ViewCount DESC;
fn q9673(db: &'static So) -> String {
    let Post { creation_date, owner_user, post_type_id, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.is_in([1, 2]))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())), 10, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 3], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + 1]);
    let v = drain((&tp).select(history_of(db).opt().and((&vc).opt())));
    rows(v.into_iter().map(|(p, (h, a))| {
        let mut f = post_fields(db, p, &["id", "title", "views", "score", "answers", "comments", "owner"]);
        f.extend(match h {
            Some(h) => [oint(db.post_history.user_id.get(h)), ostr(db.post_history.user_display_name.get(h)), V::T(db.post_history.creation_date.get(h).unwrap())],
            None => [V::Null, V::Null, V::Null],
        });
        f.extend(match a {
            Some(a) => a.map(V::I),
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.Score, p.ViewCount, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank FROM Posts p
//     WHERE p.PostTypeId = 1 AND (p.Score > 10 OR p.ViewCount > 1000)),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges, COALESCE(SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadges,
//        COALESCE(SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadges, COUNT(DISTINCT post.Id) AS NumberOfPosts, COALESCE(SUM(u.Reputation), 0) AS Reputation
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Posts post ON u.Id = post.OwnerUserId GROUP BY u.Id, u.DisplayName),
// PostHistoryComments AS (SELECT ph.PostId, ph.CreationDate, ph.Comment AS CloseComment, ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS CommentRank FROM PostHistory ph
//     WHERE ph.PostHistoryTypeId IN (10, 11))
// SELECT u.DisplayName, u.GoldBadges, u.SilverBadges, u.BronzeBadges, u.NumberOfPosts, rp.Title, rp.Score, rp.ViewCount, phc.CloseComment
// FROM UserStats u JOIN RankedPosts rp ON u.NumberOfPosts > 5 LEFT JOIN PostHistoryComments phc ON rp.Id = phc.PostId AND phc.CommentRank = 1
// WHERE u.GoldBadges > 0 ORDER BY u.Reputation DESC, rp.Score DESC OFFSET 10 ROWS FETCH NEXT 10 ROWS ONLY;
//
// The ON clause names only u, so users are crossed with the posts. The badges x posts product is driven only for users with more than five posts,
// the only ones the join keeps. A close-comment tie on CreationDate goes to the larger history id.
fn q4262(db: &'static So) -> String {
    let pc = user_distinct_posts(db);
    let cand: MatSet<Id<User>> = db.user.with((&pc).filt(|n| n > 5)).collect();
    let us = (&cand)
        .group_by(Ident::<User>::new())
        .select((&db.user.reputation).and(badges_of(db).select(&db.badge.class).opt()).and(posts_of(db).opt()))
        .fold([0i64; 4], |a, ((r, c), _)| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64, a[3] + r]);
    let ua = drain((&us).filt(|a| a[0] > 0).and(&pc));
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let rp = drain(db.post.with(post_type_id.eq(1).and(score.gt(10).or(view_count.gt(1000)))).select(score));
    let v = cross_top(ua, |&(u, (a, _))| (Reverse(a[3]), u), rp, |&(p, s)| (Reverse(s), p), 20);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let cl = drain(db.post_history.with(post_history_type_id.in_v(vec![10, 11])).select(post));
    let cl = top_per(cl, |&(_, p)| p, |&(h, _)| (Reverse(hd.get(h).unwrap()), Reverse(h)), 1, false);
    let cl = rel(cl.into_iter().map(|(h, p)| (p, h)).collect());
    let last: HashIdx<Id<Post>, Id<PostHistory>> = (&cl).map(|(p, _)| p).inv().select(&cl).map(|(_, h)| h).collect();
    rows(v.into_iter().skip(10).map(|((u, (a, n)), (p, _))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(n)];
        f.extend(post_fields(db, p, &["title", "score", "views"]));
        f.push(last.get(p).map_or(V::Null, |h| ostr(db.post_history.comment.get(h))));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.CreationDate, U.LastAccessDate, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(P.Score) AS TotalScore, SUM(COALESCE(V.BountyAmount, 0)) AS TotalBounty
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON V.UserId = U.Id GROUP BY U.Id, U.DisplayName, U.Reputation, U.CreationDate, U.LastAccessDate),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, TotalScore, TotalBounty, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserStats),
// UserBadges AS (SELECT B.UserId, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Badges B GROUP BY B.UserId)
// SELECT TU.Rank, TU.DisplayName, TU.Reputation, TU.PostCount, TU.QuestionCount, TU.AnswerCount, TU.TotalScore, TU.TotalBounty, COALESCE(UB.BadgeCount, 0) AS BadgeCount,
//        COALESCE(UB.GoldBadges, 0) AS GoldBadges, COALESCE(UB.SilverBadges, 0) AS SilverBadges, COALESCE(UB.BronzeBadges, 0) AS BronzeBadges
// FROM TopUsers TU LEFT JOIN UserBadges UB ON TU.UserId = UB.UserId WHERE TU.Rank <= 10 ORDER BY TU.Rank;
//
// Rank reads only Reputation, so the ten users are picked first and the posts x votes product is driven for them alone.
fn q8151(db: &'static So) -> String {
    let v = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10);
    let tu = rel(v.into_iter().enumerate().map(|(i, (u, _))| (u, i as i64 + 1)).collect());
    let cand: MatSet<Id<User>> = (&tu).map(|(u, _)| u).collect();
    let s = (&cand)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(&db.post.score)).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold([0i64; 5], |a, (p, b)| {
            let b = b.flatten().unwrap_or(0);
            match p {
                Some((t, s)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + 1, a[3] + s, a[4] + b],
                None => [a[0], a[1], a[2], a[3], a[4] + b],
            }
        });
    let pc = user_distinct_posts(db);
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    type R = (Id<User>, i64);
    let uid = || Same::<R>::new().map(|(u, _): R| u);
    let v = drain((&tu).select(Same::<R>::new().and(uid().select((&s).and(&pc))).and(uid().select(&ub).opt())));
    rows(v.into_iter().map(|(_, (((u, r), (a, n)), b))| {
        let mut f = vec![V::I(r)];
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), nullable(a[3], a[2]), V::I(a[4])]);
        f.extend(b.unwrap_or([0; 4]).map(V::I));
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 AND p.ViewCount > COALESCE((SELECT AVG(ViewCount) FROM Posts WHERE PostTypeId = 1), 0) AND p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName),
// RankedPosts AS (SELECT rp.*, ROW_NUMBER() OVER (PARTITION BY rp.OwnerDisplayName ORDER BY rp.Score DESC, rp.ViewCount DESC) AS Rank FROM RecentPosts rp)
// SELECT rp.OwnerDisplayName, rp.Title, rp.Score, rp.ViewCount, rp.CommentCount, COALESCE(b.BadgeCount, 0) AS BadgeCount, COALESCE(b.Class, 0) AS BadgeClass, rp.CreationDate,
//        CASE WHEN rp.Rank = 1 THEN 'Top Post' ELSE 'Regular Post' END AS PostType
// FROM RankedPosts rp LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount, MAX(Class) AS Class FROM Badges GROUP BY UserId) b ON rp.OwnerDisplayName = (SELECT DisplayName FROM Users WHERE Id = b.UserId)
// WHERE rp.Rank <= 3 ORDER BY rp.OwnerDisplayName, rp.Score DESC;
fn q20065(db: &'static So) -> String {
    let Post { post_type_id, view_count, creation_date, owner_user, score, .. } = &db.post;
    let (vs, vn) = db.post.with(post_type_id.eq(1)).select(view_count).fold_flat((0i64, 0i64), |(s, n), w| (s + w, n + 1));
    let base = db.post.with(post_type_id.eq(1).and(creation_date.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))).with(view_count.filt(|w| (w as i128) * (vn as i128) > vs as i128));
    let owner_name = owner_user.select(&db.user.display_name);
    let v = drain(base.select(owner_name.opt()));
    let ranks = per_group(ranked(v, |&(p, n)| (n, Reverse(score.get(p).unwrap()), Reverse(view_count.get(p)), p), false), |&(_, n)| n);
    let tp = rel(ranks.into_iter().filter(|x| x.1 <= 3).map(|((p, _), r)| (p, r)).collect());
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let bs = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold((0i64, 0i64), |(n, m), c| (n + 1, m.max(c)));
    let by_name: HashIdx<Str, (i64, i64)> = (&db.user.display_name).inv().select(&bs).collect();
    type R = (Id<Post>, i64);
    let pid = || Same::<R>::new().map(|(p, _): R| p);
    let v = drain((&tp).select(Same::<R>::new().and(pid().select(&cc)).and(pid().select(owner_user.select(&db.user.display_name).select(&by_name)).opt())));
    rows(v.into_iter().map(|(_, (((p, r), c), b))| {
        let (n, m) = b.unwrap_or((0, 0));
        let mut f = post_fields(db, p, &["owner", "title", "score", "views"]);
        f.extend([V::I(c), V::I(n), V::I(m)]);
        f.extend(post_fields(db, p, &["created"]));
        f.push(V::S(if r == 1 { "Top Post" } else { "Regular Post" }));
        row(f)
    }))
}

// WITH UserBadgeStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS TotalBadges, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// ActiveUsersPosts AS (SELECT p.OwnerUserId, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers
//     FROM Posts p WHERE p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.OwnerUserId),
// UserPerformance AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(ubs.TotalBadges, 0) AS TotalBadges, COALESCE(aup.TotalPosts, 0) AS TotalPosts, COALESCE(aup.Questions, 0) AS Questions,
//        COALESCE(aup.Answers, 0) AS Answers, u.Reputation FROM Users u LEFT JOIN UserBadgeStats ubs ON u.Id = ubs.UserId LEFT JOIN ActiveUsersPosts aup ON u.Id = aup.OwnerUserId)
// SELECT up.UserId, up.DisplayName, up.TotalBadges, up.TotalPosts, up.Questions, up.Answers, up.Reputation, RANK() OVER (ORDER BY up.Reputation DESC) AS ReputationRank,
//        RANK() OVER (ORDER BY up.TotalPosts DESC) AS PostRank
// FROM UserPerformance up WHERE up.TotalPosts > 0 ORDER BY up.Reputation DESC, up.TotalPosts DESC LIMIT 10;
fn q8500(db: &'static So) -> String {
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let Post { creation_date, owner_user, post_type_id, .. } = &db.post;
    let aup = db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(owner_user).select(post_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64]);
    let v = drain(db.user.select((&bc).and(&aup)));
    let v = ranked(v, |&(u, _)| Reverse(db.user.reputation.get(u).unwrap()), false);
    let v = ranked(v, |&((_, (_, a)), _)| Reverse(a[0]), false);
    let v = top_n(v, |&(((u, (_, a)), _), _)| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(a[0])), 10);
    rows(v.into_iter().map(|(((u, (b, a)), r), k)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(b), V::I(a[0]), V::I(a[1]), V::I(a[2]), user_col(db, u, "rep"), V::I(r), V::I(k)]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS Rank FROM Users U),
// PostDetails AS (SELECT P.Id AS PostId, P.OwnerUserId, P.Title, P.CreationDate, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY P.Id, P.OwnerUserId, P.Title, P.CreationDate),
// TopPosts AS (SELECT PD.PostId, PD.Title, PD.CreationDate, PD.CommentCount, PD.UpVotes, PD.DownVotes, UR.DisplayName, UR.Reputation, RANK() OVER (ORDER BY PD.UpVotes DESC) AS PostRank
//     FROM PostDetails PD JOIN UserReputation UR ON PD.OwnerUserId = UR.UserId WHERE UR.Rank <= 50)
// SELECT TP.PostId, TP.Title, TP.CreationDate, TP.CommentCount, TP.UpVotes, TP.DownVotes, COALESCE((SELECT COUNT(*) FROM PostHistory PH WHERE PH.PostId = TP.PostId AND PH.PostHistoryTypeId IN (10, 11)), 0) AS CloseHistoryCount,
//        CASE WHEN TP.CommentCount > 0 THEN 'Has Comments' ELSE 'No Comments' END AS CommentStatus,
//        CASE WHEN TP.UpVotes > TP.DownVotes THEN 'Net Positive' WHEN TP.UpVotes < TP.DownVotes THEN 'Net Negative' ELSE 'Neutral' END AS VoteStatus
// FROM TopPosts TP WHERE TP.PostRank <= 20 ORDER BY TP.UpVotes DESC, TP.CommentCount DESC;
//
// The comment x vote product is driven only for the posts of the fifty best-ranked users, the only ones TopPosts keeps.
fn q1554(db: &'static So) -> String {
    let v = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 50);
    let ur: MatSet<Id<User>> = rel(v.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let posts: MatSet<Id<Post>> = (&ur).select(posts_of(db)).collect();
    let pd = (&posts)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = ranked(drain(&pd), |&(_, a)| Reverse(a[1]), false);
    let tp = rel(v.into_iter().take_while(|x| x.1 <= 20).map(|x| x.0).collect());
    let ch = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([10, 11])));
    let hc = (&posts).group_by(Ident::<Post>::new()).select(ch.opt()).fold(0i64, |n, h| n + h.is_some() as i64);
    type R = (Id<Post>, [i64; 3]);
    let v = drain((&tp).select(Same::<R>::new().and(Same::<R>::new().map(|(p, _): R| p).select(&hc))));
    rows(v.into_iter().map(|(_, ((p, a), h))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend(a.map(V::I));
        f.push(V::I(h));
        f.push(V::S(if a[0] > 0 { "Has Comments" } else { "No Comments" }));
        f.push(V::S(if a[1] > a[2] { "Net Positive" } else if a[1] < a[2] { "Net Negative" } else { "Neutral" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, u.DisplayName AS OwnerDisplayName, p.AnswerCount, p.CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank, p.OwnerUserId
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.CreationDate >= CURRENT_TIMESTAMP - INTERVAL '1 year'),
// PostBadges AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount FROM Badges b WHERE b.Date >= CURRENT_TIMESTAMP - INTERVAL '1 year' GROUP BY b.UserId),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(pb.BadgeCount, 0) AS BadgeCount FROM Users u LEFT JOIN PostBadges pb ON u.Id = pb.UserId ORDER BY u.Reputation DESC LIMIT 10)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.OwnerDisplayName, ru.Reputation, ru.BadgeCount, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM RankedPosts rp JOIN TopUsers ru ON rp.OwnerUserId = ru.UserId LEFT JOIN Comments c ON c.PostId = rp.PostId LEFT JOIN Votes v ON v.PostId = rp.PostId
// WHERE rp.Rank <= 3 GROUP BY rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.OwnerDisplayName, ru.UserId, ru.Reputation, ru.BadgeCount ORDER BY rp.Score DESC, ru.Reputation DESC;
//
// CURRENT_TIMESTAMP is a TIMESTAMPTZ, so the TIMESTAMP columns are compared as New York wall times.
fn q5038(db: &'static So) -> String {
    let since = ny_to_utc(add_years(utc_to_ny(now_utc()), -1));
    let v = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(v.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let pb = db.badge.with((&db.badge.date).filt(move |d| ny_to_utc(d) >= since)).group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let Post { post_type_id, creation_date, owner_user, score, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1)).with(creation_date.filt(move |d| ny_to_utc(d) >= since)).with(owner_user).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 3, false);
    let rp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&rp)
        .with(owner_user.select(&tu))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = drain((&s).and(owner_user.select(Ident::<User>::new().and((&pb).opt()))));
    rows(v.into_iter().map(|(p, (a, (u, b)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "owner"]);
        f.extend([user_col(db, u, "rep"), V::I(b.unwrap_or(0))]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(COALESCE(v.VoteCount, 0)) AS TotalVotes, COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS Questions,
//        COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS Answers
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN (SELECT PostId, COUNT(*) AS VoteCount FROM Votes GROUP BY PostId) v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalVotes, Questions, Answers, RANK() OVER (ORDER BY TotalVotes DESC) AS VoteRank FROM UserActivity),
// RelatedPosts AS (SELECT pl.PostId, p.Title, pl.RelatedPostId, (SELECT COUNT(*) FROM Comments c WHERE c.PostId = pl.RelatedPostId) AS CommentCount FROM PostLinks pl JOIN Posts p ON pl.PostId = p.Id)
// SELECT tu.DisplayName, tu.TotalPosts, tu.TotalVotes, tu.Questions, tu.Answers, rp.Title AS RelatedPostTitle, rp.CommentCount,
//        CASE WHEN tu.TotalVotes >= 100 THEN 'Gold' WHEN tu.TotalVotes >= 50 THEN 'Silver' WHEN tu.TotalVotes >= 10 THEN 'Bronze' ELSE 'No Badge' END AS Badge
// FROM TopUsers tu LEFT JOIN RelatedPosts rp ON tu.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = rp.RelatedPostId) WHERE tu.VoteRank <= 10 ORDER BY tu.TotalVotes DESC
fn q800(db: &'static So) -> String {
    let vc = db.post.group_by(Ident::<Post>::new()).select(votes_of(db)).fold(0i64, |n, _| n + 1);
    let ua = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select((&db.post.post_type_id).and((&vc).opt())).opt()).fold([0i64; 4], |a, p| match p {
        Some((t, v)) => [a[0] + 1, a[1] + v.unwrap_or(0), a[2] + (t == 1) as i64, a[3] + (t == 2) as i64],
        None => a,
    });
    let v = ranked(drain(&ua), |&(_, a)| Reverse(a[1]), false);
    let tu = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0).collect());
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let PostLink { post, related_post, .. } = &db.post_link;
    let by_owner: HashIdx<Id<User>, Id<PostLink>> = db.post_link.select(related_post.select(&db.post.owner_user)).inv().collect();
    type R = (Id<User>, [i64; 4]);
    let v = drain((&tu).select(Same::<R>::new().and(Same::<R>::new().map(|(u, _): R| u).select(&by_owner).opt())));
    rows(v.into_iter().map(|(_, ((u, a), l))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(a.map(V::I));
        match l {
            Some(l) => f.extend([ostr(db.post.title.get(post.get(l).unwrap())), V::I(cc.get(related_post.get(l).unwrap()).unwrap_or(0))]),
            None => f.extend([V::Null, V::Null]),
        }
        f.push(V::S(if a[1] >= 100 { "Gold" } else if a[1] >= 50 { "Silver" } else if a[1] >= 10 { "Bronze" } else { "No Badge" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, U.DisplayName AS OwnerDisplayName,
//        RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS RankByScore
//     FROM Posts p JOIN Users U ON p.OwnerUserId = U.Id WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopQuestions AS (SELECT * FROM RankedPosts WHERE RankByScore = 1),
// UserScores AS (SELECT U.Id AS UserId, U.DisplayName, SUM(p.Score) AS TotalScore, COUNT(DISTINCT p.Id) AS QuestionCount FROM Users U JOIN Posts p ON U.Id = p.OwnerUserId WHERE p.PostTypeId = 1 GROUP BY U.Id, U.DisplayName),
// BadgeCounts AS (SELECT B.UserId, COUNT(B.Id) AS BadgeTotal FROM Badges B WHERE B.Class = 1 GROUP BY B.UserId)
// SELECT U.DisplayName, US.TotalScore, US.QuestionCount, COALESCE(BC.BadgeTotal, 0) AS GoldBadges, COUNT(C.Id) AS CommentCount, SUM(P.Score) AS TotalPostScore
// FROM UserScores US JOIN Users U ON US.UserId = U.Id LEFT JOIN BadgeCounts BC ON U.Id = BC.UserId LEFT JOIN Comments C ON C.UserId = U.Id LEFT JOIN Posts P ON P.OwnerUserId = U.Id
// WHERE U.Reputation >= 1000 GROUP BY U.DisplayName, US.TotalScore, US.QuestionCount, BC.BadgeTotal ORDER BY TotalScore DESC, GoldBadges DESC, QuestionCount DESC LIMIT 10;
//
// TopQuestions is never read. The groups are keyed by (DisplayName, TotalScore, QuestionCount, BadgeTotal), none of which needs the comments x posts product,
// so the top ten keys are picked first and the product is driven only for the users with those keys.
fn q7009(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let us = db.post.with(post_type_id.eq(1)).group_by(&db.post.owner_user).select(score).fold([0i64; 2], |a, s| [a[0] + s, a[1] + 1]);
    let bc = db.badge.with((&db.badge.class).eq(1)).group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let key = || (&db.user.display_name).and(&us).and((&bc).opt());
    let eligible = || db.user.with((&db.user.reputation).ge(1000));
    let keys: MatSet<((Str, [i64; 2]), Option<i64>)> = eligible().select(key()).collect();
    let top = top_n(drain(&keys).into_iter().map(|x| x.1).collect(), |&((n, a), b)| (Reverse(a[0]), Reverse(b.unwrap_or(0)), Reverse(a[1]), n, b), 10);
    let top: MatSet<((Str, [i64; 2]), Option<i64>)> = rel(top).map(|k| k).collect();
    let g = eligible()
        .with(key().select(&top))
        .group_by(key())
        .select(comments_by(db).opt().and(posts_of(db).select(score).opt()))
        .fold([0i64; 3], |a, (c, s)| [a[0] + c.is_some() as i64, a[1] + s.is_some() as i64, a[2] + s.unwrap_or(0)]);
    let v = top_n(drain(&g), |&(((n, a), b), _)| (Reverse(a[0]), Reverse(b.unwrap_or(0)), Reverse(a[1]), n, b), 10);
    rows(v.into_iter().map(|(((n, a), b), s)| row(vec![V::S(n), V::I(a[0]), V::I(a[1]), V::I(b.unwrap_or(0)), V::I(s[0]), nullable(s[2], s[1])])))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, COUNT(c.Id) AS TotalComments, COUNT(DISTINCT b.Id) AS TotalBadges, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes, SUM(CASE WHEN v.VoteTypeId = 6 THEN 1 ELSE 0 END) AS TotalCloseVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// RankedUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalComments, TotalBadges, TotalUpvotes, TotalDownvotes, TotalCloseVotes,
//        RANK() OVER (ORDER BY TotalPosts DESC, TotalUpvotes - TotalDownvotes DESC) AS UserRank FROM UserPostStats)
// SELECT ru.UserId, ru.DisplayName, ru.TotalPosts, ru.TotalQuestions, ru.TotalAnswers, ru.TotalComments, ru.TotalBadges, ru.TotalUpvotes, ru.TotalDownvotes, ru.TotalCloseVotes,
//        (SELECT COUNT(*) FROM Posts p WHERE p.OwnerUserId = ru.UserId AND p.ClosedDate IS NOT NULL) AS ClosedPosts, (SELECT COUNT(*) FROM Posts p WHERE p.OwnerUserId = ru.UserId AND p.FavoriteCount > 0) AS FavoritePosts
// FROM RankedUsers ru WHERE ru.UserRank <= 10 ORDER BY ru.UserRank;
fn q7445(db: &'static So) -> String {
    let Post { post_type_id, closed_date, favorite_count, .. } = &db.post;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(comments_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 7], |a, (p, _)| match p {
            Some(((t, c), v)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + c.is_some() as i64, a[4] + (v == Some(2)) as i64, a[5] + (v == Some(3)) as i64, a[6] + (v == Some(6)) as i64],
            None => a,
        });
    let v = ranked(drain(&s), |&(_, a)| (Reverse(a[0]), Reverse(a[4] - a[5])), false);
    let tu = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, a), r)| (u, (a, r))).collect());
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let cp = db.post.with(closed_date).group_by(&db.post.owner_user).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let fp = db.post.with(favorite_count.gt(0)).group_by(&db.post.owner_user).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    type R = (Id<User>, ([i64; 7], i64));
    let uid = || Same::<R>::new().map(|(u, _): R| u);
    let v = drain((&tu).select(Same::<R>::new().and(uid().select(&bc)).and(uid().select((&cp).opt()).and(uid().select((&fp).opt())))));
    rows(v.into_iter().map(|(_, (((u, (a, _)), b), (c, f)))| {
        let mut x = ucols(db, u, &["uid", "name"]);
        x.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(b), V::I(a[4]), V::I(a[5]), V::I(a[6]), V::I(c.unwrap_or(0)), V::I(f.unwrap_or(0))]);
        row(x)
    }))
}

// Rewritten (rewrites/1263.sql): the PostRank order is tie-broken on p.Id.
// WITH RankedPosts AS (SELECT p.Id, p.Title, p.OwnerUserId, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.Id) AS PostRank, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.OwnerUserId, p.CreationDate, p.Score),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges, COALESCE(SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadges,
//        COALESCE(SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation)
// SELECT us.UserId, us.DisplayName, us.Reputation, us.GoldBadges, us.SilverBadges, us.BronzeBadges, rp.Title, rp.Score, rp.CommentCount,
//        CASE WHEN rp.Score < 0 THEN 'Negative Score' WHEN rp.Score BETWEEN 0 AND 10 THEN 'Low Score' ELSE 'High Score' END AS ScoreCategory
// FROM UserStats us JOIN RankedPosts rp ON us.UserId = rp.OwnerUserId WHERE rp.PostRank = 1 ORDER BY us.Reputation DESC, rp.Score DESC LIMIT 10;
//
// PostRank reads only Score, so each owner's best post is picked first and the comment x vote product is driven for those alone.
fn q1263(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 1, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    let v = drain((&cc).and(owner_user.select(Ident::<User>::new().and(&ub))));
    let v = top_n(v, |&(p, (_, (u, _)))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(score.get(p).unwrap()), p), 10);
    rows(v.into_iter().map(|(p, (c, (u, b)))| {
        let s = score.get(p).unwrap();
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(b.map(V::I));
        f.extend(post_fields(db, p, &["title", "score"]));
        f.extend([V::I(c), V::S(if s < 0 { "Negative Score" } else if s <= 10 { "Low Score" } else { "High Score" })]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN p.PostTypeId IN (3, 4, 5) THEN 1 ELSE 0 END) AS TotalWikis, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId WHERE u.Reputation > 1000
//     GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, TotalQuestions, TotalAnswers, TotalWikis, TotalUpvotes, TotalDownvotes, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS UserRank FROM UserActivity)
// SELECT tu.DisplayName, tu.Reputation, tu.TotalPosts, tu.TotalQuestions, tu.TotalAnswers, tu.TotalWikis, tu.TotalUpvotes, tu.TotalDownvotes,
//        CASE WHEN tu.TotalQuestions > 0 THEN ROUND((CAST(tu.TotalUpvotes AS DECIMAL) / NULLIF(tu.TotalQuestions, 0)) * 100, 2) ELSE 0 END AS UpvotePercentage,
//        CASE WHEN tu.TotalAnswers > 0 THEN ROUND((CAST(tu.TotalDownvotes AS DECIMAL) / NULLIF(tu.TotalAnswers, 0)) * 100, 2) ELSE 0 END AS DownvotePercentage
// FROM TopUsers tu WHERE tu.UserRank <= 10 ORDER BY tu.Reputation DESC;
//
// UserRank reads only Reputation, so the ten users are picked first and the posts x votes product is driven for them alone.
fn q29938(db: &'static So) -> String {
    let v = top_n(drain(db.user.with((&db.user.reputation).gt(1000)).select(&db.user.reputation)), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(v.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let s = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 5], |a, p| {
            let (t, v) = p.map_or((0, None), |(t, v)| (t, v));
            [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + matches!(t, 3 | 4 | 5) as i64, a[3] + (v == Some(2)) as i64, a[4] + (v == Some(3)) as i64]
        });
    let pc = user_distinct_posts(db);
    let pct = |x: i64, n: i64| V::F(if n > 0 { (x as f64 / n as f64 * 100.0 * 100.0).round() / 100.0 } else { 0.0 });
    rows(drain((&s).and(&pc)).into_iter().map(|(u, (a, n))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.extend([pct(a[3], a[0]), pct(a[4], a[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS Rank FROM Posts p
//     WHERE p.PostTypeId = 1 AND p.Score > 0),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS QuestionCount, SUM(COALESCE(b.Class, 0)) AS TotalBadges, SUM(p.ViewCount) AS TotalViews
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId AND p.PostTypeId = 1 LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PostHistorySummary AS (SELECT ph.UserId, COUNT(DISTINCT ph.PostId) AS PostCount, COUNT(DISTINCT CASE WHEN ph.PostHistoryTypeId = 10 THEN ph.Id END) AS ClosedPostCount,
//        COUNT(DISTINCT CASE WHEN ph.PostHistoryTypeId = 11 THEN ph.Id END) AS ReopenedPostCount FROM PostHistory ph GROUP BY ph.UserId)
// SELECT us.UserId, us.DisplayName, us.QuestionCount, us.TotalBadges, us.TotalViews, COALESCE(p.Rank, 0) AS HighestRankPost, COALESCE(ph.PostCount, 0) AS TotalPostHistoryCount,
//        COALESCE(ph.ClosedPostCount, 0) AS TotalClosedPosts, COALESCE(ph.ReopenedPostCount, 0) AS TotalReopenedPosts
// FROM UserStats us LEFT JOIN RankedPosts p ON us.UserId = p.OwnerUserId AND p.Rank = 1 LEFT JOIN PostHistorySummary ph ON us.UserId = ph.UserId
// WHERE us.QuestionCount > 0 ORDER BY us.TotalViews DESC, us.QuestionCount DESC, us.TotalBadges DESC LIMIT 100;
//
// Only whether a user has a positive-score question reaches the output (the Rank = 1 row), so the rank itself is not computed.
fn q22922(db: &'static So) -> String {
    let Post { post_type_id, view_count, score, owner_user, .. } = &db.post;
    let qs = posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1)));
    let us = db
        .user
        .with(qs)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1))).select(view_count.opt()).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 3], |a, (w, c)| {
            let w = w.flatten();
            [a[0] + c.unwrap_or(0), a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0)]
        });
    let qc = db.post.with(post_type_id.eq(1)).group_by(owner_user).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let pos: MatSet<Id<User>> = db.post.with(post_type_id.eq(1).and(score.gt(0))).select(owner_user).collect();
    let PostHistory { user, post, post_history_type_id, .. } = &db.post_history;
    let phd = db.post_history.group_by(user).select(post).count_distinct();
    let phc = db.post_history.group_by(user).select(post_history_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 10) as i64, a[1] + (t == 11) as i64]);
    let v = drain((&us).and(&qc).and(Ident::<User>::new().with(&pos).opt()).and((&phd).opt()).and((&phc).opt()));
    let v = top_n(v, |&(u, ((((a, q), _), _), _))| (a[1] == 0, Reverse(a[2]), Reverse(q), Reverse(a[0]), u), 100);
    rows(v.into_iter().map(|(u, ((((a, q), r), d), c))| {
        let c = c.unwrap_or([0, 0]);
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(q), V::I(a[0]), nullable(a[2], a[1]), V::I(r.is_some() as i64), V::I(d.unwrap_or(0)), V::I(c[0]), V::I(c[1])]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COALESCE(SUM(V.BountyAmount), 0) AS TotalBounties, COUNT(DISTINCT P.Id) AS TotalPosts,
//        ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// RecentPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, COALESCE(COUNT(C.Id), 0) AS CommentCount, COUNT(DISTINCT PH.Id) AS EditHistoryCount, ROW_NUMBER() OVER (PARTITION BY P.Id ORDER BY P.CreationDate DESC) AS PostRank
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN PostHistory PH ON P.Id = PH.PostId WHERE P.CreationDate >= CURRENT_TIMESTAMP - INTERVAL '30 days' GROUP BY P.Id, P.Title, P.CreationDate),
// TopUsers AS (SELECT UR.DisplayName, UR.Reputation, UR.UserId, RANK() OVER (ORDER BY UR.Reputation DESC) AS OverallRank FROM UserReputation UR)
// SELECT TU.DisplayName, TU.Reputation, RP.Title AS RecentPostTitle, RP.CreationDate AS RecentPostDate, RP.CommentCount, RP.EditHistoryCount,
//        (SELECT COUNT(*) FROM Votes V WHERE V.UserId = TU.UserId AND V.CreationDate >= CURRENT_TIMESTAMP - INTERVAL '1 month') AS RecentVotes
// FROM TopUsers TU JOIN RecentPosts RP ON RP.PostRank = 1 WHERE TU.OverallRank <= 10 ORDER BY TU.Reputation DESC, RP.CommentCount DESC;
//
// The ON clause names only RP, so the top users are crossed with the recent posts (PostRank is 1 for every post). CURRENT_TIMESTAMP is a TIMESTAMPTZ,
// so the TIMESTAMP columns are compared as New York wall times.
fn q1910(db: &'static So) -> String {
    let now = utc_to_ny(now_utc());
    let d30 = ny_to_utc(add_days(now, -30));
    let m1 = ny_to_utc(add_months(now, -1));
    let v = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let tu: MatSet<Id<User>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let rv = (&tu).group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.creation_date).opt()).fold(0i64, |n, d| n + d.map_or(false, |d| ny_to_utc(d) >= m1) as i64);
    let rp = db
        .post
        .with((&db.post.creation_date).filt(move |d| ny_to_utc(d) >= d30))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(history_of(db).opt()))
        .fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let ed = db.post.with((&db.post.creation_date).filt(move |d| ny_to_utc(d) >= d30)).group_by(Ident::<Post>::new()).select(history_of(db).opt()).buf_fold(distinct_some);
    let mut v = Vec::new();
    (&rv).cross((&rp).and(&ed)).drive(|(u, p), (n, (c, e))| v.push((u, p, n, c, e)));
    rows(v.into_iter().map(|(u, p, n, c, e)| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["title", "created"]));
        f.extend([V::I(c), V::I(e), V::I(n)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN pt.Name = 'Answer' THEN 1 ELSE 0 END) AS AnswersCount,
//        SUM(CASE WHEN pt.Name = 'Question' THEN 1 ELSE 0 END) AS QuestionsCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesCount,
//        SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN PostTypes pt ON p.PostTypeId = pt.Id LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId
//     WHERE u.Reputation > 0 AND u.CreationDate < TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY u.Id, u.DisplayName),
// RankedUsers AS (SELECT UserId, DisplayName, PostCount, AnswersCount, QuestionsCount, UpVotesCount, DownVotesCount, GoldBadges, SilverBadges, BronzeBadges,
//        RANK() OVER (ORDER BY PostCount DESC, AnswersCount DESC, UpVotesCount DESC) AS UserRank FROM UserStats)
// SELECT UserId, DisplayName, PostCount, AnswersCount, QuestionsCount, UpVotesCount, DownVotesCount, GoldBadges, SilverBadges, BronzeBadges, UserRank FROM RankedUsers WHERE UserRank <= 10 ORDER BY UserRank;
//
// UserRank leads with the distinct post count, so only users with at least the tenth-highest count can rank in the top ten; the posts x votes x badges product is driven for those alone.
fn q9365(db: &'static So) -> String {
    let pc = user_distinct_posts(db);
    let el = || db.user.with((&db.user.reputation).gt(0).and((&db.user.creation_date).lt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))));
    let tenth = top_n(drain(el().select(&pc)), |&(u, n)| (Reverse(n), u), 10).last().unwrap().1;
    let cand: MatSet<Id<User>> = el().with((&pc).filt(|n| n >= tenth)).collect();
    let s = (&cand)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(ptype_name(db).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 7], |a, (p, c)| {
            let (t, v) = p.map_or(("", None), |(t, v)| (t, v));
            [a[0] + (t == "Answer") as i64, a[1] + (t == "Question") as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64, a[4] + (c == Some(1)) as i64, a[5] + (c == Some(2)) as i64, a[6] + (c == Some(3)) as i64]
        });
    let v = ranked(drain((&s).and(&pc)), |&(_, (a, n))| (Reverse(n), Reverse(a[0]), Reverse(a[2])), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, (a, n)), r)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts, SUM(CASE WHEN p.Score < 0 THEN 1 ELSE 0 END) AS NegativePosts,
//        AVG(u.Reputation) AS AvgReputation, COUNT(DISTINCT b.Id) AS BadgeCount FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, PositivePosts, NegativePosts, AvgReputation, BadgeCount, RANK() OVER (ORDER BY PostCount DESC) AS Rank FROM UserStats),
// UserActivity AS (SELECT u.Id AS UserId, COUNT(c.Id) AS CommentCount, COUNT(v.Id) AS VoteCount FROM Users u LEFT JOIN Comments c ON u.Id = c.UserId LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id),
// FinalReport AS (SELECT tu.DisplayName, tu.PostCount, tu.PositivePosts, tu.NegativePosts, uA.CommentCount, uA.VoteCount, tu.AvgReputation, tu.BadgeCount,
//        CASE WHEN tu.AvgReputation IS NULL THEN 'No Reputation' WHEN tu.AvgReputation < 1000 THEN 'Novice' WHEN tu.AvgReputation < 5000 THEN 'Intermediate' ELSE 'Expert' END AS ReputationLevel
//     FROM TopUsers tu JOIN UserActivity uA ON tu.UserId = uA.UserId)
// SELECT *, (PostCount + CommentCount + VoteCount) AS TotalEngagement FROM FinalReport WHERE BadgeCount > 0 ORDER BY TotalEngagement DESC LIMIT 10 ;
//
// AVG(u.Reputation) averages the user's own reputation over its joined rows, so it is the reputation.
fn q3102(db: &'static So) -> String {
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db)).fold(0i64, |n, _| n + 1);
    let el = || db.user.with(&bc);
    let us = el()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(&db.post.score).opt().and(badges_of(db).opt()))
        .fold([0i64; 2], |a, (s, _)| [a[0] + s.map_or(false, |s| s > 0) as i64, a[1] + s.map_or(false, |s| s < 0) as i64]);
    let ua = el().group_by(Ident::<User>::new()).select(comments_by(db).opt().and(votes_by(db).opt())).fold([0i64; 2], |a, (c, v)| [a[0] + c.is_some() as i64, a[1] + v.is_some() as i64]);
    let pc = user_distinct_posts(db);
    let v = drain((&us).and(&ua).and(&pc).and(&bc));
    let v = top_n(v, |&(u, (((_, c), n), _))| (Reverse(n + c[0] + c[1]), u), 10);
    rows(v.into_iter().map(|(u, (((a, c), n), b))| {
        let r = db.user.reputation.get(u).unwrap();
        let mut f = vec![user_col(db, u, "name"), V::I(n), V::I(a[0]), V::I(a[1]), V::I(c[0]), V::I(c[1]), V::F(r as f64), V::I(b)];
        f.push(V::S(if r < 1000 { "Novice" } else if r < 5000 { "Intermediate" } else { "Expert" }));
        f.push(V::I(n + c[0] + c[1]));
        row(f)
    }))
}

// WITH PostAnalytics AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.Score, p.CreationDate, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RowNum
//     FROM Posts p LEFT JOIN Comments c ON c.PostId = p.Id LEFT JOIN Votes v ON v.PostId = p.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.ViewCount, p.Score, p.CreationDate, p.OwnerUserId),
// TopPosts AS (SELECT PostId, Title, ViewCount, Score, CommentCount, UpVotes, DownVotes, RANK() OVER (ORDER BY Score DESC) AS ScoreRank FROM PostAnalytics)
// SELECT u.DisplayName AS Author, tp.Title, tp.ViewCount, tp.Score, tp.CommentCount, tp.UpVotes, tp.DownVotes, CASE WHEN tp.ScoreRank <= 10 THEN 'Top Post' ELSE 'Regular Post' END AS PostCategory,
//        COALESCE(pht.Comment, 'No comments') AS LastEditComment
// FROM Users u JOIN Posts p ON p.OwnerUserId = u.Id RIGHT JOIN TopPosts tp ON tp.PostId = p.Id
// LEFT JOIN PostHistory pht ON pht.PostId = p.Id AND pht.CreationDate = (SELECT MAX(CreationDate) FROM PostHistory WHERE PostId = p.Id AND PostHistoryTypeId IN (4, 5))
// WHERE u.Reputation > 1000 ORDER BY tp.Score DESC, tp.CommentCount DESC FETCH FIRST 20 ROWS ONLY;
//
// The WHERE on u.Reputation drops the rows the RIGHT JOIN adds, so it is an inner join. Any history row at the latest title/body edit's instant joins.
fn q4966(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let pa = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let rk = ranked(drain(&pa), |&(p, _)| Reverse(score.get(p).unwrap()), false);
    let rk = rel(rk.into_iter().map(|((p, a), r)| (p, (a, r))).collect());
    let tp: HashIdx<Id<Post>, ([i64; 3], i64)> = (&rk).map(|(p, _)| p).inv().select(&rk).map(|(_, x)| x).collect();
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let md = db.post_history.with(post_history_type_id.is_in([4, 5])).group_by(post).select(hd).fold(i64::MIN, |m, d| m.max(d));
    let at: HashIdx<(Id<Post>, i64), Id<PostHistory>> = db.post_history.select(post.and(hd)).inv().collect();
    let v = drain(db.post.with(owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(1000)))).select((&tp).and(Ident::<Post>::new().and(&md).select(&at).opt())));
    let v = top_n(v, |&(p, ((a, _), h))| (Reverse(score.get(p).unwrap()), Reverse(a[0]), p, h), 20);
    rows(v.into_iter().map(|(p, ((a, r), h))| {
        let mut f = post_fields(db, p, &["owner", "title", "views", "score"]);
        f.extend(a.map(V::I));
        f.push(V::S(if r <= 10 { "Top Post" } else { "Regular Post" }));
        f.push(V::S(h.and_then(|h| db.post_history.comment.get(h)).unwrap_or("No comments")));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, u.DisplayName AS OwnerDisplayName, u.Reputation AS OwnerReputation, COUNT(DISTINCT a.Id) AS AnswerCount,
//        COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Posts a ON a.ParentId = p.Id AND a.PostTypeId = 2 LEFT JOIN Votes v ON v.PostId = p.Id WHERE p.PostTypeId = 1
//     GROUP BY p.Id, p.Title, p.Body, p.CreationDate, u.DisplayName, u.Reputation),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.Body, rp.CreationDate, rp.OwnerDisplayName, rp.OwnerReputation, rp.AnswerCount, rp.UpVotes, rp.DownVotes, (rp.UpVotes - rp.DownVotes) AS VoteScore FROM RankedPosts rp WHERE rp.rn = 1),
// RatedPosts AS (SELECT fp.*, CASE WHEN fp.UpVotes >= 10 THEN 'Hot' WHEN fp.VoteScore > 0 THEN 'Popular' ELSE 'Normal' END AS PostStatus FROM FilteredPosts fp)
// SELECT rp.OwnerDisplayName, rp.OwnerReputation, rp.Title, rp.PostStatus, rp.CreationDate, rp.AnswerCount FROM RatedPosts rp
// WHERE rp.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' ORDER BY rp.VoteScore DESC, rp.CreationDate DESC LIMIT 50;
//
// rn is partitioned by the post, so it is always 1; the date filter reads only a base column, so it is applied before the answers x votes product.
fn q27082(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let s = db
        .post
        .with(post_type_id.eq(1).and(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))))
        .with(owner_user)
        .group_by(Ident::<Post>::new())
        .select(answers_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let ac = db.post.with(post_type_id.eq(1)).group_by(Ident::<Post>::new()).select(answers_of(db).opt()).buf_fold(distinct_some);
    let v = drain((&s).and(&ac));
    let v = top_n(v, |&(p, (a, _))| (Reverse(a[0] - a[1]), Reverse(creation_date.get(p).unwrap()), p), 50);
    rows(v.into_iter().map(|(p, (a, n))| {
        let mut f = post_fields(db, p, &["owner", "rep", "title"]);
        f.push(V::S(if a[0] >= 10 { "Hot" } else if a[0] - a[1] > 0 { "Popular" } else { "Normal" }));
        f.extend(post_fields(db, p, &["created"]));
        f.push(V::I(n));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, u.DisplayName AS Owner, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.ViewCount DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT PostId, Title, Owner, CreationDate, Score, ViewCount, AnswerCount FROM RankedPosts WHERE Rank <= 10),
// VotesSummary AS (SELECT PostId, COUNT(CASE WHEN VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN VoteTypeId = 3 THEN 1 END) AS DownVotes, COUNT(CASE WHEN VoteTypeId = 8 THEN 1 END) AS BountyStarts
//     FROM Votes GROUP BY PostId),
// PostDetails AS (SELECT tp.PostId, tp.Title, tp.Owner, tp.CreationDate, tp.Score, tp.ViewCount, tp.AnswerCount, COALESCE(vs.UpVotes, 0) AS UpVotes, COALESCE(vs.DownVotes, 0) AS DownVotes,
//        COALESCE(vs.BountyStarts, 0) AS BountyStarts FROM TopPosts tp LEFT JOIN VotesSummary vs ON tp.PostId = vs.PostId)
// SELECT pd.Title, pd.Owner, pd.CreationDate, pd.Score, pd.ViewCount, pd.AnswerCount, pd.UpVotes, pd.DownVotes, pd.BountyStarts,
//        EXTRACT(EPOCH FROM (cast('2024-10-01 12:34:56' as timestamp) - pd.CreationDate)) AS AgeInSeconds
// FROM PostDetails pd ORDER BY pd.ViewCount DESC, pd.Score DESC;
fn q5746(db: &'static So) -> String {
    let Post { creation_date, owner_user, post_type_id, view_count, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let v = drain(db.post.with(creation_date.ge(add_years(t0, -1))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), Reverse(creation_date.get(p).unwrap()), p)
    }, 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let vs = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 3], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + (t == 8) as i64]);
    rows(drain((&tp).select((&vs).opt())).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["title", "owner", "created", "score", "views", "answers"]);
        f.extend(a.unwrap_or([0; 3]).map(V::I));
        f.push(V::F(secs(t0 - creation_date.get(p).unwrap())));
        row(f)
    }))
}

// WITH RECURSIVE UserEngagement AS (SELECT u.Id AS UserId, u.DisplayName, SUM(COALESCE(p.ViewCount, 0)) AS TotalViewCount, COUNT(DISTINCT p.Id) AS TotalPosts,
//        SUM(CASE WHEN v.VoteTypeId IN (2, 3) THEN 1 ELSE 0 END) AS TotalVotes, RANK() OVER (ORDER BY SUM(COALESCE(p.ViewCount, 0)) DESC) AS EngagementRank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// RecentPostHistory AS (SELECT ph.PostId, ph.CreationDate, ph.UserDisplayName, p.Title AS PostTitle, pt.Name AS PostTypeName, ph.PostHistoryTypeId,
//        ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS HistoryRow
//     FROM PostHistory ph INNER JOIN Posts p ON ph.PostId = p.Id INNER JOIN PostHistoryTypes pt ON ph.PostHistoryTypeId = pt.Id)
// SELECT ue.UserId, ue.DisplayName, ue.TotalViewCount, ue.TotalPosts, ue.TotalVotes, php.PostTitle, php.UserDisplayName AS EditorName, php.CreationDate AS EditDate, php.PostTypeName,
//        CASE WHEN php.PostHistoryTypeId = 10 THEN 'Closed' WHEN php.PostHistoryTypeId = 11 THEN 'Reopened' WHEN php.PostHistoryTypeId = 12 THEN 'Deleted' ELSE 'Other Actions' END AS ActionType
// FROM UserEngagement ue LEFT JOIN RecentPostHistory php ON php.HistoryRow = 1 WHERE ue.TotalPosts > 5 AND ue.TotalViewCount > 1000 AND php.PostTypeName = 'Question'
// ORDER BY ue.TotalViewCount DESC, ue.TotalPosts DESC;
//
// WITH RECURSIVE, but no CTE refers to itself. The ON clause names only php, so users are crossed with each post's latest history row; a tie on
// CreationDate goes to the larger history id. PostTypeName is a PostHistoryTypes name.
fn q33954(db: &'static So) -> String {
    let ue = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select((&db.post.view_count).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt()).fold([0i64; 2], |a, p| match p {
        Some((w, t)) => [a[0] + w.unwrap_or(0), a[1] + matches!(t, Some(2 | 3)) as i64],
        None => a,
    });
    let pc = user_distinct_posts(db);
    let PostHistory { post, creation_date: hd, .. } = &db.post_history;
    let latest = top_per(drain(db.post_history.select(post)), |&(_, p)| p, |&(h, _)| (Reverse(hd.get(h).unwrap()), Reverse(h)), 1, false);
    let php = rel(latest.into_iter().map(|x| x.0).collect());
    let q = (&php).filt(|h: Id<PostHistory>| htype_name(db).get(h) == Some("Question"));
    let q = rel(drain(q).into_iter().map(|x| x.1).collect());
    let mut v = Vec::new();
    (&ue).and((&pc).filt(|n| n > 5)).filt(|(a, _): ([i64; 2], i64)| a[0] > 1000).cross(&q).drive(|(u, _), ((a, n), h)| v.push((u, a, n, h)));
    rows(v.into_iter().map(|(u, a, n, h)| {
        let t = db.post_history.post_history_type_id.get(h).unwrap();
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(n), V::I(a[1]), ostr(db.post.title.get(post.get(h).unwrap())), ostr(db.post_history.user_display_name.get(h)), V::T(hd.get(h).unwrap())]);
        f.push(V::S(htype_name(db).get(h).unwrap()));
        f.push(V::S(match t { 10 => "Closed", 11 => "Reopened", 12 => "Deleted", _ => "Other Actions" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.Score > 0),
// TopUsers AS (SELECT OwnerDisplayName, COUNT(*) AS PostCount, SUM(Score) AS TotalScore FROM RankedPosts WHERE PostRank <= 5 GROUP BY OwnerDisplayName HAVING COUNT(*) >= 3),
// RecentVotes AS (SELECT v.PostId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes FROM Votes v
//     WHERE v.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' GROUP BY v.PostId),
// PostDetails AS (SELECT p.Id, p.Title, u.DisplayName AS OwnerDisplayName, COALESCE(rv.UpVotes, 0) AS UpVotes, COALESCE(rv.DownVotes, 0) AS DownVotes, p.CreationDate
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN RecentVotes rv ON p.Id = rv.PostId)
// SELECT pu.OwnerDisplayName, COUNT(pd.Id) AS RecentPostCount, SUM(pd.UpVotes) AS TotalUpVotes, SUM(pd.DownVotes) AS TotalDownVotes
// FROM TopUsers pu JOIN PostDetails pd ON pu.OwnerDisplayName = pd.OwnerDisplayName GROUP BY pu.OwnerDisplayName ORDER BY TotalUpVotes DESC, RecentPostCount DESC;
fn q7453(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, creation_date, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(score.gt(0))).with(owner_user).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let rp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let name = || owner_user.select(&db.user.display_name);
    let tu = (&rp).group_by(name()).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let recent = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.creation_date).ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))));
    let rv = db.post.group_by(Ident::<Post>::new()).select(recent.select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let g = db
        .post
        .with(name().select((&tu).filt(|n| n >= 3)))
        .group_by(name())
        .select((&rv).opt())
        .fold([0i64; 3], |a, r| {
            let r = r.unwrap_or([0, 0]);
            [a[0] + 1, a[1] + r[0], a[2] + r[1]]
        });
    rows(drain(&g).into_iter().map(|(n, a)| row(vec![V::S(n), V::I(a[0]), V::I(a[1]), V::I(a[2])])))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.Score, P.ViewCount, P.CreationDate, ROW_NUMBER() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC) AS RankScore FROM Posts P
//     WHERE P.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'),
// UserReputation AS (SELECT U.Id AS UserId, U.Reputation, U.DisplayName, COUNT(DISTINCT B.Id) AS BadgeCount FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.Reputation, U.DisplayName),
// TopUsers AS (SELECT U.UserId, U.Reputation, U.DisplayName, U.BadgeCount, R.PostId, R.Title, R.Score FROM UserReputation U JOIN RankedPosts R ON U.UserId = R.PostId WHERE U.Reputation > 1000 AND R.RankScore <= 5),
// ClosedPosts AS (SELECT PH.PostId, PH.CreationDate, PH.UserDisplayName, CT.Name AS CloseReason FROM PostHistory PH INNER JOIN CloseReasonTypes CT ON PH.Comment::int = CT.Id WHERE PH.PostHistoryTypeId IN (10, 11)),
// FinalResults AS (SELECT TU.DisplayName, TU.Reputation, TU.BadgeCount, TU.Title, TU.Score, COALESCE(CP.CloseReason, 'Not Closed') AS CloseReason FROM TopUsers TU LEFT JOIN ClosedPosts CP ON TU.PostId = CP.PostId)
// SELECT DisplayName, Reputation, BadgeCount, Title, Score, CloseReason FROM FinalResults ORDER BY Reputation DESC, Score DESC;
//
// `U.UserId = R.PostId` joins a user id to a post id, so it goes through the raw ids. RankScore's score ties go to the smaller id.
fn q32139(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false);
    let rp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let by_raw: HashIdx<i64, Id<Post>> = (&rp).select(&db.post.origid).inv().collect();
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let rsn = comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason);
    let cp: HashIdx<Id<Post>, (Id<PostHistory>, Str)> = db.post_history.with(post_history_type_id.in_v(vec![10, 11])).select(post.and(Ident::<PostHistory>::new().and(&rsn))).map(|(p, h)| (p, h)).collect::<MatSet<(Id<Post>, (Id<PostHistory>, Str))>>().map(|(p, _)| p).inv().select(Same::<(Id<Post>, (Id<PostHistory>, Str))>::new().map(|(_, x)| x)).collect();
    let v = drain(db.user.with((&db.user.reputation).gt(1000)).select((&bc).and((&db.user.origid).select(&by_raw).select(Ident::<Post>::new().and((&cp).opt())))));
    rows(v.into_iter().map(|(u, (b, (p, c)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(b));
        f.extend(post_fields(db, p, &["title", "score"]));
        f.push(V::S(c.map_or("Not Closed", |(_, r)| r)));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId IN (4, 10) THEN 1 ELSE 0 END), 0) AS FlaggedPosts, COALESCE(COUNT(DISTINCT P.Id), 0) AS TotalPosts, ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS UserRank
//     FROM Users U LEFT JOIN Posts P ON P.OwnerUserId = U.Id LEFT JOIN Votes V ON V.UserId = U.Id AND V.PostId = P.Id GROUP BY U.Id, U.DisplayName, U.Reputation),
// ClosedPosts AS (SELECT P.Id AS PostId, P.OwnerUserId, P.Title, PH.CreationDate, PH.Comment AS CloseReason FROM Posts P JOIN PostHistory PH ON P.Id = PH.PostId WHERE PH.PostHistoryTypeId = 10),
// RankedTags AS (SELECT T.TagName, COUNT(P.Id) AS TagUsageCount, ROW_NUMBER() OVER (ORDER BY COUNT(P.Id) DESC) AS TagRank FROM Tags T JOIN Posts P ON P.Tags LIKE '%' || T.TagName || '%' GROUP BY T.TagName)
// SELECT UA.DisplayName, UA.UpVotes, UA.DownVotes, UA.TotalPosts, UA.UserRank, CP.PostId, CP.Title, CP.CloseReason, RT.TagName AS MostUsedTag, RT.TagUsageCount
// FROM UserActivity UA LEFT JOIN ClosedPosts CP ON UA.UserId = CP.OwnerUserId LEFT JOIN (SELECT TagName, TagUsageCount FROM RankedTags WHERE TagRank = 1) RT ON TRUE
// WHERE UA.UpVotes > UA.DownVotes ORDER BY UA.UserRank, UA.TotalPosts DESC;
//
// RT is joined ON TRUE, so it is crossed with every row; the tag-count tie at TagRank 1 and UserRank ties go to the smaller id.
fn q21878(db: &'static So) -> String {
    let ov = own_votes(db);
    let ua = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select((&ov).select(&db.vote.vote_type_id).opt()).opt()).fold([0i64; 2], |a, t| {
        let t = t.flatten();
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
    });
    let pc = user_distinct_posts(db);
    let rk = rel(top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 0).into_iter().enumerate().map(|(i, (u, _))| (u, i as i64 + 1)).collect());
    let rank_of: HashIdx<Id<User>, i64> = (&rk).map(|(u, _)| u).inv().select(&rk).map(|(_, r)| r).collect();
    let tm = tag_mentions(db);
    let tu = (&tm).group_by(Same::<(Id<Post>, Id<Tag>)>::new().map(|(_, t)| t)).select(Same::<(Id<Post>, Id<Tag>)>::new()).fold(0i64, |n, _| n + 1);
    let rt = left_all(top_n(drain(&tu), |&(t, n)| (Reverse(n), t), 1));
    let PostHistory { post, post_history_type_id, .. } = &db.post_history;
    let cp: HashIdx<Id<User>, Id<PostHistory>> = db.post_history.with(post_history_type_id.eq(10)).select(post.select(&db.post.owner_user)).inv().collect();
    let base = (&ua).filt(|a| a[0] > a[1]).and(&pc).and(&rank_of).and((&cp).opt());
    let mut v = Vec::new();
    base.cross(&rt).drive(|(u, _), ((((a, n), r), h), t)| v.push((u, a, n, r, h, t)));
    rows(v.into_iter().map(|(u, a, n, r, h, t)| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(n), V::I(r)];
        f.extend(match h {
            Some(h) => {
                let p = post.get(h).unwrap();
                [V::I(db.post.origid.get(p).unwrap()), ostr(db.post.title.get(p)), ostr(db.post_history.comment.get(h))]
            }
            None => [V::Null, V::Null, V::Null],
        });
        f.extend(match t {
            Some((t, n)) => [V::S(db.tag.tag_name.get(t).unwrap()), V::I(n)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RecursivePostStats AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.ViewCount, P.Score, COUNT(A.Id) AS AnswerCount, MAX(CASE WHEN C.UserId IS NOT NULL THEN 1 ELSE 0 END) AS HasComments,
//        ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS UserPostRank
//     FROM Posts P LEFT JOIN Posts A ON A.ParentId = P.Id LEFT JOIN Comments C ON C.PostId = P.Id WHERE P.PostTypeId = 1 GROUP BY P.Id, P.Title, P.CreationDate, P.ViewCount, P.Score, P.OwnerUserId),
// UserReputation AS (SELECT U.Id AS UserId, U.Reputation, COUNT(DISTINCT P.Id) AS QuestionCount, AVG(P.Score) AS AvgScore FROM Users U JOIN Posts P ON P.OwnerUserId = U.Id WHERE P.PostTypeId = 1 GROUP BY U.Id, U.Reputation)
// SELECT R.PostId, R.Title, R.CreationDate, R.ViewCount, R.Score, R.AnswerCount, R.HasComments, U.UserId, U.Reputation AS UserReputation, U.QuestionCount, U.AvgScore,
//        CASE WHEN U.Reputation IS NULL THEN 'No Reputation' ELSE CASE WHEN U.Reputation >= 1000 THEN 'High Reputation' WHEN U.Reputation BETWEEN 500 AND 999 THEN 'Medium Reputation' ELSE 'Low Reputation' END END AS ReputationLevel
// FROM RecursivePostStats R LEFT JOIN UserReputation U ON U.UserId = R.PostId WHERE R.ViewCount > 10 AND R.AnswerCount >= 1 AND R.UserPostRank <= 5 ORDER BY R.Score DESC, R.ViewCount DESC LIMIT 50;
//
// UserPostRank reads only CreationDate, so each owner's five newest questions are picked first and the answers x comments product is driven for those alone.
// `U.UserId = R.PostId` joins a user id to a post id, so it goes through the raw ids.
fn q31289(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, view_count, score, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1)).select(owner_user.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let rp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&rp)
        .with(view_count.gt(10))
        .group_by(Ident::<Post>::new())
        .select(children_of(db).opt().and(comments_of(db).select((&db.comment.user).opt()).opt()))
        .fold([0i64; 2], |a, (x, c)| [a[0] + x.is_some() as i64, a[1].max(c.flatten().is_some() as i64)]);
    let ur = db.post.with(post_type_id.eq(1)).group_by(owner_user).select(score).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let by_raw: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let v = drain((&s).filt(|a| a[0] >= 1).and((&db.post.origid).select(&by_raw).select(Ident::<User>::new().and(&ur)).opt()));
    let v = top_n(v, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 50);
    rows(v.into_iter().map(|(p, (a, u))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([V::I(a[0]), V::I(a[1])]);
        match u {
            Some((u, q)) => {
                let r = db.user.reputation.get(u).unwrap();
                f.extend(ucols(db, u, &["uid", "rep"]));
                f.extend([V::I(q[0]), avg(q[1], q[0])]);
                f.push(V::S(if r >= 1000 { "High Reputation" } else if r >= 500 { "Medium Reputation" } else { "Low Reputation" }));
            }
            None => f.extend([V::Null, V::Null, V::Null, V::Null, V::S("No Reputation")]),
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS RankScore
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId IN (1, 2) AND p.Score IS NOT NULL),
// TopComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount, MAX(c.CreationDate) AS LastCommentDate FROM Comments c GROUP BY c.PostId),
// ClosedPosts AS (SELECT ph.PostId, COUNT(ph.Id) AS CloseCount FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId),
// PostDetails AS (SELECT rp.Id AS PostId, rp.Title, rp.CreationDate, COALESCE(tc.CommentCount, 0) AS TotalComments, COALESCE(cp.CloseCount, 0) AS TotalCloseCount,
//        CASE WHEN COALESCE(cp.CloseCount, 0) > 0 THEN 'Closed' ELSE 'Open' END AS Status, rp.RankScore FROM RankedPosts rp LEFT JOIN TopComments tc ON rp.Id = tc.PostId LEFT JOIN ClosedPosts cp ON rp.Id = cp.PostId)
// SELECT pd.Title, pd.CreationDate, pd.TotalComments, pd.TotalCloseCount, pd.Status,
//        (SELECT AVG(p.Score) FROM Posts p WHERE p.OwnerUserId = (SELECT p2.OwnerUserId FROM Posts p2 WHERE p2.Id = pd.PostId)) AS AvgOwnerScore
// FROM PostDetails pd WHERE pd.RankScore <= 5 ORDER BY pd.TotalComments DESC, pd.CreationDate ASC LIMIT 10;
fn q4413(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.is_in([1, 2])).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())), 5, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let tc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10)));
    let cc = (&tp).group_by(Ident::<Post>::new()).select(closes.opt()).fold(0i64, |n, h| n + h.is_some() as i64);
    let os = db.post.group_by(owner_user).select(score).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let v = drain((&tc).and(&cc).and(owner_user.select(&os)));
    let v = top_n(v, |&(p, ((c, _), _))| (Reverse(c), creation_date.get(p).unwrap(), p), 10);
    rows(v.into_iter().map(|(p, ((c, k), a))| {
        let mut f = post_fields(db, p, &["title", "created"]);
        f.extend([V::I(c), V::I(k), V::S(if k > 0 { "Closed" } else { "Open" }), avg(a[1], a[0])]);
        row(f)
    }))
}

// WITH TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, ROW_NUMBER() OVER (ORDER BY u.Reputation DESC) AS UserRank FROM Users u WHERE u.Reputation IS NOT NULL),
// UserBadges AS (SELECT b.UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldCount, COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverCount, COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeCount FROM Badges b GROUP BY b.UserId),
// ClosedPosts AS (SELECT p.OwnerUserId, COUNT(*) AS ClosedCount FROM Posts p JOIN PostHistory ph ON p.Id = ph.PostId WHERE ph.PostHistoryTypeId = 10 GROUP BY p.OwnerUserId),
// PostStats AS (SELECT p.OwnerUserId, COUNT(*) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers FROM Posts p GROUP BY p.OwnerUserId)
// SELECT tu.DisplayName, tu.Reputation, COALESCE(ub.GoldCount, 0) AS GoldBadges, COALESCE(ub.SilverCount, 0) AS SilverBadges, COALESCE(ub.BronzeCount, 0) AS BronzeBadges, COALESCE(cp.ClosedCount, 0) AS ClosedPosts,
//        COALESCE(ps.TotalPosts, 0) AS TotalPosts, COALESCE(ps.Questions, 0) AS Questions, COALESCE(ps.Answers, 0) AS Answers,
//        (tu.Reputation * COALESCE(ps.TotalPosts, 0) * 1.0) / NULLIF(COALESCE(cp.ClosedCount, 0), 0) AS PerformanceScore
// FROM TopUsers tu LEFT JOIN UserBadges ub ON tu.UserId = ub.UserId LEFT JOIN ClosedPosts cp ON tu.UserId = cp.OwnerUserId LEFT JOIN PostStats ps ON tu.UserId = ps.OwnerUserId
// WHERE tu.UserRank <= 50 ORDER BY PerformanceScore DESC;
fn q2235(db: &'static So) -> String {
    let v = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 50);
    let tu: MatSet<Id<User>> = rel(v.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let cp = db.post_history.with((&db.post_history.post_history_type_id).eq(10)).group_by((&db.post_history.post).select(&db.post.owner_user)).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let ps = db.post.group_by(&db.post.owner_user).select(&db.post.post_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64]);
    let v = drain((&tu).select((&ub).opt().and((&cp).opt()).and((&ps).opt())));
    rows(v.into_iter().map(|(u, ((b, c), p))| {
        let (b, c, p) = (b.unwrap_or([0; 3]), c.unwrap_or(0), p.unwrap_or([0; 3]));
        let r = db.user.reputation.get(u).unwrap();
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(b.map(V::I));
        f.push(V::I(c));
        f.extend(p.map(V::I));
        f.push(if c == 0 { V::Null } else { V::F((r * p[0]) as f64 / c as f64) });
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U),
// RecentPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.OwnerUserId, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
//        RANK() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS RecentPostRank
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' GROUP BY P.Id, P.Title, P.CreationDate, P.OwnerUserId),
// TopUsers AS (SELECT UR.UserId, UR.DisplayName, UR.Reputation FROM UserReputation UR WHERE UR.Reputation > (SELECT AVG(Reputation) FROM Users)),
// ClosedPosts AS (SELECT PH.PostId, PH.CreationDate AS ClosedDate, P.Title, PH.Comment, ROW_NUMBER() OVER (PARTITION BY PH.PostId ORDER BY PH.CreationDate DESC) AS CloseHistoryRank
//     FROM PostHistory PH JOIN Posts P ON PH.PostId = P.Id WHERE PH.PostHistoryTypeId = 10)
// SELECT UR.DisplayName, UR.Reputation, RP.Title AS RecentPostTitle, RP.CommentCount, CP.ClosedDate, CP.Comment AS CloseReasonComment
// FROM TopUsers UR LEFT JOIN RecentPosts RP ON UR.UserId = RP.OwnerUserId AND RP.RecentPostRank = 1 LEFT JOIN ClosedPosts CP ON RP.PostId = CP.PostId AND CP.CloseHistoryRank = 1
// WHERE UR.Reputation > 1000 ORDER BY UR.Reputation DESC LIMIT 10;
//
// A close tie on CreationDate goes to the larger history id.
fn q4507(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).with(owner_user).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| Reverse(creation_date.get(p).unwrap()), 1, true);
    let rp = rel(top);
    let by_owner: HashIdx<Id<User>, Id<Post>> = (&rp).map(|(_, u)| u).inv().select(&rp).map(|(p, _)| p).collect();
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let cl = top_per(drain(db.post_history.with(post_history_type_id.eq(10)).select(post)), |&(_, p)| p, |&(h, _)| (Reverse(hd.get(h).unwrap()), Reverse(h)), 1, false);
    let cl = rel(cl.into_iter().map(|(h, p)| (p, h)).collect());
    let last: HashIdx<Id<Post>, Id<PostHistory>> = (&cl).map(|(p, _)| p).inv().select(&cl).map(|(_, h)| h).collect();
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let (rs, rn) = db.user.select(&db.user.reputation).fold_flat((0i64, 0i64), |(s, n), r| (s + r, n + 1));
    let tu = db.user.with((&db.user.reputation).filt(move |r| r > 1000 && (r as i128) * (rn as i128) > rs as i128));
    let v = drain(tu.select((&by_owner).select(Ident::<Post>::new().and(&cc).and((&last).opt())).opt()));
    let v = top_n(v, |&(u, _)| Reverse(db.user.reputation.get(u).unwrap()), 10);
    rows(v.into_iter().map(|(u, r)| {
        let mut f = ucols(db, u, &["name", "rep"]);
        match r {
            Some(((p, c), h)) => {
                f.extend([ostr(db.post.title.get(p)), V::I(c)]);
                f.extend(match h {
                    Some(h) => [V::T(hd.get(h).unwrap()), ostr(db.post_history.comment.get(h))],
                    None => [V::Null, V::Null],
                });
            }
            None => f.extend([V::Null, V::Null, V::Null, V::Null]),
        }
        row(f)
    }))
}

// Rewritten (rewrites/34854.sql): FinalResults carries fp.PostId and FinalRank is tie-broken on it.
// WITH RecursivePostCTE AS (SELECT p.Id AS PostId, p.Title, p.AnswerCount, p.ViewCount, p.CreationDate, p.Score, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY COALESCE(SUM(v.BountyAmount), 0) DESC) AS PostRank
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (8, 9) GROUP BY p.Id, p.Title, p.AnswerCount, p.ViewCount, p.CreationDate, p.Score, p.OwnerUserId),
// FilteredPosts AS (SELECT *, CASE WHEN Score > 10 THEN 'High Score' WHEN Score BETWEEN 5 AND 10 THEN 'Medium Score' ELSE 'Low Score' END AS ScoreCategory FROM RecursivePostCTE
//     WHERE CreationDate > DATE '2024-10-01' - INTERVAL '1 year'),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c GROUP BY c.PostId),
// FinalResults AS (SELECT fp.PostId, fp.Title, fp.ViewCount, fp.AnswerCount, fp.TotalBounty, fp.Score, fp.ScoreCategory, COALESCE(pc.CommentCount, 0) AS CommentCount FROM FilteredPosts fp
//     LEFT JOIN PostComments pc ON fp.PostId = pc.PostId WHERE fp.ScoreCategory <> 'Low Score')
// SELECT fr.Title, fr.ViewCount, fr.AnswerCount, fr.TotalBounty, fr.Score, fr.ScoreCategory, fr.CommentCount, ROW_NUMBER() OVER (ORDER BY fr.TotalBounty DESC, fr.Score DESC, fr.PostId) AS FinalRank
// FROM FinalResults fr WHERE fr.CommentCount > 5 ORDER BY fr.Score ASC, fr.ViewCount DESC;
//
// PostRank is never read.
fn q34854(db: &'static So) -> String {
    let Post { creation_date, score, .. } = &db.post;
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt());
    let fr = db
        .post
        .with(creation_date.gt(add_years(date(2024, 10, 1), -1)).and(score.ge(5)))
        .group_by(Ident::<Post>::new())
        .select(bounty.opt())
        .fold(0i64, |n, b| n + b.flatten().unwrap_or(0));
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let v = drain((&fr).and((&cc).filt(|n| n > 5)));
    let v = top_n(v, |&(p, (b, _))| (Reverse(b), Reverse(score.get(p).unwrap()), p), 0);
    rows(v.into_iter().enumerate().map(|(i, (p, (b, c)))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["title", "views", "answers"]);
        f.extend([V::I(b), V::I(s), V::S(if s > 10 { "High Score" } else { "Medium Score" }), V::I(c), V::I(i as i64 + 1)]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes, SUM(CASE WHEN p.PostTypeId = 1 THEN p.AnswerCount ELSE 0 END) AS TotalAnswers
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopBadgedUsers AS (SELECT UserId, COUNT(*) AS BadgeCount, MAX(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadge, MAX(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadge,
//        MAX(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadge FROM Badges b GROUP BY UserId),
// RankedUsers AS (SELECT ur.UserId, ur.DisplayName, ur.Reputation, ur.PostCount, ur.Upvotes, ur.Downvotes, ur.TotalAnswers, tb.BadgeCount, tb.GoldBadge, tb.SilverBadge, tb.BronzeBadge,
//        ROW_NUMBER() OVER (ORDER BY ur.Reputation DESC) AS Rank FROM UserReputation ur LEFT JOIN TopBadgedUsers tb ON ur.UserId = tb.UserId)
// SELECT r.UserId, r.DisplayName, r.Reputation, r.PostCount, r.Upvotes, r.Downvotes, r.TotalAnswers, r.BadgeCount, r.GoldBadge, r.SilverBadge, r.BronzeBadge
// FROM RankedUsers r WHERE r.Rank <= 50 AND (r.BadgeCount IS NOT NULL OR r.Upvotes > 100) ORDER BY r.Reputation DESC;
//
// Rank reads only Reputation, so the fifty users are picked first and the posts x votes product is driven for them alone.
fn q2952(db: &'static So) -> String {
    let v = top_n(drain(db.user.with((&db.user.reputation).gt(1000)).select(&db.user.reputation)), |&(u, r)| (Reverse(r), u), 50);
    let tu: MatSet<Id<User>> = rel(v.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let Post { post_type_id, answer_count, .. } = &db.post;
    let s = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(answer_count.opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 3], |a, p| match p {
            Some(((t, n), v)) => [a[0] + (v == Some(2)) as i64, a[1] + (v == Some(3)) as i64, a[2] + if t == 1 { n.unwrap_or(0) } else { 0 }],
            None => a,
        });
    let pc = user_distinct_posts(db);
    let tb = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 4], |a, c| [a[0] + 1, a[1].max((c == 1) as i64), a[2].max((c == 2) as i64), a[3].max((c == 3) as i64)]);
    let v = drain((&s).and(&pc).and((&tb).opt()).filt(|((a, _), b): (([i64; 3], i64), Option<[i64; 4]>)| b.is_some() || a[0] > 100));
    rows(v.into_iter().map(|(u, ((a, n), b))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        f.extend(match b {
            Some(b) => b.map(V::I),
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(p.Score) AS TotalScore, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, SUM(CASE WHEN b.Id IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// RankedUsers AS (SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, TotalScore, UpVotes, DownVotes, BadgeCount, RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank,
//        RANK() OVER (ORDER BY PostCount DESC) AS PostRank FROM UserPostStats),
// FilteredUsers AS (SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, TotalScore, UpVotes, DownVotes, BadgeCount, ScoreRank, PostRank FROM RankedUsers WHERE ScoreRank <= 10 OR PostRank <= 10)
// SELECT fu.DisplayName, fu.PostCount, fu.QuestionCount, fu.AnswerCount, fu.TotalScore, fu.UpVotes, fu.DownVotes, fu.BadgeCount, (CAST(fu.TotalScore AS decimal) / NULLIF(fu.PostCount, 0)) AS ScorePerPost,
//        (fu.UpVotes - fu.DownVotes) AS NetVotes FROM FilteredUsers fu ORDER BY fu.ScoreRank, fu.PostRank;
fn q28731(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 7], |a, (p, b)| {
            let b = b.is_some() as i64;
            match p {
                Some(((t, s), v)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + (v == Some(2)) as i64, a[5] + (v == Some(3)) as i64, a[6] + b],
                None => [a[0], a[1], a[2], a[3], a[4], a[5], a[6] + b],
            }
        });
    let v = ranked(drain(&s), |&(_, a)| (a[0] == 0, Reverse(a[3])), false);
    let v = ranked(v, |&((_, a), _)| Reverse(a[0]), false);
    let v = drain(rel(v).filt(|(((_, _), s), p): (((Id<User>, [i64; 7]), i64), i64)| s <= 10 || p <= 10)).into_iter().map(|x| x.1);
    rows(v.map(|(((u, a), _), _)| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), nullable(a[3], a[0]), V::I(a[4]), V::I(a[5]), V::I(a[6])];
        f.push(if a[0] == 0 { V::Null } else { V::F(a[3] as f64 / a[0] as f64) });
        f.push(V::I(a[4] - a[5]));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(CASE WHEN P.PostTypeId IN (4, 5) THEN 1 ELSE 0 END) AS TagWikis, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, SUM(CASE WHEN P.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, Questions, Answers, TagWikis, UpVotes, DownVotes, AcceptedAnswers, RANK() OVER (ORDER BY Reputation DESC) AS RankByReputation
//     FROM UserStats WHERE Reputation > 1000),
// UserBadges AS (SELECT U.Id AS UserId, COUNT(B.Id) AS BadgeCount FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id)
// SELECT TU.UserId, TU.DisplayName, TU.Reputation, TU.TotalPosts, TU.Questions, TU.Answers, TU.TagWikis, TU.UpVotes, TU.DownVotes, TU.AcceptedAnswers, UB.BadgeCount, TU.RankByReputation
// FROM TopUsers TU JOIN UserBadges UB ON TU.UserId = UB.UserId WHERE UB.BadgeCount > 5 ORDER BY TU.RankByReputation;
fn q5711(db: &'static So) -> String {
    let Post { post_type_id, accepted_answer, .. } = &db.post;
    let rich = || db.user.with((&db.user.reputation).gt(1000));
    let s = rich()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(accepted_answer.opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 6], |a, p| match p {
            Some(((t, acc), v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + matches!(t, 4 | 5) as i64, a[3] + (v == Some(2)) as i64, a[4] + (v == Some(3)) as i64, a[5] + acc.is_some() as i64],
            None => a,
        });
    let pc = user_distinct_posts(db);
    let rk = ranked(drain(rich().select(&db.user.reputation)), |&(_, r)| Reverse(r), false);
    let rk = rel(rk.into_iter().map(|((u, _), r)| (u, r)).collect());
    let rank_of: HashIdx<Id<User>, i64> = (&rk).map(|(u, _)| u).inv().select(&rk).map(|(_, r)| r).collect();
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = drain((&s).and(&pc).and((&bc).filt(|n| n > 5)).and(&rank_of));
    rows(v.into_iter().map(|(u, (((a, n), b), r))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.extend([V::I(b), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank, COUNT(c.Id) AS CommentCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.OwnerUserId),
// FilteredPosts AS (SELECT rp.Id, rp.Title, rp.CreationDate, rp.Score, rp.PostRank, rp.CommentCount, rp.UpVoteCount, rp.DownVoteCount FROM RankedPosts rp
//     WHERE rp.PostRank = 1 AND rp.Score > (SELECT AVG(Score) FROM Posts) AND rp.CommentCount IS NOT NULL),
// FinalResults AS (SELECT fp.Id, fp.Title, fp.CreationDate, fp.Score, fp.CommentCount, fp.UpVoteCount, fp.DownVoteCount,
//        CASE WHEN fp.Score > 100 THEN 'Hot' WHEN fp.Score BETWEEN 50 AND 100 THEN 'Trending' ELSE 'Normal' END AS PostCategory, DENSE_RANK() OVER (ORDER BY fp.Score DESC) AS ScoreRank FROM FilteredPosts fp)
// SELECT fr.Id, fr.Title, fr.CreationDate, fr.Score, fr.CommentCount, fr.UpVoteCount, fr.DownVoteCount, fr.PostCategory, fr.ScoreRank FROM FinalResults fr WHERE fr.ScoreRank <= 10 ORDER BY fr.Score DESC;
//
// PostRank and ScoreRank read only base columns, so the posts are picked first and the comment x vote product is driven for them alone.
fn q440(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, .. } = &db.post;
    let first = top_per(drain(db.post.select(owner_user.opt())), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let first: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let (ss, sn) = db.post.select(score).fold_flat((0i64, 0i64), |(s, n), x| (s + x, n + 1));
    let fp = drain((&first).select(score.filt(move |s| (s as i128) * (sn as i128) > ss as i128)));
    let v = ranked(fp, |&(_, s)| Reverse(s), true);
    let top = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|((p, _), r)| (p, r)).collect());
    let pids: MatSet<Id<Post>> = (&top).map(|(p, _)| p).collect();
    let s = (&pids)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    type R = (Id<Post>, i64);
    let v = drain((&top).select(Same::<R>::new().and(Same::<R>::new().map(|(p, _): R| p).select(&s))));
    rows(v.into_iter().map(|(_, ((p, r), a))| {
        let sc = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend(a.map(V::I));
        f.push(V::S(if sc > 100 { "Hot" } else if sc >= 50 { "Trending" } else { "Normal" }));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.UpVotes, U.DownVotes, U.Views, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS UserRank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName, U.Reputation, U.UpVotes, U.DownVotes, U.Views),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, UserRank FROM UserStats WHERE UserRank <= 100),
// QuestionStats AS (SELECT P.OwnerUserId, COUNT(CASE WHEN PH.PostHistoryTypeId = 10 THEN 1 END) AS CloseCount, COUNT(CASE WHEN PH.PostHistoryTypeId = 11 THEN 1 END) AS ReopenCount,
//        COUNT(CASE WHEN PH.PostHistoryTypeId IN (12, 13) THEN 1 END) AS DeleteCount FROM Posts P LEFT JOIN PostHistory PH ON P.Id = PH.PostId WHERE P.PostTypeId = 1 GROUP BY P.OwnerUserId)
// SELECT TU.DisplayName, TU.Reputation, TU.PostCount, TU.QuestionCount, TU.AnswerCount, COALESCE(QS.CloseCount, 0) AS CloseCount, COALESCE(QS.ReopenCount, 0) AS ReopenCount,
//        COALESCE(QS.DeleteCount, 0) AS DeleteCount, (TU.Reputation + COALESCE(QS.CloseCount, 0) * -5 + COALESCE(QS.ReopenCount, 0) * 5 + COALESCE(QS.DeleteCount, 0) * -10) AS AdjustedReputation
// FROM TopUsers TU LEFT JOIN QuestionStats QS ON TU.UserId = QS.OwnerUserId ORDER BY AdjustedReputation DESC LIMIT 50;
fn q1497(db: &'static So) -> String {
    let v = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 100);
    let tu: MatSet<Id<User>> = rel(v.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let ups = user_posts(db);
    let qs = db
        .post
        .with((&db.post.post_type_id).eq(1))
        .group_by(&db.post.owner_user)
        .select(history_of(db).select(&db.post_history.post_history_type_id).opt())
        .fold([0i64; 3], |a, t| [a[0] + (t == Some(10)) as i64, a[1] + (t == Some(11)) as i64, a[2] + matches!(t, Some(12 | 13)) as i64]);
    let v = drain((&tu).select((&ups).and((&qs).opt())));
    let adj = |u: Id<User>, q: [i64; 3]| db.user.reputation.get(u).unwrap() - 5 * q[0] + 5 * q[1] - 10 * q[2];
    let v = top_n(v, |&(u, (_, q))| (Reverse(adj(u, q.unwrap_or([0; 3]))), u), 50);
    rows(v.into_iter().map(|(u, (a, q))| {
        let q = q.unwrap_or([0; 3]);
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3])]);
        f.extend(q.map(V::I));
        f.push(V::I(adj(u, q)));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, p.LastActivityDate, p.OwnerUserId,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS RankScore FROM Posts p WHERE p.Score IS NOT NULL AND p.OwnerUserId IS NOT NULL),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation),
// TopRankedPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.CreationDate, rp.LastActivityDate, u.DisplayName AS OwnerDisplayName, ur.Reputation AS OwnerReputation, ur.BadgeCount
//     FROM RankedPosts rp JOIN Users u ON rp.OwnerUserId = u.Id JOIN UserReputation ur ON u.Id = ur.UserId WHERE rp.RankScore <= 10)
// SELECT trp.Title, trp.Score, trp.ViewCount, trp.CreationDate, trp.LastActivityDate, trp.OwnerDisplayName, trp.OwnerReputation, trp.BadgeCount, COUNT(CASE WHEN c.PostId IS NOT NULL THEN 1 END) AS CommentCount,
//        COUNT(DISTINCT v.Id) AS VoteCount
// FROM TopRankedPosts trp LEFT JOIN Comments c ON trp.PostId = c.PostId LEFT JOIN Votes v ON trp.PostId = v.PostId
// GROUP BY trp.PostId, trp.Title, trp.Score, trp.ViewCount, trp.CreationDate, trp.LastActivityDate, trp.OwnerDisplayName, trp.OwnerReputation, trp.BadgeCount ORDER BY trp.Score DESC, trp.ViewCount DESC;
fn q8838(db: &'static So) -> String {
    let Post { owner_user, post_type_id, score, creation_date, .. } = &db.post;
    let v = drain(db.post.with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).opt()).buf_fold(distinct_some);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = drain((&cc).and(&vc).and(owner_user.select(&bc)));
    rows(v.into_iter().map(|(p, ((c, n), b))| {
        let mut f = post_fields(db, p, &["title", "score", "views", "created", "activity", "owner", "rep"]);
        f.extend([V::I(b), V::I(c), V::I(n)]);
        row(f)
    }))
}

// WITH UserVoteStats AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes,
//        COUNT(DISTINCT p.Id) AS PostCount FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Posts p ON v.PostId = p.Id GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, Upvotes, Downvotes, ROW_NUMBER() OVER (ORDER BY Upvotes DESC) AS RN FROM UserVoteStats WHERE PostCount > 10),
// PostActivity AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COALESCE(COUNT(c.Id), 0) AS CommentCount, COALESCE(SUM(bt.Class), 0) AS TotalBadges, COUNT(DISTINCT p.Id) AS PostCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Badges bt ON p.OwnerUserId = bt.UserId GROUP BY p.Id, p.Title, p.CreationDate),
// BalancedPerformance AS (SELECT tu.DisplayName, tu.Upvotes, tu.Downvotes, uvs.PostCount, pa.CommentCount, pa.TotalBadges, (tu.Upvotes - tu.Downvotes) AS NetVotes,
//        DENSE_RANK() OVER (ORDER BY (tu.Upvotes - tu.Downvotes) DESC) AS NetVoteRank
//     FROM TopUsers tu JOIN UserVoteStats uvs ON tu.UserId = uvs.UserId JOIN PostActivity pa ON uvs.PostCount > 5 WHERE tu.RN <= 10)
// SELECT bp.DisplayName, bp.Upvotes, bp.Downvotes, bp.NetVotes, bp.CommentCount, bp.TotalBadges, bp.NetVoteRank FROM BalancedPerformance bp WHERE bp.NetVoteRank <= 5 ORDER BY bp.NetVotes DESC, bp.CommentCount DESC;
//
// The PostActivity join names only uvs, so the top users are crossed with every post. The Upvotes tie at RN 10 goes to the smaller user id.
fn q9713(db: &'static So) -> String {
    let uvs = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let vp = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.post).opt()).buf_fold(distinct_some);
    let tu = top_n(drain((&uvs).and((&vp).filt(|n| n > 10))), |&(u, (a, _))| (Reverse(a[0]), u), 10);
    let tu = ranked(tu, |&(_, (a, _))| Reverse(a[0] - a[1]), true);
    type R = ((Id<User>, ([i64; 2], i64)), i64);
    let tu = rel(tu);
    let bp = (&tu).filt(|((_, (_, n)), r): R| n > 5 && r <= 5);
    let pa = db
        .post
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and((&db.post.owner_user).select(badges_of(db).select(&db.badge.class)).opt()))
        .fold([0i64; 2], |a, (c, b)| [a[0] + c.is_some() as i64, a[1] + b.unwrap_or(0)]);
    let pa = rel(drain(&pa).into_iter().map(|x| x.1).collect());
    let mut v = Vec::new();
    bp.cross(&pa).drive(|_, (((u, (a, _)), r), c)| v.push((u, a, r, c)));
    rows(v.into_iter().map(|(u, a, r, c)| row(vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[0] - a[1]), V::I(c[0]), V::I(c[1]), V::I(r)])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, COUNT(c.Id) AS CommentCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges, COALESCE(SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadges,
//        COALESCE(SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// PostComments AS (SELECT pc.PostId, COUNT(pc.Id) AS TotalComments FROM Comments pc GROUP BY pc.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, us.DisplayName AS OwnerName, us.Reputation AS OwnerReputation, us.GoldBadges, us.SilverBadges, us.BronzeBadges, COALESCE(pc.TotalComments, 0) AS TotalComments,
//        CASE WHEN rp.PostRank = 1 THEN 'Most Recent' ELSE 'Earlier Post' END AS RecentClassification
// FROM RankedPosts rp JOIN UserStats us ON rp.OwnerUserId = us.UserId LEFT JOIN PostComments pc ON rp.PostId = pc.PostId WHERE rp.CommentCount > 5 ORDER BY rp.CreationDate DESC LIMIT 100 OFFSET 0;
fn q4225(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let rp = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let first = top_per(drain(rp().select(owner_user.opt())), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let first: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = rp().group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    let v = drain((&cc).filt(|n| n > 5).and(owner_user.select(Ident::<User>::new().and(&ub))).and(Ident::<Post>::new().with(&first).opt()));
    let v = top_n(v, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 100);
    rows(v.into_iter().map(|(p, ((c, (u, b)), r))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend(b.map(V::I));
        f.extend([V::I(c), V::S(if r.is_some() { "Most Recent" } else { "Earlier Post" })]);
        row(f)
    }))
}

// WITH UserBadges AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadgeCount, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadgeCount,
//        SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadgeCount FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// PostStats AS (SELECT P.OwnerUserId, COUNT(P.Id) AS PostCount, SUM(P.ViewCount) AS TotalViews, AVG(P.Score) AS AverageScore, MAX(P.CreationDate) AS LatestPostDate FROM Posts P GROUP BY P.OwnerUserId),
// CombinedStats AS (SELECT U.Id AS UserId, U.DisplayName, COALESCE(UB.BadgeCount, 0) AS TotalBadges, COALESCE(PS.PostCount, 0) AS TotalPosts, COALESCE(PS.TotalViews, 0) AS ViewsFromPosts,
//        COALESCE(PS.AverageScore, 0) AS AveragePostScore, ROW_NUMBER() OVER (ORDER BY COALESCE(UB.BadgeCount, 0) DESC, COALESCE(PS.TotalViews, 0) DESC) AS Rank
//     FROM Users U LEFT JOIN UserBadges UB ON U.Id = UB.UserId LEFT JOIN PostStats PS ON U.Id = PS.OwnerUserId)
// SELECT C.UserId, C.DisplayName, C.TotalBadges, C.TotalPosts, C.ViewsFromPosts, C.AveragePostScore,
//        CASE WHEN C.AveragePostScore IS NULL THEN 'No Score' WHEN C.AveragePostScore > 5 THEN 'High Scorer' ELSE 'Low Scorer' END AS ScoreCategory,
//        CASE WHEN C.Rank <= 10 THEN 'Top User' ELSE 'Regular User' END AS UserCategory
// FROM CombinedStats C WHERE C.TotalPosts > 0 ORDER BY C.Rank OFFSET 0 ROWS FETCH NEXT 20 ROWS ONLY;
fn q20365(db: &'static So) -> String {
    let ups = user_posts(db);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = ranked(drain((&bc).and(&ups)), |&(u, (b, a))| (Reverse(b), Reverse(a[6]), u), false);
    let v = drain(rel(v).filt(|((_, (_, a)), _): ((Id<User>, (i64, [i64; 10])), i64)| a[1] > 0)).into_iter().map(|x| x.1);
    let v = top_n(v.collect(), |&(_, r)| r, 20);
    rows(v.into_iter().map(|((u, (b, a)), r)| {
        let avgs = a[4] as f64 / a[1] as f64;
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(b), V::I(a[1]), V::I(a[6]), V::F(avgs)]);
        f.push(V::S(if avgs > 5.0 { "High Scorer" } else { "Low Scorer" }));
        f.push(V::S(if r <= 10 { "Top User" } else { "Regular User" }));
        row(f)
    }))
}

// WITH UserBadges AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// TopQuestions AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.CreationDate, p.Score, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS ScoreRank FROM Posts p WHERE p.PostTypeId = 1 AND p.Score > 0),
// RecentActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS QuestionCount, COUNT(DISTINCT c.Id) AS CommentCount FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId AND p.PostTypeId = 1
//     LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY u.Id, u.DisplayName)
// SELECT ub.UserId, ub.DisplayName, ub.BadgeCount, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges, ra.QuestionCount, ra.CommentCount, q.Title AS TopQuestionTitle, q.Score AS TopQuestionScore
// FROM UserBadges ub LEFT JOIN RecentActivity ra ON ub.UserId = ra.UserId LEFT JOIN TopQuestions q ON ub.UserId = q.OwnerUserId AND q.ScoreRank = 1
// WHERE ub.BadgeCount > 0 ORDER BY ub.BadgeCount DESC, ra.QuestionCount DESC NULLS LAST;
//
// The WHERE on p.CreationDate makes RecentActivity's LEFT JOIN an inner one: only users with a recent question have a row.
fn q1193(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class)).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    let Post { post_type_id, creation_date, score, owner_user, .. } = &db.post;
    let rq = || posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))));
    let ra = db.user.with(rq()).group_by(Ident::<User>::new()).select(rq().opt()).buf_fold(distinct_some);
    let rc = db.user.group_by(Ident::<User>::new()).select(rq().select(comments_of(db).opt())).buf_fold(distinct_some);
    let tq = top_per(drain(db.post.with(post_type_id.eq(1).and(score.gt(0))).select(owner_user)), |&(_, u)| u, |&(p, _)| Reverse(score.get(p).unwrap()), 1, true);
    let tq = rel(tq.into_iter().map(|(p, u)| (u, p)).collect());
    let q: HashIdx<Id<User>, Id<Post>> = (&tq).map(|(u, _)| u).inv().select(&tq).map(|(_, p)| p).collect();
    let v = drain((&ub).and((&ra).and(&rc).opt()).and((&q).opt()));
    rows(v.into_iter().map(|(u, ((b, r), p))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(b.map(V::I));
        f.extend(match r {
            Some((n, c)) => [V::I(n), V::I(c)],
            None => [V::Null, V::Null],
        });
        f.extend(match p {
            Some(p) => post_fields(db, p, &["title", "score"]),
            None => vec![V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserBadges AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// MostActivePosts AS (SELECT P.OwnerUserId, COUNT(P.Id) AS PostCount, SUM(P.ViewCount) AS TotalViews, AVG(P.Score) AS AvgScore FROM Posts P
//     WHERE P.CreationDate >= cast('2024-10-01' as date) - interval '1 year' AND P.PostTypeId = 1 GROUP BY P.OwnerUserId),
// TopUsers AS (SELECT U.Id, U.DisplayName, U.Reputation, UBadges.BadgeCount, MActive.PostCount, MActive.TotalViews, MActive.AvgScore, RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank
//     FROM Users U JOIN UserBadges UBadges ON U.Id = UBadges.UserId JOIN MostActivePosts MActive ON U.Id = MActive.OwnerUserId WHERE U.Reputation > (SELECT AVG(Reputation) FROM Users))
// SELECT U.Id, U.DisplayName, U.Reputation, U.BadgeCount, U.PostCount, U.TotalViews, U.AvgScore, U.ReputationRank, COALESCE(COUNT(C.Id), 0) AS ContributionComments
// FROM TopUsers U LEFT JOIN Comments C ON U.Id = C.UserId GROUP BY U.Id, U.DisplayName, U.Reputation, U.BadgeCount, U.PostCount, U.TotalViews, U.AvgScore, U.ReputationRank ORDER BY U.ReputationRank LIMIT 10;
fn q8964(db: &'static So) -> String {
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let Post { creation_date, post_type_id, owner_user, view_count, score, .. } = &db.post;
    let ma = db
        .post
        .with(creation_date.ge(add_years(date(2024, 10, 1), -1)).and(post_type_id.eq(1)))
        .group_by(owner_user)
        .select(view_count.opt().and(score))
        .fold([0i64; 4], |a, (w, s)| [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s]);
    let (rs, rn) = db.user.select(&db.user.reputation).fold_flat((0i64, 0i64), |(s, n), r| (s + r, n + 1));
    let tu = drain(db.user.with((&db.user.reputation).filt(move |r| (r as i128) * (rn as i128) > rs as i128)).select((&bc).and(&ma)));
    let tu = ranked(tu, |&(u, _)| Reverse(db.user.reputation.get(u).unwrap()), false);
    let tu = top_n(tu, |&(_, r)| r, 10);
    let tu = rel(tu.into_iter().map(|((u, x), r)| (u, (x, r))).collect());
    let cc = db.user.group_by(Ident::<User>::new()).select(comments_by(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    type R = (Id<User>, ((i64, [i64; 4]), i64));
    let v = drain((&tu).select(Same::<R>::new().and(Same::<R>::new().map(|(u, _): R| u).select(&cc))));
    rows(v.into_iter().map(|(_, ((u, ((b, a), r)), c))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(b), V::I(a[0]), nullable(a[2], a[1]), avg(a[3], a[0]), V::I(r), V::I(c)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(coalesce(vs.UpVotes, 0)) AS TotalUpVotes, SUM(coalesce(vs.DownVotes, 0)) AS TotalDownVotes,
//        SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId
//     LEFT JOIN (SELECT UserId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes GROUP BY UserId) vs ON u.Id = vs.UserId
//     LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// RankedUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, Questions, Answers, TotalUpVotes, TotalDownVotes, GoldBadges, SilverBadges, BronzeBadges,
//        ROW_NUMBER() OVER (ORDER BY Reputation DESC, TotalPosts DESC) as Rank FROM UserStats)
// SELECT Rank, DisplayName, Reputation, TotalPosts, Questions, Answers, TotalUpVotes, TotalDownVotes, GoldBadges, SilverBadges, BronzeBadges FROM RankedUsers WHERE Rank <= 10 ORDER BY Rank;
//
// Rank reads Reputation and the distinct post count, so the ten users are picked first and the posts x badges product is driven for them alone.
fn q9556(db: &'static So) -> String {
    let pc = user_distinct_posts(db);
    let v = top_n(drain((&db.user.reputation).and(&pc)), |&(u, (r, n))| (Reverse(r), Reverse(n), u), 10);
    let tu = rel(v.into_iter().enumerate().map(|(i, (u, (_, n)))| (u, (i as i64 + 1, n))).collect());
    let cand: MatSet<Id<User>> = (&tu).map(|(u, _)| u).collect();
    let vs = db.vote.group_by(&db.vote.user).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let s = (&cand)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(&db.post.post_type_id).opt().and((&vs).opt()).and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 7], |a, ((t, w), c)| {
            let w = w.unwrap_or([0, 0]);
            [a[0] + (t == Some(1)) as i64, a[1] + (t == Some(2)) as i64, a[2] + w[0], a[3] + w[1], a[4] + (c == Some(1)) as i64, a[5] + (c == Some(2)) as i64, a[6] + (c == Some(3)) as i64]
        });
    type R = (Id<User>, (i64, i64));
    let v = drain((&tu).select(Same::<R>::new().and(Same::<R>::new().map(|(u, _): R| u).select(&s))));
    rows(v.into_iter().map(|(_, ((u, (r, n)), a))| {
        let mut f = vec![V::I(r)];
        f.extend(ucols(db, u, &["name", "rep"]));
        f.push(V::I(n));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostsCreated, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersGiven,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsAsked, SUM(CASE WHEN C.Id IS NOT NULL THEN 1 ELSE 0 END) AS CommentsMade, SUM(CASE WHEN V.VoteTypeId IN (2, 3) THEN 1 ELSE 0 END) AS VotesReceived,
//        RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId AND V.UserId = U.Id GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostsCreated, AnswersGiven, QuestionsAsked, CommentsMade, VotesReceived, ReputationRank FROM UserActivity WHERE Reputation > 100)
// SELECT T.UserId, T.DisplayName, T.Reputation, T.PostsCreated, T.AnswersGiven, T.QuestionsAsked, T.CommentsMade, T.VotesReceived, T.ReputationRank, PT.Name AS PostTypeName, COUNT(P.Id) AS TotalPostsOfType
// FROM TopUsers T LEFT JOIN Posts P ON T.UserId = P.OwnerUserId LEFT JOIN PostTypes PT ON P.PostTypeId = PT.Id
// GROUP BY T.UserId, T.DisplayName, T.Reputation, T.PostsCreated, T.AnswersGiven, T.QuestionsAsked, T.CommentsMade, T.VotesReceived, T.ReputationRank, PT.Name ORDER BY T.Reputation DESC, TotalPostsOfType DESC LIMIT 10;
//
// Every user has at least one output row and the rows come in reputation order, so the ten best-reputed users are picked first and the posts x comments x
// votes product is driven for them alone.
fn q5320(db: &'static So) -> String {
    let rk = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let rk = rel(rk.into_iter().map(|((u, _), r)| (u, r)).collect());
    let rank_of: HashIdx<Id<User>, i64> = (&rk).map(|(u, _)| u).inv().select(&rk).map(|(_, r)| r).collect();
    let v = top_n(drain(db.user.with((&db.user.reputation).gt(100)).select(&db.user.reputation)), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(v.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let ov = own_votes(db);
    let ua = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(comments_of(db).opt()).and((&ov).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some(((t, c), v)) => [a[0] + (t == 2) as i64, a[1] + (t == 1) as i64, a[2] + c.is_some() as i64, a[3] + matches!(v, Some(2 | 3)) as i64],
            None => a,
        });
    let pc = user_distinct_posts(db);
    type J = (Id<User>, Option<Id<Post>>);
    let j: MatSet<J> = (&tu).select(Ident::<User>::new().and(posts_of(db).opt())).collect();
    let g = (&j)
        .group_by((&j).map(|(u, _): J| u).and((&j).flat_map(|(_, p): J| p).select(ptype_name(db)).opt()))
        .select(Same::<J>::new())
        .fold(0i64, |n, (_, p)| n + p.is_some() as i64);
    let v = drain((&g).and(Same::<(Id<User>, Option<Str>)>::new().map(|(u, _)| u).select((&ua).and(&pc).and(&rank_of))));
    let v = top_n(v, |&((u, t), (n, _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(n), u, t), 10);
    rows(v.into_iter().map(|((u, t), (n, ((a, c), r)))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.push(V::I(c));
        f.extend(a.map(V::I));
        f.push(V::I(r));
        f.extend([ostr(t), V::I(n)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.PostTypeId IN (1, 2)),
// PopularTags AS (SELECT t.TagName, COUNT(p.Id) AS PostCount FROM Tags t JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' GROUP BY t.TagName ORDER BY PostCount DESC LIMIT 10),
// UserContributions AS (SELECT u.DisplayName, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN b.Id IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId
//     GROUP BY u.DisplayName HAVING SUM(CASE WHEN p.PostTypeId IN (1, 2) THEN 1 ELSE 0 END) > 0)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.OwnerDisplayName, pt.TagName, uc.QuestionCount, uc.AnswerCount, uc.BadgeCount
// FROM RankedPosts rp JOIN PopularTags pt ON rp.Title LIKE '%' || pt.TagName || '%' JOIN UserContributions uc ON rp.OwnerDisplayName = uc.DisplayName WHERE rp.Rank <= 5
// ORDER BY rp.Score DESC, rp.ViewCount DESC;
//
// UserContributions is grouped by display name; the posts x badges product is driven only for the users sharing a name with a ranked post's owner.
fn q5239(db: &'static So) -> String {
    let Post { creation_date, owner_user, post_type_id, score, view_count, title, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.is_in([1, 2]))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 5, false);
    let rp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let tm = tag_mentions(db);
    let tc = (&tm).group_by(Same::<(Id<Post>, Id<Tag>)>::new().map(|(_, t)| t)).select(Same::<(Id<Post>, Id<Tag>)>::new()).fold(0i64, |n, _| n + 1);
    let pt = top_n(drain(&tc), |&(_, n)| Reverse(n), 10);
    let pt = rel(pt.into_iter().map(|(t, _)| db.tag.tag_name.get(t).unwrap()).collect());
    let ptn: HashIdx<Str, Str> = (&pt).map(|t| t).inv().select(&pt).collect();
    let titles: MatSet<Str> = (&rp).select(title).collect();
    let like: HashIdx<Str, Str> = (&titles).select_where(&ptn, |ti: Str, t: Str| ti.contains(t)).collect();
    let names: MatSet<Str> = (&rp).select(owner_user.select(&db.user.display_name)).collect();
    let uc = db
        .user
        .with((&db.user.display_name).select(&names))
        .group_by(&db.user.display_name)
        .select(posts_of(db).select(post_type_id).opt().and(badges_of(db).opt()))
        .fold([0i64; 4], |a, (t, b)| [a[0] + (t == Some(1)) as i64, a[1] + (t == Some(2)) as i64, a[2] + b.is_some() as i64, a[3] + matches!(t, Some(1 | 2)) as i64]);
    let v = drain((&rp).select(title.select(&like).and(owner_user.select(&db.user.display_name).select((&uc).filt(|a| a[3] > 0)))));
    rows(v.into_iter().map(|(p, (t, a))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "owner"]);
        f.extend([V::S(t), V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        row(f)
    }))
}

// WITH RECURSIVE UserPostSummary AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(COALESCE(P.Score, 0)) AS TotalScore FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName),
// RecentPosts AS (SELECT P.Id AS PostId, P.OwnerUserId, P.CreationDate, P.Title, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS rn FROM Posts P
//     WHERE P.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 DAY'),
// TopUsers AS (SELECT U.UserId, U.DisplayName, UPS.PostCount, UPS.QuestionCount, UPS.AnswerCount, UPS.TotalScore, RP.Title AS RecentPostTitle FROM UserPostSummary UPS
//     JOIN (SELECT Id AS UserId, DisplayName FROM Users WHERE Reputation > 1000) U ON UPS.UserId = U.UserId LEFT JOIN RecentPosts RP ON U.UserId = RP.OwnerUserId AND RP.rn = 1 WHERE UPS.TotalScore > 50)
// SELECT TU.DisplayName, TU.PostCount, TU.QuestionCount, TU.AnswerCount, TU.TotalScore, COALESCE(TU.RecentPostTitle, 'No recent posts') AS RecentPostTitle,
//        CASE WHEN TU.TotalScore > 100 THEN 'High Scorer' WHEN TU.TotalScore BETWEEN 50 AND 100 THEN 'Medium Scorer' ELSE 'Low Scorer' END AS ScoreCategory
// FROM TopUsers TU ORDER BY TU.TotalScore DESC, TU.DisplayName ASC;
//
// WITH RECURSIVE, but no CTE refers to itself.
fn q30732(db: &'static So) -> String {
    let ups = user_posts(db);
    let Post { creation_date, owner_user, .. } = &db.post;
    let rp = top_per(drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).with(owner_user).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let rp = rel(rp.into_iter().map(|(p, u)| (u, p)).collect());
    let last: HashIdx<Id<User>, Id<Post>> = (&rp).map(|(u, _)| u).inv().select(&rp).map(|(_, p)| p).collect();
    let v = drain(db.user.with((&db.user.reputation).gt(1000)).select((&ups).filt(|a| a[4] > 50).and((&last).opt())));
    rows(v.into_iter().map(|(u, (a, p))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4])];
        f.push(V::S(p.and_then(|p| db.post.title.get(p)).unwrap_or("No recent posts")));
        f.push(V::S(if a[4] > 100 { "High Scorer" } else { "Medium Scorer" }));
        row(f)
    }))
}

// WITH RecursivePostHistory AS (SELECT p.Id AS PostId, ph.CreationDate AS HistoryDate, ph.PostHistoryTypeId, ph.UserId, ph.Comment, ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY ph.CreationDate DESC) AS RN
//     FROM Posts p JOIN PostHistory ph ON p.Id = ph.PostId WHERE p.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year')),
// UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounties, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName),
// PostsWithVoteCount AS (SELECT p.Id AS PostId, p.Title, COUNT(v.Id) AS VoteCount FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.Title)
// SELECT u.UserId, u.DisplayName, u.BadgeCount, u.TotalBounties, u.UpVotes, u.DownVotes, pp.PostId, pp.Title, pp.VoteCount, RPH.HistoryDate, RPH.PostHistoryTypeId, RPH.Comment
// FROM UserStatistics u LEFT JOIN PostsWithVoteCount pp ON u.UserId = pp.PostId LEFT JOIN RecursivePostHistory RPH ON pp.PostId = RPH.PostId AND RPH.RN = 1
// WHERE (u.UpVotes > u.DownVotes OR u.BadgeCount > 0) AND RPH.PostHistoryTypeId NOT IN (10, 12) ORDER BY u.BadgeCount DESC, pp.VoteCount DESC;
//
// `u.UserId = pp.PostId` joins a user id to a post id, so it goes through the raw ids. RN's CreationDate ties go to the larger history id.
fn q33006(db: &'static So) -> String {
    let us = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt().and(votes_by(db).select((&db.vote.vote_type_id).and((&db.vote.bounty_amount).opt())).opt())).fold([0i64; 4], |a, (b, v)| {
        let (t, w) = v.map_or((0, None), |(t, w)| (t, w));
        [a[0] + b.is_some() as i64, a[1] + w.unwrap_or(0), a[2] + (t == 2) as i64, a[3] + (t == 3) as i64]
    });
    let PostHistory { post, creation_date: hd, post_history_type_id, .. } = &db.post_history;
    let recent = Ident::<PostHistory>::new().with(post.select((&db.post.creation_date).ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))));
    let rph = top_per(drain(db.post_history.with(recent).select(post)), |&(_, p)| p, |&(h, _)| (Reverse(hd.get(h).unwrap()), Reverse(h)), 1, false);
    let rph = rel(rph.into_iter().map(|(h, p)| (p, h)).collect());
    let last: HashIdx<Id<Post>, Id<PostHistory>> = (&rph).map(|(p, _)| p).inv().select(&rph).map(|(_, h)| h).collect();
    let keep = (&last).select(Ident::<PostHistory>::new().with(post_history_type_id.filt(|t| t != 10 && t != 12)));
    let vc = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let by_raw: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let v = drain((&us).filt(|a| a[2] > a[3] || a[0] > 0).and((&db.user.origid).select(&by_raw).select(Ident::<Post>::new().and(&vc).and(keep))));
    rows(v.into_iter().map(|(u, (a, ((p, n), h)))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        f.extend(post_fields(db, p, &["id", "title"]));
        f.extend([V::I(n), V::T(hd.get(h).unwrap()), V::I(post_history_type_id.get(h).unwrap()), ostr(db.post_history.comment.get(h))]);
        row(f)
    }))
}

// WITH UserVoteCounts AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVotes,
//        SUM(CASE WHEN V.VoteTypeId IN (2, 3) THEN 1 ELSE 0 END) AS TotalVotes FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.DisplayName),
// RecentPosts AS (SELECT P.Id AS PostId, P.Title, COALESCE(P.AcceptedAnswerId, 0) AS AcceptedAnswerId, P.CreationDate, P.Score, P.ViewCount, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS rn
//     FROM Posts P WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'),
// PostDetails AS (SELECT RP.PostId, RP.Title, RP.CreationDate, RP.Score, RP.ViewCount, U.DisplayName AS OwnerDisplayName, UV.UpVotes, UV.DownVotes FROM RecentPosts RP
//     LEFT JOIN Users U ON RP.AcceptedAnswerId = U.Id LEFT JOIN UserVoteCounts UV ON U.Id = UV.UserId WHERE RP.rn = 1)
// SELECT PD.PostId, PD.Title, PD.CreationDate, PD.Score, PD.ViewCount, PD.OwnerDisplayName, COALESCE(PH.UserDisplayName, 'Unknown') AS LastEditor, PH.CreationDate AS LastEditDate, PD.UpVotes, PD.DownVotes,
//        (PD.UpVotes - PD.DownVotes) AS VoteNet
// FROM PostDetails PD LEFT JOIN PostHistory PH ON PD.PostId = PH.PostId AND PH.PostHistoryTypeId IN (4, 5) WHERE PD.Score > 0 ORDER BY VoteNet DESC, PD.CreationDate DESC LIMIT 10;
//
// `RP.AcceptedAnswerId = U.Id` joins a post id to a user id, so it goes through the raw ids.
fn q604(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, accepted_answer_id, .. } = &db.post;
    let rp = top_per(drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(owner_user.opt())), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let rp: MatSet<Id<Post>> = rel(rp.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let uv = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let by_raw: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let edits = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([4, 5])));
    let acc = accepted_answer_id.opt().map(|x: Option<i64>| x.unwrap_or(0));
    let v = drain((&rp).with(score.gt(0)).select(acc.select(&by_raw).select(Ident::<User>::new().and(&uv)).opt().and(edits.opt())));
    let v = top_n(v, |&(p, (u, h))| (u.map_or(true, |_| false), Reverse(u.map(|(_, a)| a[0] - a[1])), Reverse(creation_date.get(p).unwrap()), p, h), 10);
    rows(v.into_iter().map(|(p, (u, h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.push(u.map_or(V::Null, |(u, _)| user_col(db, u, "name")));
        match h {
            Some(h) => f.extend([V::S(db.post_history.user_display_name.get(h).unwrap_or("Unknown")), V::T(db.post_history.creation_date.get(h).unwrap())]),
            None => f.extend([V::S("Unknown"), V::Null]),
        }
        f.extend(match u {
            Some((_, a)) => [V::I(a[0]), V::I(a[1]), V::I(a[0] - a[1])],
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS ScoreRank, p.OwnerUserId
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.PostTypeId = 1 AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId),
// UserBadges AS (SELECT b.UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverBadges, COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeBadges
//     FROM Badges b GROUP BY b.UserId),
// UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(ub.GoldBadges, 0) AS GoldBadges, COALESCE(ub.SilverBadges, 0) AS SilverBadges, COALESCE(ub.BronzeBadges, 0) AS BronzeBadges
//     FROM Users u LEFT JOIN UserBadges ub ON u.Id = ub.UserId)
// SELECT up.DisplayName, up.Reputation, up.GoldBadges, up.SilverBadges, up.BronzeBadges, COUNT(rp.PostId) AS QuestionCount, AVG(rp.Score) AS AvgScore, SUM(rp.ViewCount) AS TotalViews
// FROM UserReputation up LEFT JOIN RankedPosts rp ON up.UserId = rp.OwnerUserId GROUP BY up.UserId, up.DisplayName, up.Reputation, up.GoldBadges, up.SilverBadges, up.BronzeBadges
// HAVING COUNT(rp.PostId) > 0 ORDER BY AvgScore DESC, TotalViews DESC LIMIT 10;
//
// ScoreRank and CommentCount are never read, so RankedPosts is the recent questions.
fn q3989(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, view_count, .. } = &db.post;
    let g = db
        .post
        .with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))))
        .group_by(owner_user)
        .select(score.and(view_count.opt()))
        .fold([0i64; 4], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]);
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let v = drain((&g).and((&ub).opt()));
    let v = top_n(v, |&(_, (a, _))| (Reverse(fkey(a[1] as f64 / a[0] as f64)), a[2] == 0, Reverse(a[3])), 10);
    rows(v.into_iter().map(|(u, (a, b))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(b.unwrap_or([0; 3]).map(V::I));
        f.extend([V::I(a[0]), avg(a[1], a[0]), nullable(a[3], a[2])]);
        row(f)
    }))
}

// WITH UserVoteStats AS (SELECT U.Id AS UserId, U.DisplayName, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesCount, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesCount,
//        COUNT(DISTINCT P.Id) AS TotalPosts, COUNT(DISTINCT C.Id) AS TotalComments,
//        ROW_NUMBER() OVER (ORDER BY SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) - SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) DESC) AS Rank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName),
// RecentPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.ViewCount, P.Score, U.DisplayName AS OwnerDisplayName, ROW_NUMBER() OVER (PARTITION BY U.Id ORDER BY P.CreationDate DESC) AS LatestPostRank
//     FROM Posts P LEFT JOIN Users U ON P.OwnerUserId = U.Id WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days')
// SELECT uvs.UserId, uvs.DisplayName, uvs.UpVotesCount, uvs.DownVotesCount, uvs.TotalPosts, uvs.TotalComments, rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score
// FROM UserVoteStats uvs LEFT JOIN RecentPosts rp ON uvs.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = rp.PostId LIMIT 1)
// WHERE uvs.UpVotesCount - uvs.DownVotesCount > 10 AND (SELECT COUNT(*) FROM Badges b WHERE b.UserId = uvs.UserId AND b.Class = 1) > 0 ORDER BY uvs.Rank, rp.CreationDate DESC LIMIT 100;
//
// LatestPostRank is never read, so RecentPosts is every recent post; the LIMIT 1 subquery is the post's own owner. Rank ties go to the smaller user id.
fn q23160(db: &'static So) -> String {
    let uv = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 2], |a, p| match p {
            Some((_, t)) => [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64],
            None => a,
        });
    let v = top_n(drain(&uv), |&(u, a)| (Reverse(a[0] - a[1]), u), 0);
    let rk = rel(v.into_iter().enumerate().map(|(i, (u, _))| (u, i as i64 + 1)).collect());
    let rank_of: HashIdx<Id<User>, i64> = (&rk).map(|(u, _)| u).inv().select(&rk).map(|(_, r)| r).collect();
    let gold: MatSet<Id<User>> = db.badge.with((&db.badge.class).eq(1)).select(&db.badge.user).collect();
    let pc = user_distinct_posts(db);
    let cc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db).opt()).opt()).buf_fold(|v| distinct_some(v.into_iter().map(|x| x.flatten())));
    let recent = posts_of(db).select(Ident::<Post>::new().with((&db.post.creation_date).ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))));
    let v = drain(db.user.with(&gold).select((&uv).filt(|a| a[0] - a[1] > 10).and(&pc).and(&cc).and(&rank_of).and(recent.opt())));
    let v = top_n(v, |&(p, ((_, r), x))| (r, Reverse(x.map(|x| db.post.creation_date.get(x).unwrap())), p, x), 100);
    rows(v.into_iter().map(|(u, ((((a, n), c), _), p))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(n), V::I(c)]);
        f.extend(match p {
            Some(p) => post_fields(db, p, &["id", "title", "created", "views", "score"]),
            None => vec![V::Null, V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS Rank,
//        (SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 2) AS UpvoteCount, (SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 3) AS DownvoteCount
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.Score > 0),
// UserBadges AS (SELECT b.UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverBadges, COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeBadges
//     FROM Badges b GROUP BY b.UserId),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c GROUP BY c.PostId)
// SELECT u.DisplayName, up.Title, up.CreationDate, up.Score, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges, COALESCE(pc.CommentCount, 0) AS CommentCount,
//        CASE WHEN up.Rank <= 3 THEN 'Top Post' WHEN up.Score > 10 THEN 'Well-Received' ELSE 'Moderate' END AS PostStatus, CASE WHEN up.UpvoteCount IS NULL THEN 0 ELSE up.UpvoteCount END AS EffectiveUpvotes,
//        COALESCE(NULLIF(up.DownvoteCount, 0), NULL) AS EffectiveDownvotes
// FROM RankedPosts up JOIN Users u ON u.Id = up.OwnerUserId LEFT JOIN UserBadges ub ON ub.UserId = u.Id LEFT JOIN PostComments pc ON pc.PostId = up.Id WHERE up.Rank < 5
// ORDER BY up.Score DESC, u.Reputation DESC LIMIT 100;
fn q3455(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(score.gt(0))).with(owner_user).select(owner_user));
    let v = per_group(ranked(v, |&(p, u)| (u, Reverse(score.get(p).unwrap()), p), false), |&(_, u)| u);
    let tp = rel(v.into_iter().filter(|x| x.1 < 5).map(|((p, _), r)| (p, r)).collect());
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let vc = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    type R = (Id<Post>, i64);
    let pid = || Same::<R>::new().map(|(p, _): R| p);
    let v = drain((&tp).select(Same::<R>::new().and(pid().select(owner_user.select((&ub).opt()))).and(pid().select((&cc).opt())).and(pid().select((&vc).opt()))));
    let v = top_n(v, |&(_, ((((p, _), _), _), _))| (Reverse(score.get(p).unwrap()), Reverse(owner_user.get(p).map(|u| db.user.reputation.get(u).unwrap())), p), 100);
    rows(v.into_iter().map(|(_, ((((p, r), b), c), a))| {
        let s = score.get(p).unwrap();
        let a = a.unwrap_or([0, 0]);
        let mut f = post_fields(db, p, &["owner", "title", "created", "score"]);
        f.extend(match b {
            Some(b) => b.map(V::I),
            None => [V::Null, V::Null, V::Null],
        });
        f.push(V::I(c.unwrap_or(0)));
        f.push(V::S(if r <= 3 { "Top Post" } else if s > 10 { "Well-Received" } else { "Moderate" }));
        f.extend([V::I(a[0]), if a[1] == 0 { V::Null } else { V::I(a[1]) }]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount,
//        SUM(CASE WHEN C.Id IS NOT NULL THEN 1 ELSE 0 END) AS CommentCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId AND V.UserId = U.Id LEFT JOIN Comments C ON P.Id = C.PostId AND C.UserId = U.Id
//     LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, AnswerCount, QuestionCount, UpvoteCount, DownvoteCount, CommentCount, GoldBadges, SilverBadges, BronzeBadges,
//        ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserStats)
// SELECT UserId, DisplayName, Reputation, PostCount, AnswerCount, QuestionCount, UpvoteCount, DownvoteCount, CommentCount, GoldBadges, SilverBadges, BronzeBadges FROM TopUsers WHERE Rank <= 10 ORDER BY Reputation DESC;
//
// Rank reads only Reputation, so the ten users are picked first and the posts x own votes x own comments x badges product is driven for them alone.
fn q7166(db: &'static So) -> String {
    let v = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(v.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let ov = own_votes(db);
    let Comment { user, post, .. } = &db.comment;
    let oc: HashIdx<Id<Post>, Id<Comment>> = db.comment.with(user.and(post.select(&db.post.owner_user)).filt(|(a, b)| a == b)).select(post).inv().collect();
    let s = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and((&ov).select(&db.vote.vote_type_id).opt()).and((&oc).opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 8], |a, (p, c)| {
            let (t, v, m) = p.map_or((0, None, false), |((t, v), m)| (t, v, m.is_some()));
            [a[0] + (t == 2) as i64, a[1] + (t == 1) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64, a[4] + m as i64, a[5] + (c == Some(1)) as i64, a[6] + (c == Some(2)) as i64, a[7] + (c == Some(3)) as i64]
        });
    let pc = user_distinct_posts(db);
    rows(drain((&s).and(&pc)).into_iter().map(|(u, (a, n))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RECURSIVE UserBadges AS (SELECT u.Id AS UserId, u.DisplayName, b.Name AS BadgeName, b.Class, ROW_NUMBER() OVER (PARTITION BY u.Id ORDER BY b.Date DESC) AS BadgeRank
//     FROM Users u JOIN Badges b ON u.Id = b.UserId WHERE b.Class = 1),
// PostStatistics AS (SELECT p.Id AS PostId, p.Title, p.Score, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT v.Id) AS VoteCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount, u.DisplayName AS OwnerDisplayName
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year') GROUP BY p.Id, p.Title, p.Score, u.DisplayName),
// TopPosts AS (SELECT ps.PostId, ps.Title, ps.Score, ps.CommentCount, ps.VoteCount, ps.UpVoteCount, ps.DownVoteCount, ps.OwnerDisplayName, RANK() OVER (ORDER BY ps.Score DESC) AS Rank FROM PostStatistics ps)
// SELECT t.Title, t.Score, t.CommentCount, t.VoteCount, t.UpVoteCount, t.DownVoteCount, t.OwnerDisplayName, ub.BadgeName, ub.BadgeRank
// FROM TopPosts t LEFT JOIN UserBadges ub ON t.OwnerDisplayName = ub.DisplayName WHERE t.Rank <= 10 ORDER BY t.Score DESC, ub.BadgeRank ASC NULLS LAST;
//
// WITH RECURSIVE, but no CTE refers to itself. Rank reads only Score, so the top posts are picked first and the comment x vote product is driven for them alone.
// BadgeRank's Date ties go to the smaller badge id.
fn q31256(db: &'static So) -> String {
    let Post { creation_date, score, owner_user, .. } = &db.post;
    let v = ranked(drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(score)), |&(_, s)| Reverse(s), false);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).buf_fold(distinct_some);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).opt()).buf_fold(distinct_some);
    let Badge { user, date, .. } = &db.badge;
    let gold = drain(db.badge.with((&db.badge.class).eq(1)).select(user));
    let ub = per_group(ranked(gold, |&(b, u)| (u, Reverse(date.get(b).unwrap()), b), false), |&(_, u)| u);
    let ub = rel(ub.into_iter().map(|((b, u), r)| (db.user.display_name.get(u).unwrap(), (b, r))).collect());
    let by_name: HashIdx<Str, (Str, (Id<Badge>, i64))> = (&ub).map(|(n, _)| n).inv().select(&ub).collect();
    let v = drain((&s).and(&cc).and(&vc).and(owner_user.select(&db.user.display_name).select(&by_name).opt()));
    rows(v.into_iter().map(|(p, (((a, c), n), b))| {
        let mut f = post_fields(db, p, &["title", "score"]);
        f.extend([V::I(c), V::I(n), V::I(a[0]), V::I(a[1])]);
        f.extend(post_fields(db, p, &["owner"]));
        f.extend(match b {
            Some((_, (b, r))) => [V::S(db.badge.name.get(b).unwrap()), V::I(r)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("5617", q5617),
    ("25120", q25120),
    ("4496", q4496),
    ("6843", q6843),
    ("21195", q21195),
    ("31099", q31099),
    ("3281", q3281),
    ("5753", q5753),
    ("7700", q7700),
    ("3340", q3340),
    ("9993", q9993),
    ("31779", q31779),
    ("33563", q33563),
    ("1909", q1909),
    ("830", q830),
    ("1966", q1966),
    ("31629", q31629),
    ("4387", q4387),
    ("3878", q3878),
    ("9306", q9306),
    ("20231", q20231),
    ("1362", q1362),
    ("1489", q1489),
    ("9325", q9325),
    ("3405", q3405),
    ("5347", q5347),
    ("6514", q6514),
    ("9444", q9444),
    ("26959", q26959),
    ("9355", q9355),
    ("9572", q9572),
    ("1996", q1996),
    ("61", q61),
    ("2598", q2598),
    ("26263", q26263),
    ("3274", q3274),
    ("6022", q6022),
    ("30459", q30459),
    ("2632", q2632),
    ("5024", q5024),
    ("1622", q1622),
    ("2746", q2746),
    ("5103", q5103),
    ("8846", q8846),
    ("9673", q9673),
    ("4262", q4262),
    ("8151", q8151),
    ("20065", q20065),
    ("8500", q8500),
    ("1554", q1554),
    ("5038", q5038),
    ("800", q800),
    ("7009", q7009),
    ("7445", q7445),
    ("1263", q1263),
    ("29938", q29938),
    ("22922", q22922),
    ("1910", q1910),
    ("9365", q9365),
    ("3102", q3102),
    ("4966", q4966),
    ("27082", q27082),
    ("5746", q5746),
    ("33954", q33954),
    ("7453", q7453),
    ("32139", q32139),
    ("21878", q21878),
    ("31289", q31289),
    ("4413", q4413),
    ("2235", q2235),
    ("4507", q4507),
    ("34854", q34854),
    ("2952", q2952),
    ("28731", q28731),
    ("5711", q5711),
    ("440", q440),
    ("1497", q1497),
    ("8838", q8838),
    ("9713", q9713),
    ("4225", q4225),
    ("20365", q20365),
    ("1193", q1193),
    ("8964", q8964),
    ("9556", q9556),
    ("5320", q5320),
    ("5239", q5239),
    ("30732", q30732),
    ("33006", q33006),
    ("604", q604),
    ("3989", q3989),
    ("23160", q23160),
    ("3455", q3455),
    ("7166", q7166),
    ("31256", q31256),
];
