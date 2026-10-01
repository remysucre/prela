use harness::prelude::*;
use std::cmp::Reverse;

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, p.Score, COUNT(a.Id) AS AnswerCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank
//     FROM Posts p LEFT JOIN Posts a ON a.ParentId = p.Id AND a.PostTypeId = 2
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.Score),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(p.Score) AS TotalScore FROM Users u JOIN Posts p ON p.OwnerUserId = u.Id
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY u.Id, u.DisplayName ORDER BY TotalScore DESC LIMIT 10)
// SELECT r.PostId, r.Title, r.CreationDate, u.DisplayName AS OwnerDisplayName, r.Score AS PostScore, r.AnswerCount, t.TotalScore
// FROM RankedPosts r JOIN Users u ON r.OwnerUserId = u.Id JOIN TopUsers t ON u.Id = t.UserId WHERE r.UserPostRank <= 5 ORDER BY t.TotalScore DESC, r.CreationDate DESC;
fn q6276(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let tot = recent().group_by(owner_user).select(&db.post.score).fold(0i64, |s, x| s + x);
    let tu = top_n(drain(&tot), |&(u, s)| (Reverse(s), u), 10);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let w = recent()
        .with(post_type_id.eq(1))
        .group_by(owner_user.select(Ident::<User>::new().with(&tu)))
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(_, d)| d, desc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let ac = (&tp).group_by(Ident::<Post>::new()).select(answers_of(db).opt()).fold(0i64, |n, a| n + a.is_some() as i64);
    let mut v = drain((&ac).and(owner_user.select(&tot)));
    v.sort_by_key(|&(p, (_, t))| (Reverse(t), Reverse(creation_date.get(p).unwrap())));
    rows(v.into_iter().map(|(p, (a, t))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner", "score"]);
        f.extend([V::I(a), V::I(t)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.Tags, u.DisplayName AS OwnerDisplayName, p.CreationDate, COUNT(c.Id) AS CommentCount,
//        COUNT(DISTINCT v.UserId) AS VoteCount, RANK() OVER (PARTITION BY p.Tags ORDER BY COUNT(DISTINCT v.UserId) DESC) AS TagRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Body, p.Tags, u.DisplayName, p.CreationDate),
// FilteredPosts AS (SELECT PostId, Title, OwnerDisplayName, CreationDate, CommentCount, VoteCount FROM RankedPosts WHERE TagRank <= 5),
// PostStatistics AS (SELECT COUNT(PostId) AS TotalPosts, AVG(CommentCount) AS AvgComments, AVG(VoteCount) AS AvgVotes FROM FilteredPosts)
// SELECT f.OwnerDisplayName, f.Title, f.CommentCount, f.VoteCount, ps.TotalPosts, ps.AvgComments, ps.AvgVotes
// FROM FilteredPosts f CROSS JOIN PostStatistics ps ORDER BY f.VoteCount DESC;
fn q26574(db: &'static So) -> String {
    let Post { post_type_id, tags_str, .. } = &db.post;
    let qs = || db.post.with(post_type_id.eq(1)).group_by(Ident::<Post>::new());
    let cc = qs().select(comments_of(db).opt().and(votes_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let dv = qs().select(votes_of(db).select(&db.vote.user_id)).count_distinct();
    let w = db
        .post
        .with(post_type_id.eq(1))
        .group_by(tags_str.opt())
        .select(Ident::<Post>::new().and(&cc).and((&dv).opt()))
        .window(rank, |((_, _), n)| n.unwrap_or(0), desc);
    let fp: MatSet<(Id<Post>, i64, i64)> = (&w).filt(|(_, r)| r <= 5).map(|(((p, c), n), _)| (p, c, n.unwrap_or(0))).collect();
    let (n, cs, vs) = (&fp).fold_flat((0i64, 0i64, 0i64), |(n, c, s), (_, x, y)| (n + 1, c + x, s + y));
    let mut v = drain(&fp).into_iter().map(|x| x.1).collect::<Vec<_>>();
    v.sort_by_key(|&(_, _, n)| Reverse(n));
    rows(v.into_iter().map(|(p, c, d)| {
        let mut f = post_fields(db, p, &["owner", "title"]);
        f.extend([V::I(c), V::I(d), V::I(n), avg(cs, n), avg(vs, n)]);
        row(f)
    }))
}

// WITH UserBadgeCounts AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount, AVG(U.Reputation) AS AvgReputation
//     FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// PostStatistics AS (SELECT P.OwnerUserId, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(P.ViewCount) AS TotalViews FROM Posts P GROUP BY P.OwnerUserId),
// TopUsers AS (SELECT U.UserId, U.DisplayName, U.BadgeCount, P.TotalPosts, P.TotalQuestions, P.TotalAnswers, P.TotalViews,
//        RANK() OVER (ORDER BY U.BadgeCount DESC) AS BadgeRank, RANK() OVER (ORDER BY P.TotalPosts DESC) AS PostRank
//     FROM UserBadgeCounts U JOIN PostStatistics P ON U.UserId = P.OwnerUserId)
// SELECT UserId, DisplayName, BadgeCount, TotalPosts, TotalQuestions, TotalAnswers, TotalViews, BadgeRank, PostRank
// FROM TopUsers WHERE BadgeRank <= 10 OR PostRank <= 10 ORDER BY BadgeRank, PostRank;
fn q28577(db: &'static So) -> String {
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let ups = user_posts(db);
    let j = (&bc).and((&ups).filt(|a| a[1] > 0));
    let w1 = whole(db.user.iq()).select(Ident::<User>::new().and(&j)).window(rank, |(_, (b, _))| b, desc);
    let w2 = (&w1).window(rank, |((_, (_, a)), _)| a[1], desc);
    let mut v: Vec<_> = drain((&w2).filt(|((_, b), p)| b <= 10 || p <= 10)).into_iter().map(|x| x.1).collect();
    v.sort_by_key(|&((_, b), p)| (b, p));
    rows(v.into_iter().map(|(((u, (b, a)), br), pr)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(b), V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[6], a[5]), V::I(br), V::I(pr)]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId WHERE U.Reputation >= 100 GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT ..., RANK() OVER (ORDER BY Reputation DESC) AS Rank FROM UserPostStats)
// SELECT T.DisplayName, T.Reputation, T.PostCount, T.QuestionCount, T.AnswerCount, T.UpVotes, T.DownVotes,
//        CASE WHEN T.Rank <= 10 THEN 'Top Contributor' WHEN T.Rank <= 50 THEN 'Contributor' ELSE 'New User' END AS UserStatus
// FROM TopUsers T WHERE T.PostCount > 0 ORDER BY T.Rank;
fn q9045(db: &'static So) -> String {
    let s = db
        .user
        .with((&db.user.reputation).ge(100))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 5], |a, p| match p {
            Some((t, v)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (v == Some(2)) as i64, a[4] + (v == Some(3)) as i64],
            None => a,
        });
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&s).and(&db.user.reputation)).window(rank, |(_, r)| r, desc);
    rows(drain((&w).filt(|(((_, a), _), _)| a[0] > 0)).into_iter().map(|x| x.1).map(|(((u, a), _), r)| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(a.map(V::I));
        f.push(V::S(if r <= 10 { "Top Contributor" } else if r <= 50 { "Contributor" } else { "New User" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.ViewCount, COUNT(c.Id) AS CommentCount, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVoteCount,
//        COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVoteCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.Title, p.Score, p.CreationDate, p.ViewCount, p.OwnerUserId),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.CreationDate, rp.ViewCount, rp.CommentCount, rp.UpVoteCount, rp.DownVoteCount FROM RankedPosts rp WHERE rp.Rank <= 5)
// SELECT up.DisplayName AS Owner, tp.Title, tp.Score, tp.CreationDate, tp.ViewCount, tp.CommentCount, tp.UpVoteCount, tp.DownVoteCount, u.Reputation, u.LastAccessDate
// FROM TopPosts tp JOIN Users up ON tp.PostId = up.Id JOIN Users u ON up.Id = u.Id WHERE u.Reputation > 1000 ORDER BY tp.Score DESC, tp.ViewCount DESC;
//
// The row number only reads Score and CreationDate, so the posts are picked
// before their joined rows are counted.
fn q6395(db: &'static So) -> String {
    let Post { owner_user_id, score, creation_date, origid, view_count, .. } = &db.post;
    let w = db
        .post
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((_, s), d)| (Reverse(s), Reverse(d)), asc);
    let uids: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let hit = Ident::<User>::new().with((&db.user.reputation).gt(1000));
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|(((p, _), _), _)| p).collect();
    let tp: MatSet<Id<Post>> = (&tp).with(origid.select(&uids).select(hit)).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let mut v = drain((&s).and(origid.select(&uids)));
    v.sort_by_key(|&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    });
    rows(v.into_iter().map(|(p, (a, u))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title", "score", "created", "views"]));
        f.extend(a.map(V::I));
        f.extend(ucols(db, u, &["rep", "last_access"]));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY pt.Name ORDER BY p.Score DESC) AS Rank, COUNT(v.Id) AS VoteCount
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 2
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName, pt.Name),
// TopPosts AS (SELECT rp.* FROM RankedPosts rp WHERE rp.Rank <= 5),
// CommentsStats AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount, MAX(c.CreationDate) AS LastCommentDate FROM Comments c JOIN TopPosts tp ON c.PostId = tp.Id GROUP BY c.PostId)
// SELECT tp.Id, tp.Title, tp.OwnerDisplayName, tp.ViewCount, tp.Score, ts.CommentCount, ts.LastCommentDate
// FROM TopPosts tp LEFT JOIN CommentsStats ts ON tp.Id = ts.PostId ORDER BY tp.Score DESC, ts.CommentCount DESC;
fn q9194(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .with(owner_user)
        .group_by(ptype_name(db))
        .select(Ident::<Post>::new().and(score))
        .window(row_number, |(p, s)| (Reverse(s), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let cs = db.comment.group_by(&db.comment.post).select(&db.comment.creation_date).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let mut v = drain((&tp).select(Ident::<Post>::new().and((&cs).opt())));
    v.sort_by_key(|&(_, (p, c))| (Reverse(score.get(p).unwrap()), c.is_none(), Reverse(c.map(|x| x.0))));
    rows(v.into_iter().map(|(_, (p, c))| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "views", "score"]);
        f.extend(match c {
            Some((n, m)) => [V::I(n), V::T(m)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.Score, p.OwnerUserId, u.DisplayName AS OwnerDisplayName, p.CreationDate,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS Rank FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.Score > 0),
// TopUserPosts AS (SELECT rp.OwnerUserId, rp.OwnerDisplayName, COUNT(rp.Id) AS PostCount, SUM(rp.Score) AS TotalScore FROM RankedPosts rp WHERE rp.Rank <= 5
//     GROUP BY rp.OwnerUserId, rp.OwnerDisplayName),
// UserBadges AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount FROM Badges b GROUP BY b.UserId),
// UserStats AS (SELECT u.Id, u.DisplayName, COALESCE(tp.PostCount, 0) AS TopPostsCount, COALESCE(tp.TotalScore, 0) AS TopPostsScore, COALESCE(ub.BadgeCount, 0) AS UserBadges
//     FROM Users u LEFT JOIN TopUserPosts tp ON u.Id = tp.OwnerUserId LEFT JOIN UserBadges ub ON u.Id = ub.UserId WHERE u.Reputation > 1000)
// SELECT us.DisplayName, us.TopPostsCount, us.TopPostsScore, us.UserBadges FROM UserStats us WHERE us.UserBadges > 0 ORDER BY us.TopPostsScore DESC, us.TopPostsCount DESC;
fn q9913(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, .. } = &db.post;
    let w = db
        .post
        .with(post_type_id.eq(1).and(score.gt(0)))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(score))
        .window(row_number, |(p, s)| (Reverse(s), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let tu = (&tp).group_by(owner_user).select(score).fold((0i64, 0i64), |(n, s), x| (n + 1, s + x));
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let mut v = drain(db.user.with((&db.user.reputation).gt(1000)).select((&tu).opt().and(&bc)));
    v.sort_by_key(|&(_, (t, _))| {
        let (n, s) = t.unwrap_or((0, 0));
        (Reverse(s), Reverse(n))
    });
    rows(v.into_iter().map(|(u, (t, b))| {
        let (n, s) = t.unwrap_or((0, 0));
        row(vec![user_col(db, u, "name"), V::I(n), V::I(s), V::I(b)])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, u.DisplayName AS OwnerDisplayName, COUNT(a.Id) AS AnswerCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN Posts a ON p.Id = a.ParentId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId, u.DisplayName),
// MostActiveUsers AS (SELECT OwnerUserId, COUNT(PostId) AS PostCount, SUM(UpVoteCount) AS TotalUpVotes, SUM(DownVoteCount) AS TotalDownVotes
//     FROM RankedPosts GROUP BY OwnerUserId ORDER BY PostCount DESC LIMIT 10)
// SELECT u.Id AS UserId, u.DisplayName, u.Reputation, mu.PostCount, mu.TotalUpVotes, mu.TotalDownVotes
// FROM Users u JOIN MostActiveUsers mu ON u.Id = mu.OwnerUserId ORDER BY mu.TotalUpVotes DESC, mu.TotalDownVotes ASC;
//
// The NULL owner group is one of the ten busiest and drops at the join.
fn q6547(db: &'static So) -> String {
    let Post { post_type_id, owner_user_id, .. } = &db.post;
    let pc = db.post.with(post_type_id.eq(1)).group_by(owner_user_id.opt()).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let top = top_n(drain(&pc), |&(u, n)| (Reverse(n), u), 10);
    let uids: HashIdx<Option<i64>, Id<User>> = (&db.user.origid).map(Some).inv().collect();
    let tu: MatSet<Id<User>> = rel(top).map(|(u, _)| u).select(&uids).collect();
    let s = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).with(post_type_id.eq(1)).select(children_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())))
        .fold((0i64, 0i64), |(u, d), (_, t)| (u + (t == Some(2)) as i64, d + (t == Some(3)) as i64));
    let n = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).with(post_type_id.eq(1))).fold(0i64, |n, _| n + 1);
    let mut v = drain((&s).and(&n));
    v.sort_by_key(|&(_, ((u, d), _))| (Reverse(u), d));
    rows(v.into_iter().map(|(u, ((up, dn), n))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(n), V::I(up), V::I(dn)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.Score, p.CreationDate, u.DisplayName AS OwnerDisplayName,
//        RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS RankScore, COUNT(c.Id) AS CommentCount, COUNT(v.Id) AS VoteCount
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.ViewCount, p.Score, p.CreationDate, u.DisplayName, p.PostTypeId),
// TopPosts AS (SELECT PostId, Title, ViewCount, Score, CreationDate, OwnerDisplayName, RankScore, CommentCount, VoteCount FROM RankedPosts WHERE RankScore <= 5)
// SELECT tp.PostId, tp.Title, tp.ViewCount, tp.Score, tp.CreationDate, tp.OwnerDisplayName, tp.CommentCount, tp.VoteCount, pt.Name AS PostType
// FROM TopPosts tp JOIN PostTypes pt ON tp.RankScore = pt.Id ORDER BY tp.Score DESC, tp.ViewCount DESC;
fn q8087(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, view_count, .. } = &db.post;
    let key = |(s, w): (i64, Option<i64>)| (Reverse(s), w.is_none(), Reverse(w));
    let w = db
        .post
        .with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score.and(view_count.opt())))
        .window(rank, move |(_, sv)| key(sv), asc);
    let pts: HashIdx<i64, Id<PostType>> = (&db.post_type.origid).inv().collect();
    type T = ((Id<Post>, (i64, Option<i64>)), i64);
    let tp: MatSet<T> = (&w).filt(|(_, r)| r <= 5).collect();
    let tpp: MatSet<Id<Post>> = (&tp).map(|((p, _), _)| p).collect();
    let s = (&tpp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).opt()))
        .fold((0i64, 0i64), |(n, m), (c, v)| (n + c.is_some() as i64, m + v.is_some() as i64));
    let mut v = drain((&tp).select(Same::<T>::new().map(|((p, _), _): T| p).select(&s).and(Same::<T>::new().map(|(_, r): T| r).select(&pts))));
    v.sort_by_key(|&(((_, sv), _), _)| key(sv));
    rows(v.into_iter().map(|(((p, _), _), ((n, m), t))| {
        let mut f = post_fields(db, p, &["id", "title", "views", "score", "created", "owner"]);
        f.extend([V::I(n), V::I(m), V::S(db.post_type.name.get(t).unwrap())]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.CreationDate, p.Score, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.Score > 0),
// TopPosts AS (SELECT rp.* FROM RankedPosts rp WHERE rp.rn = 1),
// QuestionWithComments AS (SELECT tp.PostId, tp.Title, tp.ViewCount, tp.CreationDate, tp.Score, tp.OwnerDisplayName, COUNT(c.Id) AS CommentCount
//     FROM TopPosts tp LEFT JOIN Comments c ON tp.PostId = c.PostId GROUP BY tp.PostId, tp.Title, tp.ViewCount, tp.CreationDate, tp.Score, tp.OwnerDisplayName)
// SELECT qwc.PostId, qwc.Title, qwc.ViewCount, qwc.CreationDate, qwc.Score, qwc.OwnerDisplayName,
//        CASE WHEN qwc.CommentCount > 10 THEN 'Highly Engaged' WHEN qwc.CommentCount > 5 THEN 'Moderately Engaged' ELSE 'Less Engaged' END AS EngagementLevel
// FROM QuestionWithComments qwc ORDER BY qwc.ViewCount DESC LIMIT 10;
fn q5590(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, creation_date, view_count, .. } = &db.post;
    let w = db
        .post
        .with(post_type_id.eq(1).and(score.gt(0)))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(_, d)| d, desc);
    let top = drain((&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p));
    let top = top_n(top, |&(_, p)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w))
    }, 10);
    let tp: MatSet<Id<Post>> = rel(top.iter().map(|x| x.1).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    rows(top.into_iter().map(|(_, p)| {
        let n = cc.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "views", "created", "score", "owner"]);
        f.push(V::S(if n > 10 { "Highly Engaged" } else if n > 5 { "Moderately Engaged" } else { "Less Engaged" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.AnswerCount, p.ViewCount, p.OwnerUserId,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS OwnerRank FROM Posts p WHERE p.PostTypeId = 1 AND p.Score > 0),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(p.Score) AS TotalScore FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.Reputation),
// PostHistories AS (SELECT ph.PostId, COUNT(*) AS HistoryCount, MAX(ph.CreationDate) AS LastEditDate FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (4, 5) GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, ur.UserId, ur.Reputation, ur.PostCount, ur.TotalScore, ph.HistoryCount, ph.LastEditDate
// FROM RankedPosts rp JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId LEFT JOIN PostHistories ph ON rp.PostId = ph.PostId
// WHERE rp.OwnerRank = 1 ORDER BY ur.TotalScore DESC, rp.Score DESC;
fn q7516(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, creation_date, .. } = &db.post;
    let w = db
        .post
        .with(post_type_id.eq(1).and(score.gt(0)))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(_, d)| d, desc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let ups = user_posts(db);
    let PostHistory { post, post_history_type_id, .. } = &db.post_history;
    let ph = db.post_history.with(post_history_type_id.is_in([4, 5])).group_by(post).select(&db.post_history.creation_date).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let mut v = drain((&tp).select(Ident::<Post>::new().and(owner_user.select(Ident::<User>::new().and(&ups))).and((&ph).opt())));
    v.sort_by_key(|&(_, ((p, (_, a)), _))| (Reverse(a[4]), Reverse(score.get(p).unwrap())));
    rows(v.into_iter().map(|(_, ((p, (u, a)), h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend(ucols(db, u, &["uid", "rep"]));
        f.extend([V::I(a[1]), V::I(a[4])]);
        f.extend(match h {
            Some((n, m)) => [V::I(n), V::T(m)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 YEAR'),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.OwnerDisplayName FROM RankedPosts rp WHERE rp.PostRank <= 5),
// PostInteractions AS (SELECT p.Id AS PostId, COUNT(c.Id) AS CommentCount, COUNT(v.Id) AS VoteCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId
//     LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.Id IN (SELECT PostId FROM TopPosts) GROUP BY p.Id)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.OwnerDisplayName, pi.CommentCount, pi.VoteCount
// FROM TopPosts tp JOIN PostInteractions pi ON tp.PostId = pi.PostId ORDER BY tp.Score DESC, pi.VoteCount DESC;
fn q8804(db: &'static So) -> String {
    let Post { creation_date, owner_user, post_type_id, score, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(_, d)| d, desc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).opt()))
        .fold((0i64, 0i64), |(n, m), (c, v)| (n + c.is_some() as i64, m + v.is_some() as i64));
    let mut v = drain(&s);
    v.sort_by_key(|&(p, (_, m))| (Reverse(score.get(p).unwrap()), Reverse(m)));
    rows(v.into_iter().map(|(p, (n, m))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend([V::I(n), V::I(m)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank, p.OwnerUserId FROM Posts p WHERE p.PostTypeId = 1),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostActivity AS (SELECT p.Id AS PostId, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, pa.CommentCount, pa.UpVoteCount, pa.DownVoteCount, ub.BadgeCount, rp.Rank
// FROM RankedPosts rp JOIN UserBadges ub ON rp.OwnerUserId = ub.UserId JOIN PostActivity pa ON rp.PostId = pa.PostId WHERE rp.Rank <= 5
// ORDER BY ub.BadgeCount DESC, rp.ViewCount DESC;
fn q9078(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, creation_date, view_count, .. } = &db.post;
    let w = db
        .post
        .with(post_type_id.eq(1))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((_, s), d)| (Reverse(s), Reverse(d)), asc);
    type T = (Id<Post>, i64);
    let tp: MatSet<T> = (&w).filt(|(_, r)| r <= 5).map(|(((p, _), _), r)| (p, r)).collect();
    let tpp: MatSet<Id<Post>> = (&tp).map(|(p, _)| p).collect();
    let s = (&tpp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let mut v = drain((&tp).select(Same::<T>::new().and(Same::<T>::new().map(|(p, _): T| p).select(owner_user).select(&bc))));
    v.sort_by_key(|&(_, ((p, _), b))| {
        let w = view_count.get(p);
        (Reverse(b), w.is_none(), Reverse(w))
    });
    rows(v.into_iter().map(|(_, ((p, r), b))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views"]);
        f.extend(s.get(p).unwrap().map(V::I));
        f.extend([V::I(b), V::I(r)]);
        row(f)
    }))
}

// WITH UserPosts AS (SELECT u.Id AS UserId, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN p.PostTypeId = 3 THEN 1 ELSE 0 END) AS WikiCount,
//        SUM(CASE WHEN p.PostTypeId = 4 THEN 1 ELSE 0 END) AS TagWikiExcerptCount, SUM(CASE WHEN p.PostTypeId = 5 THEN 1 ELSE 0 END) AS TagWikiCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id),
// UserReputation AS (SELECT Id AS UserId, Reputation FROM Users),
// AggregatedData AS (SELECT u.UserId, ur.Reputation, u.PostCount, u.QuestionCount, u.AnswerCount, u.WikiCount, u.TagWikiExcerptCount, u.TagWikiCount,
//        RANK() OVER (ORDER BY ur.Reputation DESC) AS ReputationRank FROM UserPosts u JOIN UserReputation ur ON u.UserId = ur.UserId)
// SELECT UserId, Reputation, PostCount, QuestionCount, AnswerCount, WikiCount, TagWikiExcerptCount, TagWikiCount, ReputationRank
// FROM AggregatedData ORDER BY ReputationRank FETCH FIRST 100 ROWS ONLY;
fn q13019(db: &'static So) -> String {
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(&db.post.post_type_id).opt())
        .fold([0i64; 6], |a, t| match t {
            Some(t) => {
                let mut a = a;
                a[0] += 1;
                if (1..=5).contains(&t) {
                    a[t as usize] += 1;
                }
                a
            }
            None => a,
        });
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&s).and(&db.user.reputation)).window(rank, |(_, r)| r, desc);
    let v = top_n(drain(&w).into_iter().map(|x| x.1).collect(), |&(_, r)| r, 100);
    rows(v.into_iter().map(|(((u, a), _), r)| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend(a.map(V::I));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.CreationDate, u.DisplayName AS Author, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     GROUP BY p.Id, p.Title, p.ViewCount, p.CreationDate, u.DisplayName, p.PostTypeId),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.ViewCount, rp.CreationDate, rp.Author, rp.CommentCount, rp.UpVotes, rp.DownVotes FROM RankedPosts rp
//     WHERE rp.Rank <= 10 AND rp.ViewCount > 50)
// SELECT fp.PostId, fp.Title, fp.Author, fp.ViewCount, fp.CommentCount, fp.UpVotes, fp.DownVotes, CASE WHEN fp.UpVotes > fp.DownVotes THEN 'Positive' ELSE 'Negative' END AS Sentiment
// FROM FilteredPosts fp ORDER BY fp.ViewCount DESC, fp.CreationDate ASC;
//
// The newest ten posts of each type all have 50 views or fewer.
fn q5690(db: &'static So) -> String {
    let Post { owner_user, post_type_id, creation_date, view_count, .. } = &db.post;
    let w = db
        .post
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(_, d)| d, desc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|((p, _), _)| p).collect();
    let fp = (&tp).with(view_count.gt(50));
    let s = (&fp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let mut v = drain(&s);
    v.sort_by_key(|&(p, _)| (Reverse(view_count.get(p)), creation_date.get(p).unwrap()));
    rows(v.into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "views"]);
        f.extend(a.map(V::I));
        f.push(V::S(if a[1] > a[2] { "Positive" } else { "Negative" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, u.DisplayName AS OwnerDisplayName,
//        RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS RankScore
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.Score > 0)
// SELECT r.PostId, r.Title, r.CreationDate, r.Score, r.ViewCount, r.AnswerCount, r.CommentCount, r.OwnerDisplayName, COALESCE(pht.Name, 'N/A') AS PostHistoryType,
//        COUNT(DISTINCT ph.Id) AS RevisionCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
// FROM RankedPosts r LEFT JOIN PostHistory ph ON r.PostId = ph.PostId LEFT JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id LEFT JOIN Votes v ON r.PostId = v.PostId
// WHERE r.RankScore <= 5
// GROUP BY r.PostId, r.Title, r.CreationDate, r.Score, r.ViewCount, r.AnswerCount, r.CommentCount, r.OwnerDisplayName, pht.Name ORDER BY r.Score DESC, r.ViewCount DESC;
fn q8616(db: &'static So) -> String {
    let Post { creation_date, score, owner_user, post_type_id, view_count, .. } = &db.post;
    let key = |p: Id<Post>| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    };
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(score.gt(0)))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score.and(view_count.opt())))
        .window(rank, |(_, (s, w))| (Reverse(s), w.is_none(), Reverse(w)), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    type Row = (Id<Post>, Option<(Id<PostHistory>, Str)>, Option<(Id<Vote>, i64)>);
    let joined: MatSet<Row> = (&tp)
        .select(Ident::<Post>::new().and(history_of(db).select(Ident::<PostHistory>::new().and(htype_name(db))).opt()).and(votes_of(db).select(Ident::<Vote>::new().and(&db.vote.vote_type_id)).opt()))
        .map(|((p, h), v)| (p, h, v))
        .collect();
    let g = || (&joined).group_by(Same::<Row>::new().map(|(p, h, _): Row| (p, h.map(|x| x.1))));
    let ud = g().select(Same::<Row>::new().map(|(_, _, v): Row| v.map(|x| x.1))).fold((0i64, 0i64), |(u, d), t| (u + (t == Some(2)) as i64, d + (t == Some(3)) as i64));
    let dh = g().select(Same::<Row>::new().flat_map(|(_, h, _): Row| h.map(|x| x.0))).count_distinct();
    let mut v = drain((&ud).and((&dh).opt()));
    v.sort_by_key(|&((p, _), _)| key(p));
    rows(v.into_iter().map(|((p, n), ((u, d), h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "owner"]);
        f.extend([V::S(n.unwrap_or("N/A")), V::I(h.unwrap_or(0)), V::I(u), V::I(d)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, u.DisplayName AS Author, p.CreationDate,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 YEAR' AND p.PostTypeId IN (1, 2)),
// UserBadges AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount FROM Badges b GROUP BY b.UserId),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, ub.BadgeCount, ROW_NUMBER() OVER (ORDER BY u.Reputation DESC, ub.BadgeCount DESC) AS UserRank
//     FROM Users u LEFT JOIN UserBadges ub ON u.Id = ub.UserId WHERE u.Reputation > 0)
// SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.Author, rp.CreationDate, tu.DisplayName AS TopUser, tu.Reputation, tu.BadgeCount
// FROM RankedPosts rp JOIN TopUsers tu ON rp.Author = tu.DisplayName WHERE rp.Rank <= 5 AND tu.UserRank <= 10 ORDER BY rp.Score DESC, rp.ViewCount DESC;
fn q8857(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, score, view_count, .. } = &db.post;
    let key = |p: Id<Post>| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    };
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.is_in([1, 2])))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score.and(view_count.opt())))
        .window(row_number, |(p, (s, w))| (Reverse(s), w.is_none(), Reverse(w), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let us = drain(db.user.with((&db.user.reputation).gt(0)).select((&db.user.reputation).and((&bc).opt())));
    let tu = top_n(us, |&(u, (r, b))| (Reverse(r), b.is_none(), Reverse(b), u), 10);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let by_name: HashIdx<Str, Id<User>> = (&tu).select(&db.user.display_name).inv().collect();
    let mut v = drain((&tp).select(Ident::<Post>::new().and(owner_user.select(&db.user.display_name).select(&by_name))));
    v.sort_by_key(|&(_, (p, _))| key(p));
    rows(v.into_iter().map(|(_, (p, u))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "owner", "created"]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.push(bc.get(u).map_or(V::Null, V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, U.DisplayName AS OwnerDisplayName, P.Score, P.ViewCount, P.CreationDate,
//        ROW_NUMBER() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC, P.ViewCount DESC) AS Rank
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id WHERE P.CreationDate >= CURRENT_TIMESTAMP - INTERVAL '1 year' AND P.Score >= 10),
// RecentBadges AS (SELECT B.UserId, COUNT(B.Id) AS BadgeCount FROM Badges B WHERE B.Date >= CURRENT_TIMESTAMP - INTERVAL '6 months' GROUP BY B.UserId),
// ActiveUsers AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, R.BadgeCount FROM Users U LEFT JOIN RecentBadges R ON U.Id = R.UserId
//     WHERE U.LastAccessDate >= CURRENT_TIMESTAMP - INTERVAL '3 months')
// SELECT RP.PostId, RP.Title, RP.OwnerDisplayName, RP.Score, RP.ViewCount, RP.CreationDate, AU.DisplayName AS ActiveUser, AU.Reputation, AU.BadgeCount
// FROM RankedPosts RP JOIN ActiveUsers AU ON RP.OwnerDisplayName = AU.DisplayName WHERE RP.Rank <= 5 ORDER BY RP.Score DESC, RP.ViewCount DESC;
//
// CURRENT_TIMESTAMP is when the query runs; the data ends in 2024, so it is
// empty. The comparisons are TIMESTAMPTZ, so the dates are read as New York
// local time and the intervals are counted on its calendar.
fn q8652(db: &'static So) -> String {
    let now = utc_to_ny(now_utc());
    let since = move |m: i64| ny_to_utc(add_months(now, -m));
    let Post { creation_date, score, owner_user, post_type_id, view_count, .. } = &db.post;
    let key = |p: Id<Post>| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    };
    let w = db
        .post
        .with(creation_date.filt(move |d| ny_to_utc(d) >= since(12)).and(score.ge(10)))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score.and(view_count.opt())))
        .window(row_number, |(p, (s, w))| (Reverse(s), w.is_none(), Reverse(w), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let rb = db.badge.with((&db.badge.date).filt(move |d| ny_to_utc(d) >= since(6))).group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let au: HashIdx<Str, Id<User>> = db.user.with((&db.user.last_access_date).filt(move |d| ny_to_utc(d) >= since(3))).select(&db.user.display_name).inv().collect();
    let mut v = drain((&tp).select(Ident::<Post>::new().and(owner_user.select(&db.user.display_name).select(&au))));
    v.sort_by_key(|&(_, (p, _))| key(p));
    rows(v.into_iter().map(|(_, (p, u))| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "score", "views", "created"]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.push(rb.get(u).map_or(V::Null, V::I));
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(p.ViewCount) AS TotalViews, SUM(p.Score) AS TotalScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT ..., RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank, RANK() OVER (ORDER BY TotalPosts DESC) AS PostRank FROM UserPostStats)
// SELECT tu.DisplayName, tu.TotalPosts, tu.Questions, tu.Answers, tu.TotalViews, tu.TotalScore, pht.Name AS RecentActivity, COUNT(ph.Id) AS ActivityCount
// FROM TopUsers tu LEFT JOIN PostHistory ph ON tu.UserId = ph.UserId JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id
// WHERE tu.ScoreRank <= 10 OR tu.PostRank <= 10
// GROUP BY tu.UserId, tu.DisplayName, tu.TotalPosts, tu.Questions, tu.Answers, tu.TotalViews, tu.TotalScore, pht.Name ORDER BY tu.TotalScore DESC, tu.TotalPosts DESC;
fn q9717(db: &'static So) -> String {
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
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&s)).window(rank, |(_, a)| (a[0] == 0, Reverse(a[5])), asc);
    let w = (&w).window(rank, |((_, a), _)| Reverse(a[0]), asc);
    type U = (Id<User>, [i64; 6]);
    let tu: MatSet<U> = (&w).filt(|((_, s), p)| s <= 10 || p <= 10).map(|(((u, a), _), _)| (u, a)).collect();
    let by_user: HashIdx<Id<User>, Id<PostHistory>> = db.post_history.select(&db.post_history.user).inv().collect();
    type R = (U, (Id<PostHistory>, Str));
    let hr: MatSet<R> = (&tu).select(Same::<U>::new().and(Same::<U>::new().map(|(u, _): U| u).select((&by_user).select(Ident::<PostHistory>::new().and(htype_name(db)))))).collect();
    let g = (&hr).group_by(Same::<R>::new().map(|(x, (_, n)): R| (x, n))).select(Same::<R>::new()).fold(0i64, |n, _| n + 1);
    let mut v = drain(&g);
    v.sort_by_key(|&(((_, a), _), _)| (a[0] == 0, Reverse(a[5]), Reverse(a[0])));
    rows(v.into_iter().map(|(((u, a), n), c)| {
        row(vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), nullable(a[4], a[3]), nullable(a[5], a[0]), V::S(n), V::I(c)])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.OwnerDisplayName FROM RankedPosts rp WHERE rp.Rank <= 5),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c GROUP BY c.PostId),
// MaxComments AS (SELECT pc.PostId, pc.CommentCount, ROW_NUMBER() OVER (ORDER BY pc.CommentCount DESC) AS CommentRank FROM PostComments pc)
// SELECT tp.Title, tp.OwnerDisplayName, tp.CreationDate, tp.Score, tp.ViewCount, mc.CommentCount
// FROM TopPosts tp LEFT JOIN MaxComments mc ON tp.PostId = mc.PostId WHERE mc.CommentRank <= 5 ORDER BY tp.CreationDate DESC;
//
// The five most commented posts are all older than a year, so it is empty.
fn q5171(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, .. } = &db.post;
    let w = db
        .post
        .with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(score))
        .window(row_number, |(p, s)| (Reverse(s), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let pc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let mc: MatSet<Id<Post>> = rel(top_n(drain(&pc), |&(p, n)| (Reverse(n), p), 5).into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let mut v = drain((&tp).select(Ident::<Post>::new().with(&mc).and(&pc)));
    v.sort_by_key(|&(_, (p, _))| Reverse(creation_date.get(p).unwrap()));
    rows(v.into_iter().map(|(_, (p, n))| {
        let mut f = post_fields(db, p, &["title", "owner", "created", "score", "views"]);
        f.push(V::I(n));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.OwnerUserId,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank FROM Posts p WHERE p.PostTypeId = 1),
// RecentUserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, MAX(b.Date) AS LastBadgeDate FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// ClosedPosts AS (SELECT ph.PostId, COUNT(ph.Id) AS CloseCount FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, COALESCE(cb.CloseCount, 0) AS TotalClosed, r.UserId AS BadgeUserId, r.BadgeCount, r.LastBadgeDate
// FROM RankedPosts rp LEFT JOIN ClosedPosts cb ON rp.PostId = cb.PostId LEFT JOIN RecentUserBadges r ON rp.OwnerUserId = r.UserId
// WHERE rp.PostRank = 1 AND rp.Score > (SELECT AVG(Score) FROM Posts WHERE PostTypeId = 1) ORDER BY rp.CreationDate DESC, rp.Score DESC LIMIT 100;
fn q31702(db: &'static So) -> String {
    let Post { post_type_id, owner_user, owner_user_id, creation_date, score, .. } = &db.post;
    let (n, t) = db.post.with(post_type_id.eq(1)).select(score).fold_flat((0i64, 0i64), |(n, t), s| (n + 1, t + s));
    let mean = t as f64 / n as f64;
    let w = db
        .post
        .with(post_type_id.eq(1))
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let tp = (&tp).with(score.filt(move |s| s as f64 > mean));
    let v = top_n(drain(tp.select(creation_date)), |&(p, d)| (Reverse(d), Reverse(score.get(p).unwrap())), 100);
    let v = rel(v.into_iter().map(|x| x.0).collect());
    let cb = db.post_history.with((&db.post_history.post_history_type_id).eq(10)).group_by(&db.post_history.post).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let rb = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.date).opt()).fold((0i64, i64::MIN), |(n, m), d| match d {
        Some(d) => (n + 1, m.max(d)),
        None => (n, m),
    });
    let mut out = drain((&v).select(Ident::<Post>::new().and((&cb).opt()).and(owner_user.select(Ident::<User>::new().and(&rb)).opt())));
    out.sort_by_key(|x| x.0);
    rows(out.into_iter().map(|(_, ((p, c), r))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.push(V::I(c.unwrap_or(0)));
        f.extend(match r {
            Some((u, (n, m))) => [user_col(db, u, "uid"), V::I(n), tmax(m)],
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserReputation AS (SELECT Id AS UserId, Reputation, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM Users),
// TopPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, COALESCE(p.AcceptedAnswerId, 0) AS AcceptedAnswerId, u.DisplayName AS OwnerDisplayName,
//        u.Reputation AS OwnerReputation, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS PostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.Score > 0),
// PostJoin AS (SELECT tp.PostId, tp.Title, tp.Score, tp.ViewCount, tp.OwnerDisplayName, tr.Reputation AS UserReputation
//     FROM TopPosts tp JOIN UserReputation tr ON tr.UserId = tp.AcceptedAnswerId WHERE tp.PostRank <= 5),
// PostComments AS (SELECT p.Id AS PostId, COUNT(c.Id) AS CommentCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY p.Id)
// SELECT p.Title, p.Score, p.ViewCount, p.OwnerDisplayName, p.UserReputation, COALESCE(pc.CommentCount, 0) AS CommentCount
// FROM PostJoin p LEFT JOIN PostComments pc ON p.PostId = pc.PostId ORDER BY p.Score DESC, p.ViewCount DESC;
fn q9011(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, accepted_answer_id, view_count, .. } = &db.post;
    let top = top_n(drain(db.post.with(post_type_id.eq(1).and(score.gt(0))).with(owner_user).select(score)), |&(p, s)| (Reverse(s), p), 5);
    let tp = rel(top.into_iter().map(|x| x.0).collect());
    let uids: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let aid = accepted_answer_id.opt().map(|a: Option<i64>| a.unwrap_or(0));
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let mut v = drain((&tp).select(Ident::<Post>::new().and(aid.select(&uids)).and(&cc)));
    v.sort_by_key(|&(_, ((p, _), _))| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    });
    rows(v.into_iter().map(|(_, ((p, u), c))| {
        let mut f = post_fields(db, p, &["title", "score", "views", "owner"]);
        f.extend([user_col(db, u, "rep"), V::I(c)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName,
//        RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS RankScore, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount, p.PostTypeId
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName, p.PostTypeId),
// FilteredPosts AS (SELECT rp.*, pt.Name AS PostTypeName FROM RankedPosts rp JOIN PostTypes pt ON rp.PostTypeId = pt.Id WHERE rp.RankScore <= 5)
// SELECT fp.PostId, fp.Title, fp.PostTypeName, fp.CreationDate, fp.Score, fp.ViewCount, fp.CommentCount, fp.UpvoteCount, fp.DownvoteCount, fp.OwnerDisplayName
// FROM FilteredPosts fp ORDER BY fp.PostTypeName, fp.RankScore;
fn q9087(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, .. } = &db.post;
    let w = db.post.group_by(post_type_id).select(Ident::<Post>::new().and(score).and(creation_date)).window(rank, |((_, s), d)| (Reverse(s), Reverse(d)), asc);
    type T = (Id<Post>, i64);
    let tp: MatSet<T> = (&w).filt(|(_, r)| r <= 5).map(|(((p, _), _), r)| (p, r)).collect();
    let tpp: MatSet<Id<Post>> = (&tp).map(|(p, _)| p).collect();
    let s = (&tpp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let mut v = drain((&tp).select(Same::<T>::new().and(Same::<T>::new().map(|(p, _): T| p).select(ptype_name(db)))));
    v.sort_by_key(|&(_, ((_, r), n))| (n, r));
    rows(v.into_iter().map(|(_, ((p, _), n))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.push(V::S(n));
        f.extend(post_fields(db, p, &["created", "score", "views"]));
        f.extend(s.get(p).unwrap().map(V::I));
        f.extend(post_fields(db, p, &["owner"]));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, u.DisplayName),
// FilteredPosts AS (SELECT PostId, Title, CreationDate, OwnerDisplayName, CommentCount, UpVoteCount, DownVoteCount FROM RankedPosts WHERE rn = 1)
// SELECT MIN(PostId) AS MinPostId, MAX(PostId) AS MaxPostId, COUNT(*) AS TotalPosts, SUM(CommentCount) AS TotalComments, SUM(UpVoteCount) AS TotalUpVotes,
//        SUM(DownVoteCount) AS TotalDownVotes, AVG(UpVoteCount - DownVoteCount) AS AverageVoteBalance
// FROM FilteredPosts GROUP BY OwnerDisplayName ORDER BY AverageVoteBalance DESC LIMIT 10;
fn q27811(db: &'static So) -> String {
    let Post { post_type_id, owner_user, origid, .. } = &db.post;
    let pa = db
        .post
        .with(post_type_id.eq(1))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let g = db
        .post
        .with(post_type_id.eq(1))
        .group_by(owner_user.select(&db.user.display_name).opt())
        .select(origid.and(&pa))
        .fold([i64::MAX, i64::MIN, 0, 0, 0, 0], |a, (id, x)| [a[0].min(id), a[1].max(id), a[2] + 1, a[3] + x[0], a[4] + x[1], a[5] + x[2]]);
    let mut v = drain(&g);
    v.sort_by(|x, y| ((y.1[4] - y.1[5]) as f64 / y.1[2] as f64).total_cmp(&((x.1[4] - x.1[5]) as f64 / x.1[2] as f64)));
    v.truncate(10);
    rows(v.into_iter().map(|(_, a)| row(vec![V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4]), V::I(a[5]), avg(a[4] - a[5], a[2])])))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes, u.Reputation, u.CreationDate, u.LastAccessDate
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName, u.Reputation, u.CreationDate, u.LastAccessDate),
// TopUsers AS (SELECT ..., RANK() OVER (ORDER BY TotalUpvotes - TotalDownvotes DESC) AS Rank FROM UserPostStats)
// SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalUpvotes, TotalDownvotes, Reputation, CreationDate, LastAccessDate
// FROM TopUsers WHERE Rank <= 10 ORDER BY Rank;
fn q8880(db: &'static So) -> String {
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 5], |a, p| match p {
            Some((t, v)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (v == Some(2)) as i64, a[4] + (v == Some(3)) as i64],
            None => a,
        });
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&s)).window(rank, |(_, a)| Reverse(a[3] - a[4]), asc);
    let mut v: Vec<_> = drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|x| x.1).collect();
    v.sort_by_key(|&(_, r)| r);
    rows(v.into_iter().map(|((u, a), _)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        f.extend(ucols(db, u, &["rep", "ucreated", "last_access"]));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, U.DisplayName AS OwnerDisplayName, P.Score, P.ViewCount,
//        RANK() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC, P.CreationDate DESC) AS ScoreRank
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT PostId, Title, OwnerDisplayName, Score, ViewCount FROM RankedPosts WHERE ScoreRank <= 10),
// PostComments AS (SELECT C.PostId, COUNT(C.Id) AS CommentCount FROM Comments C GROUP BY C.PostId),
// PostBadges AS (SELECT B.UserId, COUNT(B.Id) AS BadgeCount FROM Badges B GROUP BY B.UserId)
// SELECT TP.PostId, TP.Title, TP.OwnerDisplayName, TP.Score, TP.ViewCount, PC.CommentCount, PB.BadgeCount
// FROM TopPosts TP LEFT JOIN PostComments PC ON TP.PostId = PC.PostId
// LEFT JOIN PostBadges PB ON (TP.OwnerDisplayName = (SELECT U.DisplayName FROM Users U WHERE U.Id = PB.UserId LIMIT 1))
// ORDER BY TP.Score DESC, TP.ViewCount DESC;
fn q7826(db: &'static So) -> String {
    let Post { creation_date, owner_user, post_type_id, score, view_count, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(rank, |((_, s), d)| (Reverse(s), Reverse(d)), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|(((p, _), _), _)| p).collect();
    let pc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let bu: MatSet<Id<User>> = db.badge.select(&db.badge.user).collect();
    let by_name: HashIdx<Str, Id<User>> = (&bu).select(&db.user.display_name).inv().collect();
    let mut v = drain((&tp).select(Ident::<Post>::new().and((&pc).opt()).and(owner_user.select(&db.user.display_name).select(&by_name).select(&bc).opt())));
    v.sort_by_key(|&(_, ((p, _), _))| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    });
    rows(v.into_iter().map(|(_, ((p, c), b))| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "score", "views"]);
        f.extend([c.map_or(V::Null, V::I), b.map_or(V::Null, V::I)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.PostTypeId, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        COUNT(v.Id) FILTER (WHERE vt.Name = 'UpMod') AS UpVotes, COUNT(v.Id) FILTER (WHERE vt.Name = 'DownMod') AS DownVotes,
//        RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS RankByScore
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN VoteTypes vt ON v.VoteTypeId = vt.Id
//     GROUP BY p.Id, p.Title, p.Score, p.CreationDate, p.PostTypeId, u.DisplayName),
// TopPosts AS (SELECT PostId, Title, Score, CreationDate, OwnerDisplayName, CommentCount, UpVotes, DownVotes FROM RankedPosts WHERE RankByScore <= 10)
// SELECT tp.Title, tp.OwnerDisplayName, tp.Score, tp.CommentCount, (tp.UpVotes - tp.DownVotes) AS NetVotes,
//        CASE WHEN tp.Score >= 100 THEN 'High Performer' WHEN tp.Score BETWEEN 50 AND 99 THEN 'Average Performer' ELSE 'Low Performer' END AS PerformanceCategory
// FROM TopPosts tp ORDER BY tp.Score DESC;
fn q7118(db: &'static So) -> String {
    let Post { owner_user, post_type_id, score, .. } = &db.post;
    let w = db.post.with(owner_user).group_by(post_type_id).select(Ident::<Post>::new().and(score)).window(rank, |(_, s)| Reverse(s), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|((p, _), _)| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(vtype_name(db)).opt()))
        .fold((0i64, 0i64), |(n, net), (c, t)| (n + c.is_some() as i64, net + (t == Some("UpMod")) as i64 - (t == Some("DownMod")) as i64));
    let mut v = drain(&s);
    v.sort_by_key(|&(p, _)| Reverse(score.get(p).unwrap()));
    rows(v.into_iter().map(|(p, (c, n))| {
        let sc = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["title", "owner", "score"]);
        f.extend([V::I(c), V::I(n), V::S(if sc >= 100 { "High Performer" } else if (50..=99).contains(&sc) { "Average Performer" } else { "Low Performer" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS rn,
//        COUNT(c.Id) AS CommentCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.Score, p.ViewCount, p.CreationDate, p.PostTypeId),
// FrequentUsers AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(v.Id) AS VoteCount FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId
//     WHERE u.CreationDate < cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '6 months' GROUP BY u.Id, u.DisplayName),
// TopBadges AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount FROM Badges b GROUP BY b.UserId)
// SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.CreationDate, rp.CommentCount, fu.DisplayName AS FrequentUser, tb.BadgeCount
// FROM RankedPosts rp JOIN FrequentUsers fu ON rp.PostId IN (SELECT PostId FROM Votes v WHERE v.UserId = fu.UserId) LEFT JOIN TopBadges tb ON fu.UserId = tb.UserId
// WHERE rp.rn <= 5 ORDER BY rp.Score DESC, rp.ViewCount DESC, tb.BadgeCount DESC;
//
// None of the top recent posts drew a vote that records its voter, so it is empty.
fn q6328(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, view_count, .. } = &db.post;
    let key = |p: Id<Post>| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    };
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score.and(view_count.opt())))
        .window(row_number, |(p, (s, w))| (Reverse(s), w.is_none(), Reverse(w), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let fu = Ident::<User>::new().with((&db.user.creation_date).lt(add_months(ts(2024, 10, 1, 12, 34, 56), -6)));
    let voters: MatSet<(Id<Post>, Id<User>)> = (&tp).select(Ident::<Post>::new().and(votes_of(db).select((&db.vote.user).select(fu)))).collect();
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let mut v = drain((&voters).select(Same::<(Id<Post>, Id<User>)>::new().map(|(_, u)| u).select(&bc).opt()));
    v.sort_by_key(|&((p, _), b)| (key(p), b.is_none(), Reverse(b)));
    rows(v.into_iter().map(|((p, u), b)| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "created"]);
        f.extend([V::I(cc.get(p).unwrap()), user_col(db, u, "name"), b.map_or(V::Null, V::I)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, p.CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS UserPostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE u.Reputation > 1000 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// UserBadges AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount FROM Badges b GROUP BY b.UserId),
// PostHistoryDetails AS (SELECT h.PostId, COUNT(h.Id) AS EditCount, MAX(h.CreationDate) AS LastEditDate FROM PostHistory h WHERE h.PostHistoryTypeId IN (4, 5, 6) GROUP BY h.PostId)
// SELECT rp.Title AS PostTitle, rp.CreationDate AS PostCreationDate, rp.ViewCount, rp.Score, rp.AnswerCount, rp.CommentCount, ub.BadgeCount, phd.EditCount, phd.LastEditDate
// FROM RankedPosts rp LEFT JOIN UserBadges ub ON rp.PostId = ub.UserId LEFT JOIN PostHistoryDetails phd ON rp.PostId = phd.PostId
// WHERE rp.UserPostRank <= 5 ORDER BY rp.Score DESC, rp.ViewCount DESC;
fn q6164(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, view_count, origid, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(1000))))
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|(((p, _), _), _)| p).collect();
    let bc = db.badge.group_by(&db.badge.user_id).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let PostHistory { post, post_history_type_id, .. } = &db.post_history;
    let phd = db.post_history.with(post_history_type_id.is_in([4, 5, 6])).group_by(post).select(&db.post_history.creation_date).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let mut v = drain((&tp).select(Ident::<Post>::new().and(origid.select(&bc).opt()).and((&phd).opt())));
    v.sort_by_key(|&(_, ((p, _), _))| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    });
    rows(v.into_iter().map(|(_, ((p, b), h))| {
        let mut f = post_fields(db, p, &["title", "created", "views", "score", "answers", "comments"]);
        f.push(b.map_or(V::Null, V::I));
        f.extend(match h {
            Some((n, m)) => [V::I(n), V::T(m)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(p.ViewCount) AS TotalViews, ROW_NUMBER() OVER (ORDER BY u.Reputation DESC) AS Rank
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId WHERE u.Reputation > 500 GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, BadgeCount, QuestionCount, AnswerCount, TotalViews FROM UserReputation WHERE Rank <= 10)
// SELECT tu.DisplayName, tu.Reputation, tu.BadgeCount, tu.QuestionCount, tu.AnswerCount, tu.TotalViews, pt.Name AS PostType, COUNT(p.Id) AS TotalPosts
// FROM TopUsers tu LEFT JOIN Posts p ON tu.UserId = p.OwnerUserId LEFT JOIN PostTypes pt ON p.PostTypeId = pt.Id
// GROUP BY tu.DisplayName, tu.Reputation, tu.BadgeCount, tu.QuestionCount, tu.AnswerCount, tu.TotalViews, pt.Name ORDER BY tu.Reputation DESC, TotalPosts DESC;
//
// The row number only reads Reputation, so the ten users are picked before
// their joined rows are aggregated.
fn q8894(db: &'static So) -> String {
    let top = top_n(drain(db.user.with((&db.user.reputation).gt(500)).select(&db.user.reputation)), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(top.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let Post { post_type_id, view_count, .. } = &db.post;
    let s = (&tu)
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(posts_of(db).select(post_type_id.and(view_count.opt())).opt()))
        .fold([0i64; 5], |a, (b, p)| {
            let (t, w) = p.map_or((None, None), |(t, w)| (Some(t), w));
            [a[0] + b.is_some() as i64, a[1] + (t == Some(1)) as i64, a[2] + (t == Some(2)) as i64, a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0)]
        });
    type R = ((((Id<User>, Str), i64), [i64; 5]), Option<(Id<Post>, Str)>);
    let j: MatSet<R> = (&tu)
        .select(Ident::<User>::new().and(&db.user.display_name).and(&db.user.reputation).and(&s).and(posts_of(db).select(Ident::<Post>::new().and(ptype_name(db))).opt()))
        .collect();
    let g = (&j).group_by(Same::<R>::new().map(|((((_, n), r), a), p): R| (n, r, a, p.map(|x| x.1)))).select(Same::<R>::new()).fold(0i64, |c, (_, p)| c + p.is_some() as i64);
    let mut v = drain(&g);
    v.sort_by_key(|&((_, r, _, _), n)| (Reverse(r), Reverse(n)));
    rows(v.into_iter().map(|((dn, r, a, t), n)| {
        let mut f = vec![V::S(dn), V::I(r)];
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), nullable(a[4], a[3]), t.map_or(V::Null, V::S), V::I(n)]);
        row(f)
    }))
}

// WITH UserVoteCounts AS (SELECT u.Id AS UserId, COUNT(v.Id) AS TotalVotes, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id),
// PostDetails AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN pt.Name = 'Answer' THEN 1 ELSE 0 END) AS AnswerCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId JOIN PostTypes pt ON p.PostTypeId = pt.Id GROUP BY p.Id, p.Title, p.Score, p.ViewCount),
// TopPosts AS (SELECT pd.PostId, pd.Title, pd.Score, pd.ViewCount, pd.CommentCount, pd.AnswerCount, RANK() OVER (ORDER BY pd.Score DESC, pd.ViewCount DESC) AS Rank FROM PostDetails pd)
// SELECT up.UserId, up.TotalVotes, up.UpVotes, up.DownVotes, tp.Title, tp.Score, tp.ViewCount, tp.CommentCount, tp.AnswerCount
// FROM UserVoteCounts up JOIN TopPosts tp ON up.TotalVotes > 0 WHERE tp.Rank <= 10 ORDER BY up.TotalVotes DESC, tp.Score DESC;
fn q9458(db: &'static So) -> String {
    let Post { score, view_count, .. } = &db.post;
    let key = |p: Id<Post>| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    };
    let w = whole(db.post.iq()).select(Ident::<Post>::new().and(score.and(view_count.opt()))).window(rank, |(_, (s, w))| (Reverse(s), w.is_none(), Reverse(w)), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|((p, _), _)| p).collect();
    let pd = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(ptype_name(db)))
        .fold((0i64, 0i64), |(n, a), (c, t)| (n + c.is_some() as i64, a + (t == "Answer") as i64));
    let uv = db.vote.group_by(&db.vote.user).select(&db.vote.vote_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64]);
    let mut v = drain((&uv).filt(|a| a[0] > 0).cross(&pd));
    v.sort_by_key(|&((u, p), (a, _))| (Reverse(a[0]), key(p), u));
    rows(v.into_iter().map(|((u, p), (a, (n, x)))| {
        let mut f = vec![user_col(db, u, "uid")];
        f.extend(a.map(V::I));
        f.extend(post_fields(db, p, &["title", "score", "views"]));
        f.extend([V::I(n), V::I(x)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostID, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= '2022-01-01' AND p.PostTypeId IN (1, 2)),
// TopRankedPosts AS (SELECT rp.PostID, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.OwnerDisplayName FROM RankedPosts rp WHERE rp.Rank <= 5)
// SELECT trp.PostID, trp.Title, trp.CreationDate, trp.Score, trp.ViewCount, trp.OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
// FROM TopRankedPosts trp LEFT JOIN Comments c ON trp.PostID = c.PostId LEFT JOIN Votes v ON trp.PostID = v.PostId
// GROUP BY trp.PostID, trp.Title, trp.CreationDate, trp.Score, trp.ViewCount, trp.OwnerDisplayName ORDER BY trp.Score DESC, trp.ViewCount DESC;
fn q8946(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, score, view_count, .. } = &db.post;
    let key = |p: Id<Post>| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    };
    let w = db
        .post
        .with(creation_date.ge(date(2022, 1, 1)).and(post_type_id.is_in([1, 2])))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score.and(view_count.opt())))
        .window(row_number, |(p, (s, w))| (Reverse(s), w.is_none(), Reverse(w), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let mut v = drain(&s);
    v.sort_by_key(|&(p, _)| key(p));
    rows(v.into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH TagDetails AS (SELECT T.Id AS TagId, T.TagName, COUNT(P.Id) AS PostCount, SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END) AS PositiveScoreCount,
//        SUM(CASE WHEN P.Score < 0 THEN 1 ELSE 0 END) AS NegativeScoreCount FROM Tags T LEFT JOIN Posts P ON P.Tags LIKE '%' || T.TagName || '%' GROUP BY T.Id, T.TagName),
// PopularTags AS (SELECT TagId, TagName, PostCount, PositiveScoreCount, NegativeScoreCount, RANK() OVER (ORDER BY PostCount DESC, PositiveScoreCount DESC) AS PopularityRank FROM TagDetails),
// TopPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, T.TagName, P.ViewCount, P.Score FROM Posts P JOIN Tags T ON P.Tags LIKE '%' || T.TagName || '%'
//     WHERE P.PostTypeId = 1 ORDER BY P.ViewCount DESC LIMIT 10)
// SELECT PT.TagId, PT.TagName, PT.PostCount, PT.PositiveScoreCount, PT.NegativeScoreCount, TP.PostId, TP.Title, TP.CreationDate, TP.ViewCount, TP.Score
// FROM PopularTags PT JOIN TopPosts TP ON TP.TagName = PT.TagName WHERE PT.PopularityRank <= 5 ORDER BY PT.PopularityRank, TP.ViewCount DESC;
fn q29893(db: &'static So) -> String {
    let lt = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&lt).map(|(_, t)| t).inv().collect();
    let td = db
        .tag
        .group_by(Ident::<Tag>::new())
        .select((&by_tag).map(|(p, _)| p).select(&db.post.score).opt())
        .fold([0i64; 3], |a, s| match s {
            Some(s) => [a[0] + 1, a[1] + (s > 0) as i64, a[2] + (s < 0) as i64],
            None => a,
        });
    let w = whole(db.tag.iq()).select(Ident::<Tag>::new().and(&td)).window(rank, |(_, a)| (Reverse(a[0]), Reverse(a[1])), asc);
    type P = ((Id<Tag>, [i64; 3]), i64);
    let pt: MatSet<P> = (&w).filt(|(_, r)| r <= 5).collect();
    let Post { post_type_id, view_count, .. } = &db.post;
    type PT = (Id<Post>, Id<Tag>);
    let pairs = drain((&lt).with(Same::<PT>::new().map(|(p, _)| p).select(post_type_id.eq(1))));
    let tp = top_n(pairs, |&((p, t), _)| (Reverse(view_count.get(p)), p, db.tag.tag_name.get(t).unwrap()), 10);
    let tp: MatSet<PT> = rel(tp.into_iter().map(|x| x.0).collect()).map(|x| x).collect();
    let by_name: HashIdx<Str, PT> = (&tp).select(Same::<PT>::new().map(|(_, t): PT| t).select(&db.tag.tag_name)).inv().collect();
    let mut v = drain((&pt).select(Same::<P>::new().and(Same::<P>::new().map(|((t, _), _): P| t).select(&db.tag.tag_name).select(&by_name))));
    v.sort_by_key(|&(_, ((_, r), (p, _)))| (r, Reverse(view_count.get(p))));
    rows(v.into_iter().map(|(_, (((t, a), _), (p, _)))| {
        let mut f = vec![V::I(db.tag.origid.get(t).unwrap()), V::S(db.tag.tag_name.get(t).unwrap()), V::I(a[0]), V::I(a[1]), V::I(a[2])];
        f.extend(post_fields(db, p, &["id", "title", "created", "views", "score"]));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS OwnerDisplayName, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount,
//        RANK() OVER (PARTITION BY u.Id ORDER BY p.CreationDate DESC) AS UserPostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.PostTypeId = 1
//     GROUP BY p.Id, p.Title, p.CreationDate, u.DisplayName, p.Score, p.ViewCount, u.Id),
// AggregatedData AS (SELECT rp.OwnerDisplayName, COUNT(rp.PostId) AS TotalQuestions, SUM(rp.Score) AS TotalScore, AVG(rp.ViewCount) AS AvgViewCount,
//        SUM(rp.CommentCount) AS TotalComments FROM RankedPosts rp WHERE rp.UserPostRank <= 3 GROUP BY rp.OwnerDisplayName)
// SELECT ad.OwnerDisplayName, ad.TotalQuestions, ad.TotalScore, ad.AvgViewCount, ad.TotalComments,
//        CASE WHEN ad.TotalQuestions > 10 THEN 'High Contributor' WHEN ad.TotalQuestions BETWEEN 5 AND 10 THEN 'Moderate Contributor' ELSE 'New Contributor' END AS ContributorCategory
// FROM AggregatedData ad ORDER BY ad.TotalScore DESC, ad.TotalQuestions DESC LIMIT 50;
fn q8567(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, view_count, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1)).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(rank, |(_, d)| Reverse(d), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 3).map(|((p, _), _)| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let g = (&tp)
        .group_by(owner_user.select(&db.user.display_name))
        .select(score.and(view_count.opt()).and(&cc))
        .fold([0i64; 5], |a, ((s, w), c)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + c]);
    let v = top_n(drain(&g), |&(_, a)| (Reverse(a[1]), Reverse(a[0])), 50);
    rows(v.into_iter().map(|(n, a)| {
        let cat = if a[0] > 10 { "High Contributor" } else if a[0] >= 5 { "Moderate Contributor" } else { "New Contributor" };
        row(vec![V::S(n), V::I(a[0]), V::I(a[1]), avg(a[3], a[2]), V::I(a[4]), V::S(cat)])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, U.DisplayName AS OwnerDisplayName, p.Tags, p.Score,
//        RANK() OVER (PARTITION BY p.Tags ORDER BY p.Score DESC) AS TagRank FROM Posts p JOIN Users U ON p.OwnerUserId = U.Id WHERE p.PostTypeId = 1),
// PostHistoryStats AS (SELECT ph.PostId, COUNT(DISTINCT ph.UserId) AS EditorCount, COUNT(ph.Id) AS EditCount, MAX(ph.CreationDate) AS LastEditDate FROM PostHistory ph GROUP BY ph.PostId),
// UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(b.Id) AS BadgeCount FROM Users U LEFT JOIN Badges b ON U.Id = b.UserId GROUP BY U.Id, U.DisplayName, U.Reputation)
// SELECT RP.PostId, RP.Title, RP.CreationDate, RP.OwnerDisplayName, RP.Tags, RP.Score, PH.EditorCount, PH.EditCount, PH.LastEditDate,
//        UR.DisplayName AS UserReputationName, UR.Reputation, UR.BadgeCount, RP.TagRank
// FROM RankedPosts RP LEFT JOIN PostHistoryStats PH ON RP.PostId = PH.PostId JOIN UserReputation UR ON RP.OwnerUserId = UR.UserId
// WHERE RP.TagRank <= 3 ORDER BY RP.Tags, RP.TagRank;
fn q29138(db: &'static So) -> String {
    let Post { post_type_id, owner_user, tags_str, score, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1)).with(owner_user).group_by(tags_str.opt()).select(Ident::<Post>::new().and(score)).window(rank, |(_, s)| Reverse(s), asc);
    type T = (Id<Post>, i64);
    let tp: MatSet<T> = (&w).filt(|(_, r)| r <= 3).map(|((p, _), r)| (p, r)).collect();
    let PostHistory { post, user_id, creation_date: hd, .. } = &db.post_history;
    let phs = db.post_history.group_by(post).select(hd).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let eds = db.post_history.group_by(post).select(user_id).count_distinct();
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let pid = || Same::<T>::new().map(|(p, _): T| p);
    let mut v = drain((&tp).select(Same::<T>::new().and(pid().select((&phs).and((&eds).opt())).opt()).and(pid().select(owner_user.select(Ident::<User>::new().and(&bc))))));
    v.sort_by_key(|&(_, (((p, r), _), _))| {
        let t = tags_str.get(p);
        (t.is_none(), t, r)
    });
    rows(v.into_iter().map(|(_, (((p, r), h), (u, b)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner", "tags", "score"]);
        f.extend(match h {
            Some(((n, m), e)) => [V::I(e.unwrap_or(0)), V::I(n), V::T(m)],
            None => [V::Null, V::Null, V::Null],
        });
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::I(b), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, u.DisplayName AS OwnerDisplayName, p.Score, p.CreationDate,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.OwnerUserId, rp.OwnerDisplayName, rp.Score, rp.CreationDate FROM RankedPosts rp WHERE rp.Rank <= 5),
// UserBadges AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount FROM Badges b GROUP BY b.UserId)
// SELECT tp.Title, tp.OwnerDisplayName, tp.Score, ub.BadgeCount, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
// FROM TopPosts tp LEFT JOIN Comments c ON tp.PostId = c.PostId LEFT JOIN Votes v ON tp.PostId = v.PostId LEFT JOIN UserBadges ub ON tp.OwnerUserId = ub.UserId
// GROUP BY tp.Title, tp.OwnerDisplayName, tp.Score, ub.BadgeCount, tp.CreationDate ORDER BY tp.Score DESC, tp.CreationDate ASC;
fn q9332(db: &'static So) -> String {
    let Post { post_type_id, owner_user, owner_user_id, score, creation_date, title, .. } = &db.post;
    let w = db
        .post
        .with(post_type_id.eq(1))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|(((p, _), _), _)| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let bc = db.badge.group_by(&db.badge.user_id).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let g = (&tp)
        .group_by(title.opt().and(owner_user.select(&db.user.display_name)).and(score).and(owner_user_id.select(&bc).opt()).and(creation_date))
        .select(&s)
        .fold([0i64; 3], |a, x| [a[0] + x[0], a[1] + x[1], a[2] + x[2]]);
    let mut v = drain(&g);
    v.sort_by_key(|&((((_, _), s), _), d)| (Reverse(s), d));
    rows(v.into_iter().map(|(((((t, n), s), b), _), a)| {
        let mut f = vec![t.map_or(V::Null, V::S), V::S(n), V::I(s), b.map_or(V::Null, V::I)];
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.Score, p.ViewCount,
//        ARRAY_LENGTH(string_to_array(substring(p.Tags, 2, length(p.Tags)-2), '><'), 1) AS TagCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.CreationDate > cast('2024-10-01' as date) - INTERVAL '1 year' ORDER BY p.CreationDate DESC),
// TopUsers AS (SELECT OwnerDisplayName, COUNT(PostId) AS QuestionCount, SUM(Score) AS TotalScore FROM RankedPosts WHERE Rank <= 5 GROUP BY OwnerDisplayName),
// UserBadges AS (SELECT u.DisplayName AS OwnerDisplayName, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.DisplayName)
// SELECT tu.OwnerDisplayName, tu.QuestionCount, tu.TotalScore, ub.BadgeCount FROM TopUsers tu LEFT JOIN UserBadges ub ON tu.OwnerDisplayName = ub.OwnerDisplayName
// ORDER BY tu.TotalScore DESC, tu.QuestionCount DESC LIMIT 10;
fn q29190(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, creation_date, .. } = &db.post;
    let w = db
        .post
        .with(post_type_id.eq(1).and(creation_date.gt(add_years(date(2024, 10, 1), -1))))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|(((p, _), _), _)| p).collect();
    let tu = (&tp).group_by(owner_user.select(&db.user.display_name)).select(score).fold((0i64, 0i64), |(n, s), x| (n + 1, s + x));
    let ub = db.user.group_by(&db.user.display_name).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = top_n(drain((&tu).and(&ub)), |&(_, ((n, s), _))| (Reverse(s), Reverse(n)), 10);
    rows(v.into_iter().map(|(name, ((n, s), b))| row(vec![V::S(name), V::I(n), V::I(s), V::I(b)])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN VoteTypes vt ON v.VoteTypeId = vt.Id
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.Score, u.DisplayName, p.OwnerUserId),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, OwnerDisplayName, CommentCount, UpVotes, DownVotes FROM RankedPosts WHERE rn <= 5)
// SELECT OwnerDisplayName, COUNT(*) AS TotalPosts, AVG(Score) AS AverageScore, SUM(CommentCount) AS TotalComments, SUM(UpVotes) AS TotalUpVotes, SUM(DownVotes) AS TotalDownVotes
// FROM TopPosts GROUP BY OwnerDisplayName ORDER BY TotalPosts DESC, AverageScore DESC;
fn q7011(db: &'static So) -> String {
    let Post { post_type_id, owner_user, owner_user_id, creation_date, score, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1)).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(vtype_name(db)).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some("UpMod")) as i64, a[2] + (t == Some("DownMod")) as i64]);
    let g = (&tp)
        .group_by(owner_user.select(&db.user.display_name).opt())
        .select(score.and(&s))
        .fold([0i64; 5], |a, (sc, x)| [a[0] + 1, a[1] + sc, a[2] + x[0], a[3] + x[1], a[4] + x[2]]);
    let mut v = drain(&g);
    v.sort_by(|x, y| y.1[0].cmp(&x.1[0]).then((y.1[1] as f64 / y.1[0] as f64).total_cmp(&(x.1[1] as f64 / x.1[0] as f64))));
    rows(v.into_iter().map(|(n, a)| row(vec![n.map_or(V::Null, V::S), V::I(a[0]), avg(a[1], a[0]), V::I(a[2]), V::I(a[3]), V::I(a[4])])))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS PostRank FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.Score > 0),
// TopPosts AS (SELECT rp.OwnerDisplayName, COUNT(*) AS PostCount, AVG(rp.Score) AS AvgScore, SUM(rp.ViewCount) AS TotalViews FROM RankedPosts rp WHERE rp.PostRank <= 5 GROUP BY rp.OwnerDisplayName),
// UserBadges AS (SELECT u.DisplayName, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.DisplayName),
// FinalResults AS (SELECT tp.OwnerDisplayName, tp.PostCount, tp.AvgScore, tp.TotalViews, ub.BadgeCount FROM TopPosts tp LEFT JOIN UserBadges ub ON tp.OwnerDisplayName = ub.DisplayName)
// SELECT OwnerDisplayName, PostCount, AvgScore, TotalViews, COALESCE(BadgeCount, 0) AS BadgeCount FROM FinalResults ORDER BY TotalViews DESC, AvgScore DESC, PostCount DESC;
fn q5342(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, creation_date, view_count, .. } = &db.post;
    let w = db
        .post
        .with(post_type_id.eq(1).and(score.gt(0)))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|(((p, _), _), _)| p).collect();
    let g = (&tp)
        .group_by(owner_user.select(&db.user.display_name))
        .select(score.and(view_count.opt()))
        .fold([0i64; 4], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]);
    let ub = db.user.group_by(&db.user.display_name).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let mut v = drain((&g).and((&ub).opt()));
    v.sort_by(|x, y| {
        let k = |a: [i64; 4]| (a[2] == 0, Reverse(a[3]));
        k(x.1 .0).cmp(&k(y.1 .0)).then((y.1 .0[1] as f64 / y.1 .0[0] as f64).total_cmp(&(x.1 .0[1] as f64 / x.1 .0[0] as f64))).then(y.1 .0[0].cmp(&x.1 .0[0]))
    });
    rows(v.into_iter().map(|(n, (a, b))| row(vec![V::S(n), V::I(a[0]), avg(a[1], a[0]), nullable(a[3], a[2]), V::I(b.unwrap_or(0))])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        COUNT(DISTINCT v.Id) AS VoteCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId IN (1, 2) GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName, p.PostTypeId),
// TopPosts AS (SELECT PostId, Title, CreationDate, ViewCount, Score, OwnerDisplayName, CommentCount, VoteCount FROM RankedPosts WHERE Rank <= 5)
// SELECT tp.OwnerDisplayName, tp.Title, tp.CreationDate, tp.ViewCount, tp.Score, tp.CommentCount, tp.VoteCount, ph.Comment AS LastEditComment, ph.CreationDate AS LastEditDate
// FROM TopPosts tp LEFT JOIN PostHistory ph ON tp.PostId = ph.PostId WHERE ph.Id IN (SELECT MAX(Id) FROM PostHistory WHERE PostId = tp.PostId GROUP BY PostId) ORDER BY tp.Score DESC;
fn q8154(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let w = db
        .post
        .with(post_type_id.is_in([1, 2]))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score.and(view_count.opt())))
        .window(row_number, |(p, (s, w))| (Reverse(s), w.is_none(), Reverse(w), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let dv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db)).count_distinct();
    let PostHistory { post, origid, .. } = &db.post_history;
    let last = db.post_history.group_by(post).select(Ident::<PostHistory>::new().and(origid)).fold(None, |m: Option<(i64, Id<PostHistory>)>, (h, o)| match m {
        Some((mo, _)) if mo >= o => m,
        _ => Some((o, h)),
    });
    let mut v = drain((&cc).and((&dv).opt()).and(&last));
    v.sort_by_key(|&(p, _)| Reverse(score.get(p).unwrap()));
    rows(v.into_iter().map(|(p, ((c, n), h))| {
        let h = h.unwrap().1;
        let mut f = post_fields(db, p, &["owner", "title", "created", "views", "score"]);
        f.extend([V::I(c), V::I(n.unwrap_or(0)),harness::fmt::ostr(db.post_history.comment.get(h)), V::T(db.post_history.creation_date.get(h).unwrap())]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.Body,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS PostRank, p.OwnerUserId
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE u.Reputation > 1000 AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// TopPosts AS (SELECT rp.* FROM RankedPosts rp WHERE rp.PostRank <= 5),
// PostCommentStatistics AS (SELECT tp.PostId, COUNT(c.Id) AS TotalComments, AVG(c.Score) AS AverageCommentScore FROM TopPosts tp LEFT JOIN Comments c ON tp.PostId = c.PostId GROUP BY tp.PostId)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.AnswerCount, tp.CommentCount, pcs.TotalComments, pcs.AverageCommentScore,
//        u.DisplayName AS OwnerDisplayName, u.Reputation AS OwnerReputation
// FROM TopPosts tp JOIN Users u ON tp.OwnerUserId = u.Id JOIN PostCommentStatistics pcs ON tp.PostId = pcs.PostId ORDER BY tp.Score DESC, tp.ViewCount DESC;
fn q8395(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, view_count, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(1000))))
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|(((p, _), _), _)| p).collect();
    let pcs = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.score).opt()).fold((0i64, 0i64), |(n, s), c| (n + c.is_some() as i64, s + c.unwrap_or(0)));
    let mut v = drain(&pcs);
    v.sort_by_key(|&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    });
    rows(v.into_iter().map(|(p, (n, s))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments"]);
        f.extend([V::I(n), avg(s, n)]);
        f.extend(post_fields(db, p, &["owner", "rep"]));
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.CreationDate, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(P.ViewCount) AS TotalViewCount, AVG(P.Score) AS AverageScore
//     FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation, U.CreationDate),
// TopUsers AS (SELECT ..., ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Ranking FROM UserStats)
// SELECT UserId, DisplayName, Reputation, BadgeCount, UpVotes, DownVotes, QuestionCount, AnswerCount, TotalViewCount, AverageScore FROM TopUsers WHERE Ranking <= 10;
//
// The row number only reads Reputation, so the ten users are picked before
// their joined rows are aggregated.
fn q13206(db: &'static So) -> String {
    let top = top_n(drain(db.user.select(&db.user.reputation)), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(top.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let Post { post_type_id, view_count, score, .. } = &db.post;
    let s = (&tu)
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(posts_of(db).select(post_type_id.and(view_count.opt()).and(score).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt()))
        .fold([0i64; 9], |a, (b, p)| match p {
            Some((((t, w), s), v)) => [
                a[0] + b.is_some() as i64,
                a[1] + (v == Some(2)) as i64,
                a[2] + (v == Some(3)) as i64,
                a[3] + (t == 1) as i64,
                a[4] + (t == 2) as i64,
                a[5] + w.is_some() as i64,
                a[6] + w.unwrap_or(0),
                a[7] + 1,
                a[8] + s,
            ],
            None => [a[0] + b.is_some() as i64, a[1], a[2], a[3], a[4], a[5], a[6], a[7], a[8]],
        });
    rows(drain(&s).into_iter().map(|(u, a)| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4]), nullable(a[6], a[5]), avg(a[8], a[7])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.Tags, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, p.Score AS PostScore,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Body, p.Tags, u.DisplayName, p.Score, p.OwnerUserId, p.CreationDate),
// FilteredPosts AS (SELECT rp.*, (UPPER(rp.Title) LIKE '%SQL%') AS ContainsSQL, (UPPER(rp.Body) LIKE '%STRING%') AS ContainsString FROM RankedPosts rp WHERE rp.PostRank = 1),
// AggregatedResults AS (SELECT COUNT(*) AS TotalPosts, SUM(CASE WHEN ContainsSQL THEN 1 ELSE 0 END) AS PostsWithSQL,
//        SUM(CASE WHEN ContainsString THEN 1 ELSE 0 END) AS PostsWithString, AVG(PostScore) AS AvgPostScore FROM FilteredPosts)
// SELECT * FROM AggregatedResults;
fn q29091(db: &'static So) -> String {
    let Post { post_type_id, owner_user_id, creation_date, title, body, score, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1)).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let fp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let up = |s: Str, pat: &str| s.to_uppercase().contains(pat);
    let a = (&fp).select(title.opt().and(body.opt()).and(score)).fold_flat([0i64; 4], |a, ((t, b), s)| {
        [a[0] + 1, a[1] + t.map_or(false, |t| up(t, "SQL")) as i64, a[2] + b.map_or(false, |b| up(b, "STRING")) as i64, a[3] + s]
    });
    rows([row(vec![V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[3], a[0])])])
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT a.Id) AS AnswerCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPosts, p.OwnerUserId
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2
//     WHERE p.PostTypeId = 1 AND p.Score > 0 GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId),
// UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, RANK() OVER (ORDER BY u.Reputation DESC) AS Rank FROM Users u WHERE u.Reputation > 1000)
// SELECT up.DisplayName AS UserName, up.Reputation, COUNT(rp.PostId) AS TotalPosts, SUM(rp.CommentCount) AS TotalComments, SUM(rp.AnswerCount) AS TotalAnswers,
//        AVG(rp.Score) AS AverageScore, AVG(rp.ViewCount) AS AverageViews
// FROM UserReputation up JOIN RankedPosts rp ON up.UserId = rp.OwnerUserId GROUP BY up.UserId, up.DisplayName, up.Reputation
// HAVING COUNT(rp.PostId) > 5 ORDER BY AverageScore DESC, TotalPosts DESC LIMIT 10;
fn q8124(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let qs = || db.post.with(post_type_id.eq(1).and(score.gt(0))).group_by(Ident::<Post>::new());
    let cc = qs().select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ac = qs().select(answers_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let g = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score.and(view_count.opt()).and(&cc).and(&ac)))
        .fold([0i64; 6], |a, (((s, w), c), x)| [a[0] + 1, a[1] + c, a[2] + x, a[3] + s, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0)]);
    let mut v = drain((&g).filt(|a| a[0] > 5));
    v.sort_by(|x, y| (y.1[3] as f64 / y.1[0] as f64).total_cmp(&(x.1[3] as f64 / x.1[0] as f64)).then(y.1[0].cmp(&x.1[0])));
    v.truncate(10);
    rows(v.into_iter().map(|(u, a)| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[3], a[0]), avg(a[5], a[4])]);
        row(f)
    }))
}

// WITH UserBadges AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PostStats AS (SELECT p.OwnerUserId, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(p.Score) AS TotalScore FROM Posts p GROUP BY p.OwnerUserId),
// UserPostBadges AS (SELECT ub.UserId, ub.DisplayName, ub.BadgeCount, ps.TotalPosts, ps.Questions, ps.Answers, ps.TotalScore,
//        ROW_NUMBER() OVER (ORDER BY ub.BadgeCount DESC, ps.TotalScore DESC) AS Rank FROM UserBadges ub JOIN PostStats ps ON ub.UserId = ps.OwnerUserId)
// SELECT UserId, DisplayName, BadgeCount, TotalPosts, Questions, Answers, TotalScore, Rank FROM UserPostBadges WHERE Rank <= 10 ORDER BY BadgeCount DESC, TotalScore DESC;
fn q5463(db: &'static So) -> String {
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let ups = user_posts(db);
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&bc).and((&ups).filt(|a| a[1] > 0))).window(row_number, |((u, b), a)| (Reverse(b), Reverse(a[4]), u), asc);
    let mut v: Vec<_> = drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|x| x.1).collect();
    v.sort_by_key(|&(_, r)| r);
    rows(v.into_iter().map(|(((u, b), a), r)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(b), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4]), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS RankScore
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= '2022-01-01' AND p.Score > 0),
// TopRankedPosts AS (SELECT PostId, Title, CreationDate, ViewCount, Score, OwnerDisplayName FROM RankedPosts WHERE RankScore <= 10),
// PostComments AS (SELECT pc.PostId, COUNT(pc.Id) AS CommentCount FROM Comments pc GROUP BY pc.PostId),
// CombinedResults AS (SELECT trp.PostId, trp.Title, trp.CreationDate, trp.ViewCount, trp.Score, trp.OwnerDisplayName, COALESCE(pc.CommentCount, 0) AS CommentCount
//     FROM TopRankedPosts trp LEFT JOIN PostComments pc ON trp.PostId = pc.PostId)
// SELECT cr.PostId, cr.Title, cr.CreationDate, cr.ViewCount, cr.Score, cr.OwnerDisplayName, cr.CommentCount FROM CombinedResults cr ORDER BY cr.Score DESC, cr.ViewCount DESC;
fn q8682(db: &'static So) -> String {
    let Post { creation_date, score, owner_user, post_type_id, view_count, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(date(2022, 1, 1)).and(score.gt(0)))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|(((p, _), _), _)| p).collect();
    let mut v = drain((&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64));
    v.sort_by_key(|&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    });
    rows(v.into_iter().map(|(p, n)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "owner"]);
        f.push(V::I(n));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, COUNT(a.Id) AS AnswerCount, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.Id) AS VoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY COUNT(a.Id) DESC) AS OwnerPostRank, p.OwnerUserId
//     FROM Posts p LEFT JOIN Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2 LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Body, p.CreationDate, p.OwnerUserId),
// UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, SUM(b.Class) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation)
// SELECT rp.PostId, rp.Title, rp.Body, rp.CreationDate, rp.AnswerCount, rp.CommentCount, rp.VoteCount, ur.DisplayName AS OwnerName, ur.Reputation AS OwnerReputation, ur.BadgeCount
// FROM RankedPosts rp JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId WHERE rp.OwnerPostRank = 1 ORDER BY rp.VoteCount DESC, rp.AnswerCount DESC FETCH FIRST 100 ROWS ONLY;
fn q29569(db: &'static So) -> String {
    let Post { post_type_id, owner_user, .. } = &db.post;
    let qs = || db.post.with(post_type_id.eq(1)).with(owner_user).group_by(Ident::<Post>::new());
    let ac = qs()
        .select(answers_of(db).opt().and(comments_of(db).opt()).and(votes_of(db).opt()))
        .fold((0i64, 0i64), |(a, c), ((x, y), _)| (a + x.is_some() as i64, c + y.is_some() as i64));
    let dv = qs().select(votes_of(db)).count_distinct();
    let w = db
        .post
        .with(post_type_id.eq(1))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(&ac).and((&dv).opt()))
        .window(row_number, |((p, (a, _)), _)| (Reverse(a), p), asc);
    let bs = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold(None, |s: Option<i64>, c| match c {
        Some(c) => Some(s.unwrap_or(0) + c),
        None => s,
    });
    let v = drain((&w).filt(|(_, r)| r == 1).and(&bs));
    let top = top_n(v, |&(_, ((((p, (a, _)), n), _), _))| (Reverse(n.unwrap_or(0)), Reverse(a), p), 100);
    rows(top.into_iter().map(|(u, ((((p, (a, c)), n), _), b))| {
        let mut f = post_fields(db, p, &["id", "title", "body", "created"]);
        f.extend([V::I(a), V::I(c), V::I(n.unwrap_or(0))]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.push(b.map_or(V::Null, V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.OwnerUserId, u.DisplayName AS OwnerDisplayName, COUNT(DISTINCT c.Id) AS CommentCount,
//        COUNT(DISTINCT a.Id) AS AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS OwnerRank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Posts a ON p.Id = a.ParentId
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.OwnerUserId, u.DisplayName),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(u.Reputation) AS TotalReputation, COUNT(DISTINCT p.Id) AS TotalPosts
//     FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName HAVING COUNT(DISTINCT p.Id) > 5)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.OwnerDisplayName, t.TotalReputation, t.TotalPosts, rp.CommentCount, rp.AnswerCount
// FROM RankedPosts rp JOIN TopUsers t ON rp.OwnerUserId = t.UserId WHERE rp.OwnerRank <= 3 ORDER BY t.TotalReputation DESC, rp.Score DESC;
fn q5529(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let tu = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(owner_user.select(&db.user.reputation))).fold((0i64, 0i64), |(r, n), x| (r + x, n + 1));
    let w = db
        .post
        .with(post_type_id.eq(1))
        .group_by(owner_user.select(Ident::<User>::new().with((&tu).filt(|(_, n)| n > 5))))
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 3).map(|((p, _), _)| p).collect();
    let dc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db)).count_distinct();
    let da = (&tp).group_by(Ident::<Post>::new()).select(children_of(db)).count_distinct();
    let mut v = drain((&tp).select(Ident::<Post>::new().and((&dc).opt()).and((&da).opt()).and(owner_user.select(&tu))));
    v.sort_by_key(|&(p, (_, (r, _)))| (Reverse(r), Reverse(score.get(p).unwrap())));
    rows(v.into_iter().map(|(p, (((_, c), a), (r, n)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "owner"]);
        f.extend([V::I(r), V::I(n), V::I(c.unwrap_or(0)), V::I(a.unwrap_or(0))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank,
//        COUNT(DISTINCT c.Id) AS CommentCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.PostTypeId IN (1, 2)
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId),
// UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, RANK() OVER (ORDER BY u.Reputation DESC) AS ReputationRank FROM Users u),
// TopUserPosts AS (SELECT up.PostId, up.Title, ur.DisplayName AS OwnerDisplayName, ur.Reputation, up.CommentCount, up.CreationDate, up.Score
//     FROM RankedPosts up JOIN UserReputation ur ON up.OwnerUserId = ur.UserId WHERE ur.ReputationRank <= 10)
// SELECT tup.OwnerDisplayName, COUNT(tup.PostId) AS NumberOfPosts, AVG(tup.Score) AS AverageScore, SUM(tup.CommentCount) AS TotalComments, MAX(tup.CreationDate) AS MostRecentPost
// FROM TopUserPosts tup GROUP BY tup.OwnerDisplayName ORDER BY NumberOfPosts DESC LIMIT 5;
fn q34931(db: &'static So) -> String {
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r)| Reverse(r), asc);
    let tu: MatSet<Id<User>> = (&w).filt(|(_, r)| r <= 10).map(|((u, _), _)| u).collect();
    let Post { post_type_id, owner_user, score, creation_date, .. } = &db.post;
    let ps = db.post.with(post_type_id.is_in([1, 2])).with(owner_user.select(Ident::<User>::new().with(&tu)));
    let cc = (&ps).group_by(Ident::<Post>::new()).select(comments_of(db)).count_distinct();
    let g = (&ps)
        .group_by(owner_user.select(&db.user.display_name))
        .select(score.and((&cc).opt()).and(creation_date))
        .fold([0, 0, 0, i64::MIN], |a, ((s, c), d)| [a[0] + 1, a[1] + s, a[2] + c.unwrap_or(0), a[3].max(d)]);
    let v = top_n(drain(&g), |&(_, a)| Reverse(a[0]), 5);
    rows(v.into_iter().map(|(n, a)| row(vec![V::S(n), V::I(a[0]), avg(a[1], a[0]), V::I(a[2]), tmax(a[3])])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId),
// BestPosts AS (SELECT r.PostId, r.Title, r.CreationDate, r.Score, r.ViewCount, r.CommentCount, r.UpVotes, r.DownVotes, ROW_NUMBER() OVER (ORDER BY r.Score DESC, r.ViewCount DESC) AS OverallRank
//     FROM RankedPosts r WHERE r.Rank <= 5)
// SELECT bp.Title, bp.CreationDate, bp.Score, bp.ViewCount, bp.CommentCount, bp.UpVotes, bp.DownVotes, u.DisplayName AS OwnerDisplayName
// FROM BestPosts bp JOIN Users u ON bp.PostId IN (SELECT Id FROM Posts WHERE OwnerUserId = u.Id) WHERE bp.OverallRank <= 10 ORDER BY bp.OverallRank;
//
// Both row numbers only read Score and ViewCount, so the posts are picked
// before their joined rows are counted.
fn q9591(db: &'static So) -> String {
    let Post { post_type_id, owner_user, owner_user_id, score, view_count, .. } = &db.post;
    let key = |p: Id<Post>| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    };
    let w = db
        .post
        .with(post_type_id.eq(1))
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(score.and(view_count.opt())))
        .window(row_number, |(p, (s, w))| (Reverse(s), w.is_none(), Reverse(w), p), asc);
    let top = top_n(drain((&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p)), |&(_, p)| (key(p), p), 10);
    let top = rel(top.into_iter().map(|x| x.1).collect());
    let tp = rel(drain((&top).select(Ident::<Post>::new().and(owner_user))));
    let tpp: MatSet<Id<Post>> = (&tp).map(|(_, (p, _))| p).collect();
    let s = (&tpp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(drain(&tp).into_iter().map(|(_, (_, (p, u)))| {
        let mut f = post_fields(db, p, &["title", "created", "score", "views"]);
        f.extend(s.get(p).unwrap().map(V::I));
        f.push(user_col(db, u, "name"));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.Id) AS Rank FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '90 days'),
// TopVotedPosts AS (SELECT rp.*, vt.Name AS VoteTypeName FROM RankedPosts rp LEFT JOIN Votes v ON rp.PostId = v.PostId LEFT JOIN VoteTypes vt ON v.VoteTypeId = vt.Id WHERE rp.Rank <= 10),
// PostWithLatestComment AS (SELECT tp.*, c.Text AS LatestCommentText, c.CreationDate AS LatestCommentDate FROM TopVotedPosts tp LEFT JOIN Comments c ON tp.PostId = c.PostId
//     WHERE c.CreationDate = (SELECT MAX(CreationDate) FROM Comments WHERE PostId = tp.PostId))
// SELECT p.PostId, p.Title, p.OwnerDisplayName, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.VoteTypeName, p.LatestCommentText, p.LatestCommentDate
// FROM PostWithLatestComment p ORDER BY p.Score DESC, p.ViewCount DESC;
//
// rewrites/9451.sql: the row number tie-broken on the Id.
fn q9451(db: &'static So) -> String {
    let Post { creation_date, owner_user, post_type_id, score, view_count, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -90)))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(&db.post.origid))
        .window(row_number, |((_, s), o)| (Reverse(s), o), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|(((p, _), _), _)| p).collect();
    let Comment { post, creation_date: cd, .. } = &db.comment;
    let md = db.comment.group_by(post).select(cd).fold(i64::MIN, |m, d| m.max(d));
    let at: HashIdx<(Id<Post>, i64), Id<Comment>> = db.comment.select(post.and(cd)).inv().collect();
    let mut v = drain((&tp).select(votes_of(db).select(vtype_name(db)).opt().and(Ident::<Post>::new().and(&md).select(&at))));
    v.sort_by_key(|&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    });
    rows(v.into_iter().map(|(p, (t, c))| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "score", "views", "answers", "comments"]);
        f.extend([t.map_or(V::Null, V::S), V::S(db.comment.text.get(c).unwrap()), V::T(cd.get(c).unwrap())]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(p.Score) AS TotalScore, SUM(p.ViewCount) AS TotalViews
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT ..., ROW_NUMBER() OVER (ORDER BY TotalScore DESC) AS Rank FROM UserPostStats),
// BadgesCount AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id)
// SELECT tu.UserId, tu.DisplayName, tu.PostCount, tu.QuestionCount, tu.AnswerCount, tu.TotalScore, tu.TotalViews, bc.BadgeCount,
//        CASE WHEN tu.Rank <= 10 THEN 'Top Contributor' ELSE 'Regular Contributor' END AS ContributorType
// FROM TopUsers tu JOIN BadgesCount bc ON tu.UserId = bc.UserId WHERE tu.PostCount > 0 ORDER BY tu.TotalScore DESC, tu.PostCount DESC;
fn q9797(db: &'static So) -> String {
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let ups = user_posts(db);
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&ups).and(&bc)).window(row_number, |((u, a), _)| (a[1] == 0, Reverse(a[4]), u), asc);
    let mut v: Vec<_> = drain((&w).filt(|(((_, a), _), _)| a[1] > 0)).into_iter().map(|x| x.1).collect();
    v.sort_by_key(|&(((_, a), _), _)| (Reverse(a[4]), Reverse(a[1])));
    rows(v.into_iter().map(|(((u, a), b), r)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4]), nullable(a[6], a[5]), V::I(b)]);
        f.push(V::S(if r <= 10 { "Top Contributor" } else { "Regular Contributor" }));
        row(f)
    }))
}

// WITH UserBadges AS (SELECT U.Id AS UserId, U.Reputation, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.Reputation),
// PostStats AS (SELECT P.OwnerUserId, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, AVG(P.ViewCount) AS AvgViewCount FROM Posts P GROUP BY P.OwnerUserId),
// TrendingUsers AS (SELECT UB.UserId, UB.BadgeCount, PS.TotalPosts, PS.TotalQuestions, PS.TotalAnswers, PS.AvgViewCount,
//        ROW_NUMBER() OVER (ORDER BY UB.Reputation DESC, UB.BadgeCount DESC) AS Rank FROM UserBadges UB JOIN PostStats PS ON UB.UserId = PS.OwnerUserId
//     WHERE UB.Reputation > 1000 AND PS.TotalPosts > 5)
// SELECT U.DisplayName, TU.Rank, TU.BadgeCount, TU.TotalPosts, TU.TotalQuestions, TU.TotalAnswers, TU.AvgViewCount FROM TrendingUsers TU JOIN Users U ON TU.UserId = U.Id
// WHERE TU.Rank <= 10 ORDER BY TU.Rank;
fn q3446(db: &'static So) -> String {
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let ups = user_posts(db);
    let w = whole(db.user.iq())
        .select(Ident::<User>::new().with((&db.user.reputation).gt(1000)).and(&db.user.reputation).and(&bc).and((&ups).filt(|a| a[1] > 5)))
        .window(row_number, |(((u, r), b), _)| (Reverse(r), Reverse(b), u), asc);
    let mut v: Vec<_> = drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|x| x.1).collect();
    v.sort_by_key(|&(_, r)| r);
    rows(v.into_iter().map(|((((u, _), b), a), i)| row(vec![user_col(db, u, "name"), V::I(i), V::I(b), V::I(a[1]), V::I(a[2]), V::I(a[3]), avg(a[6], a[5])])))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COALESCE(SUM(V.BountyAmount), 0) AS TotalBounties, ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS ReputationRank
//     FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// PostStats AS (SELECT P.Id AS PostId, P.Title, COUNT(C.Id) AS CommentCount, COUNT(A.Id) AS AnswerCount, AVG(COALESCE(P.Score, 0)) AS AvgScore, MAX(P.CreationDate) AS LastActivity
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Posts A ON P.Id = A.ParentId AND A.PostTypeId = 2 WHERE P.PostTypeId = 1 GROUP BY P.Id, P.Title)
// SELECT U.DisplayName, U.Reputation, UR.TotalBounties, PS.Title, PS.CommentCount, PS.AnswerCount, PS.AvgScore, PS.LastActivity
// FROM UserReputation UR JOIN Users U ON U.Id = UR.UserId
// JOIN PostStats PS ON PS.PostId IN (SELECT P.Id FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId WHERE V.UserId = U.Id)
// WHERE UR.ReputationRank <= 10 ORDER BY UR.Reputation DESC FETCH FIRST 5 ROWS ONLY;
fn q1595(db: &'static So) -> String {
    let top = top_n(drain(db.user.select(&db.user.reputation)), |&(u, r)| (Reverse(r), u), 10);
    let tu = rel(top.into_iter().map(|x| x.0).collect());
    let Post { post_type_id, score, creation_date, .. } = &db.post;
    let voted: MatSet<(Id<User>, Id<Post>)> = db.vote.select((&db.vote.user).and(&db.vote.post)).collect();
    let by_user: HashIdx<Id<User>, (Id<User>, Id<Post>)> = (&voted).map(|(u, _)| u).inv().collect();
    let bt = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()).fold(0i64, |s, b| s + b.flatten().unwrap_or(0));
    let ps = db
        .post
        .with(post_type_id.eq(1))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(answers_of(db).opt()))
        .fold((0i64, 0i64), |(c, a), (x, y)| (c + x.is_some() as i64, a + y.is_some() as i64));
    let v = drain((&tu).select(Ident::<User>::new().and(&bt)).and((&tu).select((&by_user).map(|(_, p)| p).select(Ident::<Post>::new().and(&ps)))));
    let v = top_n(v, |&(i, (_, (p, _)))| (i, p), 5);
    rows(v.into_iter().map(|(_, ((u, b), (p, (c, a))))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(b));
        f.extend(post_fields(db, p, &["title"]));
        f.extend([V::I(c), V::I(a), V::F(score.get(p).unwrap() as f64), V::T(creation_date.get(p).unwrap())]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p WHERE p.Score > 0),
// UserReputation AS (SELECT u.Id AS UserId, COALESCE(SUM(b.Class), 0) AS TotalBadges, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounties
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id),
// PostActivity AS (SELECT ph.PostId, ph.UserId, COUNT(*) AS EditCount, MAX(ph.CreationDate) AS LastEditDate FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (4, 5) GROUP BY ph.PostId, ph.UserId)
// SELECT u.DisplayName, ur.TotalBadges, ur.TotalBounties, rp.Title, rp.Score, pa.EditCount, pa.LastEditDate
// FROM Users u LEFT JOIN UserReputation ur ON u.Id = ur.UserId LEFT JOIN RankedPosts rp ON u.Id = rp.OwnerUserId AND rp.PostRank = 1
// LEFT JOIN PostActivity pa ON rp.Id = pa.PostId AND pa.UserId = u.Id
// WHERE ur.TotalBadges > 0 OR ur.TotalBounties > 0 ORDER BY u.Reputation DESC, rp.Score DESC OFFSET 0 ROWS FETCH NEXT 50 ROWS ONLY;
fn q1755(db: &'static So) -> String {
    let ur = db
        .user
        .group_by(Ident::<User>::new())
        .select(badges_of(db).select(&db.badge.class).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold((0i64, 0i64), |(c, b), (x, y)| (c + x.unwrap_or(0), b + y.flatten().unwrap_or(0)));
    let Post { score, owner_user, creation_date, .. } = &db.post;
    let w = db.post.with(score.gt(0)).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    type PU = (Id<Post>, Id<User>);
    let rp: MatSet<PU> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).select(Ident::<Post>::new().and(owner_user)).collect();
    let rp_of: HashIdx<Id<User>, PU> = (&rp).map(|(_, u)| u).inv().select(&rp).collect();
    let PostHistory { post, user, post_history_type_id, .. } = &db.post_history;
    let pa = db
        .post_history
        .with(post_history_type_id.is_in([4, 5]))
        .with(user)
        .group_by(post.and(user))
        .select(&db.post_history.creation_date)
        .fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let us = drain(db.user.select((&db.user.reputation).and((&ur).filt(|(c, b)| c > 0 || b > 0)).and((&rp_of).select(Same::<PU>::new().and((&pa).opt())).opt())));
    let v = top_n(us, |&(u, ((r, _), p))| (Reverse(r), p.is_none(), Reverse(p.map(|((p, _), _)| score.get(p).unwrap())), u), 50);
    rows(v.into_iter().map(|(u, ((_, (c, b)), p))| {
        let mut f = vec![user_col(db, u, "name"), V::I(c), V::I(b)];
        match p {
            Some(((p, _), h)) => {
                f.extend(post_fields(db, p, &["title", "score"]));
                f.extend(match h {
                    Some((n, m)) => [V::I(n), V::T(m)],
                    None => [V::Null, V::Null],
                });
            }
            None => f.extend([V::Null, V::Null, V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH PopularPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount, ARRAY_AGG(t.TagName) AS Tags
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Tags t ON t.ExcerptPostId = p.Id
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount
//     ORDER BY p.Score DESC LIMIT 10),
// TopUsers AS (SELECT u.Id, u.DisplayName, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName ORDER BY Upvotes DESC LIMIT 5)
// SELECT pp.Title, pp.CreationDate, pp.Score, pp.ViewCount, pp.CommentCount, tu.DisplayName AS TopUser, tu.Upvotes, tu.Downvotes, pp.Tags
// FROM PopularPosts pp JOIN TopUsers tu ON pp.Id IN (SELECT PostId FROM Votes WHERE UserId = tu.Id AND VoteTypeId = 2) ORDER BY pp.Score DESC, pp.ViewCount DESC;
//
// No up-vote records its voter, so the join is empty whichever five users
// and ten posts the tied cuts keep.
fn q8865(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, view_count, .. } = &db.post;
    let pp = top_n(drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.eq(1))).select(score)), |&(p, s)| (Reverse(s), p), 10);
    let pp: MatSet<Id<Post>> = rel(pp.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let uv = db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold((0i64, 0i64), |(u, d), t| {
        (u + (t == Some(2)) as i64, d + (t == Some(3)) as i64)
    });
    let tu: MatSet<Id<User>> = rel(top_n(drain(&uv), |&(u, (n, _))| (Reverse(n), u), 5).into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let ups: MatSet<(Id<Post>, Id<User>)> = db.vote.with((&db.vote.vote_type_id).eq(2)).select((&db.vote.post).and((&db.vote.user).select(Ident::<User>::new().with(&tu)))).collect();
    let hit = (&ups).with(Same::<(Id<Post>, Id<User>)>::new().map(|(p, _)| p).select(Ident::<Post>::new().with(&pp)));
    let cc = (&pp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ex: HashIdx<Id<Post>, Id<Tag>> = (&db.tag.excerpt_post).inv().collect();
    let tags = (&pp).group_by(Ident::<Post>::new()).select((&ex).opt()).buf_fold(|it| &*Box::leak(it.into_iter().collect::<Vec<Option<Id<Tag>>>>().into_boxed_slice()));
    let mut v = drain(&hit);
    v.sort_by_key(|&((p, _), _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    });
    rows(v.into_iter().map(|((p, u), _)| {
        let (up, dn) = uv.get(u).unwrap();
        let mut f = post_fields(db, p, &["title", "created", "score", "views"]);
        f.extend([V::I(cc.get(p).unwrap()), user_col(db, u, "name"), V::I(up), V::I(dn)]);
        f.push(V::L(tags.get(p).unwrap().iter().map(|t| t.map_or(V::Null, |t| V::S(db.tag.tag_name.get(t).unwrap()))).collect()));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, COUNT(c.Id) AS CommentCount, COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 2) AS UpVotes,
//        COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 3) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RecentPostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.OwnerUserId),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(p.Score) AS TotalScore, SUM(p.ViewCount) AS TotalViews, COUNT(DISTINCT p.Id) AS PostCount,
//        RANK() OVER (ORDER BY SUM(p.Score) DESC) AS UserRank FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName)
// SELECT ru.DisplayName AS RecentAuthor, rp.Title, rp.CreationDate, rp.CommentCount, rp.UpVotes, rp.DownVotes, tu.DisplayName AS TopAuthor, tu.TotalScore, tu.TotalViews, tu.PostCount
// FROM RankedPosts rp JOIN Users ru ON rp.PostId = ru.Id JOIN TopUsers tu ON tu.UserRank <= 10 WHERE rp.RecentPostRank = 1 ORDER BY rp.Score DESC, rp.CommentCount DESC LIMIT 50;
fn q8120(db: &'static So) -> String {
    let Post { post_type_id, owner_user_id, creation_date, origid, score, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1)).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let uids: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let tp: MatSet<Id<Post>> = (&tp).with(origid.select(&uids)).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let ups = user_posts(db);
    let tw = whole(db.user.iq()).select(Ident::<User>::new().and((&ups).filt(|a| a[1] > 0))).window(rank, |(_, a)| Reverse(a[4]), asc);
    let tu: MatSet<(Id<User>, [i64; 10])> = (&tw).filt(|(_, r)| r <= 10).map(|(x, _)| x).collect();
    let v = top_n(drain((&s).and(origid.select(&uids)).cross(&tu)), |&((p, _), ((a, _), _))| (Reverse(score.get(p).unwrap()), Reverse(a[0])), 50);
    rows(v.into_iter().map(|((p, _), ((a, ru), (t, b)))| {
        let mut f = vec![user_col(db, ru, "name")];
        f.extend(post_fields(db, p, &["title", "created"]));
        f.extend(a.map(V::I));
        f.extend([user_col(db, t, "name"), V::I(b[4]), nullable(b[6], b[5]), V::I(b[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS Ranking, p.OwnerUserId
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.AcceptedAnswerId IS NOT NULL),
// UserReputation AS (SELECT u.Id AS UserId, SUM(b.Class) AS TotalBadgeClass, COUNT(b.Id) AS TotalBadges, AVG(u.Reputation) AS AvgReputation FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostSummary AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, rp.CommentCount, ur.TotalBadgeClass, ur.TotalBadges, ur.AvgReputation
//     FROM RankedPosts rp JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId WHERE rp.Ranking <= 5)
// SELECT ps.PostId, ps.Title, ps.CreationDate, ps.Score, ps.ViewCount, ps.AnswerCount, ps.CommentCount, ps.TotalBadgeClass, ps.TotalBadges, ps.AvgReputation
// FROM PostSummary ps ORDER BY ps.Score DESC, ps.CreationDate DESC;
fn q8720(db: &'static So) -> String {
    let Post { post_type_id, accepted_answer_id, owner_user, score, creation_date, .. } = &db.post;
    let w = db
        .post
        .with(post_type_id.eq(1))
        .with(accepted_answer_id)
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let ur = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold((0i64, 0i64), |(s, n), c| (s + c.unwrap_or(0), n + c.is_some() as i64));
    let mut top = drain((&w).filt(|(_, r)| r <= 5).and(&ur));
    top.sort_by_key(|&(_, ((((_, s), d), _), _))| (Reverse(s), Reverse(d)));
    rows(top.into_iter().map(|(u, ((((p, _), _), _), (s, n)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments"]);
        f.extend([nullable(s, n), V::I(n), V::F(db.user.reputation.get(u).unwrap() as f64)]);
        row(f)
    }))
}

// WITH PostMetrics AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, p.CommentCount, p.FavoriteCount, u.Reputation AS OwnerReputation,
//        COUNT(DISTINCT c.Id) AS CommentCountDistinct, COUNT(DISTINCT v.Id) AS VoteCount
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= DATE '2022-01-01' GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, p.CommentCount, p.FavoriteCount, u.Reputation),
// TagMetrics AS (SELECT t.TagName, SUM(pm.ViewCount) AS TotalViewCount, AVG(pm.Score) AS AverageScore, SUM(pm.AnswerCount) AS TotalAnswerCount
//     FROM PostMetrics pm JOIN Posts p ON pm.PostId = p.Id
//     JOIN LATERAL (SELECT TRIM(tagname) AS TagName FROM UNNEST(string_to_array(p.Tags, ',')) AS tag(tagname)) AS tag ON TRUE
//     JOIN Tags t ON t.TagName = tag.TagName GROUP BY t.TagName)
// SELECT tm.TagName, tm.TotalViewCount, tm.AverageScore, tm.TotalAnswerCount FROM TagMetrics tm ORDER BY tm.TotalViewCount DESC LIMIT 10;
//
// Tags has no commas, so each split is the whole Tags string, which names no
// tag: it is empty.
fn q10523(db: &'static So) -> String {
    let names: HashIdx<Str, Id<Tag>> = (&db.tag.tag_name).inv().collect();
    let Post { creation_date, tags_str, view_count, score, answer_count, .. } = &db.post;
    let tm = db
        .post
        .with(creation_date.ge(date(2022, 1, 1)))
        .select(tags_str.flat_map(|t: Str| t.split(',').map(|x| x.trim()).collect::<Vec<_>>()).select(&names).and(view_count.opt().and(score).and(answer_count.opt())))
        .group_by(Same::<(Id<Tag>, ((Option<i64>, i64), Option<i64>))>::new().map(|(t, _)| t))
        .select(Same::<(Id<Tag>, ((Option<i64>, i64), Option<i64>))>::new().map(|(_, x)| x))
        .fold([0i64; 6], |a, ((w, s), n)| [a[0] + w.is_some() as i64, a[1] + w.unwrap_or(0), a[2] + 1, a[3] + s, a[4] + n.is_some() as i64, a[5] + n.unwrap_or(0)]);
    let v = top_n(drain(&tm), |&(_, a)| (a[0] == 0, Reverse(a[1])), 10);
    rows(v.into_iter().map(|(t, a)| row(vec![V::S(db.tag.tag_name.get(t).unwrap()), nullable(a[1], a[0]), avg(a[3], a[2]), nullable(a[5], a[4])])))
}

// WITH TopUsers AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.CreationDate, ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS UserRank FROM Users U WHERE U.Reputation > 1000),
// RecentPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, P.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS PostRank
//     FROM Posts P WHERE P.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// UserPostStatistics AS (SELECT U.DisplayName, COUNT(RP.PostId) AS TotalPosts, SUM(RP.ViewCount) AS TotalViews, SUM(RP.Score) AS TotalScore
//     FROM TopUsers U LEFT JOIN RecentPosts RP ON U.UserId = RP.OwnerUserId GROUP BY U.DisplayName),
// CombinedStats AS (SELECT UPS.DisplayName, UPS.TotalPosts, UPS.TotalViews, UPS.TotalScore, U.Reputation, RANK() OVER (ORDER BY UPS.TotalScore DESC) AS ScoreRank
//     FROM UserPostStatistics UPS JOIN TopUsers U ON UPS.DisplayName = U.DisplayName)
// SELECT CS.DisplayName, CS.TotalPosts, CS.TotalViews, CS.TotalScore, CS.Reputation, CS.ScoreRank FROM CombinedStats CS WHERE CS.ScoreRank <= 10 ORDER BY CS.ScoreRank;
fn q5966(db: &'static So) -> String {
    let Post { creation_date, view_count, score, .. } = &db.post;
    let recent = Ident::<Post>::new().with(creation_date.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30)));
    let tu = || db.user.with((&db.user.reputation).gt(1000));
    let ups = tu()
        .group_by(&db.user.display_name)
        .select(posts_of(db).select(recent).select(view_count.opt().and(score)).opt())
        .fold([0i64; 4], |a, p| match p {
            Some((w, s)) => [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s],
            None => a,
        });
    let w = whole(db.user.iq())
        .select(Ident::<User>::new().with((&db.user.reputation).gt(1000)).and((&db.user.display_name).select(Same::<Str>::new().and(&ups))))
        .window(rank, |(_, (_, a))| (a[0] == 0, Reverse(a[3])), asc);
    let mut v: Vec<_> = drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|x| x.1).collect();
    v.sort_by_key(|&(_, r)| r);
    rows(v.into_iter().map(|((u, (n, a)), r)| {
        row(vec![V::S(n), V::I(a[0]), nullable(a[2], a[1]), nullable(a[3], a[0]), user_col(db, u, "rep"), V::I(r)])
    }))
}

// WITH UserVoteStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(v.Id) AS TotalVotes, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName),
// PostStats AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.Title, p.OwnerUserId),
// TopUsers AS (SELECT UserId, DisplayName, TotalVotes, UpVotes, DownVotes, RANK() OVER (ORDER BY TotalVotes DESC) AS Rank FROM UserVoteStats)
// SELECT pu.DisplayName AS User, ps.Title AS PostTitle, ps.CommentCount, ps.UpVotes AS PostUpVotes, ps.DownVotes AS PostDownVotes, tu.Rank
// FROM PostStats ps JOIN Users pu ON ps.OwnerUserId = pu.Id JOIN TopUsers tu ON pu.Id = tu.UserId WHERE tu.Rank <= 10 ORDER BY tu.Rank, ps.UpVotes DESC;
fn q7951(db: &'static So) -> String {
    let uv = db.user.group_by(Ident::<User>::new()).select(votes_by(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&uv)).window(rank, |(_, n)| Reverse(n), asc);
    type T = (Id<User>, i64);
    let tu: MatSet<T> = (&w).filt(|(_, r)| r <= 10).map(|((u, _), r)| (u, r)).collect();
    let tp: MatSet<Id<Post>> = (&tu).map(|(u, _)| u).select(posts_of(db)).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let mut v = drain((&tu).select(Same::<T>::new().and(Same::<T>::new().map(|(u, _): T| u).select(posts_of(db).select(Ident::<Post>::new().and(&s))))));
    v.sort_by_key(|&(_, ((_, r), (_, a)))| (r, Reverse(a[1])));
    rows(v.into_iter().map(|(_, ((u, r), (p, a)))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title"]));
        f.extend(a.map(V::I));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, COUNT(DISTINCT c.Id) AS CommentCount,
//        COUNT(DISTINCT v.Id) FILTER (WHERE v.VoteTypeId = 2) AS UpVotes, COUNT(DISTINCT v.Id) FILTER (WHERE v.VoteTypeId = 3) AS DownVotes,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= '2023-01-01' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName, p.OwnerUserId),
// FilteredPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, OwnerDisplayName, CommentCount, UpVotes, DownVotes FROM RankedPosts WHERE PostRank <= 5)
// SELECT f.OwnerDisplayName, COUNT(DISTINCT f.PostId) AS PostsCount, SUM(f.Score) AS TotalScore, SUM(f.ViewCount) AS TotalViews, AVG(f.CommentCount) AS AvgComments,
//        AVG(f.UpVotes) AS AvgUpVotes, AVG(f.DownVotes) AS AvgDownVotes
// FROM FilteredPosts f GROUP BY f.OwnerDisplayName ORDER BY PostsCount DESC, TotalScore DESC LIMIT 10;
fn q8717(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, view_count, .. } = &db.post;
    let w = db.post.with(creation_date.ge(date(2023, 1, 1))).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let g = || (&tp).group_by(Ident::<Post>::new());
    let dc = g().select(comments_of(db)).count_distinct();
    let du = g().select(votes_of(db).with((&db.vote.vote_type_id).eq(2))).count_distinct();
    let dd = g().select(votes_of(db).with((&db.vote.vote_type_id).eq(3))).count_distinct();
    let s = (&tp)
        .group_by(owner_user.select(&db.user.display_name))
        .select(score.and(view_count.opt()).and((&dc).opt()).and((&du).opt()).and((&dd).opt()))
        .fold([0i64; 7], |a, ((((s, w), c), u), d)| {
            [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + c.unwrap_or(0), a[5] + u.unwrap_or(0), a[6] + d.unwrap_or(0)]
        });
    let v = top_n(drain(&s), |&(_, a)| (Reverse(a[0]), Reverse(a[1])), 10);
    rows(v.into_iter().map(|(n, a)| row(vec![V::S(n), V::I(a[0]), V::I(a[1]), nullable(a[3], a[2]), avg(a[4], a[0]), avg(a[5], a[0]), avg(a[6], a[0])])))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.Score, P.CreationDate, P.OwnerUserId, U.Reputation AS OwnerReputation,
//        ROW_NUMBER() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC, P.CreationDate DESC) AS Rank
//     FROM Posts P LEFT JOIN Users U ON P.OwnerUserId = U.Id WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// RecentBadges AS (SELECT B.UserId, B.Name AS BadgeName, B.Class, RANK() OVER (PARTITION BY B.UserId ORDER BY B.Date DESC) AS BadgeRank FROM Badges B WHERE B.Class IN (1, 2)),
// ClosedPosts AS (SELECT PH.PostId, PH.CreationDate, PH.UserDisplayName AS ClosedBy, C.Name AS CloseReason FROM PostHistory PH
//     JOIN CloseReasonTypes C ON CAST(PH.Comment AS INTEGER) = C.Id WHERE PH.PostHistoryTypeId = 10)
// SELECT R.PostId, R.Title, R.Score, R.CreationDate, R.OwnerReputation, RB.BadgeName, RB.Class AS BadgeClass, CP.ClosedBy, CP.CloseReason
// FROM RankedPosts R LEFT JOIN RecentBadges RB ON R.OwnerUserId = RB.UserId AND RB.BadgeRank = 1 LEFT JOIN ClosedPosts CP ON R.PostId = CP.PostId
// WHERE R.Rank <= 5 ORDER BY R.PostId;
fn q31447(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, owner_user_id, origid, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|(((p, _), _), _)| p).collect();
    let Badge { user_id, class, date: bd, .. } = &db.badge;
    let rw = db.badge.with(class.is_in([1, 2])).group_by(user_id).select(Ident::<Badge>::new().and(bd)).window(rank, |(_, d)| Reverse(d), asc);
    let rb_of: HashIdx<i64, Id<Badge>> = (&rw).filt(|(_, r)| r == 1).map(|((b, _), _)| b).collect();
    let PostHistory { post_history_type_id, comment, .. } = &db.post_history;
    let crids: HashIdx<i64, Id<CloseReasonType>> = (&db.close_reason_type.origid).inv().collect();
    let closed: HashIdx<Id<Post>, Id<PostHistory>> = db.post_history.with(post_history_type_id.eq(10)).select(&db.post_history.post).inv().collect();
    let cp = (&closed).select(Ident::<PostHistory>::new().and(comment.map(|c: Str| c.trim().parse::<i64>().unwrap()).select(&crids)));
    let mut v = drain((&tp).select(owner_user_id.select(&rb_of).opt().and(cp.opt())));
    v.sort_by_key(|&(p, _)| origid.get(p).unwrap());
    rows(v.into_iter().map(|(p, (b, c))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "created", "rep"]);
        f.extend(match b {
            Some(b) => [V::S(db.badge.name.get(b).unwrap()), V::I(class.get(b).unwrap())],
            None => [V::Null, V::Null],
        });
        f.extend(match c {
            Some((h, r)) => [harness::fmt::ostr(db.post_history.user_display_name.get(h)), V::S(db.close_reason_type.name.get(r).unwrap())],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn, p.OwnerUserId
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// MostActiveUsers AS (SELECT p.OwnerUserId, COUNT(*) AS PostCount, SUM(p.Score) AS TotalScore FROM Posts p WHERE p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 month'
//     GROUP BY p.OwnerUserId HAVING COUNT(*) > 5)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, ur.DisplayName, ur.Reputation, ur.BadgeCount, mau.PostCount, mau.TotalScore
// FROM RankedPosts rp JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId JOIN MostActiveUsers mau ON rp.OwnerUserId = mau.OwnerUserId WHERE rp.rn = 1
// ORDER BY rp.Score DESC, rp.ViewCount DESC;
fn q7866(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, view_count, .. } = &db.post;
    let mau = db.post.with(creation_date.gt(add_months(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(owner_user).select(score).fold((0i64, 0i64), |(n, s), x| (n + 1, s + x));
    let w = db
        .post
        .with(post_type_id.eq(1).and(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let mut v = drain((&tp).select(Ident::<Post>::new().and(owner_user.select(Ident::<User>::new().and(&bc).and((&mau).filt(|(n, _)| n > 5))))));
    v.sort_by_key(|&(_, (p, _))| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    });
    rows(v.into_iter().map(|(_, (p, ((u, b), (n, s))))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::I(b), V::I(n), V::I(s)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT Users.Id AS UserId, Users.Reputation, COUNT(DISTINCT Posts.Id) AS PostCount, SUM(Posts.ViewCount) AS TotalViews, SUM(Posts.Score) AS TotalScore
//     FROM Users LEFT JOIN Posts ON Users.Id = Posts.OwnerUserId GROUP BY Users.Id, Users.Reputation),
// TopUsers AS (SELECT UserId, Reputation, PostCount, TotalViews, TotalScore, ROW_NUMBER() OVER (ORDER BY TotalScore DESC) AS Rank FROM UserStats),
// PostDetails AS (SELECT Posts.Id AS PostId, Posts.Title, Posts.CreationDate, Posts.Score, Posts.ViewCount, Users.DisplayName AS OwnerDisplayName, PostTypes.Name AS PostTypeName, Posts.OwnerUserId
//     FROM Posts JOIN Users ON Posts.OwnerUserId = Users.Id JOIN PostTypes ON Posts.PostTypeId = PostTypes.Id)
// SELECT TopUsers.UserId, TopUsers.Reputation, TopUsers.PostCount, TopUsers.TotalViews, TopUsers.TotalScore, PostDetails.PostId, PostDetails.Title, PostDetails.CreationDate,
//        PostDetails.Score, PostDetails.ViewCount, PostDetails.OwnerDisplayName, PostDetails.PostTypeName
// FROM TopUsers JOIN PostDetails ON TopUsers.UserId = PostDetails.OwnerUserId WHERE TopUsers.Rank <= 10 ORDER BY TopUsers.Rank, PostDetails.Score DESC;
fn q13665(db: &'static So) -> String {
    let tu = rel(top_n(drain(&user_posts(db)), |&(u, a)| (a[1] == 0, Reverse(a[4]), u), 10));
    let mut v = drain((&tu).map(|(u, _)| u).select(posts_of(db)));
    v.sort_by_key(|&(i, p)| (i, Reverse(db.post.score.get(p).unwrap())));
    rows(v.into_iter().map(|(i, p)| {
        let (u, a) = tu.get(i).unwrap();
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend([V::I(a[1]), nullable(a[6], a[5]), nullable(a[4], a[1])]);
        f.extend(post_fields(db, p, &["id", "title", "created", "score", "views", "owner", "type"]));
        row(f)
    }))
}

// WITH ranked_posts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank FROM Posts p INNER JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1),
// top_users AS (SELECT OwnerDisplayName, COUNT(*) AS PostCount, SUM(Score) AS TotalScore, SUM(ViewCount) AS TotalViews FROM ranked_posts WHERE Rank <= 5 GROUP BY OwnerDisplayName),
// badges_summary AS (SELECT u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.DisplayName)
// SELECT tu.OwnerDisplayName, tu.PostCount, tu.TotalScore, tu.TotalViews, bs.BadgeCount, bs.GoldBadges, bs.SilverBadges, bs.BronzeBadges
// FROM top_users tu INNER JOIN badges_summary bs ON tu.OwnerDisplayName = bs.DisplayName ORDER BY tu.TotalScore DESC, tu.PostCount DESC LIMIT 10;
fn q5953(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, creation_date, view_count, .. } = &db.post;
    let w = db
        .post
        .with(post_type_id.eq(1))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|(((p, _), _), _)| p).collect();
    let tu = (&tp).group_by(owner_user.select(&db.user.display_name)).select(score.and(view_count.opt())).fold([0i64; 4], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]);
    let bs = db.user.group_by(&db.user.display_name).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let v = top_n(drain((&tu).and(&bs)), |&(_, (a, _))| (Reverse(a[1]), Reverse(a[0])), 10);
    rows(v.into_iter().map(|(n, (a, b))| {
        let mut f = vec![V::S(n), V::I(a[0]), V::I(a[1]), nullable(a[3], a[2])];
        f.extend(b.map(V::I));
        row(f)
    }))
}

// WITH FilteredPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, u.DisplayName AS OwnerName,
//        LENGTH(STRING_AGG(p.Tags, '')) - LENGTH(REPLACE(STRING_AGG(p.Tags, ''), '>', '')) + 1 AS TagCount, COUNT(c.Id) AS CommentCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Body, p.CreationDate, u.DisplayName)
// SELECT fp.PostId, fp.Title, fp.OwnerName, fp.CreationDate, fp.TagCount, fp.CommentCount, fp.UpVoteCount, fp.DownVoteCount, (fp.UpVoteCount - fp.DownVoteCount) AS NetVoteScore,
//        CASE WHEN fp.TagCount > 5 THEN 'High Tags' WHEN fp.TagCount BETWEEN 3 AND 5 THEN 'Moderate Tags' ELSE 'Low Tags' END AS TagIntensity,
//        (SELECT COUNT(*) FROM PostHistory ph WHERE ph.PostId = fp.PostId AND ph.PostHistoryTypeId IN (10, 11)) AS CloseStatusChanges
// FROM FilteredPosts fp ORDER BY NetVoteScore DESC, fp.CreationDate DESC LIMIT 20;
//
// STRING_AGG runs over the joined rows, so the Tags string repeats once per
// comment-and-vote pair and its '>' count grows with it.
fn q27176(db: &'static So) -> String {
    let Post { post_type_id, tags_str, creation_date, .. } = &db.post;
    let s = db
        .post
        .with(post_type_id.eq(1))
        .group_by(Ident::<Post>::new())
        .select(tags_str.opt().and(comments_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold((None, [0i64; 3]), |(g, a): (Option<i64>, [i64; 3]), ((t, c), v)| {
            let g = match t {
                Some(t) => Some(g.unwrap_or(0) + t.matches('>').count() as i64),
                None => g,
            };
            (g, [a[0] + c.is_some() as i64, a[1] + (v == Some(2)) as i64, a[2] + (v == Some(3)) as i64])
        });
    let v = top_n(drain(&s), |&(p, (_, a))| (Reverse(a[1] - a[2]), Reverse(creation_date.get(p).unwrap())), 20);
    let tp: MatSet<Id<Post>> = rel(v.iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cs = (&tp).group_by(Ident::<Post>::new()).select(history_of(db).with((&db.post_history.post_history_type_id).is_in([10, 11])).opt()).fold(0i64, |n, h| n + h.is_some() as i64);
    rows(v.into_iter().map(|(p, (g, a))| {
        let tc = g.map(|g| g + 1);
        let mut f = post_fields(db, p, &["id", "title", "owner", "created"]);
        f.push(tc.map_or(V::Null, V::I));
        f.extend(a.map(V::I));
        f.push(V::I(a[1] - a[2]));
        f.push(match tc {
            Some(t) if t > 5 => V::S("High Tags"),
            Some(t) if (3..=5).contains(&t) => V::S("Moderate Tags"),
            _ => V::S("Low Tags"),
        });
        f.push(V::I(cs.get(p).unwrap()));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, u.DisplayName AS OwnerDisplayName,
//        RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS PostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopAnsweredPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.OwnerDisplayName FROM RankedPosts rp WHERE rp.PostRank <= 10 AND rp.AnswerCount > 0),
// TopPostsWithComments AS (SELECT tap.PostId, tap.Title, tap.CreationDate, tap.Score, tap.ViewCount, tap.OwnerDisplayName, COUNT(c.Id) AS CommentCount
//     FROM TopAnsweredPosts tap LEFT JOIN Comments c ON tap.PostId = c.PostId GROUP BY tap.PostId, tap.Title, tap.CreationDate, tap.Score, tap.ViewCount, tap.OwnerDisplayName)
// SELECT t.Title, t.OwnerDisplayName, t.Score, t.ViewCount, t.CommentCount, t.CreationDate FROM TopPostsWithComments t ORDER BY t.Score DESC, t.ViewCount DESC, t.CommentCount DESC LIMIT 20;
fn q5276(db: &'static So) -> String {
    let Post { creation_date, owner_user, post_type_id, score, view_count, answer_count, .. } = &db.post;
    let key = |p: Id<Post>| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    };
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score.and(view_count.opt())))
        .window(rank, |(_, (s, w))| (Reverse(s), w.is_none(), Reverse(w)), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|((p, _), _)| p).collect();
    let ta = (&tp).with(answer_count.gt(0));
    let v = drain((&ta).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64));
    let v = top_n(v, |&(p, n)| (key(p), Reverse(n)), 20);
    rows(v.into_iter().map(|(p, n)| {
        let mut f = post_fields(db, p, &["title", "owner", "score", "views"]);
        f.push(V::I(n));
        f.extend(post_fields(db, p, &["created"]));
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(p.Score) AS TotalScore, MAX(p.CreationDate) AS LastPostDate
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName),
// TagPostCounts AS (SELECT t.TagName, COUNT(p.Id) AS PostCount, SUM(LENGTH(p.Body) - LENGTH(REPLACE(p.Body, ' ', ''))) + COUNT(p.Id) AS TotalWordCount
//     FROM Tags t JOIN Posts p ON p.Tags LIKE '%<' || t.TagName || '>' || '%' GROUP BY t.TagName),
// TopTags AS (SELECT TagName, PostCount, TotalWordCount, ROW_NUMBER() OVER (ORDER BY PostCount DESC, TotalWordCount DESC) AS rn FROM TagPostCounts)
// SELECT ups.UserId, ups.DisplayName, ups.TotalPosts, ups.QuestionCount, ups.AnswerCount, ups.TotalScore, ups.LastPostDate, tt.TagName, tt.PostCount, tt.TotalWordCount
// FROM UserPostStats ups JOIN TopTags tt ON tt.rn <= 5 WHERE ups.TotalPosts > 20 ORDER BY ups.TotalScore DESC, ups.LastPostDate DESC;
//
// Tag names hold no '<' or '>', so the LIKE matches exactly the posts listing
// that tag.
fn q29374(db: &'static So) -> String {
    let Post { tags_str, body, .. } = &db.post;
    let tagged: HashIdx<Str, Id<Post>> = db.post.select(tags_str.flat_map(tag_list)).inv().collect();
    let tc = db
        .tag
        .group_by(&db.tag.tag_name)
        .select((&db.tag.tag_name).select(&tagged).select(body.opt()))
        .fold((0i64, None), |(n, w): (i64, Option<i64>), b| {
            (n + 1, match b {
                Some(b) => Some(w.unwrap_or(0) + b.matches(' ').count() as i64),
                None => w,
            })
        });
    let tt = rel(top_n(drain(&tc), |&(_, (n, w))| (Reverse(n), w.is_none(), Reverse(w)), 5));
    let up = user_posts(db);
    let mut v = drain(db.user.with((&db.user.reputation).gt(1000)).select((&up).filt(|a| a[1] > 20)).cross(&tt));
    v.sort_by_key(|&((u, _), (a, _))| (Reverse(a[4]), Reverse(a[7]), u));
    rows(v.into_iter().map(|((u, _), (a, (t, (n, w))))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4]), tmax(a[7]), V::S(t), V::I(n), w.map_or(V::Null, |w| V::I(w + n))]);
        row(f)
    }))
}

// WITH TagUsage AS (SELECT UNNEST(string_to_array(SUBSTRING(Tags FROM 2 FOR LENGTH(Tags) - 2), '>_<')) AS Tag, Id AS PostId FROM Posts WHERE PostTypeId = 1),
// UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS TotalQuestions, COALESCE(SUM(P.ViewCount), 0) AS TotalViews,
//        COALESCE(SUM(P.CommentCount), 0) AS TotalComments, COALESCE(SUM(P.AnswerCount), 0) AS TotalAnswers
//     FROM Users U JOIN Posts P ON U.Id = P.OwnerUserId WHERE P.PostTypeId = 1 GROUP BY U.Id, U.DisplayName),
// TagStats AS (SELECT T.TagName, COUNT(TU.PostId) AS UsageCount, COUNT(DISTINCT TU.PostId) AS DistinctPostCount, SUM(UA.TotalViews) AS TotalViews, SUM(UA.TotalComments) AS TotalComments
//     FROM TagUsage TU JOIN Tags T ON TU.Tag = T.TagName JOIN UserActivity UA ON TU.PostId = UA.UserId GROUP BY T.TagName)
// SELECT TS.TagName, TS.UsageCount, TS.DistinctPostCount, TS.TotalViews, TS.TotalComments,
//        CASE WHEN TS.UsageCount > 100 THEN 'Very Popular' WHEN TS.UsageCount BETWEEN 50 AND 100 THEN 'Popular' ELSE 'Less Popular' END AS PopularityCategory
// FROM TagStats TS ORDER BY TS.UsageCount DESC;
//
// The '>_<' separator never occurs, so a question's one element is its whole
// tag list, which names a tag only when it lists a single one. The PostId is
// matched against a User Id.
fn q29018(db: &'static So) -> String {
    let Post { post_type_id, tags_str, view_count, comment_count, answer_count, owner_user, origid, .. } = &db.post;
    let names: HashIdx<Str, Id<Tag>> = (&db.tag.tag_name).inv().collect();
    let ua = db
        .post
        .with(post_type_id.eq(1))
        .group_by(owner_user)
        .select(view_count.opt().and(comment_count).and(answer_count.opt()))
        .fold([0i64; 3], |a, ((w, c), n)| [a[0] + w.unwrap_or(0), a[1] + c, a[2] + n.unwrap_or(0)]);
    let uids: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let tu = db
        .post
        .with(post_type_id.eq(1))
        .select(
            tags_str
                .flat_map(|t: Str| t[1..t.len() - 1].split(">_<").collect::<Vec<_>>())
                .select(&names)
                .select(Ident::<Tag>::new().and(&db.tag.tag_name))
                .and(Ident::<Post>::new().and(origid.select(&uids).select(&ua))),
        );
    type R = ((Id<Tag>, Str), (Id<Post>, [i64; 3]));
    let tu: MatSet<R> = tu.collect();
    let by_name = || (&tu).group_by(Same::<R>::new().map(|((_, n), _): R| n));
    let g = by_name().select(Same::<R>::new()).fold([0i64; 3], |a, (_, (_, x))| [a[0] + 1, a[1] + x[0], a[2] + x[1]]);
    let dp = by_name().select(Same::<R>::new().map(|(_, (p, _)): R| p)).count_distinct();
    let mut v = drain((&g).and(&dp));
    v.sort_by_key(|&(_, (a, _))| Reverse(a[0]));
    rows(v.into_iter().map(|(t, (a, d))| {
        let n = a[0];
        let cat = if n > 100 { "Very Popular" } else if n >= 50 { "Popular" } else { "Less Popular" };
        row(vec![V::S(t), V::I(n), V::I(d), V::I(a[1]), V::I(a[2]), V::S(cat)])
    }))
}

// WITH UserVoteStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(v.Id) AS TotalVotes, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName),
// PostStats AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(c.Id) AS TotalComments, COUNT(a.Id) AS TotalAnswers
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2 GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount),
// TopPosts AS (SELECT ps.*, ROW_NUMBER() OVER (ORDER BY ps.Score DESC, ps.ViewCount DESC) AS Rank FROM PostStats ps)
// SELECT uvs.UserId, uvs.DisplayName, uvs.TotalVotes, uvs.UpVotes, uvs.DownVotes, tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.TotalComments, tp.TotalAnswers
// FROM UserVoteStats uvs JOIN TopPosts tp ON uvs.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = tp.PostId) WHERE tp.Rank <= 10 ORDER BY tp.Score DESC, tp.ViewCount DESC;
fn q11389(db: &'static So) -> String {
    let Post { score, view_count, owner_user, .. } = &db.post;
    let key = |p: Id<Post>| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    };
    let top = top_n(drain(db.post.select(score)), |&(p, _)| (key(p), p), 10);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ps = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(answers_of(db).opt()))
        .fold((0i64, 0i64), |(c, a), (x, y)| (c + x.is_some() as i64, a + y.is_some() as i64));
    let uv = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64],
        None => a,
    });
    let mut v = drain((&ps).and(owner_user.select(Ident::<User>::new().and(&uv))));
    v.sort_by_key(|&(p, _)| key(p));
    rows(v.into_iter().map(|(p, ((c, a), (u, x)))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(x.map(V::I));
        f.extend(post_fields(db, p, &["id", "title", "created", "score", "views"]));
        f.extend([V::I(c), V::I(a)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.Body, p.CreationDate, p.ViewCount, p.Score, p.Tags, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) as PostRank
//     FROM Posts p WHERE p.PostTypeId = 1),
// TopQuestions AS (SELECT rp.Id, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, ARRAY_LENGTH(string_to_array(rp.Tags, '>'), 1) AS TagCount FROM RankedPosts rp WHERE rp.PostRank <= 10),
// UserEngagement AS (SELECT u.Id AS UserId, u.Reputation, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount FROM Users u LEFT JOIN Comments c ON u.Id = c.UserId LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.Reputation)
// SELECT tq.Title, tq.CreationDate, tq.ViewCount, tq.Score, tq.TagCount, ue.UserId, ue.Reputation, ue.CommentCount, ue.UpVoteCount, ue.DownVoteCount
// FROM TopQuestions tq JOIN UserEngagement ue ON ue.UserId IN (SELECT OwnerUserId FROM Posts WHERE Id = tq.Id) ORDER BY tq.Score DESC, tq.ViewCount DESC;
fn q26672(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, owner_user, tags_str, view_count, .. } = &db.post;
    let top = top_n(drain(db.post.with(post_type_id.eq(1)).select(score.and(creation_date))), |&(_, (s, d))| (Reverse(s), Reverse(d)), 10);
    let tp = rel(top.into_iter().map(|x| x.0).collect());
    let owners: MatSet<Id<User>> = (&tp).select(owner_user).collect();
    let ue = (&owners)
        .group_by(Ident::<User>::new())
        .select(comments_by(db).opt().and(votes_by(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let mut v = drain((&tp).select(Ident::<Post>::new().and(owner_user.select(Ident::<User>::new().and(&ue)))));
    v.sort_by_key(|&(_, (p, _))| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    });
    rows(v.into_iter().map(|(_, (p, (u, a)))| {
        let mut f = post_fields(db, p, &["title", "created", "views", "score"]);
        f.push(tags_str.get(p).map_or(V::Null, |t| V::I(t.matches('>').count() as i64 + 1)));
        f.extend(ucols(db, u, &["uid", "rep"]));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts,
//        SUM(CASE WHEN p.Score < 0 THEN 1 ELSE 0 END) AS NegativePosts, SUM(p.ViewCount) AS TotalViews
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT ..., RANK() OVER (ORDER BY TotalPosts DESC) AS RankByPosts, RANK() OVER (ORDER BY TotalViews DESC) AS RankByViews FROM UserPostStats)
// SELECT tu.DisplayName, tu.TotalPosts, tu.TotalQuestions, tu.TotalAnswers, tu.PositivePosts, tu.NegativePosts, tu.TotalViews,
//        CASE WHEN tu.RankByPosts <= 10 THEN 'Top Contributor' WHEN tu.RankByViews <= 10 THEN 'Popular User' ELSE 'Regular User' END AS UserCategory
// FROM TopUsers tu WHERE tu.TotalPosts > 50 OR tu.TotalViews > 1000 ORDER BY tu.TotalPosts DESC, tu.TotalViews DESC;
fn q8708(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(view_count.opt())).opt())
        .fold([0i64; 7], |a, p| match p {
            Some(((t, s), w)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (s > 0) as i64, a[4] + (s < 0) as i64, a[5] + w.is_some() as i64, a[6] + w.unwrap_or(0)],
            None => a,
        });
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&s)).window(rank, |(_, a)| Reverse(a[0]), asc);
    let w = (&w).window(rank, |((_, a), _)| (a[5] == 0, Reverse(a[6])), asc);
    let mut v: Vec<_> = drain((&w).filt(|(((_, a), _), _)| a[0] > 50 || (a[5] > 0 && a[6] > 1000))).into_iter().map(|x| x.1).collect();
    v.sort_by_key(|&(((_, a), _), _)| (Reverse(a[0]), a[5] == 0, Reverse(a[6])));
    rows(v.into_iter().map(|(((u, a), p), w)| {
        let cat = if p <= 10 { "Top Contributor" } else if w <= 10 { "Popular User" } else { "Regular User" };
        row(vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4]), nullable(a[6], a[5]), V::S(cat)])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT b.Id) AS BadgeCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS PostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Badges b ON p.OwnerUserId = b.UserId WHERE p.PostTypeId = 1
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, RANK() OVER (ORDER BY u.Reputation DESC) AS UserRank FROM Users u WHERE u.Reputation > 0),
// PostDetails AS (SELECT r.PostId, r.Title, r.CreationDate, r.Score, r.ViewCount, r.CommentCount, t.UserId, t.DisplayName AS TopUserDisplayName, t.UserRank
//     FROM RankedPosts r INNER JOIN TopUsers t ON r.PostRank = 1)
// SELECT pd.Title, pd.CreationDate, pd.Score, pd.ViewCount, pd.CommentCount, pd.TopUserDisplayName, pd.UserRank FROM PostDetails pd WHERE pd.UserRank <= 10
// ORDER BY pd.Score DESC, pd.CreationDate DESC;
fn q9499(db: &'static So) -> String {
    let Post { post_type_id, owner_user_id, score, creation_date, .. } = &db.post;
    let w = db
        .post
        .with(post_type_id.eq(1))
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|(((p, _), _), _)| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db)).count_distinct();
    let tw = whole(db.user.iq()).select(Ident::<User>::new().with((&db.user.reputation).gt(0)).and(&db.user.reputation)).window(rank, |(_, r)| Reverse(r), asc);
    let tu: MatSet<((Id<User>, i64), i64)> = (&tw).filt(|(_, r)| r <= 10).collect();
    let mut v = drain((&tp).select((&cc).opt()).cross(&tu));
    v.sort_by_key(|&((p, _), (_, (_, r)))| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), r));
    rows(v.into_iter().map(|((p, _), (c, ((u, _), r)))| {
        let mut f = post_fields(db, p, &["title", "created", "score", "views"]);
        f.extend([V::I(c.unwrap_or(0)), user_col(db, u, "name"), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.ViewCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.PostTypeId IN (1, 2)),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.CreationDate, rp.ViewCount, rp.OwnerDisplayName FROM RankedPosts rp WHERE rp.Rank <= 10),
// PostVotes AS (SELECT p.Id AS PostId, COUNT(v.Id) AS TotalVotes FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS TotalComments FROM Comments c GROUP BY c.PostId)
// SELECT tp.PostId, tp.Title, tp.Score, tp.CreationDate, tp.ViewCount, tp.OwnerDisplayName, pv.TotalVotes, pc.TotalComments
// FROM TopPosts tp LEFT JOIN PostVotes pv ON tp.PostId = pv.PostId LEFT JOIN PostComments pc ON tp.PostId = pc.PostId ORDER BY tp.Score DESC, tp.CreationDate DESC;
fn q5154(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, score, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.is_in([1, 2])))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|(((p, _), _), _)| p).collect();
    let pv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let pc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let mut v = drain((&pv).and((&pc).opt()));
    v.sort_by_key(|&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())));
    rows(v.into_iter().map(|(p, (n, c))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "created", "views", "owner"]);
        f.extend([V::I(n), c.map_or(V::Null, V::I)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, u.DisplayName AS Author, p.CreationDate, p.Tags,
//        ARRAY_LENGTH(string_to_array(SUBSTRING(p.Tags FROM 2 FOR LENGTH(p.Tags) - 2), '><'), 1) AS TagCount, COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 2) AS Upvotes,
//        COUNT(c.Id) AS CommentCount, SUM(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS CloseCount
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId
//     WHERE p.PostTypeId IN (1, 2) GROUP BY p.Id, p.Title, p.Body, u.DisplayName, p.CreationDate, p.Tags),
// TopPosts AS (SELECT rp.*, RANK() OVER (ORDER BY rp.Upvotes DESC) AS VoteRank, RANK() OVER (ORDER BY rp.CommentCount DESC) AS CommentRank,
//        RANK() OVER (ORDER BY rp.CloseCount DESC) AS CloseRank FROM RankedPosts rp)
// SELECT PostId, Title, Author, CreationDate, TagCount, Upvotes, CommentCount, CloseCount, LEAST(VoteRank, CommentRank, CloseRank) AS OverallRank
// FROM TopPosts WHERE TagCount > 0 ORDER BY OverallRank ASC LIMIT 10;
fn q29315(db: &'static So) -> String {
    let Post { post_type_id, tags_str, .. } = &db.post;
    let s = db
        .post
        .with(post_type_id.is_in([1, 2]))
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()).and(history_of(db).select(&db.post_history.post_history_type_id).opt()))
        .fold([0i64; 3], |a, ((v, c), h)| [a[0] + (v == Some(2)) as i64, a[1] + c.is_some() as i64, a[2] + (h == Some(10)) as i64]);
    let tc = |t: Str| t[1..t.len() - 1].split("><").count() as i64;
    let w = whole(db.post.iq()).select(Ident::<Post>::new().and(&s).and(tags_str.opt())).window(rank, |((_, a), _)| Reverse(a[0]), asc);
    let w = (&w).window(rank, |(((_, a), _), _)| Reverse(a[1]), asc);
    let w = (&w).window(rank, |((((_, a), _), _), _)| Reverse(a[2]), asc);
    let v: Vec<_> = drain((&w).filt(move |((((_, t), _), _), _)| t.map_or(false, |t| tc(t) > 0))).into_iter().map(|x| x.1).collect();
    let v = top_n(v, |&(((((p, _), _), a), b), c)| (a.min(b).min(c), p), 10);
    rows(v.into_iter().map(|(((((p, x), t), a), b), c)| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created"]);
        f.push(V::I(tc(t.unwrap())));
        f.extend(x.map(V::I));
        f.push(V::I(a.min(b).min(c)));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.PostTypeId IN (1, 2)),
// TopRankedPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.CreationDate, rp.OwnerDisplayName FROM RankedPosts rp WHERE rp.Rank <= 5),
// PostStats AS (SELECT tp.PostId, COUNT(c.Id) AS CommentCount, COUNT(v.Id) AS VoteCount, (SELECT COUNT(*) FROM PostHistory ph WHERE ph.PostId = tp.PostId) AS HistoryCount
//     FROM TopRankedPosts tp LEFT JOIN Comments c ON tp.PostId = c.PostId LEFT JOIN Votes v ON tp.PostId = v.PostId GROUP BY tp.PostId)
// SELECT tp.Title, tp.OwnerDisplayName, ps.CommentCount, ps.VoteCount, ps.HistoryCount, tp.Score, tp.ViewCount, tp.CreationDate
// FROM TopRankedPosts tp JOIN PostStats ps ON tp.PostId = ps.PostId ORDER BY ps.VoteCount DESC, tp.Score DESC;
fn q5074(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, score, view_count, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.is_in([1, 2])))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score.and(view_count.opt())))
        .window(row_number, |(p, (s, w))| (Reverse(s), w.is_none(), Reverse(w), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).opt()))
        .fold((0i64, 0i64), |(n, m), (c, v)| (n + c.is_some() as i64, m + v.is_some() as i64));
    let h = (&tp).group_by(Ident::<Post>::new()).select(history_of(db).opt()).fold(0i64, |n, x| n + x.is_some() as i64);
    let mut v = drain((&s).and(&h));
    v.sort_by_key(|&(p, ((_, m), _))| (Reverse(m), Reverse(score.get(p).unwrap())));
    rows(v.into_iter().map(|(p, ((n, m), h))| {
        let mut f = post_fields(db, p, &["title", "owner"]);
        f.extend([V::I(n), V::I(m), V::I(h)]);
        f.extend(post_fields(db, p, &["score", "views", "created"]));
        row(f)
    }))
}

// WITH TopUsers AS (SELECT Id, DisplayName, Reputation, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM Users WHERE Reputation > 1000),
// PopularPosts AS (SELECT p.Id, p.OwnerUserId, p.Title, p.Score, p.ViewCount, RANK() OVER (ORDER BY p.Score DESC, p.ViewCount DESC) AS PopularityRank FROM Posts p
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.PostTypeId = 1),
// PostDetails AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, COUNT(c.Id) AS CommentCount, COUNT(v.Id) AS VoteCount, MAX(ph.CreationDate) AS LastEditDate
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 2 LEFT JOIN PostHistory ph ON p.Id = ph.PostId
//     WHERE p.PostTypeId IN (1, 2) GROUP BY p.Id, p.Title, p.CreationDate, p.Score),
// FinalResults AS (SELECT tu.DisplayName, pp.Title, pp.Score, pd.CommentCount, pd.VoteCount, pd.LastEditDate, ROW_NUMBER() OVER (PARTITION BY tu.ReputationRank ORDER BY pp.PopularityRank) AS RowNum
//     FROM TopUsers tu JOIN PopularPosts pp ON tu.Id = pp.OwnerUserId JOIN PostDetails pd ON pp.Id = pd.PostId)
// SELECT DisplayName, Title, Score, CommentCount, VoteCount, LastEditDate FROM FinalResults WHERE RowNum <= 5 ORDER BY DisplayName, Score DESC;
fn q5136(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, view_count, owner_user, .. } = &db.post;
    let tw = whole(db.user.iq()).select(Ident::<User>::new().with((&db.user.reputation).gt(1000)).and(&db.user.reputation)).window(rank, |(_, r)| Reverse(r), asc);
    let tu: MatSet<(Id<User>, i64)> = (&tw).map(|((u, _), r)| (u, r)).collect();
    let by_user: HashIdx<Id<User>, (Id<User>, i64)> = (&tu).map(|(u, _)| u).inv().select(&tu).collect();
    let pw = whole(db.post.iq())
        .select(Ident::<Post>::new().with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.eq(1))).and(score.and(view_count.opt())))
        .window(rank, |(_, (s, w))| (Reverse(s), w.is_none(), Reverse(w)), asc);
    type PR = (Id<Post>, i64);
    let pp: MatSet<PR> = (&pw).map(|((p, _), r)| (p, r)).collect();
    let pid = || Same::<PR>::new().map(|(p, _): PR| p);
    let fw = (&pp).group_by(pid().select(owner_user).select(&by_user).map(|(_, rr)| rr)).select(Same::<PR>::new().and(pid().select(owner_user))).window(row_number, |((p, r), _)| (r, p), asc);
    type F = (Id<Post>, Id<User>);
    let fr: MatSet<F> = (&fw).filt(|(_, n)| n <= 5).map(|(((p, _), u), _)| (p, u)).collect();
    let tp: MatSet<Id<Post>> = (&fr).map(|(p, _)| p).collect();
    let up: HashIdx<Id<Post>, Id<Vote>> = db.vote.with((&db.vote.vote_type_id).eq(2)).select(&db.vote.post).inv().collect();
    let pd = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and((&up).opt()).and(history_of(db).select(&db.post_history.creation_date).opt()))
        .fold((0i64, 0i64, None), |(n, m, d): (i64, i64, Option<i64>), ((c, v), h)| (n + c.is_some() as i64, m + v.is_some() as i64, match h {
            Some(h) => Some(d.map_or(h, |d| d.max(h))),
            None => d,
        }));
    let mut v = drain((&fr).select(Same::<F>::new().and(Same::<F>::new().map(|(p, _): F| p).select(&pd))));
    v.sort_by_key(|&((p, u), _)| (db.user.display_name.get(u).unwrap(), Reverse(score.get(p).unwrap())));
    rows(v.into_iter().map(|((p, u), (_, (n, m, d)))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title", "score"]));
        f.extend([V::I(n), V::I(m), d.map_or(V::Null, V::T)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.Id) AS VoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Score, p.CreationDate, u.DisplayName),
// TopPosts AS (SELECT PostId, Title, Score, CreationDate, OwnerDisplayName, CommentCount, VoteCount FROM RankedPosts WHERE PostRank = 1 ORDER BY Score DESC, CreationDate DESC LIMIT 10)
// SELECT tp.Title, tp.Score, tp.CreationDate, tp.OwnerDisplayName, tp.CommentCount, tp.VoteCount, pht.Name AS PostHistoryType, COUNT(ph.Id) AS HistoryCount
// FROM TopPosts tp LEFT JOIN PostHistory ph ON tp.PostId = ph.PostId LEFT JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id
// GROUP BY tp.PostId, tp.Title, tp.Score, tp.CreationDate, tp.OwnerDisplayName, tp.CommentCount, tp.VoteCount, pht.Name ORDER BY tp.Score DESC;
fn q8042(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, .. } = &db.post;
    let top = top_n(drain(db.post.with(post_type_id.eq(1)).select(score.and(creation_date))), |&(_, (s, d))| (Reverse(s), Reverse(d)), 10);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let dv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db)).count_distinct();
    type R = (Id<Post>, Option<(Id<PostHistory>, Str)>);
    let hr: MatSet<R> = (&tp).select(Ident::<Post>::new().and(history_of(db).select(Ident::<PostHistory>::new().and(htype_name(db))).opt())).collect();
    let g = (&hr).group_by(Same::<R>::new().map(|(p, h): R| (p, h.map(|x| x.1)))).select(Same::<R>::new()).fold(0i64, |n, (_, h)| n + h.is_some() as i64);
    let mut v = drain(&g);
    v.sort_by_key(|&((p, _), _)| Reverse(score.get(p).unwrap()));
    rows(v.into_iter().map(|((p, h), n)| {
        let mut f = post_fields(db, p, &["title", "score", "created", "owner"]);
        f.extend([V::I(cc.get(p).unwrap()), V::I(dv.get(p).unwrap_or(0)), h.map_or(V::Null, V::S), V::I(n)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, u.DisplayName AS OwnerDisplayName, p.Score, p.AnswerCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.Score > 0),
// PopularUsers AS (SELECT OwnerUserId, COUNT(*) AS PostCount, SUM(Score) AS TotalScore, SUM(AnswerCount) AS TotalAnswers FROM RankedPosts GROUP BY OwnerUserId
//     HAVING COUNT(*) > 5 AND SUM(Score) > 50),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.OwnerDisplayName, pu.PostCount, pu.TotalScore, pu.TotalAnswers FROM RankedPosts rp
//     JOIN PopularUsers pu ON rp.OwnerUserId = pu.OwnerUserId WHERE rp.PostRank = 1)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.OwnerDisplayName, tp.PostCount, tp.TotalScore, tp.TotalAnswers, COALESCE(pl.RelatedPostId, 0) AS RelatedPostId, COALESCE(pl.LinkTypeId, 0) AS LinkTypeId
// FROM TopPosts tp LEFT JOIN PostLinks pl ON tp.PostId = pl.PostId ORDER BY tp.TotalScore DESC, tp.PostCount DESC;
fn q7525(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, creation_date, answer_count, .. } = &db.post;
    let qs = || db.post.with(post_type_id.eq(1).and(score.gt(0))).with(owner_user);
    let pu = qs().group_by(owner_user).select(score.and(answer_count.opt())).fold((0i64, 0i64, None), |(n, s, a): (i64, i64, Option<i64>), (x, y)| {
        (n + 1, s + x, match y {
            Some(y) => Some(a.unwrap_or(0) + y),
            None => a,
        })
    });
    let w = qs()
        .group_by(owner_user.select(Ident::<User>::new().with((&pu).filt(|(n, s, _)| n > 5 && s > 50))))
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let PostLink { related_post_id, link_type_id, .. } = &db.post_link;
    let mut v = drain((&tp).select(Ident::<Post>::new().and(owner_user.select(&pu)).and(links_of(db).select(related_post_id.and(link_type_id)).opt())));
    v.sort_by_key(|&(_, ((_, (n, s, _)), _))| (Reverse(s), Reverse(n)));
    rows(v.into_iter().map(|(_, ((p, (n, s, a)), l))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend([V::I(n), V::I(s), a.map_or(V::Null, V::I)]);
        let (r, t) = l.unwrap_or((0, 0));
        f.extend([V::I(r), V::I(t)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.Score > 0),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, OwnerDisplayName FROM RankedPosts WHERE Rank <= 5),
// UserBadges AS (SELECT u.DisplayName, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.DisplayName),
// ActiveUsers AS (SELECT u.Id, u.DisplayName, u.Reputation, ub.BadgeCount FROM Users u LEFT JOIN UserBadges ub ON u.DisplayName = ub.DisplayName
//     WHERE u.LastAccessDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year')
// SELECT ap.OwnerDisplayName, ap.Title, ap.CreationDate, ap.Score, ap.ViewCount, au.Reputation, au.BadgeCount
// FROM TopPosts ap JOIN ActiveUsers au ON ap.OwnerDisplayName = au.DisplayName ORDER BY ap.Score DESC, au.Reputation DESC LIMIT 50;
fn q5381(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, view_count, .. } = &db.post;
    let w = db
        .post
        .with(post_type_id.eq(1).and(score.gt(0)))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(score.and(view_count.opt())))
        .window(row_number, |(p, (s, w))| (Reverse(s), w.is_none(), Reverse(w), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let ub = db.user.group_by(&db.user.display_name).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let au: HashIdx<Str, Id<User>> = db.user.with((&db.user.last_access_date).gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(&db.user.display_name).inv().collect();
    let v = drain((&tp).select(Ident::<Post>::new().and(owner_user.select(&db.user.display_name).select(&au).select(Ident::<User>::new().and((&db.user.display_name).select(&ub).opt())))));
    let v = top_n(v, |&(_, (p, (u, _)))| (Reverse(score.get(p).unwrap()), Reverse(db.user.reputation.get(u).unwrap()), p, u), 50);
    rows(v.into_iter().map(|(_, (p, (u, b)))| {
        let mut f = post_fields(db, p, &["owner", "title", "created", "score", "views"]);
        f.extend([user_col(db, u, "rep"), b.map_or(V::Null, V::I)]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(p.Score) AS TotalScore, AVG(p.ViewCount) AS AverageViewCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId WHERE u.Reputation > 100 GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT ..., RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank FROM UserPostStats)
// SELECT u.UserId, u.DisplayName, u.PostCount, u.QuestionCount, u.AnswerCount, u.TotalScore, u.AverageViewCount, pt.Name AS PostType, pt.Id AS PostTypeId, COUNT(c.Id) AS CommentCount
// FROM TopUsers u LEFT JOIN Posts p ON u.UserId = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId INNER JOIN PostTypes pt ON p.PostTypeId = pt.Id WHERE u.ScoreRank <= 10
// GROUP BY u.UserId, u.DisplayName, u.PostCount, u.QuestionCount, u.AnswerCount, u.TotalScore, u.AverageViewCount, pt.Name, pt.Id ORDER BY u.TotalScore DESC;
fn q8702(db: &'static So) -> String {
    let ups = user_posts(db);
    let w = whole(db.user.iq()).select(Ident::<User>::new().with((&db.user.reputation).gt(100)).and(&ups)).window(rank, |(_, a)| (a[1] == 0, Reverse(a[4])), asc);
    type U = (Id<User>, [i64; 10]);
    let tu: MatSet<U> = (&w).filt(|(_, r)| r <= 10).map(|(x, _)| x).collect();
    type R = (U, (Id<Post>, (Id<PostType>, Option<Id<Comment>>)));
    let hr: MatSet<R> = (&tu)
        .select(Same::<U>::new().and(Same::<U>::new().map(|(u, _): U| u).select(posts_of(db).select(Ident::<Post>::new().and((&db.post.post_type).and(comments_of(db).opt()))))))
        .collect();
    let g = (&hr).group_by(Same::<R>::new().map(|(x, (_, (t, _))): R| (x, t))).select(Same::<R>::new()).fold(0i64, |n, (_, (_, (_, c)))| n + c.is_some() as i64);
    let mut v = drain(&g);
    v.sort_by_key(|&(((_, a), _), _)| Reverse(a[4]));
    rows(v.into_iter().map(|(((u, a), t), n)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[4], a[1]), avg(a[6], a[5]), V::S(db.post_type.name.get(t).unwrap()), V::I(db.post_type.origid.get(t).unwrap()), V::I(n)]);
        row(f)
    }))
}

// WITH UserBadges AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, BadgeCount, GoldBadges, SilverBadges, BronzeBadges, RANK() OVER (ORDER BY BadgeCount DESC) AS Rank FROM UserBadges),
// PostStats AS (SELECT P.OwnerUserId, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(P.ViewCount) AS TotalViews, SUM(P.Score) AS TotalScore FROM Posts P GROUP BY P.OwnerUserId)
// SELECT U.DisplayName, T.BadgeCount, T.GoldBadges, T.SilverBadges, T.BronzeBadges, P.TotalPosts, P.QuestionCount, P.AnswerCount, P.TotalViews, P.TotalScore
// FROM TopUsers T JOIN PostStats P ON T.UserId = P.OwnerUserId JOIN Users U ON U.Id = T.UserId WHERE T.Rank <= 10 ORDER BY T.Rank;
fn q9040(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let ups = user_posts(db);
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&ub)).window(rank, |(_, a)| Reverse(a[0]), asc);
    type T = ((Id<User>, [i64; 4]), i64);
    let tu: MatSet<T> = (&w).filt(|(_, r)| r <= 10).collect();
    let mut v = drain((&tu).select(Same::<T>::new().and(Same::<T>::new().map(|((u, _), _): T| u).select((&ups).filt(|a| a[1] > 0)))));
    v.sort_by_key(|&(_, ((_, r), _))| r);
    rows(v.into_iter().map(|(_, (((u, b), _), a))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(b.map(V::I));
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[6], a[5]), V::I(a[4])]);
        row(f)
    }))
}

// WITH UserBadges AS (SELECT U.Id as UserId, U.DisplayName, COUNT(B.Id) AS TotalBadges, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// UserPosts AS (SELECT P.OwnerUserId, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(P.ViewCount) AS TotalViewCount FROM Posts P GROUP BY P.OwnerUserId),
// UserMetrics AS (SELECT UB.UserId, UB.DisplayName, UB.TotalBadges, UB.GoldBadges, UB.SilverBadges, UB.BronzeBadges, UP.TotalPosts, UP.Questions, UP.Answers, UP.TotalViewCount
//     FROM UserBadges UB LEFT JOIN UserPosts UP ON UB.UserId = UP.OwnerUserId)
// SELECT DM.DisplayName, DM.TotalBadges, DM.GoldBadges, DM.SilverBadges, DM.BronzeBadges, DM.TotalPosts, DM.Questions, DM.Answers, DM.TotalViewCount,
//        RANK() OVER (ORDER BY DM.TotalViewCount DESC) AS ViewRank
// FROM UserMetrics DM WHERE DM.TotalBadges > 0 ORDER BY ViewRank, DM.TotalBadges DESC;
fn q27732(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let ups = user_posts(db);
    let w = whole(db.user.iq()).select(Ident::<User>::new().and((&ub).filt(|a| a[0] > 0)).and(&ups)).window(rank, |(_, a)| (a[5] == 0, Reverse(a[6])), asc);
    let mut v: Vec<_> = drain(&w).into_iter().map(|x| x.1).collect();
    v.sort_by_key(|&(((_, b), _), r)| (r, Reverse(b[0])));
    rows(v.into_iter().map(|(((u, b), a), r)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(b.map(V::I));
        f.extend(if a[1] > 0 { [V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[6], a[5])] } else { [V::Null, V::Null, V::Null, V::Null] });
        f.push(V::I(r));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, ARRAY_LENGTH(STRING_TO_ARRAY(p.Tags, '>'), 1) AS TagCount,
//        COALESCE((SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id), 0) AS CommentCount, COALESCE((SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 2), 0) AS Upvotes,
//        ROW_NUMBER() OVER (PARTITION BY pt.Name ORDER BY p.CreationDate DESC) AS RecentPostRank, ROW_NUMBER() OVER (PARTITION BY pt.Name ORDER BY p.Score DESC) AS TopScoreRank, pt.Id AS PostTypeId
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year')
// SELECT rpt.PostId, rpt.Title, rpt.CreationDate, rpt.TagCount, rpt.CommentCount, rpt.Upvotes, pt.Name AS PostType,
//        CASE WHEN rpt.RecentPostRank = 1 THEN 'Most Recent' WHEN rpt.TopScoreRank = 1 THEN 'Top Scoring' ELSE 'Regular Post' END AS PostCategory
// FROM RankedPosts rpt JOIN PostTypes pt ON rpt.PostTypeId = pt.Id WHERE rpt.TagCount > 2
// GROUP BY rpt.PostId, rpt.Title, rpt.CreationDate, rpt.TagCount, rpt.CommentCount, rpt.Upvotes, pt.Name, rpt.RecentPostRank, rpt.TopScoreRank
// ORDER BY rpt.Upvotes DESC, rpt.CommentCount DESC LIMIT 10;
fn q29895(db: &'static So) -> String {
    let Post { creation_date, score, tags_str, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let nw = recent().group_by(ptype_name(db)).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let newest: MatSet<Id<Post>> = (&nw).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let bw = recent().group_by(ptype_name(db)).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let best: MatSet<Id<Post>> = (&bw).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let up: HashIdx<Id<Post>, Id<Vote>> = db.vote.with((&db.vote.vote_type_id).eq(2)).select(&db.vote.post).inv().collect();
    let tc = tags_str.map(|t: Str| t.matches('>').count() as i64 + 1);
    let many = recent().with(tc.gt(2));
    let cc = (&many).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let uv = (&many).group_by(Ident::<Post>::new()).select((&up).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let v = top_n(drain((&cc).and(&uv).and((&newest).opt()).and((&best).opt())), |&(p, (((c, u), _), _))| (Reverse(u), Reverse(c), p), 10);
    rows(v.into_iter().map(|(p, (((c, u), n), b))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(tags_str.get(p).unwrap().matches('>').count() as i64 + 1), V::I(c), V::I(u)]);
        f.extend(post_fields(db, p, &["type"]));
        f.push(V::S(if n.is_some() { "Most Recent" } else if b.is_some() { "Top Scoring" } else { "Regular Post" }));
        row(f)
    }))
}

// WITH PostDetails AS (SELECT P.Id AS PostID, P.Title, P.Body, U.DisplayName AS OwnerName, P.CreationDate, P.LastActivityDate, COALESCE(PH.Comment, 'No comments') AS LastEditComment,
//        PH.CreationDate AS LastEditDate, COUNT(CMP.Id) AS CommentCount, COUNT(VO.Id) FILTER (WHERE VO.VoteTypeId = 2) AS Upvotes, COUNT(VO.Id) FILTER (WHERE VO.VoteTypeId = 3) AS Downvotes
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id
//     LEFT JOIN PostHistory PH ON P.Id = PH.PostId AND PH.CreationDate = (SELECT MAX(PH2.CreationDate) FROM PostHistory PH2 WHERE PH2.PostId = P.Id)
//     LEFT JOIN Comments CMP ON P.Id = CMP.PostId LEFT JOIN Votes VO ON P.Id = VO.PostId
//     GROUP BY P.Id, P.Title, P.Body, U.DisplayName, P.CreationDate, P.LastActivityDate, PH.Comment, PH.CreationDate),
// RankedPosts AS (SELECT PD.*, ROW_NUMBER() OVER (ORDER BY PD.Upvotes DESC, PD.LastActivityDate DESC) AS Rank FROM PostDetails PD)
// SELECT RP.PostID, RP.Title, RP.OwnerName, RP.CreationDate, RP.LastActivityDate, RP.LastEditComment, RP.LastEditDate, RP.CommentCount, RP.Upvotes, RP.Downvotes
// FROM RankedPosts RP WHERE RP.Rank <= 10 ORDER BY RP.Rank;
//
// A post whose latest history instant holds several entries gets a group per
// distinct comment among them, each over its own joined rows.
fn q6171(db: &'static So) -> String {
    let PostHistory { post, creation_date: hd, comment, .. } = &db.post_history;
    let md = db.post_history.group_by(post).select(hd).fold(i64::MIN, |m, d| m.max(d));
    let at: HashIdx<(Id<Post>, i64), Id<PostHistory>> = db.post_history.select(post.and(hd)).inv().collect();
    type A = (Id<Post>, Option<((Id<PostHistory>, Option<Str>), i64)>);
    let anchors: MatSet<A> = db
        .post
        .with(&db.post.owner_user)
        .select(Ident::<Post>::new().and(Ident::<Post>::new().and(&md).select(&at).select(Ident::<PostHistory>::new().and(comment.opt()).and(hd)).opt()))
        .collect();
    let g = (&anchors)
        .group_by(Same::<A>::new().map(|(p, h): A| (p, h.and_then(|x| x.0 .1), h.map(|x| x.1))))
        .select(Same::<A>::new().map(|(p, _): A| p).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let la = |p: Id<Post>| db.post.last_activity_date.get(p).unwrap();
    let v = top_n(drain(&g), |&((p, _, _), a)| (Reverse(a[1]), Reverse(la(p))), 10);
    rows(v.into_iter().map(|((p, c, d), a)| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "activity"]);
        f.extend([V::S(c.unwrap_or("No comments")), d.map_or(V::Null, V::T)]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.Tags, u.DisplayName AS Owner, COUNT(c.Id) AS CommentCount, COUNT(a.Id) AS AnswerCount,
//        RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Posts a ON p.Id = a.ParentId JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Body, p.Tags, u.DisplayName, p.OwnerUserId, p.CreationDate),
// TopContributors AS (SELECT Owner, SUM(CommentCount) AS TotalComments, SUM(AnswerCount) AS TotalAnswers, COUNT(PostId) AS QuestionCount FROM RankedPosts WHERE UserPostRank <= 10 GROUP BY Owner),
// BadgeCounts AS (SELECT u.DisplayName AS UserName, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.DisplayName)
// SELECT tc.Owner, tc.TotalComments, tc.TotalAnswers, tc.QuestionCount, COALESCE(bc.BadgeCount, 0) AS BadgeCount
// FROM TopContributors tc LEFT JOIN BadgeCounts bc ON tc.Owner = bc.UserName ORDER BY tc.TotalComments DESC, tc.TotalAnswers DESC, tc.QuestionCount DESC;
fn q29293(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1)).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(rank, |(_, d)| Reverse(d), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|((p, _), _)| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(children_of(db).opt()))
        .fold((0i64, 0i64), |(c, a), (x, y)| (c + x.is_some() as i64, a + y.is_some() as i64));
    let g = (&tp).group_by(owner_user.select(&db.user.display_name)).select(&s).fold([0i64; 3], |a, (c, x)| [a[0] + c, a[1] + x, a[2] + 1]);
    let bc = db.user.group_by(&db.user.display_name).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let mut v = drain((&g).and((&bc).opt()));
    v.sort_by_key(|&(_, (a, _))| (Reverse(a[0]), Reverse(a[1]), Reverse(a[2])));
    rows(v.into_iter().map(|(n, (a, b))| row(vec![V::S(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(b.unwrap_or(0))])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS OwnerPostRank,
//        COUNT(c.Id) AS CommentCount, COUNT(v.Id) AS VoteCount FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(p.Score) AS TotalScore, COUNT(p.Id) AS PostCount, DENSE_RANK() OVER (ORDER BY SUM(p.Score) DESC) AS UserRank
//     FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName HAVING COUNT(p.Id) > 5)
// SELECT ru.DisplayName AS TopUser, rp.Title AS TopPostTitle, rp.Score AS TopPostScore, rp.ViewCount AS TopPostViews, rp.CommentCount AS TopPostComments,
//        ru.TotalScore AS UserTotalScore, ru.PostCount AS UserPostCount
// FROM RankedPosts rp JOIN TopUsers ru ON rp.OwnerUserId = ru.UserId WHERE rp.OwnerPostRank = 1 ORDER BY ru.UserRank, rp.Score DESC LIMIT 10;
fn q9392(db: &'static So) -> String {
    let Post { creation_date, owner_user, owner_user_id, score, .. } = &db.post;
    let ups = user_posts(db);
    let tw = whole(db.user.iq()).select(Ident::<User>::new().and((&ups).filt(|a| a[1] > 5))).window(dense_rank, |(_, a)| Reverse(a[4]), asc);
    type U = ((Id<User>, [i64; 10]), i64);
    let tu: MatSet<U> = (&tw).map(|x| x).collect();
    let by_user: HashIdx<Id<User>, U> = (&tu).map(|((u, _), _)| u).inv().select(&tu).collect();
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(score))
        .window(row_number, |(p, s)| (Reverse(s), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let v = drain((&tp).select(Ident::<Post>::new().and(owner_user.select(&by_user))));
    let v = top_n(v, |&(p, (_, (_, r)))| (r, Reverse(score.get(p).unwrap()), p), 10);
    let tpp: MatSet<Id<Post>> = rel(v.iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tpp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    rows(v.into_iter().map(|(_, (p, ((u, a), _)))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title", "score", "views"]));
        f.extend([V::I(cc.get(p).unwrap()), V::I(a[4]), V::I(a[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, u.DisplayName AS OwnerDisplayName,
//        RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS RankScore,
//        (SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 2) AS UpVotes, (SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 3) AS DownVotes
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year' AND p.PostTypeId IN (1, 2)),
// PostHistoryAggregated AS (SELECT ph.PostId, MAX(ph.CreationDate) AS LastEditDate, COUNT(*) FILTER (WHERE ph.PostHistoryTypeId = 4) AS TitleEdits,
//        COUNT(*) FILTER (WHERE ph.PostHistoryTypeId = 5) AS BodyEdits FROM PostHistory ph GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.OwnerDisplayName, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, rp.UpVotes, rp.DownVotes, ph.LastEditDate, ph.TitleEdits, ph.BodyEdits
// FROM RankedPosts rp LEFT JOIN PostHistoryAggregated ph ON rp.PostId = ph.PostId WHERE rp.RankScore <= 10 ORDER BY rp.Score DESC, rp.CreationDate DESC;
fn q5375(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, score, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(date(2024, 10, 1), -1)).and(post_type_id.is_in([1, 2])))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(rank, |((_, s), d)| (Reverse(s), Reverse(d)), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|(((p, _), _), _)| p).collect();
    let uv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold((0i64, 0i64), |(u, d), t| (u + (t == Some(2)) as i64, d + (t == Some(3)) as i64));
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let pha = db.post_history.group_by(post).select(post_history_type_id.and(hd)).fold((i64::MIN, 0i64, 0i64), |(m, t, b), (x, d)| (m.max(d), t + (x == 4) as i64, b + (x == 5) as i64));
    let mut v = drain((&uv).and((&pha).opt()));
    v.sort_by_key(|&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())));
    rows(v.into_iter().map(|(p, ((u, d), h))| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "score", "views", "answers"]);
        f.extend([V::I(u), V::I(d)]);
        f.extend(match h {
            Some((m, t, b)) => [V::T(m), V::I(t), V::I(b)],
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COUNT(c.Id) AS CommentCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= '2020-01-01' GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.CommentCount, rp.UpVotes, rp.DownVotes, (rp.UpVotes - rp.DownVotes) AS VoteScore FROM RankedPosts rp
//     WHERE rp.CommentCount > 5 AND (rp.UpVotes - rp.DownVotes) > 0)
// SELECT f.Title, f.CreationDate, f.CommentCount, f.UpVotes, f.DownVotes,
//        CASE WHEN f.CommentCount > 10 THEN 'Highly Engaged' WHEN f.CommentCount BETWEEN 6 AND 10 THEN 'Moderately Engaged' ELSE 'Low Engagement' END AS EngagementLevel, pt.Name AS PostType
// FROM FilteredPosts f LEFT JOIN PostTypes pt ON pt.Id = (SELECT PostTypeId FROM Posts WHERE Id = f.PostId LIMIT 1) ORDER BY f.CreationDate DESC LIMIT 50;
fn q1805(db: &'static So) -> String {
    let Post { creation_date, .. } = &db.post;
    let s = db
        .post
        .with(creation_date.ge(date(2020, 1, 1)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = top_n(drain((&s).filt(|a| a[0] > 5 && a[1] - a[2] > 0)), |&(p, _)| Reverse(creation_date.get(p).unwrap()), 50);
    rows(v.into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["title", "created"]);
        f.extend(a.map(V::I));
        f.push(V::S(if a[0] > 10 { "Highly Engaged" } else if a[0] >= 6 { "Moderately Engaged" } else { "Low Engagement" }));
        f.extend(post_fields(db, p, &["type"]));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, u.DisplayName AS OwnerDisplayName, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS PostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days'),
// TopPosts AS (SELECT PostId, Title, OwnerDisplayName, CreationDate, Score, ViewCount, AnswerCount, CommentCount FROM RankedPosts WHERE PostRank <= 5),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(p.ViewCount) AS TotalViews, SUM(p.AnswerCount) AS TotalAnswers
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName)
// SELECT tp.PostId, tp.Title, tp.OwnerDisplayName, tp.CreationDate, tp.Score, tp.ViewCount, tp.AnswerCount, tp.CommentCount, us.DisplayName AS UserDisplayName, us.BadgeCount, us.TotalViews, us.TotalAnswers
// FROM TopPosts tp JOIN UserStats us ON tp.OwnerDisplayName = us.DisplayName ORDER BY tp.Score DESC, us.BadgeCount DESC;
fn q8662(db: &'static So) -> String {
    let Post { creation_date, owner_user, post_type_id, score, view_count, answer_count, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_days(date(2024, 10, 1), -30)))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score))
        .window(row_number, |(p, s)| (Reverse(s), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let names: MatSet<Str> = (&tp).select(owner_user.select(&db.user.display_name)).collect();
    let us = db
        .user
        .with((&db.user.display_name).select(Same::<Str>::new().with(&names)))
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(posts_of(db).select(view_count.opt().and(answer_count.opt())).opt()))
        .fold([0i64; 5], |a, (b, p)| {
            let (w, n) = p.unwrap_or((None, None));
            [a[0] + b.is_some() as i64, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + n.is_some() as i64, a[4] + n.unwrap_or(0)]
        });
    let by_name: HashIdx<Str, Id<User>> = db.user.with(&us).select(&db.user.display_name).inv().collect();
    let mut v = drain((&tp).select(Ident::<Post>::new().and(owner_user.select(&db.user.display_name).select(&by_name).select(Ident::<User>::new().and(&us)))));
    v.sort_by_key(|&(_, (p, (_, a)))| (Reverse(score.get(p).unwrap()), Reverse(a[0])));
    rows(v.into_iter().map(|(_, (p, (u, a)))| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "score", "views", "answers", "comments"]);
        f.extend([user_col(db, u, "name"), V::I(a[0]), nullable(a[2], a[1]), nullable(a[4], a[3])]);
        row(f)
    }))
}

// WITH UserVoteStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes,
//        COUNT(v.Id) AS TotalVotes FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName),
// PostStats AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount, COUNT(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 END) AS CloseCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount),
// TopPosts AS (SELECT ps.*, ROW_NUMBER() OVER (ORDER BY ps.Score DESC) AS Rank FROM PostStats ps WHERE ps.CloseCount = 0),
// TopUsers AS (SELECT uvs.*, RANK() OVER (ORDER BY uvs.TotalVotes DESC) AS VoteRank FROM UserVoteStats uvs)
// SELECT tu.DisplayName AS TopVoter, tu.UpVotes AS UpVotes, tu.DownVotes AS DownVotes, tp.Title AS TopPostTitle, tp.Score AS PostScore, tp.ViewCount AS PostViewCount
// FROM TopUsers tu INNER JOIN TopPosts tp ON tu.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = tp.PostId) WHERE tu.VoteRank <= 10 AND tp.Rank <= 10 ORDER BY tu.UpVotes DESC, tp.Score DESC;
fn q9205(db: &'static So) -> String {
    let Post { score, owner_user, .. } = &db.post;
    let ps = db
        .post
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(history_of(db).select(&db.post_history.post_history_type_id).opt()))
        .fold(0i64, |n, (_, h)| n + (h == Some(10)) as i64);
    let tp = top_n(drain((&ps).filt(|n| n == 0)), |&(p, _)| (Reverse(score.get(p).unwrap()), p), 10);
    let tp = rel(tp.into_iter().map(|x| x.0).collect());
    let uv = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + 1],
        None => a,
    });
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&uv)).window(rank, |(_, a)| Reverse(a[2]), asc);
    type U = (Id<User>, [i64; 3]);
    let tu: MatSet<U> = (&w).filt(|(_, r)| r <= 10).map(|(x, _)| x).collect();
    let tu_of: HashIdx<Id<User>, U> = (&tu).map(|(u, _)| u).inv().select(&tu).collect();
    let mut v = drain((&tp).select(Ident::<Post>::new().and(owner_user.select(&tu_of))));
    v.sort_by_key(|&(_, (p, (_, a)))| (Reverse(a[0]), Reverse(score.get(p).unwrap())));
    rows(v.into_iter().map(|(_, (p, (u, a)))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1])];
        f.extend(post_fields(db, p, &["title", "score", "views"]));
        row(f)
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("6276", q6276),
    ("26574", q26574),
    ("28577", q28577),
    ("9045", q9045),
    ("6395", q6395),
    ("9194", q9194),
    ("9913", q9913),
    ("6547", q6547),
    ("8087", q8087),
    ("5590", q5590),
    ("7516", q7516),
    ("8804", q8804),
    ("9078", q9078),
    ("13019", q13019),
    ("5690", q5690),
    ("8616", q8616),
    ("8857", q8857),
    ("8652", q8652),
    ("9717", q9717),
    ("5171", q5171),
    ("31702", q31702),
    ("9011", q9011),
    ("9087", q9087),
    ("27811", q27811),
    ("8880", q8880),
    ("7826", q7826),
    ("7118", q7118),
    ("6328", q6328),
    ("6164", q6164),
    ("8894", q8894),
    ("9458", q9458),
    ("8946", q8946),
    ("29893", q29893),
    ("8567", q8567),
    ("29138", q29138),
    ("9332", q9332),
    ("29190", q29190),
    ("7011", q7011),
    ("5342", q5342),
    ("8154", q8154),
    ("8395", q8395),
    ("13206", q13206),
    ("29091", q29091),
    ("8124", q8124),
    ("5463", q5463),
    ("8682", q8682),
    ("29569", q29569),
    ("5529", q5529),
    ("34931", q34931),
    ("9591", q9591),
    ("9451", q9451),
    ("9797", q9797),
    ("3446", q3446),
    ("1595", q1595),
    ("1755", q1755),
    ("8865", q8865),
    ("8120", q8120),
    ("8720", q8720),
    ("10523", q10523),
    ("5966", q5966),
    ("7951", q7951),
    ("8717", q8717),
    ("31447", q31447),
    ("7866", q7866),
    ("13665", q13665),
    ("5953", q5953),
    ("27176", q27176),
    ("5276", q5276),
    ("29374", q29374),
    ("29018", q29018),
    ("11389", q11389),
    ("26672", q26672),
    ("8708", q8708),
    ("9499", q9499),
    ("5154", q5154),
    ("29315", q29315),
    ("5074", q5074),
    ("5136", q5136),
    ("8042", q8042),
    ("7525", q7525),
    ("5381", q5381),
    ("8702", q8702),
    ("9040", q9040),
    ("27732", q27732),
    ("29895", q29895),
    ("6171", q6171),
    ("29293", q29293),
    ("9392", q9392),
    ("5375", q5375),
    ("1805", q1805),
    ("8662", q8662),
    ("9205", q9205),
];
