use harness::prelude::*;
use std::cmp::Reverse;

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        RANK() OVER (ORDER BY p.Score DESC, p.ViewCount DESC) AS PostRank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.OwnerDisplayName, rp.CommentCount, rp.UpVotes, rp.DownVotes FROM RankedPosts rp WHERE rp.PostRank <= 10)
// SELECT tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.OwnerDisplayName, tp.CommentCount, tp.UpVotes, tp.DownVotes, COALESCE(b.Name, 'No Badge') AS TopBadge
// FROM TopPosts tp LEFT JOIN Badges b ON tp.OwnerDisplayName = (SELECT DisplayName FROM Users WHERE Id = b.UserId) AND b.Class = 1
// ORDER BY tp.Score DESC, tp.ViewCount DESC;
//
// PostRank reads only Score and ViewCount, so the top questions are picked first and the comment x vote product is driven for those alone.
fn q9794(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, owner_user, .. } = &db.post;
    let key = |p: Id<Post>| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    };
    let v = ranked(drain(db.post.with(post_type_id.eq(1)).select(score)), |&(p, _)| key(p), false);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let gold: HashIdx<Str, Id<Badge>> = db.badge.with((&db.badge.class).eq(1)).select((&db.badge.user).select(&db.user.display_name)).inv().collect();
    let v = drain((&s).and(owner_user.select(&db.user.display_name).select(&gold).opt()));
    rows(v.into_iter().map(|(p, (a, b))| {
        let mut f = post_fields(db, p, &["title", "created", "score", "views", "owner"]);
        f.extend(a.map(V::I));
        f.push(V::S(b.map_or("No Badge", |b| db.badge.name.get(b).unwrap())));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.CreationDate, p.Score, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        ROW_NUMBER() OVER (ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= CURRENT_DATE - INTERVAL '6 months' GROUP BY p.Id, p.Title, p.ViewCount, p.CreationDate, p.Score),
// TopPosts AS (SELECT * FROM RankedPosts WHERE PostRank <= 10),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(v.BountyAmount) AS TotalBounties
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON u.Id = v.UserId WHERE u.Reputation >= 1000 GROUP BY u.Id, u.DisplayName)
// SELECT tp.Title, tp.ViewCount, tp.Score, tp.CommentCount, us.DisplayName AS TopContributor, us.BadgeCount, us.TotalBounties
// FROM TopPosts tp JOIN UserStats us ON tp.PostId IN (SELECT p.Id FROM Posts p WHERE p.OwnerUserId = us.UserId) ORDER BY tp.Score DESC;
//
// PostRank reads only CreationDate, so the ten newest questions are picked first; UserStats is folded only for their owners, the users the join can reach.
fn q9417(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_months(current_date(), -6)))).select(creation_date));
    let v = top_n(v, |&(p, d)| (Reverse(d), p), 10);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).opt()))
        .fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let owners: MatSet<Id<User>> = (&tp).select(owner_user.select(Ident::<User>::new().with((&db.user.reputation).ge(1000)))).collect();
    let us = (&owners)
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold([0i64; 3], |a, (b, x)| {
            let x = x.flatten();
            [a[0] + b.is_some() as i64, a[1] + x.is_some() as i64, a[2] + x.unwrap_or(0)]
        });
    let mut v = drain((&cc).and(owner_user.select(Ident::<User>::new().and(&us))));
    v.sort_by_key(|&(p, _)| Reverse(db.post.score.get(p).unwrap()));
    rows(v.into_iter().map(|(p, (c, (u, a)))| {
        let mut f = post_fields(db, p, &["title", "views", "score"]);
        f.extend([V::I(c), user_col(db, u, "name"), V::I(a[0]), nullable(a[2], a[1])]);
        row(f)
    }))
}

// WITH UserBadgeStats AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostStats AS (SELECT p.OwnerUserId, COUNT(p.Id) AS PostCount, AVG(p.Score) AS AvgScore, SUM(p.ViewCount) AS TotalViews FROM Posts p
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.OwnerUserId),
// UserPerformance AS (SELECT u.Id AS UserId, u.DisplayName, bs.BadgeCount, bs.GoldBadges, bs.SilverBadges, bs.BronzeBadges, ps.PostCount, ps.AvgScore, ps.TotalViews,
//        ROW_NUMBER() OVER (ORDER BY ps.PostCount DESC, bs.BadgeCount DESC) AS Ranking
//     FROM Users u LEFT JOIN UserBadgeStats bs ON u.Id = bs.UserId LEFT JOIN PostStats ps ON u.Id = ps.OwnerUserId)
// SELECT UserId, DisplayName, BadgeCount, GoldBadges, SilverBadges, BronzeBadges, PostCount, AvgScore, TotalViews, Ranking
// FROM UserPerformance WHERE Ranking <= 10 ORDER BY Ranking;
fn q9310(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let Post { owner_user, creation_date, score, view_count, .. } = &db.post;
    let ps = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(owner_user)
        .select(score.and(view_count.opt()))
        .fold([0i64; 4], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]);
    let v = drain((&ub).and((&ps).opt()));
    let v = top_n(v, |&(_, (b, p))| (p.is_none(), Reverse(p.map(|a| a[0])), Reverse(b[0])), 10);
    rows(v.into_iter().enumerate().map(|(i, (u, (b, p)))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(b.map(V::I));
        f.extend(match p {
            Some(a) => [V::I(a[0]), avg(a[1], a[0]), nullable(a[3], a[2])],
            None => [V::Null, V::Null, V::Null],
        });
        f.push(V::I(i as i64 + 1));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.Body, p.CreationDate, p.ViewCount, p.AnswerCount, p.CommentCount, U.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.Tags ORDER BY p.CreationDate DESC) AS TagRank FROM Posts p JOIN Users U ON p.OwnerUserId = U.Id WHERE p.PostTypeId = 1),
// FilteredPosts AS (SELECT Id, Title, Body, CreationDate, ViewCount, AnswerCount, CommentCount, OwnerDisplayName FROM RankedPosts WHERE TagRank <= 5),
// PostInsights AS (SELECT fp.Title, fp.ViewCount, fp.AnswerCount, fp.CommentCount, COUNT(c.Id) AS TotalComments, COUNT(DISTINCT b.Id) AS TotalBadges
//     FROM FilteredPosts fp LEFT JOIN Comments c ON fp.Id = c.PostId LEFT JOIN Users u ON fp.OwnerDisplayName = u.DisplayName LEFT JOIN Badges b ON u.Id = b.UserId
//     GROUP BY fp.Title, fp.ViewCount, fp.AnswerCount, fp.CommentCount)
// SELECT Title, ViewCount, AnswerCount, CommentCount, TotalComments, TotalBadges,
//        ROUND((ViewCount + AnswerCount * 2 + CommentCount * 0.5) / NULLIF(TotalBadges + 1, 0), 2) AS EngagementScore
// FROM PostInsights ORDER BY EngagementScore DESC LIMIT 10;
fn q26261(db: &'static So) -> String {
    let Post { post_type_id, owner_user, tags_str, creation_date, title, view_count, answer_count, comment_count, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1)).with(owner_user).select(tags_str.opt()));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let fp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let key = || title.opt().and(view_count.opt()).and(answer_count.opt()).and(comment_count);
    let by_name: HashIdx<Str, Id<User>> = (&db.user.display_name).inv().collect();
    let named = owner_user.select(&db.user.display_name).select(&by_name);
    let tc = (&fp).group_by(key()).select(comments_of(db).opt().and(named.select(badges_of(db).opt()))).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let named = owner_user.select(&db.user.display_name).select(&by_name);
    let tb = (&fp).group_by(key()).select(comments_of(db).opt().and(named.select(badges_of(db).opt()))).buf_fold(|x| distinct_some(x.into_iter().map(|(_, b)| b)));
    let v = drain((&tc).and(&tb));
    let score = |&(((_, w), a), c): &(((Option<Str>, Option<i64>), Option<i64>), i64), b: i64| -> Option<f64> {
        Some((((w? + a? * 2) as f64 + c as f64 * 0.5) / (b + 1) as f64 * 100.0).round() / 100.0)
    };
    let v = top_n(v, |(k, (_, b))| {
        let e = score(k, *b);
        (e.is_none(), Reverse(e.map(fkey)))
    }, 10);
    rows(v.into_iter().map(|(k, (n, b))| {
        let (((t, w), a), c) = k;
        row(vec![ostr(t), oint(w), oint(a), V::I(c), V::I(n), V::I(b), ofloat(score(&k, b))])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.FavoriteCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate ASC) AS PostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// AggregateStats AS (SELECT r.OwnerDisplayName, COUNT(*) AS TotalPosts, SUM(r.Score) AS TotalScore, SUM(r.ViewCount) AS TotalViews, AVG(r.AnswerCount) AS AverageAnswers,
//        AVG(r.CommentCount) AS AverageComments FROM RankedPosts r WHERE r.PostRank <= 5 GROUP BY r.OwnerDisplayName)
// SELECT a.OwnerDisplayName, a.TotalPosts, a.TotalScore, a.TotalViews, a.AverageAnswers, a.AverageComments, COALESCE(b.BadgeCount, 0) AS BadgeCount
// FROM AggregateStats a LEFT JOIN (SELECT u.DisplayName, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.DisplayName) b
//     ON a.OwnerDisplayName = b.DisplayName
// ORDER BY a.TotalScore DESC, a.TotalPosts DESC LIMIT 10;
fn q6431(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, view_count, answer_count, comment_count, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), creation_date.get(p).unwrap(), p), 5, false);
    let rp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let agg = (&rp)
        .group_by(owner_user.select(&db.user.display_name))
        .select(score.and(view_count.opt()).and(answer_count.opt()).and(comment_count))
        .fold([0i64; 7], |a, (((s, w), an), c)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + an.is_some() as i64, a[5] + an.unwrap_or(0), a[6] + c]);
    let bc = db.user.group_by(&db.user.display_name).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = drain((&agg).and((&bc).opt()));
    let v = top_n(v, |&(_, (a, _))| (Reverse(a[1]), Reverse(a[0])), 10);
    rows(v.into_iter().map(|(n, (a, b))| {
        row(vec![V::S(n), V::I(a[0]), V::I(a[1]), nullable(a[3], a[2]), avg(a[5], a[4]), avg(a[6], a[0]), V::I(b.unwrap_or(0))])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.OwnerUserId, p.Title, p.Score, p.ViewCount, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT a.Id) AS AnswerCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2 WHERE p.CreationDate >= '2022-01-01'
//     GROUP BY p.Id, p.OwnerUserId, p.Title, p.Score, p.ViewCount),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(p.Score) AS TotalScore, SUM(COALESCE(b.Class, 0)) AS TotalBadges, COUNT(DISTINCT p.Id) AS PostCount,
//        RANK() OVER (ORDER BY SUM(p.Score) DESC) AS UserRank FROM Users u LEFT JOIN Posts p ON p.OwnerUserId = u.Id LEFT JOIN Badges b ON b.UserId = u.Id GROUP BY u.Id, u.DisplayName)
// SELECT ru.DisplayName, ru.TotalScore, ru.TotalBadges, COUNT(DISTINCT rp.PostId) AS TotalPosts, SUM(rp.CommentCount) AS TotalComments, SUM(rp.AnswerCount) AS TotalAnswers
// FROM TopUsers ru JOIN RankedPosts rp ON ru.UserId = rp.OwnerUserId WHERE ru.UserRank <= 10 GROUP BY ru.DisplayName, ru.TotalScore, ru.TotalBadges
// ORDER BY ru.TotalScore DESC, ru.DisplayName;
fn q8285(db: &'static So) -> String {
    let tub = user_posts_badges(db);
    let r = ranked(drain(&tub), |&(_, a)| (a[0] == 0, Reverse(a[1])), false);
    let tu: MatSet<Id<User>> = rel(r.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let recent = Ident::<Post>::new().with((&db.post.creation_date).ge(ts(2022, 1, 1, 0, 0, 0)));
    let rp: MatSet<Id<Post>> = (&tu).select(posts_of(db).select(recent)).collect();
    let key = (&db.post.owner_user).select((&db.user.display_name).and((&tub).map(|a| (a[1], a[5]))));
    let g = (&rp)
        .group_by(key)
        .select(Ident::<Post>::new().and(comments_per_post(db)).and(typed_answers_per_post(db)))
        .fold([0i64; 3], |a, ((_, c), n)| [a[0] + 1, a[1] + c, a[2] + n]);
    let mut v = drain(&g);
    v.sort_by_key(|&((n, (s, _)), _)| (Reverse(s), n));
    rows(v.into_iter().map(|((n, (s, b)), a)| row(vec![V::S(n), V::I(s), V::I(b), V::I(a[0]), V::I(a[1]), V::I(a[2])])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, u.DisplayName AS OwnerDisplayName,
//        RANK() OVER (PARTITION BY pt.Name ORDER BY p.Score DESC, p.CreationDate DESC) AS RankByScore, COUNT(c.Id) AS TotalComments
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 YEAR' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, u.DisplayName, pt.Name),
// TopRankedPosts AS (SELECT * FROM RankedPosts WHERE RankByScore = 1),
// PostVotes AS (SELECT p.Id AS PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id)
// SELECT trp.PostId, trp.Title, trp.CreationDate, trp.Score, trp.ViewCount, trp.AnswerCount, trp.OwnerDisplayName, pv.UpVotes, pv.DownVotes, trp.TotalComments
// FROM TopRankedPosts trp JOIN PostVotes pv ON trp.PostId = pv.PostId ORDER BY trp.Score DESC, trp.CreationDate DESC;
fn q9010(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(ptype_name(db)));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())), 1, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let pv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = drain((&cc).and(&pv));
    rows(v.into_iter().map(|(p, (c, a))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c)]);
        row(f)
    }))
}

// WITH UserRankings AS (SELECT u.Id AS UserId, u.DisplayName, SUM(u.UpVotes) AS TotalUpVotes, SUM(u.DownVotes) AS TotalDownVotes, COUNT(DISTINCT p.Id) AS TotalPosts,
//        RANK() OVER (ORDER BY SUM(u.UpVotes) - SUM(u.DownVotes) DESC) AS UserRank FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// PostAggregates AS (SELECT p.Id AS PostId, p.Title, COUNT(c.Id) AS TotalComments, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes, AVG(p.Score) AS AverageScore, MAX(p.CreationDate) AS LatestActivity
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.Title)
// SELECT ur.UserId, ur.DisplayName, ur.TotalUpVotes, ur.TotalDownVotes, ur.TotalPosts, pa.PostId, pa.Title, pa.TotalComments, pa.TotalUpVotes AS PostUpVotes,
//        pa.TotalDownVotes AS PostDownVotes, pa.AverageScore, pa.LatestActivity
// FROM UserRankings ur JOIN PostAggregates pa ON ur.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = pa.PostId ORDER BY CreationDate DESC LIMIT 1)
// WHERE ur.UserRank <= 10 ORDER BY ur.UserRank, pa.TotalComments DESC;
//
// The correlated subquery returns the post's own owner, so the join is posts to their owners; the comment x vote product is driven for the top users' posts alone.
fn q5285(db: &'static So) -> String {
    let User { up_votes, down_votes, .. } = &db.user;
    let ur = db
        .user
        .group_by(Ident::<User>::new())
        .select(up_votes.and(down_votes).and(posts_of(db).opt()))
        .fold([0i64; 3], |a, ((u, d), p)| [a[0] + u, a[1] + d, a[2] + p.is_some() as i64]);
    let r = ranked(drain(&ur), |&(_, a)| Reverse(a[0] - a[1]), false);
    let tu: MatSet<Id<User>> = rel(r.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let tp: MatSet<Id<Post>> = (&tu).select(posts_of(db)).collect();
    let pa = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 4], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64, a[3] + 1]);
    let v = drain((&pa).and((&db.post.owner_user).select(Ident::<User>::new().and(&ur))));
    rows(v.into_iter().map(|(p, (a, (u, s)))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(s[0]), V::I(s[1]), V::I(s[2])]);
        f.extend(post_fields(db, p, &["id", "title"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::F(db.post.score.get(p).unwrap() as f64), V::T(db.post.creation_date.get(p).unwrap())]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, rp.OwnerDisplayName FROM RankedPosts rp WHERE rp.Rank <= 10),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c GROUP BY c.PostId),
// PostBadges AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount FROM Badges b GROUP BY b.UserId)
// SELECT tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.AnswerCount, tp.OwnerDisplayName, COALESCE(pc.CommentCount, 0) AS TotalComments, pb.BadgeCount
// FROM TopPosts tp LEFT JOIN PostComments pc ON tp.PostId = pc.PostId LEFT JOIN Users u ON tp.OwnerDisplayName = u.DisplayName LEFT JOIN PostBadges pb ON u.Id = pb.UserId
// ORDER BY tp.Score DESC, tp.ViewCount DESC;
fn q6257(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, post_type_id, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let by_name: HashIdx<Str, Id<User>> = (&db.user.display_name).inv().collect();
    let v = drain((&tp).select(comments_per_post(db).and(owner_user.select(&db.user.display_name).select(&by_name).select((&bc).opt()))));
    rows(v.into_iter().map(|(p, (c, b))| {
        let mut f = post_fields(db, p, &["title", "created", "score", "views", "answers", "owner"]);
        f.extend([V::I(c), oint(b)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.CreationDate, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName, u.Reputation, u.CreationDate),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, AnswerCount, UpVotes, DownVotes, DENSE_RANK() OVER (ORDER BY Reputation DESC) AS Rank FROM UserStats)
// SELECT tu.DisplayName, tu.Reputation, tu.PostCount, tu.AnswerCount, tu.UpVotes, tu.DownVotes, COALESCE(SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges,
//        COALESCE(SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadges, COALESCE(SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadges
// FROM TopUsers tu LEFT JOIN Badges b ON tu.UserId = b.UserId WHERE tu.Rank <= 10
// GROUP BY tu.UserId, tu.DisplayName, tu.Reputation, tu.PostCount, tu.AnswerCount, tu.UpVotes, tu.DownVotes, tu.Rank ORDER BY tu.Rank;
//
// Rank reads only Reputation, so the top users are picked first and the post x vote product is driven for them alone.
fn q8082(db: &'static So) -> String {
    let r = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), true);
    let tu: MatSet<Id<User>> = rel(r.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let us = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, x| match x {
            Some((t, v)) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64],
            None => a,
        });
    let ub = (&tu).group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| {
        [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]
    });
    let v = drain((&us).and(&ub));
    rows(v.into_iter().map(|(u, (a, b))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(a.map(V::I));
        f.extend(b.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.AnswerCount, p.CreationDate,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank, u.Reputation AS OwnerReputation, bt.Name AS BadgeType
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN PostHistory ph ON p.Id = ph.PostId
//     LEFT JOIN PostHistoryTypes bt ON ph.PostHistoryTypeId = bt.Id
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '365 days' AND p.PostTypeId IN (1, 2)),
// AggregateData AS (SELECT rp.PostId, COUNT(rp.BadgeType) AS BadgeCount, AVG(rp.OwnerReputation) AS AverageReputation, SUM(rp.ViewCount) AS TotalViews,
//        SUM(rp.Score) AS TotalScore FROM RankedPosts rp GROUP BY rp.PostId)
// SELECT ad.PostId, p.Title, ad.BadgeCount, ad.AverageReputation, ad.TotalViews, ad.TotalScore,
//        CASE WHEN ad.TotalScore > 100 THEN 'High Engagement' WHEN ad.TotalScore BETWEEN 50 AND 100 THEN 'Medium Engagement' ELSE 'Low Engagement' END AS EngagementLevel
// FROM AggregateData ad JOIN Posts p ON ad.PostId = p.Id ORDER BY ad.TotalScore DESC LIMIT 50;
fn q8775(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, score, view_count, .. } = &db.post;
    let ad = db
        .post
        .with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -365)).and(post_type_id.is_in([1, 2])))
        .group_by(Ident::<Post>::new())
        .select(owner_user.select((&db.user.reputation).and(badges_of(db).opt())).and(history_of(db).select(htype_name(db)).opt()).and(score).and(view_count.opt()))
        .fold([0i64; 6], |a, ((((r, _), h), s), w)| [a[0] + 1, a[1] + h.is_some() as i64, a[2] + r, a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0), a[5] + s]);
    let v = top_n(drain(&ad), |&(p, a)| (Reverse(a[5]), p), 50);
    rows(v.into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([V::I(a[1]), avg(a[2], a[0]), nullable(a[4], a[3]), V::I(a[5])]);
        f.push(V::S(if a[5] > 100 { "High Engagement" } else if a[5] >= 50 { "Medium Engagement" } else { "Low Engagement" }));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, MAX(u.CreationDate) AS AccountCreated
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT *, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserReputation WHERE Reputation > 0),
// SelectedUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, QuestionCount, AnswerCount, AccountCreated FROM TopUsers WHERE ReputationRank <= 10),
// PostStatistics AS (SELECT p.OwnerUserId, COUNT(p.Id) AS PostCount, AVG(p.ViewCount) AS AvgViewCount, SUM(p.Score) AS TotalScore, MAX(p.LastActivityDate) AS LastActivePostDate
//     FROM Posts p GROUP BY p.OwnerUserId)
// SELECT su.DisplayName, su.Reputation, su.QuestionCount, su.AnswerCount, ps.PostCount, ps.AvgViewCount, ps.TotalScore, ps.LastActivePostDate,
//        EXTRACT(YEAR FROM age(su.AccountCreated)) AS AccountAgeYears
// FROM SelectedUsers su JOIN PostStatistics ps ON su.UserId = ps.OwnerUserId ORDER BY su.Reputation DESC;
//
// age(ts) is measured from CURRENT_DATE (local midnight), so AccountAgeYears depends on the day it runs.
fn q9509(db: &'static So) -> String {
    let r = top_n(drain(db.user.with((&db.user.reputation).gt(0)).select(&db.user.reputation)), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(r.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let Post { post_type_id, view_count, score, last_activity_date, .. } = &db.post;
    let ps = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(view_count.opt()).and(score).and(last_activity_date)))
        .fold([0, 0, 0, 0, 0, 0, i64::MIN], |a, (((t, w), s), d)| {
            [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0), a[5] + s, a[6].max(d)]
        });
    let today = trunc_day(utc_to_ny(now_utc()));
    let tod = |t: i64| t - trunc_day(t);
    let age = |t: i64| year(today) - year(t) - ((month(today), day(today), 0) < (month(t), day(t), tod(t))) as i64;
    let mut v = drain(&ps);
    v.sort_by_key(|&(u, _)| Reverse(db.user.reputation.get(u).unwrap()));
    rows(v.into_iter().map(|(u, a)| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[0]), avg(a[4], a[3]), V::I(a[5]), V::T(a[6]), V::I(age(db.user.creation_date.get(u).unwrap()))]);
        row(f)
    }))
}

// WITH UserBadgeCounts AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId WHERE U.Reputation > 1000 GROUP BY U.Id, U.DisplayName),
// RecentPosts AS (SELECT P.Id AS PostId, P.Title, P.OwnerUserId, P.CreationDate, P.Score, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS rn
//     FROM Posts P WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' AND P.PostTypeId = 1),
// TopUsers AS (SELECT U.UserId, U.DisplayName, U.BadgeCount, U.GoldBadges, U.SilverBadges, U.BronzeBadges, RP.PostId, RP.Title, RP.CreationDate AS PostCreatedDate, RP.Score
//     FROM UserBadgeCounts U JOIN RecentPosts RP ON U.UserId = RP.OwnerUserId WHERE RP.rn = 1 ORDER BY U.BadgeCount DESC LIMIT 10)
// SELECT TU.DisplayName, TU.BadgeCount, TU.GoldBadges, TU.SilverBadges, TU.BronzeBadges, TU.Title AS RecentPostTitle, TU.PostCreatedDate, TU.Score
// FROM TopUsers TU ORDER BY TU.BadgeCount DESC, TU.Score DESC;
fn q7838(db: &'static So) -> String {
    let ub = db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let Post { creation_date, post_type_id, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)).and(post_type_id.eq(1))).select(owner_user));
    let first = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let rp: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let v = drain((&rp).select(owner_user.select(Ident::<User>::new().and(&ub))));
    let mut v = top_n(v, |&(_, (_, b))| Reverse(b[0]), 10);
    v.sort_by_key(|&(p, (_, b))| (Reverse(b[0]), Reverse(db.post.score.get(p).unwrap())));
    rows(v.into_iter().map(|(p, (u, b))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(b.map(V::I));
        f.extend(post_fields(db, p, &["title", "created", "score"]));
        row(f)
    }))
}

// WITH UserStats AS (SELECT Users.Id AS UserId, Users.DisplayName, SUM(CASE WHEN Posts.PostTypeId = 1 THEN Posts.Score ELSE 0 END) AS TotalScore,
//        COUNT(DISTINCT Posts.Id) AS TotalPosts, COUNT(DISTINCT CASE WHEN Posts.PostTypeId = 2 THEN Posts.Id END) AS TotalAnswers,
//        COUNT(DISTINCT CASE WHEN Posts.PostTypeId = 1 THEN Posts.Id END) AS TotalQuestions
//     FROM Users LEFT JOIN Posts ON Users.Id = Posts.OwnerUserId GROUP BY Users.Id, Users.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalScore, TotalPosts, TotalQuestions, RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank FROM UserStats),
// TotalBadges AS (SELECT UserId, COUNT(*) AS BadgeCount, SUM(CASE WHEN Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Badges GROUP BY UserId)
// SELECT TOPUsers.UserId, TOPUsers.DisplayName, TOPUsers.TotalScore, TOPUsers.TotalPosts, TOPUsers.TotalQuestions, TotalBadges.BadgeCount, TotalBadges.GoldBadges,
//        TotalBadges.SilverBadges, TotalBadges.BronzeBadges, TOPUsers.ScoreRank
// FROM TopUsers LEFT JOIN TotalBadges ON TopUsers.UserId = TotalBadges.UserId WHERE TOPUsers.ScoreRank <= 10 ORDER BY TOPUsers.ScoreRank;
fn q9401(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let us = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(score)).opt()).fold([0i64; 3], |a, p| match p {
        Some((t, s)) => [a[0] + if t == 1 { s } else { 0 }, a[1] + 1, a[2] + (t == 1) as i64],
        None => a,
    });
    let tb = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    let r = ranked(drain(&us), |&(_, a)| Reverse(a[0]), false);
    let top: Vec<_> = r.into_iter().take_while(|x| x.1 <= 10).collect();
    let tu = rel(top.into_iter().map(|((u, a), r)| (u, (a, r))).collect());
    let v = drain((&tu).select(Same::<(Id<User>, ([i64; 3], i64))>::new().and(Same::<(Id<User>, ([i64; 3], i64))>::new().map(|(u, _)| u).select((&tb).opt()))));
    rows(v.into_iter().map(|(_, ((u, (a, r)), b))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        f.extend(match b {
            Some(b) => b.map(V::I),
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        f.push(V::I(r));
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, AVG(COALESCE(v.Score, 0)) AS AverageScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId
//     LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 WHEN VoteTypeId = 3 THEN -1 ELSE 0 END) AS Score FROM Votes GROUP BY PostId) v ON p.Id = v.PostId
//     GROUP BY u.Id, u.DisplayName),
// PopularTags AS (SELECT TagName, COUNT(*) AS TagCount FROM (SELECT UNNEST(string_to_array(SUBSTRING(Tags, 2, LENGTH(Tags) - 2), '><')) AS TagName FROM Posts WHERE PostTypeId = 1) AS TagsList
//     GROUP BY TagName ORDER BY TagCount DESC LIMIT 5)
// SELECT ups.DisplayName, ups.TotalPosts, ups.Questions, ups.Answers, ups.AverageScore, pt.TagName, pt.TagCount
// FROM UserPostStats ups CROSS JOIN PopularTags pt WHERE ups.TotalPosts > 10 ORDER BY ups.AverageScore DESC, pt.TagCount DESC OFFSET 0 ROWS FETCH NEXT 10 ROWS ONLY;
fn q1432(db: &'static So) -> String {
    let Vote { post, vote_type_id, .. } = &db.vote;
    let vs = db.vote.group_by(post).select(vote_type_id).fold(0i64, |s, t| s + if t == 2 { 1 } else if t == 3 { -1 } else { 0 });
    let ups = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and((&vs).opt())).opt())
        .fold([0i64; 5], |a, p| match p {
            Some((t, s)) => [a[0] + 1, a[1] + 1, a[2] + (t == 1) as i64, a[3] + (t == 2) as i64, a[4] + s.unwrap_or(0)],
            None => [a[0] + 1, a[1], a[2], a[3], a[4]],
        });
    let freq = db.post.with((&db.post.post_type_id).eq(1)).select((&db.post.tags_str).flat_map(tag_list)).group_by(Same::<Str>::new()).select(Same::<Str>::new()).fold(0i64, |n, _| n + 1);
    let pt = rel(top_n(drain(&freq), |&(t, n)| (Reverse(n), t), 5));
    let mut v = Vec::new();
    (&ups).filt(|a| a[1] > 10).cross(&pt).drive(|(u, _), (a, (t, n))| v.push((u, a, t, n)));
    let v = top_n(v, |&(u, a, t, n)| (Reverse(fkey(a[4] as f64 / a[0] as f64)), Reverse(n), u, t), 10);
    rows(v.into_iter().map(|(u, a, t, n)| row(vec![user_col(db, u, "name"), V::I(a[1]), V::I(a[2]), V::I(a[3]), avg(a[4], a[0]), V::S(t), V::I(n)])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.Id) AS VoteCount,
//        RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS ScoreRank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 2
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.Score, u.DisplayName, p.OwnerUserId),
// TopRankedPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.OwnerDisplayName, rp.CommentCount, rp.VoteCount FROM RankedPosts rp WHERE rp.ScoreRank = 1),
// UserBadgeCounts AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount FROM Badges b GROUP BY b.UserId)
// SELECT trp.PostId, trp.Title, trp.CreationDate, trp.Score, trp.OwnerDisplayName, trp.CommentCount, trp.VoteCount, COALESCE(ub.BadgeCount, 0) AS UserBadges
// FROM TopRankedPosts trp LEFT JOIN UserBadgeCounts ub ON trp.OwnerDisplayName = (SELECT DisplayName FROM Users WHERE Id = ub.UserId)
// ORDER BY trp.Score DESC, trp.CommentCount DESC;
//
// ScoreRank reads only Score, so each owner's top questions are picked first and the comment x upvote product is driven for those alone.
fn q7231(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1)).select(owner_user.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| Reverse(score.get(p).unwrap()), 1, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let up = || votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2)));
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(up().opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(up().opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let by_name: HashIdx<Str, Id<User>> = (&db.user.display_name).inv().collect();
    let v = drain((&cc).and(&vc).and(owner_user.select(&db.user.display_name).select(&by_name).select(&bc).opt()));
    rows(v.into_iter().map(|(p, ((c, n), b))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "owner"]);
        f.extend([V::I(c), V::I(n), V::I(b.unwrap_or(0))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, pt.Name AS PostType, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount
//     FROM Posts p LEFT JOIN PostTypes pt ON p.PostTypeId = pt.Id LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= DATE '2022-01-01' GROUP BY p.Id, p.Title, p.CreationDate, pt.Name, u.DisplayName),
// PostStatistics AS (SELECT PostId, Title, CreationDate, PostType, OwnerDisplayName, CommentCount, UpVoteCount, DownVoteCount,
//        ROW_NUMBER() OVER (ORDER BY UpVoteCount DESC, CommentCount DESC) AS Rank FROM RankedPosts)
// SELECT ps.PostId, ps.Title, ps.CreationDate, ps.PostType, ps.OwnerDisplayName, ps.CommentCount, ps.UpVoteCount, ps.DownVoteCount, ps.Rank,
//        ROUND((CAST(ps.UpVoteCount AS FLOAT) / NULLIF((ps.UpVoteCount + ps.DownVoteCount), 0)) * 100, 2) AS UpVotePercentage
// FROM PostStatistics ps WHERE ps.Rank <= 10 ORDER BY ps.Rank;
fn q29891(db: &'static So) -> String {
    let s = db
        .post
        .with((&db.post.creation_date).ge(ts(2022, 1, 1, 0, 0, 0)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = top_n(drain(&s), |&(_, a)| (Reverse(a[1]), Reverse(a[0])), 10);
    rows(v.into_iter().enumerate().map(|(i, (p, a))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "type", "owner"]);
        f.extend(a.map(V::I));
        f.push(V::I(i as i64 + 1));
        f.push(if a[1] + a[2] == 0 { V::Null } else { V::F(((a[1] as f32 / (a[1] + a[2]) as f32 * 100.0 * 100.0).round() / 100.0) as f64) });
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END) AS TotalUpvotedPosts,
//        AVG(P.ViewCount) AS AverageViewsPerPost, RANK() OVER (ORDER BY COUNT(P.Id) DESC) AS Rank FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalUpvotedPosts, AverageViewsPerPost, Rank FROM UserPostStats WHERE Rank <= 10),
// LatestPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, U.DisplayName AS OwnerDisplayName, P.ViewCount, P.Score FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id
//     WHERE P.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days')
// SELECT TU.DisplayName, TU.TotalPosts, TU.TotalQuestions, TU.TotalAnswers, TU.TotalUpvotedPosts, TU.AverageViewsPerPost, LP.PostId, LP.Title, LP.CreationDate, LP.ViewCount, LP.Score
// FROM TopUsers TU LEFT JOIN LatestPosts LP ON LP.OwnerDisplayName = TU.DisplayName ORDER BY TU.Rank, LP.CreationDate DESC;
fn q6873(db: &'static So) -> String {
    let ups = user_posts(db);
    let r = ranked(drain(&ups), |&(_, a)| Reverse(a[1]), false);
    let tu = rel(r.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0).collect());
    let Post { creation_date, owner_user, .. } = &db.post;
    let lp: HashIdx<Str, Id<Post>> = db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 0, 0, 0), -30))).select(owner_user.select(&db.user.display_name)).inv().collect();
    type R = (Id<User>, [i64; 10]);
    let v = drain((&tu).select(Same::<R>::new().and(Same::<R>::new().map(|(u, _): R| u).select((&db.user.display_name).select(&lp).opt()))));
    rows(v.into_iter().map(|(_, ((u, a), p))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[8]), avg(a[6], a[5])];
        f.extend(match p {
            Some(p) => post_fields(db, p, &["id", "title", "created", "views", "score"]),
            None => vec![V::Null, V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH TagStats AS (SELECT t.TagName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN pt.Name = 'Answer' THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN pt.Name = 'Question' THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS TotalUpVotes,
//        SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS TotalDownVotes, AVG(u.Reputation) AS AvgUserReputation
//     FROM Tags t JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' JOIN PostTypes pt ON p.PostTypeId = pt.Id LEFT JOIN Votes v ON v.PostId = p.Id
//     LEFT JOIN VoteTypes vt ON v.VoteTypeId = vt.Id LEFT JOIN Users u ON p.OwnerUserId = u.Id GROUP BY t.TagName),
// RankingTags AS (SELECT TagName, PostCount, AnswerCount, QuestionCount, TotalUpVotes, TotalDownVotes, AvgUserReputation,
//        ROW_NUMBER() OVER (ORDER BY PostCount DESC, TotalUpVotes - TotalDownVotes DESC) AS Rank FROM TagStats)
// SELECT rt.TagName, rt.PostCount, rt.AnswerCount, rt.QuestionCount, rt.TotalUpVotes, rt.TotalDownVotes, rt.AvgUserReputation,
//        CASE WHEN rt.Rank <= 10 THEN 'Top Tag' WHEN rt.Rank <= 20 THEN 'Mid Tag' ELSE 'Low Tag' END AS TagRankCategory
// FROM RankingTags rt ORDER BY rt.Rank;
fn q28603(db: &'static So) -> String {
    let lt = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&lt).map(|(_, t)| t).inv().collect();
    let posts = || (&by_tag).map(|(p, _)| p);
    let Post { post_type, owner_user, .. } = &db.post;
    let row_of = post_type.select(&db.post_type.name).and(owner_user.select(&db.user.reputation).opt()).and(votes_of(db).select(vtype_name(db)).opt());
    let ts = db.tag.group_by(&db.tag.tag_name).select(posts().select(row_of)).fold([0i64; 6], |a, ((t, r), v)| {
        [a[0] + (t == "Answer") as i64, a[1] + (t == "Question") as i64, a[2] + (v == Some("UpMod")) as i64, a[3] + (v == Some("DownMod")) as i64, a[4] + r.is_some() as i64, a[5] + r.unwrap_or(0)]
    });
    let pc = db.tag.group_by(&db.tag.tag_name).select(posts()).fold(0i64, |n, _| n + 1);
    let v = top_n(drain((&pc).and(&ts)), |&(_, (n, a))| (Reverse(n), Reverse(a[2] - a[3])), 0);
    rows(v.into_iter().enumerate().map(|(i, (t, (n, a)))| {
        let c = if i < 10 { "Top Tag" } else if i < 20 { "Mid Tag" } else { "Low Tag" };
        row(vec![V::S(t), V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), avg(a[5], a[4]), V::S(c)])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank,
//        p.OwnerUserId FROM Posts p WHERE p.PostTypeId = 1),
// PostVotes AS (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes v GROUP BY v.PostId),
// UserBadges AS (SELECT b.UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverBadges,
//        COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeBadges FROM Badges b GROUP BY b.UserId)
// SELECT u.DisplayName, rp.PostId, rp.Title, rp.CreationDate, COALESCE(pv.UpVotes, 0) AS TotalUpVotes, COALESCE(pv.DownVotes, 0) AS TotalDownVotes, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges
// FROM Users u LEFT JOIN RankedPosts rp ON u.Id = rp.OwnerUserId LEFT JOIN PostVotes pv ON rp.PostId = pv.PostId LEFT JOIN UserBadges ub ON u.Id = ub.UserId
// WHERE rp.PostRank = 1 AND (ub.GoldBadges > 0 OR ub.SilverBadges > 0 OR ub.BronzeBadges > 0) ORDER BY rp.CreationDate DESC OFFSET 0 ROWS FETCH NEXT 10 ROWS ONLY;
fn q1517(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let first = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let rp: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pv = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let v = drain((&rp).select(owner_user.select(Ident::<User>::new().and((&ub).filt(|b| b[0] > 0 || b[1] > 0 || b[2] > 0))).and((&pv).opt())));
    let v = top_n(v, |&(p, _)| Reverse(creation_date.get(p).unwrap()), 10);
    rows(v.into_iter().map(|(p, ((u, b), a))| {
        let a = a.unwrap_or([0, 0]);
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["id", "title", "created"]));
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.extend(b.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.OwnerUserId, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank, COUNT(a.Id) AS AnswerCount
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Posts a ON p.Id = a.ParentId
//     WHERE p.CreationDate >= (cast('2024-10-01' as date) - INTERVAL '1 year') AND p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Body, p.CreationDate, p.OwnerUserId, u.DisplayName),
// TopUsers AS (SELECT OwnerUserId, COUNT(PostId) AS PostCount, SUM(AnswerCount) AS TotalAnswers FROM RankedPosts WHERE PostRank = 1 GROUP BY OwnerUserId),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id)
// SELECT u.DisplayName, u.Reputation, COALESCE(tb.PostCount, 0) AS TotalQuestions, COALESCE(tb.TotalAnswers, 0) AS TotalAnswers, COALESCE(ub.BadgeCount, 0) AS TotalBadges
// FROM Users u LEFT JOIN TopUsers tb ON u.Id = tb.OwnerUserId LEFT JOIN UserBadges ub ON u.Id = ub.UserId WHERE u.Reputation > 1000 ORDER BY u.Reputation DESC LIMIT 10;
fn q3539(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 0, 0, 0), -1)).and(post_type_id.eq(1))).select(owner_user));
    let first = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let rp: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let tb = (&rp).group_by(owner_user).select(answers_per_post(db)).fold([0i64; 2], |a, n| [a[0] + 1, a[1] + n]);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = drain(db.user.with((&db.user.reputation).gt(1000)).select((&tb).opt().and((&ub).opt())));
    let v = top_n(v, |&(u, _)| Reverse(db.user.reputation.get(u).unwrap()), 10);
    rows(v.into_iter().map(|(u, (t, b))| {
        let t = t.unwrap_or([0, 0]);
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(t[0]), V::I(t[1]), V::I(b.unwrap_or(0))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, OwnerDisplayName FROM RankedPosts WHERE Rank <= 5),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c GROUP BY c.PostId),
// PostMetrics AS (SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.OwnerDisplayName, COALESCE(pc.CommentCount, 0) AS CommentCount
//     FROM TopPosts tp LEFT JOIN PostComments pc ON tp.PostId = pc.PostId)
// SELECT pm.PostId, pm.Title, pm.CreationDate, pm.Score, pm.ViewCount, pm.CommentCount,
//        (SELECT COUNT(*) FROM Votes v WHERE v.PostId = pm.PostId AND v.VoteTypeId = 2) AS UpVotes,
//        (SELECT COUNT(*) FROM Votes v WHERE v.PostId = pm.PostId AND v.VoteTypeId = 3) AS DownVotes
// FROM PostMetrics pm ORDER BY pm.Score DESC, pm.ViewCount DESC;
fn q9755(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, post_type_id, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = drain((&pv).and(comments_per_post(db)));
    rows(v.into_iter().map(|(p, (a, c))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, P.AnswerCount, U.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC, P.CreationDate DESC) AS Rank
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 YEAR'),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, AnswerCount, OwnerDisplayName FROM RankedPosts WHERE Rank <= 5),
// PostComments AS (SELECT C.PostId, COUNT(C.Id) AS CommentCount FROM Comments C GROUP BY C.PostId),
// PostBadges AS (SELECT B.UserId, COUNT(B.Id) AS BadgeCount FROM Badges B GROUP BY B.UserId)
// SELECT TP.Title, TP.CreationDate, TP.Score, TP.ViewCount, TP.AnswerCount, TP.OwnerDisplayName, COALESCE(PC.CommentCount, 0) AS CommentCount, COALESCE(PB.BadgeCount, 0) AS OwnerBadgeCount
// FROM TopPosts TP LEFT JOIN PostComments PC ON TP.PostId = PC.PostId LEFT JOIN Users U ON TP.OwnerDisplayName = U.DisplayName LEFT JOIN PostBadges PB ON U.Id = PB.UserId
// ORDER BY TP.Score DESC, TP.CreationDate DESC;
fn q5752(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, post_type_id, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let by_name: HashIdx<Str, Id<User>> = (&db.user.display_name).inv().collect();
    let v = drain((&tp).select(comments_per_post(db).and(owner_user.select(&db.user.display_name).select(&by_name).select((&bc).opt()))));
    rows(v.into_iter().map(|(p, (c, b))| {
        let mut f = post_fields(db, p, &["title", "created", "score", "views", "answers", "owner"]);
        f.extend([V::I(c), V::I(b.unwrap_or(0))]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, COUNT(DISTINCT c.Id) AS CommentCount, SUM(b.Class) AS TotalBadges, AVG(u.Reputation) AS AvgReputation
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// UserRanking AS (SELECT UserId, DisplayName, PostCount, AnswerCount, QuestionCount, CommentCount, TotalBadges, AvgReputation,
//        RANK() OVER (ORDER BY PostCount DESC, AnswerCount DESC, QuestionCount DESC, CommentCount DESC, TotalBadges DESC) AS Rank FROM UserActivity),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, AnswerCount, QuestionCount, CommentCount, TotalBadges, AvgReputation FROM UserRanking WHERE Rank <= 10)
// SELECT tu.DisplayName, tu.PostCount, tu.AnswerCount, tu.QuestionCount, tu.CommentCount, tu.TotalBadges, tu.AvgReputation,
//        (SELECT COUNT(*) FROM Votes v WHERE v.UserId = tu.UserId) AS TotalVotes
// FROM TopUsers tu ORDER BY tu.PostCount DESC, tu.AnswerCount DESC;
fn q6741(db: &'static So) -> String {
    let Post { post_type_id, .. } = &db.post;
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(comments_of(db).opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 6], |a, (p, b)| {
            let t = p.map(|(t, _)| t);
            [a[0] + p.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(1)) as i64, a[3] + b.is_some() as i64, a[4] + b.unwrap_or(0), a[5] + 1]
        });
    let dc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db))).fold(0i64, |n, _| n + 1);
    let vc = db.user.group_by(Ident::<User>::new()).select(votes_by(db)).fold(0i64, |n, _| n + 1);
    let v = ranked(drain((&ua).and((&dc).opt())), |&(_, (a, c))| (Reverse(a[0]), Reverse(a[1]), Reverse(a[2]), Reverse(c.unwrap_or(0)), a[3] == 0, Reverse(a[4])), false);
    let tu = rel(v.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0).collect());
    type R = (Id<User>, ([i64; 6], Option<i64>));
    let v = drain((&tu).select(Same::<R>::new().and(Same::<R>::new().map(|(u, _): R| u).select((&vc).opt()))));
    rows(v.into_iter().map(|(_, ((u, (a, c)), n))| {
        let rep = db.user.reputation.get(u).unwrap();
        row(vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(c.unwrap_or(0)), nullable(a[4], a[3]), V::F(rep as f64), V::I(n.unwrap_or(0))])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS OwnerDisplayName, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT a.Id) AS AnswerCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2
//     LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, u.DisplayName, p.OwnerUserId),
// UserTopPosts AS (SELECT PostId, Title, CreationDate, OwnerDisplayName, CommentCount, AnswerCount, UpvoteCount, DownvoteCount, UserPostRank,
//        DENSE_RANK() OVER (ORDER BY UpvoteCount DESC) AS UpvoteRank FROM RankedPosts)
// SELECT utp.OwnerDisplayName, utp.Title, utp.CommentCount, utp.AnswerCount, utp.UpvoteCount, utp.DownvoteCount
// FROM UserTopPosts utp WHERE utp.UserPostRank = 1 AND utp.UpvoteRank <= 10 ORDER BY utp.UpvoteCount DESC, utp.CreationDate DESC;
fn q8018(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let qs = || db.post.with(post_type_id.eq(1)).with(owner_user);
    let s = qs()
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(answers_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let r = ranked(drain(&s), |&(_, a)| Reverse(a[0]), true);
    let top: MatSet<Id<Post>> = rel(r.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let first = top_per(drain(qs().select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let first: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let mut v = drain((&top).with(&first).select((&s).and(comments_per_post(db)).and(typed_answers_per_post(db))));
    v.sort_by_key(|&(p, ((a, _), _))| (Reverse(a[0]), Reverse(creation_date.get(p).unwrap())));
    rows(v.into_iter().map(|(p, ((a, c), n))| {
        let mut f = post_fields(db, p, &["owner", "title"]);
        f.extend([V::I(c), V::I(n), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, u.DisplayName AS OwnerDisplayName,
//        DENSE_RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS ScoreRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.Score > 0),
// TopRankedPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.CreationDate, rp.OwnerDisplayName FROM RankedPosts rp WHERE rp.ScoreRank <= 10),
// PostStatistics AS (SELECT p.PostId, p.Title, p.OwnerDisplayName, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount
//     FROM TopRankedPosts p LEFT JOIN Comments c ON p.PostId = c.PostId LEFT JOIN Votes v ON p.PostId = v.PostId GROUP BY p.PostId, p.Title, p.OwnerDisplayName)
// SELECT ps.PostId, ps.Title, ps.OwnerDisplayName, ps.CommentCount, ps.UpvoteCount, ps.DownvoteCount, ps.UpvoteCount - ps.DownvoteCount AS NetVotes
// FROM PostStatistics ps ORDER BY ps.UpvoteCount DESC, ps.CommentCount DESC;
fn q8755(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, post_type_id, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(score.gt(0))).with(owner_user).select(post_type_id.and(score)));
    let r = per_group(ranked(v, |&(_, (t, s))| (t, Reverse(s)), true), |&(_, (t, _))| t);
    let tp: MatSet<Id<Post>> = rel(r.into_iter().filter(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = drain(&s);
    rows(v.into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[1] - a[2])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank FROM Posts p WHERE p.PostTypeId IN (1, 2)),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, AnswerCount, CommentCount FROM RankedPosts WHERE Rank <= 5),
// UserEngagement AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT v.Id) AS VoteCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName),
// PostDetails AS (SELECT tp.PostId, tp.Title, tp.Score, tp.ViewCount, tp.AnswerCount, ue.UserId, ue.DisplayName, ue.VoteCount, ue.UpVotes, ue.DownVotes
//     FROM TopPosts tp JOIN UserEngagement ue ON tp.PostId = ue.UserId)
// SELECT pd.Title, pd.Score, pd.ViewCount, pd.AnswerCount, pd.DisplayName AS EngagingUser, pd.VoteCount, pd.UpVotes, pd.DownVotes FROM PostDetails pd ORDER BY pd.Score DESC, pd.ViewCount DESC;
//
// The join compares a post id with a user id, so it goes through origid.
fn q9606(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, origid, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.is_in([1, 2])).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let users: MatSet<Id<User>> = (&tp).select(origid.select(&uidx)).collect();
    let ue = (&users).group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| {
        [a[0] + t.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let v = drain((&tp).select(origid.select(&uidx).select(Ident::<User>::new().and(&ue))));
    rows(v.into_iter().map(|(p, (u, a))| {
        let mut f = post_fields(db, p, &["title", "score", "views", "answers"]);
        f.push(user_col(db, u, "name"));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, u.DisplayName AS Author, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.Id) AS VoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Score, p.CreationDate, u.DisplayName),
// PostHistoryDetails AS (SELECT ph.PostId, MAX(CASE WHEN pht.Name = 'Post Closed' THEN ph.CreationDate END) AS LastClosedDate,
//        MAX(CASE WHEN pht.Name = 'Post Reopened' THEN ph.CreationDate END) AS LastReopenedDate, MAX(CASE WHEN pht.Name = 'Edit Body' THEN ph.CreationDate END) AS LastEditedDate
//     FROM PostHistory ph JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.Score, rp.CreationDate, rp.Author, rp.CommentCount, rp.VoteCount, phd.LastClosedDate, phd.LastReopenedDate, phd.LastEditedDate
// FROM RankedPosts rp LEFT JOIN PostHistoryDetails phd ON rp.PostId = phd.PostId WHERE rp.rn = 1 ORDER BY rp.Score DESC, rp.CreationDate DESC LIMIT 10;
//
// rn partitions by the post itself, so it is always 1; the ORDER BY reads only base columns, so the ten questions are picked first and the comment x vote product is driven for those.
fn q9955(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, .. } = &db.post;
    let v = top_n(drain(db.post.with(post_type_id.eq(1)).select(score)), |&(p, s)| (Reverse(s), Reverse(creation_date.get(p).unwrap())), 10);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let PostHistory { post, creation_date: hd, .. } = &db.post_history;
    let phd = db.post_history.group_by(post).select(htype_name(db).and(hd)).fold([i64::MIN; 3], |a, (n, d)| {
        [if n == "Post Closed" { a[0].max(d) } else { a[0] }, if n == "Post Reopened" { a[1].max(d) } else { a[1] }, if n == "Edit Body" { a[2].max(d) } else { a[2] }]
    });
    let mut v = drain((&cc).and(&vc).and((&phd).opt()));
    v.sort_by_key(|&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())));
    rows(v.into_iter().map(|(p, ((c, n), h))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "created", "owner"]);
        f.extend([V::I(c), V::I(n)]);
        f.extend(h.unwrap_or([i64::MIN; 3]).map(tmax));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.PostTypeId),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.CommentCount, rp.UpVotes, rp.DownVotes FROM RankedPosts rp WHERE rp.PostRank <= 5),
// UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, RANK() OVER (ORDER BY u.Reputation DESC) AS ReputationRank FROM Users u)
// SELECT tp.Title, tp.CommentCount, tp.UpVotes, tp.DownVotes, ur.DisplayName AS TopUser, ur.Reputation, ur.ReputationRank
// FROM TopPosts tp LEFT JOIN Posts p ON tp.PostId = p.Id LEFT JOIN UserReputation ur ON p.OwnerUserId = ur.UserId WHERE ur.ReputationRank <= 10
// ORDER BY tp.UpVotes DESC, tp.CommentCount DESC;
fn q33778(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let r = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let ur = rel(r.into_iter().filter(|x| x.1 <= 10).map(|((u, _), r)| (u, r)).collect());
    let by_user: HashIdx<Id<User>, (Id<User>, i64)> = (&ur).map(|(u, _)| u).inv().select(&ur).collect();
    let v = drain((&s).and(owner_user.select(&by_user)));
    rows(v.into_iter().map(|(p, (a, (u, r)))| {
        let mut f = post_fields(db, p, &["title"]);
        f.extend(a.map(V::I));
        f.extend(ucols(db, u, &["name", "rep"]));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days'),
// UserStats AS (SELECT u.Id AS UserId, u.Reputation, u.Views, (u.UpVotes - u.DownVotes) AS VoteBalance, COUNT(b.Id) AS BadgeCount
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation, u.Views, u.UpVotes, u.DownVotes),
// RecentPostLinks AS (SELECT pl.PostId, COUNT(pl.RelatedPostId) AS RelatedPostCount FROM PostLinks pl JOIN Posts p ON pl.PostId = p.Id
//     WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days' GROUP BY pl.PostId)
// SELECT up.UserId, up.Reputation, up.Views, up.VoteBalance, up.BadgeCount, pp.PostId, pp.Title, pp.CreationDate, rpl.RelatedPostCount
// FROM UserStats up JOIN RankedPosts pp ON up.UserId = pp.OwnerUserId LEFT JOIN RecentPostLinks rpl ON pp.PostId = rpl.PostId
// WHERE up.Reputation > 1000 AND pp.PostRank <= 3 AND (rpl.RelatedPostCount IS NULL OR rpl.RelatedPostCount > 2) ORDER BY up.Reputation DESC, pp.CreationDate DESC LIMIT 10;
fn q1531(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 0, 0, 0), -30)));
    let top = top_per(drain(recent().select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 3, false);
    let pp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let rpl = recent().group_by(Ident::<Post>::new()).select(links_of(db)).fold(0i64, |n, _| n + 1);
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = drain((&pp).select(owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(1000)).and(&bc)).and((&rpl).opt())).filt(|x| x.1.map_or(true, |l| l > 2)));
    let v = top_n(v, |&(p, ((u, _), _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(creation_date.get(p).unwrap())), 10);
    rows(v.into_iter().map(|(p, ((u, b), l))| {
        let User { up_votes, down_votes, .. } = &db.user;
        let mut f = ucols(db, u, &["uid", "rep", "uviews"]);
        f.extend([V::I(up_votes.get(u).unwrap() - down_votes.get(u).unwrap()), V::I(b)]);
        f.extend(post_fields(db, p, &["id", "title", "created"]));
        f.push(oint(l));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(c.Id) AS CommentCount, COUNT(v.Id) AS VoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId),
// UserActivities AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, SUM(p.Score) AS TotalPostScore, COUNT(DISTINCT p.Id) AS TotalPosts,
//        COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS TotalAnswers, COUNT(DISTINCT bh.Id) AS TotalBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges bh ON u.Id = bh.UserId WHERE u.Reputation >= 100 GROUP BY u.Id, u.DisplayName, u.Reputation)
// SELECT ua.DisplayName, ua.Reputation, ua.TotalPosts, ua.TotalAnswers, ua.TotalBadges, rp.Title, rp.Score, rp.ViewCount, rp.CommentCount, rp.PostRank
// FROM UserActivities ua JOIN RankedPosts rp ON ua.UserId = rp.PostId WHERE rp.PostRank <= 5 ORDER BY ua.Reputation DESC, rp.Score DESC;
//
// The join compares a user id with a post id, so it goes through origid. Only COUNT(DISTINCT ...) columns of UserActivities are read, so each is a plain count per user.
fn q5207(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, origid, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1)).select(owner_user.opt()));
    let v = per_group(ranked(v, |&(p, u)| (u, Reverse(creation_date.get(p).unwrap()), p), false), |&(_, u)| u);
    let rp = rel(v.into_iter().filter(|x| x.1 <= 5).map(|((p, _), r)| (p, r)).collect());
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let ua_of = origid.select(&uidx).select(Ident::<User>::new().with((&db.user.reputation).ge(100)));
    let by_post: HashIdx<Id<Post>, (Id<Post>, i64)> = (&rp).map(|(p, _)| p).inv().select(&rp).collect();
    let hit: MatSet<Id<Post>> = (&rp).map(|(p, _)| p).select(Ident::<Post>::new().with(ua_of)).collect();
    let users: MatSet<Id<User>> = (&hit).select(origid.select(&uidx)).collect();
    let tp = (&users).group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + t.is_some() as i64, a[1] + (t == Some(2)) as i64]);
    let tb = (&users).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let cc = (&hit).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let v = drain((&cc).and(&by_post).and(origid.select(&uidx).select(Ident::<User>::new().and(&tp).and(&tb))));
    rows(v.into_iter().map(|(p, ((c, (_, r)), ((u, a), b)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(b)]);
        f.extend(post_fields(db, p, &["title", "score", "views"]));
        f.extend([V::I(c), V::I(r)]);
        row(f)
    }))
}

// WITH UserBadges AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PostSummary AS (SELECT p.OwnerUserId, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
//        SUM(p.Score) AS TotalScore, SUM(p.ViewCount) AS TotalViews FROM Posts p GROUP BY p.OwnerUserId),
// FinalSummary AS (SELECT ub.UserId, ub.DisplayName, COALESCE(ps.PostCount, 0) AS PostCount, COALESCE(ps.Questions, 0) AS Questions, COALESCE(ps.Answers, 0) AS Answers,
//        COALESCE(ps.TotalScore, 0) AS TotalScore, COALESCE(ps.TotalViews, 0) AS TotalViews, ub.BadgeCount, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges
//     FROM UserBadges ub LEFT JOIN PostSummary ps ON ub.UserId = ps.OwnerUserId)
// SELECT *, RANK() OVER (ORDER BY TotalScore DESC, PostCount DESC, BadgeCount DESC) AS UserRank FROM FinalSummary WHERE PostCount > 0 ORDER BY UserRank LIMIT 10;
fn q7872(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let Post { owner_user, post_type_id, score, view_count, .. } = &db.post;
    let ps = db.post.group_by(owner_user).select(post_type_id.and(score).and(view_count.opt())).fold([0i64; 5], |a, ((t, s), w)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + w.unwrap_or(0)]
    });
    let v = ranked(drain((&ub).and((&ps).filt(|a| a[0] > 0))), |&(_, (b, a))| (Reverse(a[3]), Reverse(a[0]), Reverse(b[0])), false);
    let v = top_n(v, |x| x.1, 10);
    rows(v.into_iter().map(|((u, (b, a)), r)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        f.extend(b.map(V::I));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId IN (1, 2) AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, OwnerDisplayName FROM RankedPosts WHERE Rank <= 5),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(v.BountyAmount) AS TotalBounties, COUNT(c.Id) AS TotalComments
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY u.Id, u.DisplayName)
// SELECT tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.OwnerDisplayName, ua.DisplayName AS Contributor, ua.TotalPosts, ua.TotalBounties, ua.TotalComments
// FROM TopPosts tp JOIN UserActivity ua ON tp.OwnerDisplayName = ua.DisplayName ORDER BY tp.Score DESC, tp.ViewCount DESC;
//
// UserActivity is folded only for the users whose name matches a top post's owner, the rows the join can reach.
fn q9867(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.is_in([1, 2]).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let by_name: HashIdx<Str, Id<User>> = (&db.user.display_name).inv().collect();
    let named = || owner_user.select(&db.user.display_name).select(&by_name);
    let users: MatSet<Id<User>> = (&tp).select(named()).collect();
    let ua = (&users)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select((&db.vote.bounty_amount).opt()).opt().and(comments_of(db).opt())).opt())
        .fold([0i64; 3], |a, x| match x {
            Some((b, c)) => {
                let b = b.flatten();
                [a[0] + b.is_some() as i64, a[1] + b.unwrap_or(0), a[2] + c.is_some() as i64]
            }
            None => a,
        });
    let np = (&users).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let v = drain((&tp).select(named().select(Ident::<User>::new().and(&ua).and(&np))));
    rows(v.into_iter().map(|(p, ((u, a), n))| {
        let mut f = post_fields(db, p, &["title", "created", "score", "views", "owner"]);
        f.extend([user_col(db, u, "name"), V::I(n), nullable(a[1], a[0]), V::I(a[2])]);
        row(f)
    }))
}

// WITH PostStatistics AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COALESCE((SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id), 0) AS CommentCount,
//        COALESCE((SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 2), 0) AS UpVotes,
//        COALESCE((SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 3), 0) AS DownVotes
//     FROM Posts p WHERE p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// OldPostStatistics AS (SELECT p.Id AS PostId, p.Title, p.Score AS OldScore, COALESCE(ph.UserDisplayName, 'Unknown') AS LastEditedBy,
//        ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY ph.CreationDate DESC) AS EditRank
//     FROM Posts p LEFT JOIN PostHistory ph ON p.Id = ph.PostId AND ph.PostHistoryTypeId IN (4, 5) WHERE p.CreationDate <= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year')
// SELECT ps.PostId, ps.Title, ps.CreationDate, ps.Score AS CurrentScore, ps.ViewCount, ps.CommentCount, ps.UpVotes, ps.DownVotes, ops.OldScore, ops.LastEditedBy
// FROM PostStatistics ps LEFT JOIN OldPostStatistics ops ON ps.PostId = ops.PostId AND ops.EditRank = 1 WHERE ps.Score > COALESCE(ops.OldScore, 0)
// ORDER BY ps.Score DESC, ps.ViewCount DESC;
fn q741(db: &'static So) -> String {
    let Post { creation_date, score, .. } = &db.post;
    let t0 = add_years(ts(2024, 10, 1, 12, 34, 56), -1);
    let ps = db
        .post
        .with(creation_date.gt(t0))
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt())
        .fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let PostHistory { post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let edits = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.is_in([4, 5])).and(hd));
    let ops = db.post.with(creation_date.le(t0)).group_by(Ident::<Post>::new()).select(edits.opt()).fold(None, |m: Option<(i64, Id<PostHistory>)>, x| match (m, x) {
        (_, None) => m,
        (None, Some((h, d))) => Some((d, h)),
        (Some((md, mh)), Some((h, d))) => if (d, Reverse(h)) > (md, Reverse(mh)) { Some((d, h)) } else { m },
    });
    let v = drain((&ps).and(comments_per_post(db)).and((&ops).opt()).and(score).filt(|x| x.1 > x.0 .1.map_or(0, |_| x.1)));
    rows(v.into_iter().map(|(p, (((a, c), o), _))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1])]);
        f.extend(match o {
            Some(h) => [V::I(score.get(p).unwrap()), V::S(h.and_then(|(_, h)| db.post_history.user_display_name.get(h)).unwrap_or("Unknown"))],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserBadges AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId WHERE U.Reputation > 1000 GROUP BY U.Id, U.DisplayName),
// UserPosts AS (SELECT P.OwnerUserId, COUNT(P.Id) AS PostCount, SUM(P.Score) AS TotalScore, AVG(P.ViewCount) AS AvgViewCount FROM Posts P
//     WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY P.OwnerUserId),
// RankedUsers AS (SELECT UB.UserId, UB.DisplayName, UB.BadgeCount, UB.GoldBadges, UB.SilverBadges, UB.BronzeBadges, UP.PostCount, UP.TotalScore, UP.AvgViewCount,
//        RANK() OVER (ORDER BY UB.BadgeCount DESC, UP.TotalScore DESC) AS UserRank FROM UserBadges UB JOIN UserPosts UP ON UB.UserId = UP.OwnerUserId)
// SELECT UserId, DisplayName, BadgeCount, GoldBadges, SilverBadges, BronzeBadges, PostCount, TotalScore, AvgViewCount, UserRank FROM RankedUsers WHERE UserRank <= 10 ORDER BY UserRank;
fn q7015(db: &'static So) -> String {
    let ub = db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let Post { owner_user, creation_date, score, view_count, .. } = &db.post;
    let up = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(owner_user)
        .select(score.and(view_count.opt()))
        .fold([0i64; 4], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]);
    let v = ranked(drain((&ub).and(&up)), |&(_, (b, a))| (Reverse(b[0]), Reverse(a[1])), false);
    rows(v.into_iter().take_while(|x| x.1 <= 10).map(|((u, (b, a)), r)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(b.map(V::I));
        f.extend([V::I(a[0]), V::I(a[1]), avg(a[3], a[2]), V::I(r)]);
        row(f)
    }))
}

// WITH UserBadges AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PostStats AS (SELECT p.OwnerUserId, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(p.Score) AS TotalScore FROM Posts p GROUP BY p.OwnerUserId),
// RankedPosts AS (SELECT ps.OwnerUserId, ps.PostCount, ps.QuestionCount, ps.AnswerCount, ps.TotalScore, ROW_NUMBER() OVER (ORDER BY ps.TotalScore DESC) AS PostRank
//     FROM PostStats ps WHERE ps.PostCount > 0)
// SELECT ub.UserId, ub.DisplayName, COALESCE(rp.PostCount, 0) AS PostCount, COALESCE(rp.QuestionCount, 0) AS QuestionCount, COALESCE(rp.AnswerCount, 0) AS AnswerCount,
//        COALESCE(rp.TotalScore, 0) AS TotalScore, ub.BadgeCount, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges
// FROM UserBadges ub LEFT JOIN RankedPosts rp ON ub.UserId = rp.OwnerUserId WHERE ub.BadgeCount > 5 ORDER BY TotalScore DESC, PostCount DESC LIMIT 10;
fn q871(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let Post { owner_user, post_type_id, score, .. } = &db.post;
    let ps = db.post.group_by(owner_user).select(post_type_id.and(score)).fold([0i64; 4], |a, (t, s)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s]);
    let v = drain((&ub).filt(|b| b[0] > 5).and((&ps).opt()));
    let v = top_n(v, |&(_, (_, a))| {
        let a = a.unwrap_or([0; 4]);
        (Reverse(a[3]), Reverse(a[0]))
    }, 10);
    rows(v.into_iter().map(|(u, (b, a))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.unwrap_or([0; 4]).map(V::I));
        f.extend(b.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.Reputation AS OwnerReputation,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days'),
// TagOverview AS (SELECT t.TagName, COUNT(p.Id) AS PostCount, SUM(COALESCE(p.Score, 0)) AS TotalScore FROM Tags t LEFT JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' GROUP BY t.TagName),
// TopTags AS (SELECT TagName, TotalScore FROM TagOverview WHERE PostCount > 0 ORDER BY TotalScore DESC LIMIT 5),
// CommentCounts AS (SELECT PostId, COUNT(c.Id) AS TotalComments FROM Comments c GROUP BY PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.OwnerReputation, COALESCE(cc.TotalComments, 0) AS TotalComments, tt.TagName
// FROM RankedPosts rp LEFT JOIN CommentCounts cc ON rp.PostId = cc.PostId JOIN Posts p ON rp.PostId = p.Id JOIN TopTags tt ON p.Tags LIKE '%' || tt.TagName || '%'
// WHERE rp.Rank <= 5 ORDER BY rp.Score DESC, rp.ViewCount DESC;
fn q34076(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, post_type_id, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 0, 0, 0), -30))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let rp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let lt = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&lt).map(|(_, t)| t).inv().collect();
    let to = db.tag.group_by(&db.tag.tag_name).select((&by_tag).map(|(p, _)| p).select(score).opt()).fold([0i64; 2], |a, s| [a[0] + s.is_some() as i64, a[1] + s.unwrap_or(0)]);
    let tt = top_n(drain((&to).filt(|a| a[0] > 0)), |&(_, a)| Reverse(a[1]), 5);
    let tt: MatSet<Str> = rel(tt.into_iter().map(|x| x.0).collect()).map(|t| t).collect();
    let by_post: HashIdx<Id<Post>, (Id<Post>, Id<Tag>)> = (&lt).map(|(p, _)| p).inv().collect();
    let v = drain((&rp).select(comments_per_post(db).and((&by_post).map(|(_, t)| t).select(&db.tag.tag_name).select(Same::<Str>::new().with(&tt)))));
    rows(v.into_iter().map(|(p, (c, t))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "rep"]);
        f.extend([V::I(c), V::S(t)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.OwnerUserId, p.Score, p.Tags,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank FROM Posts p WHERE p.PostTypeId = 1),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownVotes, COUNT(c.Id) AS TotalComments
//     FROM Users u LEFT JOIN Posts p ON p.OwnerUserId = u.Id LEFT JOIN Votes v ON v.UserId = u.Id AND v.PostId = p.Id LEFT JOIN Comments c ON c.UserId = u.Id GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT ua.UserId, ua.DisplayName, ua.TotalUpVotes - ua.TotalDownVotes AS NetVotes, RANK() OVER (ORDER BY ua.TotalUpVotes DESC) AS UserRank FROM UserActivity ua WHERE ua.TotalUpVotes > 0)
// SELECT rp.PostId, rp.Title, rp.Body, rp.CreationDate, rp.Rank, tu.DisplayName AS TopUser, tu.NetVotes, rp.Score AS PostScore, ARRAY_LENGTH(string_to_array(rp.Tags, '><'), 1) AS TagCount
// FROM RankedPosts rp JOIN TopUsers tu ON rp.OwnerUserId = tu.UserId WHERE rp.Rank = 1 ORDER BY rp.CreationDate DESC LIMIT 10;
//
// v.UserId = u.Id AND v.PostId = p.Id with p owned by u: the owner's own votes on the post (own_votes).
fn q29985(db: &'static So) -> String {
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(own_votes(db).select(&db.vote.vote_type_id).opt()).opt().and(comments_by(db).opt()))
        .fold([0i64; 2], |a, (v, _)| {
            let t = v.flatten();
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
        });
    let Post { post_type_id, owner_user, creation_date, tags_str, .. } = &db.post;
    let first = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let rp: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let v = drain((&rp).select(owner_user.select(Ident::<User>::new().and((&ua).filt(|a| a[0] > 0)))));
    let v = top_n(v, |&(p, _)| Reverse(creation_date.get(p).unwrap()), 10);
    rows(v.into_iter().map(|(p, (u, a))| {
        let mut f = post_fields(db, p, &["id", "title", "body", "created"]);
        f.extend([V::I(1), user_col(db, u, "name"), V::I(a[0] - a[1]), V::I(db.post.score.get(p).unwrap()), oint(tags_str.get(p).map(|t| t.split("><").count() as i64))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.ViewCount, COALESCE(v.upVotes, 0) - COALESCE(v.downVotes, 0) AS VoteScore,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY COALESCE(v.upVotes, 0) DESC) AS RankPerUser
//     FROM Posts p LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS upVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS downVotes
//     FROM Votes GROUP BY PostId) v ON p.Id = v.PostId),
// TopPosts AS (SELECT rp.Id, rp.Title, rp.ViewCount, rp.VoteScore FROM RankedPosts rp WHERE rp.RankPerUser <= 5),
// PostComments AS (SELECT c.PostId, COUNT(*) AS CommentCount FROM Comments c GROUP BY c.PostId),
// FinalResults AS (SELECT tp.Id, tp.Title, tp.ViewCount, tp.VoteScore, COALESCE(pc.CommentCount, 0) AS CommentCount FROM TopPosts tp LEFT JOIN PostComments pc ON tp.Id = pc.PostId)
// SELECT fr.Id, fr.Title, fr.ViewCount, fr.VoteScore, fr.CommentCount, CASE WHEN fr.VoteScore > 0 THEN 'Positive' WHEN fr.VoteScore < 0 THEN 'Negative' ELSE 'Neutral' END AS ScoreStatus
// FROM FinalResults fr ORDER BY fr.VoteScore DESC, fr.ViewCount DESC LIMIT 10;
fn q866(db: &'static So) -> String {
    let pv = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let v = drain(db.post.select((&db.post.owner_user).opt().and((&pv).opt())));
    let top = top_per(v, |&(_, (u, _))| u, |&(p, (_, a))| (Reverse(a.map_or(0, |a| a[0])), p), 5, false);
    let tp = rel(top.into_iter().map(|(p, (_, a))| (p, a.map_or(0, |a| a[0] - a[1]))).collect());
    type R = (Id<Post>, i64);
    let v = drain((&tp).select(Same::<R>::new().and(Same::<R>::new().map(|(p, _): R| p).select(comments_per_post(db)))));
    let v = top_n(v, |&(_, ((p, s), _))| {
        let w = db.post.view_count.get(p);
        (Reverse(s), w.is_none(), Reverse(w))
    }, 10);
    rows(v.into_iter().map(|(_, ((p, s), c))| {
        let mut f = post_fields(db, p, &["id", "title", "views"]);
        f.extend([V::I(s), V::I(c), V::S(if s > 0 { "Positive" } else if s < 0 { "Negative" } else { "Neutral" })]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT Id, Reputation, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM Users),
// PostStats AS (SELECT OwnerUserId, COUNT(CASE WHEN PostTypeId = 1 THEN 1 END) AS QuestionCount, COUNT(CASE WHEN PostTypeId = 2 THEN 1 END) AS AnswerCount, SUM(ViewCount) AS TotalViews,
//        SUM(CASE WHEN CommentCount > 0 THEN 1 END) AS PostedCommentCount, SUM(Score) AS TotalScore FROM Posts GROUP BY OwnerUserId),
// UserBadgeCounts AS (SELECT UserId, COUNT(CASE WHEN Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN Class = 2 THEN 1 END) AS SilverBadges,
//        COUNT(CASE WHEN Class = 3 THEN 1 END) AS BronzeBadges FROM Badges GROUP BY UserId)
// SELECT U.DisplayName, U.Reputation, UR.Rank, COALESCE(PS.QuestionCount, 0) AS QuestionCount, COALESCE(PS.AnswerCount, 0) AS AnswerCount, COALESCE(PS.TotalViews, 0) AS TotalViews,
//        COALESCE(PS.PostedCommentCount, 0) AS PostedCommentCount, COALESCE(UBC.GoldBadges, 0) AS GoldBadges, COALESCE(UBC.SilverBadges, 0) AS SilverBadges,
//        COALESCE(UBC.BronzeBadges, 0) AS BronzeBadges, PS.TotalScore
// FROM Users U JOIN UserReputation UR ON U.Id = UR.Id LEFT JOIN PostStats PS ON U.Id = PS.OwnerUserId LEFT JOIN UserBadgeCounts UBC ON U.Id = UBC.UserId
// ORDER BY U.Reputation DESC, U.DisplayName ASC LIMIT 50;
fn q6250(db: &'static So) -> String {
    let Post { owner_user, post_type_id, view_count, comment_count, score, .. } = &db.post;
    let ps = db.post.group_by(owner_user).select(post_type_id.and(view_count.opt()).and(comment_count).and(score)).fold([0i64; 5], |a, (((t, w), c), s)| {
        [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + w.unwrap_or(0), a[3] + (c > 0) as i64, a[4] + s]
    });
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let r = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 50);
    let ur = rel(r.into_iter().enumerate().map(|(i, (u, _))| (u, i as i64 + 1)).collect());
    type R = (Id<User>, i64);
    let u_of = || Same::<R>::new().map(|(u, _): R| u);
    let v = drain((&ur).select(Same::<R>::new().and(u_of().select((&ps).opt())).and(u_of().select((&ub).opt()))));
    let v = top_n(v, |&(_, (((u, _), _), _))| (Reverse(db.user.reputation.get(u).unwrap()), db.user.display_name.get(u).unwrap()), 50);
    rows(v.into_iter().map(|(_, (((u, r), p), b))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(r));
        let a = p.unwrap_or([0; 5]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3])]);
        f.extend(b.unwrap_or([0; 3]).map(V::I));
        f.push(p.map_or(V::Null, |a| V::I(a[4])));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(COALESCE(vs.Score, 0)) AS TotalScore, COUNT(DISTINCT b.Id) AS BadgeCount,
//        MAX(p.CreationDate) AS LastPostDate
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId
//     LEFT JOIN (SELECT PostId, SUM(Score) AS Score FROM Comments GROUP BY PostId) AS vs ON p.Id = vs.PostId WHERE u.Reputation > 500 GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, TotalScore, BadgeCount, LastPostDate, RANK() OVER (ORDER BY TotalScore DESC, PostCount DESC) AS Rank FROM UserActivity)
// SELECT tu.DisplayName, tu.PostCount, tu.TotalScore, tu.BadgeCount, tu.LastPostDate, ph.Comment AS LastEditComment, COALESCE(NULLIF(p.AcceptedAnswerId, -1), p.ParentId) AS FinalPostLink
// FROM TopUsers tu LEFT JOIN Posts p ON tu.UserId = p.OwnerUserId LEFT JOIN PostHistory ph ON p.Id = ph.PostId
// WHERE tu.Rank <= 10 AND ph.CreationDate = (SELECT MAX(ph2.CreationDate) FROM PostHistory ph2 WHERE ph2.PostId = p.Id) ORDER BY tu.TotalScore DESC;
fn q4007(db: &'static So) -> String {
    let cs = db.comment.group_by(&db.comment.post).select(&db.comment.score).fold(0i64, |s, x| s + x);
    let Post { creation_date, .. } = &db.post;
    let users = || db.user.with((&db.user.reputation).gt(500));
    let ua = users()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&cs).opt().and(votes_of(db).opt())).opt().and(badges_of(db).opt()))
        .fold(0i64, |s, (p, _)| s + p.and_then(|(c, _)| c).unwrap_or(0));
    let pc = users().group_by(Ident::<User>::new()).select(posts_of(db).select(creation_date).opt()).fold([0i64, i64::MIN], |a, d| match d {
        Some(d) => [a[0] + 1, a[1].max(d)],
        None => a,
    });
    let bc = users().group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let r = ranked(drain((&ua).and(&pc).and(&bc)), |&(_, ((s, a), _))| (Reverse(s), Reverse(a[0])), false);
    let tu = rel(r.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0).collect());
    let PostHistory { post, creation_date: hd, .. } = &db.post_history;
    let md = db.post_history.group_by(post).select(hd).fold(i64::MIN, |m, d| m.max(d));
    let at: HashIdx<(Id<Post>, i64), Id<PostHistory>> = db.post_history.select(post.and(hd)).inv().collect();
    type R = (Id<User>, ((i64, [i64; 2]), i64));
    let v = drain((&tu).select(Same::<R>::new().and(Same::<R>::new().map(|(u, _): R| u).select(posts_of(db)).select(Ident::<Post>::new().and(Ident::<Post>::new().and(&md).select(&at))))));
    let mut v = v;
    v.sort_by_key(|&(_, ((_, ((s, _), _)), _))| Reverse(s));
    rows(v.into_iter().map(|(_, ((u, ((s, a), b)), (p, h)))| {
        let link = db.post.accepted_answer_id.get(p).filter(|&x| x != -1).or(db.post.parent_id.get(p));
        row(vec![user_col(db, u, "name"), V::I(a[0]), V::I(s), V::I(b), tmax(a[1]), ostr(db.post_history.comment.get(h)), oint(link)])
    }))
}

// WITH RankedUsers AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, RANK() OVER (ORDER BY U.Reputation DESC) AS UserRank FROM Users U),
// PostStatistics AS (SELECT P.Id AS PostId, P.Title, P.PostTypeId, COUNT(C.Id) AS NumberOfComments, COALESCE(SUM(V.BountyAmount), 0) AS TotalBounty,
//        COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpvotes, COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownvotes
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY P.Id, P.Title, P.PostTypeId),
// UserPostStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS TotalPosts, SUM(PS.TotalUpvotes) AS TotalUpvotes, SUM(PS.TotalDownvotes) AS TotalDownvotes,
//        SUM(PS.TotalBounty) AS TotalBounty FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN PostStatistics PS ON P.Id = PS.PostId GROUP BY U.Id, U.DisplayName)
// SELECT RU.UserRank, UPS.UserId, UPS.DisplayName, UPS.TotalPosts, UPS.TotalUpvotes, UPS.TotalDownvotes, UPS.TotalBounty
// FROM RankedUsers RU JOIN UserPostStats UPS ON RU.UserId = UPS.UserId WHERE RU.UserRank <= 10 ORDER BY RU.UserRank;
//
// UserRank reads only Reputation, so the top users are picked first and PostStatistics is folded for their posts alone.
fn q9626(db: &'static So) -> String {
    let r = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let tu = rel(r.into_iter().take_while(|x| x.1 <= 10).map(|((u, _), r)| (u, r)).collect());
    let users: MatSet<Id<User>> = (&tu).map(|(u, _)| u).collect();
    let tp: MatSet<Id<Post>> = (&users).select(posts_of(db)).collect();
    let ps = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select((&db.vote.vote_type_id).and((&db.vote.bounty_amount).opt())).opt()))
        .fold([0i64; 3], |a, (_, v)| match v {
            Some((t, b)) => [a[0] + b.unwrap_or(0), a[1] + (t == 2) as i64, a[2] + (t == 3) as i64],
            None => a,
        });
    let ups = (&users).group_by(Ident::<User>::new()).select(posts_of(db).select(&ps).opt()).fold([0i64; 4], |a, p| match p {
        Some(s) => [a[0] + 1, a[1] + s[1], a[2] + s[2], a[3] + s[0]],
        None => a,
    });
    let by_user: HashIdx<Id<User>, (Id<User>, i64)> = (&tu).map(|(u, _)| u).inv().select(&tu).collect();
    let v = drain((&ups).and(&by_user));
    rows(v.into_iter().map(|(u, (a, (_, r)))| {
        let mut f = vec![V::I(r)];
        f.extend(ucols(db, u, &["uid", "name"]));
        f.push(V::I(a[0]));
        f.extend(if a[0] == 0 { [V::Null, V::Null, V::Null] } else { [V::I(a[1]), V::I(a[2]), V::I(a[3])] });
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, AnswerCount, QuestionCount, UpVotes, DownVotes, GoldBadges, SilverBadges, BronzeBadges,
//        ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Rank FROM UserStats)
// SELECT Rank, DisplayName, Reputation, PostCount, AnswerCount, QuestionCount, UpVotes, DownVotes, GoldBadges, SilverBadges, BronzeBadges FROM TopUsers WHERE Rank <= 10 ORDER BY Rank;
//
// Rank reads only Reputation, so the top users are picked first and the post x vote x badge product is driven for them alone.
fn q7387(db: &'static So) -> String {
    let r = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10);
    let tu = rel(r.into_iter().enumerate().map(|(i, (u, _))| (u, i as i64 + 1)).collect());
    let users: MatSet<Id<User>> = (&tu).map(|(u, _)| u).collect();
    let us = (&users)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 7], |a, (p, b)| {
            let (t, v) = p.map_or((None, None), |(t, v)| (Some(t), v));
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(1)) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64, a[4] + (b == Some(1)) as i64, a[5] + (b == Some(2)) as i64, a[6] + (b == Some(3)) as i64]
        });
    let pc = (&users).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let by_user: HashIdx<Id<User>, (Id<User>, i64)> = (&tu).map(|(u, _)| u).inv().select(&tu).collect();
    let v = drain((&us).and(&pc).and(&by_user));
    rows(v.into_iter().map(|(u, ((a, n), (_, r)))| {
        let mut f = vec![V::I(r)];
        f.extend(ucols(db, u, &["name", "rep"]));
        f.push(V::I(n));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, AnswerCount, QuestionCount, GoldBadges, SilverBadges, BronzeBadges,
//        RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserStats)
// SELECT tu.UserId, tu.DisplayName, tu.Reputation, tu.PostCount, tu.AnswerCount, tu.QuestionCount, tu.GoldBadges, tu.SilverBadges, tu.BronzeBadges,
//        CASE WHEN tu.ReputationRank <= 10 THEN 'Top User' WHEN tu.ReputationRank BETWEEN 11 AND 50 THEN 'Popular User' ELSE 'Regular User' END AS UserCategory
// FROM TopUsers tu WHERE tu.PostCount > 5 ORDER BY tu.Reputation DESC, tu.AnswerCount DESC LIMIT 50;
fn q7949(db: &'static So) -> String {
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(&db.post.post_type_id).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 5], |a, (t, b)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(1)) as i64, a[2] + (b == Some(1)) as i64, a[3] + (b == Some(2)) as i64, a[4] + (b == Some(3)) as i64]);
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let r = ranked(drain((&us).and(&pc)), |&(u, _)| Reverse(db.user.reputation.get(u).unwrap()), false);
    let v = drain(rel(r).filt(|((_, (_, n)), _)| n > 5));
    let v = top_n(v, |&(_, ((u, (a, _)), _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(a[0])), 50);
    rows(v.into_iter().map(|(_, ((u, (a, n)), r))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.push(V::S(if r <= 10 { "Top User" } else if r <= 50 { "Popular User" } else { "Regular User" }));
        row(f)
    }))
}

// WITH UserBadges AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount, STRING_AGG(DISTINCT B.Name, ', ') AS BadgeNames
//     FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// TopPosts AS (SELECT P.Id AS PostId, P.Title, P.ViewCount, P.Score, P.Tags, P.CreationDate, U.DisplayName AS OwnerDisplayName,
//        COALESCE((SELECT COUNT(*) FROM Comments C WHERE C.PostId = P.Id), 0) AS CommentCount
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id WHERE P.PostTypeId = 1 ORDER BY P.ViewCount DESC LIMIT 10),
// VoteCounts AS (SELECT V.PostId, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        COUNT(V.Id) AS TotalVotes FROM Votes V GROUP BY V.PostId)
// SELECT UpP.Title AS PostTitle, UpP.OwnerDisplayName, UpP.ViewCount AS PostViews, UpP.Score AS PostScore, UpV.UpVotes, UpV.DownVotes, UpV.TotalVotes, UB.BadgeCount, UB.BadgeNames
// FROM TopPosts UpP LEFT JOIN VoteCounts UpV ON UpP.PostId = UpV.PostId LEFT JOIN UserBadges UB ON UpP.OwnerDisplayName = UB.DisplayName
// WHERE UpP.CommentCount > 5 ORDER BY UpP.Score DESC, UpVotes DESC;
//
// STRING_AGG(DISTINCT ...) has no ORDER BY, so its order is DuckDB's choice; the port sorts the names. No row reaches the output on this data.
fn q26787(db: &'static So) -> String {
    let Post { post_type_id, owner_user, view_count, .. } = &db.post;
    let v = top_n(drain(db.post.with(post_type_id.eq(1)).with(owner_user).select(view_count.opt())), |&(_, w)| (w.is_none(), Reverse(w)), 10);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let vc = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 3], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + 1]);
    let by_name: HashIdx<Str, Id<User>> = (&db.user.display_name).inv().collect();
    let names: MatSet<Id<User>> = (&tp).select(owner_user.select(&db.user.display_name).select(&by_name)).collect();
    let ub = (&names).group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.name).opt()).buf_fold(|it| {
        let n0 = it.iter().filter(|x| x.is_some()).count() as i64;
        let mut n: Vec<Str> = it.into_iter().flatten().collect();
        n.sort_unstable();
        n.dedup();
        (n0, if n.is_empty() { None } else { Some(&*Box::leak(n.join(", ").into_boxed_str())) })
    });
    let v = drain((&tp).with(comments_per_post(db).filt(|c| c > 5)).select((&vc).opt().and(owner_user.select(&db.user.display_name).select(&by_name).select(&ub).opt())));
    let mut v = v;
    v.sort_by_key(|&(p, (a, _))| (Reverse(db.post.score.get(p).unwrap()), a.is_none(), Reverse(a.map(|a| a[0]))));
    rows(v.into_iter().map(|(p, (a, b))| {
        let mut f = post_fields(db, p, &["title", "owner", "views", "score"]);
        f.extend(match a {
            Some(a) => a.map(V::I),
            None => [V::Null, V::Null, V::Null],
        });
        f.extend(match b {
            Some((n, s)) => [V::I(n), ostr(s)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank FROM Posts p WHERE p.Score IS NOT NULL),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) FILTER (WHERE b.Class = 1) AS GoldBadges, COUNT(b.Id) FILTER (WHERE b.Class = 2) AS SilverBadges,
//        COUNT(b.Id) FILTER (WHERE b.Class = 3) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostVoteCounts AS (SELECT v.PostId, SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes
//     FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY v.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, u.DisplayName, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges, pvc.UpVotes, pvc.DownVotes
// FROM RankedPosts rp JOIN Posts p ON rp.PostId = p.Id LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN UserBadges ub ON u.Id = ub.UserId LEFT JOIN PostVoteCounts pvc ON p.Id = pvc.PostId
// WHERE rp.Rank <= 5 AND (p.AcceptedAnswerId IS NOT NULL OR p.AnswerCount > 0) ORDER BY rp.Score DESC, rp.CreationDate DESC;
fn q1339(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, accepted_answer_id, answer_count, .. } = &db.post;
    let top = top_per(drain(db.post.select(post_type_id)), |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false);
    let rp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| {
        [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]
    });
    let pvc = db.vote.group_by(&db.vote.post).select(vtype_name(db)).fold([0i64; 2], |a, n| [a[0] + (n == "UpMod") as i64, a[1] + (n == "DownMod") as i64]);
    let answered = Ident::<Post>::new().with(accepted_answer_id).or(Ident::<Post>::new().with(answer_count.gt(0)));
    let v = drain((&rp).with(answered).select(owner_user.select(Ident::<User>::new().and(&ub)).opt().and((&pvc).opt())));
    let mut v = v;
    v.sort_by_key(|&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(db.post.creation_date.get(p).unwrap())));
    rows(v.into_iter().map(|(p, (u, a))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend(match u {
            Some((u, b)) => [user_col(db, u, "name"), V::I(b[0]), V::I(b[1]), V::I(b[2])],
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        f.extend(match a {
            Some(a) => a.map(V::I),
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedUsers AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U),
// PostStatistics AS (SELECT P.OwnerUserId, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(P.ViewCount) AS TotalViews, AVG(P.Score) AS AverageScore FROM Posts P
//     WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY P.OwnerUserId),
// UserBadges AS (SELECT B.UserId, COUNT(B.Id) AS TotalBadges, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Badges B GROUP BY B.UserId)
// SELECT RU.DisplayName, RU.Reputation, P.TotalPosts, P.TotalQuestions, P.TotalAnswers, P.TotalViews, P.AverageScore, UB.TotalBadges, UB.GoldBadges, UB.SilverBadges, UB.BronzeBadges
// FROM RankedUsers RU LEFT JOIN PostStatistics P ON RU.UserId = P.OwnerUserId LEFT JOIN UserBadges UB ON RU.UserId = UB.UserId WHERE RU.ReputationRank <= 10 ORDER BY RU.ReputationRank;
fn q6824(db: &'static So) -> String {
    let r = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(r.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let Post { owner_user, creation_date, post_type_id, view_count, score, .. } = &db.post;
    let ps = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(owner_user)
        .select(post_type_id.and(view_count.opt()).and(score))
        .fold([0i64; 6], |a, ((t, w), s)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0), a[5] + s]);
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    let v = drain((&tu).select((&ps).opt().and((&ub).opt())));
    rows(v.into_iter().map(|(u, (p, b))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(match p {
            Some(a) => [V::I(a[0]), V::I(a[1]), V::I(a[2]), nullable(a[4], a[3]), avg(a[5], a[0])],
            None => [V::Null, V::Null, V::Null, V::Null, V::Null],
        });
        f.extend(match b {
            Some(b) => b.map(V::I),
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.Tags, u.DisplayName AS OwnerDisplayName, p.CreationDate, p.ViewCount, COUNT(c.Id) AS CommentCount, COUNT(v.Id) AS VoteCount,
//        RANK() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS Rank, COALESCE(SUM(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 ELSE 0 END), 0) AS ClosedCount
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Body, p.Tags, u.DisplayName, p.CreationDate, p.ViewCount),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.Body, rp.Tags, rp.OwnerDisplayName, rp.CreationDate, rp.ViewCount, rp.CommentCount, rp.VoteCount, rp.ClosedCount,
//        ROW_NUMBER() OVER (ORDER BY rp.ViewCount DESC, rp.CreationDate ASC) AS RowNum FROM RankedPosts rp WHERE rp.ClosedCount > 0)
// SELECT fp.Title, fp.OwnerDisplayName, fp.CreationDate, fp.ViewCount, fp.CommentCount, fp.VoteCount FROM FilteredPosts fp WHERE fp.RowNum <= 10 ORDER BY fp.ViewCount DESC, fp.CreationDate ASC;
fn q28985(db: &'static So) -> String {
    let Post { post_type_id, view_count, creation_date, .. } = &db.post;
    let s = db
        .post
        .with(post_type_id.eq(1))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).opt()).and(history_of(db).select(&db.post_history.post_history_type_id).opt()))
        .fold([0i64; 3], |a, ((c, v), h)| [a[0] + c.is_some() as i64, a[1] + v.is_some() as i64, a[2] + (h == Some(10)) as i64]);
    let v = top_n(drain((&s).filt(|a| a[2] > 0)), |&(p, _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), creation_date.get(p).unwrap())
    }, 10);
    rows(v.into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["title", "owner", "created", "views"]);
        f.extend([V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, ROW_NUMBER() OVER (ORDER BY u.Reputation DESC) AS ReputationRank FROM Users u),
// PostDetail AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount,
//        COALESCE((SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 2), 0) AS UpVotes,
//        COALESCE((SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 3), 0) AS DownVotes,
//        COALESCE((SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id), 0) AS TotalComments
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT pd.PostId, pd.Title, pd.Score, pd.ViewCount, pd.AnswerCount, pd.CommentCount, pd.UpVotes, pd.DownVotes,
//        RANK() OVER (ORDER BY pd.Score DESC, pd.ViewCount DESC) AS PostRank FROM PostDetail pd WHERE pd.ViewCount > 50)
// SELECT ur.DisplayName, ur.Reputation, ur.ReputationRank, tp.Title, tp.Score, tp.ViewCount, tp.AnswerCount, tp.CommentCount, tp.UpVotes, tp.DownVotes
// FROM UserReputation ur LEFT JOIN TopPosts tp ON ur.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = tp.PostId) WHERE ur.ReputationRank <= 10 ORDER BY ur.Reputation DESC, tp.Score DESC;
//
// The correlated subquery is the post's own owner, so the join is users to the top posts they own.
fn q17(db: &'static So) -> String {
    let r = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10);
    let ur = rel(r.into_iter().enumerate().map(|(i, (u, _))| (u, i as i64 + 1)).collect());
    let Post { creation_date, view_count, owner_user, .. } = &db.post;
    let tp: HashIdx<Id<User>, Id<Post>> = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(view_count.gt(50))).select(owner_user).inv().collect();
    let pv = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    type R = (Id<User>, i64);
    let v = drain((&ur).select(Same::<R>::new().and(Same::<R>::new().map(|(u, _): R| u).select((&tp).select(Ident::<Post>::new().and((&pv).opt()))).opt())));
    rows(v.into_iter().map(|(_, ((u, r), t))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(r));
        f.extend(match t {
            Some((p, a)) => {
                let mut g = post_fields(db, p, &["title", "score", "views", "answers", "comments"]);
                let a = a.unwrap_or([0, 0]);
                g.extend([V::I(a[0]), V::I(a[1])]);
                g
            }
            None => (0..7).map(|_| V::Null).collect(),
        });
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (8, 9) WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName),
// MaxPostUser AS (SELECT UserId, TotalPosts, RANK() OVER (ORDER BY TotalPosts DESC) AS Rank FROM UserPostStats),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts FROM UserPostStats WHERE UserId IN (SELECT UserId FROM MaxPostUser WHERE Rank <= 10))
// SELECT tu.DisplayName, tu.TotalPosts, CASE WHEN ups.Questions > 0 THEN ROUND(ups.TotalBounty / ups.Questions, 2) ELSE 0 END AS AvgBountyPerQuestion,
//        p.Title AS LastPostTitle, p.CreationDate AS LastPostDate, p.ViewCount
// FROM TopUsers tu LEFT JOIN Posts p ON tu.UserId = p.OwnerUserId AND p.LastActivityDate = (SELECT MAX(LastActivityDate) FROM Posts WHERE OwnerUserId = tu.UserId)
// LEFT JOIN UserPostStats ups ON tu.UserId = ups.UserId ORDER BY tu.TotalPosts DESC, AvgBountyPerQuestion DESC;
fn q114(db: &'static So) -> String {
    let Post { post_type_id, owner_user, last_activity_date, .. } = &db.post;
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9]))).select((&db.vote.bounty_amount).opt());
    let ups = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(bounty.opt())).opt())
        .fold([0i64; 3], |a, p| match p {
            Some((t, b)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + b.flatten().unwrap_or(0)],
            None => a,
        });
    let r = ranked(drain(&ups), |&(_, a)| Reverse(a[0]), false);
    let tu = rel(r.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0).collect());
    let ml = db.post.group_by(owner_user).select(last_activity_date).fold(i64::MIN, |m, d| m.max(d));
    let at: HashIdx<(Id<User>, i64), Id<Post>> = db.post.with(owner_user).select(owner_user.and(last_activity_date)).inv().collect();
    type R = (Id<User>, [i64; 3]);
    let v = drain((&tu).select(Same::<R>::new().and(Same::<R>::new().map(|(u, _): R| u).select(Ident::<User>::new().and(&ml).select(&at)).opt())));
    let avgb = |a: [i64; 3]| if a[1] > 0 { ((a[2] as f64 / a[1] as f64) * 100.0).round() / 100.0 } else { 0.0 };
    let mut v = v;
    v.sort_by_key(|&(_, ((_, a), _))| (Reverse(a[0]), Reverse(fkey(avgb(a)))));
    rows(v.into_iter().map(|(_, ((u, a), p))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::F(avgb(a))];
        f.extend(match p {
            Some(p) => post_fields(db, p, &["title", "created", "views"]),
            None => (0..3).map(|_| V::Null).collect(),
        });
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.CreationDate, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(CASE WHEN p.PostTypeId = 2 AND p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers,
//        AVG(vote_count) AS AvgVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN (SELECT PostId, COUNT(*) AS vote_count FROM Votes GROUP BY PostId) v ON p.Id = v.PostId
//     GROUP BY u.Id, u.DisplayName, u.Reputation, u.CreationDate),
// TopTags AS (SELECT t.TagName, COUNT(p.Id) AS PostCount FROM Tags t JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' GROUP BY t.TagName ORDER BY PostCount DESC LIMIT 5)
// SELECT us.UserId, us.DisplayName, us.Reputation, us.TotalPosts, us.Questions, us.Answers, us.AcceptedAnswers, us.AvgVotes, STRING_AGG(tt.TagName, ', ') AS PopularTags
// FROM UserStats us LEFT JOIN TopTags tt ON us.Questions > 0
// GROUP BY us.UserId, us.DisplayName, us.Reputation, us.TotalPosts, us.Questions, us.Answers, us.AcceptedAnswers, us.AvgVotes
// HAVING us.Reputation > (SELECT AVG(Reputation) FROM Users WHERE Reputation IS NOT NULL) ORDER BY us.Reputation DESC LIMIT 10;
//
// The ON names only us, so TopTags is crossed with the users that have questions. STRING_AGG has no ORDER BY; the port lists the tags in TopTags order, as DuckDB does here.
// The ORDER BY and HAVING read only Reputation, so the ten users are picked first.
fn q2023(db: &'static So) -> String {
    let (n, s) = db.user.select(&db.user.reputation).fold_flat((0i64, 0i64), |(n, s), r| (n + 1, s + r));
    let mean = s as f64 / n as f64;
    let top = top_n(drain((&db.user.reputation).filt(|r| r as f64 > mean)), |&(_, r)| Reverse(r), 10);
    let tu: MatSet<Id<User>> = rel(top.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let vc = db.vote.group_by(&db.vote.post).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let Post { post_type_id, accepted_answer_id, .. } = &db.post;
    let us = (&tu).group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(accepted_answer_id.opt()).and((&vc).opt())).opt()).fold([0i64; 6], |a, p| match p {
        Some(((t, acc), c)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (t == 2 && acc.is_some()) as i64, a[4] + c.is_some() as i64, a[5] + c.unwrap_or(0)],
        None => a,
    });
    let lt = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&lt).map(|(_, t)| t).inv().collect();
    let tc = db.tag.group_by(&db.tag.tag_name).select(&by_tag).fold(0i64, |n, _| n + 1);
    let tt = rel(top_n(drain(&tc), |&(_, n)| Reverse(n), 5));
    type T = (Id<User>, (Str, i64));
    let agg = (&tu).with((&us).filt(|a| a[1] > 0)).cross(&tt).group_by(Same::<T>::new().map(|(u, _): T| u)).select(Same::<T>::new().map(|(_, t): T| t)).buf_fold(|it| {
        let mut t: Vec<(Str, i64)> = it.into_iter().collect();
        t.sort_by_key(|&(_, n)| Reverse(n));
        &*Box::leak(t.iter().map(|x| x.0).collect::<Vec<_>>().join(", ").into_boxed_str())
    });
    let mut v = drain((&us).and((&agg).opt()));
    v.sort_by_key(|&(u, _)| Reverse(db.user.reputation.get(u).unwrap()));
    rows(v.into_iter().map(|(u, (a, t))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), avg(a[5], a[4]), ostr(t)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.OwnerUserId, COUNT(c.Id) AS CommentCount, COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 2) AS UpVotes,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS UserRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.Score, p.CreationDate, p.OwnerUserId),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.CommentCount, rp.UpVotes, u.DisplayName FROM RankedPosts rp JOIN Users u ON rp.OwnerUserId = u.Id WHERE rp.UserRank <= 5),
// PostsWithCloseReasons AS (SELECT p.Id AS PostId, p.Title, ph.Comment AS CloseReason FROM Posts p JOIN PostHistory ph ON p.Id = ph.PostId WHERE ph.PostHistoryTypeId = 10)
// SELECT tp.PostId, tp.Title, tp.Score, tp.CommentCount, tp.UpVotes, tp.DisplayName, COALESCE(pcr.CloseReason, 'Not Closed') AS CloseReason
// FROM TopPosts tp LEFT JOIN PostsWithCloseReasons pcr ON tp.PostId = pcr.PostId ORDER BY tp.Score DESC, tp.CommentCount DESC LIMIT 10;
//
// UserRank reads only Score, so each owner's top posts are picked first and the comment x vote product is driven for those alone.
fn q3830(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64]);
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10)));
    let v = drain((&s).and(closes.opt()));
    let v = top_n(v, |&(p, (a, _))| (Reverse(score.get(p).unwrap()), Reverse(a[0])), 10);
    rows(v.into_iter().map(|(p, (a, h))| {
        let mut f = post_fields(db, p, &["id", "title", "score"]);
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.push(post_fields(db, p, &["owner"]).pop().unwrap());
        f.push(V::S(h.and_then(|h| db.post_history.comment.get(h)).unwrap_or("Not Closed")));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.Score, p.CreationDate, u.DisplayName AS OwnerDisplayName, p.ViewCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year' AND p.Score > 0),
// TopPosts AS (SELECT rp.Id, rp.Title, rp.OwnerDisplayName, rp.Score, rp.ViewCount FROM RankedPosts rp WHERE rp.Rank <= 10),
// PostWithComments AS (SELECT tp.Id AS PostId, tp.Title, tp.OwnerDisplayName, tp.Score, COUNT(c.Id) AS CommentCount FROM TopPosts tp LEFT JOIN Comments c ON tp.Id = c.PostId
//     GROUP BY tp.Id, tp.Title, tp.OwnerDisplayName, tp.Score)
// SELECT pwc.PostId, pwc.Title, pwc.OwnerDisplayName, pwc.Score, pwc.CommentCount,
//        (SELECT AVG(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) FROM Votes v WHERE v.PostId = pwc.PostId) AS AverageUpVotes,
//        (SELECT COUNT(*) FROM PostHistory ph WHERE ph.PostId = pwc.PostId) AS EditCount
// FROM PostWithComments pwc ORDER BY pwc.Score DESC, pwc.CommentCount DESC;
fn q9793(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, post_type_id, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(score.gt(0))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let av = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + 1, a[1] + (t == 2) as i64]);
    let v = drain((&tp).select(comments_per_post(db).and((&av).opt()).and(history_per_post(db))));
    rows(v.into_iter().map(|(p, ((c, a), h))| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "score"]);
        f.extend([V::I(c), a.map_or(V::Null, |a| avg(a[1], a[0])), V::I(h)]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS UpvotedPosts
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.Reputation),
// TopUsers AS (SELECT UserId, Reputation, PostCount, QuestionCount, AnswerCount, UpvotedPosts, RANK() OVER (ORDER BY Reputation DESC) AS RankReputation,
//        RANK() OVER (ORDER BY PostCount DESC) AS RankPostCount FROM UserStats),
// UserBadges AS (SELECT b.UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldBadgeCount, COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverBadgeCount,
//        COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeBadgeCount FROM Badges b GROUP BY b.UserId)
// SELECT u.DisplayName, ts.Reputation, ts.PostCount, ts.QuestionCount, ts.AnswerCount, ts.UpvotedPosts, ub.GoldBadgeCount, ub.SilverBadgeCount, ub.BronzeBadgeCount,
//        ts.RankReputation, ts.RankPostCount
// FROM TopUsers ts JOIN Users u ON ts.UserId = u.Id LEFT JOIN UserBadges ub ON ts.UserId = ub.UserId WHERE ts.RankReputation <= 10 OR ts.RankPostCount <= 10
// ORDER BY ts.RankReputation, ts.RankPostCount;
fn q6793(db: &'static So) -> String {
    let ups = user_posts(db);
    let v = ranked(drain(&ups), |&(u, _)| Reverse(db.user.reputation.get(u).unwrap()), false);
    let v = ranked(v, |&((_, a), _)| Reverse(a[1]), false);
    let tu = rel(v.into_iter().map(|(((u, a), r), c)| (u, (a, r, c))).collect());
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    type R = (Id<User>, ([i64; 10], i64, i64));
    let v = drain((&tu).filt(|(_, (_, r, c))| r <= 10 || c <= 10).select(Same::<R>::new().and(Same::<R>::new().map(|(u, _): R| u).select((&ub).opt()))));
    rows(v.into_iter().map(|(_, ((u, (a, r, c)), b))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[8])]);
        f.extend(match b {
            Some(b) => b.map(V::I),
            None => [V::Null, V::Null, V::Null],
        });
        f.extend([V::I(r), V::I(c)]);
        row(f)
    }))
}

// Rewritten (rewrites/9911.sql): the final ORDER BY tie-broken on u.Id.
// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, COUNT(c.Id) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank, p.OwnerUserId
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.PostTypeId = 1
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.OwnerUserId),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id)
// SELECT u.Id AS UserId, u.DisplayName, COUNT(rp.PostId) AS TotalQuestions, SUM(rp.CommentCount) AS TotalComments, ub.BadgeCount, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges
// FROM RankedPosts rp JOIN Users u ON rp.OwnerUserId = u.Id JOIN UserBadges ub ON u.Id = ub.UserId WHERE rp.UserPostRank = 1
// GROUP BY u.Id, u.DisplayName, ub.BadgeCount, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges ORDER BY TotalQuestions DESC, TotalComments DESC, u.Id LIMIT 10;
fn q9911(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.eq(1))).select(owner_user));
    let first = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let rp: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let g = (&rp).group_by(owner_user).select(comments_per_post(db)).fold([0i64; 2], |a, c| [a[0] + 1, a[1] + c]);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let v = top_n(drain((&g).and(&ub)), |&(u, (a, _))| (Reverse(a[0]), Reverse(a[1]), db.user.origid.get(u).unwrap()), 10);
    rows(v.into_iter().map(|(u, (a, b))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.extend(b.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostID, p.Title, p.CreationDate, p.Score, COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount,
//        RANK() OVER (ORDER BY p.Score DESC, p.CreationDate ASC) AS ScoreRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days'),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, MAX(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadge, MAX(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadge,
//        MAX(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadge FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id)
// SELECT rp.PostID, rp.Title, rp.CreationDate, rp.Score, rp.CommentCount, ub.UserId AS BadgeUserId, ub.BadgeCount,
//        CASE WHEN ub.GoldBadge = 1 THEN 'Gold' WHEN ub.SilverBadge = 1 THEN 'Silver' WHEN ub.BronzeBadge = 1 THEN 'Bronze' ELSE 'No Badge' END AS HighestBadge,
//        (SELECT COUNT(*) FROM Votes v WHERE v.PostId = rp.PostID AND v.VoteTypeId = 2) AS UpvoteCount
// FROM RankedPosts rp LEFT JOIN Users u ON rp.Score >= u.Reputation LEFT JOIN UserBadges ub ON u.Id = ub.UserId WHERE rp.ScoreRank <= 10 ORDER BY rp.Score DESC, rp.CreationDate ASC;
//
// RankedPosts has one row per (post, comment), and ScoreRank ranks those rows. The ON compares with Reputation only, so it is a filtered cross join with Users.
fn q3796(db: &'static So) -> String {
    let Post { creation_date, score, .. } = &db.post;
    let joined: MatSet<(Id<Post>, Option<Id<Comment>>)> = db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 0, 0, 0), -30))).select(Ident::<Post>::new().and(comments_of(db).opt())).collect();
    let r = ranked(drain(&joined), |&(_, (p, _))| (Reverse(score.get(p).unwrap()), creation_date.get(p).unwrap()), false);
    let rp = rel(r.into_iter().filter(|x| x.1 <= 10).enumerate().map(|(i, x)| (i, x.0 .1 .0, x.0 .1 .1, score.get(x.0 .1 .0).unwrap())).collect());
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1].max((c == 1) as i64), a[2].max((c == 2) as i64), a[3].max((c == 3) as i64)],
        None => a,
    });
    let users = rel(drain(&db.user.reputation));
    type P = (usize, Id<Post>, Option<Id<Comment>>, i64);
    let hit = (&rp).cross(&users).filt(|((_, _, _, s), (_, r)): (P, (Id<User>, i64))| s >= r);
    let matched: MatSet<usize> = (&hit).map(|((i, _, _, _), _): (P, (Id<User>, i64))| i).collect();
    let cc = comments_per_post(db);
    let up = votes_of_type(db, 2);
    let mut out = Vec::new();
    let emit = |out: &mut Vec<String>, p: Id<Post>, u: Option<Id<User>>| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.push(V::I(cc.get(p).unwrap()));
        f.extend(match u {
            Some(u) => {
                let b = ub.get(u).unwrap();
                let h = if b[1] == 1 { "Gold" } else if b[2] == 1 { "Silver" } else if b[3] == 1 { "Bronze" } else { "No Badge" };
                [user_col(db, u, "uid"), V::I(b[0]), V::S(h)]
            }
            None => [V::Null, V::Null, V::S("No Badge")],
        });
        f.push(V::I(up.get(p).unwrap()));
        out.push(row(f));
    };
    (&hit).drive(|_, ((_, p, _, _), (u, _))| emit(&mut out, p, Some(u)));
    (&rp).minus(&matched).drive(|_, (_, p, _, _)| emit(&mut out, p, None));
    rows(out)
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, COUNT(a.Id) AS AnswerCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, ROW_NUMBER() OVER (ORDER BY p.Score DESC) AS Rank
//     FROM Posts p LEFT JOIN Posts a ON a.ParentId = p.Id AND p.PostTypeId = 1 LEFT JOIN Votes v ON v.PostId = p.Id
//     WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days' AND p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score),
// TopPosts AS (SELECT rp.*, pt.Name AS PostTypeName, ut.DisplayName AS OwnerName FROM RankedPosts rp
//     JOIN PostTypes pt ON pt.Id = (SELECT PostTypeId FROM Posts WHERE Id = rp.PostId LIMIT 1) JOIN Users ut ON ut.Id = (SELECT OwnerUserId FROM Posts WHERE Id = rp.PostId LIMIT 1)
//     WHERE rp.Rank <= 10)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.ViewCount, tp.AnswerCount, tp.UpVotes, tp.DownVotes, tp.OwnerName, tp.PostTypeName
// FROM TopPosts tp JOIN PostHistory ph ON ph.PostId = tp.PostId WHERE ph.CreationDate >= '2023-01-01' AND ph.PostHistoryTypeId IN (10, 11) ORDER BY tp.UpVotes DESC, tp.ViewCount DESC;
//
// Rank reads only Score, so the ten questions are picked first and the answer x vote product is driven for those. The correlated subqueries return the post's own type and owner.
fn q5591(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, owner_user, .. } = &db.post;
    let v = top_n(drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)).and(post_type_id.eq(1))).select(score)), |&(p, s)| (Reverse(s), p), 10);
    let tp: MatSet<Id<Post>> = rel(v.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(children_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let PostHistory { post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let closes = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.is_in([10, 11]).and(hd.ge(ts(2023, 1, 1, 0, 0, 0)))));
    let v = drain((&s).and(owner_user).and(closes));
    let mut v = v;
    v.sort_by_key(|&(p, ((a, _), _))| {
        let w = db.post.view_count.get(p);
        (Reverse(a[1]), w.is_none(), Reverse(w))
    });
    rows(v.into_iter().map(|(p, ((a, u), _))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views"]);
        f.extend(a.map(V::I));
        f.extend([user_col(db, u, "name"), post_fields(db, p, &["type"]).pop().unwrap()]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.CreationDate, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation, U.CreationDate),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, UpvoteCount, DownvoteCount, RANK() OVER (ORDER BY Reputation DESC) AS Rank FROM UserReputation)
// SELECT TU.Rank, TU.DisplayName, TU.Reputation, TU.PostCount, TU.QuestionCount, TU.AnswerCount, TU.UpvoteCount, TU.DownvoteCount,
//        AVG(CASE WHEN PH.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS AverageCloseReasons, COUNT(DISTINCT B.Id) AS BadgeCount
// FROM TopUsers TU LEFT JOIN PostHistory PH ON TU.UserId = PH.UserId LEFT JOIN Badges B ON TU.UserId = B.UserId WHERE TU.Rank <= 10
// GROUP BY TU.Rank, TU.DisplayName, TU.Reputation, TU.PostCount, TU.QuestionCount, TU.AnswerCount, TU.UpvoteCount, TU.DownvoteCount ORDER BY TU.Rank;
//
// Rank reads only Reputation, so the top users are picked first and both products are driven for them alone.
fn q6236(db: &'static So) -> String {
    let r = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let tu = rel(r.into_iter().take_while(|x| x.1 <= 10).map(|((u, _), r)| (u, r)).collect());
    let users: MatSet<Id<User>> = (&tu).map(|(u, _)| u).collect();
    let ur = (&users)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some((t, v)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64],
            None => a,
        });
    let pc = (&users).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let hist_by: HashIdx<Id<User>, Id<PostHistory>> = (&db.post_history.user).inv().collect();
    let ph = (&users)
        .group_by(Ident::<User>::new())
        .select((&hist_by).select(&db.post_history.post_history_type_id).opt().and(badges_of(db).opt()))
        .fold([0i64; 2], |a, (t, _)| [a[0] + 1, a[1] + (t == Some(10)) as i64]);
    let bc = (&users).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let by_user: HashIdx<Id<User>, (Id<User>, i64)> = (&tu).map(|(u, _)| u).inv().select(&tu).collect();
    let v = drain((&ur).and(&pc).and(&ph).and(&bc).and(&by_user));
    rows(v.into_iter().map(|(u, ((((a, n), h), b), (_, r)))| {
        let mut f = vec![V::I(r)];
        f.extend(ucols(db, u, &["name", "rep"]));
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.extend([avg(h[1], h[0]), V::I(b)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS AuthorName,
//        ROW_NUMBER() OVER (PARTITION BY p.Tags ORDER BY p.Score DESC) AS TagRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// PostSummary AS (SELECT r.PostId, r.Title, r.Body, r.CreationDate, r.Score, r.ViewCount, r.AuthorName, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM RankedPosts r LEFT JOIN Comments c ON r.PostId = c.PostId LEFT JOIN Votes v ON r.PostId = v.PostId WHERE r.TagRank <= 5
//     GROUP BY r.PostId, r.Title, r.Body, r.CreationDate, r.Score, r.ViewCount, r.AuthorName)
// SELECT ps.PostId, ps.Title, ps.Body, ps.CreationDate, ps.Score, ps.ViewCount, ps.AuthorName, ps.CommentCount, ps.UpVotes, ps.DownVotes, pt.Name AS PostType
// FROM PostSummary ps JOIN PostTypes pt ON pt.Id = (SELECT PostTypeId FROM Posts WHERE Id = ps.PostId) ORDER BY ps.Score DESC, ps.ViewCount DESC LIMIT 50;
//
// TagRank reads only Score, so each tag set's top questions are picked first and the comment x vote product is driven for those alone.
fn q7519(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, tags_str, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).with(owner_user).select(tags_str.opt()));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = top_n(drain(&s), |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    }, 50);
    rows(v.into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "body", "created", "score", "views", "owner"]);
        f.extend(a.map(V::I));
        f.extend(post_fields(db, p, &["type"]));
        row(f)
    }))
}

// WITH UserVoteCounts AS (SELECT v.UserId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes, COUNT(v.Id) AS TotalVotes
//     FROM Votes v GROUP BY v.UserId),
// PostDetails AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, u.DisplayName AS OwnerDisplayName, COALESCE(pc.CommentCount, 0) AS CommentCount, COALESCE(ph.EditCount, 0) AS EditCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) pc ON p.Id = pc.PostId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS EditCount FROM PostHistory GROUP BY PostId) ph ON p.Id = ph.PostId)
// SELECT pd.PostId, pd.Title, pd.CreationDate, pd.Score, pd.OwnerDisplayName, uvc.UpVotes, uvc.DownVotes, pd.CommentCount, pd.EditCount,
//        CASE WHEN pd.Score > 100 THEN 'High' WHEN pd.Score BETWEEN 50 AND 100 THEN 'Medium' ELSE 'Low' END AS ScoreCategory
// FROM PostDetails pd JOIN UserVoteCounts uvc ON pd.PostId IN (SELECT PostId FROM Votes WHERE UserId = uvc.UserId GROUP BY PostId)
// WHERE pd.PostRank <= 10 ORDER BY pd.CreationDate DESC FETCH FIRST 50 ROWS ONLY;
//
// The IN subquery pairs each post with every distinct user who voted on it; UserId is the raw column, so the pairs and the counts are keyed by it.
fn q3059(db: &'static So) -> String {
    let Vote { user_id, post, vote_type_id, .. } = &db.vote;
    let uvc = db.vote.group_by(user_id).select(vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let pairs: MatSet<(Id<Post>, i64)> = db.vote.select(post.and(user_id)).collect();
    let voters: HashIdx<Id<Post>, (Id<Post>, i64)> = (&pairs).map(|(p, _)| p).inv().collect();
    let Post { owner_user, creation_date, score, .. } = &db.post;
    let top = top_per(drain(db.post.select(owner_user.opt())), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 10, false);
    let pd: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let v = drain((&pd).select(comments_per_post(db).and(history_per_post(db)).and((&voters).map(|(_, u)| u).select(&uvc))));
    let v = top_n(v, |&(p, _)| Reverse(creation_date.get(p).unwrap()), 50);
    rows(v.into_iter().map(|(p, ((c, e), a))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c), V::I(e), V::S(if s > 100 { "High" } else if s >= 50 { "Medium" } else { "Low" })]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(CASE WHEN P.PostTypeId IN (4, 5) THEN 1 ELSE 0 END) AS TagWikiPosts, MAX(P.CreationDate) AS LastActivity,
//        AVG(P.ViewCount) AS AvgViewCount FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName),
// BadgeCounts AS (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId),
// UserStatistics AS (SELECT UA.UserId, UA.DisplayName, UA.TotalPosts, UA.Questions, UA.Answers, UA.TagWikiPosts, UA.LastActivity, UA.AvgViewCount, COALESCE(BC.BadgeCount, 0) AS BadgeCount,
//        RANK() OVER (ORDER BY UA.TotalPosts DESC) AS PostRank, RANK() OVER (ORDER BY UA.AvgViewCount DESC) AS ViewRank FROM UserActivity UA LEFT JOIN BadgeCounts BC ON UA.UserId = BC.UserId)
// SELECT UserId, DisplayName, TotalPosts, Questions, Answers, TagWikiPosts, LastActivity, AvgViewCount, BadgeCount, PostRank, ViewRank
// FROM UserStatistics WHERE TotalPosts > 0 ORDER BY TotalPosts DESC, AvgViewCount DESC FETCH FIRST 10 ROWS ONLY;
fn q5081(db: &'static So) -> String {
    let Post { post_type_id, creation_date, view_count, .. } = &db.post;
    let ua = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(creation_date).and(view_count.opt())).opt()).fold([0, 0, 0, 0, i64::MIN, 0, 0], |a, p| match p {
        Some(((t, d), w)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + matches!(t, 4 | 5) as i64, a[4].max(d), a[5] + w.is_some() as i64, a[6] + w.unwrap_or(0)],
        None => a,
    });
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let av = |a: &[i64; 7]| if a[5] == 0 { None } else { Some(a[6] as f64 / a[5] as f64) };
    let v = ranked(drain((&ua).and((&bc).opt())), |&(_, (a, _))| Reverse(a[0]), false);
    let v = ranked(v, |&((_, (a, _)), _)| {
        let m = av(&a);
        (m.is_none(), Reverse(m.map(fkey)))
    }, false);
    let v = drain(rel(v).filt(|(((_, (a, _)), _), _)| a[0] > 0));
    let v = top_n(v, |&(_, (((_, (a, _)), _), _))| {
        let m = av(&a);
        (Reverse(a[0]), m.is_none(), Reverse(m.map(fkey)))
    }, 10);
    rows(v.into_iter().map(|(_, (((u, (a, b)), pr), vr))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), tmax(a[4]), avg(a[6], a[5]), V::I(b.unwrap_or(0)), V::I(pr), V::I(vr)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT b.Id) AS BadgeCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank, p.OwnerUserId
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Badges b ON p.OwnerUserId = b.UserId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.OwnerUserId),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.CommentCount, u.DisplayName AS UserDisplayName, u.Reputation AS UserReputation
//     FROM RankedPosts rp JOIN Users u ON rp.OwnerUserId = u.Id WHERE rp.UserPostRank <= 5)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.CommentCount, tp.UserDisplayName, tp.UserReputation, COUNT(DISTINCT v.Id) AS VoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount
// FROM TopPosts tp LEFT JOIN Votes v ON tp.PostId = v.PostId GROUP BY tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.CommentCount, tp.UserDisplayName, tp.UserReputation
// ORDER BY tp.Score DESC, tp.CommentCount DESC;
//
// CommentCount is COUNT(DISTINCT c.Id), which the badge join does not change, so it is the post's comment count.
fn q8115(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let top = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| {
        [a[0] + t.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let v = drain((&pv).and(comments_per_post(db)));
    rows(v.into_iter().map(|(p, (a, c))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.push(V::I(c));
        f.extend(post_fields(db, p, &["owner", "rep"]));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END), 0) AS QuestionCount,
//        COALESCE(SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS AnswerCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COALESCE(COUNT(c.Id), 0) AS CommentCount, COALESCE(SUM(b.Class), 0) AS BadgePoints
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Badges b ON u.Id = b.UserId
//     WHERE u.CreationDate >= '2020-01-01' GROUP BY u.Id, u.DisplayName, u.Reputation),
// UserRanking AS (SELECT UserId, DisplayName, Reputation, QuestionCount, AnswerCount, UpVotes, DownVotes, CommentCount, BadgePoints,
//        ROW_NUMBER() OVER (ORDER BY Reputation DESC, QuestionCount DESC, AnswerCount DESC) AS Rank FROM UserActivity)
// SELECT ur.Rank, ur.DisplayName, ur.Reputation, ur.QuestionCount, ur.AnswerCount, ur.UpVotes, ur.DownVotes, ur.CommentCount, ur.BadgePoints FROM UserRanking ur WHERE ur.Rank <= 10 ORDER BY ur.Rank;
fn q6943(db: &'static So) -> String {
    let ua = db
        .user
        .with((&db.user.creation_date).ge(ts(2020, 1, 1, 0, 0, 0)))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(comments_of(db).opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 6], |a, (p, b)| {
            let (t, v, c) = p.map_or((None, None, None), |((t, v), c)| (Some(t), v, c));
            [a[0] + (t == Some(1)) as i64, a[1] + (t == Some(2)) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64, a[4] + c.is_some() as i64, a[5] + b.unwrap_or(0)]
        });
    let v = top_n(drain(&ua), |&(u, a)| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(a[0]), Reverse(a[1])), 10);
    rows(v.into_iter().enumerate().map(|(i, (u, a))| {
        let mut f = vec![V::I(i as i64 + 1)];
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount, RANK() OVER (PARTITION BY p.Tags ORDER BY p.Score DESC) AS TagRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Score, u.DisplayName, p.Tags),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.OwnerDisplayName, rp.CommentCount FROM RankedPosts rp WHERE rp.TagRank <= 5),
// PostDetails AS (SELECT tp.PostId, tp.Title, tp.Score, tp.OwnerDisplayName, COALESCE(b.Name, 'No Badge') AS Badge, tp.CommentCount, p.LastActivityDate
//     FROM TopPosts tp LEFT JOIN Badges b ON tp.PostId = b.UserId JOIN Posts p ON tp.PostId = p.Id ORDER BY tp.Score DESC, tp.CommentCount DESC)
// SELECT pd.Title, pd.Score, pd.OwnerDisplayName, pd.Badge, pd.CommentCount, pd.LastActivityDate FROM PostDetails pd
// WHERE pd.LastActivityDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' ORDER BY pd.CommentCount DESC, pd.LastActivityDate DESC LIMIT 10;
//
// b.UserId = tp.PostId compares a user id with a post id, so it goes through origid.
fn q7417(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, tags_str, score, last_activity_date, origid, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(t0, -1)))).with(owner_user).select(tags_str.opt()));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| Reverse(score.get(p).unwrap()), 5, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let buid: HashIdx<i64, Id<Badge>> = (&db.badge.user_id).inv().collect();
    let v = drain((&tp).with(last_activity_date.ge(add_days(t0, -30))).select(comments_per_post(db).and(origid.select(&buid).opt())));
    let v = top_n(v, |&(p, (c, _))| (Reverse(c), Reverse(last_activity_date.get(p).unwrap())), 10);
    rows(v.into_iter().map(|(p, (c, b))| {
        let mut f = post_fields(db, p, &["title", "score", "owner"]);
        f.extend([V::S(b.map_or("No Badge", |b| db.badge.name.get(b).unwrap())), V::I(c), V::T(last_activity_date.get(p).unwrap())]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT us.UserId, us.DisplayName, us.PostCount, us.QuestionCount, us.AnswerCount, us.UpVotes, us.DownVotes, us.GoldBadges, us.SilverBadges, us.BronzeBadges,
//        RANK() OVER (ORDER BY u.Reputation DESC) AS Rank FROM UserStats us JOIN Users u ON us.UserId = u.Id WHERE u.Views > 1000)
// SELECT tu.Rank, tu.DisplayName, tu.PostCount, tu.QuestionCount, tu.AnswerCount, tu.UpVotes, tu.DownVotes, tu.GoldBadges, tu.SilverBadges, tu.BronzeBadges FROM TopUsers tu WHERE tu.Rank <= 10 ORDER BY tu.Rank;
//
// Rank reads only Reputation, so the top users are picked first and the post x vote x badge product is driven for them alone.
fn q6224(db: &'static So) -> String {
    let User { reputation, views, .. } = &db.user;
    let r = ranked(drain(db.user.with(reputation.gt(1000).and(views.gt(1000))).select(reputation)), |&(_, r)| Reverse(r), false);
    let tu = rel(r.into_iter().take_while(|x| x.1 <= 10).map(|((u, _), r)| (u, r)).collect());
    let users: MatSet<Id<User>> = (&tu).map(|(u, _)| u).collect();
    let us = (&users)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 7], |a, (p, b)| {
            let (t, v) = p.map_or((None, None), |(t, v)| (Some(t), v));
            [a[0] + (t == Some(1)) as i64, a[1] + (t == Some(2)) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64, a[4] + (b == Some(1)) as i64, a[5] + (b == Some(2)) as i64, a[6] + (b == Some(3)) as i64]
        });
    let pc = (&users).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let by_user: HashIdx<Id<User>, (Id<User>, i64)> = (&tu).map(|(u, _)| u).inv().select(&tu).collect();
    let v = drain((&us).and(&pc).and(&by_user));
    rows(v.into_iter().map(|(u, ((a, n), (_, r)))| {
        let mut f = vec![V::I(r), user_col(db, u, "name"), V::I(n)];
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(CASE WHEN p.ViewCount > 100 THEN 1 ELSE 0 END) AS PopularPosts, SUM(COALESCE(c.CommentCount, 0)) AS TotalComments,
//        SUM(COALESCE(b.Count, 0)) AS TotalBadges, RANK() OVER (ORDER BY COUNT(p.Id) DESC) AS PostRank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
//     LEFT JOIN (SELECT UserId, COUNT(*) AS Count FROM Badges GROUP BY UserId) b ON u.Id = b.UserId WHERE u.Reputation > 100 GROUP BY u.Id, u.DisplayName),
// PopularUsers AS (SELECT UserId, DisplayName, TotalPosts, Questions, Answers, PopularPosts, TotalComments, TotalBadges, PostRank FROM UserActivity WHERE PostRank <= 10)
// SELECT pu.DisplayName, pu.TotalPosts, pu.Questions, pu.Answers, pu.PopularPosts, pu.TotalComments, pu.TotalBadges, (SELECT AVG(TotalPosts) FROM UserActivity) AS AvgPosts,
//        (SELECT MAX(ViewCount) FROM Posts) AS MaxViewCount
// FROM PopularUsers pu ORDER BY pu.TotalPosts DESC;
fn q7298(db: &'static So) -> String {
    let Post { post_type_id, view_count, .. } = &db.post;
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let ua = db
        .user
        .with((&db.user.reputation).gt(100))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(view_count.opt()).and(comments_per_post(db))).opt().and((&bc).opt()))
        .fold([0i64; 6], |a, (p, b)| {
            let b = b.unwrap_or(0);
            match p {
                Some(((t, w), c)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.map_or(false, |w| w > 100) as i64, a[4] + c, a[5] + b],
                None => [a[0], a[1], a[2], a[3], a[4], a[5] + b],
            }
        });
    let (n, s) = (&ua).fold_flat((0i64, 0i64), |(n, s), a| (n + 1, s + a[0]));
    let mv = db.post.select(view_count).fold_flat(i64::MIN, |m, w| m.max(w));
    let r = ranked(drain(&ua), |&(_, a)| Reverse(a[0]), false);
    rows(r.into_iter().take_while(|x| x.1 <= 10).map(|((u, a), _)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(a.map(V::I));
        f.extend([avg(s, n), V::I(mv)]);
        row(f)
    }))
}

// Rewritten (rewrites/7920.sql): the PostRank window order tie-broken on p.Id.
// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.Id) AS VoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC, p.Id) AS PostRank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (2, 3)
//     WHERE p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName, p.OwnerUserId),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, OwnerDisplayName, CommentCount, VoteCount FROM RankedPosts WHERE PostRank <= 5)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.OwnerDisplayName, tp.CommentCount, tp.VoteCount, pt.Name AS PostTypeName, pht.Name AS PostHistoryTypeName
// FROM TopPosts tp JOIN PostTypes pt ON EXISTS (SELECT 1 FROM Posts p WHERE p.Id = tp.PostId AND p.PostTypeId = pt.Id)
// LEFT JOIN PostHistory ph ON ph.PostId = tp.PostId LEFT JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id ORDER BY tp.Score DESC, tp.ViewCount DESC;
//
// The EXISTS matches exactly the post's own type.
fn q7920(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(owner_user.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ud = || votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([2, 3])));
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(ud().opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(ud().opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let v = drain((&cc).and(&vc).and(history_of(db).select(htype_name(db)).opt()));
    rows(v.into_iter().map(|(p, ((c, n), h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend([V::I(c), V::I(n)]);
        f.extend(post_fields(db, p, &["type"]));
        f.push(ostr(h));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.Tags, p.CreationDate, u.DisplayName AS Author, COUNT(v.Id) AS VoteCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.Tags ORDER BY COUNT(v.Id) DESC) AS TagRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Body, p.Tags, p.CreationDate, u.DisplayName),
// TagAnalytics AS (SELECT t.TagName, COUNT(DISTINCT p.Id) AS QuestionCount, SUM(rp.UpVotes) AS TotalUpVotes, SUM(rp.DownVotes) AS TotalDownVotes, AVG(rp.VoteCount) AS AvgVotesPerPost
//     FROM Tags t JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' JOIN RankedPosts rp ON p.Id = rp.PostId GROUP BY t.TagName),
// TopTags AS (SELECT TagName, QuestionCount, TotalUpVotes, TotalDownVotes, AvgVotesPerPost, RANK() OVER (ORDER BY QuestionCount DESC) AS TagRank FROM TagAnalytics)
// SELECT TagName, QuestionCount, TotalUpVotes, TotalDownVotes, AvgVotesPerPost, TagRank FROM TopTags WHERE TagRank <= 10 ORDER BY QuestionCount DESC;
fn q29886(db: &'static So) -> String {
    let Post { post_type_id, owner_user, .. } = &db.post;
    let rp = db.post.with(post_type_id.eq(1)).with(owner_user).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| {
        [a[0] + t.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let lt = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, (Id<Post>, Id<Tag>)> = (&lt).map(|(_, t)| t).inv().collect();
    let ta = db.tag.group_by(&db.tag.tag_name).select((&by_tag).map(|(p, _)| p).select(&rp)).fold([0i64; 4], |a, r| [a[0] + 1, a[1] + r[1], a[2] + r[2], a[3] + r[0]]);
    let r = ranked(drain(&ta), |&(_, a)| Reverse(a[0]), false);
    rows(r.into_iter().take_while(|x| x.1 <= 10).map(|((t, a), r)| row(vec![V::S(t), V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[3], a[0]), V::I(r)])))
}

// WITH UserBadges AS (SELECT b.UserId, COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverBadges,
//        COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeBadges FROM Badges b GROUP BY b.UserId),
// PostStats AS (SELECT p.OwnerUserId, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
//        AVG(p.Score) AS AverageScore FROM Posts p GROUP BY p.OwnerUserId),
// UserEngagement AS (SELECT u.Id AS UserId, u.Reputation, COALESCE(pb.TotalPosts, 0) AS TotalPosts, COALESCE(pb.Questions, 0) AS Questions, COALESCE(pb.Answers, 0) AS Answers,
//        COALESCE(ub.GoldBadges, 0) AS GoldBadges, COALESCE(ub.SilverBadges, 0) AS SilverBadges, COALESCE(ub.BronzeBadges, 0) AS BronzeBadges
//     FROM Users u LEFT JOIN PostStats pb ON u.Id = pb.OwnerUserId LEFT JOIN UserBadges ub ON u.Id = ub.UserId),
// RankedUsers AS (SELECT ue.*, RANK() OVER (ORDER BY ue.Reputation DESC, ue.TotalPosts DESC) AS UserRank FROM UserEngagement ue)
// SELECT ru.UserId, ru.Reputation, ru.TotalPosts, ru.Questions, ru.Answers, ru.GoldBadges, ru.SilverBadges, ru.BronzeBadges, ru.UserRank FROM RankedUsers ru WHERE ru.UserRank <= 10 ORDER BY ru.UserRank;
fn q1284(db: &'static So) -> String {
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let Post { owner_user, post_type_id, .. } = &db.post;
    let ps = db.post.group_by(owner_user).select(post_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64]);
    let v = drain(db.user.select((&ps).opt().and((&ub).opt())));
    let r = ranked(v, |&(u, (p, _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(p.map_or(0, |a| a[0]))), false);
    rows(r.into_iter().take_while(|x| x.1 <= 10).map(|((u, (p, b)), r)| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend(p.unwrap_or([0; 3]).map(V::I));
        f.extend(b.unwrap_or([0; 3]).map(V::I));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH PostStats AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.Score, p.AnswerCount, p.CommentCount, p.CreationDate, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.ViewCount, p.Score, p.AnswerCount, p.CommentCount, p.CreationDate),
// TopPosts AS (SELECT ps.PostId, ps.Title, ps.ViewCount, ps.Score, ps.AnswerCount, ps.CommentCount, ps.CreationDate, ps.TotalBounty, ps.UpVotes, ps.DownVotes,
//        RANK() OVER (ORDER BY ps.Score DESC, ps.ViewCount DESC) AS RankScore FROM PostStats ps)
// SELECT tp.PostId, tp.Title, tp.ViewCount, tp.Score, tp.AnswerCount, tp.CommentCount, tp.CreationDate, tp.TotalBounty, tp.UpVotes, tp.DownVotes, pt.Name AS PostTypeName,
//        ut.Reputation AS UserReputation, ut.DisplayName AS OwnerDisplayName
// FROM TopPosts tp JOIN Posts p ON tp.PostId = p.Id JOIN PostTypes pt ON p.PostTypeId = pt.Id LEFT JOIN Users ut ON p.OwnerUserId = ut.Id WHERE tp.RankScore <= 10 ORDER BY tp.RankScore;
//
// RankScore reads only Score and ViewCount, so the top questions are picked first and their votes folded.
fn q6173(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let r = ranked(drain(db.post.with(post_type_id.eq(1)).select(score)), |&(p, s)| {
        let w = view_count.get(p);
        (Reverse(s), w.is_none(), Reverse(w))
    }, false);
    let tp: MatSet<Id<Post>> = rel(r.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let pv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select((&db.vote.vote_type_id).and((&db.vote.bounty_amount).opt())).opt()).fold([0i64; 3], |a, v| match v {
        Some((t, b)) => [a[0] + b.unwrap_or(0), a[1] + (t == 2) as i64, a[2] + (t == 3) as i64],
        None => a,
    });
    let v = drain(&pv);
    rows(v.into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "views", "score", "answers", "comments", "created"]);
        f.extend(a.map(V::I));
        f.extend(post_fields(db, p, &["type", "rep", "owner"]));
        row(f)
    }))
}

// WITH UserVoteSummary AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVotes,
//        COUNT(DISTINCT P.Id) AS PostsVotedOn FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId LEFT JOIN Posts P ON V.PostId = P.Id GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, UpVotes, DownVotes, PostsVotedOn, RANK() OVER (ORDER BY UpVotes DESC) AS UpVoteRank, RANK() OVER (ORDER BY DownVotes DESC) AS DownVoteRank
//     FROM UserVoteSummary)
// SELECT T.DisplayName, T.UpVotes, T.DownVotes, T.PostsVotedOn, PH.Location, PH.WebsiteUrl, PH.Reputation, PT.Name AS PostType, COUNT(P.Id) AS TotalPosts,
//        SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
// FROM TopUsers T JOIN Users PH ON T.UserId = PH.Id LEFT JOIN Posts P ON PH.Id = P.OwnerUserId LEFT JOIN PostTypes PT ON P.PostTypeId = PT.Id LEFT JOIN Votes V ON P.Id = V.PostId
// WHERE T.UpVoteRank <= 10 OR T.DownVoteRank <= 10 GROUP BY T.UserId, T.DisplayName, T.UpVotes, T.DownVotes, T.PostsVotedOn, PH.Location, PH.WebsiteUrl, PH.Reputation, PT.Name
// ORDER BY T.UpVotes DESC, T.DownVotes ASC;
//
// PT.Name is a column of the post, so the (user, type) groups are built over the posts, and a user with no posts is its own NULL group.
fn q27782(db: &'static So) -> String {
    let Vote { vote_type_id, post, .. } = &db.vote;
    let uv = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let dp = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(post).opt()).buf_fold(distinct_some);
    let v = ranked(drain((&uv).and(&dp)), |&(_, (a, _))| Reverse(a[0]), false);
    let v = ranked(v, |&((_, (a, _)), _)| Reverse(a[1]), false);
    let tu = rel(v.into_iter().map(|(((u, (a, d)), r1), r2)| (u, (a, d, r1, r2))).collect());
    type T = (Id<User>, ([i64; 2], i64, i64, i64));
    let top: MatSet<Id<User>> = (&tu).filt(|(_, (_, _, r1, r2))| r1 <= 10 || r2 <= 10).map(|(u, _): T| u).collect();
    let g = db.post.group_by((&db.post.owner_user).and(ptype_name(db))).select(votes_of(db).select(vote_type_id).opt()).fold([0i64; 3], |a, t| {
        [a[0] + 1, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let gv = rel(drain(&g));
    let by_user: HashIdx<Id<User>, ((Id<User>, Str), [i64; 3])> = (&gv).map(|((u, _), _)| u).inv().select(&gv).collect();
    let by_t: HashIdx<Id<User>, T> = (&tu).map(|(u, _): T| u).inv().select(&tu).collect();
    let v = drain((&top).select((&by_t).and((&by_user).map(|((_, t), a)| (t, a)).opt())));
    rows(v.into_iter().map(|(u, ((_, (a, d, _, _)), g))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(d), ostr(db.user.location.get(u)), ostr(db.user.website_url.get(u)), user_col(db, u, "rep")];
        f.extend(match g {
            Some((t, a)) => [V::S(t), V::I(a[0]), V::I(a[1]), V::I(a[2])],
            None => [V::Null, V::I(0), V::I(0), V::I(0)],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.OwnerUserId, COUNT(a.Id) AS AnswerCount, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RankPerUser
//     FROM Posts p LEFT JOIN Posts a ON p.Id = a.ParentId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, CASE WHEN u.Reputation < 100 THEN 'Newbie' WHEN u.Reputation >= 100 AND u.Reputation <= 1000 THEN 'Intermediate' ELSE 'Expert' END AS ReputationCategory FROM Users u),
// ClosedPosts AS (SELECT ph.PostId, COUNT(ph.Id) AS ClosureCount FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.PostId)
// SELECT rp.Title, rp.CreationDate, ur.Reputation, ur.ReputationCategory, COALESCE(cp.ClosureCount, 0) AS ClosureCount, rp.AnswerCount,
//        CASE WHEN rp.RankPerUser = 1 THEN 'Most Recent' ELSE NULL END AS RecentPostIndicator
// FROM RankedPosts rp JOIN Users u ON rp.OwnerUserId = u.Id JOIN UserReputation ur ON u.Id = ur.UserId LEFT JOIN ClosedPosts cp ON rp.Id = cp.PostId WHERE ur.Reputation > 100
// ORDER BY rp.CreationDate DESC;
fn q3713(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1)).select(owner_user.opt()));
    let r = per_group(ranked(v, |&(p, u)| (u, Reverse(creation_date.get(p).unwrap())), false), |&(_, u)| u);
    let rp = rel(r.into_iter().map(|((p, _), r)| (p, r)).collect());
    let cp = db.post_history.with((&db.post_history.post_history_type_id).is_in([10, 11])).group_by(&db.post_history.post).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    type R = (Id<Post>, i64);
    let p_of = || Same::<R>::new().map(|(p, _): R| p);
    let v = drain((&rp).select(Same::<R>::new().and(p_of().select(owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(100))))).and(p_of().select((&cp).opt())).and(p_of().select(answers_per_post(db)))));
    rows(v.into_iter().map(|(_, ((((p, r), u), c), n))| {
        let rep = db.user.reputation.get(u).unwrap();
        let mut f = post_fields(db, p, &["title", "created"]);
        f.extend([V::I(rep), V::S(if rep < 100 { "Newbie" } else if rep <= 1000 { "Intermediate" } else { "Expert" }), V::I(c.unwrap_or(0)), V::I(n)]);
        f.push(if r == 1 { V::S("Most Recent") } else { V::Null });
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.UpVotes, u.DownVotes, (u.UpVotes - u.DownVotes) AS NetVotes, RANK() OVER (ORDER BY u.Reputation DESC) AS ReputationRank FROM Users u),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, NetVotes, ReputationRank FROM UserStats WHERE ReputationRank <= 10),
// PostAggregate AS (SELECT p.OwnerUserId, COUNT(p.Id) AS PostCount, SUM(p.ViewCount) AS TotalViews, AVG(p.Score) AS AverageScore FROM Posts p GROUP BY p.OwnerUserId),
// JoinedData AS (SELECT t.DisplayName AS TopUser, pa.PostCount, pa.TotalViews, pa.AverageScore, COALESCE(b.BadgeCount, 0) AS BadgeCount
//     FROM TopUsers t LEFT JOIN PostAggregate pa ON t.UserId = pa.OwnerUserId LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId) b ON t.UserId = b.UserId)
// SELECT jd.TopUser, jd.PostCount, jd.TotalViews, jd.AverageScore, jd.BadgeCount, CASE WHEN jd.BadgeCount > 5 THEN 'Expert' WHEN jd.BadgeCount BETWEEN 1 AND 5 THEN 'Novice' ELSE 'Newbie' END AS UserLevel,
//        t.ReputationRank
// FROM JoinedData jd JOIN TopUsers t ON jd.TopUser = t.DisplayName ORDER BY t.ReputationRank, jd.TotalViews DESC LIMIT 10;
fn q1271(db: &'static So) -> String {
    let r = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let tu = rel(r.into_iter().take_while(|x| x.1 <= 10).map(|((u, _), r)| (u, r)).collect());
    let Post { owner_user, view_count, score, .. } = &db.post;
    let pa = db.post.group_by(owner_user).select(score.and(view_count.opt())).fold([0i64; 4], |a, (s, w)| [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s]);
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    type R = (Id<User>, i64);
    let u_of = || Same::<R>::new().map(|(u, _): R| u);
    let by_name: HashIdx<Str, R> = (&tu).map(|(u, _): R| db.user.display_name.get(u).unwrap()).inv().select(&tu).collect();
    let v = drain((&tu).select(u_of().and(u_of().select((&pa).opt())).and(u_of().select((&bc).opt())).and(u_of().select(&db.user.display_name).select(&by_name))));
    let v = top_n(v, |&(_, (((_, a), _), (_, r)))| (r, a.map_or(true, |a| a[1] == 0), Reverse(a.map(|a| a[2]))), 10);
    rows(v.into_iter().map(|(_, (((u, a), b), (_, r)))| {
        let b = b.unwrap_or(0);
        let mut f = vec![user_col(db, u, "name")];
        f.extend(match a {
            Some(a) => [V::I(a[0]), nullable(a[2], a[1]), avg(a[3], a[0])],
            None => [V::Null, V::Null, V::Null],
        });
        f.extend([V::I(b), V::S(if b > 5 { "Expert" } else if b >= 1 { "Novice" } else { "Newbie" }), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.AnswerCount, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank FROM Posts p
//     WHERE p.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year')),
// UserStatistics AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, SUM(u.UpVotes) AS TotalUpVotes, SUM(u.DownVotes) AS TotalDownVotes, MAX(u.Reputation) AS MaxReputation
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostCloseReasonCounts AS (SELECT ph.PostId, COUNT(*) FILTER (WHERE ph.PostHistoryTypeId = 10) AS CloseCount, COUNT(*) FILTER (WHERE ph.PostHistoryTypeId = 11) AS ReopenCount
//     FROM PostHistory ph GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.Score, rp.AnswerCount, rp.ViewCount, us.UserId, us.BadgeCount, us.TotalUpVotes, us.TotalDownVotes, us.MaxReputation,
//        COALESCE(prc.CloseCount, 0) AS CloseCount, COALESCE(prc.ReopenCount, 0) AS ReopenCount
// FROM RankedPosts rp LEFT JOIN Users u ON rp.PostId = u.Id JOIN UserStatistics us ON u.Id = us.UserId LEFT JOIN PostCloseReasonCounts prc ON rp.PostId = prc.PostId
// WHERE rp.Rank <= 5 ORDER BY rp.Score DESC, us.MaxReputation DESC;
//
// u.Id = rp.PostId compares a user id with a post id, so it goes through origid.
fn q4819(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, origid, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false);
    let rp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let User { up_votes, down_votes, reputation, .. } = &db.user;
    let us = db.user.group_by(Ident::<User>::new()).select(up_votes.and(down_votes).and(reputation).and(badges_of(db).opt())).fold([0i64, 0, 0, i64::MIN], |a, (((u, d), r), b)| {
        [a[0] + b.is_some() as i64, a[1] + u, a[2] + d, a[3].max(r)]
    });
    let prc = db.post_history.group_by(&db.post_history.post).select(&db.post_history.post_history_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 10) as i64, a[1] + (t == 11) as i64]);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let mut v = drain((&rp).select(origid.select(&uidx).select(Ident::<User>::new().and(&us)).and((&prc).opt())));
    v.sort_by_key(|&(p, ((_, a), _))| (Reverse(score.get(p).unwrap()), Reverse(a[3])));
    rows(v.into_iter().map(|(p, ((u, a), c))| {
        let c = c.unwrap_or([0, 0]);
        let mut f = post_fields(db, p, &["id", "title", "score", "answers", "views"]);
        f.extend([user_col(db, u, "uid"), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(c[0]), V::I(c[1])]);
        row(f)
    }))
}

// WITH RankedUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.Views, u.UpVotes, u.DownVotes, RANK() OVER (ORDER BY u.Reputation DESC) AS ReputationRank FROM Users u WHERE u.Reputation > 0),
// RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days'),
// UserPostCounts AS (SELECT rp.OwnerUserId, COUNT(rp.PostId) AS RecentPostCount FROM RecentPosts rp GROUP BY rp.OwnerUserId),
// TopUsers AS (SELECT ru.UserId, ru.DisplayName, ru.Reputation, upc.RecentPostCount FROM RankedUsers ru JOIN UserPostCounts upc ON ru.UserId = upc.OwnerUserId WHERE ru.ReputationRank <= 100)
// SELECT tu.DisplayName, tu.Reputation, tu.RecentPostCount, COALESCE(SUM(p.Score), 0) AS TotalPostScore, COALESCE(SUM(c.Score), 0) AS TotalCommentScore
// FROM TopUsers tu LEFT JOIN Posts p ON p.OwnerUserId = tu.UserId LEFT JOIN Comments c ON c.UserId = tu.UserId GROUP BY tu.DisplayName, tu.Reputation, tu.RecentPostCount ORDER BY tu.Reputation DESC;
fn q7198(db: &'static So) -> String {
    let r = ranked(drain(db.user.with((&db.user.reputation).gt(0)).select(&db.user.reputation)), |&(_, r)| Reverse(r), false);
    let ru: MatSet<Id<User>> = rel(r.into_iter().take_while(|x| x.1 <= 100).map(|x| x.0 .0).collect()).map(|u| u).collect();
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let upc = db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 0, 0, 0), -30))).group_by(owner_user).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let tu = (&ru).with(&upc);
    let g = tu
        .group_by((&db.user.display_name).and(&db.user.reputation).and(&upc))
        .select(posts_of(db).select(score).opt().and(comments_by(db).select(&db.comment.score).opt()))
        .fold([0i64; 2], |a, (p, c)| [a[0] + p.unwrap_or(0), a[1] + c.unwrap_or(0)]);
    let mut v = drain(&g);
    v.sort_by_key(|&(((_, r), _), _)| Reverse(r));
    rows(v.into_iter().map(|(((n, r), c), a)| row(vec![V::S(n), V::I(r), V::I(c), V::I(a[0]), V::I(a[1])])))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(CASE WHEN P.ViewCount > 1000 THEN 1 ELSE 0 END) AS HighViews, AVG(U.Reputation) AS AvgReputation
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, Questions, Answers, HighViews, AvgReputation, RANK() OVER (ORDER BY PostCount DESC) AS RankByPosts FROM UserActivity),
// UserBadges AS (SELECT B.UserId, COUNT(B.Id) AS BadgeCount, MAX(B.Class) AS HighestBadgeClass FROM Badges B GROUP BY B.UserId),
// FinalMetrics AS (SELECT TU.UserId, TU.DisplayName, TU.PostCount, TU.Questions, TU.Answers, TU.HighViews, TU.AvgReputation, UB.BadgeCount, UB.HighestBadgeClass, TU.RankByPosts
//     FROM TopUsers TU LEFT JOIN UserBadges UB ON TU.UserId = UB.UserId)
// SELECT *, CASE WHEN RankByPosts <= 10 THEN 'Top Contributor' WHEN BadgeCount > 5 THEN 'Veteran Contributor' ELSE 'Regular Contributor' END AS ContributorType
// FROM FinalMetrics ORDER BY RankByPosts, AvgReputation DESC;
fn q9164(db: &'static So) -> String {
    let Post { post_type_id, view_count, .. } = &db.post;
    let ua = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(view_count.opt())).opt()).fold([0i64; 4], |a, p| match p {
        Some((t, w)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.map_or(false, |w| w > 1000) as i64],
        None => a,
    });
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64, i64::MIN], |a, c| [a[0] + 1, a[1].max(c)]);
    let r = ranked(drain((&ua).and((&ub).opt())), |&(_, (a, _))| Reverse(a[0]), false);
    rows(r.into_iter().map(|((u, (a, b)), r)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        f.push(V::F(db.user.reputation.get(u).unwrap() as f64));
        f.extend(match b {
            Some(b) => [V::I(b[0]), V::I(b[1])],
            None => [V::Null, V::Null],
        });
        f.push(V::I(r));
        f.push(V::S(if r <= 10 { "Top Contributor" } else if b.map_or(false, |b| b[0] > 5) { "Veteran Contributor" } else { "Regular Contributor" }));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, TotalViews, UpVotes, DownVotes, RANK() OVER (ORDER BY TotalViews DESC) AS ViewRank FROM UserActivity),
// ClosedPosts AS (SELECT ph.PostId, COUNT(*) AS CloseCount FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId)
// SELECT tu.UserId, tu.DisplayName, tu.PostCount, tu.TotalViews, tu.UpVotes, tu.DownVotes, tu.ViewRank, COALESCE(cp.CloseCount, 0) AS ClosePostCount,
//        CASE WHEN tu.UpVotes > tu.DownVotes THEN 'Positive Contributor' WHEN tu.UpVotes < tu.DownVotes THEN 'Negative Contributor' ELSE 'Neutral Contributor' END AS ContributorType
// FROM TopUsers tu LEFT JOIN ClosedPosts cp ON tu.UserId = (SELECT p.OwnerUserId FROM Posts p WHERE p.Id = cp.PostId LIMIT 1) WHERE tu.ViewRank <= 10 ORDER BY tu.ViewRank;
//
// The correlated subquery is the closed post's own owner, so each user meets the closed posts they own.
fn q334(db: &'static So) -> String {
    let Post { view_count, .. } = &db.post;
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(view_count.opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some((w, t)) => [a[0] + 1, a[1] + w.unwrap_or(0), a[2] + (t == Some(2)) as i64, a[3] + (t == Some(3)) as i64],
            None => a,
        });
    let cp = db.post_history.with((&db.post_history.post_history_type_id).eq(10)).group_by(&db.post_history.post).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let r = ranked(drain(&ua), |&(_, a)| Reverse(a[1]), false);
    let tu = rel(r.into_iter().take_while(|x| x.1 <= 10).collect());
    type R = ((Id<User>, [i64; 4]), i64);
    let v = drain((&tu).select(Same::<R>::new().and(Same::<R>::new().map(|((u, _), _): R| u).select(posts_of(db).select(&cp)).opt())));
    rows(v.into_iter().map(|(_, (((u, a), r), c))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        f.extend([V::I(r), V::I(c.unwrap_or(0))]);
        f.push(V::S(if a[2] > a[3] { "Positive Contributor" } else if a[2] < a[3] { "Negative Contributor" } else { "Neutral Contributor" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, u.DisplayName AS Author, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS rn
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1),
// PostVoteCounts AS (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes v GROUP BY v.PostId),
// ClosedPosts AS (SELECT ph.PostId, COUNT(*) AS CloseVotes FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId),
// AggregatePostData AS (SELECT rp.PostId, rp.Title, rp.Score, rp.CreationDate, rp.Author, COALESCE(pvc.UpVotes, 0) AS UpVotes, COALESCE(pvc.DownVotes, 0) AS DownVotes,
//        COALESCE(cp.CloseVotes, 0) AS CloseVotes FROM RankedPosts rp LEFT JOIN PostVoteCounts pvc ON rp.PostId = pvc.PostId LEFT JOIN ClosedPosts cp ON rp.PostId = cp.PostId WHERE rp.rn = 1)
// SELECT apd.PostId, apd.Title, apd.Score, apd.CreationDate, apd.Author, apd.UpVotes, apd.DownVotes, apd.CloseVotes, CASE WHEN apd.CloseVotes > 0 THEN 'Closed' ELSE 'Open' END AS PostStatus
// FROM AggregatePostData apd ORDER BY apd.Score DESC, apd.CreationDate DESC LIMIT 10;
fn q3806(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, creation_date, .. } = &db.post;
    let first = top_per(drain(db.post.with(post_type_id.eq(1)).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 1, false);
    let rp: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pv = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let cp = db.post_history.with((&db.post_history.post_history_type_id).eq(10)).group_by(&db.post_history.post).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let v = top_n(drain((&rp).select((&pv).opt().and((&cp).opt()))), |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())), 10);
    rows(v.into_iter().map(|(p, (a, c))| {
        let a = a.unwrap_or([0, 0]);
        let c = c.unwrap_or(0);
        let mut f = post_fields(db, p, &["id", "title", "score", "created", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c), V::S(if c > 0 { "Closed" } else { "Open" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Tags, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.Tags ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// AggregatedData AS (SELECT r.PostId, r.Title, r.Tags, r.CreationDate, r.Score, r.ViewCount, r.AnswerCount, COUNT(c.Id) AS CommentCount, AVG(u.Reputation) AS AverageReputation,
//        SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM RankedPosts r LEFT JOIN Comments c ON c.PostId = r.PostId LEFT JOIN Users u ON u.Id = r.PostId LEFT JOIN Badges b ON b.UserId = u.Id WHERE r.Rank = 1
//     GROUP BY r.PostId, r.Title, r.Tags, r.CreationDate, r.Score, r.ViewCount, r.AnswerCount)
// SELECT a.PostId, a.Title, a.Tags, a.CreationDate, a.Score, a.ViewCount, a.AnswerCount, a.CommentCount, a.AverageReputation, a.GoldBadges, a.SilverBadges, a.BronzeBadges
// FROM AggregatedData a ORDER BY a.Score DESC, a.ViewCount DESC LIMIT 50;
//
// u.Id = r.PostId compares a user id with a post id, so it goes through origid.
fn q27641(db: &'static So) -> String {
    let Post { post_type_id, creation_date, tags_str, score, view_count, origid, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(tags_str.opt()));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let rp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let ad = (&rp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(origid.select(&uidx).select((&db.user.reputation).and(badges_of(db).select(&db.badge.class).opt())).opt()))
        .fold([0i64; 6], |a, (c, u)| {
            let (r, b) = u.map_or((None, None), |(r, b)| (Some(r), b));
            [a[0] + c.is_some() as i64, a[1] + r.is_some() as i64, a[2] + r.unwrap_or(0), a[3] + (b == Some(1)) as i64, a[4] + (b == Some(2)) as i64, a[5] + (b == Some(3)) as i64]
        });
    let v = top_n(drain(&ad), |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    }, 50);
    rows(v.into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "tags", "created", "score", "views", "answers"]);
        f.extend([V::I(a[0]), avg(a[2], a[1]), V::I(a[3]), V::I(a[4]), V::I(a[5])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, p.Tags, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY t.Id ORDER BY p.CreationDate DESC) AS PostRank, t.TagName
//     FROM Posts p JOIN Tags t ON p.Tags LIKE '%' || t.TagName || '%' JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.Body, rp.CreationDate, rp.ViewCount, rp.Score, rp.AnswerCount, rp.Tags, rp.OwnerDisplayName, rp.TagName,
//        COALESCE(PH.RevisionsCount, 0) AS RevisionsCount
//     FROM RankedPosts rp LEFT JOIN (SELECT PostId, COUNT(*) AS RevisionsCount FROM PostHistory WHERE PostHistoryTypeId NOT IN (10, 12) GROUP BY PostId) PH ON rp.PostId = PH.PostId
//     WHERE rp.PostRank <= 5)
// SELECT fp.*, PH.Comment AS PostEditComment FROM FilteredPosts fp LEFT JOIN PostHistory PH ON fp.PostId = PH.PostId WHERE PH.PostHistoryTypeId IN (4, 5, 6)
// ORDER BY fp.TagName, fp.Score DESC, fp.RevisionsCount DESC;
fn q25270(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let recent = Ident::<Post>::new().with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).with(owner_user);
    let lt = tag_mentions(db);
    let rows_: Vec<((Id<Post>, Id<Tag>), Id<Post>)> = drain((&lt).map(|(p, _)| p).select(recent)).into_iter().map(|((p, t), _)| ((p, t), p)).collect();
    let top = top_per(rows_, |&((_, t), _)| t, |&((p, _), _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let fp = rel(top.into_iter().map(|x| x.0).collect());
    let PostHistory { post_history_type_id, .. } = &db.post_history;
    let rc = db.post_history.with(post_history_type_id.filt(|t| t != 10 && t != 12)).group_by(&db.post_history.post).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let edits = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.is_in([4, 5, 6])));
    type R = (Id<Post>, Id<Tag>);
    let p_of = || Same::<R>::new().map(|(p, _): R| p);
    let v = drain((&fp).select(Same::<R>::new().and(p_of().select((&rc).opt())).and(p_of().select(edits))));
    rows(v.into_iter().map(|(_, (((p, t), r), h))| {
        let mut f = post_fields(db, p, &["id", "title", "body", "created", "views", "score", "answers", "tags", "owner"]);
        f.extend([V::S(db.tag.tag_name.get(t).unwrap()), V::I(r.unwrap_or(0)), ostr(db.post_history.comment.get(h))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount, RANK() OVER (ORDER BY p.Score DESC) AS RankByScore
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= '2024-10-01 12:34:56'::timestamp - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.Score, p.CreationDate, u.DisplayName),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName
//     HAVING SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) > SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END))
// SELECT r.PostId, r.Title, r.Score, r.CreationDate, r.OwnerDisplayName, r.CommentCount, t.UserId, t.DisplayName AS TopUser, t.UpVotes, t.DownVotes, t.BadgeCount
// FROM RankedPosts r JOIN TopUsers t ON r.OwnerDisplayName = t.DisplayName WHERE r.RankByScore <= 10 ORDER BY r.Score DESC, t.UpVotes DESC;
//
// TopUsers is folded only for the users whose name matches a top question's owner, the rows the join can reach.
fn q5722(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, .. } = &db.post;
    let r = ranked(drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).with(owner_user).select(score)), |&(_, s)| Reverse(s), false);
    let rp: MatSet<Id<Post>> = rel(r.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect()).map(|p| p).collect();
    let by_name: HashIdx<Str, Id<User>> = (&db.user.display_name).inv().collect();
    let named = || owner_user.select(&db.user.display_name).select(&by_name);
    let users: MatSet<Id<User>> = (&rp).select(named()).collect();
    let tu = (&users)
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt().and(badges_of(db).opt()))
        .fold([0i64; 3], |a, (t, b)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + b.is_some() as i64]);
    let v = drain((&rp).select(comments_per_post(db).and(named().select(Ident::<User>::new().and((&tu).filt(|a| a[0] > a[1]))))));
    let mut v = v;
    v.sort_by_key(|&(p, (_, (_, a)))| (Reverse(score.get(p).unwrap()), Reverse(a[0])));
    rows(v.into_iter().map(|(p, (c, (u, a)))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "created", "owner"]);
        f.push(V::I(c));
        f.extend(ucols(db, u, &["uid", "name"]));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        COUNT(DISTINCT p.Id) AS PostCount, COUNT(DISTINCT c.Id) AS CommentCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON v.UserId = u.Id AND v.PostId = p.Id GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, (UpVotes - DownVotes) AS NetVotes, PostCount + (CommentCount / 2) AS EngagementScore FROM UserReputation WHERE PostCount > 0
//     ORDER BY NetVotes DESC, EngagementScore DESC LIMIT 10),
// PostDetails AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS Rank, p.OwnerUserId
//     FROM Posts p JOIN TopUsers tu ON p.OwnerUserId = tu.UserId),
// FinalOutput AS (SELECT tu.DisplayName, pd.PostId, pd.Title, pd.CreationDate, pd.Score, RANK() OVER (ORDER BY pd.Score DESC) AS PostRank FROM PostDetails pd JOIN TopUsers tu ON pd.OwnerUserId = tu.UserId)
// SELECT DisplayName, PostId, Title, CreationDate, Score, PostRank FROM FinalOutput ORDER BY DisplayName, PostRank;
//
// v.UserId = u.Id AND v.PostId = p.Id with p owned by u: the owner's own votes on the post (own_votes).
fn q6163(db: &'static So) -> String {
    let ur = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(comments_of(db).opt().and(own_votes(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 2], |a, p| match p {
            Some((_, t)) => [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64],
            None => a,
        });
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let cc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let eng = |p: i64, c: i64| p as f64 + c as f64 / 2.0;
    let v = top_n(drain((&ur).and(&pc).and(&cc)), |&(_, ((a, p), c))| (Reverse(a[0] - a[1]), Reverse(fkey(eng(p, c)))), 10);
    let tu: MatSet<Id<User>> = rel(v.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let pd = drain((&tu).select(posts_of(db).select(Ident::<Post>::new().and(&db.post.score))));
    let r = ranked(pd, |&(_, (_, s))| Reverse(s), false);
    rows(r.into_iter().map(|((u, (p, _)), r)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["id", "title", "created", "score"]));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH UserVoteStats AS (SELECT U.Id AS UserId, U.DisplayName, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        COUNT(DISTINCT P.Id) AS TotalPosts FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId LEFT JOIN Posts P ON V.PostId = P.Id GROUP BY U.Id, U.DisplayName),
// PostWithMaxVotes AS (SELECT P.Id AS PostId, COUNT(DISTINCT V.Id) AS VoteCount, ROW_NUMBER() OVER(PARTITION BY P.OwnerUserId ORDER BY COUNT(DISTINCT V.Id) DESC) AS Rank
//     FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY P.Id, P.OwnerUserId),
// ClosedPosts AS (SELECT PH.PostId, PH.CreationDate AS CloseDate, C.Name AS CloseReason FROM PostHistory PH JOIN CloseReasonTypes C ON CAST(PH.Comment AS INTEGER) = C.Id WHERE PH.PostHistoryTypeId = 10)
// SELECT U.DisplayName AS UserName, U.TotalPosts, U.UpVotes, U.DownVotes, P.Title AS TopPostTitle, PM.VoteCount, CP.CloseDate, CP.CloseReason
// FROM UserVoteStats U LEFT JOIN PostWithMaxVotes PM ON U.UserId = PM.PostId LEFT JOIN Posts P ON PM.PostId = P.Id LEFT JOIN ClosedPosts CP ON P.Id = CP.PostId
// WHERE U.TotalPosts > 0 AND (U.UpVotes - U.DownVotes) > 10 ORDER BY U.UpVotes DESC, U.DisplayName;
//
// U.UserId = PM.PostId compares a user id with a post id, so it goes through origid.
fn q3260(db: &'static So) -> String {
    let Vote { vote_type_id, post, .. } = &db.vote;
    let uv = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let dp = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(post).opt()).buf_fold(distinct_some);
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post_history_type_id, comment, creation_date: hd, .. } = &db.post_history;
    let cp: HashIdx<Id<Post>, Id<PostHistory>> = db
        .post_history
        .with(post_history_type_id.eq(10))
        .with(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason))
        .select(&db.post_history.post)
        .inv()
        .collect();
    let pm = (Ident::<Post>::new()).and(votes_per_post(db));
    let v = drain((&uv).filt(|a| a[0] - a[1] > 10).and((&dp).filt(|n| n > 0)).and((&db.user.origid).select(&pidx).select(pm.and((&cp).opt())).opt()));
    let mut v = v;
    v.sort_by_key(|&(u, ((a, _), _))| (Reverse(a[0]), db.user.display_name.get(u).unwrap()));
    rows(v.into_iter().map(|(u, ((a, n), m))| {
        let mut f = vec![user_col(db, u, "name"), V::I(n), V::I(a[0]), V::I(a[1])];
        f.extend(match m {
            Some(((p, c), h)) => [
                ostr(db.post.title.get(p)),
                V::I(c),
                h.map_or(V::Null, |h| V::T(hd.get(h).unwrap())),
                h.map_or(V::Null, |h| V::S(comment.get(h).and_then(|s| s.trim().parse::<i64>().ok()).and_then(|i| reason.get(i)).unwrap())),
            ],
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank,
//        p.OwnerUserId FROM Posts p WHERE p.PostTypeId = 1),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS TotalBadges, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(ub.TotalBadges, 0) AS BadgeCount, COALESCE(ub.GoldBadges, 0) AS GoldBadges, COALESCE(ub.SilverBadges, 0) AS SilverBadges,
//        COALESCE(ub.BronzeBadges, 0) AS BronzeBadges FROM Users u LEFT JOIN UserBadges ub ON u.Id = ub.UserId ORDER BY u.Reputation DESC LIMIT 10)
// SELECT tu.UserId, tu.DisplayName, tu.Reputation, tu.BadgeCount, rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount
// FROM TopUsers tu LEFT JOIN RankedPosts rp ON tu.UserId = rp.OwnerUserId WHERE rp.Rank <= 3 ORDER BY tu.Reputation DESC, rp.Score DESC;
fn q32588(db: &'static So) -> String {
    let r = top_n(drain(&db.user.reputation), |&(_, r)| Reverse(r), 10);
    let tu: MatSet<Id<User>> = rel(r.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let Post { post_type_id, owner_user, score, creation_date, .. } = &db.post;
    let top = top_per(drain((&tu).select(posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1))))), |&(u, _)| u, |&(_, p)| {
        (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p)
    }, 3, false);
    let rp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.1).collect()).map(|p| p).collect();
    let bc = (&tu).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let mut v = drain((&rp).select(owner_user.select(Ident::<User>::new().and(&bc))));
    v.sort_by_key(|&(p, (u, _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(score.get(p).unwrap())));
    rows(v.into_iter().map(|(p, (u, b))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.push(V::I(b));
        f.extend(post_fields(db, p, &["id", "title", "created", "score", "views"]));
        row(f)
    }))
}

// WITH TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, RANK() OVER (ORDER BY u.Reputation DESC) AS UserRank FROM Users u WHERE u.Reputation > 1000),
// PostDetails AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, COUNT(c.Id) AS CommentCount, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounties
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 9 WHERE p.CreationDate >= '2023-01-01'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId),
// FinalPostStats AS (SELECT pd.PostId, pd.Title, pd.CreationDate, pd.CommentCount, pu.DisplayName AS OwnerDisplayName, pu.Reputation AS OwnerReputation, pu.UserRank, pd.TotalBounties,
//        ROW_NUMBER() OVER (PARTITION BY pu.UserId ORDER BY pd.CreationDate DESC) AS UserPostRank FROM PostDetails pd JOIN TopUsers pu ON pd.OwnerUserId = pu.UserId)
// SELECT fps.PostId, fps.Title, fps.CreationDate, fps.CommentCount, fps.OwnerDisplayName, fps.TotalBounties,
//        CASE WHEN fps.UserPostRank = 1 THEN 'Newest Post' WHEN fps.UserPostRank <= 5 THEN 'Top 5 Recent Posts' ELSE 'Past Posts' END AS PostStatus
// FROM FinalPostStats fps WHERE fps.CommentCount > 10 ORDER BY fps.TotalBounties DESC, fps.CreationDate DESC LIMIT 20;
fn q3811(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let top = Ident::<User>::new().with((&db.user.reputation).gt(1000));
    let pd: MatSet<Id<Post>> = db.post.with(creation_date.ge(ts(2023, 1, 1, 0, 0, 0))).with(owner_user.select(top)).collect();
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(9))).select((&db.vote.bounty_amount).opt());
    let s = (&pd).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(bounty.opt())).fold([0i64; 2], |a, (c, b)| [a[0] + c.is_some() as i64, a[1] + b.flatten().unwrap_or(0)]);
    let r = per_group(ranked(drain((&pd).select(owner_user)), |&(p, u)| (u, Reverse(creation_date.get(p).unwrap()), p), false), |&(_, u)| u);
    let rk = rel(r.into_iter().map(|((p, _), r)| (p, r)).collect());
    let by_post: HashIdx<Id<Post>, (Id<Post>, i64)> = (&rk).map(|(p, _)| p).inv().select(&rk).collect();
    let v = drain((&s).filt(|a| a[0] > 10).and((&by_post).map(|(_, r)| r)));
    let v = top_n(v, |&(p, (a, _))| (Reverse(a[1]), Reverse(creation_date.get(p).unwrap())), 20);
    rows(v.into_iter().map(|(p, (a, r))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.push(V::I(a[0]));
        f.extend(post_fields(db, p, &["owner"]));
        f.extend([V::I(a[1]), V::S(if r == 1 { "Newest Post" } else if r <= 5 { "Top 5 Recent Posts" } else { "Past Posts" })]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U),
// PostsStats AS (SELECT P.OwnerUserId, COUNT(*) AS TotalPosts, SUM(COALESCE(P.Score, 0)) AS TotalScore, SUM(COALESCE(P.ViewCount, 0)) AS TotalViews,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount FROM Posts P GROUP BY P.OwnerUserId),
// ClosedPosts AS (SELECT PH.UserId, COUNT(*) AS ClosedPostCount FROM PostHistory PH WHERE PH.PostHistoryTypeId = 10 GROUP BY PH.UserId)
// SELECT U.DisplayName, U.Reputation, UR.ReputationRank, PS.TotalPosts, PS.TotalScore, PS.TotalViews, PS.QuestionCount, PS.AnswerCount, COALESCE(CP.ClosedPostCount, 0) AS ClosedPostCount,
//        CASE WHEN UR.ReputationRank <= 10 THEN 'Top Contributor' WHEN UR.ReputationRank <= 50 THEN 'Valuable Contributor' ELSE 'Regular Contributor' END AS ContributorType
// FROM Users U LEFT JOIN UserReputation UR ON U.Id = UR.UserId LEFT JOIN PostsStats PS ON U.Id = PS.OwnerUserId LEFT JOIN ClosedPosts CP ON U.Id = CP.UserId
// WHERE U.Reputation > 0 ORDER BY U.Reputation DESC, PS.TotalScore DESC LIMIT 100;
fn q3909(db: &'static So) -> String {
    let Post { owner_user, score, view_count, post_type_id, .. } = &db.post;
    let ps = db.post.group_by(owner_user).select(score.and(view_count.opt()).and(post_type_id)).fold([0i64; 5], |a, ((s, w), t)| {
        [a[0] + 1, a[1] + s, a[2] + w.unwrap_or(0), a[3] + (t == 1) as i64, a[4] + (t == 2) as i64]
    });
    let cp = db.post_history.with((&db.post_history.post_history_type_id).eq(10)).group_by(&db.post_history.user).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let r = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let ur = rel(r.into_iter().map(|((u, _), r)| (u, r)).collect());
    type R = (Id<User>, i64);
    let u_of = || Same::<R>::new().map(|(u, _): R| u);
    let v = drain((&ur).select(Same::<R>::new().and(u_of().select((&ps).opt())).and(u_of().select((&cp).opt()))).filt(|(((u, _), _), _): ((R, Option<[i64; 5]>), Option<i64>)| db.user.reputation.get(u).unwrap() > 0));
    let v = top_n(v, |&(_, (((u, _), p), _))| (Reverse(db.user.reputation.get(u).unwrap()), p.is_none(), Reverse(p.map(|a| a[1]))), 100);
    rows(v.into_iter().map(|(_, (((u, r), p), c))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(r));
        f.extend(match p {
            Some(a) => a.map(V::I),
            None => [V::Null, V::Null, V::Null, V::Null, V::Null],
        });
        f.extend([V::I(c.unwrap_or(0)), V::S(if r <= 10 { "Top Contributor" } else if r <= 50 { "Valuable Contributor" } else { "Regular Contributor" })]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, UpVotes, DownVotes, GoldBadges, SilverBadges, BronzeBadges,
//        ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS Ranking FROM UserStats)
// SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, UpVotes, DownVotes, GoldBadges, SilverBadges, BronzeBadges, Ranking FROM TopUsers WHERE Ranking <= 10 ORDER BY Reputation DESC;
//
// Ranking reads only Reputation, so the top users are picked first and the post x vote x badge product is driven for them alone.
fn q7392(db: &'static So) -> String {
    let r = top_n(drain(&db.user.reputation), |&(u, r)| (Reverse(r), u), 10);
    let tu = rel(r.into_iter().enumerate().map(|(i, (u, _))| (u, i as i64 + 1)).collect());
    let users: MatSet<Id<User>> = (&tu).map(|(u, _)| u).collect();
    let us = (&users)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 7], |a, (p, b)| {
            let (t, v) = p.map_or((None, None), |(t, v)| (Some(t), v));
            [a[0] + (t == Some(1)) as i64, a[1] + (t == Some(2)) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64, a[4] + (b == Some(1)) as i64, a[5] + (b == Some(2)) as i64, a[6] + (b == Some(3)) as i64]
        });
    let pc = (&users).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let by_user: HashIdx<Id<User>, (Id<User>, i64)> = (&tu).map(|(u, _)| u).inv().select(&tu).collect();
    let v = drain((&us).and(&pc).and(&by_user));
    rows(v.into_iter().map(|(u, ((a, n), (_, r)))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.push(V::I(n));
        f.extend(a.map(V::I));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVoteCount, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.Tags ORDER BY p.CreationDate DESC) AS TagRank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 AND p.CreationDate > (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year') GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName, p.Tags),
// FilteredPosts AS (SELECT rp.*, pt.Name AS PostTypeName, vt.Name AS VoteTypeName FROM RankedPosts rp LEFT JOIN PostTypes pt ON pt.Id = 1 LEFT JOIN VoteTypes vt ON vt.Id = rp.UpVoteCount
//     WHERE rp.TagRank <= 5)
// SELECT fp.PostId, fp.Title, fp.CreationDate, fp.ViewCount, fp.Score, fp.OwnerDisplayName, fp.CommentCount, fp.UpVoteCount, fp.DownVoteCount, fp.PostTypeName, fp.VoteTypeName
// FROM FilteredPosts fp WHERE fp.ViewCount > 1000 ORDER BY fp.Score DESC, fp.ViewCount DESC FETCH FIRST 50 ROWS ONLY;
//
// TagRank reads only base columns, so each tag set's newest questions are picked first. pt.Id = 1 names only pt, so PostTypes is filtered and crossed.
fn q8694(db: &'static So) -> String {
    let Post { post_type_id, creation_date, tags_str, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(tags_str.opt()));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .with(view_count.gt(1000))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let vt: HashIdx<i64, Str> = (&db.vote_type.origid).inv().select(&db.vote_type.name).collect();
    let pt = left_all(drain(db.post_type.with((&db.post_type.origid).eq(1)).select(&db.post_type.name)));
    let mut v = Vec::new();
    (&s).select(Same::<[i64; 3]>::new().and(Same::<[i64; 3]>::new().map(|a: [i64; 3]| a[1]).select(&vt).opt())).cross(&pt).drive(|(p, _), ((a, n), t)| v.push((p, a, n, t)));
    let v = top_n(v, |&(p, ..)| (Reverse(score.get(p).unwrap()), Reverse(view_count.get(p))), 50);
    rows(v.into_iter().map(|(p, a, n, t)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "owner"]);
        f.extend(a.map(V::I));
        f.extend([ostr(t.map(|(_, n)| n)), ostr(n)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, ROW_NUMBER() OVER (ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId),
// UserMetrics AS (SELECT u.Id AS UserId, u.DisplayName, SUM(u.UpVotes) AS TotalUpVotes, SUM(u.DownVotes) AS TotalDownVotes, COUNT(DISTINCT p.Id) AS TotalPosts,
//        SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName)
// SELECT rp.Rank, rp.Title, rp.CreationDate, um.TotalPosts, um.TotalUpVotes, um.TotalDownVotes, um.GoldBadges, um.SilverBadges, um.BronzeBadges
// FROM RankedPosts rp JOIN UserMetrics um ON rp.OwnerUserId = um.UserId WHERE rp.Rank <= 10 ORDER BY rp.Rank;
//
// Rank reads only CreationDate, and none of RankedPosts' aggregates is projected, so the ten newest questions are picked and UserMetrics is folded for their owners alone.
fn q9583(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let v = top_n(drain(db.post.with(post_type_id.eq(1)).select(creation_date)), |&(p, d)| (Reverse(d), p), 10);
    let rk = rel(v.into_iter().enumerate().map(|(i, (p, _))| (p, i as i64 + 1)).collect());
    let owners: MatSet<Id<User>> = (&rk).map(|(p, _)| p).select(owner_user).collect();
    let User { up_votes, down_votes, .. } = &db.user;
    let um = (&owners)
        .group_by(Ident::<User>::new())
        .select(up_votes.and(down_votes).and(posts_of(db).opt()).and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 5], |a, (((u, d), _), b)| [a[0] + u, a[1] + d, a[2] + (b == Some(1)) as i64, a[3] + (b == Some(2)) as i64, a[4] + (b == Some(3)) as i64]);
    let pc = (&owners).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    type R = (Id<Post>, i64);
    let v = drain((&rk).select(Same::<R>::new().and(Same::<R>::new().map(|(p, _): R| p).select(owner_user.select((&um).and(&pc))))));
    rows(v.into_iter().map(|(_, ((p, r), (a, n)))| {
        let mut f = vec![V::I(r)];
        f.extend(post_fields(db, p, &["title", "created"]));
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.OwnerUserId,
//        RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS Rank FROM Posts p WHERE p.CreationDate >= DATE '2024-10-01' - INTERVAL '1 year'),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(v.Id) AS VoteCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName),
// PostDetails AS (SELECT r.PostId, r.Title, r.CreationDate, r.Score, r.ViewCount, r.AnswerCount, r.CommentCount, u.DisplayName AS OwnerDisplayName, ua.VoteCount, ua.UpVotes, ua.DownVotes, r.Rank
//     FROM RankedPosts r JOIN Users u ON r.OwnerUserId = u.Id LEFT JOIN UserActivity ua ON u.Id = ua.UserId)
// SELECT pd.Title, pd.CreationDate, pd.Score, pd.ViewCount, pd.AnswerCount, pd.CommentCount, pd.OwnerDisplayName, pd.VoteCount, pd.UpVotes, pd.DownVotes
// FROM PostDetails pd WHERE pd.Rank <= 5 ORDER BY pd.Score DESC, pd.ViewCount DESC;
fn q5164(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 0, 0, 0), -1))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| Reverse(creation_date.get(p).unwrap()), 5, true);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ua = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| {
        [a[0] + t.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let v = drain((&tp).select(owner_user.select(&ua)));
    rows(v.into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["title", "created", "score", "views", "answers", "comments", "owner"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.ViewCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// TopPosts AS (SELECT r.PostId, r.Title, r.Score, r.CreationDate, r.ViewCount, r.OwnerDisplayName FROM RankedPosts r WHERE r.Rank <= 5),
// PostVoteStats AS (SELECT p.Id AS PostId, COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 2) AS UpVotes, COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 3) AS DownVotes, COUNT(v.Id) AS TotalVotes
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id),
// FinalOutput AS (SELECT t.PostId, t.Title, t.Score, t.CreationDate, t.ViewCount, t.OwnerDisplayName, v.UpVotes, v.DownVotes, v.TotalVotes FROM TopPosts t LEFT JOIN PostVoteStats v ON t.PostId = v.PostId)
// SELECT f.*, COALESCE(f.UpVotes, 0) - COALESCE(f.DownVotes, 0) AS NetVotes FROM FinalOutput f ORDER BY f.Score DESC, f.CreationDate DESC;
fn q5901(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, score, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))).with(owner_user).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + t.is_some() as i64]
    });
    let v = drain(&pv);
    rows(v.into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "score", "created", "views", "owner"]);
        f.extend(a.map(V::I));
        f.push(V::I(a[0] - a[1]));
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(COALESCE(P.ViewCount, 0)) AS TotalViews, AVG(P.Score) AS AverageScore
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, TotalViews, AverageScore, RANK() OVER (ORDER BY PostCount DESC) AS PostRank FROM UserPostStats),
// PopularQuestions AS (SELECT P.Id AS QuestionId, P.Title, P.ViewCount, P.Score, U.DisplayName AS OwnerDisplayName FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id
//     WHERE P.PostTypeId = 1 AND P.ViewCount > 1000)
// SELECT TU.DisplayName AS TopUser, TU.PostCount, TU.QuestionCount, TU.AnswerCount, TU.TotalViews, TU.AverageScore, PQ.Title AS PopularQuestionTitle, PQ.ViewCount AS PopularQuestionViews,
//        PQ.Score AS PopularQuestionScore, PQ.OwnerDisplayName AS PopularQuestionOwner
// FROM TopUsers TU CROSS JOIN PopularQuestions PQ WHERE TU.PostRank <= 10 ORDER BY TU.PostCount DESC, PQ.ViewCount DESC;
fn q7333(db: &'static So) -> String {
    let ups = user_posts(db);
    let r = ranked(drain(&ups), |&(_, a)| Reverse(a[1]), false);
    let tu = rel(r.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0).collect());
    let Post { post_type_id, view_count, owner_user, .. } = &db.post;
    let pq: MatSet<Id<Post>> = db.post.with(post_type_id.eq(1).and(view_count.gt(1000))).with(owner_user).collect();
    let mut out = Vec::new();
    (&tu).cross(&pq).drive(|_, ((u, a), p)| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[6]), avg(a[4], a[1])];
        f.extend(post_fields(db, p, &["title", "views", "score", "owner"]));
        out.push(row(f));
    });
    rows(out)
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.OwnerUserId, p.CreationDate, p.Score, p.Title, p.Tags, ROW_NUMBER() OVER(PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.OwnerUserId IS NOT NULL),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.Score >= 0 THEN 1 ELSE 0 END) AS PositivePosts,
//        SUM(CASE WHEN p.Score < 0 THEN 1 ELSE 0 END) AS NegativePosts FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT ua.UserId, ua.DisplayName, ua.Reputation, ua.PostCount, ua.PositivePosts, ua.NegativePosts, RANK() OVER (ORDER BY ua.Reputation DESC) AS UserRank
//     FROM UserActivity ua WHERE ua.PostCount > 0)
// SELECT pu.OwnerUserId AS PostOwner, pu.PostId, pu.Title, pu.CreationDate, tu.DisplayName AS TopUser, tu.Reputation AS TopUserReputation, tu.PostCount AS TotalPosts,
//        tu.PositivePosts AS UpVotes, tu.NegativePosts AS DownVotes
// FROM RankedPosts pu JOIN TopUsers tu ON pu.OwnerUserId = tu.UserId WHERE pu.rn = 1 AND pu.Score >= 5 AND tu.UserRank <= 10 ORDER BY tu.Reputation DESC, pu.CreationDate DESC;
fn q33815(db: &'static So) -> String {
    let Post { owner_user, creation_date, score, .. } = &db.post;
    let ua = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(score)).fold([0i64; 3], |a, s| [a[0] + 1, a[1] + (s >= 0) as i64, a[2] + (s < 0) as i64]);
    let r = ranked(drain((&ua).and(&db.user.reputation)), |&(_, (_, r))| Reverse(r), false);
    let tu = rel(r.into_iter().take_while(|x| x.1 <= 10).map(|((u, (a, _)), _)| (u, a)).collect());
    let by_user: HashIdx<Id<User>, (Id<User>, [i64; 3])> = (&tu).map(|(u, _)| u).inv().select(&tu).collect();
    let first = top_per(drain(db.post.select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false);
    let rp: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let v = drain((&rp).with(score.ge(5)).select(owner_user.select(&by_user)));
    rows(v.into_iter().map(|(p, (u, a))| {
        let mut f = post_fields(db, p, &["owner_id", "id", "title", "created"]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.CreationDate, u.DisplayName, p.PostTypeId),
// TopPosts AS (SELECT PostId, Title, CreationDate, OwnerDisplayName, CommentCount, UpVoteCount, DownVoteCount FROM RankedPosts WHERE Rank <= 5)
// SELECT tp.Title, tp.OwnerDisplayName, tp.CommentCount, tp.UpVoteCount, tp.DownVoteCount, pht.Name AS PostHistoryType, COUNT(ph.Id) AS HistoryCount
// FROM TopPosts tp LEFT JOIN PostHistory ph ON tp.PostId = ph.PostId LEFT JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id
// GROUP BY tp.Title, tp.OwnerDisplayName, tp.CommentCount, tp.UpVoteCount, tp.DownVoteCount, pht.Name ORDER BY tp.UpVoteCount DESC, tp.CommentCount DESC;
//
// The final GROUP BY has no post id, so posts with the same title, owner and counts merge; it groups the (post, history) rows.
fn q8184(db: &'static So) -> String {
    let Post { creation_date, post_type_id, title, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(post_type_id));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let j: MatSet<(Id<Post>, Option<Id<PostHistory>>)> = (&tp).select(Ident::<Post>::new().and(history_of(db).opt())).collect();
    let post_of = (&j).map(|(p, _)| p);
    let hist_of = (&j).flat_map(|(_, h)| h);
    let key = (&post_of).select(title.opt().and(owner_user.select(&db.user.display_name).opt()).and(&s)).and((&hist_of).select(htype_name(db)).opt());
    let g = (&j).group_by(key).select((&hist_of).opt()).fold(0i64, |n, h| n + h.is_some() as i64);
    let mut v = drain(&g);
    v.sort_by_key(|&(((_, a), _), _)| (Reverse(a[1]), Reverse(a[0])));
    rows(v.into_iter().map(|((((t, o), a), h), n)| row(vec![ostr(t), ostr(o), V::I(a[0]), V::I(a[1]), V::I(a[2]), ostr(h), V::I(n)])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, u.DisplayName AS OwnerDisplayName, pt.Name AS PostType,
//        ROW_NUMBER() OVER (PARTITION BY pt.Name ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id JOIN PostTypes pt ON p.PostTypeId = pt.Id WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.CreationDate, rp.OwnerDisplayName, rp.PostType FROM RankedPosts rp WHERE rp.Rank <= 5),
// PostStats AS (SELECT tp.PostId, tp.Title, tp.OwnerDisplayName, tp.PostType, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM TopPosts tp LEFT JOIN Comments c ON tp.PostId = c.PostId LEFT JOIN Votes v ON tp.PostId = v.PostId GROUP BY tp.PostId, tp.Title, tp.OwnerDisplayName, tp.PostType)
// SELECT ps.PostId, ps.Title, ps.OwnerDisplayName, ps.PostType, ps.CommentCount, ps.UpVotes, ps.DownVotes, (ps.UpVotes - ps.DownVotes) AS NetVotes FROM PostStats ps
// ORDER BY NetVotes DESC, ps.CommentCount DESC;
fn q9426(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).select(ptype_name(db)));
    let top = top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = drain(&s);
    rows(v.into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "type"]);
        f.extend(a.map(V::I));
        f.push(V::I(a[1] - a[2]));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, u.DisplayName AS Owner, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// PostStats AS (SELECT p.Owner, COUNT(p.PostId) AS TotalPosts, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts, SUM(CASE WHEN p.ViewCount > 100 THEN 1 ELSE 0 END) AS PopularPosts,
//        AVG(p.Score) AS AvgScore FROM RankedPosts p GROUP BY p.Owner),
// RecentEdits AS (SELECT ph.PostId, ph.UserDisplayName, ph.CreationDate, ph.Comment FROM PostHistory ph JOIN Posts p ON ph.PostId = p.Id
//     WHERE ph.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' AND ph.PostHistoryTypeId IN (4, 5, 6))
// SELECT ps.Owner, ps.TotalPosts, ps.PositivePosts, ps.PopularPosts, ps.AvgScore, re.UserDisplayName AS LastEditor, re.CreationDate AS LastEditDate, re.Comment AS LastEditComment
// FROM PostStats ps LEFT JOIN RecentEdits re ON ps.Owner = re.UserDisplayName WHERE ps.TotalPosts > 10 ORDER BY ps.AvgScore DESC, ps.TotalPosts DESC LIMIT 50;
fn q8398(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, view_count, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let ps = db
        .post
        .with(creation_date.gt(add_years(t0, -1)))
        .group_by(owner_user.select(&db.user.display_name))
        .select(score.and(view_count.opt()))
        .fold([0i64; 4], |a, (s, w)| [a[0] + 1, a[1] + (s > 0) as i64, a[2] + w.map_or(false, |w| w > 100) as i64, a[3] + s]);
    let PostHistory { creation_date: hd, post_history_type_id, user_display_name, .. } = &db.post_history;
    let re: HashIdx<Str, Id<PostHistory>> = db.post_history.with(hd.gt(add_days(t0, -30)).and(post_history_type_id.is_in([4, 5, 6]))).select(user_display_name).inv().collect();
    let v = drain((&ps).filt(|a| a[0] > 10).and((&re).opt()));
    let v = top_n(v, |&(_, (a, _))| (Reverse(fkey(a[3] as f64 / a[0] as f64)), Reverse(a[0])), 50);
    rows(v.into_iter().map(|(o, (a, h))| {
        let mut f = vec![V::S(o), V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[3], a[0])];
        f.extend(match h {
            Some(h) => [ostr(user_display_name.get(h)), V::T(hd.get(h).unwrap()), ostr(db.post_history.comment.get(h))],
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.ViewCount, u.DisplayName AS OwnerDisplayName, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT a.Id) AS AnswerCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Score, p.CreationDate, p.ViewCount, u.DisplayName, p.OwnerUserId),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.CreationDate, rp.ViewCount, rp.OwnerDisplayName, rp.CommentCount, rp.AnswerCount FROM RankedPosts rp WHERE rp.Rank <= 5)
// SELECT tp.Title, tp.Score, tp.CreationDate, tp.ViewCount, tp.OwnerDisplayName, tp.CommentCount, tp.AnswerCount, COALESCE(b.BadgeCount, 0) AS UserBadgeCount
// FROM TopPosts tp LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges WHERE Class = 1 GROUP BY UserId) b ON tp.OwnerDisplayName = (SELECT DisplayName FROM Users WHERE Id = b.UserId)
// ORDER BY tp.Score DESC, tp.CreationDate DESC;
fn q9825(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, creation_date, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1)).select(owner_user.opt()));
    let top = top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 5, false);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let gb = db.badge.with((&db.badge.class).eq(1)).group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let by_name: HashIdx<Str, Id<User>> = (&db.user.display_name).inv().collect();
    let v = drain((&tp).select(comments_per_post(db).and(typed_answers_per_post(db)).and(owner_user.select(&db.user.display_name).select(&by_name).select(&gb).opt())));
    rows(v.into_iter().map(|(p, ((c, a), b))| {
        let mut f = post_fields(db, p, &["title", "score", "created", "views", "owner"]);
        f.extend([V::I(c), V::I(a), V::I(b.unwrap_or(0))]);
        row(f)
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("9794", q9794),
    ("9417", q9417),
    ("9310", q9310),
    ("26261", q26261),
    ("6431", q6431),
    ("8285", q8285),
    ("9010", q9010),
    ("5285", q5285),
    ("6257", q6257),
    ("8082", q8082),
    ("8775", q8775),
    ("9509", q9509),
    ("7838", q7838),
    ("9401", q9401),
    ("1432", q1432),
    ("7231", q7231),
    ("29891", q29891),
    ("6873", q6873),
    ("28603", q28603),
    ("1517", q1517),
    ("3539", q3539),
    ("9755", q9755),
    ("5752", q5752),
    ("6741", q6741),
    ("8018", q8018),
    ("8755", q8755),
    ("9606", q9606),
    ("9955", q9955),
    ("33778", q33778),
    ("1531", q1531),
    ("5207", q5207),
    ("7872", q7872),
    ("9867", q9867),
    ("741", q741),
    ("7015", q7015),
    ("871", q871),
    ("34076", q34076),
    ("29985", q29985),
    ("866", q866),
    ("6250", q6250),
    ("4007", q4007),
    ("9626", q9626),
    ("7387", q7387),
    ("7949", q7949),
    ("26787", q26787),
    ("1339", q1339),
    ("6824", q6824),
    ("28985", q28985),
    ("17", q17),
    ("114", q114),
    ("2023", q2023),
    ("3830", q3830),
    ("9793", q9793),
    ("6793", q6793),
    ("9911", q9911),
    ("3796", q3796),
    ("5591", q5591),
    ("6236", q6236),
    ("7519", q7519),
    ("3059", q3059),
    ("5081", q5081),
    ("8115", q8115),
    ("6943", q6943),
    ("7417", q7417),
    ("6224", q6224),
    ("7298", q7298),
    ("7920", q7920),
    ("29886", q29886),
    ("1284", q1284),
    ("6173", q6173),
    ("27782", q27782),
    ("3713", q3713),
    ("1271", q1271),
    ("4819", q4819),
    ("7198", q7198),
    ("9164", q9164),
    ("334", q334),
    ("3806", q3806),
    ("27641", q27641),
    ("25270", q25270),
    ("5722", q5722),
    ("6163", q6163),
    ("3260", q3260),
    ("32588", q32588),
    ("3811", q3811),
    ("3909", q3909),
    ("7392", q7392),
    ("8694", q8694),
    ("9583", q9583),
    ("5164", q5164),
    ("5901", q5901),
    ("7333", q7333),
    ("33815", q33815),
    ("8184", q8184),
    ("9426", q9426),
    ("8398", q8398),
    ("9825", q9825),
];
