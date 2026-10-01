use harness::prelude::*;
use std::cmp::Reverse;

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS OwnerDisplayName, p.ViewCount, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT v.Id) AS UpvoteCount,
//        RANK() OVER (ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 2
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, u.DisplayName, p.ViewCount),
// TopPosts AS (SELECT PostId, Title, OwnerDisplayName, CreationDate, ViewCount, CommentCount, UpvoteCount, PostRank FROM RankedPosts WHERE PostRank <= 10)
// SELECT tp.Title, tp.OwnerDisplayName, tp.CreationDate, tp.ViewCount, tp.CommentCount, tp.UpvoteCount, ph.UserDisplayName AS LastEditedBy, ph.CreationDate AS LastEditDate, ph.Comment AS EditComment
// FROM TopPosts tp LEFT JOIN PostHistory ph ON tp.PostId = ph.PostId AND ph.PostHistoryTypeId IN (4, 5, 24)
// WHERE ph.CreationDate = (SELECT MAX(CreationDate) FROM PostHistory WHERE PostId = tp.PostId AND PostHistoryTypeId IN (4, 5, 24)) ORDER BY tp.ViewCount DESC;
fn q7545(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, view_count, .. } = &db.post;
    let w = whole(db.post.iq()).select(Ident::<Post>::new().with(post_type_id.eq(1)).with(owner_user).and(creation_date)).window(rank, |(_, d)| Reverse(d), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|((p, _), _)| p).collect();
    let up: HashIdx<Id<Post>, Id<Vote>> = db.vote.with((&db.vote.vote_type_id).eq(2)).select(&db.vote.post).inv().collect();
    let dc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db)).count_distinct();
    let du = (&tp).group_by(Ident::<Post>::new()).select(&up).count_distinct();
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let edits = || db.post_history.with(post_history_type_id.is_in([4, 5, 24]));
    let md = edits().group_by(post).select(hd).fold(i64::MIN, |m, d| m.max(d));
    let at: HashIdx<(Id<Post>, i64), Id<PostHistory>> = edits().select(post.and(hd)).inv().collect();
    let mut v = drain((&tp).select((&dc).opt().and((&du).opt()).and(Ident::<Post>::new().and(&md).select(&at))));
    v.sort_by_key(|&(p, _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w))
    });
    rows(v.into_iter().map(|(p, ((c, u), h))| {
        let mut f = post_fields(db, p, &["title", "owner", "created", "views"]);
        f.extend([V::I(c.unwrap_or(0)), V::I(u.unwrap_or(0)),harness::fmt::ostr(db.post_history.user_display_name.get(h)), V::T(hd.get(h).unwrap()), harness::fmt::ostr(db.post_history.comment.get(h))]);
        row(f)
    }))
}

// WITH PostStatistics AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, ARRAY_LENGTH(string_to_array(p.Tags, '><'), 1) AS TagCount,
//        COALESCE(a.AnswerCount, 0) AS AnswerCount, COALESCE(c.CommentCount, 0) AS CommentCount, COALESCE(substring(p.Body, 1, 100), '') AS Snippet
//     FROM Posts p LEFT JOIN (SELECT ParentId, COUNT(*) AS AnswerCount FROM Posts WHERE PostTypeId = 2 GROUP BY ParentId) a ON p.Id = a.ParentId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId WHERE p.PostTypeId = 1),
// TopPosts AS (SELECT ps.PostId, ps.Title, ps.CreationDate, ps.ViewCount, ps.Score, ps.TagCount, ps.AnswerCount, ps.CommentCount, ps.Snippet,
//        ROW_NUMBER() OVER (ORDER BY ps.Score DESC, ps.ViewCount DESC) AS Rank FROM PostStatistics ps)
// SELECT t.PostId, t.Title, t.CreationDate, t.ViewCount, t.Score, t.TagCount, t.AnswerCount, t.CommentCount, t.Snippet FROM TopPosts t WHERE t.Rank <= 10 ORDER BY t.Score DESC, t.ViewCount DESC;
fn q26887(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, tags_str, body, .. } = &db.post;
    let key = |p: Id<Post>| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    };
    let top = top_n(drain(db.post.with(post_type_id.eq(1)).select(score)), |&(p, _)| (key(p), p), 10);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ac = (&tp).group_by(Ident::<Post>::new()).select(answers_of(db).opt()).fold(0i64, |n, a| n + a.is_some() as i64);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, a| n + a.is_some() as i64);
    let mut v = drain((&ac).and(&cc));
    v.sort_by_key(|&(p, _)| key(p));
    rows(v.into_iter().map(|(p, (a, c))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.push(tags_str.get(p).map_or(V::Null, |t| V::I(t.matches("><").count() as i64 + 1)));
        f.extend([V::I(a), V::I(c)]);
        f.push(V::S(Box::leak(body.get(p).map_or(String::new(), |b| b.chars().take(100).collect()).into_boxed_str())));
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, u.DisplayName AS OwnerDisplayName, u.Reputation AS OwnerReputation,
//        ROW_NUMBER() OVER (PARTITION BY u.Id ORDER BY p.CreationDate DESC) AS rn FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '6 MONTH'),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, rp.CommentCount, rp.OwnerDisplayName, rp.OwnerReputation, ht.Name AS HistoryTypeName,
//        ph.CreationDate AS HistoryCreationDate, ph.Comment AS HistoryComment, RANK() OVER (ORDER BY rp.Score DESC) AS Rank
//     FROM RecentPosts rp LEFT JOIN PostHistory ph ON rp.PostId = ph.PostId LEFT JOIN PostHistoryTypes ht ON ph.PostHistoryTypeId = ht.Id)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.AnswerCount, tp.CommentCount, tp.OwnerDisplayName, tp.OwnerReputation, tp.HistoryTypeName,
//        tp.HistoryCreationDate, tp.HistoryComment FROM TopPosts tp WHERE tp.Rank <= 10 ORDER BY tp.Score DESC, tp.CreationDate DESC;
//
// CURRENT_DATE is the day the query runs; the data ends in 2024, so it is empty.
fn q6153(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    type J = (Id<Post>, Option<(Id<PostHistory>, Str)>);
    let j: MatSet<J> = db
        .post
        .with(creation_date.ge(add_months(current_date(), -6)))
        .with(owner_user)
        .select(Ident::<Post>::new().and(history_of(db).select(Ident::<PostHistory>::new().and(htype_name(db))).opt()))
        .collect();
    let w = whole(&j).select(Same::<J>::new().and(Same::<J>::new().map(|(p, _): J| p).select(score))).window(rank, |(_, s)| Reverse(s), asc);
    let mut v: Vec<_> = drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|x| x.1 .0).collect();
    v.sort_by_key(|&((p, _), s)| (Reverse(s), Reverse(creation_date.get(p).unwrap())));
    rows(v.into_iter().map(|((p, h), _)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "owner", "rep"]);
        f.extend(match h {
            Some((h, n)) => [V::S(n), V::T(db.post_history.creation_date.get(h).unwrap()), harness::fmt::ostr(db.post_history.comment.get(h))],
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserBadgeCounts AS (SELECT U.Id AS UserId, COUNT(B.Id) FILTER (WHERE B.Class = 1) AS GoldCount, COUNT(B.Id) FILTER (WHERE B.Class = 2) AS SilverCount,
//        COUNT(B.Id) FILTER (WHERE B.Class = 3) AS BronzeCount FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id),
// PostAggregate AS (SELECT P.OwnerUserId, COUNT(C.Id) AS CommentCount, SUM(P.Score) AS TotalScore, AVG(P.ViewCount) AS AvgViewCount, MAX(P.CreationDate) AS LastPostDate
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId GROUP BY P.OwnerUserId),
// ActiveUsers AS (SELECT U.Id, U.DisplayName, U.Reputation, COALESCE(B.GoldCount, 0) AS GoldCount, COALESCE(B.SilverCount, 0) AS SilverCount, COALESCE(B.BronzeCount, 0) AS BronzeCount,
//        P.CommentCount, P.TotalScore, P.AvgViewCount, P.LastPostDate, ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS Rank
//     FROM Users U LEFT JOIN UserBadgeCounts B ON U.Id = B.UserId LEFT JOIN PostAggregate P ON U.Id = P.OwnerUserId WHERE U.Reputation > 0)
// SELECT A.DisplayName, A.Reputation, A.GoldCount, A.SilverCount, A.BronzeCount, A.CommentCount, A.TotalScore, A.AvgViewCount, A.LastPostDate
// FROM ActiveUsers A WHERE A.Rank <= 10 ORDER BY A.TotalScore DESC, A.LastPostDate DESC;
//
// The row number only reads Reputation, so the ten users are picked before
// their joined rows are aggregated.
fn q3565(db: &'static So) -> String {
    let top = top_n(drain(db.user.with((&db.user.reputation).gt(0)).select(&db.user.reputation)), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(top.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let bc = (&tu).group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    let Post { score, view_count, creation_date, .. } = &db.post;
    let pa = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score.and(view_count.opt()).and(creation_date).and(comments_of(db).opt())))
        .fold([0, 0, 0, 0, i64::MIN], |a, (((s, w), d), c)| [a[0] + c.is_some() as i64, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4].max(d)]);
    let mut v = drain((&bc).and((&pa).opt()));
    v.sort_by_key(|&(_, (_, p))| p.map_or((true, Reverse(0), Reverse(0)), |a| (false, Reverse(a[1]), Reverse(a[4]))));
    rows(v.into_iter().map(|(u, (b, p))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(b.map(V::I));
        f.extend(match p {
            Some(a) => [V::I(a[0]), V::I(a[1]), avg(a[3], a[2]), tmax(a[4])],
            None => [V::Null, V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'),
// AggregatedStats AS (SELECT pt.Id AS PostTypeId, pt.Name AS PostTypeName, COUNT(rp.PostId) AS TotalPosts, SUM(rp.ViewCount) AS TotalViews, AVG(rp.Score) AS AverageScore,
//        SUM(rp.AnswerCount) AS TotalAnswers FROM PostTypes pt LEFT JOIN RankedPosts rp ON pt.Id = rp.PostId GROUP BY pt.Id, pt.Name),
// TopPosts AS (SELECT p.Title, p.Score, u.DisplayName AS OwnerDisplayName, p.CreationDate, DENSE_RANK() OVER (ORDER BY p.Score DESC) AS ScoreRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.Score > 0)
// SELECT a.PostTypeName, a.TotalPosts, a.TotalViews, a.AverageScore, a.TotalAnswers, t.Title, t.Score, t.OwnerDisplayName
// FROM AggregatedStats a LEFT JOIN TopPosts t ON a.PostTypeId = t.ScoreRank ORDER BY a.TotalPosts DESC, a.AverageScore DESC;
//
// RankedPosts is joined on its PostId against a PostTypes Id, and no recent
// post has an Id that small, so every type aggregates nothing.
fn q9179(db: &'static So) -> String {
    let Post { creation_date, owner_user, origid, score, view_count, answer_count, .. } = &db.post;
    let recent: HashIdx<i64, Id<Post>> = db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).with(owner_user).select(origid).inv().collect();
    let agg = db
        .post_type
        .group_by(Ident::<PostType>::new())
        .select((&db.post_type.origid).select(&recent).select(view_count.opt().and(score).and(answer_count.opt())).opt())
        .fold([0i64; 6], |a, x| match x {
            Some(((w, s), n)) => [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s, a[4] + n.is_some() as i64, a[5] + n.unwrap_or(0)],
            None => a,
        });
    let w = whole(db.post.iq()).select(Ident::<Post>::new().with(score.gt(0)).with(owner_user).and(score)).window(dense_rank, |(_, s)| Reverse(s), asc);
    let tpr: MatSet<(i64, Id<Post>)> = (&w).map(|((p, _), r)| (r, p)).collect();
    let by_rank: HashIdx<i64, Id<Post>> = (&tpr).map(|(r, _)| r).inv().select(&tpr).map(|(_, p)| p).collect();
    let mut v = drain((&agg).and((&db.post_type.origid).select(&by_rank).opt()));
    v.sort_by(|x, y| y.1 .0[0].cmp(&x.1 .0[0]));
    rows(v.into_iter().map(|(t, (a, p))| {
        let mut f = vec![V::S(db.post_type.name.get(t).unwrap()), V::I(a[0]), nullable(a[2], a[1]), avg(a[3], a[0]), nullable(a[5], a[4])];
        match p {
            Some(p) => f.extend(post_fields(db, p, &["title", "score", "owner"])),
            None => f.extend([V::Null, V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId
//     GROUP BY u.Id, u.DisplayName, u.Reputation),
// ActiveUsers AS (SELECT ..., RANK() OVER (ORDER BY TotalPosts DESC) AS ActivityRank FROM UserActivity WHERE TotalPosts > 0),
// TopUsers AS (SELECT * FROM ActiveUsers WHERE ActivityRank <= 10),
// PostStats AS (SELECT p.OwnerUserId, COUNT(p.Id) AS UserPostCount, AVG(p.Score) AS AvgPostScore FROM Posts p GROUP BY p.OwnerUserId)
// SELECT t.DisplayName, t.Reputation, t.TotalPosts, t.TotalQuestions, t.TotalAnswers, t.TotalUpvotes, t.TotalDownvotes, ps.UserPostCount, ps.AvgPostScore
// FROM TopUsers t JOIN PostStats ps ON t.UserId = ps.OwnerUserId ORDER BY t.Reputation DESC;
fn q9088(db: &'static So) -> String {
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 5], |a, p| match p {
            Some((t, v)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (v == Some(2)) as i64, a[4] + (v == Some(3)) as i64],
            None => a,
        });
    let ups = user_posts(db);
    let w = whole(db.user.iq()).select(Ident::<User>::new().and((&s).filt(|a| a[0] > 0)).and(&ups)).window(rank, |((_, a), _)| Reverse(a[0]), asc);
    let mut v: Vec<_> = drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|x| x.1 .0).collect();
    v.sort_by_key(|&((u, _), _)| Reverse(db.user.reputation.get(u).unwrap()));
    rows(v.into_iter().map(|((u, a), b)| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(a.map(V::I));
        f.extend([V::I(b[1]), avg(b[4], b[1])]);
        row(f)
    }))
}

// WITH UserBadgeStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS TotalBadges, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PopularPosts AS (SELECT p.Id AS PostId, p.Title, p.OwnerUserId, p.Score, p.ViewCount, ROW_NUMBER() OVER (ORDER BY p.Score DESC, p.ViewCount DESC) AS PopularityRank FROM Posts p WHERE p.PostTypeId = 1 AND p.Score > 0),
// UserPostCounts AS (SELECT p.OwnerUserId, COUNT(p.Id) AS PostCount FROM Posts p WHERE p.PostTypeId IN (1, 2) GROUP BY p.OwnerUserId)
// SELECT u.UserId, u.DisplayName, u.TotalBadges, u.GoldBadges, u.SilverBadges, u.BronzeBadges, up.PostCount, pp.PostId, pp.Title AS PopularPostTitle, pp.Score AS PopularPostScore, pp.ViewCount AS PopularPostViewCount
// FROM UserBadgeStats u JOIN UserPostCounts up ON u.UserId = up.OwnerUserId LEFT JOIN PopularPosts pp ON u.UserId = pp.OwnerUserId WHERE u.TotalBadges > 0 ORDER BY u.TotalBadges DESC, up.PostCount DESC;
fn q5973(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let Post { post_type_id, score, .. } = &db.post;
    let upc = db.post.with(post_type_id.is_in([1, 2])).group_by(&db.post.owner_user).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let mut v = drain((&ub).filt(|a| a[0] > 0).and(&upc).and(posts_of(db).with(post_type_id.eq(1).and(score.gt(0))).opt()));
    v.sort_by_key(|&(_, ((b, n), _))| (Reverse(b[0]), Reverse(n)));
    rows(v.into_iter().map(|(u, ((b, n), p))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(b.map(V::I));
        f.push(V::I(n));
        match p {
            Some(p) => f.extend(post_fields(db, p, &["id", "title", "score", "views"])),
            None => f.extend([V::Null, V::Null, V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC, p.Id) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopQuestions AS (SELECT PostId, Title, OwnerDisplayName, Score, ViewCount FROM RankedPosts WHERE Rank <= 10 AND PostId IN (SELECT PostId FROM Comments WHERE Score > 0)),
// TopAnswers AS (SELECT p.Id AS AnswerId, p.Score AS AnswerScore, p.ViewCount AS AnswerViewCount, q.Title AS QuestionTitle, q.OwnerDisplayName AS QuestionOwnerDisplayName
//     FROM Posts p JOIN Posts q ON p.ParentId = q.Id WHERE p.PostTypeId = 2 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year')
// SELECT tq.PostId, tq.Title AS QuestionTitle, tq.OwnerDisplayName AS QuestionOwner, ta.AnswerId, ta.AnswerScore, ta.AnswerViewCount, ta.QuestionTitle AS RelatedQuestionTitle, ta.QuestionOwnerDisplayName
// FROM TopQuestions tq LEFT JOIN TopAnswers ta ON tq.PostId = ta.AnswerId ORDER BY tq.Score DESC, tq.ViewCount DESC, tq.PostId;
//
// rewrites/5382.sql: the row number and the final order tie-broken on the Id.
fn q5382(db: &'static So) -> String {
    let Post { creation_date, owner_user, post_type_id, score, view_count, origid, parent, .. } = &db.post;
    let key = |p: Id<Post>| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), origid.get(p).unwrap())
    };
    let cutoff = add_years(ts(2024, 10, 1, 12, 34, 56), -1);
    let w = db
        .post
        .with(creation_date.ge(cutoff))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score.and(view_count.opt())).and(origid))
        .window(row_number, |((_, (s, w)), o)| (Reverse(s), w.is_none(), Reverse(w), o), asc);
    let liked: MatSet<Id<Post>> = db.comment.with((&db.comment.score).gt(0)).select(&db.comment.post).collect();
    let tq: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|(((p, _), _), _)| p).collect();
    let tq = (&tq).with(Ident::<Post>::new().with(&liked));
    let ta = Ident::<Post>::new().with(post_type_id.eq(2).and(creation_date.ge(cutoff))).and(parent);
    let mut v = drain(tq.select(ta.opt()));
    v.sort_by_key(|&(p, _)| key(p));
    rows(v.into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "owner"]);
        match a {
            Some((a, q)) => {
                f.extend(post_fields(db, a, &["id", "score", "views"]));
                f.extend(post_fields(db, q, &["title", "owner_name"]));
            }
            None => f.extend([V::Null, V::Null, V::Null, V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn,
//        DENSE_RANK() OVER (ORDER BY p.Score DESC) AS ScoreRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.Score),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, u.DisplayName, CASE WHEN u.Reputation >= 1000 THEN 'Experienced' WHEN u.Reputation >= 100 THEN 'Novice' ELSE 'Beginner' END AS UserLevel FROM Users u)
// SELECT up.DisplayName AS UserName, rp.Title, rp.CreationDate, rp.CommentCount, rp.UpVotes, rp.DownVotes, up.Reputation, up.UserLevel, rp.ScoreRank
// FROM RankedPosts rp INNER JOIN UserReputation up ON rp.OwnerUserId = up.UserId WHERE rp.rn = 1 AND rp.CommentCount > 5 AND (rp.UpVotes - rp.DownVotes) > 10
// ORDER BY up.Reputation DESC, rp.ScoreRank ASC FETCH FIRST 50 ROWS ONLY;
fn q1128(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, .. } = &db.post;
    let sw = whole(db.post.iq()).select(Ident::<Post>::new().with(post_type_id.eq(1)).and(score)).window(dense_rank, |(_, s)| Reverse(s), asc);
    let sr: MatSet<(Id<Post>, i64)> = (&sw).map(|((p, _), r)| (p, r)).collect();
    let by_post: HashIdx<Id<Post>, (Id<Post>, i64)> = (&sr).map(|(p, _)| p).inv().select(&sr).collect();
    let w = db.post.with(post_type_id.eq(1)).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = drain((&s).filt(|a| a[0] > 5 && a[1] - a[2] > 10).and(owner_user).and((&by_post).map(|(_, r)| r)));
    let v = top_n(v, |&(p, ((_, u), r))| (Reverse(db.user.reputation.get(u).unwrap()), r, p), 50);
    rows(v.into_iter().map(|(p, ((a, u), r))| {
        let rep = db.user.reputation.get(u).unwrap();
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title", "created"]));
        f.extend(a.map(V::I));
        f.extend([V::I(rep), V::S(if rep >= 1000 { "Experienced" } else if rep >= 100 { "Novice" } else { "Beginner" }), V::I(r)]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT Id AS UserId, Reputation, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM Users),
// PopularPosts AS (SELECT p.Id AS PostId, p.Title, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, p.CreationDate FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate HAVING COUNT(c.Id) > 5),
// TopUsers AS (SELECT ur.UserId, SUM(CASE WHEN pp.CommentCount > 10 THEN 1 ELSE 0 END) AS ActivePostCount, AVG(ur.Reputation) AS AvgReputation
//     FROM UserReputation ur LEFT JOIN PopularPosts pp ON ur.UserId = pp.PostId GROUP BY ur.UserId),
// RankedUsers AS (SELECT UserId, ActivePostCount, AvgReputation, RANK() OVER (ORDER BY AvgReputation DESC, ActivePostCount DESC) AS UserRank FROM TopUsers)
// SELECT ru.UserId, u.DisplayName, ru.ActivePostCount, ru.AvgReputation, CASE WHEN ru.ActivePostCount >= 5 THEN 'High Activity' ELSE 'Low Activity' END AS ActivityLevel
// FROM RankedUsers ru JOIN Users u ON ru.UserId = u.Id WHERE ru.UserRank <= 10 ORDER BY ru.AvgReputation DESC, ru.ActivePostCount DESC;
fn q3684(db: &'static So) -> String {
    let pc = db
        .post
        .with((&db.post.post_type_id).eq(1))
        .group_by(&db.post.origid)
        .select(comments_of(db).opt().and(votes_of(db).opt()))
        .fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let pp = (&pc).filt(|n| n > 5);
    let tu = db.user.group_by(Ident::<User>::new()).select((&db.user.origid).select(&pp).opt()).fold(0i64, |a, n| a + n.map_or(0, |n| (n > 10) as i64));
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&db.user.reputation).and(&tu)).window(rank, |((_, r), a)| (Reverse(r), Reverse(a)), asc);
    rows(drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|x| x.1 .0).map(|((u, rep), a)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a), V::F(rep as f64),V::S(if a >= 5 { "High Activity" } else { "Low Activity" })]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS UpvotedPosts,
//        SUM(CASE WHEN p.Score < 0 THEN 1 ELSE 0 END) AS DownvotedPosts, AVG(p.ViewCount) AS AverageViews, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT ..., RANK() OVER (ORDER BY TotalPosts DESC) AS PostRank FROM UserPostStats)
// SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, UpvotedPosts, DownvotedPosts, AverageViews, GoldBadges, SilverBadges, BronzeBadges
// FROM TopUsers WHERE PostRank <= 10 ORDER BY PostRank;
fn q5553(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(view_count.opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 10], |a, (p, b)| {
            let (t, s, w) = p.map_or((None, None, None), |((t, s), w)| (Some(t), Some(s), w));
            [
                a[0] + t.is_some() as i64,
                a[1] + (t == Some(1)) as i64,
                a[2] + (t == Some(2)) as i64,
                a[3] + s.map_or(0, |s| (s > 0) as i64),
                a[4] + s.map_or(0, |s| (s < 0) as i64),
                a[5] + w.is_some() as i64,
                a[6] + w.unwrap_or(0),
                a[7] + (b == Some(1)) as i64,
                a[8] + (b == Some(2)) as i64,
                a[9] + (b == Some(3)) as i64,
            ]
        });
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&s)).window(rank, |(_, a)| Reverse(a[0]), asc);
    rows(drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|x| x.1 .0).map(|(u, a)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4]), avg(a[6], a[5]), V::I(a[7]), V::I(a[8]), V::I(a[9])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.Tags, p.Score, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.Tags ORDER BY p.Score DESC) AS Rank,
//        u.DisplayName AS OwnerDisplayName, u.Reputation AS OwnerReputation, COALESCE(ah.AcceptedAnswerId, 0) AS AcceptedAnswerId
//     FROM Posts p INNER JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Posts ah ON p.Id = ah.AcceptedAnswerId WHERE p.PostTypeId = 1 AND p.Score > 0),
// TagStatistics AS (SELECT t.TagName, COUNT(rp.PostId) AS PostCount, SUM(rp.Score) AS TotalScore, AVG(rp.Score) AS AverageScore
//     FROM RankedPosts rp INNER JOIN Tags t ON rp.Tags LIKE '%' || t.TagName || '%' GROUP BY t.TagName),
// TopTags AS (SELECT TagName, PostCount, TotalScore, AverageScore, RANK() OVER (ORDER BY TotalScore DESC) AS TagRank FROM TagStatistics)
// SELECT tt.TagName, tt.PostCount, tt.TotalScore, tt.AverageScore, rp.OwnerDisplayName, rp.OwnerReputation, rp.Title AS PostTitle, rp.PostId AS QuestionId, rp.AcceptedAnswerId
// FROM TopTags tt JOIN RankedPosts rp ON rp.Tags LIKE '%' || tt.TagName || '%' WHERE tt.TagRank <= 5 ORDER BY tt.TotalScore DESC, rp.Score DESC;
fn q27312(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, origid, accepted_answer_id, .. } = &db.post;
    let acc: HashIdx<i64, Id<Post>> = accepted_answer_id.inv().collect();
    let rp: MatSet<(Id<Post>, Option<Id<Post>>)> = db.post.with(post_type_id.eq(1).and(score.gt(0))).with(owner_user).select(Ident::<Post>::new().and(origid.select(&acc).opt())).collect();
    let lt = tag_mentions(db);
    let rp_of: HashIdx<Id<Post>, (Id<Post>, Option<Id<Post>>)> = (&rp).map(|(p, _)| p).inv().collect();
    type RP = (Id<Post>, Option<Id<Post>>);
    type PT = (Id<Post>, Id<Tag>);
    type NR = (Str, RP);
    let pairs: MatSet<NR> = (&lt).select(Same::<PT>::new().map(|(_, t): PT| t).select(&db.tag.tag_name).and(Same::<PT>::new().map(|(p, _): PT| p).select(&rp_of))).collect();
    let ts = (&pairs).group_by(Same::<NR>::new().map(|(n, _): NR| n)).select(Same::<NR>::new().map(|(_, (p, _)): NR| p).select(score)).fold((0i64, 0i64), |(n, s), x| (n + 1, s + x));
    let w = whole(&ts).select(Same::<Str>::new().and(&ts)).window(rank, |(_, (_, s))| Reverse(s), asc);
    type T = (Str, (i64, i64));
    let tt: MatSet<T> = (&w).filt(|(_, r)| r <= 5).map(|(x, _)| x).collect();
    let rows_of_name: HashIdx<Str, RP> = (&pairs).map(|(n, _)| n).inv().select(&pairs).map(|(_, r)| r).collect();
    let mut v = drain((&tt).select(Same::<T>::new().and(Same::<T>::new().map(|(n, _): T| n).select(&rows_of_name))));
    v.sort_by_key(|&(_, ((_, (_, s)), (p, _)))| (Reverse(s), Reverse(score.get(p).unwrap())));
    rows(v.into_iter().map(|(_, ((t, (n, s)), (p, a)))| {
        let mut f = vec![V::S(t), V::I(n), V::I(s), avg(s, n)];
        f.extend(post_fields(db, p, &["owner", "rep", "title", "id"]));
        f.push(V::I(a.map_or(0, |a| accepted_answer_id.get(a).unwrap())));
        row(f)
    }))
}

// WITH RECURSIVE UserBadgeCounts AS (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId),
// PostRanks AS (SELECT P.Id AS PostId, P.Title, P.OwnerUserId, RANK() OVER (PARTITION BY P.OwnerUserId ORDER BY P.Score DESC) AS ScoreRank FROM Posts P WHERE P.PostTypeId = 1),
// CloseReasons AS (SELECT PH.PostId, MAX(PH.CreationDate) AS LastClosedDate, CR.Name AS CloseReason FROM PostHistory PH JOIN CloseReasonTypes CR ON CAST(PH.Comment AS INTEGER) = CR.Id
//     WHERE PH.PostHistoryTypeId = 10 GROUP BY PH.PostId, CR.Name),
// UserReputation AS (SELECT U.Id AS UserId, U.Reputation, COALESCE(UB.BadgeCount, 0) AS BadgeCount FROM Users U LEFT JOIN UserBadgeCounts UB ON U.Id = UB.UserId)
// SELECT U.DisplayName, U.Reputation, UR.BadgeCount, P.Title, P.ViewCount, PR.ScoreRank, COALESCE(CR.CloseReason, 'Not Closed') AS CloseReason, CR.LastClosedDate
// FROM Users U JOIN UserReputation UR ON U.Id = UR.UserId JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN PostRanks PR ON P.Id = PR.PostId LEFT JOIN CloseReasons CR ON P.Id = CR.PostId
// WHERE UR.Reputation > 1000 AND UR.BadgeCount > 0 ORDER BY UR.Reputation DESC, PR.ScoreRank LIMIT 10;
//
// No CTE refers to itself, so RECURSIVE changes nothing.
fn q30080(db: &'static So) -> String {
    let Post { post_type_id, owner_user_id, score, .. } = &db.post;
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let w = db.post.with(post_type_id.eq(1)).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(score)).window(rank, |(_, s)| Reverse(s), asc);
    let pr: MatSet<(Id<Post>, i64)> = (&w).map(|((p, _), r)| (p, r)).collect();
    let rank_of: HashIdx<Id<Post>, (Id<Post>, i64)> = (&pr).map(|(p, _)| p).inv().select(&pr).collect();
    let PostHistory { post_history_type_id, comment, post, creation_date: hd, .. } = &db.post_history;
    let crids: HashIdx<i64, Id<CloseReasonType>> = (&db.close_reason_type.origid).inv().collect();
    let cr = db
        .post_history
        .with(post_history_type_id.eq(10))
        .group_by(post.and(comment.map(|c: Str| c.trim().parse::<i64>().unwrap()).select(&crids).select(&db.close_reason_type.name)))
        .select(hd)
        .fold(i64::MIN, |m, d| m.max(d));
    let cr_rows = rel(drain(&cr));
    let cr_of: HashIdx<Id<Post>, ((Id<Post>, Str), i64)> = (&cr_rows).map(|((p, _), _)| p).inv().select(&cr_rows).collect();
    let users = db.user.with((&db.user.reputation).gt(1000)).with((&bc).filt(|n| n > 0));
    let v = drain(users.select(posts_of(db).select(Ident::<Post>::new().and((&rank_of).map(|(_, r)| r).opt()).and((&cr_of).opt()))));
    let v = top_n(v, |&(u, ((p, r), _))| (Reverse(db.user.reputation.get(u).unwrap()), r.is_none(), r, p), 10);
    rows(v.into_iter().map(|(u, ((p, r), c))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(bc.get(u).unwrap()));
        f.extend(post_fields(db, p, &["title", "views"]));
        f.push(r.map_or(V::Null, V::I));
        f.extend(match c {
            Some(((_, n), d)) => [V::S(n), V::T(d)],
            None => [V::S("Not Closed"), V::Null],
        });
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COALESCE(BadgeCount.BadgeCount, 0) AS BadgeCount, COALESCE(PostCount.PostCount, 0) AS PostCount,
//        COALESCE(CommentCount.CommentCount, 0) AS CommentCount, COALESCE(VoteCount.VoteCount, 0) AS VoteCount
//     FROM Users U LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId) AS BadgeCount ON U.Id = BadgeCount.UserId
//     LEFT JOIN (SELECT OwnerUserId, COUNT(*) AS PostCount FROM Posts GROUP BY OwnerUserId) AS PostCount ON U.Id = PostCount.OwnerUserId
//     LEFT JOIN (SELECT UserId, COUNT(*) AS CommentCount FROM Comments GROUP BY UserId) AS CommentCount ON U.Id = CommentCount.UserId
//     LEFT JOIN (SELECT UserId, COUNT(*) AS VoteCount FROM Votes GROUP BY UserId) AS VoteCount ON U.Id = VoteCount.UserId),
// RankedUsers AS (SELECT UserId, DisplayName, Reputation, BadgeCount, PostCount, CommentCount, VoteCount, RANK() OVER (ORDER BY Reputation DESC, BadgeCount DESC, PostCount DESC) AS Rank FROM UserReputation)
// SELECT R.UserId, R.DisplayName, R.Reputation, R.BadgeCount, R.PostCount, R.CommentCount, R.VoteCount, R.Rank FROM RankedUsers R WHERE R.Rank <= 10 ORDER BY R.Rank;
fn q8442(db: &'static So) -> String {
    let s = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let p = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let c = db.user.group_by(Ident::<User>::new()).select(comments_by(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = db.user.group_by(Ident::<User>::new()).select(votes_by(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let w = whole(db.user.iq())
        .select(Ident::<User>::new().and(&db.user.reputation).and(&s).and(&p).and(&c).and(&v))
        .window(rank, |(((((_, r), b), n), _), _)| (Reverse(r), Reverse(b), Reverse(n)), asc);
    let mut out: Vec<_> = drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|x| x.1).collect();
    out.sort_by_key(|&(_, r)| r);
    rows(out.into_iter().map(|((((((u, _), b), n), c), v), r)| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(b), V::I(n), V::I(c), V::I(v), V::I(r)]);
        row(f)
    }))
}

// WITH UserBadges AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS TotalBadges, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// PostStats AS (SELECT P.OwnerUserId, COUNT(P.Id) AS TotalPosts, COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS Questions, COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS Answers,
//        SUM(P.ViewCount) AS TotalViews, SUM(P.Score) AS TotalScore FROM Posts P GROUP BY P.OwnerUserId),
// UserPerformance AS (SELECT U.Id AS UserId, U.DisplayName, UB.TotalBadges, PS.TotalPosts, PS.Questions, PS.Answers, PS.TotalViews, PS.TotalScore
//     FROM Users U JOIN UserBadges UB ON U.Id = UB.UserId LEFT JOIN PostStats PS ON U.Id = PS.OwnerUserId)
// SELECT UP.UserId, UP.DisplayName, UP.TotalBadges, UP.TotalPosts, UP.Questions, UP.Answers, UP.TotalViews, UP.TotalScore, RANK() OVER (ORDER BY UP.TotalScore DESC) AS ScoreRank
// FROM UserPerformance UP WHERE UP.TotalPosts > 0 ORDER BY UP.TotalScore DESC LIMIT 10;
fn q9520(db: &'static So) -> String {
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let up = user_posts(db);
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&bc).and((&up).filt(|a| a[1] > 0))).window(rank, |(_, a)| Reverse(a[4]), asc);
    let v = top_n(drain(&w).into_iter().map(|x| x.1).collect(), |&(_, r)| r, 10);
    rows(v.into_iter().map(|(((u, b), a), r)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(b), V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[6], a[5]), V::I(a[4]), V::I(r)]);
        row(f)
    }))
}

// WITH UserBadgeCounts AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) FILTER (WHERE B.Class = 1) AS GoldBadges, COUNT(B.Id) FILTER (WHERE B.Class = 2) AS SilverBadges,
//        COUNT(B.Id) FILTER (WHERE B.Class = 3) AS BronzeBadges, SUM(COALESCE(U.UpVotes, 0) - COALESCE(U.DownVotes, 0)) AS NetVotes
//     FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// PopularPosts AS (SELECT P.Id AS PostId, P.OwnerUserId, P.Title, P.CreationDate, P.Score, P.ViewCount,
//        ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.Score DESC, P.ViewCount DESC) AS PopularRank FROM Posts P WHERE P.PostTypeId = 1 AND P.Score > 0),
// UsersWithPopularPosts AS (SELECT U.UserId, U.DisplayName, P.PostId, P.Title, P.Score FROM UserBadgeCounts U JOIN PopularPosts P ON U.UserId = P.OwnerUserId WHERE P.PopularRank <= 3)
// SELECT U.DisplayName, COALESCE(P.Title, 'No Popular Posts') AS PopularPostTitle, COALESCE(P.Score, 0) AS PostScore, U.GoldBadges, U.SilverBadges, U.BronzeBadges, U.NetVotes
// FROM UserBadgeCounts U LEFT JOIN UsersWithPopularPosts P ON U.UserId = P.UserId ORDER BY U.NetVotes DESC, U.GoldBadges DESC, U.SilverBadges DESC, U.BronzeBadges DESC LIMIT 10;
fn q194(db: &'static So) -> String {
    let User { up_votes, down_votes, .. } = &db.user;
    let ub = db
        .user
        .group_by(Ident::<User>::new())
        .select(up_votes.and(down_votes).and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 4], |a, ((u, d), c)| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64, a[3] + u - d]);
    let Post { post_type_id, score, owner_user, view_count, .. } = &db.post;
    let key = |p: Id<Post>| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    };
    let w = db
        .post
        .with(post_type_id.eq(1).and(score.gt(0)))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(score.and(view_count.opt())))
        .window(row_number, |(p, (s, w))| (Reverse(s), w.is_none(), Reverse(w), p), asc);
    let pp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 3).map(|((p, _), _)| p).collect();
    let pp_of: HashIdx<Id<User>, Id<Post>> = (&pp).select(owner_user).inv().collect();
    let v = drain((&ub).and((&pp_of).opt()));
    let v = top_n(v, |&(u, (a, p))| (Reverse(a[3]), Reverse(a[0]), Reverse(a[1]), Reverse(a[2]), u, p.map(key)), 10);
    rows(v.into_iter().map(|(u, (a, p))| {
        let mut f = vec![user_col(db, u, "name")];
        match p {
            Some(p) => f.extend(post_fields(db, p, &["title", "score"])),
            None => f.extend([V::S("No Popular Posts"), V::I(0)]),
        }
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, COUNT(a.Id) AS AnswerCount, DENSE_RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank
//     FROM Posts p LEFT JOIN Posts a ON p.Id = a.ParentId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounties FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId AND v.VoteTypeId = 8 GROUP BY u.Id, u.Reputation),
// ActiveUsers AS (SELECT u.Id, u.DisplayName, ur.Reputation, ur.TotalBounties, RANK() OVER (ORDER BY ur.Reputation + ur.TotalBounties DESC) AS UserRanking
//     FROM Users u JOIN UserReputation ur ON u.Id = ur.UserId WHERE u.Reputation > 100)
// SELECT rp.Title, rp.CreationDate, up.DisplayName AS UserDisplayName, up.Reputation, up.TotalBounties, rp.AnswerCount, (SELECT COUNT(*) FROM Comments c WHERE c.PostId = rp.PostId) AS CommentCount
// FROM RankedPosts rp JOIN ActiveUsers up ON rp.OwnerUserId = up.Id WHERE rp.UserPostRank <= 5 AND rp.AnswerCount > 0 ORDER BY rp.CreationDate DESC OFFSET 10 ROWS FETCH NEXT 10 ROWS ONLY;
fn q3074(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let w = db
        .post
        .with(post_type_id.eq(1))
        .group_by(owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(100))))
        .select(Ident::<Post>::new().and(creation_date))
        .window(dense_rank, |(_, d)| Reverse(d), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let ac = (&tp).group_by(Ident::<Post>::new()).select(children_of(db).opt()).fold(0i64, |n, a| n + a.is_some() as i64);
    let bt = db.user.group_by(Ident::<User>::new()).select(votes_by(db).with((&db.vote.vote_type_id).eq(8)).select((&db.vote.bounty_amount).opt()).opt()).fold(0i64, |s, b| s + b.flatten().unwrap_or(0));
    let cc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let mut v = drain((&ac).filt(|n| n > 0).and(owner_user.select(Ident::<User>::new().and(&bt))).and((&cc).opt()));
    v.sort_by_key(|&(p, _)| Reverse(creation_date.get(p).unwrap()));
    rows(v.into_iter().skip(10).take(10).map(|(p, ((a, (u, b)), c))| {
        let mut f = post_fields(db, p, &["title", "created"]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::I(b), V::I(a), V::I(c.unwrap_or(0))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, COUNT(a.Id) AS AnswerCount,
//        ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Posts a ON a.ParentId = p.Id AND a.PostTypeId = 2 WHERE p.PostTypeId = 1
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, OwnerDisplayName, AnswerCount FROM RankedPosts WHERE rn = 1),
// UserParticipation AS (SELECT p.Id AS PostId, COUNT(DISTINCT c.Id) AS CommentCount, SUM(v.BountyAmount) AS TotalBounty FROM Posts p LEFT JOIN Comments c ON c.PostId = p.Id
//     LEFT JOIN Votes v ON v.PostId = p.Id WHERE p.PostTypeId = 1 GROUP BY p.Id)
// SELECT tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.OwnerDisplayName, tp.AnswerCount, up.CommentCount, up.TotalBounty
// FROM TopPosts tp JOIN UserParticipation up ON tp.PostId = up.PostId ORDER BY tp.Score DESC, up.CommentCount DESC, tp.ViewCount DESC LIMIT 100;
fn q9075(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let qs = || db.post.with(post_type_id.eq(1)).group_by(Ident::<Post>::new());
    let dc = qs().select(comments_of(db)).count_distinct();
    let tb = qs().select(comments_of(db).opt().and(votes_of(db).select((&db.vote.bounty_amount).opt()).opt())).fold(None, |s: Option<i64>, (_, b)| match b.flatten() {
        Some(b) => Some(s.unwrap_or(0) + b),
        None => s,
    });
    let v = top_n(drain((&tb).and((&dc).opt())).into_iter().map(|(p, (b, c))| (p, (c.unwrap_or(0), b))).collect(), |&(p, (c, _))| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), Reverse(c), w.is_none(), Reverse(w), p)
    }, 100);
    let tp: MatSet<Id<Post>> = rel(v.iter().map(|x| x.0).collect()).map(|p| p).collect();
    let ac = (&tp).group_by(Ident::<Post>::new()).select(answers_of(db).opt()).fold(0i64, |n, a| n + a.is_some() as i64);
    rows(v.into_iter().map(|(p, (c, b))| {
        let mut f = post_fields(db, p, &["title", "created", "score", "views", "owner"]);
        f.extend([V::I(ac.get(p).unwrap()), V::I(c), b.map_or(V::Null, V::I)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.AnswerCount, p.CommentCount, p.ViewCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year' AND p.Score > 10),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId WHERE u.Reputation > 1000
//     GROUP BY u.Id, u.DisplayName, u.Reputation HAVING COUNT(b.Id) > 0),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS TotalComments, MAX(c.CreationDate) AS LastCommentDate FROM Comments c GROUP BY c.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.AnswerCount, rp.CommentCount, rp.ViewCount, tu.UserId, tu.DisplayName, tu.Reputation, tu.BadgeCount, pc.TotalComments, pc.LastCommentDate
// FROM RankedPosts rp JOIN TopUsers tu ON rp.PostId IN (SELECT pl.RelatedPostId FROM PostLinks pl WHERE pl.PostId = rp.PostId)
// LEFT JOIN PostComments pc ON rp.PostId = pc.PostId WHERE rp.Rank <= 5 ORDER BY rp.PostId, rp.Score DESC;
//
// The join holds only for a top post that links to itself, and none does.
fn q8529(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, origid, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(score.gt(10)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|(((p, _), _), _)| p).collect();
    let selfish = links_of(db).select(&db.post_link.related_post).and(Ident::<Post>::new()).filt(|(r, p)| r == p);
    let bc = db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new()).select(badges_of(db)).fold(0i64, |n, _| n + 1);
    let pc = db.comment.group_by(&db.comment.post).select(&db.comment.creation_date).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let mut v = drain((&tp).with(selfish).select((&pc).opt()).cross(&bc));
    v.sort_by_key(|&((p, _), _)| (origid.get(p).unwrap(), Reverse(score.get(p).unwrap())));
    rows(v.into_iter().map(|((p, u), (c, b))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "answers", "comments", "views"]);
        f.extend(ucols(db, u, &["uid", "name", "rep"]));
        f.push(V::I(b));
        f.extend(match c {
            Some((n, m)) => [V::I(n), V::T(m)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, u.DisplayName AS OwnerDisplayName, p.CreationDate, p.Score, p.ViewCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS rn FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= CURRENT_DATE - INTERVAL '1 year'),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.OwnerDisplayName, rp.CreationDate, rp.Score, rp.ViewCount FROM RankedPosts rp WHERE rp.rn <= 10),
// PostWithComments AS (SELECT tp.PostId, tp.Title, tp.OwnerDisplayName, tp.CreationDate, tp.Score, tp.ViewCount, COUNT(c.Id) AS CommentCount FROM TopPosts tp
//     LEFT JOIN Comments c ON tp.PostId = c.PostId GROUP BY tp.PostId, tp.Title, tp.OwnerDisplayName, tp.CreationDate, tp.Score, tp.ViewCount),
// FinalResults AS (SELECT pwc.*, RANK() OVER (ORDER BY pwc.Score DESC, pwc.ViewCount DESC) AS PopularityRank FROM PostWithComments pwc)
// SELECT f.PostId, f.Title, f.OwnerDisplayName, f.CreationDate, f.Score, f.ViewCount, f.CommentCount, f.PopularityRank FROM FinalResults f WHERE f.PopularityRank <= 5 ORDER BY f.PopularityRank;
//
// CURRENT_DATE is the day the query runs; the data ends in 2024, so it is empty.
fn q5516(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, view_count, .. } = &db.post;
    let w = db
        .post
        .with(post_type_id.eq(1).and(creation_date.ge(add_years(current_date(), -1))))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(score))
        .window(row_number, |(p, s)| (Reverse(s), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|((p, _), _)| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let w2 = whole(&cc).select(Ident::<Post>::new().and(&cc).and(score.and(view_count.opt()))).window(rank, |(_, (s, w))| (Reverse(s), w.is_none(), Reverse(w)), asc);
    let mut v: Vec<_> = drain((&w2).filt(|(_, r)| r <= 5)).into_iter().map(|x| x.1).collect();
    v.sort_by_key(|&(_, r)| r);
    rows(v.into_iter().map(|(((p, n), _), r)| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "score", "views"]);
        f.extend([V::I(n), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS RowNum FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName, p.PostTypeId),
// FilteredComments AS (SELECT PostId, COUNT(*) AS TotalComments, AVG(Score) AS AverageCommentScore FROM Comments GROUP BY PostId),
// OuterJoinResults AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.OwnerDisplayName, COALESCE(fc.TotalComments, 0) AS TotalComments,
//        COALESCE(fc.AverageCommentScore, 0) AS AverageCommentScore FROM RankedPosts rp LEFT JOIN FilteredComments fc ON rp.PostId = fc.PostId)
// SELECT ojr.PostId, ojr.Title, ojr.CreationDate, ojr.Score, ojr.ViewCount, ojr.OwnerDisplayName, ojr.TotalComments, ojr.AverageCommentScore
// FROM OuterJoinResults ojr WHERE ojr.TotalComments > 5 OR (ojr.AverageCommentScore > 1 AND ojr.Score > 100) ORDER BY ojr.Score DESC, ojr.ViewCount DESC FETCH FIRST 10 ROWS ONLY;
fn q2442(db: &'static So) -> String {
    let Post { score, view_count, .. } = &db.post;
    let fc = db.comment.group_by(&db.comment.post).select(&db.comment.score).fold((0i64, 0i64), |(n, s), x| (n + 1, s + x));
    let v = drain(db.post.select(score.and((&fc).opt())).filt(|(sc, c)| {
        let (n, s) = c.unwrap_or((0, 0));
        n > 5 || (n > 0 && s > n && sc > 100)
    }));
    let v = top_n(v, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 10);
    rows(v.into_iter().map(|(p, (_, c))| {
        let (n, s) = c.unwrap_or((0, 0));
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend([V::I(n), if n == 0 { V::F(0.0) } else { V::F(s as f64 / n as f64) }]);
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN p.PostTypeId = 1 THEN p.AnswerCount ELSE 0 END) AS TotalAnswers,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT ..., RANK() OVER (ORDER BY Reputation DESC, PostCount DESC) AS Rank FROM UserStatistics),
// MostActiveUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, QuestionCount, AnswerCount, TotalAnswers, UpVotes, DownVotes FROM TopUsers WHERE Rank <= 10)
// SELECT mau.DisplayName, mau.Reputation, mau.PostCount, mau.QuestionCount, mau.AnswerCount, mau.TotalAnswers, (mau.UpVotes - mau.DownVotes) AS NetVoteScore
// FROM MostActiveUsers mau ORDER BY NetVoteScore DESC;
fn q7384(db: &'static So) -> String {
    let Post { post_type_id, answer_count, .. } = &db.post;
    let s = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(answer_count.opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold(([0i64; 5], None), |(a, t): ([i64; 5], Option<i64>), p| match p {
            Some(((ty, n), v)) => {
                let x = if ty == 1 { n } else { Some(0) };
                ([a[0] + 1, a[1] + (ty == 1) as i64, a[2] + (ty == 2) as i64, a[3] + (v == Some(2)) as i64, a[4] + (v == Some(3)) as i64], match x {
                    Some(x) => Some(t.unwrap_or(0) + x),
                    None => t,
                })
            }
            None => (a, Some(t.unwrap_or(0))),
        });
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&db.user.reputation).and(&s)).window(rank, |((_, r), (a, _))| (Reverse(r), Reverse(a[0])), asc);
    let mut v: Vec<_> = drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|x| x.1 .0).collect();
    v.sort_by_key(|&(_, (a, _))| Reverse(a[3] - a[4]));
    rows(v.into_iter().map(|((u, _), (a, t))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), t.map_or(V::Null, V::I), V::I(a[3] - a[4])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank FROM Posts p WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '1 year'),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN p.ViewCount > 100 THEN 1 ELSE 0 END) AS HighViewPosts,
//        SUM(CASE WHEN p.AnswerCount > 0 THEN 1 ELSE 0 END) AS AnsweredQuestions FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT us.UserId, us.DisplayName, us.PostCount, us.HighViewPosts, us.AnsweredQuestions, RANK() OVER (ORDER BY us.HighViewPosts DESC, us.AnsweredQuestions DESC) AS UserRank FROM UserStats us)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, rp.CommentCount, tu.DisplayName AS TopUser, tu.PostCount AS UserPostCount,
//        tu.HighViewPosts AS UserHighViewPosts, tu.AnsweredQuestions AS UserAnsweredQuestions
// FROM RankedPosts rp JOIN TopUsers tu ON rp.PostId = (SELECT p.Id FROM Posts p WHERE p.OwnerUserId = tu.UserId ORDER BY p.Score DESC LIMIT 1)
// WHERE rp.Rank <= 10 ORDER BY rp.Score DESC, rp.ViewCount DESC;
//
// CURRENT_DATE is the day the query runs; the data ends in 2024, so it is empty.
fn q8371(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, view_count, owner_user, answer_count, .. } = &db.post;
    let key = |p: Id<Post>| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    };
    let w = db
        .post
        .with(creation_date.ge(add_years(current_date(), -1)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score.and(view_count.opt())))
        .window(row_number, |(p, (s, w))| (Reverse(s), w.is_none(), Reverse(w), p), asc);
    let rp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|((p, _), _)| p).collect();
    let bw = db.post.group_by(owner_user).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let best: MatSet<Id<Post>> = (&bw).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(view_count.opt().and(answer_count.opt())))
        .fold([0i64; 3], |a, (w, n)| [a[0] + 1, a[1] + w.map_or(0, |w| (w > 100) as i64), a[2] + n.map_or(0, |n| (n > 0) as i64)]);
    let mut v = drain((&rp).select(Ident::<Post>::new().with(&best).and(owner_user.select(Ident::<User>::new().and(&us)))));
    v.sort_by_key(|&(_, (p, _))| key(p));
    rows(v.into_iter().map(|(_, (p, (u, a)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments"]);
        f.push(user_col(db, u, "name"));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(p.ViewCount) AS TotalViews, SUM(p.Score) AS TotalScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// HighScoringUsers AS (SELECT ..., RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank FROM UserPostStats WHERE TotalPosts > 5),
// TopTags AS (SELECT t.TagName, COUNT(p.Id) AS TagPostCount, SUM(p.ViewCount) AS TagTotalViews FROM Tags t JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%'
//     WHERE p.PostTypeId = 1 GROUP BY t.TagName ORDER BY TagPostCount DESC LIMIT 10)
// SELECT u.DisplayName, u.TotalPosts, u.TotalAnswers, u.TotalViews, u.TotalScore, t.TagName, t.TagPostCount, t.TagTotalViews
// FROM HighScoringUsers u JOIN TopTags t ON u.UserId IN (SELECT OwnerUserId FROM Posts WHERE Tags LIKE '%' || t.TagName || '%') ORDER BY u.TotalScore DESC, t.TagPostCount DESC;
fn q8086(db: &'static So) -> String {
    let lt = tag_mentions(db);
    let Post { post_type_id, view_count, owner_user, .. } = &db.post;
    type P = (Id<Post>, Id<Tag>);
    type PN = (Id<Post>, Str);
    let pn: MatSet<PN> = (&lt).select(Same::<P>::new().map(|(p, _): P| p).and(Same::<P>::new().map(|(_, t): P| t).select(&db.tag.tag_name))).collect();
    let tq = (&pn)
        .group_by(Same::<PN>::new().map(|(_, n): PN| n))
        .select(Same::<PN>::new().map(|(p, _): PN| p).select(Ident::<Post>::new().with(post_type_id.eq(1))).select(view_count.opt()))
        .fold((0i64, 0i64, 0i64), |(n, c, s), w| (n + 1, c + w.is_some() as i64, s + w.unwrap_or(0)));
    let tt = top_n(drain(&tq), |&(t, (n, _, _))| (Reverse(n), t), 10);
    let tset: MatSet<Str> = rel(tt.iter().map(|x| x.0).collect()).map(|t| t).collect();
    let ut: MatSet<(Id<User>, Str)> = (&pn).select(Same::<PN>::new().map(|(p, _): PN| p).select(owner_user).and(Same::<PN>::new().map(|(_, n): PN| n))).collect();
    let ups = user_posts(db);
    type Q = (Id<User>, Str);
    let hs = Ident::<User>::new().with((&ups).filt(|a| a[1] > 5));
    let mut v = drain(
        (&ut)
            .with(Same::<Q>::new().map(|(_, t): Q| t).select(Same::<Str>::new().with(&tset)))
            .with(Same::<Q>::new().map(|(u, _): Q| u).select(hs))
            .select(Same::<Q>::new().map(|(u, _): Q| u).select(&ups).and(Same::<Q>::new().map(|(_, t): Q| t).select(&tq))),
    );
    v.sort_by_key(|&(_, (a, (n, _, _)))| (Reverse(a[4]), Reverse(n)));
    rows(v.into_iter().map(|((u, t), (a, (n, c, s)))| {
        row(vec![user_col(db, u, "name"), V::I(a[1]), V::I(a[3]), nullable(a[6], a[5]), V::I(a[4]), V::S(t), V::I(n), nullable(s, c)])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, p.CommentCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days'),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.AnswerCount, rp.CommentCount, rp.OwnerDisplayName FROM RankedPosts rp WHERE rp.Rank <= 5),
// HighlightedPosts AS (SELECT tp.PostId, tp.Title, tp.CreationDate, tp.ViewCount, tp.Score, tp.AnswerCount, tp.CommentCount, tp.OwnerDisplayName, COUNT(c.Id) AS CommentTotal
//     FROM TopPosts tp LEFT JOIN Comments c ON tp.PostId = c.PostId GROUP BY tp.PostId, tp.Title, tp.CreationDate, tp.ViewCount, tp.Score, tp.AnswerCount, tp.CommentCount, tp.OwnerDisplayName)
// SELECT hp.PostId, hp.Title, hp.CreationDate, hp.ViewCount, hp.Score, hp.AnswerCount, hp.CommentCount, hp.OwnerDisplayName, hp.CommentTotal FROM HighlightedPosts hp ORDER BY hp.Score DESC, hp.ViewCount DESC;
fn q8675(db: &'static So) -> String {
    let Post { creation_date, owner_user, post_type_id, score, view_count, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_days(date(2024, 10, 1), -30)))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|(((p, _), _), _)| p).collect();
    let mut v = drain((&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64));
    v.sort_by_key(|&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    });
    rows(v.into_iter().map(|(p, n)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "answers", "comments", "owner"]);
        f.push(V::I(n));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, COUNT(DISTINCT c.Id) AS CommentCount,
//        COUNT(DISTINCT a.Id) AS AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName, p.OwnerUserId),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.OwnerDisplayName, rp.CommentCount, rp.AnswerCount FROM RankedPosts rp WHERE rp.Rank <= 3),
// UserBadges AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount, MAX(b.Class) AS HighestBadge FROM Badges b GROUP BY b.UserId)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.Score, tp.ViewCount, tp.OwnerDisplayName, ub.BadgeCount, ub.HighestBadge
// FROM TopPosts tp LEFT JOIN UserBadges ub ON tp.OwnerDisplayName = (SELECT u.DisplayName FROM Users u WHERE u.Id = ub.UserId) ORDER BY tp.Score DESC, tp.CreationDate DESC;
fn q7086(db: &'static So) -> String {
    let Post { post_type_id, owner_user, owner_user_id, creation_date, score, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1)).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 3).map(|((p, _), _)| p).collect();
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold((0i64, i64::MIN), |(n, m), c| (n + 1, m.max(c)));
    let bu: MatSet<Id<User>> = db.badge.select(&db.badge.user).collect();
    let by_name: HashIdx<Str, Id<User>> = (&bu).select(&db.user.display_name).inv().collect();
    let mut v = drain((&tp).select(Ident::<Post>::new().and(owner_user.select(&db.user.display_name).select(&by_name).select(&ub).opt())));
    v.sort_by_key(|&(_, (p, _))| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())));
    rows(v.into_iter().map(|(_, (p, b))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner"]);
        f.extend(match b {
            Some((n, m)) => [V::I(n), V::I(m)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, SUBSTRING(p.Body, 1, 200) AS ShortBody, ARRAY_LENGTH(string_to_array(p.Tags, '>'), 1) AS TagCount, p.CreationDate,
//        u.DisplayName AS OwnerDisplayName, ROW_NUMBER() OVER(PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// PostStats AS (SELECT rp.OwnerDisplayName, COUNT(*) AS TotalPosts, COUNT(DISTINCT rp.PostId) AS UniquePosts, AVG(rp.TagCount) AS AvgTags, MAX(rp.CreationDate) AS LatestPostDate
//     FROM RankedPosts rp WHERE rp.Rank < 4 GROUP BY rp.OwnerDisplayName),
// TopUsers AS (SELECT ps.OwnerDisplayName, ps.TotalPosts, ps.UniquePosts, ps.AvgTags, RANK() OVER(ORDER BY ps.TotalPosts DESC, ps.UniquePosts DESC) AS UserRank FROM PostStats ps)
// SELECT tu.OwnerDisplayName, tu.TotalPosts, tu.UniquePosts, tu.AvgTags, tu.UserRank,
//        CASE WHEN tu.UserRank <= 10 THEN 'Top Contributor' WHEN tu.UserRank BETWEEN 11 AND 50 THEN 'Moderate Contributor' ELSE 'New Contributor' END AS ContributorLevel
// FROM TopUsers tu WHERE tu.UniquePosts >= 5 ORDER BY tu.UserRank;
fn q29683(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, tags_str, .. } = &db.post;
    let w = db
        .post
        .with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 3).map(|((p, _), _)| p).collect();
    let ps = (&tp)
        .group_by(owner_user.select(&db.user.display_name))
        .select(tags_str.map(|t: Str| t.matches('>').count() as i64 + 1).opt())
        .fold((0i64, 0i64, 0i64), |(n, c, s), t| (n + 1, c + t.is_some() as i64, s + t.unwrap_or(0)));
    let w = whole(&ps).select(Same::<Str>::new().and(&ps)).window(rank, |(_, (n, _, _))| Reverse(n), asc);
    rows(drain((&w).filt(|((_, (n, _, _)), _)| n >= 5)).into_iter().map(|x| x.1).map(|((name, (n, c, s)), r)| {
        let lvl = if r <= 10 { "Top Contributor" } else if r <= 50 { "Moderate Contributor" } else { "New Contributor" };
        row(vec![V::S(name), V::I(n), V::I(n), avg(s, c), V::I(r), V::S(lvl)])
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.Body, p.ViewCount, p.CreationDate, p.Tags, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.Tags ORDER BY p.ViewCount DESC) AS TagRank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Body, p.ViewCount, p.CreationDate, p.Tags, u.DisplayName),
// FilteredPosts AS (SELECT Id, Title, Body, ViewCount, CreationDate, Tags, OwnerDisplayName, CommentCount, UpVoteCount, DownVoteCount FROM RankedPosts
//     WHERE UpVoteCount > DownVoteCount AND CommentCount > 5),
// FinalRanking AS (SELECT *, RANK() OVER (ORDER BY ViewCount DESC, CreationDate DESC) AS PopularityRank FROM FilteredPosts)
// SELECT Id, Title, Body, ViewCount, CreationDate, Tags, OwnerDisplayName, CommentCount, UpVoteCount, DownVoteCount, PopularityRank FROM FinalRanking WHERE PopularityRank <= 10;
fn q25254(db: &'static So) -> String {
    let Post { post_type_id, view_count, creation_date, .. } = &db.post;
    let s = db
        .post
        .with(post_type_id.eq(1))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let fp = (&s).filt(|a| a[1] > a[2] && a[0] > 5);
    let w = whole(&fp).select(Ident::<Post>::new().and(&s).and(view_count.opt().and(creation_date))).window(rank, |(_, (w, d))| (w.is_none(), Reverse(w), Reverse(d)), asc);
    rows(drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|x| x.1).map(|(((p, a), _), r)| {
        let mut f = post_fields(db, p, &["id", "title", "body", "views", "created", "tags", "owner"]);
        f.extend(a.map(V::I));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, COUNT(DISTINCT a.Id) AS AnswerCount, COUNT(DISTINCT c.Id) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.Score DESC) AS Rank
//     FROM Posts p LEFT JOIN Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2 LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.Score),
// PostHistoryCTE AS (SELECT ph.PostId, COUNT(ph.Id) AS EditCount, MAX(ph.CreationDate) AS LastEditDate FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (4, 5, 6, 24) GROUP BY ph.PostId),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(v.BountyAmount) AS TotalBounties, RANK() OVER (ORDER BY SUM(v.BountyAmount) DESC) AS UserRank
//     FROM Users u JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName)
// SELECT rp.Id AS PostId, rp.Title, rp.CreationDate, rp.Score, rp.AnswerCount, rp.CommentCount, ph.EditCount, ph.LastEditDate, tu.UserId, tu.DisplayName AS TopUser, tu.TotalBounties
// FROM RankedPosts rp LEFT JOIN PostHistoryCTE ph ON rp.Id = ph.PostId LEFT JOIN TopUsers tu ON tu.UserRank = 1 WHERE rp.Rank = 1 ORDER BY rp.Score DESC LIMIT 10;
fn q34168(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let top = top_n(drain(db.post.with(post_type_id.eq(1)).select(score)), |&(p, s)| (Reverse(s), p), 10);
    let tp: MatSet<Id<Post>> = rel(top.iter().map(|x| x.0).collect()).map(|p| p).collect();
    let da = (&tp).group_by(Ident::<Post>::new()).select(answers_of(db)).count_distinct();
    let dc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db)).count_distinct();
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let ph = db.post_history.with(post_history_type_id.is_in([4, 5, 6, 24])).group_by(post).select(hd).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let tb = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select((&db.vote.bounty_amount).opt())).fold(None, |s: Option<i64>, b| match b {
        Some(b) => Some(s.unwrap_or(0) + b),
        None => s,
    });
    let tw = whole(&tb).select(Ident::<User>::new().and(&tb)).window(rank, |(_, b)| (b.is_none(), Reverse(b)), asc);
    let tu = (&tw).filt(|(_, r)| r == 1).map(|(x, _)| x);
    let rp = drain((&tp).select((&da).opt().and((&dc).opt()).and((&ph).opt()).and(Ident::<Post>::new().map(|_| ()).select(tu).opt())));
    let v = top_n(rp, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 10);
    rows(v.into_iter().map(|(p, (((a, c), h), t))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend([V::I(a.unwrap_or(0)), V::I(c.unwrap_or(0))]);
        f.extend(match h {
            Some((n, m)) => [V::I(n), V::T(m)],
            None => [V::Null, V::Null],
        });
        f.extend(match t {
            Some((u, b)) => [user_col(db, u, "uid"), user_col(db, u, "name"), b.map_or(V::Null, V::I)],
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.Tags, p.CreationDate, p.ViewCount, p.Score, p.OwnerUserId,
//        ROW_NUMBER() OVER (PARTITION BY p.Tags ORDER BY p.CreationDate DESC) AS TagRank FROM Posts p WHERE p.PostTypeId = 1),
// UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT a.Id) AS AnswerCount FROM Users u LEFT JOIN Posts a ON u.Id = a.OwnerUserId AND a.PostTypeId = 2
//     GROUP BY u.Id, u.DisplayName, u.Reputation),
// PostDetails AS (SELECT rp.PostId, rp.Title, rp.Body, rp.Tags, rp.CreationDate, rp.ViewCount, rp.Score, ur.UserId, ur.DisplayName AS OwnerDisplayName, ur.Reputation AS OwnerReputation, ur.AnswerCount
//     FROM RankedPosts rp JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId WHERE rp.TagRank <= 3)
// SELECT pd.Title, pd.Body, pd.Tags, pd.CreationDate, pd.ViewCount, pd.Score, pd.OwnerDisplayName, pd.OwnerReputation, pd.AnswerCount,
//        (SELECT COUNT(*) FROM Comments c WHERE c.PostId = pd.PostId) AS CommentCount, (SELECT COUNT(*) FROM Votes v WHERE v.PostId = pd.PostId AND v.VoteTypeId = 2) AS UpVoteCount
// FROM PostDetails pd ORDER BY pd.Score DESC, pd.CreationDate DESC;
fn q28980(db: &'static So) -> String {
    let Post { post_type_id, tags_str, creation_date, owner_user, score, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1)).group_by(tags_str.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 3).map(|((p, _), _)| p).collect();
    let ac = db.user.group_by(Ident::<User>::new()).select(posts_of(db).with(post_type_id.eq(2))).fold(0i64, |n, _| n + 1);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let uv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).with((&db.vote.vote_type_id).eq(2)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let mut v = drain((&cc).and(&uv).and(owner_user.select(Ident::<User>::new().and((&ac).opt()))));
    v.sort_by_key(|&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())));
    rows(v.into_iter().map(|(p, ((c, n), (u, a)))| {
        let mut f = post_fields(db, p, &["title", "body", "tags", "created", "views", "score"]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::I(a.unwrap_or(0)), V::I(c), V::I(n)]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(P.Score) AS TotalScore, SUM(P.ViewCount) AS TotalViews, SUM(P.CommentCount) AS TotalComments
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId WHERE U.Reputation > 100 GROUP BY U.Id, U.DisplayName),
// BadgeStats AS (SELECT UserId, COUNT(CASE WHEN Class = 1 THEN 1 END) AS GoldBadges, COUNT(CASE WHEN Class = 2 THEN 1 END) AS SilverBadges,
//        COUNT(CASE WHEN Class = 3 THEN 1 END) AS BronzeBadges FROM Badges GROUP BY UserId),
// FinalStats AS (SELECT UPS.UserId, UPS.DisplayName, UPS.TotalPosts, UPS.TotalQuestions, UPS.TotalAnswers, UPS.TotalScore, UPS.TotalViews, UPS.TotalComments,
//        BS.GoldBadges, BS.SilverBadges, BS.BronzeBadges FROM UserPostStats UPS LEFT JOIN BadgeStats BS ON UPS.UserId = BS.UserId)
// SELECT UserId, DisplayName, TotalPosts, TotalQuestions, TotalAnswers, TotalScore, TotalViews, TotalComments, GoldBadges, SilverBadges, BronzeBadges,
//        RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank
// FROM FinalStats WHERE TotalPosts > 10 ORDER BY TotalScore DESC, TotalPosts DESC LIMIT 50;
fn q5481(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, comment_count, .. } = &db.post;
    let s = db
        .user
        .with((&db.user.reputation).gt(100))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(view_count.opt()).and(comment_count)))
        .fold([0i64; 7], |a, (((t, s), w), c)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0), a[6] + c]);
    let bs = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 3], |a, c| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64]);
    let w = whole(db.user.iq()).select(Ident::<User>::new().and((&s).filt(|a| a[0] > 10)).and((&bs).opt())).window(rank, |((_, a), _)| Reverse(a[3]), asc);
    let v = top_n(drain(&w).into_iter().map(|x| x.1).collect(), |&(((u, a), _), _)| (Reverse(a[3]), Reverse(a[0]), u), 50);
    rows(v.into_iter().map(|(((u, a), b), r)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[5], a[4]), V::I(a[6])]);
        f.extend(match b {
            Some(b) => b.map(V::I),
            None => [V::Null, V::Null, V::Null],
        });
        f.push(V::I(r));
        row(f)
    }))
}

// WITH UserRankings AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS Rank FROM Users U),
// PostDetails AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, P.AnswerCount, P.CommentCount, U.DisplayName AS OwnerDisplayName,
//        (SELECT COUNT(*) FROM Votes V WHERE V.PostId = P.Id AND V.VoteTypeId = 2) AS UpVotes, (SELECT COUNT(*) FROM Votes V WHERE V.PostId = P.Id AND V.VoteTypeId = 3) AS DownVotes
//     FROM Posts P LEFT JOIN Users U ON P.OwnerUserId = U.Id WHERE P.PostTypeId = 1 AND P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT PD.*, UR.Reputation AS UserReputation, UR.Rank FROM PostDetails PD JOIN UserRankings UR ON PD.OwnerDisplayName = UR.DisplayName ORDER BY PD.Score DESC, PD.ViewCount DESC LIMIT 10)
// SELECT TP.PostId, TP.Title, TP.CreationDate, TP.Score, TP.ViewCount, TP.AnswerCount, TP.CommentCount, TP.UpVotes, TP.DownVotes, TP.OwnerDisplayName, TP.UserReputation, TP.Rank
// FROM TopPosts TP WHERE EXISTS (SELECT 1 FROM Comments C WHERE C.PostId = TP.PostId) ORDER BY TP.UpVotes DESC, TP.CreationDate DESC;
fn q6142(db: &'static So) -> String {
    let rw = whole(db.user.iq()).select(Ident::<User>::new().and(&db.user.reputation)).window(row_number, |(u, r)| (Reverse(r), u), asc);
    let rk: MatSet<(Id<User>, i64)> = (&rw).map(|((u, _), n)| (u, n)).collect();
    let by_name: HashIdx<Str, (Id<User>, i64)> = (&rk).map(|(u, _)| u).select(&db.user.display_name).inv().select(&rk).collect();
    let Post { post_type_id, creation_date, owner_user, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(owner_user.select(&db.user.display_name).select(&by_name)));
    let v = top_n(v, |&(p, (u, _))| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p, u)
    }, 10);
    let tp = rel(v);
    let tpp: MatSet<Id<Post>> = (&tp).map(|(p, _)| p).collect();
    let ud = (&tpp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold((0i64, 0i64), |(u, d), t| (u + (t == Some(2)) as i64, d + (t == Some(3)) as i64));
    let mut v = drain((&tp).and((&tp).map(|(p, _)| p).select(Ident::<Post>::new().with(comments_of(db)))));
    v.sort_by_key(|&(_, ((p, _), _))| (Reverse(ud.get(p).unwrap().0), Reverse(creation_date.get(p).unwrap())));
    rows(v.into_iter().map(|(_, ((p, (u, r)), _))| {
        let (up, dn) = ud.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments"]);
        f.extend([V::I(up), V::I(dn)]);
        f.extend(post_fields(db, p, &["owner"]));
        f.extend([user_col(db, u, "rep"), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.Score, COUNT(c.Id) AS CommentCount, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.ViewCount DESC) AS RankByViews,
//        RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS RankByScore FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY p.Id, p.Title, p.ViewCount, p.Score, p.PostTypeId),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PostActivity AS (SELECT p.Id AS PostId, SUM(CASE WHEN v.VoteTypeId IN (2, 4) THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes,
//        MAX(ph.CreationDate) AS LastActivityDate FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId GROUP BY p.Id)
// SELECT rp.PostId, rp.Title, rp.ViewCount, rp.CommentCount, ua.BadgeCount, pa.Upvotes, pa.Downvotes, pa.LastActivityDate
// FROM RankedPosts rp JOIN Users u ON rp.PostId = u.AccountId JOIN UserBadges ua ON u.Id = ua.UserId JOIN PostActivity pa ON rp.PostId = pa.PostId
// WHERE rp.RankByViews <= 5 OR rp.RankByScore <= 5 ORDER BY rp.RankByViews, rp.RankByScore;
fn q7236(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, origid, .. } = &db.post;
    let w = db.post.group_by(post_type_id).select(Ident::<Post>::new().and(view_count.opt()).and(score)).window(rank, |((_, w), _)| (w.is_none(), Reverse(w)), asc);
    let w = (&w).window(rank, |(((_, _), s), _)| Reverse(s), asc);
    type K = (Id<Post>, (i64, i64));
    let keep: MatSet<K> = (&w).filt(|((_, x), y)| x <= 5 || y <= 5).map(|((((p, _), _), x), y)| (p, (x, y))).collect();
    let accts: HashIdx<i64, Id<User>> = (&db.user.account_id).inv().collect();
    type J = (K, Id<User>);
    let joined: MatSet<J> = (&keep).select(Same::<K>::new().and(Same::<K>::new().map(|(p, _): K| p).select(origid).select(&accts))).collect();
    let tp: MatSet<Id<Post>> = (&joined).map(|((p, _), _)| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let pa = (&tp)
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(history_of(db).select(&db.post_history.creation_date).opt()))
        .fold((0i64, 0i64, None), |(u, d, m): (i64, i64, Option<i64>), (t, h)| (u + matches!(t, Some(2) | Some(4)) as i64, d + (t == Some(3)) as i64, match h {
            Some(h) => Some(m.map_or(h, |m| m.max(h))),
            None => m,
        }));
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let mut v = drain((&joined).select(Same::<J>::new().map(|((p, _), _): J| p).select((&cc).and(&pa)).and(Same::<J>::new().map(|(_, u): J| u).select(&bc))));
    v.sort_by_key(|&(((_, r), _), _)| r);
    rows(v.into_iter().map(|(((p, _), _), ((c, (up, dn, m)), b))| {
        let mut f = post_fields(db, p, &["id", "title", "views"]);
        f.extend([V::I(c), V::I(b), V::I(up), V::I(dn), m.map_or(V::Null, V::T)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, COUNT(c.Id) AS CommentCount, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank,
//        SUM(v.BountyAmount) AS TotalBounty, p.OwnerUserId FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 8
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.OwnerUserId),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, SUM(u.UpVotes) AS TotalUpVotes, SUM(u.DownVotes) AS TotalDownVotes, COUNT(b.Id) AS BadgeCount, MAX(u.Reputation) AS MaxReputation
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.CommentCount, us.DisplayName AS UserDisplayName, us.TotalUpVotes, us.TotalDownVotes, rp.TotalBounty,
//        CASE WHEN rp.UserPostRank <= 5 THEN 'Top Posts' ELSE 'Regular Posts' END AS PostRankCategory
// FROM RankedPosts rp INNER JOIN UserStats us ON rp.OwnerUserId = us.UserId WHERE rp.CommentCount > 5 ORDER BY rp.Score DESC, rp.ViewCount DESC OFFSET 10 ROWS FETCH NEXT 20 ROWS ONLY;
fn q526(db: &'static So) -> String {
    let Post { post_type_id, owner_user, owner_user_id, creation_date, score, view_count, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1)).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date)).window(rank, |(_, d)| Reverse(d), asc);
    let rk: MatSet<(Id<Post>, i64)> = (&w).map(|((p, _), r)| (p, r)).collect();
    let rank_of: HashIdx<Id<Post>, (Id<Post>, i64)> = (&rk).map(|(p, _)| p).inv().select(&rk).collect();
    let bounty: HashIdx<Id<Post>, Id<Vote>> = db.vote.with((&db.vote.vote_type_id).eq(8)).select(&db.vote.post).inv().collect();
    let s = db
        .post
        .with(post_type_id.eq(1))
        .with(owner_user)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and((&bounty).select((&db.vote.bounty_amount).opt()).opt()))
        .fold((0i64, None), |(n, b): (i64, Option<i64>), (c, v)| (n + c.is_some() as i64, match v.flatten() {
            Some(x) => Some(b.unwrap_or(0) + x),
            None => b,
        }));
    let User { up_votes, down_votes, .. } = &db.user;
    let us = db.user.group_by(Ident::<User>::new()).select(up_votes.and(down_votes).and(badges_of(db).opt())).fold((0i64, 0i64), |(a, b), ((u, d), _)| (a + u, b + d));
    let key = |p: Id<Post>| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    };
    let mut v = drain((&s).filt(|(n, _)| n > 5).and(owner_user.select(Ident::<User>::new().and(&us))).and((&rank_of).map(|(_, r)| r)));
    v.sort_by_key(|&(p, _)| key(p));
    rows(v.into_iter().skip(10).take(20).map(|(p, (((n, b), (u, (up, dn))), r))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([V::I(n), user_col(db, u, "name"), V::I(up), V::I(dn), b.map_or(V::Null, V::I), V::S(if r <= 5 { "Top Posts" } else { "Regular Posts" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS OwnerPostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= '2023-01-01' AND p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName, p.OwnerUserId),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, OwnerDisplayName, CommentCount, UpVoteCount, DownVoteCount FROM RankedPosts WHERE OwnerPostRank <= 5)
// SELECT tp.OwnerDisplayName, COUNT(tp.PostId) AS TotalPosts, AVG(tp.Score) AS AverageScore, SUM(tp.ViewCount) AS TotalViewCount, SUM(tp.CommentCount) AS TotalComments,
//        SUM(tp.UpVoteCount) AS TotalUpVotes, SUM(tp.DownVoteCount) AS TotalDownVotes
// FROM TopPosts tp GROUP BY tp.OwnerDisplayName ORDER BY TotalPosts DESC, AverageScore DESC LIMIT 10;
fn q5671(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, score, view_count, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(date(2023, 1, 1)).and(post_type_id.eq(1)))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let g = (&tp)
        .group_by(owner_user.select(&db.user.display_name))
        .select(score.and(view_count.opt()).and(&s))
        .fold([0i64; 7], |a, ((sc, w), x)| [a[0] + 1, a[1] + sc, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + x[0], a[5] + x[1], a[6] + x[2]]);
    let mut v = drain(&g);
    v.sort_by(|x, y| y.1[0].cmp(&x.1[0]).then((y.1[1] as f64 / y.1[0] as f64).total_cmp(&(x.1[1] as f64 / x.1[0] as f64))));
    v.truncate(10);
    rows(v.into_iter().map(|(n, a)| row(vec![V::S(n), V::I(a[0]), avg(a[1], a[0]), nullable(a[3], a[2]), V::I(a[4]), V::I(a[5]), V::I(a[6])])))
}

// WITH UserBadges AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PopularPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.Score, p.OwnerUserId, ROW_NUMBER() OVER (ORDER BY p.ViewCount DESC) AS PopularityRank FROM Posts p WHERE p.PostTypeId = 1 AND p.Score > 0),
// UserPostStats AS (SELECT p.OwnerUserId, COUNT(p.Id) AS PostCount, SUM(p.ViewCount) AS TotalViews, SUM(p.Score) AS TotalScore FROM Posts p WHERE p.PostTypeId IN (1, 2) GROUP BY p.OwnerUserId)
// SELECT ub.UserId, ub.DisplayName, ub.BadgeCount, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges, ups.PostCount, ups.TotalViews, ups.TotalScore, pp.Title AS PopularPostTitle, pp.ViewCount AS PopularPostViewCount
// FROM UserBadges ub LEFT JOIN UserPostStats ups ON ub.UserId = ups.OwnerUserId LEFT JOIN PopularPosts pp ON ub.UserId = pp.OwnerUserId AND pp.PopularityRank = 1
// ORDER BY ub.BadgeCount DESC, ups.TotalViews DESC;
fn q8424(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let Post { post_type_id, score, view_count, owner_user, .. } = &db.post;
    let ups = db.post.with(post_type_id.is_in([1, 2])).group_by(owner_user).select(score.and(view_count.opt())).fold([0i64; 4], |a, (s, w)| [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s]);
    let top = top_n(drain(db.post.with(post_type_id.eq(1).and(score.gt(0))).select(view_count.opt())), |&(p, w)| (w.is_none(), Reverse(w), p), 1);
    let pp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let pp_of: HashIdx<Id<User>, Id<Post>> = (&pp).select(owner_user).inv().collect();
    let mut v = drain((&ub).and((&ups).opt()).and((&pp_of).opt()));
    v.sort_by_key(|&(_, ((b, s), _))| (Reverse(b[0]), s.map_or(true, |s| s[1] == 0), Reverse(s.map(|s| s[2]))));
    rows(v.into_iter().map(|(u, ((b, s), p))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(b.map(V::I));
        f.extend(match s {
            Some(s) => [V::I(s[0]), nullable(s[2], s[1]), V::I(s[3])],
            None => [V::Null, V::Null, V::Null],
        });
        match p {
            Some(p) => f.extend(post_fields(db, p, &["title", "views"])),
            None => f.extend([V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerName, COUNT(a.Id) AS AnswerCount, COUNT(c.Id) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Posts a ON p.Id = a.ParentId LEFT JOIN Comments c ON p.Id = c.PostId
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.OwnerName, rp.AnswerCount, rp.CommentCount FROM RankedPosts rp WHERE rp.Rank = 1 ORDER BY rp.Score DESC LIMIT 10)
// SELECT tp.Title, tp.OwnerName, tp.CreationDate, tp.Score, tp.ViewCount, COALESCE(b.BadgeCount, 0) AS BadgeCount, COALESCE(v.VoteCount, 0) AS VoteCount
// FROM TopPosts tp LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId) b ON tp.OwnerName = (SELECT DisplayName FROM Users WHERE Id = b.UserId)
// LEFT JOIN (SELECT PostId, COUNT(*) AS VoteCount FROM Votes WHERE VoteTypeId = 2 GROUP BY PostId) v ON tp.PostId = v.PostId ORDER BY tp.Score DESC, tp.ViewCount DESC;
fn q6501(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, view_count, .. } = &db.post;
    let top = top_n(drain(db.post.with(post_type_id.eq(1)).select(score)), |&(p, s)| (Reverse(s), p), 10);
    let tp = rel(top.into_iter().map(|x| x.0).collect());
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let bu: MatSet<Id<User>> = db.badge.select(&db.badge.user).collect();
    let by_name: HashIdx<Str, Id<User>> = (&bu).select(&db.user.display_name).inv().collect();
    let vc = db.vote.with((&db.vote.vote_type_id).eq(2)).group_by(&db.vote.post).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let mut v = drain((&tp).select(Ident::<Post>::new().and(owner_user.select(&db.user.display_name).select(&by_name).select(&bc).opt()).and((&vc).opt())));
    v.sort_by_key(|&(_, ((p, _), _))| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    });
    rows(v.into_iter().map(|(_, ((p, b), n))| {
        let mut f = post_fields(db, p, &["title", "owner", "created", "score", "views"]);
        f.extend([V::I(b.unwrap_or(0)), V::I(n.unwrap_or(0))]);
        row(f)
    }))
}

// WITH UserStats AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COALESCE(BadgeCount.BadgeTotal, 0) AS TotalBadges, COALESCE(PostStats.PostCount, 0) AS TotalPosts,
//        COALESCE(VoteStats.VoteCount, 0) AS TotalVotes, COALESCE(ViewStats.ViewTotal, 0) AS TotalViews
//     FROM Users U LEFT JOIN (SELECT UserId, COUNT(*) AS BadgeTotal FROM Badges GROUP BY UserId) AS BadgeCount ON U.Id = BadgeCount.UserId
//     LEFT JOIN (SELECT OwnerUserId, COUNT(*) AS PostCount FROM Posts GROUP BY OwnerUserId) AS PostStats ON U.Id = PostStats.OwnerUserId
//     LEFT JOIN (SELECT UserId, COUNT(*) AS VoteCount FROM Votes GROUP BY UserId) AS VoteStats ON U.Id = VoteStats.UserId
//     LEFT JOIN (SELECT U.Id AS UserId, SUM(P.ViewCount) AS ViewTotal FROM Users U JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id) AS ViewStats ON U.Id = ViewStats.UserId),
// RankedUsers AS (SELECT ..., RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM UserStats)
// SELECT UserId, DisplayName, Reputation, TotalBadges, TotalPosts, TotalVotes, TotalViews, ReputationRank FROM RankedUsers WHERE TotalPosts > 0 ORDER BY TotalPosts DESC, Reputation DESC LIMIT 10;
fn q7923(db: &'static So) -> String {
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let vc = db.user.group_by(Ident::<User>::new()).select(votes_by(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let ups = user_posts(db);
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&db.user.reputation).and(&bc).and(&vc).and(&ups)).window(rank, |((((_, r), _), _), _)| Reverse(r), asc);
    let v: Vec<_> = drain((&w).filt(|((_, a), _)| a[1] > 0)).into_iter().map(|x| x.1).collect();
    let v = top_n(v, |&(((((u, r), _), _), a), _)| (Reverse(a[1]), Reverse(r), u), 10);
    rows(v.into_iter().map(|(((((u, _), b), n), a), r)| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(b), V::I(a[1]), V::I(n), V::I(a[6]), V::I(r)]);
        row(f)
    }))
}

// WITH UserBadgeCounts AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PopularPosts AS (SELECT p.Id, p.Title, p.OwnerUserId, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.ViewCount DESC, p.Score DESC) AS rn FROM Posts p WHERE p.PostTypeId = 1),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, ubc.BadgeCount, ubc.GoldBadges, ubc.SilverBadges, ubc.BronzeBadges, pp.Title AS MostViewedPostTitle, pp.ViewCount AS MostViewedPostCount
//     FROM Users u JOIN UserBadgeCounts ubc ON u.Id = ubc.UserId LEFT JOIN PopularPosts pp ON u.Id = pp.OwnerUserId AND pp.rn = 1 WHERE u.Reputation > 1000)
// SELECT tu.UserId, tu.DisplayName, tu.Reputation, tu.BadgeCount, tu.GoldBadges, tu.SilverBadges, tu.BronzeBadges, tu.MostViewedPostTitle, tu.MostViewedPostCount
// FROM TopUsers tu ORDER BY tu.Reputation DESC, tu.BadgeCount DESC LIMIT 10;
fn q7588(db: &'static So) -> String {
    let rep = |u: Id<User>| db.user.reputation.get(u).unwrap();
    let ub = db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let top = top_n(drain(&ub), |&(u, a)| (Reverse(rep(u)), Reverse(a[0]), u), 10);
    let tu = rel(top);
    let Post { post_type_id, owner_user, score, view_count, .. } = &db.post;
    let w = db
        .post
        .with(post_type_id.eq(1))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(view_count.opt().and(score)))
        .window(row_number, |(p, (w, s))| (w.is_none(), Reverse(w), Reverse(s), p), asc);
    let pp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let pp_of: HashIdx<Id<User>, Id<Post>> = (&pp).select(owner_user).inv().collect();
    let mut v = drain((&tu).and((&tu).map(|(u, _)| u).select((&pp_of).opt())));
    v.sort_by_key(|x| x.0);
    rows(v.into_iter().map(|(_, ((u, a), p))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend(a.map(V::I));
        match p {
            Some(p) => f.extend(post_fields(db, p, &["title", "views"])),
            None => f.extend([V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH PostStats AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.ViewCount, p.AnswerCount, p.Score, ARRAY_LENGTH(STRING_TO_ARRAY(p.Tags, '>'), 1) AS TagCount,
//        u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, AVG(b.Class) AS AvgBadgeClass
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Body, p.CreationDate, p.ViewCount, p.AnswerCount, p.Score, ARRAY_LENGTH(STRING_TO_ARRAY(p.Tags, '>'), 1), u.DisplayName),
// RankedPosts AS (SELECT PostId, Title, ViewCount, AnswerCount, Score, TagCount, OwnerDisplayName, CommentCount, UpVotes, DownVotes, AvgBadgeClass,
//        ROW_NUMBER() OVER (ORDER BY Score DESC, ViewCount DESC) AS Rank FROM PostStats)
// SELECT Rank, Title, ViewCount, AnswerCount, Score, TagCount, OwnerDisplayName, CommentCount, UpVotes, DownVotes, AvgBadgeClass FROM RankedPosts WHERE Rank <= 10 ORDER BY Rank;
//
// The row number only reads Score and ViewCount, so the ten posts are picked
// before their joined rows are aggregated.
fn q29686(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, view_count, tags_str, .. } = &db.post;
    let w = whole(db.post.iq())
        .select(Ident::<Post>::new().with(post_type_id.eq(1)).with(owner_user).and(score.and(view_count.opt())))
        .window(row_number, |(p, (s, w))| (Reverse(s), w.is_none(), Reverse(w), p), asc);
    let top = top_n(drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|x| ((x.1).0 .0, (x.1).1)).collect(), |&(_, r)| r, 0);
    let tp: MatSet<Id<Post>> = rel(top.iter().map(|x| x.0).collect()).map(|p| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(owner_user.select(badges_of(db).select(&db.badge.class)).opt()))
        .fold([0i64; 5], |a, ((c, t), b)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64, a[3] + b.is_some() as i64, a[4] + b.unwrap_or(0)]);
    rows(top.into_iter().map(|(p, i)| {
        let a = s.get(p).unwrap();
        let mut f = vec![V::I(i)];
        f.extend(post_fields(db, p, &["title", "views", "answers", "score"]));
        f.push(tags_str.get(p).map_or(V::Null, |t| V::I(t.matches('>').count() as i64 + 1)));
        f.extend(post_fields(db, p, &["owner"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[4], a[3])]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN p.PostTypeId IN (3, 4, 5) THEN 1 ELSE 0 END) AS TotalWikis,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId WHERE u.Reputation >= 1000 GROUP BY u.Id, u.DisplayName),
// MostActiveUsers AS (SELECT ..., ROW_NUMBER() OVER (ORDER BY TotalPosts DESC) AS Rank FROM UserPostStats)
// SELECT mu.Rank, mu.DisplayName, mu.TotalPosts, mu.TotalQuestions, mu.TotalAnswers, mu.TotalWikis, mu.UpVotes, mu.DownVotes,
//        CASE WHEN mu.TotalQuestions > 0 THEN CAST(mu.UpVotes AS FLOAT) / mu.TotalQuestions ELSE 0 END AS AvgUpVotesPerQuestion,
//        CASE WHEN mu.TotalAnswers > 0 THEN CAST(mu.DownVotes AS FLOAT) / mu.TotalAnswers ELSE 0 END AS AvgDownVotesPerAnswer
// FROM MostActiveUsers mu WHERE mu.Rank <= 10 ORDER BY mu.Rank;
//
// FLOAT is single precision, so the ratios are f32 quotients.
fn q7354(db: &'static So) -> String {
    let s = db
        .user
        .with((&db.user.reputation).ge(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 6], |a, p| match p {
            Some((t, v)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (3..=5).contains(&t) as i64, a[4] + (v == Some(2)) as i64, a[5] + (v == Some(3)) as i64],
            None => a,
        });
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&s)).window(row_number, |(u, a)| (Reverse(a[0]), u), asc);
    let v = top_n(drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|x| x.1).collect(), |&(_, r)| r, 0);
    let q = |x: i64, n: i64| V::F(if n > 0 { (x as f32 / n as f32) as f64 } else { 0.0 });
    rows(v.into_iter().map(|((u, a), i)| {
        let mut f = vec![V::I(i), user_col(db, u, "name")];
        f.extend(a.map(V::I));
        f.extend([q(a[4], a[1]), q(a[5], a[2])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank, p.OwnerUserId
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate > (TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year')),
// RecentBadges AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount, MAX(b.Date) AS LastBadgeDate FROM Badges b WHERE b.Date > (TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '3 months') GROUP BY b.UserId),
// PostHistoryAggregated AS (SELECT ph.PostId, COUNT(ph.Id) AS EditCount, MAX(ph.CreationDate) AS LastEditDate FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (4, 5, 6, 24) GROUP BY ph.PostId)
// SELECT rp.Title, rp.OwnerDisplayName, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, rp.CommentCount, r.FeaturedBadgeCount, ph.EditCount, ph.LastEditDate
// FROM RankedPosts rp LEFT JOIN (SELECT rb.UserId, rb.BadgeCount AS FeaturedBadgeCount FROM RecentBadges rb JOIN Users u ON rb.UserId = u.Id WHERE rb.BadgeCount > 0) r ON rp.OwnerUserId = r.UserId
// LEFT JOIN PostHistoryAggregated ph ON rp.PostId = ph.PostId WHERE rp.Rank <= 10 ORDER BY rp.Score DESC, rp.CreationDate DESC;
fn q8643(db: &'static So) -> String {
    let Post { creation_date, owner_user, post_type_id, score, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|(((p, _), _), _)| p).collect();
    let rb = db.badge.with((&db.badge.date).gt(add_months(ts(2024, 10, 1, 12, 34, 56), -3))).group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let ph = db.post_history.with(post_history_type_id.is_in([4, 5, 6, 24])).group_by(post).select(hd).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let mut v = drain((&tp).select(Ident::<Post>::new().and(owner_user.select(&rb).opt()).and((&ph).opt())));
    v.sort_by_key(|&(_, ((p, _), _))| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())));
    rows(v.into_iter().map(|(_, ((p, b), h))| {
        let mut f = post_fields(db, p, &["title", "owner", "created", "score", "views", "answers", "comments"]);
        f.push(b.map_or(V::Null, V::I));
        f.extend(match h {
            Some((n, m)) => [V::I(n), V::T(m)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, u.DisplayName AS OwnerDisplayName, p.ViewCount, COUNT(c.Id) AS CommentCount,
//        COUNT(v.Id) FILTER (WHERE v.VoteTypeId IN (2, 6)) AS UpVotes, COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 3) AS DownVotes, ROW_NUMBER() OVER (ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Body, p.CreationDate, u.DisplayName, p.ViewCount),
// FilteredPosts AS (SELECT *, (UpVotes - DownVotes) AS NetVotes FROM RankedPosts WHERE ViewCount > 100),
// TopPosts AS (SELECT PostId, Title, OwnerDisplayName, CreationDate, ViewCount, CommentCount, NetVotes, RANK() OVER (ORDER BY NetVotes DESC) AS VoteRank FROM FilteredPosts)
// SELECT tp.PostId, tp.Title, tp.OwnerDisplayName, tp.CreationDate, tp.ViewCount, tp.CommentCount, tp.NetVotes,
//        CASE WHEN tp.VoteRank <= 10 THEN 'Top Trending' WHEN tp.VoteRank BETWEEN 11 AND 50 THEN 'Popular' ELSE 'Less Active' END AS PostCategory
// FROM TopPosts tp WHERE tp.VoteRank <= 50 ORDER BY tp.VoteRank;
fn q27928(db: &'static So) -> String {
    let Post { post_type_id, view_count, .. } = &db.post;
    let s = db
        .post
        .with(post_type_id.eq(1).and(view_count.gt(100)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold((0i64, 0i64), |(n, net), (c, t)| (n + c.is_some() as i64, net + matches!(t, Some(2) | Some(6)) as i64 - (t == Some(3)) as i64));
    let w = whole(&s).select(Ident::<Post>::new().and(&s)).window(rank, |(_, (_, n))| Reverse(n), asc);
    rows(drain((&w).filt(|(_, r)| r <= 50)).into_iter().map(|x| x.1).map(|((p, (c, n)), r)| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "views"]);
        f.extend([V::I(c), V::I(n), V::S(if r <= 10 { "Top Trending" } else { "Popular" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, u.DisplayName AS OwnerDisplayName, p.Score, p.ViewCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.Score > 0),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(DISTINCT p.Id) AS QuestionCount FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId AND p.PostTypeId = 1 GROUP BY u.Id, u.Reputation),
// MaxUserReputation AS (SELECT UserId, MAX(Reputation) AS MaxReputation FROM UserReputation GROUP BY UserId),
// TopTags AS (SELECT t.TagName, COUNT(DISTINCT p.Id) AS PostCount FROM Tags t JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' GROUP BY t.TagName
//     HAVING COUNT(DISTINCT p.Id) > 10 ORDER BY PostCount DESC LIMIT 5)
// SELECT up.DisplayName, up.Reputation, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, tt.TagName
// FROM RankedPosts rp JOIN Users up ON rp.OwnerUserId = up.Id JOIN MaxUserReputation mur ON up.Id = mur.UserId JOIN TopTags tt ON tt.PostCount > 0
// WHERE rp.PostRank = 1 ORDER BY mur.MaxReputation DESC, rp.Score DESC;
fn q6038(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, creation_date, .. } = &db.post;
    let w = db
        .post
        .with(post_type_id.eq(1).and(score.gt(0)))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (Reverse(d), p), asc);
    type P = (Id<Post>, Id<Tag>);
    type PN = (Id<Post>, Str);
    let pn: MatSet<PN> = (&tag_mentions(db)).select(Same::<P>::new().map(|(p, _): P| p).and(Same::<P>::new().map(|(_, t): P| t).select(&db.tag.tag_name))).collect();
    let tc = (&pn).group_by(Same::<PN>::new().map(|(_, n): PN| n)).select(Same::<PN>::new().map(|(p, _): PN| p)).count_distinct();
    let tt = rel(top_n(drain((&tc).filt(|n| n > 10)), |&(t, n)| (Reverse(n), t), 5));
    let mut v = drain((&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).cross((&tt).filt(|(_, n)| n > 0)));
    v.sort_by_key(|&((u, _), (p, _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(score.get(p).unwrap())));
    rows(v.into_iter().map(|((u, _), (p, (t, _)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["title", "created", "score", "views"]));
        f.push(V::S(t));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank FROM Posts p WHERE p.PostTypeId = 1 AND p.Score > 0),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(p.Score) AS TotalScore FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId WHERE p.PostTypeId = 1
//     GROUP BY u.Id, u.DisplayName ORDER BY TotalScore DESC LIMIT 10),
// RecentComments AS (SELECT c.PostId, c.Text AS CommentText, c.CreationDate AS CommentDate, u.DisplayName AS CommentedBy FROM Comments c JOIN Users u ON c.UserId = u.Id
//     WHERE c.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '7 days')
// SELECT rp.PostId, rp.Title, rp.CreationDate AS PostCreationDate, rp.Score AS PostScore, rp.ViewCount AS PostViewCount, rp.AnswerCount AS PostAnswerCount,
//        rp.CommentCount AS PostCommentCount, tu.DisplayName AS TopUser, tc.CommentText, tc.CommentDate, tc.CommentedBy
// FROM RankedPosts rp JOIN TopUsers tu ON rp.Rank <= 3 LEFT JOIN RecentComments tc ON rp.PostId = tc.PostId WHERE rp.Rank <= 5 ORDER BY rp.Score DESC, rp.CreationDate DESC;
fn q5279(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, owner_user_id, creation_date, .. } = &db.post;
    let w = db
        .post
        .with(post_type_id.eq(1).and(score.gt(0)))
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 3).map(|(((p, _), _), _)| p).collect();
    let tot = db.post.with(post_type_id.eq(1)).group_by(owner_user).select(score).fold(0i64, |s, x| s + x);
    let tu = rel(top_n(drain(&tot), |&(u, s)| (Reverse(s), u), 10));
    let Comment { post, user, creation_date: cd, .. } = &db.comment;
    let rc: HashIdx<Id<Post>, Id<Comment>> = db.comment.with(cd.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -7))).with(user).select(post).inv().collect();
    let mut v = drain((&tp).select(Ident::<Post>::new().and((&rc).opt())).cross(&tu));
    v.sort_by_key(|&((p, _), _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())));
    rows(v.into_iter().map(|(_, ((p, c), (u, _)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments"]);
        f.push(user_col(db, u, "name"));
        f.extend(match c {
            Some(c) => [V::S(db.comment.text.get(c).unwrap()), V::T(cd.get(c).unwrap()), user_col(db, user.get(c).unwrap(), "name")],
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, upvotes.UpVoteCount, downvotes.DownVoteCount, COALESCE(c.CommentCount, 0) AS CommentCount,
//        p.ViewCount, r.Rank, p.OwnerUserId
//     FROM Posts p LEFT JOIN (SELECT PostId, COUNT(*) AS UpVoteCount FROM Votes WHERE VoteTypeId = 2 GROUP BY PostId) upvotes ON p.Id = upvotes.PostId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS DownVoteCount FROM Votes WHERE VoteTypeId = 3 GROUP BY PostId) downvotes ON p.Id = downvotes.PostId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
//     JOIN (SELECT Id, RANK() OVER (ORDER BY Score DESC, CreationDate ASC) AS Rank FROM Posts WHERE PostTypeId = 1) r ON p.Id = r.Id)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.UpVoteCount, rp.DownVoteCount, rp.CommentCount, rp.ViewCount, rp.Rank, u.DisplayName AS Owner, u.Reputation
// FROM RankedPosts rp JOIN Users u ON rp.OwnerUserId = u.Id WHERE rp.Rank <= 10 ORDER BY rp.Rank;
fn q5749(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, owner_user, .. } = &db.post;
    let w = whole(db.post.iq()).select(Ident::<Post>::new().with(post_type_id.eq(1)).and(score.and(creation_date))).window(rank, |(_, (s, d))| (Reverse(s), d), asc);
    type T = (Id<Post>, i64);
    let tp: MatSet<T> = (&w).filt(|(_, r)| r <= 10).map(|((p, _), r)| (p, r)).collect();
    let cnt = |t: i64| db.vote.with((&db.vote.vote_type_id).eq(t)).group_by(&db.vote.post).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let (up, dn) = (cnt(2), cnt(3));
    let cc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let mut v = drain((&tp).select(Same::<T>::new().and(Same::<T>::new().map(|(p, _): T| p).select((&up).opt().and((&dn).opt()).and((&cc).opt()).and(owner_user)))));
    v.sort_by_key(|&(_, ((_, r), _))| r);
    rows(v.into_iter().map(|(_, ((p, r), (((u, d), c), o)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend([u.map_or(V::Null, V::I), d.map_or(V::Null, V::I), V::I(c.unwrap_or(0))]);
        f.extend(post_fields(db, p, &["views"]));
        f.push(V::I(r));
        f.extend(ucols(db, o, &["name", "rep"]));
        row(f)
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.CreationDate, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges, ROW_NUMBER() OVER (ORDER BY u.Reputation DESC) AS UserRank
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation, u.CreationDate),
// PostSummaries AS (SELECT p.OwnerUserId, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, MAX(p.LastActivityDate) AS LastActive FROM Posts p GROUP BY p.OwnerUserId)
// SELECT us.DisplayName, us.Reputation, us.BadgeCount, us.GoldBadges, us.SilverBadges, us.BronzeBadges, ps.TotalPosts, ps.Questions, ps.Answers, ps.LastActive,
//        CASE WHEN us.Reputation > 1000 THEN 'High Reputation' WHEN us.Reputation > 500 THEN 'Medium Reputation' ELSE 'Low Reputation' END AS ReputationLevel
// FROM UserStats us JOIN PostSummaries ps ON us.UserId = ps.OwnerUserId LEFT JOIN (SELECT UserId, COUNT(*) AS TotalVotes FROM Votes WHERE VoteTypeId = 2 GROUP BY UserId) v ON us.UserId = v.UserId
// ORDER BY us.Reputation DESC, ps.TotalPosts DESC LIMIT 10;
fn q3144(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let Post { post_type_id, last_activity_date, owner_user, .. } = &db.post;
    let ps = db.post.group_by(owner_user).select(post_type_id.and(last_activity_date)).fold([0, 0, 0, i64::MIN], |a, (t, d)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3].max(d)]);
    let rep = |u: Id<User>| db.user.reputation.get(u).unwrap();
    let v = top_n(drain((&ub).and(&ps)), |&(u, (_, s))| (Reverse(rep(u)), Reverse(s[0]), u), 10);
    rows(v.into_iter().map(|(u, (b, s))| {
        let r = rep(u);
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(b.map(V::I));
        f.extend([V::I(s[0]), V::I(s[1]), V::I(s[2]), V::T(s[3])]);
        f.push(V::S(if r > 1000 { "High Reputation" } else if r > 500 { "Medium Reputation" } else { "Low Reputation" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS Rank,
//        COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (2, 3) GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.OwnerUserId),
// UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.CreationDate, COUNT(p.Id) AS PostCount, SUM(CASE WHEN bp.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN bp.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN bp.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges bp ON u.Id = bp.UserId GROUP BY u.Id, u.DisplayName, u.Reputation, u.CreationDate)
// SELECT us.DisplayName, us.Reputation, us.PostCount, us.GoldBadges, us.SilverBadges, us.BronzeBadges, rp.Title, rp.CreationDate, rp.Score, rp.CommentCount, rp.UpvoteCount
// FROM UserStatistics us JOIN RankedPosts rp ON us.UserId = rp.OwnerUserId WHERE us.Reputation > 1000 ORDER BY us.Reputation DESC, rp.Score DESC LIMIT 10 OFFSET 0;
//
// The order reads no aggregate, so the ten rows are picked before the
// joined rows are counted.
fn q33353(db: &'static So) -> String {
    let rep = |u: Id<User>| db.user.reputation.get(u).unwrap();
    let Post { score, .. } = &db.post;
    let v = drain(db.user.with((&db.user.reputation).gt(1000)).select(posts_of(db)));
    let v = top_n(v, |&(u, p)| (Reverse(rep(u)), Reverse(score.get(p).unwrap()), p), 10);
    let tp: MatSet<Id<Post>> = rel(v.iter().map(|x| x.1).collect()).map(|p| p).collect();
    let tu: MatSet<Id<User>> = rel(v.iter().map(|x| x.0).collect()).map(|u| u).collect();
    let ud: HashIdx<Id<Post>, Id<Vote>> = db.vote.with((&db.vote.vote_type_id).is_in([2, 3])).select(&db.vote.post).inv().collect();
    let rp = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and((&ud).select(&db.vote.vote_type_id).opt()))
        .fold((0i64, 0i64), |(c, u), (x, t)| (c + x.is_some() as i64, u + (t == Some(2)) as i64));
    let us = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 4], |a, (p, c)| [a[0] + p.is_some() as i64, a[1] + (c == Some(1)) as i64, a[2] + (c == Some(2)) as i64, a[3] + (c == Some(3)) as i64]);
    rows(v.into_iter().map(|(u, p)| {
        let (c, n) = rp.get(p).unwrap();
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(us.get(u).unwrap().map(V::I));
        f.extend(post_fields(db, p, &["title", "created", "score"]));
        f.extend([V::I(c), V::I(n)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS RankScore,
//        COUNT(c.Id) AS CommentCount, p.OwnerUserId FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.PostTypeId = 1
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.OwnerUserId),
// UserStats AS (SELECT u.Id AS UserId, u.Reputation, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId
//     LEFT JOIN Badges b ON u.Id = b.UserId WHERE u.Reputation > 1000 GROUP BY u.Id, u.Reputation),
// TopContributors AS (SELECT us.UserId, us.Reputation, us.TotalBounty, us.BadgeCount, rp.PostId, rp.Title, rp.RankScore, rp.CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY us.UserId ORDER BY rp.RankScore) AS UserPostRank FROM UserStats us JOIN RankedPosts rp ON us.UserId = rp.OwnerUserId)
// SELECT t.UserId, u.DisplayName, t.Title, t.RankScore, t.CommentCount, t.TotalBounty, t.BadgeCount FROM TopContributors t JOIN Users u ON t.UserId = u.Id
// WHERE t.UserPostRank = 1 ORDER BY t.TotalBounty DESC LIMIT 10;
fn q4265(db: &'static So) -> String {
    let Post { post_type_id, owner_user, score, .. } = &db.post;
    let us = db
        .user
        .with((&db.user.reputation).gt(1000))
        .with(posts_of(db).with(post_type_id.eq(1)))
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select((&db.vote.bounty_amount).opt()).opt().and(badges_of(db).opt()))
        .fold((0i64, 0i64), |(s, n), (v, b)| (s + v.flatten().unwrap_or(0), n + b.is_some() as i64));
    let tu = rel(top_n(drain(&us), |&(u, (s, _))| (Reverse(s), u), 10));
    let tset: MatSet<Id<User>> = (&tu).map(|(u, _)| u).collect();
    let w = db
        .post
        .with(post_type_id.eq(1))
        .group_by(owner_user.select(Ident::<User>::new().with(&tset)))
        .select(Ident::<Post>::new().and(score))
        .window(rank, |(_, s)| Reverse(s), asc);
    let w = (&w).window(row_number, |((p, _), r)| (r, p), asc);
    type PR = (Id<Post>, i64);
    let bp: HashIdx<Id<User>, PR> = (&w).filt(|(_, n)| n == 1).map(|(((p, _), r), _)| (p, r)).collect();
    let cc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let mut v = drain((&tu).and((&tu).map(|(u, _)| u).select((&bp).select(Same::<PR>::new().and(Same::<PR>::new().map(|(p, _): PR| p).select(&cc).opt())))));
    v.sort_by_key(|x| x.0);
    rows(v.into_iter().map(|(_, ((u, (s, n)), ((p, r), c)))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(post_fields(db, p, &["title"]));
        f.extend([V::I(r), V::I(c.unwrap_or(0)), V::I(s), V::I(n)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostID, p.Title, p.Body, p.Tags, u.DisplayName AS OwnerDisplayName, p.CreationDate, COUNT(a.Id) AS AnswerCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        ROW_NUMBER() OVER (PARTITION BY p.Tags ORDER BY COUNT(a.Id) DESC) AS RankByTags
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2 LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Body, p.Tags, u.DisplayName, p.CreationDate),
// FilteredPosts AS (SELECT PostID, Title, Body, Tags, OwnerDisplayName, CreationDate, AnswerCount, UpVotes, DownVotes FROM RankedPosts WHERE RankByTags <= 5),
// PostAnalysis AS (SELECT fp.*, CHAR_LENGTH(fp.Body) AS BodyLength, CHAR_LENGTH(fp.Title) AS TitleLength, CASE WHEN fp.AnswerCount > 0 THEN 'Answered' ELSE 'Unanswered' END AS PostStatus
//     FROM FilteredPosts fp)
// SELECT PostID, Title, OwnerDisplayName, CreationDate, BodyLength, TitleLength, PostStatus, UpVotes, DownVotes FROM PostAnalysis ORDER BY UpVotes DESC, CreationDate DESC LIMIT 50;
fn q29559(db: &'static So) -> String {
    let Post { post_type_id, tags_str, creation_date, body, title, .. } = &db.post;
    let s = db
        .post
        .with(post_type_id.eq(1))
        .group_by(Ident::<Post>::new())
        .select(answers_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (x, t)| [a[0] + x.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let w = db.post.with(post_type_id.eq(1)).group_by(tags_str.opt()).select(Ident::<Post>::new().and(&s)).window(row_number, |(p, a)| (Reverse(a[0]), p), asc);
    let v = top_n(drain((&w).filt(|(_, r)| r <= 5)).into_iter().map(|x| x.1 .0).collect(), |&(p, a)| (Reverse(a[1]), Reverse(creation_date.get(p).unwrap()), p), 50);
    rows(v.into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created"]);
        f.push(body.get(p).map_or(V::Null, |b| V::I(b.chars().count() as i64)));
        f.push(title.get(p).map_or(V::Null, |t| V::I(t.chars().count() as i64)));
        f.extend([V::S(if a[0] > 0 { "Answered" } else { "Unanswered" }), V::I(a[1]), V::I(a[2])]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, AVG(p.Score) AS AverageScore, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Badges b ON u.Id = b.UserId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName),
// TopActiveUsers AS (SELECT ..., RANK() OVER (ORDER BY PostCount DESC) AS ActivityRank FROM UserActivity)
// SELECT u.DisplayName, u.PostCount AS TotalPosts, u.QuestionCount, u.AnswerCount, u.AverageScore, u.UpVotes, u.DownVotes, (u.GoldBadges + u.SilverBadges + u.BronzeBadges) AS TotalBadges,
//        u.GoldBadges, u.SilverBadges, u.BronzeBadges
// FROM TopActiveUsers u WHERE u.ActivityRank <= 10 ORDER BY u.PostCount DESC;
fn q9238(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let s = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 9], |a, (p, b)| {
            let (t, s, v) = p.map_or((None, 0, None), |((t, s), v)| (Some(t), s, v));
            [
                a[0] + t.is_some() as i64,
                a[1] + (t == Some(1)) as i64,
                a[2] + (t == Some(2)) as i64,
                a[3] + s,
                a[4] + (v == Some(2)) as i64,
                a[5] + (v == Some(3)) as i64,
                a[6] + (b == Some(1)) as i64,
                a[7] + (b == Some(2)) as i64,
                a[8] + (b == Some(3)) as i64,
            ]
        });
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&s)).window(rank, |(_, a)| Reverse(a[0]), asc);
    rows(drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|x| x.1 .0).map(|(u, a)| {
        row(vec![
            user_col(db, u, "name"),
            V::I(a[0]),
            V::I(a[1]),
            V::I(a[2]),
            avg(a[3], a[0]),
            V::I(a[4]),
            V::I(a[5]),
            V::I(a[6] + a[7] + a[8]),
            V::I(a[6]),
            V::I(a[7]),
            V::I(a[8]),
        ])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.Tags, p.CreationDate, p.ViewCount, COUNT(a.Id) AS AnswerCount, RANK() OVER (PARTITION BY p.Tags ORDER BY p.ViewCount DESC) AS TagRank
//     FROM Posts p LEFT JOIN Posts a ON p.Id = a.ParentId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Body, p.Tags, p.CreationDate, p.ViewCount),
// TopTagQuestions AS (SELECT rp.PostId, rp.Title, rp.ViewCount, rp.Tags, rp.CreationDate, rp.AnswerCount FROM RankedPosts rp WHERE rp.TagRank = 1),
// UserBadgeSummary AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName)
// SELECT ttq.Title, ttq.ViewCount, ttq.Tags, ub.UserId, ub.DisplayName, ub.BadgeCount, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges
// FROM TopTagQuestions ttq JOIN Posts p ON ttq.PostId = p.Id JOIN Users u ON p.OwnerUserId = u.Id JOIN UserBadgeSummary ub ON u.Id = ub.UserId WHERE ub.BadgeCount > 0
// ORDER BY ttq.ViewCount DESC, ub.BadgeCount DESC;
fn q27619(db: &'static So) -> String {
    let Post { post_type_id, tags_str, view_count, owner_user, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1)).group_by(tags_str.opt()).select(Ident::<Post>::new().and(view_count.opt())).window(rank, |(_, w)| (w.is_none(), Reverse(w)), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let mut v = drain((&tp).select(Ident::<Post>::new().and(owner_user.select(Ident::<User>::new().and((&ub).filt(|a| a[0] > 0))))));
    v.sort_by_key(|&(_, (p, (_, a)))| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), Reverse(a[0]))
    });
    rows(v.into_iter().map(|(_, (p, (u, a)))| {
        let mut f = post_fields(db, p, &["title", "views", "tags"]);
        f.extend(ucols(db, u, &["uid", "name"]));
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RECURSIVE UserPostCount AS (SELECT U.Id AS UserId, COUNT(P.Id) AS PostCount FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id),
// RecentEdits AS (SELECT PH.UserId, PH.PostId, PH.CreationDate, ROW_NUMBER() OVER (PARTITION BY PH.PostId ORDER BY PH.CreationDate DESC) AS EditRank FROM PostHistory PH WHERE PH.PostHistoryTypeId IN (4, 5)),
// TopContributors AS (SELECT U.Id, U.DisplayName, U.Reputation, UPC.PostCount, COUNT(RE.PostId) AS RecentEditCount FROM Users U JOIN UserPostCount UPC ON U.Id = UPC.UserId
//     LEFT JOIN RecentEdits RE ON U.Id = RE.UserId WHERE U.Reputation > 1000 GROUP BY U.Id, U.DisplayName, U.Reputation, UPC.PostCount HAVING COUNT(RE.PostId) > 5),
// MostViewedPosts AS (SELECT P.Id, P.Title, P.ViewCount, RANK() OVER (ORDER BY P.ViewCount DESC) AS ViewRank FROM Posts P WHERE P.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '6 months'))
// SELECT U.DisplayName AS Contributor, U.Reputation, U.PostCount AS TotalPosts, U.RecentEditCount AS RecentEdits, P.Title AS MostViewedPost, P.ViewCount
// FROM TopContributors U LEFT JOIN MostViewedPosts P ON U.PostCount > 0 WHERE P.ViewRank <= 10 ORDER BY U.Reputation DESC, TotalPosts DESC LIMIT 50;
//
// No CTE refers to itself, so RECURSIVE changes nothing.
fn q32582(db: &'static So) -> String {
    let PostHistory { post_history_type_id, user, .. } = &db.post_history;
    let re = db.post_history.with(post_history_type_id.is_in([4, 5])).group_by(user).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let ups = user_posts(db);
    let tc = db.user.with((&db.user.reputation).gt(1000)).select((&re).filt(|n| n > 5).and((&ups).filt(|a| a[1] > 0)));
    let Post { creation_date, view_count, .. } = &db.post;
    let mw = whole(db.post.iq())
        .select(Ident::<Post>::new().with(creation_date.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -6))).and(view_count.opt()))
        .window(rank, |(_, w)| (w.is_none(), Reverse(w)), asc);
    let mv: MatSet<(Id<Post>, Option<i64>)> = (&mw).filt(|(_, r)| r <= 10).map(|(x, _)| x).collect();
    let v = top_n(drain(tc.cross(&mv)), |&((u, (p, _)), ((_, a), _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(a[1]), u, p), 50);
    rows(v.into_iter().map(|((u, (p, _)), ((n, a), _))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[1]), V::I(n)]);
        f.extend(post_fields(db, p, &["title", "views"]));
        row(f)
    }))
}

// WITH UserBadgeRanks AS (SELECT Users.Id AS UserId, Users.DisplayName, COUNT(Badges.Id) AS BadgeCount,
//        SUM(CASE WHEN Badges.Class = 1 THEN 3 WHEN Badges.Class = 2 THEN 2 WHEN Badges.Class = 3 THEN 1 ELSE 0 END) AS TotalBadgeValue
//     FROM Users LEFT JOIN Badges ON Users.Id = Badges.UserId GROUP BY Users.Id, Users.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, RANK() OVER (ORDER BY TotalBadgeValue DESC) AS BadgeRank FROM UserBadgeRanks WHERE BadgeCount > 0),
// UserPostStatistics AS (SELECT Users.Id AS UserId, Users.DisplayName, COUNT(Posts.Id) AS PostCount, AVG(Posts.Score) AS AvgPostScore FROM Users LEFT JOIN Posts ON Users.Id = Posts.OwnerUserId GROUP BY Users.Id, Users.DisplayName),
// TopUsersWithPostStats AS (SELECT t.UserId, t.DisplayName, t.BadgeRank, ups.PostCount, ups.AvgPostScore FROM TopUsers t JOIN UserPostStatistics ups ON t.UserId = ups.UserId),
// BenchmarkResults AS (SELECT t.UserId, t.DisplayName, t.BadgeRank, t.PostCount, t.AvgPostScore, (t.AvgPostScore * t.PostCount) AS PerformanceScore FROM TopUsersWithPostStats t WHERE t.BadgeRank <= 10)
// SELECT DisplayName, BadgeRank, PostCount, AvgPostScore, PerformanceScore FROM BenchmarkResults ORDER BY PerformanceScore DESC, BadgeRank ASC;
fn q27195(db: &'static So) -> String {
    let bv = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class)).fold(0i64, |s, c| s + match c {
        1 => 3,
        2 => 2,
        3 => 1,
        _ => 0,
    });
    let w = whole(&bv).select(Ident::<User>::new().and(&bv)).window(rank, |(_, s)| Reverse(s), asc);
    type T = (Id<User>, i64);
    let tu: MatSet<T> = (&w).filt(|(_, r)| r <= 10).map(|((u, _), r)| (u, r)).collect();
    let ups = user_posts(db);
    let perf = |a: [i64; 10]| if a[1] == 0 { None } else { Some((a[4] as f64 / a[1] as f64) * a[1] as f64) };
    let mut v = drain((&tu).select(Same::<T>::new().and(Same::<T>::new().map(|(u, _): T| u).select(&ups))));
    v.sort_by(|x, y| {
        let (px, py) = (perf(x.1 .1), perf(y.1 .1));
        match (px, py) {
            (Some(a), Some(b)) => b.total_cmp(&a),
            (None, None) => std::cmp::Ordering::Equal,
            (None, _) => std::cmp::Ordering::Greater,
            (_, None) => std::cmp::Ordering::Less,
        }
        .then(x.1 .0 .1.cmp(&y.1 .0 .1))
    });
    rows(v.into_iter().map(|(_, ((u, r), a))| row(vec![user_col(db, u, "name"), V::I(r), V::I(a[1]), avg(a[4], a[1]), perf(a).map_or(V::Null, V::F)])))
}

// WITH UserRankings AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U),
// PopularPosts AS (SELECT P.Id AS PostId, P.Title, P.Score, P.ViewCount, COUNT(C.Id) AS CommentCount, RANK() OVER (ORDER BY P.Score DESC, P.ViewCount DESC) AS PopularityRank, P.OwnerUserId
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId WHERE P.CreationDate >= DATE '2024-10-01' - INTERVAL '30 days' GROUP BY P.Id, P.Title, P.Score, P.ViewCount, P.OwnerUserId),
// UserPostCounts AS (SELECT P.OwnerUserId, COUNT(P.Id) AS PostCount FROM Posts P WHERE P.CreationDate >= DATE '2024-10-01' - INTERVAL '1 year' GROUP BY P.OwnerUserId),
// RecentVotes AS (SELECT V.PostId, COUNT(V.Id) AS TotalVotes FROM Votes V WHERE V.CreationDate >= DATE '2024-10-01' - INTERVAL '30 days' GROUP BY V.PostId)
// SELECT UR.DisplayName, UR.Reputation, U.PostCount, PP.Title, PP.Score, PP.CommentCount, RV.TotalVotes, PP.PopularityRank
// FROM UserRankings UR JOIN UserPostCounts U ON UR.UserId = U.OwnerUserId JOIN PopularPosts PP ON U.OwnerUserId = PP.OwnerUserId JOIN RecentVotes RV ON PP.PostId = RV.PostId
// WHERE UR.ReputationRank <= 100 ORDER BY PP.PopularityRank, UR.Reputation DESC;
fn q7845(db: &'static So) -> String {
    let rep = |u: Id<User>| db.user.reputation.get(u).unwrap();
    let uw = whole(db.user.iq()).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r)| Reverse(r), asc);
    let ur: MatSet<Id<User>> = (&uw).filt(|(_, r)| r <= 100).map(|((u, _), _)| u).collect();
    let Post { creation_date, score, view_count, owner_user, .. } = &db.post;
    let month = add_days(date(2024, 10, 1), -30);
    let upc = db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1))).group_by(owner_user).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let recent = || db.post.with(creation_date.ge(month));
    let cc = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let pw = whole(&cc).select(Ident::<Post>::new().and(&cc).and(score.and(view_count.opt()))).window(rank, |(_, (s, w))| (Reverse(s), w.is_none(), Reverse(w)), asc);
    type PR = (Id<Post>, (i64, i64));
    let pr: MatSet<PR> = (&pw).map(|(((p, c), _), r)| (p, (c, r))).collect();
    let rv = db.vote.with((&db.vote.creation_date).ge(month)).group_by(&db.vote.post).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let mut v = drain((&pr).select(Same::<PR>::new().and(Same::<PR>::new().map(|(p, _): PR| p).select(Ident::<Post>::new().and(&rv).and(owner_user.select(Ident::<User>::new().with(&ur).and(&upc)))))));
    v.sort_by_key(|&(_, ((_, (_, r)), (_, (u, _))))| (r, Reverse(rep(u))));
    rows(v.into_iter().map(|(_, ((_, (c, r)), ((p, n), (u, k))))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(k));
        f.extend(post_fields(db, p, &["title", "score"]));
        f.extend([V::I(c), V::I(n), V::I(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, p.CommentCount,
//        array_length(string_to_array(substring(p.Tags, 2, length(p.Tags) - 2), '><'), 1) AS TagCount, u.DisplayName AS OwnerDisplayName, u.Reputation AS OwnerReputation,
//        RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS RankByScore
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT PostId, Title, Body, CreationDate, ViewCount, Score, AnswerCount, CommentCount, TagCount, OwnerDisplayName, OwnerReputation FROM RankedPosts WHERE RankByScore <= 5)
// SELECT tp.Title, tp.Body, tp.ViewCount, tp.Score, tp.AnswerCount, tp.CommentCount, tp.TagCount, tp.OwnerDisplayName, tp.OwnerReputation, COUNT(c.Id) AS TotalComments, SUM(b.Class) AS TotalBadges
// FROM TopPosts tp LEFT JOIN Comments c ON c.PostId = tp.PostId LEFT JOIN Badges b ON b.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = tp.PostId)
// GROUP BY tp.PostId, tp.Title, tp.Body, tp.ViewCount, tp.Score, tp.AnswerCount, tp.CommentCount, tp.TagCount, tp.OwnerDisplayName, tp.OwnerReputation ORDER BY tp.Score DESC, tp.ViewCount DESC;
fn q26361(db: &'static So) -> String {
    let Post { creation_date, owner_user, post_type_id, score, view_count, tags_str, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score))
        .window(rank, |(_, s)| Reverse(s), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(owner_user.select(badges_of(db).select(&db.badge.class)).opt()))
        .fold((0i64, None), |(n, b): (i64, Option<i64>), (c, x)| (n + c.is_some() as i64, match x {
            Some(x) => Some(b.unwrap_or(0) + x),
            None => b,
        }));
    let mut v = drain(&s);
    v.sort_by_key(|&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    });
    rows(v.into_iter().map(|(p, (n, b))| {
        let mut f = post_fields(db, p, &["title", "body", "views", "score", "answers", "comments"]);
        f.push(tags_str.get(p).map_or(V::Null, |t| V::I(t[1..t.len() - 1].split("><").count() as i64)));
        f.extend(post_fields(db, p, &["owner", "rep"]));
        f.extend([V::I(n), b.map_or(V::Null, V::I)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, u.DisplayName AS Author, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, SUM(CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN 1 ELSE 0 END) AS CloseReopenedEvents,
//        ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS RowNum
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Body, p.CreationDate, u.DisplayName),
// FilteredPosts AS (SELECT PostId, Title, Body, CreationDate, Author, CommentCount, UpVotes, DownVotes, CloseReopenedEvents FROM RankedPosts WHERE RowNum = 1)
// SELECT f.PostId, f.Title, f.Author, f.CreationDate, f.CommentCount, f.UpVotes, f.DownVotes, f.CloseReopenedEvents,
//        CASE WHEN f.UpVotes > f.DownVotes THEN 'Positive Outweighs' WHEN f.UpVotes < f.DownVotes THEN 'Negative Outweighs' ELSE 'Neutral' END AS OverallVoteBias
// FROM FilteredPosts f WHERE f.CommentCount > 5 ORDER BY f.UpVotes DESC, f.CommentCount DESC;
fn q26806(db: &'static So) -> String {
    let s = db
        .post
        .with((&db.post.post_type_id).eq(1))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(history_of(db).select(&db.post_history.post_history_type_id).opt()))
        .fold([0i64; 4], |a, ((c, t), h)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64, a[3] + matches!(h, Some(10) | Some(11)) as i64]);
    let mut v = drain((&s).filt(|a| a[0] > 5));
    v.sort_by_key(|&(_, a)| (Reverse(a[1]), Reverse(a[0])));
    rows(v.into_iter().map(|(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created"]);
        f.extend(a.map(V::I));
        f.push(V::S(if a[1] > a[2] { "Positive Outweighs" } else if a[1] < a[2] { "Negative Outweighs" } else { "Neutral" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.Score, p.AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.PostTypeId IN (1, 2)),
// PopularUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(p.ViewCount) AS TotalViews, SUM(p.Score) AS TotalScore FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId
//     WHERE p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY u.Id, u.DisplayName HAVING SUM(p.ViewCount) > 1000),
// RecentActions AS (SELECT ph.PostId, COUNT(*) AS EditCount, MAX(ph.CreationDate) AS LastEdit FROM PostHistory ph
//     WHERE ph.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '6 months' AND ph.PostHistoryTypeId IN (4, 5, 6) GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.ViewCount, rp.Score, rp.AnswerCount, pu.DisplayName AS TopUser, pu.TotalViews, pu.TotalScore, ra.EditCount, ra.LastEdit
// FROM RankedPosts rp JOIN PopularUsers pu ON pu.UserId = (SELECT p.OwnerUserId FROM Posts p WHERE p.Id = rp.PostId) LEFT JOIN RecentActions ra ON ra.PostId = rp.PostId
// WHERE rp.Rank = 1 ORDER BY rp.Score DESC, rp.ViewCount DESC LIMIT 10;
fn q9377(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, owner_user_id, score, view_count, .. } = &db.post;
    let cut = add_years(ts(2024, 10, 1, 12, 34, 56), -1);
    let key = |p: Id<Post>| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    };
    let pu = db.post.with(creation_date.gt(cut)).group_by(owner_user).select(score.and(view_count.opt())).fold((0i64, 0i64, 0i64), |(c, w, s), (x, y)| (c + y.is_some() as i64, w + y.unwrap_or(0), s + x));
    let w = db
        .post
        .with(creation_date.gt(cut).and(post_type_id.is_in([1, 2])))
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(score.and(view_count.opt())))
        .window(row_number, |(p, (s, w))| (Reverse(s), w.is_none(), Reverse(w), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let ra = db.post_history.with(hd.gt(add_months(ts(2024, 10, 1, 12, 34, 56), -6)).and(post_history_type_id.is_in([4, 5, 6]))).group_by(post).select(hd).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let v = drain((&tp).select(Ident::<Post>::new().and(owner_user.select(Ident::<User>::new().and((&pu).filt(|(c, w, _)| c > 0 && w > 1000)))).and((&ra).opt())));
    let v = top_n(v, |&(_, ((p, _), _))| (key(p), p), 10);
    rows(v.into_iter().map(|(_, ((p, (u, (_, w, s))), r))| {
        let mut f = post_fields(db, p, &["id", "title", "views", "score", "answers"]);
        f.extend([user_col(db, u, "name"), V::I(w), V::I(s)]);
        f.extend(match r {
            Some((n, m)) => [V::I(n), V::T(m)],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH UserPostCounts AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositivePostCount,
//        SUM(CASE WHEN p.Score < 0 THEN 1 ELSE 0 END) AS NegativePostCount FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, PositivePostCount, NegativePostCount, RANK() OVER (ORDER BY PostCount DESC) AS UserRank FROM UserPostCounts WHERE PostCount > 0),
// RecentPostHistory AS (SELECT ph.PostId, ph.UserId, ph.CreationDate, ph.Comment, p.Title, ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS RecentEdit
//     FROM PostHistory ph JOIN Posts p ON ph.PostId = p.Id WHERE ph.PostHistoryTypeId IN (4, 5, 6))
// SELECT tu.DisplayName, tu.PostCount AS TotalPosts, tu.PositivePostCount AS PostsWithPositiveScore, tu.NegativePostCount AS PostsWithNegativeScore, COUNT(rph.PostId) AS RecentEditsCount,
//        MIN(rph.CreationDate) AS FirstRecentEditDate, MAX(rph.CreationDate) AS LastRecentEditDate
// FROM TopUsers tu LEFT JOIN RecentPostHistory rph ON tu.UserId = rph.UserId WHERE tu.UserRank <= 10
// GROUP BY tu.UserId, tu.DisplayName, tu.PostCount, tu.PositivePostCount, tu.NegativePostCount, tu.UserRank ORDER BY tu.UserRank;
fn q8922(db: &'static So) -> String {
    let ups = user_posts(db);
    let negs = db.user.group_by(Ident::<User>::new()).select(posts_of(db).with((&db.post.score).lt(0))).fold(0i64, |n, _| n + 1);
    let w = whole(db.user.iq()).select(Ident::<User>::new().and((&ups).filt(|a| a[1] > 0))).window(rank, |(_, a)| Reverse(a[1]), asc);
    type T = ((Id<User>, [i64; 10]), i64);
    let tu: MatSet<T> = (&w).filt(|(_, r)| r <= 10).collect();
    let PostHistory { post_history_type_id, user, creation_date: hd, .. } = &db.post_history;
    let rph = db.post_history.with(post_history_type_id.is_in([4, 5, 6])).group_by(user).select(hd).fold((0i64, i64::MAX, i64::MIN), |(n, a, b), d| (n + 1, a.min(d), b.max(d)));
    let mut v = drain((&tu).select(Same::<T>::new().and(Same::<T>::new().map(|((u, _), _): T| u).select((&rph).opt().and((&negs).opt())))));
    v.sort_by_key(|&(_, ((_, r), _))| r);
    rows(v.into_iter().map(|(_, (((u, a), _), (h, n)))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[1]), V::I(a[8]), V::I(n.unwrap_or(0))];
        f.extend(match h {
            Some((c, lo, hi)) => [V::I(c), V::T(lo), V::T(hi)],
            None => [V::I(0), V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH TagAnalysis AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COUNT(DISTINCT c.Id) AS CommentCount, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty, STRING_AGG(DISTINCT t.TagName, ', ') AS Tags
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 9 LEFT JOIN Tags t ON t.ExcerptPostId = p.Id OR t.WikiPostId = p.Id
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate),
// UserBadgeStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT b.Id) AS BadgeCount, SUM(u.Reputation) AS TotalReputation FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PostHistoryStats AS (SELECT p.Id AS PostId, COUNT(h.Id) AS HistoryCount, MAX(h.CreationDate) AS LastHistoryDate FROM Posts p LEFT JOIN PostHistory h ON p.Id = h.PostId GROUP BY p.Id)
// SELECT ta.PostId, ta.Title, ta.CreationDate, ta.CommentCount, ta.TotalBounty, ta.Tags, ubs.DisplayName, ubs.BadgeCount, ubs.TotalReputation, phs.HistoryCount, phs.LastHistoryDate
// FROM TagAnalysis ta JOIN Users u ON u.Id = ta.PostId JOIN UserBadgeStats ubs ON u.Id = ubs.UserId JOIN PostHistoryStats phs ON ta.PostId = phs.PostId
// ORDER BY ta.CommentCount DESC, ubs.TotalReputation DESC;
//
// Questions are never a tag's excerpt or wiki post, so Tags aggregates only
// NULLs.
fn q28597(db: &'static So) -> String {
    let Post { post_type_id, origid, .. } = &db.post;
    let uids: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let qs: MatSet<Id<Post>> = db.post.with(post_type_id.eq(1)).with(origid.select(&uids)).collect();
    let Tag { excerpt_post, wiki_post, .. } = &db.tag;
    type TP = (Id<Tag>, Id<Post>);
    let tg: MatSet<TP> = db.tag.select(Ident::<Tag>::new().and(excerpt_post)).union(db.tag.select(Ident::<Tag>::new().and(wiki_post))).collect();
    let tag_of: HashIdx<Id<Post>, Str> = (&tg).map(|(_, p)| p).inv().select(&tg).map(|(t, _)| t).select(&db.tag.tag_name).collect();
    let bounty: HashIdx<Id<Post>, Id<Vote>> = db.vote.with((&db.vote.vote_type_id).eq(9)).select(&db.vote.post).inv().collect();
    let g = || (&qs).group_by(Ident::<Post>::new());
    let dc = g().select(comments_of(db)).count_distinct();
    let tb = g()
        .select(comments_of(db).opt().and((&bounty).select((&db.vote.bounty_amount).opt()).opt()).and((&tag_of).opt()))
        .fold(0i64, |s, ((_, b), _)| s + b.flatten().unwrap_or(0));
    let names = g().select(&tag_of).buf_fold(|it| {
        let mut n: Vec<Str> = it.into_iter().collect();
        n.sort_unstable();
        n.dedup();
        &*Box::leak(n.join(", ").into_boxed_str())
    });
    let ubr = db.user.group_by(Ident::<User>::new()).select((&db.user.reputation).and(badges_of(db).opt())).fold(0i64, |r, (x, _)| r + x);
    let ubn = db.user.group_by(Ident::<User>::new()).select(badges_of(db)).count_distinct();
    let phs = g().select(history_of(db).select(&db.post_history.creation_date).opt()).fold((0i64, None), |(n, m): (i64, Option<i64>), d| (n + d.is_some() as i64, match d {
        Some(d) => Some(m.map_or(d, |m| m.max(d))),
        None => m,
    }));
    let mut v = drain((&qs).select((&dc).opt().and(&tb).and((&names).opt()).and(&phs).and(origid.select(&uids).select(Ident::<User>::new().and((&ubn).opt()).and(&ubr)))));
    v.sort_by_key(|&(_, ((((c, _), _), _), (_, r)))| (Reverse(c.unwrap_or(0)), Reverse(r)));
    rows(v.into_iter().map(|(p, ((((c, b), t), (hn, hm)), ((u, bn), r)))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(c.unwrap_or(0)), V::I(b), t.map_or(V::Null, V::S), user_col(db, u, "name"), V::I(bn.unwrap_or(0)), V::I(r), V::I(hn), hm.map_or(V::Null, V::T)]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers, SUM(CASE WHEN p.PostTypeId = 1 THEN p.AnswerCount ELSE 0 END) AS TotalAnswersToQuestions,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId WHERE u.Reputation > 0 GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT ..., RANK() OVER (ORDER BY TotalPosts DESC) AS PostRank, RANK() OVER (ORDER BY TotalUpvotes DESC) AS UpvoteRank FROM UserActivity)
// SELECT UserId, DisplayName, TotalPosts, Questions, Answers, TotalAnswersToQuestions, TotalUpvotes, TotalDownvotes, PostRank, UpvoteRank,
//        CASE WHEN PostRank = 1 AND UpvoteRank = 1 THEN 'Top Contributor' WHEN PostRank <= 10 THEN 'Top Posts Contributor' WHEN UpvoteRank <= 10 THEN 'Top Voted Contributor' ELSE 'Regular Contributor' END AS ContributorLevel
// FROM TopUsers WHERE TotalPosts > 10 ORDER BY PostRank, UpvoteRank;
fn q7890(db: &'static So) -> String {
    let Post { post_type_id, answer_count, .. } = &db.post;
    let s = db
        .user
        .with((&db.user.reputation).gt(0))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(answer_count.opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold(([0i64; 5], None), |(a, t): ([i64; 5], Option<i64>), p| match p {
            Some(((ty, n), v)) => {
                let x = if ty == 1 { n } else { Some(0) };
                ([a[0] + 1, a[1] + (ty == 1) as i64, a[2] + (ty == 2) as i64, a[3] + (v == Some(2)) as i64, a[4] + (v == Some(3)) as i64], match x {
                    Some(x) => Some(t.unwrap_or(0) + x),
                    None => t,
                })
            }
            None => (a, Some(t.unwrap_or(0))),
        });
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&s)).window(rank, |(_, (a, _))| Reverse(a[0]), asc);
    let w = (&w).window(rank, |((_, (a, _)), _)| Reverse(a[3]), asc);
    let mut v: Vec<_> = drain((&w).filt(|(((_, (a, _)), _), _)| a[0] > 10)).into_iter().map(|x| x.1).collect();
    v.sort_by_key(|&((_, p), q)| (p, q));
    rows(v.into_iter().map(|(((u, (a, t)), p), q)| {
        let lvl = if p == 1 && q == 1 { "Top Contributor" } else if p <= 10 { "Top Posts Contributor" } else if q <= 10 { "Top Voted Contributor" } else { "Regular Contributor" };
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), t.map_or(V::Null, V::I), V::I(a[3]), V::I(a[4]), V::I(p), V::I(q), V::S(lvl)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, COALESCE(up.UpVoteCount, 0) AS UpVoteCount, COALESCE(down.DownVoteCount, 0) AS DownVoteCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS Rank, p.OwnerUserId
//     FROM Posts p LEFT JOIN (SELECT PostId, COUNT(*) AS UpVoteCount FROM Votes WHERE VoteTypeId = 2 GROUP BY PostId) up ON p.Id = up.PostId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS DownVoteCount FROM Votes WHERE VoteTypeId = 3 GROUP BY PostId) down ON p.Id = down.PostId),
// PostedBadges AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount FROM Badges b JOIN Users u ON b.UserId = u.Id WHERE u.Reputation > 1000 GROUP BY b.UserId),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, pb.BadgeCount FROM Users u LEFT JOIN PostedBadges pb ON u.Id = pb.UserId WHERE u.Reputation > 1000)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, rp.UpVoteCount, rp.DownVoteCount, tu.DisplayName AS TopUser, tu.Reputation AS UserReputation, tu.BadgeCount
// FROM RankedPosts rp LEFT JOIN TopUsers tu ON rp.OwnerUserId = tu.UserId WHERE rp.Rank <= 10 ORDER BY rp.CreationDate DESC;
fn q6695(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let w = db.post.group_by(post_type_id).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|((p, _), _)| p).collect();
    let ud = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold((0i64, 0i64), |(u, d), t| (u + (t == Some(2)) as i64, d + (t == Some(3)) as i64));
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let tu = Ident::<User>::new().with((&db.user.reputation).gt(1000)).and((&bc).opt());
    let mut v = drain((&ud).and(owner_user.select(tu).opt()));
    v.sort_by_key(|&(p, _)| Reverse(creation_date.get(p).unwrap()));
    rows(v.into_iter().map(|(p, ((u, d), t))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers"]);
        f.extend([V::I(u), V::I(d)]);
        f.extend(match t {
            Some((u, b)) => [user_col(db, u, "name"), user_col(db, u, "rep"), b.map_or(V::Null, V::I)],
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY SUBSTRING(p.Tags, 2, LENGTH(p.Tags) - 2) ORDER BY p.CreationDate DESC) AS TagRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.CreationDate >= CURRENT_DATE - INTERVAL '1 year'),
// TopRepliedToPosts AS (SELECT rp.PostId, rp.Title, rp.OwnerDisplayName, COUNT(c.Id) AS CommentCount FROM RankedPosts rp LEFT JOIN Comments c ON c.PostId = rp.PostId
//     WHERE rp.TagRank = 1 GROUP BY rp.PostId, rp.Title, rp.OwnerDisplayName HAVING COUNT(c.Id) > 5),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, u.DisplayName FROM Users u WHERE u.Reputation > 1000),
// PopularBadges AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount FROM Badges b GROUP BY b.UserId HAVING COUNT(b.Id) >= 3)
// SELECT trp.Title, trp.OwnerDisplayName, trp.CommentCount, ur.DisplayName AS ActiveUserDisplayName, ur.Reputation, pb.BadgeCount
// FROM TopRepliedToPosts trp JOIN UserReputation ur ON trp.OwnerDisplayName = ur.DisplayName LEFT JOIN PopularBadges pb ON ur.UserId = pb.UserId
// ORDER BY trp.CommentCount DESC, ur.Reputation DESC;
//
// CURRENT_DATE is the day the query runs; the data ends in 2024, so it is empty.
fn q27030(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, tags_str, .. } = &db.post;
    let w = db
        .post
        .with(post_type_id.eq(1).and(creation_date.ge(add_years(current_date(), -1))))
        .with(owner_user)
        .group_by(tags_str.opt())
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ur: HashIdx<Str, Id<User>> = db.user.with((&db.user.reputation).gt(1000)).select(&db.user.display_name).inv().collect();
    let pb = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let mut v = drain((&cc).filt(|n| n > 5).and(owner_user.select(&db.user.display_name).select(&ur).select(Ident::<User>::new().and((&pb).filt(|n| n >= 3).opt()))));
    v.sort_by_key(|&(_, (n, (u, _)))| (Reverse(n), Reverse(db.user.reputation.get(u).unwrap())));
    rows(v.into_iter().map(|(p, (n, (u, b)))| {
        let mut f = post_fields(db, p, &["title", "owner"]);
        f.push(V::I(n));
        f.extend(ucols(db, u, &["name", "rep"]));
        f.push(b.map_or(V::Null, V::I));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// PostStats AS (SELECT p.OwnerUserId, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
//        SUM(p.ViewCount) AS TotalViews, AVG(p.Score) AS AverageScore FROM Posts p GROUP BY p.OwnerUserId),
// QualifiedUsers AS (SELECT ur.UserId, ur.DisplayName, ur.Reputation, ur.BadgeCount, ps.TotalPosts, ps.Questions, ps.Answers, ps.TotalViews, ps.AverageScore
//     FROM UserReputation ur JOIN PostStats ps ON ur.UserId = ps.OwnerUserId WHERE ur.Reputation > 100 AND ps.TotalPosts > 10),
// FinalReport AS (SELECT q.DisplayName, q.Reputation, q.BadgeCount, q.TotalPosts, q.Questions, q.Answers, q.TotalViews, q.AverageScore, RANK() OVER (ORDER BY q.Reputation DESC) AS ReputationRank FROM QualifiedUsers q)
// SELECT DisplayName, Reputation, BadgeCount, TotalPosts, Questions, Answers, TotalViews, AverageScore, ReputationRank FROM FinalReport WHERE ReputationRank <= 10 ORDER BY Reputation DESC;
fn q5018(db: &'static So) -> String {
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let ups = user_posts(db);
    let w = whole(db.user.iq())
        .select(Ident::<User>::new().with((&db.user.reputation).gt(100)).and(&db.user.reputation).and(&bc).and((&ups).filt(|a| a[1] > 10)))
        .window(rank, |(((_, r), _), _)| Reverse(r), asc);
    rows(drain((&w).filt(|(_, r)| r <= 10)).into_iter().map(|x| x.1).map(|((((u, _), b), a), r)| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(b), V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[6], a[5]), avg(a[4], a[1]), V::I(r)]);
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS PostCount, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews, SUM(COALESCE(p.Score, 0)) AS TotalScore,
//        AVG(COALESCE(p.ViewCount, 0)) AS AvgViews, ROW_NUMBER() OVER (PARTITION BY u.Id ORDER BY SUM(COALESCE(p.ViewCount, 0)) DESC) AS Rank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// PopularPosts AS (SELECT p.Id, p.Title, p.ViewCount, p.Score, p.CreationDate, u.DisplayName AS OwnerDisplayName, ROW_NUMBER() OVER (ORDER BY p.Score DESC, p.ViewCount DESC) AS PopularityRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.Score > 0),
// PostHistoryLatest AS (SELECT ph.PostId, ph.CreationDate, RANK() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS LatestRank FROM PostHistory ph)
// SELECT ups.DisplayName, ups.PostCount, ups.TotalViews, ups.TotalScore, ups.AvgViews, pp.Title, pp.ViewCount, pp.Score, pp.CreationDate, pp.OwnerDisplayName, phl.CreationDate AS LatestChangeDate
// FROM UserPostStats ups LEFT JOIN PopularPosts pp ON pp.PopularityRank <= 10 LEFT JOIN PostHistoryLatest phl ON pp.Id = phl.PostId AND phl.LatestRank = 1
// WHERE (ups.PostCount > 10 OR ups.TotalScore > 100) ORDER BY ups.TotalViews DESC, ups.TotalScore DESC;
fn q962(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, owner_user, .. } = &db.post;
    let ups = user_posts(db);
    let pp = top_n(drain(db.post.with(post_type_id.eq(1).and(score.gt(0))).with(owner_user).select(score)), |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 10);
    let PostHistory { post, creation_date: hd, .. } = &db.post_history;
    let md = db.post_history.group_by(post).select(hd).fold(i64::MIN, |m, d| m.max(d));
    let at: HashIdx<(Id<Post>, i64), Id<PostHistory>> = db.post_history.select(post.and(hd)).inv().collect();
    let tp: MatSet<Id<Post>> = rel(pp.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    type L = (Id<Post>, Option<Id<PostHistory>>);
    let ls: MatSet<L> = (&tp).select(Ident::<Post>::new().and(Ident::<Post>::new().and(&md).select(&at).opt())).collect();
    let lat: HashIdx<(), L> = (&ls).map(|_| ()).inv().collect();
    let mut v = drain((&ups).filt(|a| a[1] > 10 || a[4] > 100).and(Ident::<User>::new().map(|_| ()).select(&lat).opt()));
    v.sort_by_key(|&(u, (a, _))| (Reverse(a[6]), Reverse(a[4]), u));
    rows(v.into_iter().map(|(u, (a, x))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[1]), V::I(a[6]), V::I(a[4]), if a[1] == 0 { V::F(0.0) } else { avg(a[6], a[1]) }];
        match x {
            Some((p, h)) => {
                f.extend(post_fields(db, p, &["title", "views", "score", "created", "owner"]));
                f.push(h.map_or(V::Null, |h| V::T(hd.get(h).unwrap())));
            }
            None => f.extend([V::Null, V::Null, V::Null, V::Null, V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH UserPostStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, COUNT(CASE WHEN P.Score > 0 THEN 1 END) AS PositiveScorePosts, COUNT(CASE WHEN P.Score < 0 THEN 1 END) AS NegativeScorePosts,
//        AVG(P.ViewCount) AS AvgViewCount, AVG(P.AnswerCount) AS AvgAnswerCount FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT ..., RANK() OVER (ORDER BY TotalPosts DESC) AS PostRank, RANK() OVER (ORDER BY PositiveScorePosts DESC) AS PosScoreRank FROM UserPostStats WHERE TotalPosts > 0)
// SELECT U.DisplayName, U.TotalPosts, U.TotalQuestions, U.TotalAnswers, U.PositiveScorePosts, U.NegativeScorePosts, U.AvgViewCount, U.AvgAnswerCount, PHT.Name AS PostHistoryType,
//        PH.CreationDate AS HistoryDate, PH.Comment
// FROM TopUsers U LEFT JOIN Posts P ON U.UserId = P.OwnerUserId LEFT JOIN PostHistory PH ON P.Id = PH.PostId LEFT JOIN PostHistoryTypes PHT ON PH.PostHistoryTypeId = PHT.Id
// WHERE U.PostRank <= 10 OR U.PosScoreRank <= 10 ORDER BY U.PostRank, U.PosScoreRank;
fn q28529(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, answer_count, .. } = &db.post;
    let s = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(view_count.opt()).and(answer_count.opt())))
        .fold([0i64; 9], |a, (((t, s), w), n)| {
            [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (s > 0) as i64, a[4] + (s < 0) as i64, a[5] + w.is_some() as i64, a[6] + w.unwrap_or(0), a[7] + n.is_some() as i64, a[8] + n.unwrap_or(0)]
        });
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&s)).window(rank, |(_, a)| Reverse(a[0]), asc);
    let w = (&w).window(rank, |((_, a), _)| Reverse(a[3]), asc);
    type T = (((Id<User>, [i64; 9]), i64), i64);
    let tu: MatSet<T> = (&w).filt(|((_, p), q)| p <= 10 || q <= 10).collect();
    let ph = history_of(db).select(Ident::<PostHistory>::new().and(htype_name(db))).opt();
    let mut v = drain((&tu).select(Same::<T>::new().and(Same::<T>::new().map(|(((u, _), _), _): T| u).select(posts_of(db).select(ph).opt()))));
    v.sort_by_key(|&(_, (((_, p), q), _))| (p, q));
    rows(v.into_iter().map(|(_, ((((u, a), _), _), h))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4]), avg(a[6], a[5]), avg(a[8], a[7])];
        f.extend(match h.flatten() {
            Some((h, n)) => [V::S(n),V::T(db.post_history.creation_date.get(h).unwrap()), harness::fmt::ostr(db.post_history.comment.get(h))],
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH TagStats AS (SELECT T.TagName, COUNT(P.Id) AS PostCount, SUM(P.ViewCount) AS TotalViews, AVG(P.Score) AS AverageScore FROM Tags T JOIN Posts P ON P.Tags LIKE '%' || T.TagName || '%'
//     WHERE P.PostTypeId = 1 GROUP BY T.TagName),
// UserStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS QuestionsAnswered, SUM(CASE WHEN P.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers,
//        SUM(CASE WHEN P.Score > 0 THEN P.Score ELSE 0 END) AS TotalPositiveScore, AVG(P.Score) AS AverageScore FROM Users U JOIN Posts P ON U.Id = P.OwnerUserId WHERE P.PostTypeId = 2 GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, QuestionsAnswered, AcceptedAnswers, TotalPositiveScore, AverageScore, RANK() OVER (ORDER BY TotalPositiveScore DESC) AS ScoreRank FROM UserStats),
// RankedTags AS (SELECT TagName, PostCount, TotalViews, AverageScore, RANK() OVER (ORDER BY TotalViews DESC) AS ViewRank FROM TagStats)
// SELECT U.DisplayName AS TopUser, U.QuestionsAnswered, U.AcceptedAnswers, U.TotalPositiveScore, T.TagName AS PopularTag, T.PostCount, T.TotalViews, T.AverageScore
// FROM TopUsers U JOIN RankedTags T ON T.ViewRank <= 5 WHERE U.ScoreRank <= 10 ORDER BY U.TotalPositiveScore DESC, T.TotalViews DESC;
fn q28516(db: &'static So) -> String {
    let lt = tag_mentions(db);
    let Post { post_type_id, view_count, score, accepted_answer_id, .. } = &db.post;
    type P = (Id<Post>, Id<Tag>);
    type PN = (Id<Post>, Str);
    let pn: MatSet<PN> = (&lt).select(Same::<P>::new().map(|(p, _): P| p).and(Same::<P>::new().map(|(_, t): P| t).select(&db.tag.tag_name))).collect();
    let ts_ = (&pn)
        .group_by(Same::<PN>::new().map(|(_, n): PN| n))
        .select(Same::<PN>::new().map(|(p, _): PN| p).select(Ident::<Post>::new().with(post_type_id.eq(1))).select(view_count.opt().and(score)))
        .fold([0i64; 4], |a, (w, s)| [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s]);
    let tw = whole(&ts_).select(Same::<Str>::new().and(&ts_)).window(rank, |(_, a)| (a[1] == 0, Reverse(a[2])), asc);
    let tt: MatSet<(Str, [i64; 4])> = (&tw).filt(|(_, r)| r <= 5).map(|(x, _)| x).collect();
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).with(post_type_id.eq(2)).select(score.and(accepted_answer_id.opt())))
        .fold([0i64; 3], |a, (s, x)| [a[0] + 1, a[1] + x.is_some() as i64, a[2] + s.max(0)]);
    let uw = whole(&us).select(Ident::<User>::new().and(&us)).window(rank, |(_, a)| Reverse(a[2]), asc);
    let tu: MatSet<(Id<User>, [i64; 3])> = (&uw).filt(|(_, r)| r <= 10).map(|(x, _)| x).collect();
    let mut v = drain((&tu).cross(&tt));
    v.sort_by_key(|&(_, ((u, a), (_, b)))| (Reverse(a[2]), u, b[1] == 0, Reverse(b[2])));
    rows(v.into_iter().map(|(_, ((u, a), (t, b)))| {
        row(vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::S(t), V::I(b[0]), nullable(b[2], b[1]), avg(b[3], b[0])])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, u.DisplayName AS Owner, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS PostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= '2023-01-01' AND p.Score > 10),
// TopRankedPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.Owner, ph.UserDisplayName AS LastEditor, ph.CreationDate AS LastEditDate FROM RankedPosts rp
//     LEFT JOIN PostHistory ph ON rp.PostId = ph.PostId AND ph.CreationDate = (SELECT MAX(CreationDate) FROM PostHistory WHERE PostId = rp.PostId) WHERE rp.PostRank <= 5),
// PostDetails AS (SELECT trp.PostId, trp.Title, trp.Score, trp.Owner, trp.LastEditor, trp.LastEditDate, COUNT(c.Id) AS CommentCount, COUNT(v.Id) AS VoteCount
//     FROM TopRankedPosts trp LEFT JOIN Comments c ON trp.PostId = c.PostId LEFT JOIN Votes v ON trp.PostId = v.PostId GROUP BY trp.PostId, trp.Title, trp.Score, trp.Owner, trp.LastEditor, trp.LastEditDate)
// SELECT pd.PostId, pd.Title, pd.Score, pd.Owner, pd.LastEditor, pd.LastEditDate, pd.CommentCount, pd.VoteCount, COALESCE(ROUND(pd.Score * 1.0 / NULLIF(pd.CommentCount + 1, 0), 2), 0) AS ScorePerComment
// FROM PostDetails pd ORDER BY pd.Score DESC, pd.PostId ASC;
fn q6820(db: &'static So) -> String {
    let Post { creation_date, score, owner_user, post_type_id, origid, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(date(2023, 1, 1)).and(score.gt(10)))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|(((p, _), _), _)| p).collect();
    let PostHistory { post, creation_date: hd, user_display_name, .. } = &db.post_history;
    let md = db.post_history.group_by(post).select(hd).fold(i64::MIN, |m, d| m.max(d));
    let at: HashIdx<(Id<Post>, i64), Id<PostHistory>> = db.post_history.select(post.and(hd)).inv().collect();
    type A = (Id<Post>, Option<((Id<PostHistory>, Option<Str>), i64)>);
    let anchors: MatSet<A> = (&tp)
        .select(Ident::<Post>::new().and(Ident::<Post>::new().and(&md).select(&at).select(Ident::<PostHistory>::new().and(user_display_name.opt()).and(hd)).opt()))
        .collect();
    let g = (&anchors)
        .group_by(Same::<A>::new().map(|(p, h): A| (p, h.and_then(|x| x.0 .1), h.map(|x| x.1))))
        .select(Same::<A>::new().map(|(p, _): A| p).select(comments_of(db).opt().and(votes_of(db).opt())))
        .fold((0i64, 0i64), |(n, m), (c, v)| (n + c.is_some() as i64, m + v.is_some() as i64));
    let mut v = drain(&g);
    v.sort_by_key(|&((p, _, _), _)| (Reverse(score.get(p).unwrap()), origid.get(p).unwrap()));
    rows(v.into_iter().map(|((p, e, d), (n, m))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "score", "owner"]);
        f.extend([e.map_or(V::Null, V::S), d.map_or(V::Null, V::T), V::I(n), V::I(m), V::F((s as f64 / (n + 1) as f64 * 100.0).round() / 100.0)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Body, COALESCE(UP.AnswerCount, 0) AS AnswerCount, COALESCE(CS.CommentCount, 0) AS CommentCount,
//        COALESCE(V.TotalVotes, 0) AS TotalVotes, ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN (SELECT ParentId, COUNT(*) AS AnswerCount FROM Posts WHERE PostTypeId = 2 GROUP BY ParentId) AS UP ON p.Id = UP.ParentId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) AS CS ON p.Id = CS.PostId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS TotalVotes FROM Votes GROUP BY PostId) AS V ON p.Id = V.PostId WHERE p.PostTypeId = 1),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Body, rp.AnswerCount, rp.CommentCount, rp.TotalVotes FROM RankedPosts rp WHERE rp.Rank = 1)
// SELECT fp.PostId, fp.Title, fp.CreationDate, LENGTH(fp.Body) - LENGTH(REPLACE(fp.Body, ' ', '')) + 1 AS WordCount, fp.AnswerCount, fp.CommentCount, fp.TotalVotes
// FROM FilteredPosts fp WHERE fp.AnswerCount > 0 OR fp.CommentCount > 0 ORDER BY fp.TotalVotes DESC, fp.CreationDate DESC;
fn q28664(db: &'static So) -> String {
    let Post { post_type_id, creation_date, body, .. } = &db.post;
    let g = || db.post.with(post_type_id.eq(1)).group_by(Ident::<Post>::new());
    let ac = g().select(answers_of(db).opt()).fold(0i64, |n, x| n + x.is_some() as i64);
    let cc = g().select(comments_of(db).opt()).fold(0i64, |n, x| n + x.is_some() as i64);
    let vc = g().select(votes_of(db).opt()).fold(0i64, |n, x| n + x.is_some() as i64);
    let mut v = drain((&ac).and(&cc).and(&vc).filt(|((a, c), _)| a > 0 || c > 0));
    v.sort_by_key(|&(p, (_, n))| (Reverse(n), Reverse(creation_date.get(p).unwrap())));
    rows(v.into_iter().map(|(p, ((a, c), n))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.push(body.get(p).map_or(V::Null, |b| V::I(b.matches(' ').count() as i64 + 1)));
        f.extend([V::I(a), V::I(c), V::I(n)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, U.DisplayName AS OwnerDisplayName, P.CreationDate, P.ViewCount, P.Score, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS PostRank
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id WHERE P.PostTypeId IN (1, 2)),
// PostDetails AS (SELECT RP.PostId, RP.Title, RP.OwnerDisplayName, RP.CreationDate, RP.ViewCount, RP.Score, COUNT(C.Id) AS CommentCount,
//        SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
//     FROM RankedPosts RP LEFT JOIN Comments C ON RP.PostId = C.PostId LEFT JOIN Votes V ON RP.PostId = V.PostId WHERE RP.PostRank = 1
//     GROUP BY RP.PostId, RP.Title, RP.OwnerDisplayName, RP.CreationDate, RP.ViewCount, RP.Score),
// TopPosts AS (SELECT ..., RANK() OVER (ORDER BY PD.Score DESC, PD.ViewCount DESC) AS RankScore FROM PostDetails PD)
// SELECT TP.PostId, TP.Title, TP.OwnerDisplayName, TP.CreationDate, TP.ViewCount, TP.Score, TP.CommentCount, TP.UpVoteCount, TP.DownVoteCount FROM TopPosts TP WHERE TP.RankScore <= 10 ORDER BY TP.RankScore;
//
// The rank only reads Score and ViewCount, so the posts are picked before
// their joined rows are counted.
fn q9024(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, view_count, .. } = &db.post;
    let w = db.post.with(post_type_id.is_in([1, 2])).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let top: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let w2 = whole(&top).select(Ident::<Post>::new().and(score.and(view_count.opt()))).window(rank, |(_, (s, w))| (Reverse(s), w.is_none(), Reverse(w)), asc);
    let tp: MatSet<Id<Post>> = (&w2).filt(|(_, r)| r <= 10).map(|((p, _), _)| p).collect();
    let r: Vec<Id<Post>> = drain(&tp).into_iter().map(|x| x.1).collect();
    let s = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(r.into_iter().map(|p| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "views", "score"]);
        f.extend(s.get(p).unwrap().map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p INNER JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1
//     GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, u.DisplayName),
// PopularPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.OwnerDisplayName, rp.ViewCount, rp.CommentCount, rp.UpVotes, rp.DownVotes FROM RankedPosts rp WHERE rp.rn = 1 AND rp.ViewCount > 1000),
// FinalResults AS (SELECT pp.*, (pp.UpVotes - pp.DownVotes) AS NetVotes,
//        CASE WHEN pp.ViewCount > 5000 THEN 'Highly Viewed' WHEN pp.ViewCount > 1000 THEN 'Moderately Viewed' ELSE 'Low Visibility' END AS ViewCategory FROM PopularPosts pp)
// SELECT FR.PostId, FR.Title, FR.OwnerDisplayName, FR.CreationDate, FR.ViewCount, FR.CommentCount, FR.UpVotes, FR.DownVotes, FR.NetVotes, FR.ViewCategory FROM FinalResults FR ORDER BY FR.NetVotes DESC, FR.ViewCount DESC;
fn q6684(db: &'static So) -> String {
    let Post { post_type_id, owner_user, view_count, .. } = &db.post;
    let s = db
        .post
        .with(post_type_id.eq(1).and(view_count.gt(1000)))
        .with(owner_user)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let mut v = drain(&s);
    v.sort_by_key(|&(p, a)| (Reverse(a[1] - a[2]), Reverse(view_count.get(p))));
    rows(v.into_iter().map(|(p, a)| {
        let w = view_count.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "views"]);
        f.extend(a.map(V::I));
        f.push(V::I(a[1] - a[2]));
        f.push(V::S(if w > 5000 { "Highly Viewed" } else { "Moderately Viewed" }));
        row(f)
    }))
}

// WITH RecursiveTags AS (SELECT P.Id AS PostId, P.Title, P.Tags, unnest(string_to_array(substring(P.Tags, 2, length(P.Tags) - 2), '><')) AS TagName, P.CreationDate FROM Posts P WHERE P.PostTypeId = 1),
// TagCounts AS (SELECT TagName, COUNT(PostId) AS PostCount, MIN(CreationDate) AS FirstUsage FROM RecursiveTags GROUP BY TagName),
// TopTags AS (SELECT TagName, PostCount, FirstUsage FROM TagCounts ORDER BY PostCount DESC LIMIT 10),
// UserPostCounts AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(P.Id) AS PostCount, SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END) AS PositiveScoreCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId WHERE P.PostTypeId IN (1, 2) GROUP BY U.Id, U.DisplayName),
// UserTopContributors AS (SELECT UserId, DisplayName, PostCount, PositiveScoreCount FROM UserPostCounts WHERE PostCount > 0 ORDER BY PositiveScoreCount DESC LIMIT 5),
// FinalOutput AS (SELECT T.TagName, T.PostCount AS TagPostCount, U.DisplayName AS TopUser, U.PositiveScoreCount AS TopUserScores FROM TopTags T JOIN UserTopContributors U ON U.PostCount > T.PostCount)
// SELECT TagName, TagPostCount, TopUser, TopUserScores FROM FinalOutput ORDER BY TagPostCount DESC, TopUserScores DESC;
//
// RecursiveTags does not refer to itself; it is an ordinary CTE.
fn q25352(db: &'static So) -> String {
    let Post { post_type_id, tags_str, score, .. } = &db.post;
    let tc = db.post.with(post_type_id.eq(1)).select(tags_str.flat_map(tag_list)).group_by(Same::new()).fold(0i64, |n, _| n + 1);
    let tt = top_n(drain(&tc), |&(t, n)| (Reverse(n), t), 10);
    let upc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).with(post_type_id.is_in([1, 2])).select(score)).fold((0i64, 0i64), |(n, p), s| (n + 1, p + (s > 0) as i64));
    let tu = top_n(drain(&upc), |&(u, (_, p))| (Reverse(p), u), 5);
    let (tt, tu) = (rel(tt), rel(tu));
    let mut out: Vec<_> = drain((&tt).cross(&tu).filt(|((_, n), (_, (c, _)))| c > n)).into_iter().map(|(_, ((t, n), (u, (_, p))))| (t, n, u, p)).collect();
    out.sort_by_key(|&(_, n, _, p)| (Reverse(n), Reverse(p)));
    rows(out.into_iter().map(|(t, n, u, p)| row(vec![V::S(t), V::I(n), user_col(db, u, "name"), V::I(p)])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.AcceptedAnswerId, p.Score, p.ViewCount, p.Tags, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank,
//        COUNT(c.Id) AS CommentCount, p.OwnerUserId FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.PostTypeId = 1
//     GROUP BY p.Id, p.Title, p.AcceptedAnswerId, p.Score, p.ViewCount, p.Tags, p.CreationDate, p.OwnerUserId),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges, AVG(p.Score) AS AvgPostScore, SUM(p.ViewCount) AS TotalViews
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation)
// SELECT u.UserId, u.DisplayName, u.Reputation, u.BadgeCount, u.GoldBadges, u.SilverBadges, u.BronzeBadges, p.PostId, p.Title, p.AcceptedAnswerId, p.Score AS PostScore, p.ViewCount,
//        p.CreationDate, p.CommentCount, u.AvgPostScore, u.TotalViews
// FROM UserStats u JOIN RankedPosts p ON u.UserId = p.OwnerUserId WHERE p.PostRank <= 5 ORDER BY u.Reputation DESC, p.CreationDate DESC;
fn q27437(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, view_count, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1)).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let owners: MatSet<Id<User>> = (&tp).select(owner_user).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let us = (&owners)
        .group_by(Ident::<User>::new())
        .select(badges_of(db).select(&db.badge.class).opt().and(posts_of(db).select(score.and(view_count.opt())).opt()))
        .fold([0i64; 8], |a, (b, p)| {
            let (s, w) = p.map_or((None, None), |(s, w)| (Some(s), w));
            [a[0] + b.is_some() as i64, a[1] + (b == Some(1)) as i64, a[2] + (b == Some(2)) as i64, a[3] + (b == Some(3)) as i64, a[4] + s.is_some() as i64, a[5] + s.unwrap_or(0), a[6] + w.is_some() as i64, a[7] + w.unwrap_or(0)]
        });
    let mut v = drain((&cc).and(owner_user.select(Ident::<User>::new().and(&us))));
    v.sort_by_key(|&(p, (_, (u, _)))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(creation_date.get(p).unwrap())));
    rows(v.into_iter().map(|(p, (c, (u, a)))| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3])]);
        f.extend(post_fields(db, p, &["id", "title", "accepted", "score", "views", "created"]));
        f.extend([V::I(c), avg(a[5], a[4]), nullable(a[7], a[6])]);
        row(f)
    }))
}

// WITH TaggedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.Tags, ARRAY_LENGTH(string_to_array(substring(p.Tags, 2, length(p.Tags)-2), '><'), 1) AS TagCount,
//        u.DisplayName AS OwnerDisplayName, COALESCE(pc.CommentCount, 0) AS CommentCount, COALESCE(pa.AnswerCount, 0) AS AnswerCount, u.Reputation, pt.Name AS PostTypeName
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) pc ON p.Id = pc.PostId
//     LEFT JOIN (SELECT ParentId, COUNT(*) AS AnswerCount FROM Posts WHERE PostTypeId = 2 GROUP BY ParentId) pa ON p.Id = pa.ParentId JOIN PostTypes pt ON p.PostTypeId = pt.Id WHERE p.PostTypeId = 1),
// TopPosts AS (SELECT tp.PostId, tp.Title, tp.CreationDate, tp.TagCount, tp.OwnerDisplayName, tp.CommentCount, tp.AnswerCount, tp.Reputation, tp.PostTypeName,
//        ROW_NUMBER() OVER (ORDER BY tp.Reputation DESC, tp.CreationDate DESC) AS Rank FROM TaggedPosts tp WHERE tp.TagCount > 0)
// SELECT t.Title, t.OwnerDisplayName, t.CreationDate, t.TagCount, t.CommentCount, t.AnswerCount, t.Reputation, t.PostTypeName FROM TopPosts t WHERE t.Rank <= 10 ORDER BY t.Rank;
fn q25847(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, tags_str, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1)).with(tags_str).select(owner_user.and(creation_date)));
    let top = top_n(v, |&(p, (u, d))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(d), p), 10);
    let tp: MatSet<Id<Post>> = rel(top.iter().map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ac = (&tp).group_by(Ident::<Post>::new()).select(answers_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    rows(top.into_iter().map(|(p, (u, _))| {
        let t = tags_str.get(p).unwrap();
        let mut f = post_fields(db, p, &["title", "owner", "created"]);
        f.extend([V::I(t[1..t.len() - 1].split("><").count() as i64), V::I(cc.get(p).unwrap()), V::I(ac.get(p).unwrap()), user_col(db, u, "rep")]);
        f.extend(post_fields(db, p, &["type"]));
        row(f)
    }))
}

// WITH UserBadgeStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, BadgeCount, GoldBadges, SilverBadges, BronzeBadges, RANK() OVER (ORDER BY BadgeCount DESC) AS UserRank FROM UserBadgeStats WHERE BadgeCount > 0),
// PostDetails AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, COUNT(C.Id) AS CommentCount FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId WHERE P.PostTypeId = 1 GROUP BY P.Id, P.Title, P.CreationDate, P.Score),
// TopPosts AS (SELECT PD.PostId, PD.Title, PD.CreationDate, PD.Score, PD.CommentCount, RANK() OVER (ORDER BY PD.Score DESC) AS PostRank FROM PostDetails PD WHERE PD.CommentCount > 0)
// SELECT U.DisplayName AS TopUser, U.BadgeCount, U.GoldBadges, U.SilverBadges, U.BronzeBadges, P.Title AS TopPost, P.Score, P.CommentCount
// FROM TopUsers U JOIN TopPosts P ON P.CommentCount > 5 WHERE U.UserRank <= 10 AND P.PostRank <= 10 ORDER BY U.BadgeCount DESC, P.Score DESC;
fn q29080(db: &'static So) -> String {
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class)).fold([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    let uw = whole(&ub).select(Ident::<User>::new().and(&ub)).window(rank, |(_, a)| Reverse(a[0]), asc);
    let tu: MatSet<(Id<User>, [i64; 4])> = (&uw).filt(|(_, r)| r <= 10).map(|(x, _)| x).collect();
    let cc = db.post.with((&db.post.post_type_id).eq(1)).group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let score = |p: Id<Post>| db.post.score.get(p).unwrap();
    let pw = whole(&cc).select(Ident::<Post>::new().and(&cc).and(&db.post.score)).window(rank, |(_, s)| Reverse(s), asc);
    let tp: MatSet<(Id<Post>, i64)> = (&pw).filt(|(((_, n), _), r)| r <= 10 && n > 5).map(|((x, _), _)| x).collect();
    let mut v = drain((&tu).cross(&tp));
    v.sort_by_key(|&(_, ((u, a), (p, _)))| (Reverse(a[0]), u, Reverse(score(p))));
    rows(v.into_iter().map(|(_, ((u, a), (p, n)))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(a.map(V::I));
        f.extend(post_fields(db, p, &["title", "score"]));
        f.push(V::I(n));
        row(f)
    }))
}

// WITH UserVoteStats AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(V.Id) AS TotalVotes, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes, SUM(CASE WHEN V.VoteTypeId IN (2, 3) THEN 1 ELSE 0 END) AS VoteBalance FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.DisplayName),
// PostStats AS (SELECT P.Id AS PostId, P.Title, P.OwnerUserId, P.CreationDate, COUNT(C.Id) AS CommentCount, SUM(CASE WHEN PV.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount,
//        SUM(CASE WHEN PV.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS RecentPostRank
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes PV ON P.Id = PV.PostId GROUP BY P.Id, P.Title, P.OwnerUserId, P.CreationDate),
// TopPosts AS (SELECT PS.PostId, PS.Title, PS.CommentCount, PS.UpvoteCount, PS.DownvoteCount, PS.RecentPostRank, PS.OwnerUserId FROM PostStats PS WHERE PS.RecentPostRank <= 5)
// SELECT U.DisplayName AS UserName, U.TotalVotes, U.Upvotes, U.Downvotes, U.VoteBalance, TP.Title, TP.CommentCount, TP.UpvoteCount, TP.DownvoteCount
// FROM UserVoteStats U LEFT JOIN TopPosts TP ON U.UserId = TP.OwnerUserId ORDER BY U.VoteBalance DESC, U.TotalVotes DESC LIMIT 10;
fn q1677(db: &'static So) -> String {
    let uv = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 4], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64, a[3] + (t == 2 || t == 3) as i64],
        None => a,
    });
    let Post { owner_user, owner_user_id, creation_date, .. } = &db.post;
    let w = db.post.group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let tp_of: HashIdx<Id<User>, Id<Post>> = (&tp).select(owner_user).inv().collect();
    let rows_ = drain((&uv).and((&tp_of).opt()));
    let v = top_n(rows_, |&(u, (a, p))| (Reverse(a[3]), Reverse(a[0]), u, p), 10);
    let sel: MatSet<Id<Post>> = rel(v.clone()).flat_map(|(_, (_, p))| p).collect();
    let s = (&sel)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    rows(v.into_iter().map(|(u, (a, p))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(a.map(V::I));
        match p {
            Some(p) => {
                f.extend(post_fields(db, p, &["title"]));
                f.extend(s.get(p).unwrap().map(V::I));
            }
            None => f.extend([V::Null, V::Null, V::Null, V::Null]),
        }
        row(f)
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("7545", q7545),
    ("26887", q26887),
    ("6153", q6153),
    ("3565", q3565),
    ("9179", q9179),
    ("9088", q9088),
    ("5973", q5973),
    ("5382", q5382),
    ("1128", q1128),
    ("3684", q3684),
    ("5553", q5553),
    ("27312", q27312),
    ("30080", q30080),
    ("8442", q8442),
    ("9520", q9520),
    ("194", q194),
    ("3074", q3074),
    ("9075", q9075),
    ("8529", q8529),
    ("5516", q5516),
    ("2442", q2442),
    ("7384", q7384),
    ("8371", q8371),
    ("8086", q8086),
    ("8675", q8675),
    ("7086", q7086),
    ("29683", q29683),
    ("25254", q25254),
    ("34168", q34168),
    ("28980", q28980),
    ("5481", q5481),
    ("6142", q6142),
    ("7236", q7236),
    ("526", q526),
    ("5671", q5671),
    ("8424", q8424),
    ("6501", q6501),
    ("7923", q7923),
    ("7588", q7588),
    ("29686", q29686),
    ("7354", q7354),
    ("8643", q8643),
    ("27928", q27928),
    ("6038", q6038),
    ("5279", q5279),
    ("5749", q5749),
    ("3144", q3144),
    ("33353", q33353),
    ("4265", q4265),
    ("29559", q29559),
    ("9238", q9238),
    ("27619", q27619),
    ("32582", q32582),
    ("27195", q27195),
    ("7845", q7845),
    ("26361", q26361),
    ("26806", q26806),
    ("9377", q9377),
    ("8922", q8922),
    ("28597", q28597),
    ("7890", q7890),
    ("6695", q6695),
    ("27030", q27030),
    ("5018", q5018),
    ("962", q962),
    ("28529", q28529),
    ("28516", q28516),
    ("6820", q6820),
    ("28664", q28664),
    ("9024", q9024),
    ("6684", q6684),
    ("25352", q25352),
    ("27437", q27437),
    ("25847", q25847),
    ("29080", q29080),
    ("1677", q1677),
];
