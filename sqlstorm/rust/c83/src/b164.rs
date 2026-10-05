use harness::prelude::*;
use std::cmp::Reverse;

fn by_first<A: Copy + Eq + std::hash::Hash, B: Copy + Eq + std::hash::Hash>(m: &MatSet<(A, B)>) -> HashIdx<A, B> {
    m.map(|(a, _)| a).inv().map(|(_, b): (A, B)| b).collect()
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, COUNT(c.Id) AS CommentCount, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 8
//     WHERE p.CreationDate >= CURRENT_TIMESTAMP - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.OwnerUserId),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.CommentCount, rp.TotalBounty, RANK() OVER (ORDER BY rp.Score DESC, rp.CommentCount DESC) AS RankScore,
//        RANK() OVER (ORDER BY rp.TotalBounty DESC) AS RankBounty FROM RankedPosts rp)
// SELECT tp.Title, tp.CreationDate, tp.Score, tp.CommentCount, tp.TotalBounty, pt.Name AS PostType, CASE WHEN tp.RankScore <= 10 THEN 'Top 10 Posts' ELSE 'Other Posts' END AS RankingCategory
// FROM TopPosts tp JOIN PostTypes pt ON tp.PostId IN (SELECT Id FROM Posts WHERE PostTypeId = pt.Id) WHERE tp.RankBounty <= 5 ORDER BY tp.RankScore, tp.TotalBounty DESC;
fn q8832(db: &'static So) -> String {
    let Post { creation_date, score, .. } = &db.post;
    let since = ny_to_utc(add_years(utc_to_ny(now_utc()), -1));
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let rp = db
        .post
        .with(creation_date.filt(move |d| ny_to_utc(d) >= since))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(bounty.opt()))
        .fold([0i64; 2], |a, (c, b)| [a[0] + c.is_some() as i64, a[1] + b.flatten().unwrap_or(0)]);
    type R = ((Id<Post>, [i64; 2]), (i64, Id<PostType>));
    let w = whole(&rp).select(Ident::<Post>::new().and(&rp).and(score.and(&db.post.post_type))).window(rank, |((_, a), (s, _)): R| (Reverse(s), Reverse(a[0])), asc);
    let w = (&w).window(rank, |(((_, a), _), _): (R, i64)| Reverse(a[1]), asc);
    let mut v = drain((&w).filt(|(_, b)| b <= 5));
    v.sort_by_key(|&(_, ((((_, a), _), s), _))| (s, Reverse(a[1])));
    rows(v.into_iter().map(|(_, ((((p, a), (_, t)), s), _))| {
        let mut f = post_fields(db, p, &["title", "created", "score"]);
        f.extend([V::I(a[0]), V::I(a[1]), tname(db, t), V::S(if s <= 10 { "Top 10 Posts" } else { "Other Posts" })]);
        row(f)
    }))
}

// WITH TagStats AS (SELECT t.Id AS TagId, t.TagName, COUNT(p.Id) AS PostCount, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews, AVG(COALESCE(p.Score, 0)) AS AvgScore
//     FROM Tags t LEFT JOIN Posts p ON p.Tags ILIKE '%' || t.TagName || '%' GROUP BY t.Id, t.TagName),
// UserReputation AS (SELECT u.Id AS UserId, SUM(CASE WHEN b.Class = 1 THEN 3 WHEN b.Class = 2 THEN 2 WHEN b.Class = 3 THEN 1 ELSE 0 END) AS TotalBadges,
//        MAX(u.Reputation) AS HighestReputation FROM Users u LEFT JOIN Badges b ON b.UserId = u.Id GROUP BY u.Id),
// RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RecentRank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 month')
// SELECT ts.TagName, ts.PostCount, ts.TotalViews, ts.AvgScore, ur.UserId, ur.TotalBadges, ur.HighestReputation, rp.Title AS RecentPostTitle, rp.CreationDate AS RecentPostDate
// FROM TagStats ts INNER JOIN UserReputation ur ON ur.TotalBadges > 1 LEFT JOIN RecentPosts rp ON rp.RecentRank = 1 AND rp.OwnerUserId = ur.UserId
// WHERE ts.AvgScore > 5 ORDER BY ts.TotalViews DESC, ur.HighestReputation DESC LIMIT 10;
//
// The ON clause names only ur, so tags and users are crossed. ILIKE lowercases both sides of the substring test.
fn q1384(db: &'static So) -> String {
    let Post { tags_str, view_count, score, creation_date, owner_user, .. } = &db.post;
    let elems: MatSet<Str> = tags_str.flat_map(tag_list).collect();
    let contains: HashIdx<Str, Id<Tag>> = (&elems).select_where((&db.tag.tag_name).inv(), |e: Str, n: Str| e.to_lowercase().contains(&n.to_lowercase())).collect();
    let pairs: MatSet<(Id<Post>, Id<Tag>)> = db.post.select(Ident::<Post>::new().and(tags_str.flat_map(tag_list).select(&contains))).collect();
    let by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&pairs).map(|(_, t)| t).inv().collect();
    let tst = db
        .tag
        .group_by(Ident::<Tag>::new())
        .select((&by_tag).map(|(p, _)| p).select(view_count.opt().and(score)).opt())
        .fold([0i64; 4], |a, p| match p {
            Some((w, s)) => [a[0] + 1, a[1] + w.unwrap_or(0), a[2] + s, a[3] + 1],
            None => [a[0], a[1], a[2], a[3] + 1],
        });
    let ur = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold(0i64, |n, c| {
        n + match c {
            Some(1) => 3,
            Some(2) => 2,
            Some(3) => 1,
            _ => 0,
        }
    });
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let w = db.post.with(creation_date.ge(add_months(t0, -1))).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let last: HashIdx<Id<User>, Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let users: HashIdx<Id<User>, (i64, Option<Id<Post>>)> = (&ur).filt(|b| b > 1).and((&last).opt()).collect();
    let v = drain((&tst).filt(|a| a[2] > 5 * a[3]).cross(&users));
    let v = top_n(v, |&((t, u), (a, _))| (Reverse(a[1]), Reverse(db.user.reputation.get(u).unwrap()), t, u), 10);
    rows(v.into_iter().map(|((t, u), (a, (b, p)))| {
        let mut f = vec![V::S(db.tag.tag_name.get(t).unwrap()), V::I(a[0]), V::I(a[1]), avg(a[2], a[3]), user_col(db, u, "uid"), V::I(b), user_col(db, u, "rep")];
        f.extend(match p {
            Some(p) => post_fields(db, p, &["title", "created"]),
            None => vec![V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p WHERE p.PostTypeId = 1),
// RecentVotes AS (SELECT v.PostId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes FROM Votes v
//     WHERE v.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' GROUP BY v.PostId),
// ClosedPosts AS (SELECT h.PostId, h.CreationDate AS ClosedDate, h.UserId AS CloserUserId, h.Comment FROM PostHistory h WHERE h.PostHistoryTypeId = 10)
// SELECT up.PostId, up.Title, up.CreationDate, up.Score, up.ViewCount, up.AnswerCount, rv.UpVotes, rv.DownVotes, cp.ClosedDate, cp.CloserUserId
// FROM RankedPosts up LEFT JOIN RecentVotes rv ON up.PostId = rv.PostId LEFT JOIN ClosedPosts cp ON up.PostId = cp.PostId
// WHERE up.Rank = 1 AND (cp.ClosedDate IS NULL OR cp.ClosedDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '60 days')
// ORDER BY up.Score DESC, up.CreationDate DESC LIMIT 100;
fn q31338(db: &'static So) -> String {
    let Post { post_type_id, owner_user_id, creation_date, score, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let w = db.post.with(post_type_id.eq(1)).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let fp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let Vote { post, vote_type_id, creation_date: vd, .. } = &db.vote;
    let rv = db.vote.with(vd.gt(add_days(t0, -30))).group_by(post).select(vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let hd = &db.post_history.creation_date;
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10))).select(Ident::<PostHistory>::new().and(hd));
    let cut = add_days(t0, -60);
    let v = drain((&fp).select((&rv).opt().and(closes.opt())).filt(move |(_, h)| h.map_or(true, |(_, d)| d >= cut)));
    let v = top_n(v, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())), 100);
    rows(v.into_iter().map(|(p, (r, h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers"]);
        f.extend(match r {
            Some(a) => [V::I(a[0]), V::I(a[1])],
            None => [V::Null, V::Null],
        });
        f.extend(match h {
            Some((h, d)) => [V::T(d), oint(db.post_history.user_id.get(h))],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, COUNT(DISTINCT c.Id) AS TotalComments, SUM(v.BountyAmount) AS TotalBounty,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes,
//        ROW_NUMBER() OVER (PARTITION BY u.Id ORDER BY COUNT(DISTINCT p.Id) DESC) AS Rank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalComments, TotalBounty, TotalUpvotes, TotalDownvotes, Rank FROM UserActivity WHERE TotalPosts > 0)
// SELECT tu.DisplayName, tu.TotalPosts, COALESCE(tu.TotalComments, 0) AS TotalComments, COALESCE(tu.TotalBounty, 0) AS TotalBounty, tu.TotalUpvotes, tu.TotalDownvotes,
//        CASE WHEN tu.TotalPosts < 50 THEN 'Newbie' WHEN tu.TotalPosts BETWEEN 50 AND 200 THEN 'Intermediate' ELSE 'Expert' END AS UserLevel
// FROM TopUsers tu WHERE tu.Rank <= 10 ORDER BY tu.TotalPosts DESC;
//
// Rank partitions by u.Id, so it is 1 for every user.
fn q2257(db: &'static So) -> String {
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(comments_of(db).opt()).opt().and(votes_by(db).select(vote_type_id.and(bounty_amount.opt())).opt()))
        .fold([0i64; 4], |a, (_, v)| match v {
            Some((t, b)) => [a[0] + b.is_some() as i64, a[1] + b.unwrap_or(0), a[2] + (t == 2) as i64, a[3] + (t == 3) as i64],
            None => a,
        });
    let np = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let nc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db))).fold(0i64, |n, _| n + 1);
    let v = drain((&np).filt(|n| n > 0).and((&nc).opt()).and(&s));
    rows(v.into_iter().map(|(u, ((n, c), a))| {
        row(vec![
            user_col(db, u, "name"),
            V::I(n),
            V::I(c.unwrap_or(0)),
            V::I(a[1]),
            V::I(a[2]),
            V::I(a[3]),
            V::S(if n < 50 { "Newbie" } else if n <= 200 { "Intermediate" } else { "Expert" }),
        ])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.CreationDate, COUNT(DISTINCT c.Id) AS CommentCount, AVG(v.BountyAmount) AS AvgBounty,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS OwnerPostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 8
//     WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.OwnerUserId, p.CreationDate),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, ROW_NUMBER() OVER (ORDER BY u.Reputation DESC) AS ReputationRank FROM Users u WHERE u.Reputation > 0)
// SELECT up.UserId, up.Reputation, rp.Title, rp.CommentCount, rp.AvgBounty, CASE WHEN rp.OwnerPostRank = 1 THEN 'Most Recent Post' ELSE 'Other Posts' END AS PostStatus
// FROM UserReputation up JOIN RankedPosts rp ON up.UserId = rp.OwnerUserId
// WHERE NOT EXISTS (SELECT 1 FROM Posts p WHERE p.OwnerUserId = up.UserId AND p.CreationDate < rp.CreationDate AND p.ViewCount < 100)
// ORDER BY up.Reputation DESC, rp.CommentCount DESC LIMIT 100;
//
// The NOT EXISTS is decorrelated: no post of the user with ViewCount < 100 is older than rp exactly when the user's
// earliest such post (if any) is not before rp.CreationDate.
fn q3188(db: &'static So) -> String {
    let Post { creation_date, owner_user, view_count, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1)));
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let rp = recent().group_by(Ident::<Post>::new()).select(bounty.opt().and(comments_of(db).opt())).fold([0i64; 2], |a, (b, _)| {
        let b = b.flatten();
        [a[0] + b.is_some() as i64, a[1] + b.unwrap_or(0)]
    });
    let cc = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let w = recent().group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let newest: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let low = db.post.with(view_count.lt(100)).group_by(owner_user).select(creation_date).fold(i64::MAX, |m, d| m.min(d));
    let up = Ident::<User>::new().with((&db.user.reputation).gt(0));
    let v = drain(
        (&rp)
            .and(&cc)
            .and(creation_date)
            .and(owner_user.select(up.and((&low).opt())))
            .filt(|(((_, _), d), (_, m))| m.map_or(true, |m| m >= d))
            .and(Ident::<Post>::new().with(&newest).opt()),
    );
    let v = top_n(v, |&(_, ((((_, c), _), (u, _)), _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(c)), 100);
    rows(v.into_iter().map(|(p, ((((b, c), _), (u, _)), n))| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.push(post_fields(db, p, &["title"]).remove(0));
        f.extend([V::I(c), avg(b[1], b[0]), V::S(if n.is_some() { "Most Recent Post" } else { "Other Posts" })]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount, SUM(CASE WHEN b.Id IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount, MAX(p.CreationDate) AS LastPostDate
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId
//     WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, UpvoteCount, DownvoteCount, BadgeCount, LastPostDate,
//        RANK() OVER (ORDER BY PostCount DESC) AS PostRank, RANK() OVER (ORDER BY UpvoteCount DESC) AS UpvoteRank FROM UserActivity)
// SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, UpvoteCount, DownvoteCount, BadgeCount, LastPostDate, PostRank, UpvoteRank
// FROM TopUsers WHERE PostRank <= 10 OR UpvoteRank <= 10 ORDER BY PostRank, UpvoteRank;
fn q8278(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let users = || db.user.with((&db.user.reputation).gt(1000));
    let s = users()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(creation_date).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).opt()))
        .fold([0, 0, 0, 0, 0, i64::MIN], |a, (p, b)| {
            let (t, d, v) = p.map_or((0, i64::MIN, None), |((t, d), v)| (t, d, v));
            [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64, a[4] + b.is_some() as i64, a[5].max(d)]
        });
    let np = users().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    type R = (Id<User>, (i64, [i64; 6]));
    let w = whole(&np).select(Ident::<User>::new().and((&np).and(&s))).window(rank, |(_, (n, _)): R| Reverse(n), asc);
    let w = (&w).window(rank, |((_, (_, a)), _): (R, i64)| Reverse(a[2]), asc);
    let mut v = drain((&w).filt(|((_, r), u)| r <= 10 || u <= 10));
    v.sort_by_key(|&(_, ((_, r), w))| (r, w));
    rows(v.into_iter().map(|(_, (((u, (n, a)), r), w))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4]), tmax(a[5]), V::I(r), V::I(w)]);
        row(f)
    }))
}

// Rewritten (rewrites/9646.sql): the final ORDER BY tie-broken on TP.PostId.
// WITH RankedPosts AS (SELECT P.Id AS PostId, P.CreationDate, P.Title, U.DisplayName AS OwnerDisplayName, COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS Upvotes,
//        COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS Downvotes, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS RN
//     FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Users U ON P.OwnerUserId = U.Id
//     WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY P.Id, P.CreationDate, P.Title, U.DisplayName, P.OwnerUserId),
// TopPosts AS (SELECT PostId, Title, OwnerDisplayName, Upvotes, Downvotes, CommentCount, RN FROM RankedPosts WHERE RN = 1)
// SELECT TP.Title, TP.OwnerDisplayName, (TP.Upvotes - TP.Downvotes) AS NetVotes, TP.CommentCount,
//        CASE WHEN TP.Upvotes > 100 THEN 'Hot' WHEN TP.Upvotes BETWEEN 50 AND 100 THEN 'Trending' ELSE 'Normal' END AS Popularity
// FROM TopPosts TP WHERE TP.CommentCount > 5 ORDER BY NetVotes DESC, TP.CommentCount DESC, TP.PostId LIMIT 10;
//
// RN reads only base columns, so each owner's newest post is picked first and the vote x comment product is driven for those alone.
fn q9646(db: &'static So) -> String {
    let Post { creation_date, owner_user_id, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (Reverse(d), p), asc);
    let fp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let s = (&fp)
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()))
        .fold([0i64; 3], |a, (t, c)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64]);
    let v = top_n(drain((&s).filt(|a| a[2] > 5)), |&(p, a)| (Reverse(a[0] - a[1]), Reverse(a[2]), p), 10);
    rows(v.into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["title", "owner"]);
        f.extend([V::I(a[0] - a[1]), V::I(a[2]), V::S(if a[0] > 100 { "Hot" } else if a[0] >= 50 { "Trending" } else { "Normal" })]);
        row(f)
    }))
}

// WITH UserVoteStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(v.Id) AS TotalVotes, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, AVG(COALESCE(p.Score, 0)) AS AvgPostScore
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Posts p ON v.PostId = p.Id WHERE u.Reputation > 100 GROUP BY u.Id, u.DisplayName),
// PostDetails AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, COALESCE(ph.Comment, 'No comments') AS LastEditComment,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.LastActivityDate DESC) AS rn, p.OwnerUserId
//     FROM Posts p LEFT JOIN PostHistory ph ON p.Id = ph.PostId AND ph.CreationDate = (SELECT MAX(CreationDate) FROM PostHistory WHERE PostId = p.Id)
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year')
// SELECT u.DisplayName AS UserName, u.TotalVotes, u.UpVotes, u.DownVotes, u.AvgPostScore, pd.PostId, pd.Title, pd.CreationDate, pd.ViewCount, pd.LastEditComment
// FROM UserVoteStats u JOIN PostDetails pd ON u.UserId = pd.OwnerUserId WHERE u.TotalVotes > 20 AND pd.rn = 1 ORDER BY u.AvgPostScore DESC, u.TotalVotes DESC LIMIT 50;
//
// rn numbers the joined (post, latest history) rows; history rows of one post tied at its latest date are ordered by id.
fn q2273(db: &'static So) -> String {
    let Post { creation_date, owner_user, last_activity_date, score, .. } = &db.post;
    let uv = db
        .user
        .with((&db.user.reputation).gt(100))
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select((&db.vote.vote_type_id).and((&db.vote.post).select(score).opt())).opt())
        .fold([0i64; 5], |a, v| match v {
            Some((t, s)) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64, a[3] + s.unwrap_or(0), a[4] + 1],
            None => [a[0], a[1], a[2], a[3], a[4] + 1],
        });
    let PostHistory { post, creation_date: hd, .. } = &db.post_history;
    let md = db.post_history.group_by(post).select(hd).fold(i64::MIN, |m, d| m.max(d));
    let at: HashIdx<(Id<Post>, i64), Id<PostHistory>> = db.post_history.select(post.and(hd)).inv().collect();
    type J = (Id<Post>, Option<Id<PostHistory>>);
    let j: MatSet<J> = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(owner_user)
        .select(Ident::<Post>::new().and(Ident::<Post>::new().and(&md).select(&at).opt()))
        .collect();
    let post_of = || Same::<J>::new().map(|(p, _): J| p);
    let w = (&j).group_by(post_of().select(owner_user)).select(Same::<J>::new().and(post_of().select(last_activity_date))).window(row_number, |((p, h), d)| (Reverse(d), p, h), asc);
    let fr: HashIdx<Id<User>, J> = (&w).filt(|(_, r)| r == 1).map(|(((p, h), _), _)| (p, h)).collect();
    let v = drain((&fr).and((&uv).filt(|a| a[0] > 20)));
    let v = top_n(v, |&(_, ((p, _), a))| (Reverse(fkey(a[3] as f64 / a[4] as f64)), Reverse(a[0]), p), 50);
    rows(v.into_iter().map(|(u, ((p, h), a))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[3], a[4])];
        f.extend(post_fields(db, p, &["id", "title", "created", "views"]));
        f.push(V::S(h.and_then(|h| db.post_history.comment.get(h)).unwrap_or("No comments")));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.Views, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(V.BountyAmount) AS TotalBounties, ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS UserRank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON U.Id = V.UserId AND V.VoteTypeId IN (8, 9) GROUP BY U.Id, U.DisplayName, U.Reputation, U.Views),
// ClosedPostDetails AS (SELECT PH.PostId, PH.CreationDate AS CloseDate, COUNT(*) AS CloseReasonCount FROM PostHistory PH WHERE PH.PostHistoryTypeId = 10 GROUP BY PH.PostId, PH.CreationDate),
// RankedClosedPosts AS (SELECT CP.PostId, ROW_NUMBER() OVER (ORDER BY CP.CloseDate DESC) AS CloseRank FROM ClosedPostDetails CP)
// SELECT UA.DisplayName, UA.Reputation, UA.Views, UA.PostCount, UA.AnswerCount, UA.TotalBounties, COALESCE(RCP.CloseRank, 0) AS RecentClosedPostRank
// FROM UserActivity UA LEFT JOIN RankedClosedPosts RCP ON UA.UserId = RCP.PostId WHERE UA.Reputation > 1000 AND UA.PostCount > 5
// ORDER BY UA.UserRank, RecentClosedPostRank DESC;
//
// `UA.UserId = RCP.PostId` joins a user id to a post id, so it goes through the raw ids. Close groups tied on CloseDate are numbered in post id order.
fn q3706(db: &'static So) -> String {
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let users = || db.user.with((&db.user.reputation).gt(1000));
    let bv = votes_by(db).select(Ident::<Vote>::new().with(vote_type_id.is_in([8, 9]))).select(bounty_amount.opt());
    let ua = users()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(&db.post.post_type_id).opt().and(bv.opt()))
        .fold([0i64; 3], |a, (t, b)| {
            let b = b.flatten();
            [a[0] + (t == Some(2)) as i64, a[1] + b.is_some() as i64, a[2] + b.unwrap_or(0)]
        });
    let np = users().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let PostHistory { post_id, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let cpd = db.post_history.with(post_history_type_id.eq(10)).group_by(post_id.and(hd)).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let w = whole(&cpd).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let rk: MatSet<(i64, i64)> = (&w).map(|((p, _), r)| (p, r)).collect();
    let by_post = by_first(&rk);
    let v = drain((&np).filt(|n| n > 5).and(&ua).and((&db.user.origid).select(&by_post).opt()));
    rows(v.into_iter().map(|(u, ((n, a), r))| {
        let mut f = ucols(db, u, &["name", "rep", "uviews"]);
        f.extend([V::I(n), V::I(a[0]), nullable(a[2], a[1]), V::I(r.unwrap_or(0))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.PostTypeId, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= DATE '2024-10-01' - INTERVAL '1 year'),
// PostVoteStats AS (SELECT p.Id AS PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.OwnerDisplayName, rp.Score, rp.ViewCount, pvs.UpVotes, pvs.DownVotes FROM RankedPosts rp JOIN PostVoteStats pvs ON rp.PostId = pvs.PostId WHERE rp.Rank <= 5)
// SELECT tp.PostId, tp.Title, tp.OwnerDisplayName, tp.Score, tp.ViewCount, tp.UpVotes, tp.DownVotes, (CAST(tp.UpVotes AS FLOAT) / NULLIF((tp.UpVotes + tp.DownVotes), 0)) * 100 AS UpvotePercentage
// FROM TopPosts tp ORDER BY tp.Score DESC, tp.ViewCount DESC;
fn q8099(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, owner_user, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(date(2024, 10, 1), -1)))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|(((p, _), _), _)| p).collect();
    let s = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    rows(drain(&s).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "score", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), if a[0] + a[1] == 0 { V::Null } else { V::F((a[0] as f32 / (a[0] + a[1]) as f32 * 100.0f32) as f64) }]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.LastActivityDate, U.DisplayName AS OwnerDisplayName,
//        DENSE_RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS ScoreRank FROM Posts p JOIN Users U ON p.OwnerUserId = U.Id WHERE p.Score IS NOT NULL),
// PopularPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.CreationDate, rp.OwnerDisplayName FROM RankedPosts rp WHERE rp.ScoreRank <= 10),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c GROUP BY c.PostId),
// FinalResults AS (SELECT pp.PostId, pp.Title, pp.Score, pp.OwnerDisplayName, COALESCE(pc.CommentCount, 0) AS CommentCount FROM PopularPosts pp LEFT JOIN PostComments pc ON pp.PostId = pc.PostId)
// SELECT fr.PostId, fr.Title, fr.Score, fr.OwnerDisplayName, fr.CommentCount,
//        CASE WHEN fr.Score > 50 THEN 'High Score' WHEN fr.Score BETWEEN 20 AND 50 THEN 'Medium Score' ELSE 'Low Score' END AS ScoreCategory
// FROM FinalResults fr WHERE fr.CommentCount > 5 ORDER BY fr.Score DESC, fr.PostId;
fn q2956(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, .. } = &db.post;
    let w = db.post.with(owner_user).group_by(post_type_id).select(Ident::<Post>::new().and(score)).window(dense_rank, |(_, s)| Reverse(s), asc);
    let pp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|((p, _), _)| p).collect();
    let pc = (&pp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    rows(drain((&pc).filt(|n| n > 5)).into_iter().map(|(p, n)| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "score", "owner"]);
        f.extend([V::I(n), V::S(if s > 50 { "High Score" } else if s >= 20 { "Medium Score" } else { "Low Score" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS PostRank,
//        COALESCE(u.DisplayName, 'Community') AS OwnerDisplayName FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' AND p.PostTypeId IN (1, 2)),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.OwnerDisplayName FROM RankedPosts rp WHERE rp.PostRank <= 5),
// PostDetails AS (SELECT tp.PostId, tp.Title, tp.OwnerDisplayName, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes, COUNT(c.Id) AS CommentCount
//     FROM TopPosts tp LEFT JOIN Votes v ON tp.PostId = v.PostId LEFT JOIN Comments c ON tp.PostId = c.PostId GROUP BY tp.PostId, tp.Title, tp.OwnerDisplayName)
// SELECT pd.PostId, pd.Title, pd.OwnerDisplayName, pd.Upvotes, pd.Downvotes, pd.CommentCount, (pd.Upvotes - pd.Downvotes) AS ScoreBalance FROM PostDetails pd ORDER BY ScoreBalance DESC;
fn q5960(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)).and(post_type_id.is_in([1, 2])))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score))
        .window(row_number, |(p, s)| (Reverse(s), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()))
        .fold([0i64; 3], |a, (t, c)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64]);
    rows(drain(&s).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.push(V::S(db.post.owner_user.get(p).map_or("Community", |u| db.user.display_name.get(u).unwrap())));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[0] - a[1])]);
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, COALESCE(SUM(V.BountyAmount), 0) AS TotalBounty, ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS UserRank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON V.UserId = U.Id GROUP BY U.Id, U.DisplayName, U.Reputation),
// TagPopularity AS (SELECT T.TagName, COUNT(P.Id) AS PostCount FROM Tags T JOIN Posts P ON P.Tags LIKE '%' || T.TagName || '%' GROUP BY T.TagName),
// TopTags AS (SELECT TagName, PostCount, RANK() OVER (ORDER BY PostCount DESC) AS TagRank FROM TagPopularity)
// SELECT U.UserId, U.DisplayName, U.Reputation, U.TotalPosts, U.TotalQuestions, U.TotalAnswers, U.TotalBounty, T.TagName, T.PostCount
// FROM UserStatistics U LEFT JOIN TopTags T ON U.TotalPosts > 10 AND U.UserRank <= 10 AND T.TagRank <= 5
// WHERE U.Reputation > 1000 OR (U.TotalQuestions > 0 AND U.TotalAnswers > 0) ORDER BY U.Reputation DESC, U.UserId ASC;
//
// The ON clause tests U and T separately: the users that pass it are crossed with the top tags, the rest keep one NULL row.
fn q3055(db: &'static So) -> String {
    let Vote { bounty_amount, .. } = &db.vote;
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(&db.post.post_type_id).opt().and(votes_by(db).select(bounty_amount.opt()).opt()))
        .fold([0i64; 3], |a, (t, b)| [a[0] + (t == Some(1)) as i64, a[1] + (t == Some(2)) as i64, a[2] + b.flatten().unwrap_or(0)]);
    let np = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let by_rep = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10);
    let top10: MatSet<Id<User>> = rel(by_rep.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let lt = tag_mentions(db);
    let tp = (&lt).group_by(Same::<(Id<Post>, Id<Tag>)>::new().map(|(_, t)| t).select(&db.tag.tag_name)).select(Same::<(Id<Post>, Id<Tag>)>::new()).fold(0i64, |n, _| n + 1);
    let w = whole(&tp).select(Same::<Str>::new().and(&tp)).window(rank, |(_, n)| Reverse(n), asc);
    let tt: MatSet<(Str, i64)> = (&w).filt(|(_, r)| r <= 5).map(|(t, _)| t).collect();
    let rich = Ident::<User>::new().with((&db.user.reputation).gt(1000));
    let both = Ident::<User>::new().with((&us).filt(|a| a[0] > 0 && a[1] > 0));
    let qual: MatSet<Id<User>> = db.user.with(rich.or(both)).collect();
    let pass: MatSet<Id<User>> = (&top10).with((&np).filt(|n| n > 10)).collect();
    let joined: MatSet<(Id<User>, (Str, i64))> = (&qual).with(&pass).cross(&tt).map(|(u, t)| (u, t)).collect();
    let by_user = by_first(&joined);
    let v = drain((&qual).select((&np).and(&us).and((&by_user).opt())));
    rows(v.into_iter().map(|(u, ((n, a), t))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        f.extend(match t {
            Some((t, c)) => [V::S(t), V::I(c)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounties, ROW_NUMBER() OVER (ORDER BY u.Reputation DESC) AS UserRank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 8 GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT us.UserId, us.DisplayName, us.TotalPosts, us.TotalQuestions, us.TotalAnswers, us.TotalBounties, us.UserRank FROM UserStats us WHERE us.TotalPosts > 5
//     ORDER BY us.TotalBounties DESC LIMIT 10)
// SELECT tu.DisplayName, tu.TotalPosts, tu.TotalQuestions, tu.TotalAnswers, tu.TotalBounties, COALESCE(ph.RevisionGUID, 'No History') AS RevisionGUID, COALESCE(ph.Comment, 'No Comments') AS LastComment
// FROM TopUsers tu LEFT JOIN PostHistory ph ON tu.UserId = ph.UserId AND ph.CreationDate = (SELECT MAX(CreationDate) FROM PostHistory WHERE UserId = tu.UserId) ORDER BY tu.UserRank;
fn q3859(db: &'static So) -> String {
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(bounty.opt())).opt())
        .fold([0i64; 3], |a, p| match p {
            Some((t, b)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + b.flatten().unwrap_or(0)],
            None => a,
        });
    let np = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let tu = top_n(drain((&np).filt(|n| n > 5).and(&us)), |&(u, (_, a))| (Reverse(a[2]), u), 10);
    let tu = rel(tu);
    let PostHistory { user, creation_date: hd, .. } = &db.post_history;
    let md = db.post_history.group_by(user).select(hd).fold(i64::MIN, |m, d| m.max(d));
    let at: HashIdx<(Id<User>, i64), Id<PostHistory>> = db.post_history.select(user.and(hd)).inv().collect();
    type R = (Id<User>, (i64, [i64; 3]));
    let v = drain((&tu).select(Same::<R>::new().and(Same::<R>::new().map(|(u, _): R| u).select(Ident::<User>::new().and(&md).select(&at)).opt())));
    rows(v.into_iter().map(|(_, ((u, (n, a)), h))| {
        row(vec![
            user_col(db, u, "name"),
            V::I(n),
            V::I(a[0]),
            V::I(a[1]),
            V::I(a[2]),
            V::S(h.map_or("No History", |h| db.post_history.revision_guid.get(h).unwrap())),
            V::S(h.and_then(|h| db.post_history.comment.get(h)).unwrap_or("No Comments")),
        ])
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.ViewCount, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS PostRank
//     FROM Posts P WHERE P.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '6 months'),
// UserStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS TotalPosts, COUNT(DISTINCT C.Id) AS TotalComments, COALESCE(SUM(V.BountyAmount), 0) AS TotalBounty
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId IN (8, 9) GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalComments, TotalBounty, RANK() OVER (ORDER BY TotalPosts DESC, TotalBounty DESC) AS UserRank FROM UserStats)
// SELECT TU.DisplayName AS User, R.Title AS RecentPost, R.CreationDate AS PostDate, TU.TotalPosts, TU.TotalComments, TU.TotalBounty
// FROM TopUsers TU LEFT JOIN RankedPosts R ON TU.UserId = R.PostId WHERE TU.UserRank <= 10 ORDER BY TU.TotalPosts DESC, TU.TotalBounty DESC NULLS LAST;
//
// `TU.UserId = R.PostId` joins a user id to a post id, so it goes through the raw ids.
fn q4990(db: &'static So) -> String {
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt());
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(comments_of(db).opt().and(bounty.opt())).opt())
        .fold(0i64, |n, p| n + p.and_then(|(_, b)| b.flatten()).unwrap_or(0));
    let np = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let nc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let w = whole(&np).select(Ident::<User>::new().and((&np).and(&us))).window(rank, |(_, (n, b))| (Reverse(n), Reverse(b)), asc);
    let tu: MatSet<(Id<User>, (i64, i64))> = (&w).filt(|(_, r)| r <= 10).map(|(x, _)| x).collect();
    let creation_date = &db.post.creation_date;
    let recent: HashIdx<i64, Id<Post>> = db.post.with(creation_date.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -6))).select(&db.post.origid).inv().collect();
    type R = (Id<User>, (i64, i64));
    let v = drain((&tu).select(Same::<R>::new().and(Same::<R>::new().map(|(u, _): R| u).select((&nc).and((&db.user.origid).select(&recent).opt())))));
    rows(v.into_iter().map(|(_, ((u, (n, b)), (c, r)))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(match r {
            Some(p) => post_fields(db, p, &["title", "created"]),
            None => vec![V::Null, V::Null],
        });
        f.extend([V::I(n), V::I(c), V::I(b)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS Rank FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 YEAR'),
// RecentVotes AS (SELECT v.PostId, COUNT(*) AS VoteCount FROM Votes v WHERE v.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 MONTH' GROUP BY v.PostId),
// PostSummary AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.OwnerDisplayName, COALESCE(rv.VoteCount, 0) AS RecentVoteCount
//     FROM RankedPosts rp LEFT JOIN RecentVotes rv ON rp.PostId = rv.PostId WHERE rp.Rank <= 5)
// SELECT ps.PostId, ps.Title, ps.CreationDate, ps.Score, ps.ViewCount, ps.OwnerDisplayName, ps.RecentVoteCount,
//        CASE WHEN ps.RecentVoteCount > 10 THEN 'Trending' WHEN ps.Score > 100 THEN 'Popular' ELSE 'Normal' END AS PostStatus
// FROM PostSummary ps ORDER BY ps.RecentVoteCount DESC, ps.Score DESC;
fn q6799(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(date(2024, 10, 1), -1)))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let rv = db.vote.with((&db.vote.creation_date).ge(add_months(date(2024, 10, 1), -1))).group_by(&db.vote.post).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    rows(drain((&tp).select((&rv).opt())).into_iter().map(|(p, n)| {
        let n = n.unwrap_or(0);
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend([V::I(n), V::S(if n > 10 { "Trending" } else if score.get(p).unwrap() > 100 { "Popular" } else { "Normal" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank,
//        COUNT(c.Id) AS CommentCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) - SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS NetVotes
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate > (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year') GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.OwnerUserId)
// SELECT u.DisplayName, u.Reputation, u.Views, rp.Title, rp.CreationDate, rp.Score, rp.CommentCount, rp.NetVotes,
//        CASE WHEN rp.CommentCount > 10 THEN 'Very Active' WHEN rp.CommentCount BETWEEN 5 AND 10 THEN 'Active' ELSE 'Less Active' END AS ActivityLevel,
//        CASE WHEN rp.NetVotes > 50 THEN 'Highly Regarded' WHEN rp.NetVotes BETWEEN 20 AND 50 THEN 'Regarded' ELSE 'Needs Improvement' END AS ReputationLevel
// FROM Users u JOIN RankedPosts rp ON u.Id = rp.PostId WHERE (SELECT COUNT(*) FROM Badges b WHERE b.UserId = u.Id AND b.Class = 1) > 0 AND rp.PostRank = 1
// ORDER BY u.Reputation DESC, rp.Score DESC;
//
// `u.Id = rp.PostId` joins a user id to a post id, so it goes through the raw ids. PostRank reads only base columns, so each owner's newest post is picked first.
fn q20800(db: &'static So) -> String {
    let Post { creation_date, owner_user_id, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (Reverse(d), p), asc);
    let fp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let s = (&fp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64 - (t == Some(3)) as i64]);
    let by_orig: HashIdx<i64, Id<Post>> = (&fp).select(&db.post.origid).inv().collect();
    let gold = badges_of(db).select(Ident::<Badge>::new().with((&db.badge.class).eq(1)));
    let v = drain(db.user.with(gold).select((&db.user.origid).select(&by_orig).select(Ident::<Post>::new().and(&s))));
    rows(v.into_iter().map(|(u, (p, a))| {
        let mut f = ucols(db, u, &["name", "rep", "uviews"]);
        f.extend(post_fields(db, p, &["title", "created", "score"]));
        f.extend([
            V::I(a[0]),
            V::I(a[1]),
            V::S(if a[0] > 10 { "Very Active" } else if a[0] >= 5 { "Active" } else { "Less Active" }),
            V::S(if a[1] > 50 { "Highly Regarded" } else if a[1] >= 20 { "Regarded" } else { "Needs Improvement" }),
        ]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.Score, p.CreationDate, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS rnk
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, MAX(b.Class) AS HighestBadgeClass FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// RecentVotes AS (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Votes v WHERE v.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 month' GROUP BY v.PostId)
// SELECT up.DisplayName, rp.Title, rp.Score, ub.BadgeCount, ub.HighestBadgeClass, COALESCE(rv.UpVotes, 0) AS UpVotes, COALESCE(rv.DownVotes, 0) AS DownVotes
// FROM RankedPosts rp JOIN Users up ON rp.OwnerUserId = up.Id JOIN UserBadges ub ON up.Id = ub.UserId LEFT JOIN RecentVotes rv ON rp.Id = rv.PostId
// WHERE rp.rnk = 1 ORDER BY rp.Score DESC, ub.BadgeCount DESC LIMIT 10;
fn q3824(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let w = db
        .post
        .with(post_type_id.eq(1).and(creation_date.ge(add_years(t0, -1))))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(score))
        .window(row_number, |(p, s)| (Reverse(s), p), asc);
    let fp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold((0i64, i64::MIN), |(n, m), c| match c {
        Some(c) => (n + 1, m.max(c)),
        None => (n, m),
    });
    let rv = db.vote.with((&db.vote.creation_date).ge(add_months(t0, -1))).group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let v = drain((&fp).select(owner_user.select(Ident::<User>::new().and(&ub)).and((&rv).opt())));
    let v = top_n(v, |&(p, ((_, (n, _)), _))| (Reverse(score.get(p).unwrap()), Reverse(n)), 10);
    rows(v.into_iter().map(|(p, ((u, (n, m)), r))| {
        let r = r.unwrap_or([0, 0]);
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title", "score"]));
        f.extend([V::I(n), omax(m, n), V::I(r[0]), V::I(r[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS PostRank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName, p.PostTypeId),
// TopScoringPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.OwnerDisplayName, rp.CommentCount FROM RankedPosts rp WHERE rp.PostRank <= 10)
// SELECT tsp.*, COALESCE(v.UpVoteCount, 0) AS UpVoteCount, COALESCE(v.DownVoteCount, 0) AS DownVoteCount
// FROM TopScoringPosts tsp LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
//     FROM Votes GROUP BY PostId) v ON tsp.PostId = v.PostId ORDER BY tsp.Score DESC, tsp.CreationDate DESC;
fn q7108(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|(((p, _), _), _)| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    rows(drain((&cc).and(&vc)).into_iter().map(|(p, (c, a))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, COALESCE(p.AcceptedAnswerId, 0) AS AcceptedAnswerId, COUNT(c.Id) AS CommentCount,
//        COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RN
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.Score, p.AcceptedAnswerId, p.OwnerUserId, p.CreationDate),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.AcceptedAnswerId, rp.CommentCount, rp.UpVotes, rp.DownVotes FROM RankedPosts rp WHERE rp.RN = 1)
// SELECT u.DisplayName, SUM(tp.Score) AS TotalScore, COUNT(tp.PostId) AS TotalPosts, AVG(tp.CommentCount) AS AvgComments, SUM(tp.UpVotes) AS TotalUpVotes, SUM(tp.DownVotes) AS TotalDownVotes
// FROM Users u JOIN TopPosts tp ON u.Id = tp.AcceptedAnswerId WHERE u.Reputation > 1000 GROUP BY u.DisplayName HAVING COUNT(tp.PostId) >= 5 ORDER BY TotalScore DESC LIMIT 10;
//
// `u.Id = tp.AcceptedAnswerId` joins a user id to a post id, so it goes through the raw ids. RN reads only base columns, so each owner's newest post is picked first.
fn q7428(db: &'static So) -> String {
    let Post { creation_date, owner_user_id, score, accepted_answer_id, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (Reverse(d), p), asc);
    let fp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let s = (&fp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let by_acc: HashIdx<i64, Id<Post>> = (&fp).select(accepted_answer_id.opt().map(|a| a.unwrap_or(0))).inv().collect();
    let g = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(&db.user.display_name)
        .select((&db.user.origid).select(&by_acc).select(score.and(&s)))
        .fold([0i64; 5], |a, (sc, c)| [a[0] + sc, a[1] + 1, a[2] + c[0], a[3] + c[1], a[4] + c[2]]);
    let v = top_n(drain((&g).filt(|a| a[1] >= 5)), |&(n, a)| (Reverse(a[0]), n), 10);
    rows(v.into_iter().map(|(n, a)| row(vec![V::S(n), V::I(a[0]), V::I(a[1]), avg(a[2], a[1]), V::I(a[3]), V::I(a[4])])))
}

// WITH UserStatistics AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes, COALESCE(SUM(B.Class), 0) AS TotalBadges, (SELECT COUNT(*) FROM Comments C WHERE C.UserId = U.Id) AS TotalComments
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalUpVotes, TotalDownVotes, TotalBadges, TotalComments,
//        RANK() OVER (ORDER BY TotalPosts DESC, TotalUpVotes DESC) AS PostRank FROM UserStatistics)
// SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalUpVotes, TotalDownVotes, TotalBadges, TotalComments, PostRank FROM TopUsers WHERE PostRank <= 10 ORDER BY PostRank;
fn q8503(db: &'static So) -> String {
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 5], |a, (p, b)| {
            let (t, v) = p.map_or((0, None), |x| x);
            [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64, a[4] + b.unwrap_or(0)]
        });
    let np = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let nc = db.user.group_by(Ident::<User>::new()).select(comments_by(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let w = whole(&np).select(Ident::<User>::new().and((&np).and(&s).and(&nc))).window(rank, |(_, ((n, a), _))| (Reverse(n), Reverse(a[2])), asc);
    let v = drain((&w).filt(|(_, r)| r <= 10));
    rows(v.into_iter().map(|(_, ((u, ((n, a), c)), r))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.extend([V::I(c), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, U.DisplayName AS OwnerDisplayName,
//        RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS ScoreRank FROM Posts p JOIN Users U ON p.OwnerUserId = U.Id
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// PostStats AS (SELECT rp.PostId, rp.Title, rp.ViewCount, rp.Score, rp.OwnerDisplayName,
//        CASE WHEN rp.ScoreRank = 1 THEN 'Top Post' WHEN rp.ScoreRank <= 5 THEN 'Top 5 Posts' ELSE 'Other Posts' END AS PostCategory FROM RankedPosts rp),
// PostVotes AS (SELECT v.PostId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes FROM Votes v GROUP BY v.PostId)
// SELECT ps.PostId, ps.Title, ps.ViewCount, ps.Score, ps.OwnerDisplayName, ps.PostCategory, COALESCE(pv.UpVotes, 0) AS TotalUpVotes, COALESCE(pv.DownVotes, 0) AS TotalDownVotes
// FROM PostStats ps LEFT JOIN PostVotes pv ON ps.PostId = pv.PostId ORDER BY ps.Score DESC, ps.ViewCount DESC;
fn q8896(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, .. } = &db.post;
    let rp = || db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).with(owner_user);
    let w = rp().group_by(owner_user).select(Ident::<Post>::new().and(score)).window(rank, |(_, s)| Reverse(s), asc);
    let rk: MatSet<(Id<Post>, i64)> = (&w).map(|((p, _), r)| (p, r)).collect();
    let by_post = by_first(&rk);
    let pv = rp().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    rows(drain((&pv).and(&by_post)).into_iter().map(|(p, (a, r))| {
        let mut f = post_fields(db, p, &["id", "title", "views", "score", "owner"]);
        f.extend([V::S(if r == 1 { "Top Post" } else if r <= 5 { "Top 5 Posts" } else { "Other Posts" }), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.CreationDate, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN p.LastEditDate IS NOT NULL THEN 1 ELSE 0 END) AS TotalEdits,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName, u.Reputation, u.CreationDate),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, TotalQuestions, TotalAnswers, TotalEdits, TotalUpvotes, TotalDownvotes, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserStats)
// SELECT tu.Rank, tu.DisplayName, tu.Reputation, tu.TotalPosts, tu.TotalQuestions, tu.TotalAnswers, tu.TotalEdits, tu.TotalUpvotes, tu.TotalDownvotes FROM TopUsers tu WHERE tu.Rank <= 10 ORDER BY tu.Rank;
fn q8905(db: &'static So) -> String {
    let Post { post_type_id, last_edit_date, .. } = &db.post;
    let users = || db.user.with((&db.user.reputation).gt(1000));
    let s = users()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(last_edit_date.opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())))
        .fold([0i64; 5], |a, ((t, e), v)| [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + e.is_some() as i64, a[3] + (v == Some(2)) as i64, a[4] + (v == Some(3)) as i64]);
    let np = users().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let w = whole(&np).select(Ident::<User>::new().and(&db.user.reputation)).window(row_number, |(u, r)| (Reverse(r), u), asc);
    let top: MatSet<(Id<User>, i64)> = (&w).filt(|(_, k)| k <= 10).map(|((u, _), k)| (u, k)).collect();
    let v = drain((&top).map(|(u, _)| u).select(Ident::<User>::new().and(by_first(&top)).and(&np).and((&s).opt())));
    rows(v.into_iter().map(|(_, (((u, i), n), a))| {
        let mut f = vec![V::I(i)];
        f.extend(ucols(db, u, &["name", "rep"]));
        f.push(V::I(n));
        f.extend(a.unwrap_or([0; 5]).map(V::I));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.Reputation, U.CreationDate, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN P.ViewCount IS NOT NULL THEN P.ViewCount ELSE 0 END) AS TotalViews,
//        SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.Reputation, U.CreationDate),
// TopUsers AS (SELECT UserId, Reputation, CreationDate, PostCount, QuestionCount, AnswerCount, TotalViews, UpVotes, DownVotes,
//        RANK() OVER (ORDER BY Reputation DESC) AS RankByReputation, RANK() OVER (ORDER BY TotalViews DESC) AS RankByViews FROM UserStats)
// SELECT UserId, Reputation, CreationDate, PostCount, QuestionCount, AnswerCount, TotalViews, UpVotes, DownVotes, RankByReputation, RankByViews
// FROM TopUsers WHERE RankByReputation <= 10 OR RankByViews <= 10 ORDER BY RankByReputation, RankByViews;
fn q10682(db: &'static So) -> String {
    let Post { post_type_id, view_count, .. } = &db.post;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(view_count.opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 5], |a, p| match p {
            Some(((t, w), v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + w.unwrap_or(0), a[3] + (v == Some(2)) as i64, a[4] + (v == Some(3)) as i64],
            None => a,
        });
    let np = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    type R = ((Id<User>, (i64, [i64; 5])), i64);
    let w = whole(&np).select(Ident::<User>::new().and((&np).and(&s)).and(&db.user.reputation)).window(rank, |(_, r): R| Reverse(r), asc);
    let w = (&w).window(rank, |(((_, (_, a)), _), _): (R, i64)| Reverse(a[2]), asc);
    let mut v = drain((&w).filt(|((_, r), u)| r <= 10 || u <= 10));
    v.sort_by_key(|&(_, ((_, r), w))| (r, w));
    rows(v.into_iter().map(|(_, ((((u, (n, a)), _), r), w))| {
        let mut f = ucols(db, u, &["uid", "rep", "ucreated"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.extend([V::I(r), V::I(w)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.OwnerUserId, COALESCE(u.DisplayName, 'Community User') AS OwnerDisplayName, p.CreationDate, p.Score,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS OwnerPostRank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// PostStats AS (SELECT rp.OwnerDisplayName, COUNT(rp.Id) AS PostCount, AVG(rp.Score) AS AvgScore, SUM(CASE WHEN rp.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts,
//        SUM(CASE WHEN rp.Score < 0 THEN 1 ELSE 0 END) AS NegativePosts FROM RankedPosts rp GROUP BY rp.OwnerDisplayName),
// TopUsers AS (SELECT ps.OwnerDisplayName, ps.PostCount, ps.AvgScore, PS.PositivePosts, ps.NegativePosts, ROW_NUMBER() OVER (ORDER BY ps.PostCount DESC) AS UserRank FROM PostStats ps)
// SELECT tu.OwnerDisplayName, tu.PostCount, tu.AvgScore, CASE WHEN tu.PositivePosts > tu.NegativePosts THEN 'Positive Contributor' ELSE 'Negative Contributor' END AS ContributorType
// FROM TopUsers tu WHERE tu.UserRank <= 10 ORDER BY tu.PostCount DESC;
fn q2315(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let ps = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(owner_user.select(&db.user.display_name).opt().map(|n: Option<Str>| n.unwrap_or("Community User")))
        .select(score)
        .fold([0i64; 4], |a, s| [a[0] + 1, a[1] + s, a[2] + (s > 0) as i64, a[3] + (s < 0) as i64]);
    let v = top_n(drain(&ps), |&(n, a)| (Reverse(a[0]), n), 10);
    rows(v.into_iter().map(|(n, a)| row(vec![V::S(n), V::I(a[0]), avg(a[1], a[0]), V::S(if a[2] > a[3] { "Positive Contributor" } else { "Negative Contributor" })])))
}

// WITH UserPerformance AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, AVG(COALESCE(v.VoteCount, 0)) AS AvgVotes, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN (SELECT PostId, COUNT(*) AS VoteCount FROM Votes GROUP BY PostId) v ON p.Id = v.PostId
//     LEFT JOIN Badges b ON u.Id = b.UserId WHERE u.Reputation > 100 GROUP BY u.Id, u.DisplayName),
// TopPerformers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, AvgVotes, GoldBadges, SilverBadges, BronzeBadges,
//        RANK() OVER (ORDER BY TotalPosts DESC, AvgVotes DESC) AS Rank FROM UserPerformance)
// SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, AvgVotes, GoldBadges, SilverBadges, BronzeBadges FROM TopPerformers WHERE Rank <= 10 ORDER BY Rank;
fn q6098(db: &'static So) -> String {
    let vc = db.vote.group_by(&db.vote.post).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let users = || db.user.with((&db.user.reputation).gt(100));
    let s = users()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and((&vc).opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 7], |a, (p, b)| {
            let (t, n) = p.map_or((0, None), |x| x);
            [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + n.unwrap_or(0), a[3] + 1, a[4] + (b == Some(1)) as i64, a[5] + (b == Some(2)) as i64, a[6] + (b == Some(3)) as i64]
        });
    let np = users().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let w = whole(&np).select(Ident::<User>::new().and((&np).and(&s))).window(rank, |(_, (n, a))| (Reverse(n), Reverse(fkey(a[2] as f64 / a[3] as f64))), asc);
    let v = drain((&w).filt(|(_, r)| r <= 10));
    rows(v.into_iter().map(|(_, ((u, (n, a)), _))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), avg(a[2], a[3]), V::I(a[4]), V::I(a[5]), V::I(a[6])]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN C.Id IS NOT NULL THEN 1 ELSE 0 END) AS CommentCount,
//        SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, AnswerCount, QuestionCount, CommentCount, GoldBadges, SilverBadges, BronzeBadges,
//        RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserStats)
// SELECT UserId, DisplayName, Reputation, PostCount, AnswerCount, QuestionCount, CommentCount, GoldBadges, SilverBadges, BronzeBadges FROM TopUsers WHERE ReputationRank <= 10;
//
// ReputationRank reads only Reputation, so the top users are picked first and the post x comment x badge product is driven for those alone.
fn q9462(db: &'static So) -> String {
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r)| Reverse(r), asc);
    let tu: MatSet<Id<User>> = (&w).filt(|(_, r)| r <= 10).map(|((u, _), _)| u).collect();
    let s = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(comments_of(db).opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 6], |a, (p, b)| {
            let (t, c) = p.map_or((0, None), |x| x);
            [a[0] + (t == 2) as i64, a[1] + (t == 1) as i64, a[2] + c.is_some() as i64, a[3] + (b == Some(1)) as i64, a[4] + (b == Some(2)) as i64, a[5] + (b == Some(3)) as i64]
        });
    let np = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    rows(drain((&np).and(&s)).into_iter().map(|(u, (n, a))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RecursiveUserActivity AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, ROW_NUMBER() OVER (PARTITION BY u.Id ORDER BY COUNT(p.Id) DESC) AS ActivityRank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation HAVING COUNT(p.Id) > 0),
// RecentPostHistory AS (SELECT ph.PostId, ph.CreationDate, ph.UserId, p.Title, p.PostTypeId, ph.Comment, ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS CommentRank
//     FROM PostHistory ph JOIN Posts p ON ph.PostId = p.Id WHERE ph.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days')
// SELECT ua.DisplayName, ua.Reputation, ua.PostCount, ua.QuestionCount, ua.AnswerCount, rp.Title, rp.Comment, rp.CreationDate AS LastCommentDate,
//        CASE WHEN rp.Comment IS NOT NULL THEN 'Engaged' ELSE 'Inactive' END AS EngagementStatus
// FROM RecursiveUserActivity ua LEFT JOIN RecentPostHistory rp ON ua.UserId = rp.UserId WHERE ua.ActivityRank = 1 ORDER BY ua.Reputation DESC, LastCommentDate DESC LIMIT 50;
//
// ActivityRank partitions by u.Id, so it is 1 for every user.
fn q32414(db: &'static So) -> String {
    let ups = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(&db.post.post_type_id)).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64]);
    let PostHistory { user, creation_date: hd, .. } = &db.post_history;
    let recent: HashIdx<Id<User>, Id<PostHistory>> = db.post_history.with(hd.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(user).inv().collect();
    let v = drain((&ups).and((&recent).opt()));
    let v = top_n(v, |&(u, (_, h))| {
        let d = h.map(|h| hd.get(h).unwrap());
        (Reverse(db.user.reputation.get(u).unwrap()), d.is_none(), Reverse(d), u, h)
    }, 50);
    rows(v.into_iter().map(|(u, (a, h))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(a.map(V::I));
        f.extend(match h {
            Some(h) => {
                let c = db.post_history.comment.get(h);
                vec![title(db, db.post_history.post.get(h).unwrap()), ostr(c), V::T(hd.get(h).unwrap()), V::S(if c.is_some() { "Engaged" } else { "Inactive" })]
            }
            None => vec![V::Null, V::Null, V::Null, V::S("Inactive")],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT a.Id) AS AnswerCount,
//        RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2 JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.Score > 0 GROUP BY p.Id, p.Title, p.Score, p.ViewCount, u.DisplayName, p.PostTypeId),
// TopPosts AS (SELECT Rank, PostId, Title, Score, ViewCount, OwnerDisplayName, CommentCount, AnswerCount FROM RankedPosts WHERE Rank <= 10)
// SELECT t.PostId, t.Title, t.Score, t.ViewCount, t.OwnerDisplayName, t.CommentCount, t.AnswerCount, COALESCE(b.BadgeCount, 0) AS BadgeCount
// FROM TopPosts t LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId) b ON b.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = t.PostId) ORDER BY t.Score DESC;
fn q6509(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(score.gt(0)))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score))
        .window(rank, |(_, s)| Reverse(s), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|((p, _), _)| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ac = (&tp).group_by(Ident::<Post>::new()).select(answers_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    rows(drain((&cc).and(&ac).and(owner_user.select((&bc).opt()))).into_iter().map(|(p, ((c, a), b))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "owner"]);
        f.extend([V::I(c), V::I(a), V::I(b.unwrap_or(0))]);
        row(f)
    }))
}

// WITH PostDetails AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        COALESCE((SELECT COUNT(*) FROM Posts a WHERE a.AcceptedAnswerId = p.Id), 0) AS AcceptedAnswerCount
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year' GROUP BY p.Id, u.DisplayName, p.Title, p.CreationDate, p.ViewCount),
// TopPosts AS (SELECT pd.PostId, pd.Title, pd.CreationDate, pd.ViewCount, pd.OwnerDisplayName, pd.CommentCount, pd.UpVotes, pd.DownVotes, pd.AcceptedAnswerCount,
//        RANK() OVER (ORDER BY pd.ViewCount DESC, pd.UpVotes DESC) AS Rank FROM PostDetails pd)
// SELECT tp.*, CASE WHEN tp.AcceptedAnswerCount > 0 THEN 'Yes' ELSE 'No' END AS HasAcceptedAnswer FROM TopPosts tp WHERE tp.Rank <= 10 ORDER BY tp.Rank;
fn q7868(db: &'static So) -> String {
    let Post { post_type_id, creation_date, view_count, accepted_answer, .. } = &db.post;
    let pd = db
        .post
        .with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let acc: HashIdx<Id<Post>, Id<Post>> = accepted_answer.inv().collect();
    let ac = db.post.group_by(Ident::<Post>::new()).select((&acc).opt()).fold(0i64, |n, a| n + a.is_some() as i64);
    let w = whole(&pd).select(Ident::<Post>::new().and((&pd).and(&ac)).and(view_count.opt())).window(rank, |((_, (a, _)), w)| (w.is_none(), Reverse(w), Reverse(a[1])), asc);
    let v = drain((&w).filt(|(_, r)| r <= 10));
    rows(v.into_iter().map(|(_, (((p, (a, n)), _), r))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(n), V::I(r), V::S(if n > 0 { "Yes" } else { "No" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS rn
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT rp.Id, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.OwnerDisplayName FROM RankedPosts rp WHERE rp.rn <= 5),
// PostStats AS (SELECT p.Id AS PostId, COUNT(c.Id) AS CommentCount, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (8, 9) WHERE p.PostTypeId = 1 GROUP BY p.Id)
// SELECT tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.OwnerDisplayName, ps.CommentCount, ps.TotalBounty, CASE WHEN ps.TotalBounty > 0 THEN 'Has Bounty' ELSE 'No Bounty' END AS BountyStatus
// FROM TopPosts tp LEFT JOIN PostStats ps ON tp.Id = ps.PostId ORDER BY tp.Score DESC, tp.ViewCount DESC LIMIT 10;
fn q2485(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, view_count, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(score))
        .window(row_number, |(p, s)| (Reverse(s), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt());
    let ps = (&tp)
        .with(post_type_id.eq(1))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(bounty.opt()))
        .fold([0i64; 2], |a, (c, b)| [a[0] + c.is_some() as i64, a[1] + b.flatten().unwrap_or(0)]);
    let v = top_n(drain((&tp).select((&ps).opt())), |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 10);
    rows(v.into_iter().map(|(p, s)| {
        let mut f = post_fields(db, p, &["title", "created", "score", "views", "owner"]);
        f.extend(match s {
            Some(a) => [V::I(a[0]), V::I(a[1]), V::S(if a[1] > 0 { "Has Bounty" } else { "No Bounty" })],
            None => [V::Null, V::Null, V::S("No Bounty")],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.CreationDate, p.Score, p.Tags, u.DisplayName AS Author, ROW_NUMBER() OVER (PARTITION BY p.Tags ORDER BY p.Score DESC) AS TagRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.Score > 0),
// TagStatistics AS (SELECT TRIM(BOTH '<>' FROM tag) AS Tag, COUNT(*) AS TotalQuestions, SUM(ViewCount) AS TotalViews, AVG(Score) AS AverageScore
//     FROM (SELECT UNNEST(string_to_array(Trim(Both '<>' FROM Tags), '>tag<')) AS tag, ViewCount, Score FROM RankedPosts) AS unnested_tags GROUP BY Tag),
// TopAuthorStats AS (SELECT Author, COUNT(*) AS TotalPosts, SUM(ViewCount) AS TotalViews, AVG(Score) AS AverageScore FROM RankedPosts GROUP BY Author)
// SELECT ts.Tag, ts.TotalQuestions, ts.TotalViews, ts.AverageScore, tas.Author, tas.TotalPosts, tas.TotalViews AS AuthorViews, tas.AverageScore AS AuthorAverageScore
// FROM TagStatistics ts JOIN TopAuthorStats tas ON ts.TotalQuestions > 5 ORDER BY ts.TotalQuestions DESC, tas.TotalPosts DESC LIMIT 10;
//
// Each question's Tags is trimmed of '<' and '>', split on '>tag<', and each piece trimmed again. The ON clause names only ts, so the two are crossed.
fn q29933(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, view_count, tags_str, .. } = &db.post;
    let rp = || db.post.with(post_type_id.eq(1).and(score.gt(0))).with(owner_user);
    let trim = |t: Str| -> Str { t.trim_matches(|c| c == '<' || c == '>') };
    let tst = rp()
        .group_by(tags_str.flat_map(move |t: Str| trim(t).split(">tag<").map(trim).collect::<Vec<Str>>()))
        .select(score.and(view_count.opt()))
        .fold([0i64; 4], |a, (s, w)| [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s]);
    let tas = rp()
        .group_by(owner_user.select(&db.user.display_name))
        .select(score.and(view_count.opt()))
        .fold([0i64; 4], |a, (s, w)| [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s]);
    let tst = (&tst).filt(|a| a[0] > 5);
    let tt = whole(&tst).select(Same::new().and(&tst)).window(rank, |(_, a)| a[0], desc).filt(|(_, r)| r <= 10).map(|(x, _)| x);
    let ta = whole(&tas).select(Same::new().and(&tas)).window(rank, |(_, b)| b[0], desc).filt(|(_, r)| r <= 10).map(|(x, _)| x);
    let v = drain(tt.cross(ta));
    let v = top_n(v, |&(_, ((t, a), (n, b)))| (Reverse(a[0]), Reverse(b[0]), t, n), 10);
    rows(v.into_iter().map(|(_, ((t, a), (n, b)))| row(vec![V::S(t), V::I(a[0]), nullable(a[2], a[1]), avg(a[3], a[0]), V::S(n), V::I(b[0]), nullable(b[2], b[1]), avg(b[3], b[0])])))
}

// WITH RECURSIVE UserBadges AS (SELECT U.Id AS UserId, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id),
// RecentPosts AS (SELECT P.Id AS PostId, P.OwnerUserId, P.Title, P.CreationDate, P.Score, RANK() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS PostRank
//     FROM Posts P WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days')
// SELECT U.DisplayName, U.Reputation, U.Location, U.CreationDate, UBad.BadgeCount, UBad.GoldBadges, UBad.SilverBadges, UBad.BronzeBadges, RPost.PostId, RPost.Title AS RecentPostTitle,
//        RPost.Score AS RecentPostScore, RPost.CreationDate AS RecentPostDate
// FROM Users U JOIN UserBadges UBad ON U.Id = UBad.UserId LEFT JOIN RecentPosts RPost ON U.Id = RPost.OwnerUserId AND RPost.PostRank = 1
// WHERE U.Reputation > 1000 AND U.Location IS NOT NULL AND (UBad.GoldBadges > 0 OR UBad.SilverBadges > 2) ORDER BY U.Reputation DESC, RPost.CreationDate DESC FETCH FIRST 10 ROWS ONLY;
//
// WITH RECURSIVE, but no CTE refers to itself.
fn q33807(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let Post { creation_date, owner_user, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(creation_date))
        .window(rank, |(_, d)| Reverse(d), asc);
    let last: HashIdx<Id<User>, Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let v = drain(
        db.user
            .with((&db.user.reputation).gt(1000))
            .with(&db.user.location)
            .select((&ub).filt(|a| a[1] > 0 || a[2] > 2))
            .and((&last).opt()),
    );
    let v = top_n(v, |&(u, (_, p))| {
        let d = p.map(|p| creation_date.get(p).unwrap());
        (Reverse(db.user.reputation.get(u).unwrap()), d.is_none(), Reverse(d), u, p)
    }, 10);
    rows(v.into_iter().map(|(u, (a, p))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(ostr(db.user.location.get(u)));
        f.push(user_col(db, u, "ucreated"));
        f.extend(a.map(V::I));
        f.extend(match p {
            Some(p) => post_fields(db, p, &["id", "title", "score", "created"]),
            None => vec![V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.LastActivityDate DESC) AS RankByOwner FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, AnswerCount, OwnerDisplayName FROM RankedPosts WHERE RankByOwner = 1),
// PopularTags AS (SELECT t.TagName, COUNT(*) AS PostCount FROM Posts p JOIN UNNEST(string_to_array(p.Tags, '><')) AS t(TagName) ON p.Id = p.Id GROUP BY t.TagName ORDER BY PostCount DESC LIMIT 5),
// CombinedMetrics AS (SELECT tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.AnswerCount, pt.TagName, pt.PostCount FROM TopPosts tp CROSS JOIN PopularTags pt)
// SELECT cm.Title, cm.CreationDate, cm.Score, cm.ViewCount, cm.AnswerCount, cm.TagName, cm.PostCount FROM CombinedMetrics cm ORDER BY cm.Score DESC, cm.ViewCount DESC;
//
// string_to_array splits on '><' without stripping the outer brackets, so the first and last elements keep one.
fn q5405(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, last_activity_date, tags_str, .. } = &db.post;
    let w = db
        .post
        .with(post_type_id.eq(1))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(score).and(last_activity_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|(((p, _), _), _)| p).collect();
    let pt = db.post.select(tags_str.flat_map(|t: Str| t.split("><"))).group_by(Same::<Str>::new()).select(Same::<Str>::new()).fold(0i64, |n, _| n + 1);
    let pt = rel(top_n(drain(&pt), |&(t, n)| (Reverse(n), t), 5));
    let v = drain((&tp).cross(&pt));
    rows(v.into_iter().map(|((p, _), (_, (t, n)))| {
        let mut f = post_fields(db, p, &["title", "created", "score", "views", "answers"]);
        f.extend([V::S(t), V::I(n)]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(P.Score) AS TotalScore, MAX(P.CreationDate) AS LastPostDate
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalScore, LastPostDate, RANK() OVER (ORDER BY TotalScore DESC) AS Rank FROM UserPostStats),
// PostHistoryCount AS (SELECT PH.UserId, COUNT(PH.Id) AS HistoryCount FROM PostHistory PH GROUP BY PH.UserId)
// SELECT TU.DisplayName, TU.TotalPosts, TU.TotalQuestions, TU.TotalAnswers, TU.TotalScore, TU.LastPostDate, COALESCE(PHC.HistoryCount, 0) AS HistoryCount,
//        CASE WHEN TU.TotalPosts > 50 THEN 'Veteran' WHEN TU.TotalPosts BETWEEN 20 AND 50 THEN 'Active' ELSE 'Newcomer' END AS UserCategory
// FROM TopUsers TU LEFT JOIN PostHistoryCount PHC ON TU.UserId = PHC.UserId WHERE TU.Rank <= 10 ORDER BY TU.TotalScore DESC;
fn q2159(db: &'static So) -> String {
    let ups = user_posts(db);
    let hc = db.post_history.group_by(&db.post_history.user).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let w = whole(&ups).select(Ident::<User>::new().and((&ups).and((&hc).opt()))).window(rank, |(_, (a, _))| (a[1] == 0, Reverse(a[4])), asc);
    let v = drain((&w).filt(|(_, r)| r <= 10));
    rows(v.into_iter().map(|(_, ((u, (a, h)), _))| {
        row(vec![
            user_col(db, u, "name"),
            V::I(a[1]),
            V::I(a[2]),
            V::I(a[3]),
            nullable(a[4], a[1]),
            tmax(a[7]),
            V::I(h.unwrap_or(0)),
            V::S(if a[1] > 50 { "Veteran" } else if a[1] >= 20 { "Active" } else { "Newcomer" }),
        ])
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, U.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS PostRank FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id WHERE P.PostTypeId = 1 AND P.Score > 0),
// ClosedPosts AS (SELECT PH.PostId, PH.CreationDate, C.Name AS CloseReason FROM PostHistory PH JOIN CloseReasonTypes C ON CAST(PH.Comment AS INTEGER) = C.Id WHERE PH.PostHistoryTypeId = 10),
// CombinedData AS (SELECT RP.PostId, RP.Title, RP.CreationDate, RP.Score, RP.ViewCount, RP.OwnerDisplayName, COALESCE(CP.CloseReason, 'Not Closed') AS CloseReason, RP.PostRank
//     FROM RankedPosts RP LEFT JOIN ClosedPosts CP ON RP.PostId = CP.PostId)
// SELECT CD.OwnerDisplayName, CD.Title, CD.CreationDate, CD.Score, CD.ViewCount, CD.CloseReason, CASE WHEN CD.PostRank = 1 THEN 'Latest Post' ELSE 'Other Posts' END AS PostStatus
// FROM CombinedData CD WHERE CD.CloseReason IS NULL OR CD.CloseReason = 'Not Closed' ORDER BY CD.Score DESC, CD.CreationDate DESC;
//
// CloseReason is 'Not Closed' exactly when the LEFT JOIN found no close row, so the WHERE keeps the questions without one.
fn q2911(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, creation_date, .. } = &db.post;
    let rp: MatSet<Id<Post>> = db.post.with(post_type_id.eq(1).and(score.gt(0))).with(owner_user).collect();
    let w = (&rp).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let newest: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let reason_of = || comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason);
    let closes: HashIdx<Id<Post>, Id<PostHistory>> = db.post_history.with(post_history_type_id.eq(10)).with(reason_of()).select(post).inv().collect();
    let v = drain((&rp).select((&closes).select(reason_of()).opt().filt(|r: Option<Str>| r.map_or(true, |r| r == "Not Closed")).and(Ident::<Post>::new().with(&newest).opt())));
    rows(v.into_iter().map(|(p, (r, n))| {
        let mut f = post_fields(db, p, &["owner", "title", "created", "score", "views"]);
        f.extend([V::S(r.unwrap_or("Not Closed")), V::S(if n.is_some() { "Latest Post" } else { "Other Posts" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, COUNT(a.Id) AS AnswerCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, RANK() OVER (ORDER BY COUNT(a.Id) DESC, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) DESC) AS Rank
//     FROM Posts p LEFT JOIN Posts a ON p.Id = a.ParentId AND p.PostTypeId = 1 LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year' GROUP BY p.Id, p.Title),
// PostDetails AS (SELECT rp.PostId, rp.Title, rp.AnswerCount, rp.UpVotes, rp.DownVotes,
//        CASE WHEN rp.AnswerCount > 0 THEN ROUND(CAST(rp.UpVotes AS FLOAT) / (rp.UpVotes + rp.DownVotes) * 100, 2) ELSE 0 END AS UpVotePercentage FROM RankedPosts rp WHERE rp.Rank <= 100)
// SELECT pd.Title, pd.AnswerCount, pd.UpVotes, pd.DownVotes, pd.UpVotePercentage, COALESCE(t.TagName, 'No Tags') AS MostRelevantTag
// FROM PostDetails pd LEFT JOIN Posts p ON pd.PostId = p.Id LEFT JOIN Tags t ON p.Id = t.ExcerptPostId ORDER BY pd.UpVotePercentage DESC, pd.AnswerCount DESC;
//
// The `p.PostTypeId = 1` in the first ON names only p, so every child post joins, not just answers.
fn q7686(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let rp = db
        .post
        .with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))))
        .group_by(Ident::<Post>::new())
        .select(children_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let w = whole(&rp).select(Ident::<Post>::new().and(&rp)).window(rank, |(_, a)| (Reverse(a[0]), Reverse(a[1])), asc);
    let pd: MatSet<(Id<Post>, [i64; 3])> = (&w).filt(|(_, r)| r <= 100).map(|(x, _)| x).collect();
    let excerpt: HashIdx<Id<Post>, Id<Tag>> = (&db.tag.excerpt_post).inv().collect();
    let v = drain((&pd).select(Same::<(Id<Post>, [i64; 3])>::new().and(Same::<(Id<Post>, [i64; 3])>::new().map(|(p, _)| p).select((&excerpt).opt()))));
    rows(v.into_iter().map(|(_, ((p, a), t))| {
        let pct = if a[0] > 0 {
            let d = a[1] + a[2];
            if d == 0 { V::Null } else { V::F(((a[1] as f32 / d as f32 * 100.0f32 * 100.0f32).round() / 100.0f32) as f64) }
        } else {
            V::F(0.0)
        };
        let mut f = post_fields(db, p, &["title"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), pct, V::S(t.map_or("No Tags", |t| db.tag.tag_name.get(t).unwrap()))]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(COALESCE(V.VoteCount, 0)) AS TotalVotes, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN (SELECT PostId, COUNT(Id) AS VoteCount FROM Votes GROUP BY PostId) V ON P.Id = V.PostId
//     LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, TotalVotes, GoldBadges, SilverBadges, BronzeBadges, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserStats)
// SELECT Rank, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, TotalVotes, GoldBadges, SilverBadges, BronzeBadges FROM TopUsers WHERE Rank <= 10 ORDER BY Rank;
//
// Rank reads only Reputation, so the top users are picked first and the post x badge product is driven for those alone.
fn q8368(db: &'static So) -> String {
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&db.user.reputation)).window(row_number, |(u, r)| (Reverse(r), u), asc);
    let tu: MatSet<(Id<User>, i64)> = (&w).filt(|(_, k)| k <= 10).map(|((u, _), k)| (u, k)).collect();
    let vc = db.vote.group_by(&db.vote.post).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let s = (&tu)
        .map(|(u, _)| u)
        .group_by(Same::<Id<User>>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and((&vc).opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 6], |a, (p, b)| {
            let (t, n) = p.map_or((0, None), |x| x);
            [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + n.unwrap_or(0), a[3] + (b == Some(1)) as i64, a[4] + (b == Some(2)) as i64, a[5] + (b == Some(3)) as i64]
        });
    let np = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let v = drain((&tu).select(Same::<(Id<User>, i64)>::new().and(Same::<(Id<User>, i64)>::new().map(|(u, _)| u).select((&np).and(&s)))));
    rows(v.into_iter().map(|(_, ((u, r), (n, a)))| {
        let mut f = vec![V::I(r)];
        f.extend(ucols(db, u, &["name", "rep"]));
        f.push(V::I(n));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounties, ROW_NUMBER() OVER (ORDER BY COUNT(p.Id) DESC) AS PostRank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 8 GROUP BY u.Id, u.DisplayName),
// ClosedPosts AS (SELECT ph.UserId, COUNT(DISTINCT ph.PostId) AS ClosedPostCount FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.UserId),
// TopUsers AS (SELECT ups.UserId, ups.DisplayName, ups.TotalPosts, ups.TotalQuestions, ups.TotalAnswers, ups.TotalBounties, COALESCE(cp.ClosedPostCount, 0) AS ClosedPosts
//     FROM UserPostStats ups LEFT JOIN ClosedPosts cp ON ups.UserId = cp.UserId WHERE ups.PostRank <= 10)
// SELECT tu.DisplayName, tu.TotalPosts, tu.TotalQuestions, tu.TotalAnswers, tu.TotalBounties, tu.ClosedPosts FROM TopUsers tu ORDER BY tu.TotalPosts DESC, tu.TotalBounties DESC;
fn q3280(db: &'static So) -> String {
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(bounty.opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some((t, b)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + b.flatten().unwrap_or(0)],
            None => a,
        });
    let PostHistory { user, post, post_history_type_id, .. } = &db.post_history;
    let cp = db.post_history.with(post_history_type_id.eq(10)).group_by(user).select(post).count_distinct();
    let v = top_n(drain((&s).and((&cp).opt())), |&(u, (a, _))| (Reverse(a[0]), u), 10);
    rows(v.into_iter().map(|(u, (a, c))| row(vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(c.unwrap_or(0))])))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.Reputation, U.CreationDate, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(CASE WHEN P.PostTypeId = 3 THEN 1 ELSE 0 END) AS Wikis,
//        SUM(CASE WHEN P.LastActivityDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' THEN 1 ELSE 0 END) AS RecentActivity, MAX(P.LastActivityDate) AS LastActiveDate
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId WHERE U.Reputation > 1000 GROUP BY U.Id, U.Reputation, U.CreationDate),
// TopUsers AS (SELECT UserId, Reputation, PostCount, Questions, Answers, Wikis, RecentActivity, LastActiveDate, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserStats)
// SELECT T.UserId, U.DisplayName, T.Reputation, T.PostCount, T.Questions, T.Answers, T.Wikis, T.RecentActivity, T.LastActiveDate,
//        RANK() OVER (PARTITION BY T.ReputationRank ORDER BY T.Reputation DESC) AS RankInGroup
// FROM TopUsers T JOIN Users U ON T.UserId = U.Id WHERE T.ReputationRank <= 10 ORDER BY T.Reputation DESC, RankInGroup;
//
// RankInGroup ranks within a group of equal reputation, so it is 1 for every row.
fn q7249(db: &'static So) -> String {
    let Post { post_type_id, last_activity_date, .. } = &db.post;
    let cut = add_days(ts(2024, 10, 1, 12, 34, 56), -30);
    let s = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(last_activity_date)).opt())
        .fold([0, 0, 0, 0, 0, i64::MIN], |a, p| match p {
            Some((t, d)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (t == 3) as i64, a[4] + (d > cut) as i64, a[5].max(d)],
            None => a,
        });
    let w = whole(&s).select(Ident::<User>::new().and(&s).and(&db.user.reputation)).window(rank, |(_, r)| Reverse(r), asc);
    let v = drain((&w).filt(|(_, k)| k <= 10));
    rows(v.into_iter().map(|(_, (((u, a), _), _))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4]), tmax(a[5]), V::I(1)]);
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, COUNT(DISTINCT c.Id) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS OwnerPostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId),
// TopUsers AS (SELECT u.Id, u.DisplayName, u.Reputation, RANK() OVER (ORDER BY u.Reputation DESC) AS UserRank FROM Users u WHERE u.Reputation > 1000)
// SELECT rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, tu.DisplayName AS OwnerDisplayName, tu.Reputation, CASE WHEN rp.CommentCount > 0 THEN 'Has Comments' ELSE 'No Comments' END AS CommentStatus,
//        (SELECT COUNT(*) FROM Votes v WHERE v.PostId = rp.Id AND v.VoteTypeId = 2) AS UpVoteCount, (SELECT COUNT(*) FROM Votes v WHERE v.PostId = rp.Id AND v.VoteTypeId = 3) AS DownVoteCount
// FROM RecentPosts rp JOIN TopUsers tu ON rp.OwnerUserId = tu.Id WHERE rp.OwnerPostRank = 1 ORDER BY rp.CreationDate DESC LIMIT 10;
fn q3064(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_days(date(2024, 10, 1), -30)))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (Reverse(d), p), asc);
    let fp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let rich = Ident::<User>::new().with((&db.user.reputation).gt(1000));
    let tp: MatSet<Id<Post>> = (&fp).with(owner_user.select(rich)).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = top_n(drain((&cc).and(&vc)), |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 10);
    rows(v.into_iter().map(|(p, (c, a))| {
        let mut f = post_fields(db, p, &["title", "created", "score", "views", "owner", "rep"]);
        f.extend([V::S(if c > 0 { "Has Comments" } else { "No Comments" }), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.Score, p.ViewCount, p.CreationDate, u.DisplayName AS OwnerDisplayName, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) as rn
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1),
// TopUsers AS (SELECT u.Id, u.DisplayName, COUNT(p.Id) AS TotalQuestions, SUM(COALESCE(p.Score, 0)) AS TotalScore FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId
//     WHERE p.PostTypeId = 1 GROUP BY u.Id, u.DisplayName HAVING COUNT(p.Id) > 5),
// ClosedPostHistory AS (SELECT ph.PostId, ph.CreationDate, ph.Comment FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11))
// SELECT rp.Title, rp.Score, rp.ViewCount, rp.CreationDate, rp.OwnerDisplayName, CASE WHEN CUP.Comment IS NOT NULL THEN 'Closed' ELSE 'Active' END AS PostStatus, tu.TotalQuestions, tu.TotalScore
// FROM RankedPosts rp JOIN TopUsers tu ON rp.OwnerDisplayName = tu.DisplayName LEFT JOIN ClosedPostHistory CUP ON rp.Id = CUP.PostId WHERE rp.rn = 1
// ORDER BY tu.TotalScore DESC, rp.CreationDate DESC LIMIT 50;
fn q4419(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1)).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let fp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let tu = db.post.with(post_type_id.eq(1)).group_by(owner_user).select(score).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let tus: MatSet<Id<User>> = db.user.with((&tu).filt(|a| a[0] > 5)).collect();
    let by_name: HashIdx<Str, Id<User>> = (&tus).select(&db.user.display_name).inv().collect();
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).in_v(vec![10, 11])));
    let v = drain((&fp).select(owner_user.select(&db.user.display_name).select(&by_name).select(Ident::<User>::new().and(&tu)).and(closes.opt())));
    let v = top_n(v, |&(p, ((_, a), h))| (Reverse(a[1]), Reverse(creation_date.get(p).unwrap()), p, h), 50);
    rows(v.into_iter().map(|(p, ((_, a), h))| {
        let mut f = post_fields(db, p, &["title", "score", "views", "created", "owner"]);
        f.extend([V::S(if h.map_or(false, |h| db.post_history.comment.get(h).is_some()) { "Closed" } else { "Active" }), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p WHERE p.PostTypeId IN (1, 2) AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 YEAR'),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT b.Id) AS BadgeCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON u.Id = v.UserId
//     GROUP BY u.Id, u.DisplayName ORDER BY BadgeCount DESC LIMIT 10)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, rp.CommentCount, tu.DisplayName AS TopUser, tu.BadgeCount, tu.Upvotes, tu.Downvotes
// FROM RankedPosts rp JOIN TopUsers tu ON rp.PostId IN (SELECT p.Id FROM Posts p WHERE p.OwnerUserId = tu.UserId) WHERE rp.Rank <= 5 ORDER BY rp.Score DESC;
//
// The LIMIT reads only BadgeCount, so the ten users are picked first and the badge x vote product is driven for those alone.
fn q6373(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, owner_user, .. } = &db.post;
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let tu: MatSet<Id<User>> = rel(top_n(drain(&bc), |&(u, n)| (Reverse(n), u), 10).into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let uv = (&tu)
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(votes_by(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let w = db
        .post
        .with(post_type_id.is_in([1, 2]).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|(((p, _), _), _)| p).collect();
    rows(drain((&tp).select(owner_user.select(Ident::<User>::new().and(&bc).and(&uv)))).into_iter().map(|(p, ((u, b), a))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments"]);
        f.extend([user_col(db, u, "name"), V::I(b), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'),
// UserActivities AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpvotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownvotes FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName),
// PostHistoryCounts AS (SELECT ph.PostId, COUNT(*) AS HistoryCount FROM PostHistory ph WHERE ph.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, ua.DisplayName, ua.TotalUpvotes, ua.TotalDownvotes, COALESCE(phc.HistoryCount, 0) AS HistoryCount
// FROM RecentPosts rp JOIN Users u ON rp.OwnerUserId = u.Id JOIN UserActivities ua ON u.Id = ua.UserId LEFT JOIN PostHistoryCounts phc ON rp.PostId = phc.PostId
// WHERE rp.rn = 1 AND (ua.TotalUpvotes - ua.TotalDownvotes) >= 10 ORDER BY rp.CreationDate DESC LIMIT 50;
fn q2709(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let w = db.post.with(creation_date.ge(add_days(t0, -30))).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let fp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let ua = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let hd = &db.post_history.creation_date;
    let hc = db.post_history.with(hd.ge(add_years(t0, -1))).group_by(&db.post_history.post).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&fp).select(owner_user.select(Ident::<User>::new().and((&ua).filt(|a| a[0] - a[1] >= 10))).and((&hc).opt())));
    let v = top_n(v, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 50);
    rows(v.into_iter().map(|(p, ((u, a), h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(h.unwrap_or(0))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.Score, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn, p.OwnerUserId
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews, SUM(COALESCE(p.Score, 0)) AS TotalScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation HAVING COUNT(DISTINCT p.Id) > 0),
// TopUsers AS (SELECT us.UserId, us.DisplayName, us.Reputation, us.PostCount, us.TotalViews, us.TotalScore, RANK() OVER (ORDER BY us.Reputation DESC) AS Rank FROM UserStats us)
// SELECT tu.DisplayName, tu.Reputation, tu.PostCount, tu.TotalViews, tu.TotalScore, COALESCE(rp.Title, 'No Posts Yet') AS RecentPostTitle
// FROM TopUsers tu LEFT JOIN RankedPosts rp ON tu.UserId = rp.OwnerUserId AND rp.rn = 1 WHERE tu.Reputation > (SELECT AVG(Reputation) FROM Users) ORDER BY tu.Rank LIMIT 10;
//
// `Reputation > AVG(Reputation)` is compared exactly, as `Reputation * COUNT > SUM`.
fn q3207(db: &'static So) -> String {
    let Post { creation_date, owner_user, view_count, score, .. } = &db.post;
    let us = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(view_count.opt().and(score))).fold([0i64; 3], |a, (w, s)| [a[0] + 1, a[1] + w.unwrap_or(0), a[2] + s]);
    let (sum, n) = db.user.select(&db.user.reputation).fold_flat((0i64, 0i64), |(s, n), r| (s + r, n + 1));
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (Reverse(d), p), asc);
    let last: HashIdx<Id<User>, Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let v = drain(db.user.with((&db.user.reputation).filt(move |r| r * n > sum)).select((&us).and((&last).opt())));
    let v = top_n(v, |&(u, _)| (Reverse(db.user.reputation.get(u).unwrap()), u), 10);
    rows(v.into_iter().map(|(u, (a, p))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::S(p.and_then(|p| db.post.title.get(p)).unwrap_or("No Posts Yet"))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS OwnerDisplayName, COUNT(DISTINCT c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        DENSE_RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS OwnerPostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 YEAR' GROUP BY p.Id, p.Title, p.CreationDate, u.DisplayName, p.OwnerUserId),
// FilteredPosts AS (SELECT PostId, Title, CreationDate, OwnerDisplayName, CommentCount, UpVotes, DownVotes FROM RankedPosts WHERE OwnerPostRank = 1)
// SELECT fp.PostId, fp.Title, fp.CreationDate, fp.OwnerDisplayName, fp.CommentCount, fp.UpVotes, fp.DownVotes, CASE WHEN fp.UpVotes > fp.DownVotes THEN 'Popular' ELSE 'Less Popular' END AS PopularityStatus
// FROM FilteredPosts fp ORDER BY fp.UpVotes DESC, fp.CommentCount DESC FETCH FIRST 10 ROWS ONLY;
//
// OwnerPostRank reads only base columns, so each owner's newest posts are picked first and the comment x vote product is driven for those alone.
fn q7204(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(creation_date))
        .window(dense_rank, |(_, d)| Reverse(d), asc);
    let fp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let s = (&fp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = (&fp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = top_n(drain((&s).and(&cc)), |&(p, (a, c))| (Reverse(a[0]), Reverse(c), p), 10);
    rows(v.into_iter().map(|(p, (a, c))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1]), V::S(if a[0] > a[1] { "Popular" } else { "Less Popular" })]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN p.PostTypeId = 1 AND p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedQuestionCount, SUM(CASE WHEN p.PostTypeId = 1 THEN p.Score ELSE 0 END) AS TotalQuestionScore,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN p.Score ELSE 0 END) AS TotalAnswerScore FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId WHERE u.Reputation > 0 GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, AnswerCount, AcceptedQuestionCount, TotalQuestionScore, TotalAnswerScore,
//        ROW_NUMBER() OVER (ORDER BY PostCount DESC, TotalQuestionScore DESC) AS Rank FROM UserPostStats)
// SELECT u.UserId, u.DisplayName, u.PostCount, u.AnswerCount, u.AcceptedQuestionCount, u.TotalQuestionScore, u.TotalAnswerScore, COALESCE(badge_count.BadgeCount, 0) AS BadgeCount
// FROM TopUsers u LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId) badge_count ON u.UserId = badge_count.UserId WHERE u.Rank <= 10 ORDER BY u.Rank;
fn q6997(db: &'static So) -> String {
    let Post { post_type_id, accepted_answer_id, score, .. } = &db.post;
    let s = db
        .user
        .with((&db.user.reputation).gt(0))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(accepted_answer_id.opt()).and(score)).opt())
        .fold([0i64; 5], |a, p| match p {
            Some(((t, acc), s)) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 1 && acc.is_some()) as i64, a[3] + if t == 1 { s } else { 0 }, a[4] + if t == 2 { s } else { 0 }],
            None => a,
        });
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let v = top_n(drain((&s).and((&bc).opt())), |&(u, (a, _))| (Reverse(a[0]), Reverse(a[3]), u), 10);
    rows(v.into_iter().map(|(u, (a, b))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        f.push(V::I(b.unwrap_or(0)));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, u.DisplayName AS OwnerDisplayName, COALESCE(v.UpVotes, 0) AS UpVotes, COALESCE(v.DownVotes, 0) AS DownVotes,
//        COALESCE(c.CommentCount, 0) AS CommentCount, COALESCE(a.AnswerCount, 0) AS AnswerCount, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id
//     LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes GROUP BY PostId) v ON p.Id = v.PostId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
//     LEFT JOIN (SELECT ParentId, COUNT(*) AS AnswerCount FROM Posts WHERE PostTypeId = 2 GROUP BY ParentId) a ON p.Id = a.ParentId
//     WHERE p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year')
// SELECT rp.PostId, rp.Title, rp.OwnerDisplayName, rp.UpVotes, rp.DownVotes, rp.CommentCount, rp.AnswerCount FROM RankedPosts rp WHERE rp.PostRank <= 10 ORDER BY rp.CreationDate DESC;
fn q29644(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(creation_date))
        .window(rank, |(_, d)| Reverse(d), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|((p, _), _)| p).collect();
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ac = (&tp).group_by(Ident::<Post>::new()).select(answers_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    rows(drain((&vc).and(&cc).and(&ac)).into_iter().map(|(p, ((a, c), n))| {
        let mut f = post_fields(db, p, &["id", "title", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c), V::I(n)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank,
//        COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, u.DisplayName, COALESCE(b.Count, 0) AS BadgeCount FROM Users u LEFT JOIN (SELECT UserId, COUNT(*) AS Count FROM Badges GROUP BY UserId) b ON u.Id = b.UserId)
// SELECT up.DisplayName, up.Reputation, up.BadgeCount, rp.Title, rp.CreationDate, rp.Upvotes, rp.Downvotes, COALESCE(rp.CommentCount, 0) AS CommentCount
// FROM UserReputation up JOIN RankedPosts rp ON up.UserId = rp.OwnerUserId WHERE rp.UserPostRank <= 5 ORDER BY up.Reputation DESC, rp.CreationDate DESC LIMIT 10;
//
// UserPostRank reads only base columns, so each owner's five newest posts are picked first and the comment x vote product is driven for those alone.
fn q252(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&s).and(owner_user.select(Ident::<User>::new().and((&bc).opt()))));
    let v = top_n(v, |&(p, (_, (u, _)))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10);
    rows(v.into_iter().map(|(p, (a, (u, b)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(b.unwrap_or(0)));
        f.extend(post_fields(db, p, &["title", "created"]));
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[0])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn,
//        COALESCE(c.Score, 0) AS CommentScore, COALESCE(badgeCount.BadgeCount, 0) AS BadgeCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId) badgeCount ON p.OwnerUserId = badgeCount.UserId
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// FilteredPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, CommentScore, BadgeCount FROM RankedPosts WHERE rn = 1 AND Score > 10 AND BadgeCount > 2)
// SELECT fp.Title, fp.CreationDate, fp.Score, fp.ViewCount, CASE WHEN fp.CommentScore > 5 THEN 'Highly Commented' ELSE 'Less Commented' END AS CommentStatus,
//        (SELECT COUNT(*) FROM Votes v WHERE v.PostId = fp.PostId AND v.VoteTypeId = 2) AS UpvoteCount
// FROM FilteredPosts fp ORDER BY fp.Score DESC, fp.ViewCount DESC;
//
// rn numbers the joined (post, comment) rows; the comments of the newest post tie, and are taken in id order (the ones reached here all score 0).
fn q2719(db: &'static So) -> String {
    let Post { creation_date, owner_user_id, score, .. } = &db.post;
    type R = (Id<Post>, Option<Id<Comment>>);
    let j: MatSet<R> = db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(Ident::<Post>::new().and(comments_of(db).opt())).collect();
    let p_of = || Same::<R>::new().map(|(p, _): R| p);
    let w = (&j).group_by(p_of().select(owner_user_id.opt())).select(Same::<R>::new().and(p_of().select(creation_date))).window(row_number, |((p, c), d)| (Reverse(d), p, c), asc);
    let fr: MatSet<R> = (&w).filt(|(_, r)| r == 1).map(|((x, _), _)| x).collect();
    let bc = db.badge.group_by(&db.badge.user_id).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let up = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2)));
    let uc = db.post.group_by(Ident::<Post>::new()).select(up.opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let v = drain(
        (&fr)
            .select(Same::<R>::new().and(p_of().select(score.gt(10))).and(p_of().select(owner_user_id.select((&bc).filt(|n| n > 2)))).and(p_of().select(&uc))),
    );
    rows(v.into_iter().map(|(_, ((((p, c), _), _), n))| {
        let cs = c.map_or(0, |c| db.comment.score.get(c).unwrap());
        let mut f = post_fields(db, p, &["title", "created", "score", "views"]);
        f.extend([V::S(if cs > 5 { "Highly Commented" } else { "Less Commented" }), V::I(n)]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, ROW_NUMBER() OVER (ORDER BY u.Reputation DESC) AS Ranking FROM Users u),
// PostStats AS (SELECT p.Id AS PostId, p.OwnerUserId, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) - SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS NetVotes,
//        COUNT(c.Id) AS CommentCount, COUNT(DISTINCT ph.Id) AS HistoryCount
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId GROUP BY p.Id, p.OwnerUserId),
// UserActivity AS (SELECT ur.UserId, COUNT(ps.PostId) AS TotalPosts, SUM(ps.CommentCount) AS TotalComments, AVG(ps.NetVotes) AS AverageNetVotes, MAX(ps.NetVotes) AS HighestNetVotes
//     FROM UserReputation ur JOIN PostStats ps ON ur.UserId = ps.OwnerUserId GROUP BY ur.UserId)
// SELECT ua.UserId, ua.TotalPosts, ua.TotalComments, ua.AverageNetVotes, ua.HighestNetVotes, ur.Ranking FROM UserActivity ua JOIN UserReputation ur ON ua.UserId = ur.UserId
// WHERE ua.TotalPosts > 0 ORDER BY ur.Ranking ASC, ua.TotalComments DESC FETCH FIRST 10 ROWS ONLY;
//
// Ranking reads only Reputation and is unique, so the ten best-ranked users with a post are picked first and the vote x comment x history product is driven for their posts alone.
fn q3354(db: &'static So) -> String {
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&db.user.reputation)).window(row_number, |(u, r)| (Reverse(r), u), asc);
    let with_post = drain((&w).map(|((u, _), r)| (u, r)).with(Same::<(Id<User>, i64)>::new().map(|(u, _)| u).select(posts_of(db))));
    let tu = rel(top_n(with_post, |&(_, (_, r))| r, 10).into_iter().map(|x| x.1).collect());
    let ps = (&tu)
        .map(|(u, _)| u)
        .select(posts_of(db))
        .group_by(Same::<Id<Post>>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()).and(history_of(db).opt()))
        .fold([0i64; 2], |a, ((t, c), _)| [a[0] + (t == Some(2)) as i64 - (t == Some(3)) as i64, a[1] + c.is_some() as i64]);
    let ua = (&tu).map(|(u, _)| u).group_by(Same::<Id<User>>::new()).select(posts_of(db).select(&ps)).fold([0, 0, 0, i64::MIN], |s, a| [s[0] + 1, s[1] + a[1], s[2] + a[0], s[3].max(a[0])]);
    let v = drain((&tu).select(Same::<(Id<User>, i64)>::new().and(Same::<(Id<User>, i64)>::new().map(|(u, _)| u).select(&ua))));
    rows(v.into_iter().map(|(_, ((u, r), a))| row(vec![user_col(db, u, "uid"), V::I(a[0]), V::I(a[1]), avg(a[2], a[0]), V::I(a[3]), V::I(r)])))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS QuestionCount, SUM(COALESCE(P.ViewCount, 0)) AS TotalViews, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(COALESCE(V.BountyAmount, 0)) AS TotalBounty FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId = 8
//     WHERE U.Reputation > 100 GROUP BY U.Id, U.DisplayName),
// RankedUsers AS (SELECT UserId, DisplayName, QuestionCount, TotalViews, AnswerCount, TotalBounty, RANK() OVER (ORDER BY TotalViews DESC, QuestionCount DESC) AS ViewRank FROM UserActivity),
// TopUsers AS (SELECT UserId, DisplayName, QuestionCount, TotalViews, AnswerCount, TotalBounty FROM RankedUsers WHERE ViewRank <= 10)
// SELECT U.UserId, U.DisplayName, U.QuestionCount, U.TotalViews, U.AnswerCount, COALESCE(B.BadgeCount, 0) AS BadgeCount, CASE WHEN U.AnswerCount > 5 THEN 'Answer Enthusiast' ELSE 'New Contributor' END AS UserCategory
// FROM TopUsers U LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId) B ON U.UserId = B.UserId ORDER BY U.TotalViews DESC;
fn q3427(db: &'static So) -> String {
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8)));
    let users = || db.user.with((&db.user.reputation).gt(100));
    let s = users()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.view_count).opt().and(&db.post.post_type_id).and(bounty.opt())).opt())
        .fold([0i64; 2], |a, p| match p {
            Some(((w, t), _)) => [a[0] + w.unwrap_or(0), a[1] + (t == 2) as i64],
            None => a,
        });
    let np = users().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let w = whole(&np).select(Ident::<User>::new().and((&np).and(&s).and((&bc).opt()))).window(rank, |(_, ((n, a), _))| (Reverse(a[0]), Reverse(n)), asc);
    let v = drain((&w).filt(|(_, r)| r <= 10));
    rows(v.into_iter().map(|(_, ((u, ((n, a), b)), _))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(b.unwrap_or(0)), V::S(if a[1] > 5 { "Answer Enthusiast" } else { "New Contributor" })]);
        row(f)
    }))
}

// WITH UserBadges AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// PopularPosts AS (SELECT P.Id, P.Title, P.OwnerUserId, P.ViewCount, P.CreationDate, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.ViewCount DESC) AS Rank
//     FROM Posts P WHERE P.PostTypeId = 1 AND P.ViewCount > 1000),
// ActiveUsers AS (SELECT U.Id, U.DisplayName, U.Reputation, U.LastAccessDate FROM Users U WHERE U.LastAccessDate >= CURRENT_TIMESTAMP - INTERVAL '30 days'),
// PostComments AS (SELECT C.PostId, COUNT(C.Id) AS CommentCount FROM Comments C GROUP BY C.PostId)
// SELECT AU.DisplayName AS ActiveUser, UB.BadgeCount, UB.GoldBadges, UB.SilverBadges, UB.BronzeBadges, PP.Title AS PopularPostTitle, PP.ViewCount, PP.CreationDate, COALESCE(PC.CommentCount, 0) AS TotalComments
// FROM ActiveUsers AU JOIN UserBadges UB ON AU.Id = UB.UserId JOIN PopularPosts PP ON AU.Id = PP.OwnerUserId LEFT JOIN PostComments PC ON PP.Id = PC.PostId
// WHERE UB.BadgeCount > 5 AND PP.Rank = 1 ORDER BY AU.Reputation DESC, PP.ViewCount DESC;
fn q7685(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let Post { post_type_id, view_count, owner_user, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1).and(view_count.gt(1000))).group_by(owner_user).select(Ident::<Post>::new().and(view_count)).window(row_number, |(p, w)| (Reverse(w), p), asc);
    let pp: HashIdx<Id<User>, Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let since = ny_to_utc(add_days(utc_to_ny(now_utc()), -30));
    let active = db.user.with((&db.user.last_access_date).filt(move |d| ny_to_utc(d) >= since));
    let pc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let v = drain(active.select((&ub).filt(|a| a[0] > 5).and((&pp).select(Ident::<Post>::new().and((&pc).opt())))));
    rows(v.into_iter().map(|(u, (a, (p, c)))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(a.map(V::I));
        f.extend(post_fields(db, p, &["title", "views", "created"]));
        f.push(V::I(c.unwrap_or(0)));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT B.Id) AS BadgeCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesCount,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesCount, SUM(CASE WHEN P.ViewCount IS NOT NULL THEN P.ViewCount ELSE 0 END) AS TotalViews,
//        COUNT(DISTINCT P.Id) AS TotalPosts, AVG(P.Score) AS AveragePostScore
//     FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, BadgeCount, UpVotesCount, DownVotesCount, TotalViews, TotalPosts, AveragePostScore,
//        RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank, RANK() OVER (ORDER BY UpVotesCount DESC) AS UpVotesRank FROM UserStats)
// SELECT UserId, DisplayName, Reputation, BadgeCount, UpVotesCount, DownVotesCount, TotalViews, TotalPosts, AveragePostScore, ReputationRank, UpVotesRank
// FROM TopUsers WHERE ReputationRank <= 10 OR UpVotesRank <= 10 ORDER BY ReputationRank, UpVotesRank;
fn q8777(db: &'static So) -> String {
    let Post { view_count, score, .. } = &db.post;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(posts_of(db).select(view_count.opt().and(score).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt()))
        .fold([0i64; 5], |a, (_, p)| match p {
            Some(((w, s), t)) => [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + w.unwrap_or(0), a[3] + 1, a[4] + s],
            None => a,
        });
    let nb = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let np = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    type R = ((Id<User>, (([i64; 5], i64), i64)), i64);
    let w = whole(&s).select(Ident::<User>::new().and((&s).and(&nb).and(&np)).and(&db.user.reputation)).window(rank, |(_, r): R| Reverse(r), asc);
    let w = (&w).window(rank, |(((_, ((a, _), _)), _), _): (R, i64)| Reverse(a[0]), asc);
    let mut v = drain((&w).filt(|((_, r), u)| r <= 10 || u <= 10));
    v.sort_by_key(|&(_, ((_, r), w))| (r, w));
    rows(v.into_iter().map(|(_, ((((u, ((a, b), n)), _), r), w))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(b), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(n), avg(a[4], a[3]), V::I(r), V::I(w)]);
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.CreationDate, u.DisplayName AS OwnerName, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn, p.OwnerUserId
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'),
// PostVoteSummary AS (SELECT PostId, COUNT(CASE WHEN VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN VoteTypeId = 3 THEN 1 END) AS DownVotes FROM Votes GROUP BY PostId),
// MostActiveUsers AS (SELECT u.Id, u.DisplayName, SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounty FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName
//     HAVING SUM(COALESCE(v.BountyAmount, 0)) > 0)
// SELECT rp.PostId, rp.Title, rp.ViewCount, rp.CreationDate, rp.OwnerName, COALESCE(pvs.UpVotes, 0) AS UpVotes, COALESCE(pvs.DownVotes, 0) AS DownVotes, mau.TotalBounty
// FROM RecentPosts rp LEFT JOIN PostVoteSummary pvs ON rp.PostId = pvs.PostId LEFT JOIN MostActiveUsers mau ON rp.OwnerUserId = mau.Id WHERE rp.rn = 1 ORDER BY rp.CreationDate DESC LIMIT 10;
fn q3490(db: &'static So) -> String {
    let Post { creation_date, owner_user, owner_user_id, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (Reverse(d), p), asc);
    let fp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let vs = (&fp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let mau = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()).fold(0i64, |n, b| n + b.flatten().unwrap_or(0));
    let v = top_n(drain((&vs).and(owner_user.select((&mau).filt(|n| n > 0)).opt())), |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 10);
    rows(v.into_iter().map(|(p, (a, b))| {
        let mut f = post_fields(db, p, &["id", "title", "views", "created", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), oint(b)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.CreationDate, COUNT(DISTINCT P.Id) AS TotalPosts, COUNT(DISTINCT CASE WHEN P.PostTypeId = 1 THEN P.Id END) AS TotalQuestions,
//        COUNT(DISTINCT CASE WHEN P.PostTypeId = 2 THEN P.Id END) AS TotalAnswers, SUM(COALESCE(P.ViewCount, 0)) AS TotalViews, SUM(COALESCE(P.Score, 0)) AS TotalScore,
//        SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation, U.CreationDate),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, TotalQuestions, TotalAnswers, TotalViews, TotalScore, TotalUpVotes, TotalDownVotes, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserStats)
// SELECT UserId, DisplayName, Reputation, TotalPosts, TotalQuestions, TotalAnswers, TotalViews, TotalScore, TotalUpVotes, TotalDownVotes, ReputationRank FROM TopUsers WHERE ReputationRank <= 10 ORDER BY ReputationRank;
//
// ReputationRank reads only Reputation, so the top users are picked first and the post x vote product is driven for those alone.
fn q8727(db: &'static So) -> String {
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r)| Reverse(r), asc);
    let tu: MatSet<(Id<User>, i64)> = (&w).filt(|(_, r)| r <= 10).map(|((u, _), r)| (u, r)).collect();
    let Post { post_type_id, view_count, score, .. } = &db.post;
    let s = (&tu)
        .map(|(u, _)| u)
        .group_by(Same::<Id<User>>::new())
        .select(posts_of(db).select(view_count.opt().and(score).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some(((w, s), t)) => [a[0] + w.unwrap_or(0), a[1] + s, a[2] + (t == Some(2)) as i64, a[3] + (t == Some(3)) as i64],
            None => a,
        });
    let d = (&tu).map(|(u, _)| u).group_by(Same::<Id<User>>::new()).select(posts_of(db).select(post_type_id).opt()).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64],
        None => a,
    });
    type R = (Id<User>, i64);
    let v = drain((&tu).select(Same::<R>::new().and(Same::<R>::new().map(|(u, _): R| u).select((&d).and(&s)))));
    rows(v.into_iter().map(|(_, ((u, r), (d, a)))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(d.map(V::I));
        f.extend(a.map(V::I));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, QuestionCount, AnswerCount, ROW_NUMBER() OVER (ORDER BY TotalPosts DESC) AS Rank FROM UserPostStats WHERE TotalPosts > 0),
// RecentVotes AS (SELECT V.UserId, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes V
//     WHERE V.CreationDate >= timestamp '2024-10-01 12:34:56' - INTERVAL '1 month' GROUP BY V.UserId)
// SELECT TU.DisplayName, TU.TotalPosts, TU.QuestionCount, TU.AnswerCount, COALESCE(RV.UpVotes, 0) AS MonthlyUpVotes, COALESCE(RV.DownVotes, 0) AS MonthlyDownVotes,
//        (COALESCE(RV.UpVotes, 0) - COALESCE(RV.DownVotes, 0)) AS NetVotes, CASE WHEN TU.QuestionCount > 0 THEN 'Active Contributor' ELSE 'Observer' END AS UserStatus
// FROM TopUsers TU LEFT JOIN RecentVotes RV ON TU.UserId = RV.UserId WHERE TU.Rank <= 10 ORDER BY TU.TotalPosts DESC;
fn q3642(db: &'static So) -> String {
    let ups = user_posts(db);
    let Vote { user, vote_type_id, creation_date, .. } = &db.vote;
    let rv = db.vote.with(creation_date.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(user).select(vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let v = top_n(drain((&ups).filt(|a| a[1] > 0).and((&rv).opt())), |&(u, (a, _))| (Reverse(a[1]), u), 10);
    rows(v.into_iter().map(|(u, (a, r))| {
        let r = r.unwrap_or([0, 0]);
        row(vec![user_col(db, u, "name"), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(r[0]), V::I(r[1]), V::I(r[0] - r[1]), V::S(if a[2] > 0 { "Active Contributor" } else { "Observer" })])
    }))
}

// WITH RecursiveTags AS (SELECT t.Id, t.TagName, t.Count, ROW_NUMBER() OVER (ORDER BY t.Count DESC) AS Rank FROM Tags t WHERE t.Count > 0),
// UserReputation AS (SELECT u.Id, u.DisplayName, u.Reputation, u.CreationDate, CASE WHEN u.Reputation >= 1000 THEN 'High' WHEN u.Reputation >= 100 THEN 'Medium' ELSE 'Low' END AS ReputationLevel FROM Users u),
// PostActivity AS (SELECT p.OwnerUserId, COUNT(DISTINCT c.Id) AS CommentsCount, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounties
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 8 WHERE p.CreationDate >= '2023-01-01' GROUP BY p.OwnerUserId),
// UserStats AS (SELECT u.Id, u.DisplayName, COALESCE(pa.CommentsCount, 0) AS CommentsCount, COALESCE(pa.TotalBounties, 0) AS TotalBounties FROM Users u LEFT JOIN PostActivity pa ON u.Id = pa.OwnerUserId)
// SELECT ur.DisplayName, ur.Reputation, ur.ReputationLevel, ut.CommentsCount, ut.TotalBounties, rt.TagName
// FROM UserReputation ur JOIN UserStats ut ON ur.Id = ut.Id LEFT JOIN RecursiveTags rt ON rt.Rank <= 10 WHERE ur.Reputation > 500 ORDER BY ur.Reputation DESC, ut.TotalBounties DESC LIMIT 50;
//
// The name says recursive but nothing recurses. The ON clause names only rt, so the users are crossed with the top ten tags.
fn q31871(db: &'static So) -> String {
    let rt = top_n(drain(db.tag.with((&db.tag.count).gt(0)).select(&db.tag.count)), |&(t, n)| (Reverse(n), t), 10);
    let rt = rel(rt);
    let Post { creation_date, owner_user, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(ts(2023, 1, 1, 0, 0, 0)));
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let pb = recent().group_by(owner_user).select(comments_of(db).opt().and(bounty.opt())).fold(0i64, |n, (_, b)| n + b.flatten().unwrap_or(0));
    let pc = recent().group_by(owner_user).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let users: MatSet<Id<User>> = db.user.with((&db.user.reputation).gt(500)).collect();
    let joined: MatSet<(Id<User>, Id<Tag>)> = (&users).cross(&rt).map(|(u, (t, _))| (u, t)).collect();
    let by_user = by_first(&joined);
    let v = drain((&users).select((&pc).opt().and((&pb).opt()).and((&by_user).opt())));
    let v = top_n(v, |&(u, ((_, b), t))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(b.unwrap_or(0)), u, t), 50);
    rows(v.into_iter().map(|(u, ((c, b), t))| {
        let r = db.user.reputation.get(u).unwrap();
        row(vec![user_col(db, u, "name"), V::I(r), V::S(if r >= 1000 { "High" } else if r >= 100 { "Medium" } else { "Low" }), V::I(c.unwrap_or(0)), V::I(b.unwrap_or(0)), t.map_or(V::Null, |t| V::S(db.tag.tag_name.get(t).unwrap()))])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName),
// PostHistoryStats AS (SELECT ph.PostId, COUNT(ph.Id) AS TotalEdits, MAX(ph.CreationDate) AS LastEditDate FROM PostHistory ph GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, ua.TotalPosts, ua.UpVotes, ua.DownVotes, phs.TotalEdits, phs.LastEditDate
// FROM RankedPosts rp JOIN UserActivity ua ON rp.PostId = ua.UserId JOIN PostHistoryStats phs ON rp.PostId = phs.PostId WHERE rp.Rank <= 10 ORDER BY rp.Score DESC, rp.ViewCount DESC;
//
// `rp.PostId = ua.UserId` joins a post id to a user id, so it goes through the raw ids.
fn q5170(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(row_number, |((p, s), w)| (Reverse(s), w.is_none(), Reverse(w), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|(((p, _), _), _)| p).collect();
    let users = || db.user.with((&db.user.reputation).gt(1000));
    let ua = users()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt())
        .fold([0i64; 2], |a, t| [a[0] + (t.flatten() == Some(2)) as i64, a[1] + (t.flatten() == Some(3)) as i64]);
    let np = users().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let by_orig: HashIdx<i64, Id<User>> = users().select(&db.user.origid).inv().collect();
    let PostHistory { post, creation_date: hd, .. } = &db.post_history;
    let phs = db.post_history.group_by(post).select(hd).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let v = drain((&tp).select((&db.post.origid).select(&by_orig).select((&np).and(&ua)).and(&phs)));
    rows(v.into_iter().map(|(p, ((n, a), (e, d)))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(e), V::T(d)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN p.PostTypeId = 3 THEN 1 ELSE 0 END) AS WikiCount, AVG(v.BountyAmount) AS AvgBounty,
//        COUNT(DISTINCT b.Id) AS BadgeCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (8, 9) LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// RankedUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, WikiCount, AvgBounty, BadgeCount,
//        RANK() OVER (ORDER BY Reputation DESC, PostCount DESC) AS ReputationRank FROM UserStats)
// SELECT ru.DisplayName, ru.Reputation, ru.PostCount, ru.QuestionCount, ru.AnswerCount, ru.WikiCount, COALESCE(ru.AvgBounty, 0) AS AvgBounty, ru.BadgeCount, pt.Name AS PostType
// FROM RankedUsers ru JOIN PostTypes pt ON ru.PostCount > 0 WHERE ru.ReputationRank <= 10 ORDER BY ru.Reputation DESC, ru.PostCount DESC;
//
// ReputationRank reads Reputation and the distinct post count, so the top users are picked first and the post x vote x badge product is driven for those alone.
// The ON clause names only ru, so the users with a post are crossed with every post type.
fn q6190(db: &'static So) -> String {
    let np = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let w = whole(&np).select(Ident::<User>::new().and(&np).and(&db.user.reputation)).window(rank, |((_, n), r)| (Reverse(r), Reverse(n)), asc);
    let tu: MatSet<Id<User>> = (&w).filt(|(_, r)| r <= 10).map(|(((u, _), _), _)| u).collect();
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt());
    let s = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(bounty.opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 5], |a, (p, _)| match p {
            Some((t, b)) => {
                let b = b.flatten();
                [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64, a[3] + b.is_some() as i64, a[4] + b.unwrap_or(0)]
            }
            None => a,
        });
    let nb = (&tu).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let ru: HashIdx<Id<User>, ((i64, [i64; 5]), i64)> = (&tu).select((&np).filt(|n| n > 0).and(&s).and(&nb)).collect();
    let pts: MatSet<Id<PostType>> = db.post_type.select(Ident::<PostType>::new()).collect();
    let v = drain((&ru).cross(&pts));
    rows(v.into_iter().map(|((u, _), (((n, a), b), t))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), if a[3] == 0 { V::F(0.0) } else { avg(a[4], a[3]) }, V::I(b), tname(db, t)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days'),
// UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(p.Score) AS TotalScore, SUM(p.ViewCount) AS TotalViews
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// PostHistorySummary AS (SELECT ph.PostId, COUNT(*) AS EditCount FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (4, 5, 6, 24) GROUP BY ph.PostId)
// SELECT up.UserId, up.DisplayName, up.Reputation, up.TotalPosts, up.TotalScore, up.TotalViews, rp.Title, rp.CreationDate, rp.Score AS PostScore, rp.ViewCount AS PostViewCount,
//        COALESCE(ph.EditCount, 0) AS TotalEdits
// FROM UserStatistics up JOIN RankedPosts rp ON up.UserId = rp.OwnerUserId AND rp.rn = 1 LEFT JOIN PostHistorySummary ph ON rp.PostId = ph.PostId ORDER BY up.TotalScore DESC, up.Reputation DESC;
fn q9711(db: &'static So) -> String {
    let ups = user_posts(db);
    let Post { creation_date, owner_user, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_days(date(2024, 10, 1), -30))).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let fp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let PostHistory { post, post_history_type_id, .. } = &db.post_history;
    let ec = db.post_history.with(post_history_type_id.in_v(vec![4, 5, 6, 24])).group_by(post).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    rows(drain((&fp).select(owner_user.select(Ident::<User>::new().and(&ups)).and((&ec).opt()))).into_iter().map(|(p, ((u, a), e))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[1]), nullable(a[4], a[1]), nullable(a[6], a[5])]);
        f.extend(post_fields(db, p, &["title", "created", "score", "views"]));
        f.push(V::I(e.unwrap_or(0)));
        row(f)
    }))
}

// Rewritten (rewrites/3088.sql): the RankedPosts window tie-broken on p.Id.
// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.ViewCount DESC, p.Id) AS Rank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'),
// RecentVotes AS (SELECT v.PostId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes FROM Votes v
//     WHERE v.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 month' GROUP BY v.PostId),
// PostWithRating AS (SELECT rp.PostId, rp.Title, rp.ViewCount, rv.UpVotes, rv.DownVotes, COALESCE(rv.UpVotes - rv.DownVotes, 0) AS Score, rp.Rank FROM RankedPosts rp LEFT JOIN RecentVotes rv ON rp.PostId = rv.PostId)
// SELECT pwr.PostId, pwr.Title, pwr.ViewCount, pwr.UpVotes, pwr.DownVotes, pwr.Score, CASE WHEN pwr.Score > 0 THEN 'Positive' WHEN pwr.Score < 0 THEN 'Negative' ELSE 'Neutral' END AS VoteStatus,
//        CASE WHEN pwr.Rank <= 5 THEN 'Top Post' ELSE 'Regular Post' END AS PostCategory
// FROM PostWithRating pwr WHERE pwr.Score <> 0 ORDER BY pwr.Score DESC, pwr.ViewCount DESC;
fn q3088(db: &'static So) -> String {
    let Post { post_type_id, creation_date, view_count, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(date(2024, 10, 1), -1)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(view_count.opt()))
        .window(row_number, |(p, w)| (w.is_none(), Reverse(w), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let rv = db.vote.with((&db.vote.creation_date).ge(add_months(date(2024, 10, 1), -1))).group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let v = drain(db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1))).select((&rv).filt(|a| a[0] != a[1]).and(Ident::<Post>::new().with(&tp).opt())));
    rows(v.into_iter().map(|(p, (a, t))| {
        let s = a[0] - a[1];
        let mut f = post_fields(db, p, &["id", "title", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(s), V::S(if s > 0 { "Positive" } else { "Negative" }), V::S(if t.is_some() { "Top Post" } else { "Regular Post" })]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(CASE WHEN C.Id IS NOT NULL THEN 1 ELSE 0 END) AS CommentCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, SUM(CASE WHEN B.Id IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId AND V.UserId = U.Id LEFT JOIN Badges B ON U.Id = B.UserId
//     GROUP BY U.Id, U.DisplayName),
// UserRanked AS (SELECT UserId, DisplayName, PostCount, Questions, Answers, CommentCount, UpVotes, DownVotes, BadgeCount, DENSE_RANK() OVER (ORDER BY PostCount DESC) AS PostRank,
//        DENSE_RANK() OVER (ORDER BY UpVotes DESC) AS UpVoteRank FROM UserActivity)
// SELECT DisplayName, PostCount, Questions, Answers, CommentCount, UpVotes, DownVotes, BadgeCount, PostRank, UpVoteRank FROM UserRanked WHERE PostCount > 5 ORDER BY PostRank, UpVotes DESC FETCH FIRST 10 ROWS ONLY;
//
// `V.UserId = U.Id` with U the post's owner: the owner's own votes on the post (`own_votes`).
fn q5558(db: &'static So) -> String {
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(comments_of(db).opt()).and(own_votes(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 6], |a, (p, b)| {
            let ((t, c), v) = p.unwrap_or(((0, None), None));
            [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + c.is_some() as i64, a[3] + (v == Some(2)) as i64, a[4] + (v == Some(3)) as i64, a[5] + b.is_some() as i64]
        });
    let np = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    type R = (Id<User>, (i64, [i64; 6]));
    let w = whole(&np).select(Ident::<User>::new().and((&np).and(&s))).window(dense_rank, |(_, (n, _)): R| Reverse(n), asc);
    let w = (&w).window(dense_rank, |((_, (_, a)), _): (R, i64)| Reverse(a[3]), asc);
    let v = drain((&w).filt(|(((_, (n, _)), _), _)| n > 5));
    let v = top_n(v, |&(_, (((u, (_, a)), r), _))| (r, Reverse(a[3]), u), 10);
    rows(v.into_iter().map(|(_, (((u, (n, a)), r), w))| {
        let mut f = vec![user_col(db, u, "name"), V::I(n)];
        f.extend(a.map(V::I));
        f.extend([V::I(r), V::I(w)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT a.Id) AS AnswerCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS PostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2 LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.PostTypeId),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.CommentCount, rp.AnswerCount, rp.UpVoteCount, rp.DownVoteCount FROM RankedPosts rp WHERE rp.PostRank <= 5)
// SELECT u.DisplayName, u.Reputation, tp.Title, tp.CreationDate, tp.Score, tp.CommentCount, tp.AnswerCount, tp.UpVoteCount, tp.DownVoteCount
// FROM TopPosts tp JOIN Users u ON tp.PostId = u.Id ORDER BY tp.Score DESC, tp.CreationDate DESC;
//
// `tp.PostId = u.Id` joins a post id to a user id, so it goes through the raw ids. PostRank reads only base columns, so the top posts are picked first.
fn q8323(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|(((p, _), _), _)| p).collect();
    let vs = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(answers_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ac = (&tp).group_by(Ident::<Post>::new()).select(answers_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let by_orig: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    rows(drain((&vs).and(&cc).and(&ac).and((&db.post.origid).select(&by_orig))).into_iter().map(|(p, (((a, c), n), u))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["title", "created", "score"]));
        f.extend([V::I(c), V::I(n), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, u.DisplayName AS Author, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS Rank,
//        (SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id) AS CommentCount FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1),
// ClosedPosts AS (SELECT ph.PostId, ph.CreationDate, ph.UserDisplayName, ph.Comment AS CloseReason FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11)),
// TopQuestions AS (SELECT rp.PostId, rp.Title, rp.Author, rp.CreationDate, rp.Score, COALESCE(cp.CloseReason, 'Open') AS Status, rp.CommentCount
//     FROM RankedPosts rp LEFT JOIN ClosedPosts cp ON rp.PostId = cp.PostId WHERE rp.Rank = 1)
// SELECT tq.PostId, tq.Title, tq.Author, tq.CreationDate, tq.Score, tq.Status, tq.CommentCount,
//        CASE WHEN tq.Score IS NULL THEN 'No Score' WHEN tq.Score > 100 THEN 'High Score' ELSE 'Moderate Score' END AS ScoreCategory
// FROM TopQuestions tq WHERE tq.CommentCount > 0 OR tq.Status = 'Closed' ORDER BY tq.Score DESC, tq.CreationDate ASC LIMIT 50;
fn q2530(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, creation_date, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1)).group_by(owner_user).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let fp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let cc = (&fp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let PostHistory { post_history_type_id, comment, .. } = &db.post_history;
    let closes = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.in_v(vec![10, 11]))).select(Ident::<PostHistory>::new().and(comment.opt()));
    let v = drain((&cc).and(closes.opt()).filt(|(c, h)| c > 0 || h.and_then(|(_, s)| s).unwrap_or("Open") == "Closed"));
    let v = top_n(v, |&(p, (_, h))| (Reverse(score.get(p).unwrap()), creation_date.get(p).unwrap(), p, h.map(|x| x.0)), 50);
    rows(v.into_iter().map(|(p, (c, h))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "score"]);
        f.extend([V::S(h.and_then(|(_, s)| s).unwrap_or("Open")), V::I(c), V::S(if s > 100 { "High Score" } else { "Moderate Score" })]);
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.OwnerUserId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, COALESCE(c.CommentCount, 0) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
//     WHERE p.CreationDate > (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days'))
// SELECT r.OwnerUserId, u.DisplayName, r.PostId, r.Title, r.CreationDate, r.Score, r.ViewCount, r.AnswerCount, r.CommentCount,
//        (SELECT COUNT(*) FROM Votes v WHERE v.PostId = r.PostId AND v.VoteTypeId = 2) AS UpVotes, (SELECT COUNT(*) FROM Votes v WHERE v.PostId = r.PostId AND v.VoteTypeId = 3) AS DownVotes,
//        (SELECT COUNT(*) FROM PostHistory ph WHERE ph.PostId = r.PostId AND ph.PostHistoryTypeId = 10) AS CloseVotes,
//        CASE WHEN r.Score > 10 THEN 'Highly Rated' WHEN r.Score BETWEEN 1 AND 10 THEN 'Moderately Rated' ELSE 'Low Rated' END AS RatingCategory
// FROM RecentPosts r JOIN Users u ON r.OwnerUserId = u.Id WHERE r.rn = 1 ORDER BY r.CreationDate DESC FETCH FIRST 50 ROWS ONLY;
fn q3906(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let w = db.post.with(creation_date.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let fp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let cc = (&fp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = (&fp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10)));
    let hc = (&fp).group_by(Ident::<Post>::new()).select(closes.opt()).fold(0i64, |n, h| n + h.is_some() as i64);
    let v = top_n(drain((&cc).and(&vc).and(&hc)), |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 50);
    rows(v.into_iter().map(|(p, ((c, a), h))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["owner_id", "owner", "id", "title", "created", "score", "views", "answers"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1]), V::I(h), V::S(if s > 10 { "Highly Rated" } else if s >= 1 { "Moderately Rated" } else { "Low Rated" })]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, COUNT(DISTINCT c.Id) AS TotalComments, SUM(COALESCE(v.VoteCount, 0)) AS TotalVotes,
//        SUM(COALESCE(b.Class, 0)) AS TotalBadges, COALESCE(MAX(p.CreationDate), '1900-01-01') AS LatestPostDate
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId AND p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
//     LEFT JOIN Comments c ON u.Id = c.UserId AND c.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
//     LEFT JOIN (SELECT postId, COUNT(*) AS VoteCount FROM Votes GROUP BY postId) v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// RankedUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalComments, TotalVotes, TotalBadges, LatestPostDate, ROW_NUMBER() OVER (ORDER BY TotalPosts DESC, TotalVotes DESC) AS Rank FROM UserActivity)
// SELECT ru.DisplayName, ru.TotalPosts, ru.TotalComments, ru.TotalVotes, ru.TotalBadges, ru.LatestPostDate FROM RankedUsers ru WHERE ru.Rank <= 10 ORDER BY ru.Rank;
//
// Rank leads with the distinct post count, so the users tied with or above the tenth are picked first and the post x comment x badge product is driven for those alone.
fn q9415(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let Post { creation_date, .. } = &db.post;
    let rp = posts_of(db).select(Ident::<Post>::new().with(creation_date.gt(add_years(t0, -1))));
    let rc = comments_by(db).select(Ident::<Comment>::new().with((&db.comment.creation_date).gt(add_years(t0, -1))));
    let np = db.user.group_by(Ident::<User>::new()).select(rp.opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let w = whole(&np).select(Ident::<User>::new().and(&np)).window(rank, |(_, n)| Reverse(n), asc);
    let cand: MatSet<Id<User>> = (&w).filt(|(_, r)| r <= 10).map(|((u, _), _)| u).collect();
    let vc = db.vote.group_by(&db.vote.post).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let rp = posts_of(db).select(Ident::<Post>::new().with(creation_date.gt(add_years(t0, -1))));
    let s = (&cand)
        .group_by(Ident::<User>::new())
        .select(rp.select(creation_date.and((&vc).opt())).opt().and(rc.opt()).and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64, 0, i64::MIN], |a, ((p, _), b)| {
            let (d, n) = p.map_or((i64::MIN, None), |x| x);
            [a[0] + n.unwrap_or(0), a[1] + b.unwrap_or(0), a[2].max(d)]
        });
    let rc = comments_by(db).select(Ident::<Comment>::new().with((&db.comment.creation_date).gt(add_years(t0, -1))));
    let nc = (&cand).group_by(Ident::<User>::new()).select(rc.opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = top_n(drain((&cand).select((&np).and(&s).and(&nc))), |&(u, ((n, a), _))| (Reverse(n), Reverse(a[0]), u), 10);
    rows(v.into_iter().map(|(u, ((n, a), c))| row(vec![user_col(db, u, "name"), V::I(n), V::I(c), V::I(a[0]), V::I(a[1]), V::T(if a[2] == i64::MIN { ts(1900, 1, 1, 0, 0, 0) } else { a[2] })])))
}

// WITH UserBadgeCounts AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount, MAX(b.Class) AS HighestBadgeClass FROM Badges b GROUP BY b.UserId),
// RecentPosts AS (SELECT p.Id AS PostId, p.OwnerUserId, p.Title, p.CreationDate, COUNT(c.Id) AS CommentCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= (TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '7 days')
//     GROUP BY p.Id, p.OwnerUserId, p.Title, p.CreationDate),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, ub.BadgeCount, up.PostId, up.Title, up.CreationDate, up.CommentCount, up.UpVoteCount,
//        RANK() OVER (PARTITION BY ub.BadgeCount ORDER BY up.UpVoteCount DESC) AS UserRank
//     FROM Users u JOIN UserBadgeCounts ub ON u.Id = ub.UserId JOIN RecentPosts up ON u.Id = up.OwnerUserId)
// SELECT t.DisplayName, t.Title, t.CreationDate, t.CommentCount, t.UpVoteCount, CASE WHEN t.UserRank <= 3 THEN 'Top Contributor' ELSE 'Contributor' END AS ContributionLevel
// FROM TopUsers t WHERE t.BadgeCount > 0 ORDER BY t.BadgeCount DESC, t.UpVoteCount DESC LIMIT 10;
fn q394(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let rp = db
        .post
        .with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -7)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64]);
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let ob = || owner_user.select(&bc);
    let w = db.post.with(&rp).with(ob()).group_by(ob()).select(Ident::<Post>::new().and(&rp).and(owner_user)).window(rank, |((_, a), _)| Reverse(a[1]), asc);
    let v = top_n(drain(&w), |&(b, (((p, a), _), _))| (Reverse(b), Reverse(a[1]), p), 10);
    rows(v.into_iter().map(|(_, (((p, a), u), r))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title", "created"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::S(if r <= 3 { "Top Contributor" } else { "Contributor" })]);
        row(f)
    }))
}

// WITH UserEngagement AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COUNT(DISTINCT p.Id) AS TotalPosts, COUNT(DISTINCT c.Id) AS TotalComments, COUNT(DISTINCT b.Id) AS TotalBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.UserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Badges b ON u.Id = b.UserId
//     GROUP BY u.Id, u.DisplayName, u.Reputation),
// RankedUsers AS (SELECT ue.*, ROW_NUMBER() OVER (ORDER BY ue.Reputation DESC) AS Rank FROM UserEngagement ue),
// TopUsers AS (SELECT * FROM RankedUsers WHERE Rank <= 10)
// SELECT tu.UserId, tu.DisplayName, tu.Reputation, tu.UpVotes, tu.DownVotes, tu.TotalPosts, COALESCE(b.TotalBadges, 0) AS BadgeCount,
//        CASE WHEN tu.TotalPosts > 0 THEN ROUND((CAST(tu.UpVotes AS numeric) / NULLIF(tu.UpVotes + tu.DownVotes + tu.TotalPosts, 0)) * 100, 2) ELSE 0 END AS EngagementRate
// FROM TopUsers tu LEFT JOIN (SELECT UserId, COUNT(*) AS TotalBadges FROM Badges GROUP BY UserId) b ON tu.UserId = b.UserId ORDER BY tu.Reputation DESC;
//
// Rank reads only Reputation, so the ten users are picked first and the post x vote x comment x badge product is driven for those alone.
fn q3342(db: &'static So) -> String {
    let tu: MatSet<Id<User>> = rel(top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10).into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let s = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(own_votes(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 2], |a, (p, _)| {
            let t = p.and_then(|(t, _)| t);
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
        });
    let np = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    rows(drain((&s).and(&np).and((&bc).opt())).into_iter().map(|(u, ((a, n), b))| {
        let d = a[0] + a[1] + n;
        let rate = if n > 0 { V::F((a[0] as f64 / d as f64 * 100.0 * 100.0).round() / 100.0) } else { V::F(0.0) };
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(n), V::I(b.unwrap_or(0)), rate]);
        row(f)
    }))
}

// WITH UserBadges AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// PostStats AS (SELECT P.OwnerUserId, COUNT(P.Id) AS PostCount, SUM(P.Score) AS TotalScore, SUM(P.ViewCount) AS TotalViews FROM Posts P
//     WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY P.OwnerUserId),
// RankedUsers AS (SELECT UB.UserId, UB.DisplayName, PS.PostCount, PS.TotalScore, PS.TotalViews, RANK() OVER (ORDER BY PS.TotalScore DESC) AS ScoreRank FROM UserBadges UB JOIN PostStats PS ON UB.UserId = PS.OwnerUserId)
// SELECT RU.DisplayName, RU.PostCount, RU.TotalScore, RU.TotalViews, RU.ScoreRank, COALESCE(UB.GoldBadges, 0) AS GoldBadges, COALESCE(UB.SilverBadges, 0) AS SilverBadges, COALESCE(UB.BronzeBadges, 0) AS BronzeBadges
// FROM RankedUsers RU LEFT JOIN UserBadges UB ON RU.UserId = UB.UserId ORDER BY RU.ScoreRank;
fn q4961(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    let Post { creation_date, owner_user, score, view_count, .. } = &db.post;
    let ps = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(owner_user)
        .select(score.and(view_count.opt()))
        .fold([0i64; 4], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]);
    let w = whole(&ps).select(Ident::<User>::new().and((&ps).and(&ub))).window(rank, |(_, (a, _))| Reverse(a[1]), asc);
    rows(drain(&w).into_iter().map(|(_, ((u, (a, b)), r))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), nullable(a[3], a[2]), V::I(r)];
        f.extend(b.map(V::I));
        row(f)
    }))
}

// WITH UserBadgeCounts AS (SELECT u.Id AS UserId, COUNT(b.Id) FILTER (WHERE b.Class = 1) AS GoldBadges, COUNT(b.Id) FILTER (WHERE b.Class = 2) AS SilverBadges,
//        COUNT(b.Id) FILTER (WHERE b.Class = 3) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PopularPosts AS (SELECT p.Id, p.OwnerUserId, p.Title, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.ViewCount DESC) AS RN FROM Posts p WHERE p.PostTypeId = 1)
// SELECT u.DisplayName AS UserName, COALESCE(ubc.GoldBadges, 0) AS GoldBadges, COALESCE(ubc.SilverBadges, 0) AS SilverBadges, COALESCE(ubc.BronzeBadges, 0) AS BronzeBadges,
//        pp.Title AS MostPopularQuestion, pp.ViewCount AS MostPopularViewCount, CASE WHEN pp.ViewCount IS NULL THEN 'No Questions' ELSE 'Has Questions' END AS QuestionStatus,
//        (SELECT COUNT(*) FROM Votes v WHERE v.PostId = pp.Id AND v.VoteTypeId = 2) AS UpvoteCount, (SELECT COUNT(*) FROM Votes v WHERE v.PostId = pp.Id AND v.VoteTypeId = 3) AS DownvoteCount
// FROM Users u LEFT JOIN UserBadgeCounts ubc ON u.Id = ubc.UserId LEFT JOIN PopularPosts pp ON u.Id = pp.OwnerUserId AND pp.RN = 1 WHERE u.Reputation > 1000 ORDER BY u.Reputation DESC LIMIT 50;
fn q650(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    let Post { post_type_id, owner_user, view_count, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1)).group_by(owner_user).select(Ident::<Post>::new().and(view_count.opt())).window(row_number, |(p, w)| (w.is_none(), Reverse(w), p), asc);
    let pp: HashIdx<Id<User>, Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let vc = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = drain(db.user.with((&db.user.reputation).gt(1000)).select((&ub).and((&pp).select(Ident::<Post>::new().and(&vc)).opt())));
    let v = top_n(v, |&(u, _)| (Reverse(db.user.reputation.get(u).unwrap()), u), 50);
    rows(v.into_iter().map(|(u, (b, p))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(b.map(V::I));
        f.extend(match p {
            Some((p, a)) => {
                let w = view_count.get(p);
                vec![title(db, p), oint(w), V::S(if w.is_none() { "No Questions" } else { "Has Questions" }), V::I(a[0]), V::I(a[1])]
            }
            None => vec![V::Null, V::Null, V::S("No Questions"), V::I(0), V::I(0)],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Comments c ON c.PostId = p.Id LEFT JOIN Votes v ON v.PostId = p.Id WHERE p.CreationDate >= (DATE '2024-10-01' - INTERVAL '1 year')
//     GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.OwnerUserId),
// FilteredPosts AS (SELECT rp.Id, rp.Title, rp.CreationDate, rp.ViewCount, rp.CommentCount, rp.UpVotes, rp.DownVotes FROM RankedPosts rp WHERE rp.rn = 1 AND rp.ViewCount > 100 AND (rp.UpVotes - rp.DownVotes) > 10),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON b.UserId = u.Id GROUP BY u.Id)
// SELECT fp.Title, fp.ViewCount, fp.CommentCount, ub.BadgeCount, CASE WHEN ub.BadgeCount IS NULL THEN 'No Badges' WHEN ub.BadgeCount >= 10 THEN 'Veteran' ELSE 'Newbie' END AS UserStatus
// FROM FilteredPosts fp LEFT JOIN Users u ON u.Id = (SELECT OwnerUserId FROM Posts WHERE Id = fp.Id) LEFT JOIN UserBadges ub ON ub.UserId = u.Id ORDER BY fp.ViewCount DESC LIMIT 50;
//
// rn reads only base columns, so each owner's newest post is picked first and the comment x vote product is driven for those alone.
fn q2689(db: &'static So) -> String {
    let Post { creation_date, owner_user, owner_user_id, view_count, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1))).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let fp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let s = (&fp)
        .with(view_count.gt(100))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = drain((&s).filt(|a| a[1] - a[2] > 10).and(owner_user.select(&ub).opt()));
    let v = top_n(v, |&(p, _)| (Reverse(view_count.get(p)), p), 50);
    rows(v.into_iter().map(|(p, (a, b))| {
        let mut f = post_fields(db, p, &["title", "views"]);
        f.extend([V::I(a[0]), oint(b), V::S(match b {
            None => "No Badges",
            Some(b) if b >= 10 => "Veteran",
            _ => "Newbie",
        })]);
        row(f)
    }))
}

// Rewritten (rewrites/2878.sql): the final ORDER BY tie-broken on tp.PostId.
// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, RANK() OVER (ORDER BY u.Reputation DESC) AS ReputationRank FROM Users u WHERE u.Views > 100),
// TopPosts AS (SELECT rp.*, ur.Reputation, ur.ReputationRank FROM RankedPosts rp JOIN UserReputation ur ON rp.PostId IN (SELECT p.Id FROM Posts p WHERE p.OwnerUserId = ur.UserId) WHERE rp.PostRank <= 3)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.CommentCount, tp.Reputation, tp.ReputationRank, COALESCE(b.Name, 'No Badge') AS BadgeName
// FROM TopPosts tp LEFT JOIN Badges b ON tp.PostId = b.UserId AND b.Class = 1 WHERE tp.Reputation > 500 AND NOT tp.ReputationRank IS NULL
// ORDER BY tp.Score DESC, tp.Reputation DESC, tp.PostId LIMIT 10;
//
// `tp.PostId = b.UserId` joins a post id to a user id, so it goes through the raw ids.
fn q2878(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, origid, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1))).group_by(owner_user).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 3).map(|((p, _), _)| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let w = whole(db.user.with((&db.user.views).gt(100))).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r)| Reverse(r), asc);
    let ur: MatSet<(Id<User>, i64)> = (&w).map(|((u, _), r)| (u, r)).collect();
    let by_user = by_first(&ur);
    let gold: HashIdx<i64, Id<Badge>> = db.badge.with((&db.badge.class).eq(1)).select(&db.badge.user_id).inv().collect();
    let rich = Ident::<User>::new().with((&db.user.reputation).gt(500));
    let v = drain((&cc).and(owner_user.select(rich).select(Ident::<User>::new().and(&by_user))).and(origid.select(&gold).opt()));
    let v = top_n(v, |&(p, ((_, (u, _)), _))| (Reverse(score.get(p).unwrap()), Reverse(db.user.reputation.get(u).unwrap()), origid.get(p).unwrap()), 10);
    rows(v.into_iter().map(|(p, ((c, (u, r)), b))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(c), user_col(db, u, "rep"), V::I(r), V::S(b.map_or("No Badge", |b| db.badge.name.get(b).unwrap()))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.PostTypeId),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.CommentCount, rp.UpVoteCount, rp.DownVoteCount FROM RankedPosts rp WHERE rp.Rank <= 5)
// SELECT tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.CommentCount, tp.UpVoteCount, tp.DownVoteCount, u.DisplayName AS OwnerDisplayName, u.Reputation AS OwnerReputation, p.Tags
// FROM TopPosts tp JOIN Users u ON tp.PostId = u.Id JOIN Posts p ON tp.PostId = p.Id WHERE p.OwnerUserId IS NOT NULL ORDER BY tp.Score DESC, tp.ViewCount DESC;
//
// `tp.PostId = u.Id` joins a post id to a user id, so it goes through the raw ids. Rank reads only base columns, so the top posts are picked first.
fn q7776(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, owner_user_id, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(row_number, |((p, s), w)| (Reverse(s), w.is_none(), Reverse(w), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|(((p, _), _), _)| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let by_orig: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    rows(drain((&s).and(owner_user_id).and((&db.post.origid).select(&by_orig))).into_iter().map(|(p, ((a, _), u))| {
        let mut f = post_fields(db, p, &["title", "created", "score", "views"]);
        f.extend(a.map(V::I));
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend(post_fields(db, p, &["tags"]));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, COUNT(c.Id) AS CommentCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 YEAR' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.OwnerUserId),
// UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(b.Class, 0) AS BadgeClass, u.Reputation FROM Users u LEFT JOIN (SELECT UserId, MAX(Class) AS Class FROM Badges GROUP BY UserId) b ON u.Id = b.UserId),
// PostScores AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, ur.Reputation, ur.BadgeClass, rp.CommentCount FROM RankedPosts rp JOIN UserReputation ur ON rp.PostId = ur.UserId WHERE rp.UserPostRank <= 5)
// SELECT ps.Title, ps.CreationDate, ps.Score, ps.Reputation, ps.BadgeClass, COALESCE(CASE WHEN ps.CommentCount = 0 THEN 'No Comments' ELSE 'Has Comments' END, 'Unknown') AS CommentStatus
// FROM PostScores ps ORDER BY ps.Score DESC, ps.Reputation DESC LIMIT 20;
//
// `rp.PostId = ur.UserId` joins a post id to a user id, so it goes through the raw ids.
fn q2106(db: &'static So) -> String {
    let Post { creation_date, owner_user_id, score, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let mc = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold(i64::MIN, |m, c| m.max(c));
    let by_orig: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let v = drain((&cc).and((&db.post.origid).select(&by_orig).select(Ident::<User>::new().and((&mc).opt()))));
    let v = top_n(v, |&(p, (_, (u, _)))| (Reverse(score.get(p).unwrap()), Reverse(db.user.reputation.get(u).unwrap()), p), 20);
    rows(v.into_iter().map(|(p, (c, (u, m)))| {
        let mut f = post_fields(db, p, &["title", "created", "score"]);
        f.extend([user_col(db, u, "rep"), V::I(m.unwrap_or(0)), V::S(if c == 0 { "No Comments" } else { "Has Comments" })]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT Id, Reputation, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM Users),
// RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, COUNT(c.Id) AS CommentCount, SUM(v.BountyAmount) AS TotalBountyAmount,
//        MAX(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS HasUpvote, MAX(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS HasDownvote
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId),
// TopUsers AS (SELECT ur.Id AS UserId, ur.Reputation, ur.ReputationRank, COALESCE(SUM(CASE WHEN rp.HasUpvote = 1 THEN 1 ELSE 0 END), 0) AS UpvotedPosts,
//        COALESCE(SUM(CASE WHEN rp.HasDownvote = 1 THEN 1 ELSE 0 END), 0) AS DownvotedPosts
//     FROM UserReputation ur LEFT JOIN RecentPosts rp ON ur.Id = rp.OwnerUserId WHERE ur.Reputation > (SELECT AVG(Reputation) FROM Users) GROUP BY ur.Id, ur.Reputation, ur.ReputationRank)
// SELECT tu.UserId, tu.Reputation, tu.ReputationRank, tu.UpvotedPosts, tu.DownvotedPosts FROM TopUsers tu WHERE tu.ReputationRank <= 10 ORDER BY tu.Reputation DESC;
//
// ReputationRank reads only Reputation, so the ten users are picked first and their recent posts' comment x vote product is driven for those alone.
// `Reputation > AVG(Reputation)` is compared exactly, as `Reputation * COUNT > SUM`.
fn q4194(db: &'static So) -> String {
    let (sum, n) = db.user.select(&db.user.reputation).fold_flat((0i64, 0i64), |(s, n), r| (s + r, n + 1));
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&db.user.reputation)).window(row_number, |(u, r)| (Reverse(r), u), asc);
    let tu: MatSet<(Id<User>, i64)> = (&w).filt(|(_, k)| k <= 10).map(|((u, _), k)| (u, k)).collect();
    let rich: MatSet<Id<User>> = (&tu).map(|(u, _)| u).with((&db.user.reputation).filt(move |r| r * n > sum)).collect();
    let creation_date = &db.post.creation_date;
    let recent = posts_of(db).select(Ident::<Post>::new().with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))));
    let rp = (&rich)
        .select(recent)
        .group_by(Same::<Id<Post>>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (_, t)| [a[0].max((t == Some(2)) as i64), a[1].max((t == Some(3)) as i64)]);
    let recent = posts_of(db).select(Ident::<Post>::new().with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))));
    let s = (&rich).group_by(Ident::<User>::new()).select(recent.select(&rp).opt()).fold([0i64; 2], |a, h| match h {
        Some(h) => [a[0] + h[0], a[1] + h[1]],
        None => a,
    });
    type R = (Id<User>, i64);
    rows(drain((&tu).select(Same::<R>::new().and(Same::<R>::new().map(|(u, _): R| u).select(&s)))).into_iter().map(|(_, ((u, r), a))| {
        row(vec![user_col(db, u, "uid"), user_col(db, u, "rep"), V::I(r), V::I(a[0]), V::I(a[1])])
    }))
}

// Rewritten (rewrites/2004.sql): the final ORDER BY tie-broken on tp.PostId, UserBadge.
// WITH RankedPosts AS (SELECT p.Id AS PostId, p.OwnerUserId, p.Title, p.CreationDate, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY COUNT(c.Id) DESC) AS RankByComments
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
//     GROUP BY p.Id, p.OwnerUserId, p.Title, p.CreationDate),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.OwnerUserId, (rp.UpVotes - rp.DownVotes) AS NetVotes, rp.CommentCount FROM RankedPosts rp WHERE rp.RankByComments = 1)
// SELECT u.DisplayName, tp.Title, tp.NetVotes, tp.CommentCount, CASE WHEN tp.NetVotes > 100 THEN 'Hot Post' WHEN tp.NetVotes BETWEEN 50 AND 100 THEN 'Trending Post' ELSE 'Regular Post' END AS PostCategory,
//        COALESCE(b.Name, 'No Badge') AS UserBadge
// FROM TopPosts tp JOIN Users u ON tp.OwnerUserId = u.Id LEFT JOIN Badges b ON b.UserId = u.Id AND b.Class = 1 WHERE u.Reputation > 1000 ORDER BY tp.NetVotes DESC, tp.PostId, UserBadge LIMIT 10;
fn q2004(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let rp = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(owner_user)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64 - (t == Some(3)) as i64]);
    let w = db.post.with(&rp).group_by(owner_user).select(Ident::<Post>::new().and(&rp)).window(rank, |(_, a)| Reverse(a[0]), asc);
    let tp: MatSet<(Id<Post>, [i64; 2])> = (&w).filt(|(_, r)| r == 1).map(|(x, _)| x).collect();
    let gold = badges_of(db).select(Ident::<Badge>::new().with((&db.badge.class).eq(1))).select(&db.badge.name);
    let rich = Ident::<User>::new().with((&db.user.reputation).gt(1000));
    type R = (Id<Post>, [i64; 2]);
    let v = drain((&tp).select(Same::<R>::new().and(Same::<R>::new().map(|(p, _): R| p).select(owner_user.select(rich.and(gold.opt()))))));
    let v = top_n(v, |&(_, ((p, a), (_, b)))| (Reverse(a[1]), p, b.unwrap_or("No Badge")), 10);
    rows(v.into_iter().map(|(_, ((p, a), (u, b)))| {
        let mut f = vec![user_col(db, u, "name"), title(db, p)];
        f.extend([V::I(a[1]), V::I(a[0]), V::S(if a[1] > 100 { "Hot Post" } else if a[1] >= 50 { "Trending Post" } else { "Regular Post" }), V::S(b.unwrap_or("No Badge"))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// PostMetrics AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, rp.OwnerDisplayName, COALESCE(SUM(CASE WHEN vt.Id = 2 THEN 1 ELSE 0 END), 0) AS TotalUpvotes,
//        COALESCE(SUM(CASE WHEN vt.Id = 3 THEN 1 ELSE 0 END), 0) AS TotalDownvotes
//     FROM RankedPosts rp LEFT JOIN Votes v ON rp.PostId = v.PostId LEFT JOIN VoteTypes vt ON v.VoteTypeId = vt.Id WHERE rp.Rank <= 10
//     GROUP BY rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, rp.OwnerDisplayName)
// SELECT pm.PostId, pm.Title, pm.CreationDate, pm.Score, pm.ViewCount, pm.AnswerCount, pm.OwnerDisplayName, pm.TotalUpvotes, pm.TotalDownvotes FROM PostMetrics pm ORDER BY pm.Score DESC, pm.ViewCount DESC;
fn q8415(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, .. } = &db.post;
    let w = db
        .post
        .with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|(((p, _), _), _)| p).collect();
    let vt = votes_of(db).select((&db.vote.vote_type).select(&db.vote_type.origid));
    let s = (&tp).group_by(Ident::<Post>::new()).select(vt.opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    rows(drain(&s).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "owner"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.Score IS NOT NULL THEN p.Score ELSE 0 END) AS TotalScore,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, TotalScore, UpVotes, DownVotes, RANK() OVER (ORDER BY TotalScore DESC) AS RankScore, RANK() OVER (ORDER BY UpVotes DESC) AS RankUpVotes FROM UserActivity)
// SELECT tu.UserId, tu.DisplayName, COALESCE(tu.PostCount, 0) AS PostCount, COALESCE(tu.TotalScore, 0) AS TotalScore, COALESCE(tu.UpVotes, 0) AS UpVotes, COALESCE(tu.DownVotes, 0) AS DownVotes,
//        CASE WHEN tu.RankScore <= 10 THEN 'Top 10 by Score' ELSE 'Not in Top 10' END AS ScoreCategory, CASE WHEN tu.RankUpVotes <= 10 THEN 'Top 10 by UpVotes' ELSE 'Not in Top 10' END AS VoteCategory
// FROM TopUsers tu WHERE tu.TotalScore IS NOT NULL OR tu.UpVotes IS NOT NULL ORDER BY tu.TotalScore DESC, tu.UpVotes DESC;
//
// Every SUM here is over at least the one LEFT JOIN row and adds 0 for a NULL, so it is never NULL and the WHERE keeps every user.
fn q727(db: &'static So) -> String {
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.score).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some((s, t)) => [a[0] + 1, a[1] + s, a[2] + (t == Some(2)) as i64, a[3] + (t == Some(3)) as i64],
            None => a,
        });
    type R = (Id<User>, [i64; 4]);
    let w = whole(&s).select(Ident::<User>::new().and(&s)).window(rank, |(_, a): R| Reverse(a[1]), asc);
    let w = (&w).window(rank, |((_, a), _): (R, i64)| Reverse(a[2]), asc);
    rows(drain(&w).into_iter().map(|(_, (((u, a), r), w))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        f.extend([V::S(if r <= 10 { "Top 10 by Score" } else { "Not in Top 10" }), V::S(if w <= 10 { "Top 10 by UpVotes" } else { "Not in Top 10" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.ViewCount, p.AnswerCount, p.CommentCount, p.AcceptedAnswerId,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.Score > 10),
// PostDetails AS (SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.AnswerCount, rp.CommentCount, COALESCE(a.Id, -1) AS AcceptedAnswerId, COALESCE(c.UserDisplayName, 'No Comments') AS LastCommenter,
//        COUNT(c.Id) AS TotalComments, rp.Rank FROM RankedPosts rp LEFT JOIN Posts a ON rp.AcceptedAnswerId = a.Id LEFT JOIN Comments c ON rp.PostId = c.PostId
//     GROUP BY rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.AnswerCount, rp.CommentCount, a.Id, c.UserDisplayName, rp.Rank)
// SELECT pd.Title, pd.Score, pd.ViewCount, pd.AnswerCount, pd.CommentCount, pd.LastCommenter, pd.TotalComments, CASE WHEN pd.Rank <= 3 THEN 'Top Performer' ELSE 'Regular Performer' END AS PerformanceCategory
// FROM PostDetails pd ORDER BY pd.Score DESC, pd.ViewCount DESC LIMIT 50;
//
// The groups are (post, commenter name): the joined (post, comment) rows are materialised and grouped by the name.
fn q5206(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, .. } = &db.post;
    let rp = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(score.gt(10)));
    let w = rp().group_by(post_type_id).select(Ident::<Post>::new().and(score).and(creation_date)).window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let top3: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 3).map(|(((p, _), _), _)| p).collect();
    let j: MatSet<(Id<Post>, Option<Id<Comment>>)> = rp().select(Ident::<Post>::new().and(comments_of(db).opt())).collect();
    type R = (Id<Post>, Option<Id<Comment>>);
    let name = Same::<R>::new().flat_map(|(_, c): R| c).select(&db.comment.user_display_name).opt();
    let g = (&j).group_by(Same::<R>::new().map(|(p, _): R| p).and(name)).select(Same::<R>::new()).fold(0i64, |n, (_, c)| n + c.is_some() as i64);
    let v = drain((&g).and(Same::<(Id<Post>, Option<Str>)>::new().map(|(p, _)| p).select(Ident::<Post>::new().with(&top3).opt())));
    let v = top_n(v, |&((p, n), _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p, n)
    }, 50);
    rows(v.into_iter().map(|((p, n), (c, t))| {
        let mut f = post_fields(db, p, &["title", "score", "views", "answers", "comments"]);
        f.extend([V::S(n.unwrap_or("No Comments")), V::I(c), V::S(if t.is_some() { "Top Performer" } else { "Regular Performer" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.CreationDate, p.Score, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS PostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.PostTypeId = 1),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.ViewCount, rp.CreationDate, rp.Score, rp.OwnerDisplayName FROM RankedPosts rp WHERE rp.PostRank <= 5),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c GROUP BY c.PostId),
// PostBadges AS (SELECT b.UserId, COUNT(DISTINCT b.Id) AS BadgeCount FROM Badges b GROUP BY b.UserId)
// SELECT tp.PostId, tp.Title, tp.ViewCount, tp.CreationDate, tp.Score, tp.OwnerDisplayName, COALESCE(pc.CommentCount, 0) AS CommentCount, COALESCE(pb.BadgeCount, 0) AS BadgeCount
// FROM TopPosts tp LEFT JOIN PostComments pc ON tp.PostId = pc.PostId LEFT JOIN PostBadges pb ON tp.PostId = pb.UserId ORDER BY tp.Score DESC, tp.ViewCount DESC;
//
// `tp.PostId = pb.UserId` joins a post id to a user id, so it goes through the raw ids.
fn q5537(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, origid, .. } = &db.post;
    let w = db
        .post
        .with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|(((p, _), _), _)| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let pb = db.badge.group_by(&db.badge.user_id).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    rows(drain((&cc).and(origid.select(&pb).opt())).into_iter().map(|(p, (c, b))| {
        let mut f = post_fields(db, p, &["id", "title", "views", "created", "score", "owner"]);
        f.extend([V::I(c), V::I(b.unwrap_or(0))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.PostTypeId IN (1, 2)),
// TopRankedPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.CreationDate, rp.OwnerDisplayName FROM RankedPosts rp WHERE rp.Rank <= 5),
// PostStatistics AS (SELECT p.Id AS PostId, COUNT(c.Id) AS CommentCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id)
// SELECT trp.PostId, trp.Title, trp.OwnerDisplayName, trp.Score, trp.ViewCount, ps.CommentCount, ps.Upvotes, ps.Downvotes
// FROM TopRankedPosts trp JOIN PostStatistics ps ON trp.PostId = ps.PostId ORDER BY trp.Score DESC, ps.Upvotes DESC;
fn q7055(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, view_count, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.is_in([1, 2])))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(row_number, |((p, s), w)| (Reverse(s), w.is_none(), Reverse(w), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|(((p, _), _), _)| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain(&s).into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "score", "views"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH PostStats AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName AS OwnerName, COUNT(DISTINCT c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId IN (2, 3) THEN 1 ELSE 0 END) AS VoteCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '1 YEAR' GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName, p.PostTypeId),
// TopPosts AS (SELECT ps.PostId, ps.Title, ps.OwnerName, ps.Score, ps.ViewCount, ps.Rank FROM PostStats ps WHERE ps.Rank <= 10)
// SELECT tp.Title, tp.OwnerName, tp.Score, tp.ViewCount, ph.Comment AS LastEditComment, ph.CreationDate AS LastEditDate
// FROM TopPosts tp LEFT JOIN PostHistory ph ON tp.PostId = ph.PostId AND ph.CreationDate = (SELECT MAX(ph2.CreationDate) FROM PostHistory ph2 WHERE ph2.PostId = tp.PostId AND ph2.PostHistoryTypeId IN (4, 5))
// WHERE tp.Score > 10 ORDER BY tp.Score DESC, tp.ViewCount DESC;
//
// Rank reads only base columns, so the top posts are picked first; the comment and vote counts are never projected.
fn q32697(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(current_date(), -1))).group_by(post_type_id).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|((p, _), _)| p).collect();
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let md = db.post_history.with(post_history_type_id.in_v(vec![4, 5])).group_by(post).select(hd).fold(i64::MIN, |m, d| m.max(d));
    let at: HashIdx<(Id<Post>, i64), Id<PostHistory>> = db.post_history.select(post.and(hd)).inv().collect();
    rows(drain((&tp).with(score.gt(10)).select(Ident::<Post>::new().and(&md).select(&at).opt())).into_iter().map(|(p, h)| {
        let mut f = post_fields(db, p, &["title", "owner", "score", "views"]);
        f.extend(match h {
            Some(h) => [ostr(db.post_history.comment.get(h)), V::T(hd.get(h).unwrap())],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.Reputation, U.DisplayName, COUNT(DISTINCT P.Id) AS PostCount, COUNT(DISTINCT C.Id) AS CommentCount,
//        SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId WHERE U.Reputation > 100
//     GROUP BY U.Id, U.Reputation, U.DisplayName),
// TopUsers AS (SELECT UserId, Reputation, DisplayName, PostCount, CommentCount, UpVotes, DownVotes, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS UserRank FROM UserReputation)
// SELECT RU.UserRank, RU.DisplayName, RU.PostCount, RU.CommentCount, COALESCE(NULLIF(RU.UpVotes, 0), 1) AS EffectiveUpVotes, COALESCE(NULLIF(RU.DownVotes, 0), 1) AS EffectiveDownVotes,
//        CASE WHEN RU.UpVotes > RU.DownVotes THEN 'Positive Influencer' WHEN RU.UpVotes < RU.DownVotes THEN 'Negative Influencer' ELSE 'Balanced User' END AS InfluenceType,
//        STDDEV(RU.Reputation) OVER () AS StdDevReputation
// FROM TopUsers RU WHERE RU.UserRank <= 10 ORDER BY RU.UserRank;
//
// UserRank reads only Reputation, so the ten users are picked first and the post x comment x vote product is driven for those alone.
// The window STDDEV runs over the ten rows the WHERE keeps.
fn q24126(db: &'static So) -> String {
    let w = whole(db.user.with((&db.user.reputation).gt(100))).select(Ident::<User>::new().and(&db.user.reputation)).window(row_number, |(u, r)| (Reverse(r), u), asc);
    let tu: MatSet<(Id<User>, i64)> = (&w).filt(|(_, k)| k <= 10).map(|((u, _), k)| (u, k)).collect();
    let s = (&tu)
        .map(|(u, _)| u)
        .group_by(Same::<Id<User>>::new())
        .select(posts_of(db).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 2], |a, p| {
            let t = p.and_then(|(_, t)| t);
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
        });
    let np = (&tu).map(|(u, _)| u).group_by(Same::<Id<User>>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let nc = (&tu).map(|(u, _)| u).group_by(Same::<Id<User>>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let (n, sum) = (&tu).map(|(u, _)| u).select(&db.user.reputation).fold_flat((0i64, 0i64), |(n, s), r| (n + 1, s + r));
    let mean = sum as f64 / n as f64;
    let m2 = (&tu).map(|(u, _)| u).select(&db.user.reputation).fold_flat(0.0f64, |m, r| m + (r as f64 - mean) * (r as f64 - mean));
    let sd = if n > 1 { V::F((m2 / (n - 1) as f64).sqrt()) } else { V::Null };
    type R = (Id<User>, i64);
    rows(drain((&tu).select(Same::<R>::new().and(Same::<R>::new().map(|(u, _): R| u).select((&np).and(&nc).and(&s))))).into_iter().map(|(_, ((u, r), ((p, c), a)))| {
        let f = |x: i64| if x == 0 { 1 } else { x };
        row(vec![
            V::I(r),
            user_col(db, u, "name"),
            V::I(p),
            V::I(c),
            V::I(f(a[0])),
            V::I(f(a[1])),
            V::S(if a[0] > a[1] { "Positive Influencer" } else if a[0] < a[1] { "Negative Influencer" } else { "Balanced User" }),
            match &sd {
                V::F(x) => V::F(*x),
                _ => V::Null,
            },
        ])
    }))
}

// WITH RecentPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, p.AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.CreationDate >= DATE '2024-10-01' - INTERVAL '30 days'),
// EngagedUsers AS (SELECT u.Id, u.DisplayName, u.Reputation, COUNT(DISTINCT c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes FROM Users u LEFT JOIN Comments c ON u.Id = c.UserId LEFT JOIN Votes v ON u.Id = v.UserId
//     WHERE u.CreationDate < DATE '2024-10-01' - INTERVAL '90 days' GROUP BY u.Id, u.DisplayName, u.Reputation HAVING COUNT(DISTINCT c.Id) > 0),
// HighScoringPosts AS (SELECT p.Id, p.Title, p.Score, u.DisplayName, u.Reputation FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.Score > 100)
// SELECT rp.Title AS RecentPostTitle, rp.CreationDate AS RecentPostDate, eu.DisplayName AS EngagedUserName, eu.Reputation AS EngagedUserReputation, hsp.Title AS HighScoringPostTitle,
//        hsp.Score AS HighScoringPostScore
// FROM RecentPosts rp JOIN EngagedUsers eu ON eu.Id = rp.OwnerUserId JOIN HighScoringPosts hsp ON hsp.Id = rp.Id WHERE rp.rn = 1 ORDER BY rp.CreationDate DESC, eu.Reputation DESC LIMIT 10;
//
// EngagedUsers' vote sums are never read, so only its HAVING (a comment by the user) is computed.
fn q6201(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_days(date(2024, 10, 1), -30))).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let fp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let engaged = Ident::<User>::new().with((&db.user.creation_date).lt(add_days(date(2024, 10, 1), -90))).with(comments_by(db));
    let v = drain((&fp).with(score.gt(100)).select(owner_user.select(engaged)));
    let v = top_n(v, |&(p, u)| (Reverse(creation_date.get(p).unwrap()), Reverse(db.user.reputation.get(u).unwrap()), p), 10);
    rows(v.into_iter().map(|(p, u)| {
        let mut f = post_fields(db, p, &["title", "created"]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend(post_fields(db, p, &["title", "score"]));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, COUNT(a.Id) AS AnswerCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN Posts a ON p.Id = a.ParentId AND p.PostTypeId = 1 LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount),
// TopPosts AS (SELECT PostId, Title, CreationDate, ViewCount, AnswerCount, UpVotes, DownVotes, RANK() OVER (ORDER BY ViewCount DESC, UpVotes DESC) AS PopularityRank FROM RankedPosts WHERE rn = 1)
// SELECT tp.Title, tp.CreationDate, tp.ViewCount, tp.AnswerCount, tp.UpVotes, tp.DownVotes, COALESCE(bt.Name, 'No Badge') AS Badge, u.DisplayName AS OwnerDisplayName
// FROM TopPosts tp LEFT JOIN Users u ON tp.PostId = u.Id LEFT JOIN Badges bt ON u.Id = bt.UserId AND bt.Date >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'
// WHERE tp.PopularityRank <= 10 ORDER BY tp.PopularityRank;
//
// rn partitions by p.Id, so it is 1 for every post. `tp.PostId = u.Id` joins a post id to a user id, so it goes through the raw ids.
fn q6839(db: &'static So) -> String {
    let Post { creation_date, post_type_id, view_count, origid, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let kids = Ident::<Post>::new().with(post_type_id.eq(1)).select(children_of(db));
    let rp = db
        .post
        .with(creation_date.ge(add_days(t0, -30)))
        .group_by(Ident::<Post>::new())
        .select(kids.opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let w = whole(&rp).select(Ident::<Post>::new().and(&rp).and(view_count.opt())).window(rank, |((_, a), w)| (w.is_none(), Reverse(w), Reverse(a[1])), asc);
    let tp: MatSet<((Id<Post>, [i64; 3]), i64)> = (&w).filt(|(_, r)| r <= 10).map(|(((p, a), _), r)| ((p, a), r)).collect();
    let by_orig: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let recent_badges = badges_of(db).select(Ident::<Badge>::new().with((&db.badge.date).ge(add_days(t0, -30))));
    type R = ((Id<Post>, [i64; 3]), i64);
    let v = drain((&tp).select(Same::<R>::new().and(Same::<R>::new().map(|((p, _), _): R| p).select(origid.select(&by_orig).select(Ident::<User>::new().and(recent_badges.opt())).opt()))));
    rows(v.into_iter().map(|(_, (((p, a), _), u))| {
        let mut f = post_fields(db, p, &["title", "created", "views"]);
        f.extend(a.map(V::I));
        f.push(V::S(u.and_then(|(_, b)| b).map_or("No Badge", |b| db.badge.name.get(b).unwrap())));
        f.push(u.map_or(V::Null, |(u, _)| user_col(db, u, "name")));
        row(f)
    }))
}

// WITH UserVoteStats AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        COUNT(DISTINCT p.Id) AS PostCount FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Posts p ON v.PostId = p.Id GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, UpVotes - DownVotes AS NetVotes, RANK() OVER (ORDER BY UpVotes DESC) AS Rank FROM UserVoteStats WHERE PostCount > 5),
// ClosedQuestions AS (SELECT p.Id AS ClosedPostId, p.Title, ph.CreationDate, ph.Comment AS CloseReason FROM Posts p JOIN PostHistory ph ON p.Id = ph.PostId WHERE ph.PostHistoryTypeId = 10),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c GROUP BY c.PostId)
// SELECT tu.UserId, tu.DisplayName, tu.NetVotes, COUNT(DISTINCT cq.ClosedPostId) AS ClosedQuestionsCount, COALESCE(SUM(pc.CommentCount), 0) AS TotalComments
// FROM TopUsers tu LEFT JOIN ClosedQuestions cq ON tu.UserId = cq.ClosedPostId LEFT JOIN PostComments pc ON cq.ClosedPostId = pc.PostId
// GROUP BY tu.UserId, tu.DisplayName, tu.NetVotes HAVING COUNT(DISTINCT cq.ClosedPostId) > 0 ORDER BY tu.NetVotes DESC, tu.DisplayName;
//
// `tu.UserId = cq.ClosedPostId` joins a user id to a post id, so it goes through the raw ids.
fn q188(db: &'static So) -> String {
    let uv = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold(0i64, |n, t| n + (t == Some(2)) as i64 - (t == Some(3)) as i64);
    let pc_ = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.post)).count_distinct();
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10)));
    let pc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let by_orig: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let cq = db.post.group_by(Ident::<Post>::new()).select(closes.and((&pc).opt())).fold((0i64, 0i64), |(n, s), (_, c)| (n + 1, s + c.unwrap_or(0)));
    let v = drain((&pc_).filt(|n| n > 5).and(&uv).and((&db.user.origid).select(&by_orig).select(&cq)));
    rows(v.into_iter().map(|(u, ((_, n), (_, s)))| row(vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(n), V::I(1), V::I(s)])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, u.DisplayName AS OwnerName, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS PostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days'),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.OwnerName, rp.CreationDate FROM RankedPosts rp WHERE rp.PostRank <= 5),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c GROUP BY c.PostId),
// PostVoteStats AS (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes v GROUP BY v.PostId)
// SELECT tp.PostId, tp.Title, tp.Score, tp.OwnerName, tp.CreationDate, COALESCE(pc.CommentCount, 0) AS TotalComments, COALESCE(pvs.UpVotes, 0) AS TotalUpVotes, COALESCE(pvs.DownVotes, 0) AS TotalDownVotes
// FROM TopPosts tp LEFT JOIN PostComments pc ON tp.PostId = pc.PostId LEFT JOIN PostVoteStats pvs ON tp.PostId = pvs.PostId ORDER BY tp.Score DESC, tp.CreationDate DESC;
fn q6500(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_days(date(2024, 10, 1), -30))).with(owner_user).group_by(post_type_id).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    rows(drain((&cc).and(&vc)).into_iter().map(|(p, (c, a))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "owner", "created"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.PostTypeId),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.CommentCount, rp.UpvoteCount, rp.DownvoteCount FROM RankedPosts rp WHERE rp.Rank <= 10)
// SELECT tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.CommentCount, tp.UpvoteCount, tp.DownvoteCount, COALESCE(u.DisplayName, 'Anonymous') AS AuthorName, COALESCE(b.Name, 'No Badge') AS UserBadge
// FROM TopPosts tp LEFT JOIN Users u ON tp.PostId = u.Id LEFT JOIN Badges b ON u.Id = b.UserId AND b.Class = 1 ORDER BY tp.Score DESC, tp.ViewCount DESC;
//
// `tp.PostId = u.Id` joins a post id to a user id, so it goes through the raw ids. Rank reads only base columns, so the top posts are picked first.
fn q8679(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, origid, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|(((p, _), _), _)| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let by_orig: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let gold = badges_of(db).select(Ident::<Badge>::new().with((&db.badge.class).eq(1))).select(&db.badge.name);
    rows(drain((&s).and(origid.select(&by_orig).select(Ident::<User>::new().and(gold.opt())).opt())).into_iter().map(|(p, (a, u))| {
        let mut f = post_fields(db, p, &["title", "created", "score", "views"]);
        f.extend(a.map(V::I));
        f.push(u.map_or(V::S("Anonymous"), |(u, _)| user_col(db, u, "name")));
        f.push(V::S(u.and_then(|(_, b)| b).unwrap_or("No Badge")));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT a.Id) AS AnswerCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2 LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.PostTypeId),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, CommentCount, AnswerCount, UpVoteCount, DownVoteCount FROM RankedPosts WHERE rn <= 10)
// SELECT tp.*, u.DisplayName AS AuthorDisplayName, u.Reputation AS AuthorReputation, (SELECT COUNT(*) FROM Votes v WHERE v.PostId = tp.PostId AND v.VoteTypeId IN (2, 3)) AS TotalVotes
// FROM TopPosts tp JOIN Users u ON tp.PostId IN (SELECT p.Id FROM Posts p WHERE p.OwnerUserId = u.Id) ORDER BY tp.Score DESC, tp.CreationDate DESC;
//
// rn reads only base columns, so the newest posts are picked first and the comment x answer x vote product is driven for those alone.
fn q5246(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(post_type_id).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|((p, _), _)| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(answers_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + matches!(t, Some(2 | 3)) as i64]);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ac = (&tp).group_by(Ident::<Post>::new()).select(answers_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let tv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([2, 3]))).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    rows(drain((&s).and(&cc).and(&ac).and(&tv).and(owner_user)).into_iter().map(|(p, ((((a, c), n), t), u))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend([V::I(c), V::I(n), V::I(a[0]), V::I(a[1])]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.push(V::I(t));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS ScoreRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days'),
// RecentVotes AS (SELECT v.PostId, v.VoteTypeId, COUNT(*) AS VoteCount FROM Votes v WHERE v.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days' GROUP BY v.PostId, v.VoteTypeId),
// PostMetrics AS (SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.CreationDate, rp.OwnerDisplayName, COALESCE(rv.VoteCount, 0) AS UpVotes, COALESCE(rv.VoteCount, 0) AS DownVotes, rp.ScoreRank
//     FROM RankedPosts rp LEFT JOIN RecentVotes rv ON rp.PostId = rv.PostId AND rv.VoteTypeId = 2)
// SELECT pm.PostId, pm.Title, pm.Score, pm.UpVotes, pm.DownVotes, pm.ViewCount, pm.CreationDate, pm.OwnerDisplayName, CASE WHEN pm.ScoreRank <= 5 THEN 'Top 5' ELSE 'Other' END AS RankCategory
// FROM PostMetrics pm ORDER BY pm.Score DESC, pm.ViewCount DESC;
fn q7452(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, view_count, .. } = &db.post;
    let since = add_days(date(2024, 10, 1), -30);
    let rp: MatSet<Id<Post>> = db.post.with(creation_date.ge(since)).with(owner_user).collect();
    let w = (&rp).group_by(post_type_id).select(Ident::<Post>::new().and(score).and(view_count.opt())).window(row_number, |((p, s), w)| (Reverse(s), w.is_none(), Reverse(w), p), asc);
    let top5: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|(((p, _), _), _)| p).collect();
    let Vote { post, vote_type_id, creation_date: vd, .. } = &db.vote;
    let rv = db.vote.with(vd.ge(since)).group_by(post.and(vote_type_id)).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let up = Ident::<Post>::new().map(|p| (p, 2i64)).select(&rv);
    rows(drain((&rp).select(up.opt().and(Ident::<Post>::new().with(&top5).opt()))).into_iter().map(|(p, (n, t))| {
        let n = n.unwrap_or(0);
        let mut f = post_fields(db, p, &["id", "title", "score"]);
        f.extend([V::I(n), V::I(n)]);
        f.extend(post_fields(db, p, &["views", "created", "owner"]));
        f.push(V::S(if t.is_some() { "Top 5" } else { "Other" }));
        row(f)
    }))
}

// Rewritten (rewrites/2757.sql): the final ORDER BY tie-broken on UserId.
// WITH UserBadgeCounts AS (SELECT UserId, COUNT(*) FILTER (WHERE Class = 1) AS GoldCount, COUNT(*) FILTER (WHERE Class = 2) AS SilverCount, COUNT(*) FILTER (WHERE Class = 3) AS BronzeCount
//     FROM Badges GROUP BY UserId),
// PostAggregate AS (SELECT OwnerUserId, COUNT(*) AS TotalPosts, SUM(ViewCount) AS TotalViews, AVG(Score) AS AverageScore FROM Posts
//     WHERE CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year' GROUP BY OwnerUserId),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(ub.GoldCount, 0) AS GoldCount, COALESCE(ub.SilverCount, 0) AS SilverCount, COALESCE(ub.BronzeCount, 0) AS BronzeCount,
//        pa.TotalPosts, pa.TotalViews, pa.AverageScore, RANK() OVER (ORDER BY pa.TotalPosts DESC, pa.TotalViews DESC) AS UserRank
//     FROM Users u LEFT JOIN UserBadgeCounts ub ON u.Id = ub.UserId LEFT JOIN PostAggregate pa ON u.Id = pa.OwnerUserId)
// SELECT UserId, DisplayName, GoldCount, SilverCount, BronzeCount, TotalPosts, TotalViews, AverageScore, UserRank,
//        CASE WHEN UserRank <= 10 THEN 'Top User' WHEN UserRank BETWEEN 11 AND 50 THEN 'Active User' ELSE 'Inactive User' END AS UserCategory
// FROM TopUsers WHERE TotalPosts > 0 ORDER BY UserRank, UserId LIMIT 100;
//
// The users without a recent post rank after every user with one (NULLs last), so the ranks of the rows kept come from the users with posts alone.
fn q2757(db: &'static So) -> String {
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let Post { creation_date, owner_user, score, view_count, .. } = &db.post;
    let pa = db
        .post
        .with(creation_date.ge(add_years(date(2024, 10, 1), -1)))
        .group_by(owner_user)
        .select(score.and(view_count.opt()))
        .fold([0i64; 4], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]);
    let w = whole(&pa).select(Ident::<User>::new().and((&pa).and((&ub).opt()))).window(rank, |(_, (a, _))| (Reverse(a[0]), a[2] == 0, Reverse(a[3])), asc);
    let v = top_n(drain(&w), |&(_, ((u, _), r))| (r, db.user.origid.get(u).unwrap()), 100);
    rows(v.into_iter().map(|(_, ((u, (a, b)), r))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(b.unwrap_or([0; 3]).map(V::I));
        f.extend([V::I(a[0]), nullable(a[3], a[2]), avg(a[1], a[0]), V::I(r), V::S(if r <= 10 { "Top User" } else if r <= 50 { "Active User" } else { "Inactive User" })]);
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.Tags, p.CreationDate, COUNT(a.Id) AS AnswerCount, STRING_AGG(DISTINCT u.DisplayName, ', ') AS TopContributors,
//        COALESCE(MAX(v.CreationDate), '1900-01-01') AS LastVoteDate
//     FROM Posts p LEFT JOIN Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2 LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Users u ON u.Id = p.OwnerUserId
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.Tags, p.CreationDate),
// PostDetails AS (SELECT rp.PostId, rp.Title, rp.Tags, rp.CreationDate, rp.AnswerCount, rp.TopContributors, rp.LastVoteDate,
//        CASE WHEN rp.AnswerCount > 5 THEN 'Hot' WHEN rp.LastVoteDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '2 days' THEN 'Trending' ELSE 'Regular' END AS PostStatus FROM RecentPosts rp)
// SELECT pd.PostId, pd.Title, pd.Tags, pd.CreationDate, pd.AnswerCount, pd.TopContributors, pd.PostStatus, (SELECT COUNT(*) FROM Comments c WHERE c.PostId = pd.PostId) AS CommentCount
// FROM PostDetails pd WHERE pd.PostStatus IN ('Hot', 'Trending') ORDER BY pd.CreationDate DESC;
//
// The DISTINCT display names of one post's joined rows are its owner's name, or NULL.
fn q25750(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let Post { creation_date, owner_user, .. } = &db.post;
    let rp = db
        .post
        .with(creation_date.ge(add_days(t0, -30)))
        .group_by(Ident::<Post>::new())
        .select(answers_of(db).opt().and(votes_of(db).select(&db.vote.creation_date).opt()).and(owner_user.select(&db.user.display_name).opt()))
        .fold((0i64, i64::MIN, None::<Str>), |(n, m, u), ((a, d), name)| (n + a.is_some() as i64, m.max(d.unwrap_or(i64::MIN)), u.or(name)));
    let cut = add_days(t0, -2);
    let status = |(n, m, _): (i64, i64, Option<Str>)| if n > 5 { "Hot" } else if m != i64::MIN && m >= cut { "Trending" } else { "Regular" };
    let cc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    rows(drain((&rp).filt(move |a| status(a) != "Regular").and((&cc).opt())).into_iter().map(|(p, (a, c))| {
        let mut f = post_fields(db, p, &["id", "title", "tags", "created"]);
        f.extend([V::I(a.0), ostr(a.2), V::S(status(a)), V::I(c.unwrap_or(0))]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END) AS PositiveScorePosts,
//        SUM(CASE WHEN P.Score < 0 THEN 1 ELSE 0 END) AS NegativeScorePosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
//        SUM(CASE WHEN C.Id IS NOT NULL THEN 1 ELSE 0 END) AS CommentsCount, MAX(P.CreationDate) AS LastPostDate
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, PositiveScorePosts, NegativeScorePosts, Questions, Answers, CommentsCount, LastPostDate, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserStats)
// SELECT T.DisplayName, T.Reputation, T.TotalPosts, T.PositiveScorePosts, T.NegativeScorePosts, T.Questions, T.Answers, T.CommentsCount, T.LastPostDate,
//        ROW_NUMBER() OVER (PARTITION BY T.ReputationRank ORDER BY T.LastPostDate DESC) AS PostRecencyRank
// FROM TopUsers T WHERE T.ReputationRank <= 10 ORDER BY T.Reputation DESC, T.LastPostDate DESC;
//
// ReputationRank reads only Reputation, so the top users are picked first and the post x comment product is driven for those alone.
fn q5628(db: &'static So) -> String {
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r)| Reverse(r), asc);
    let tr: MatSet<(Id<User>, i64)> = (&w).filt(|(_, r)| r <= 10).map(|((u, _), r)| (u, r)).collect();
    let tu: MatSet<Id<User>> = (&tr).map(|(u, _)| u).collect();
    let Post { score, post_type_id, creation_date, .. } = &db.post;
    let s = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score.and(post_type_id).and(creation_date).and(comments_of(db).opt())).opt())
        .fold([0, 0, 0, 0, 0, i64::MIN], |a, p| match p {
            Some((((s, t), d), c)) => [a[0] + (s > 0) as i64, a[1] + (s < 0) as i64, a[2] + (t == 1) as i64, a[3] + (t == 2) as i64, a[4] + c.is_some() as i64, a[5].max(d)],
            None => a,
        });
    let np = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    type T = (Id<User>, i64);
    let w = (&tr)
        .group_by(Same::<T>::new().map(|(_, r): T| r))
        .select(Same::<T>::new().map(|(u, _): T| u).select(Ident::<User>::new().and((&np).and(&s))))
        .window(row_number, |(u, (_, a))| (a[5] == i64::MIN, Reverse(a[5]), u), asc);
    rows(drain(&w).into_iter().map(|(_, ((u, (n, a)), r))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(n));
        f.extend(a[..5].iter().map(|&x| V::I(x)));
        f.extend([tmax(a[5]), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, U.DisplayName AS Owner, P.CreationDate, P.Score, P.ViewCount, ROW_NUMBER() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC) AS Rank
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// PostVoteCounts AS (SELECT PostId, COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVotes FROM Votes V GROUP BY PostId),
// TopPosts AS (SELECT RP.PostId, RP.Title, RP.Owner, RP.CreationDate, RP.Score, RP.ViewCount, PVC.UpVotes, PVC.DownVotes FROM RankedPosts RP LEFT JOIN PostVoteCounts PVC ON RP.PostId = PVC.PostId WHERE RP.Rank <= 5)
// SELECT TP.PostId, TP.Title, TP.Owner, TP.CreationDate, TP.Score, TP.ViewCount, COALESCE(TP.UpVotes, 0) AS UpVotes, COALESCE(TP.DownVotes, 0) AS DownVotes,
//        CASE WHEN TP.Score > 100 THEN 'High scorer' WHEN TP.Score BETWEEN 50 AND 100 THEN 'Moderate scorer' ELSE 'Low scorer' END AS ScoreCategory
// FROM TopPosts TP ORDER BY TP.Score DESC, TP.ViewCount DESC;
fn q6182(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).with(owner_user).group_by(post_type_id).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    rows(drain(&vc).into_iter().map(|(p, a)| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "score", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::S(if s > 100 { "High scorer" } else if s >= 50 { "Moderate scorer" } else { "Low scorer" })]);
        row(f)
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("8832", q8832),
    ("1384", q1384),
    ("31338", q31338),
    ("2257", q2257),
    ("3188", q3188),
    ("8278", q8278),
    ("9646", q9646),
    ("2273", q2273),
    ("3706", q3706),
    ("8099", q8099),
    ("2956", q2956),
    ("5960", q5960),
    ("3055", q3055),
    ("3859", q3859),
    ("4990", q4990),
    ("6799", q6799),
    ("20800", q20800),
    ("3824", q3824),
    ("7108", q7108),
    ("7428", q7428),
    ("8503", q8503),
    ("8896", q8896),
    ("8905", q8905),
    ("10682", q10682),
    ("2315", q2315),
    ("6098", q6098),
    ("9462", q9462),
    ("32414", q32414),
    ("6509", q6509),
    ("7868", q7868),
    ("2485", q2485),
    ("29933", q29933),
    ("33807", q33807),
    ("5405", q5405),
    ("2159", q2159),
    ("2911", q2911),
    ("7686", q7686),
    ("8368", q8368),
    ("3280", q3280),
    ("7249", q7249),
    ("3064", q3064),
    ("4419", q4419),
    ("6373", q6373),
    ("2709", q2709),
    ("3207", q3207),
    ("7204", q7204),
    ("6997", q6997),
    ("29644", q29644),
    ("252", q252),
    ("2719", q2719),
    ("3354", q3354),
    ("3427", q3427),
    ("7685", q7685),
    ("8777", q8777),
    ("3490", q3490),
    ("8727", q8727),
    ("3642", q3642),
    ("31871", q31871),
    ("5170", q5170),
    ("6190", q6190),
    ("9711", q9711),
    ("3088", q3088),
    ("5558", q5558),
    ("8323", q8323),
    ("2530", q2530),
    ("3906", q3906),
    ("9415", q9415),
    ("394", q394),
    ("3342", q3342),
    ("4961", q4961),
    ("650", q650),
    ("2689", q2689),
    ("2878", q2878),
    ("7776", q7776),
    ("2106", q2106),
    ("4194", q4194),
    ("2004", q2004),
    ("8415", q8415),
    ("727", q727),
    ("5206", q5206),
    ("5537", q5537),
    ("7055", q7055),
    ("32697", q32697),
    ("24126", q24126),
    ("6201", q6201),
    ("6839", q6839),
    ("188", q188),
    ("6500", q6500),
    ("8679", q8679),
    ("5246", q5246),
    ("7452", q7452),
    ("2757", q2757),
    ("25750", q25750),
    ("5628", q5628),
    ("6182", q6182),
];
