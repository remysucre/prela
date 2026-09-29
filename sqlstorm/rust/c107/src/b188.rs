use harness::prelude::*;
use std::cmp::Reverse;

fn close_reasons(db: &'static So) -> HashIdx<i64, Str> {
    (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect()
}

fn join_sorted(v: &[Str]) -> Str {
    let mut x: Vec<Str> = v.to_vec();
    x.sort_unstable();
    x.dedup();
    Box::leak(x.join(", ").into_boxed_str())
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, COUNT(c.Id) AS CommentCount, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS ScoreRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= '2023-01-01' AND p.Score IS NOT NULL GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.PostTypeId),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(v.BountyAmount) AS TotalBounty FROM Users u JOIN Votes v ON u.Id = v.UserId WHERE v.VoteTypeId IN (8, 9) GROUP BY u.Id, u.DisplayName HAVING SUM(v.BountyAmount) > 0),
// ClosedPosts AS (SELECT ph.PostId, STRING_AGG(DISTINCT ctr.Name, ', ') AS ClosedReasons FROM PostHistory ph JOIN CloseReasonTypes ctr ON CAST(ph.Comment AS INTEGER) = ctr.Id WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId)
// SELECT rp.Title, rp.Score, rp.CommentCount, tu.DisplayName AS TopUser, tu.TotalBounty, cp.ClosedReasons
// FROM RankedPosts rp LEFT JOIN TopUsers tu ON rp.ScoreRank = 1 AND tu.TotalBounty IS NOT NULL LEFT JOIN ClosedPosts cp ON rp.Id = cp.PostId
// WHERE rp.Score > 0 AND COALESCE(cp.ClosedReasons, '') <> '' ORDER BY rp.Score DESC, rp.CreationDate DESC;
//
// The TopUsers ON names no key of tu: a rank-1 post is crossed with every top user, any other keeps one NULL row. The distinct reasons are listed sorted.
fn q1250(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, .. } = &db.post;
    let base = || db.post.with(creation_date.ge(ts(2023, 1, 1, 0, 0, 0)));
    let rk = per_group(ranked(drain(base().select(post_type_id)), |&(p, t)| (t, Reverse(score.get(p).unwrap())), false), |&(_, t)| t);
    let rv = rel(rk.into_iter().map(|((p, _), r)| (p, r)).collect());
    let ridx: HashIdx<Id<Post>, (Id<Post>, i64)> = (&rv).map(|(p, _)| p).inv().select(&rv).collect();
    let cc = base().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let reason = close_reasons(db);
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let cp = db
        .post_history
        .with(post_history_type_id.eq(10))
        .group_by(post)
        .select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason))
        .buf_fold(|v| join_sorted(&v));
    let Vote { vote_type_id, bounty_amount, user, .. } = &db.vote;
    let tu = db.vote.with(vote_type_id.is_in([8, 9])).group_by(user).select(bounty_amount).fold(0i64, |s, b| s + b);
    let tus = left_all(drain((&tu).filt(|s| s > 0)));
    let none = left_all(Vec::<(Id<User>, i64)>::new());
    let rp = drain(base().with(score.gt(0)).select((&ridx).and(&cc).and((&cp).filt(|s: Str| !s.is_empty()))));
    type J = (((Id<Post>, i64), i64), Str);
    let rpv = rel(rp.into_iter().map(|x| x.1).collect::<Vec<J>>());
    let mut v = Vec::new();
    (&rpv).filt(|(((_, r), _), _): J| r == 1).cross(&tus).drive(|_, (j, t)| v.push((j, t)));
    (&rpv).filt(|(((_, r), _), _): J| r != 1).cross(&none).drive(|_, (j, t)| v.push((j, t)));
    rows(v.into_iter().map(|((((p, _), c), s), t)| {
        let mut f = post_fields(db, p, &["title", "score"]);
        f.push(V::I(c));
        f.extend(match t {
            Some((u, b)) => [user_col(db, u, "name"), V::I(b)],
            None => [V::Null, V::Null],
        });
        f.push(V::S(s));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.Tags, u.DisplayName AS Author, p.CreationDate, p.Score, p.ViewCount,
//        ARRAY_LENGTH(string_to_array(substring(p.Tags, 2, length(p.Tags) - 2), '><'), 1) AS TagCount, RANK() OVER (ORDER BY p.Score DESC) AS ScoreRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1),
// PopularTags AS (SELECT unnest(string_to_array(substring(Tags, 2, length(Tags) - 2), '><')) AS Tag FROM RankedPosts),
// TagCounts AS (SELECT Tag, COUNT(*) AS UsageCount FROM PopularTags GROUP BY Tag ORDER BY UsageCount DESC LIMIT 10),
// PostStatistics AS (SELECT rp.PostId, rp.Title, rp.Author, rp.CreationDate, rp.Score, rp.ViewCount, COALESCE(tp.UsageCount, 0) AS TagPopularity
//     FROM RankedPosts rp LEFT JOIN TagCounts tp ON rp.Tags ILIKE '%' || tp.Tag || '%')
// SELECT ps.PostId, ps.Title, ps.Author, ps.CreationDate, ps.Score, ps.ViewCount, ps.TagPopularity,
//        CASE WHEN ps.Score > 100 THEN 'High' WHEN ps.Score BETWEEN 50 AND 100 THEN 'Medium' ELSE 'Low' END AS ScoreCategory
// FROM PostStatistics ps WHERE ps.TagPopularity > 0 ORDER BY ps.TagPopularity DESC, ps.Score DESC;
//
// The ILIKE goes through the distinct tag-list elements, as tag_mentions does; a top tag has no '<' or '>', so it can only match inside one element.
fn q28110(db: &'static So) -> String {
    let Post { post_type_id, owner_user, tags_str, score, .. } = &db.post;
    let rp = || db.post.with(post_type_id.eq(1)).with(owner_user);
    let tc = rp().group_by(tags_str.flat_map(tag_list)).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let top = top_n(drain(&tc), |&(t, n)| (Reverse(n), t), 10);
    let tv = rel(top);
    let by_name: HashIdx<Str, (Str, i64)> = (&tv).map(|(t, _)| t).inv().select(&tv).collect();
    let elems: MatSet<Str> = rp().select(tags_str.flat_map(tag_list)).collect();
    let contains: HashIdx<Str, (Str, i64)> = (&elems).select_where((&by_name).map(|x: (Str, i64)| x.0).inv(), |e: Str, n: Str| e.to_lowercase().contains(&n.to_lowercase())).select(&by_name).collect();
    let pairs: MatSet<(Id<Post>, (Str, i64))> = rp().select(Ident::<Post>::new().and(tags_str.flat_map(tag_list).select(&contains))).collect();
    let v = drain(&pairs);
    rows(v.into_iter().map(|(_, (p, (_, n)))| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "score", "views"]);
        f.push(V::I(n));
        let s = score.get(p).unwrap();
        f.push(V::S(if s > 100 { "High" } else if s >= 50 { "Medium" } else { "Low" }));
        row(f)
    }))
}

// WITH ProcessedTags AS (SELECT P.Id AS PostId, UNNEST(string_to_array(SUBSTRING(P.Tags, 2, LENGTH(P.Tags) - 2), '><')) AS TagName, P.Title, P.Body, P.CreationDate FROM Posts P WHERE P.PostTypeId = 1),
// UserEngagement AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS QuestionCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName),
// TagStatistics AS (SELECT T.TagName, COUNT(DISTINCT P.Id) AS TagUsageCount, AVG(U.QuestionCount) AS AvgQuestionsPerUser, SUM(U.UpVoteCount) AS TotalUpVotes
//     FROM ProcessedTags T JOIN Posts P ON T.PostId = P.Id JOIN UserEngagement U ON P.OwnerUserId = U.UserId GROUP BY T.TagName),
// EngagingTags AS (SELECT TagName, TagUsageCount, AvgQuestionsPerUser, TotalUpVotes, RANK() OVER (ORDER BY TotalUpVotes DESC) AS PopularityRank FROM TagStatistics WHERE TagUsageCount > 10)
// SELECT TagName, TagUsageCount, AvgQuestionsPerUser, TotalUpVotes, PopularityRank FROM EngagingTags WHERE PopularityRank <= 10 ORDER BY PopularityRank;
fn q28238(db: &'static So) -> String {
    let Post { post_type_id, owner_user, tags_str, .. } = &db.post;
    let up = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id)).opt()).fold(0i64, |n, t| n + (t == Some(2)) as i64);
    let qc = user_distinct_posts(db);
    let ts_ = db
        .post
        .with(post_type_id.eq(1))
        .group_by(tags_str.flat_map(tag_list))
        .select(Ident::<Post>::new().and(owner_user.select((&qc).and(&up))))
        .buf_fold(|v| (distinct_some(v.iter().map(|x| Some(x.0))), v.len() as i64, v.iter().map(|x| x.1 .0).sum::<i64>(), v.iter().map(|x| x.1 .1).sum::<i64>()));
    let r = ranked(drain((&ts_).filt(|(d, _, _, _)| d > 10)), |&(_, (_, _, _, u))| Reverse(u), false);
    rows(r.into_iter().take_while(|x| x.1 <= 10).map(|((t, (d, n, q, u)), k)| row(vec![V::S(t), V::I(d), avg(q, n), V::I(u), V::I(k)])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT a.Id) AS AnswerCount,
//        DENSE_RANK() OVER (ORDER BY p.Score DESC) AS RankScore, DENSE_RANK() OVER (ORDER BY p.ViewCount DESC) AS RankViews, STRING_AGG(DISTINCT t.TagName, ', ') AS Tags
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2 LEFT JOIN PostLinks pl ON p.Id = pl.PostId
//     LEFT JOIN Tags t ON t.Id = pl.RelatedPostId WHERE p.PostTypeId = 1 AND p.ClosedDate IS NULL GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount),
// AggregatedData AS (SELECT PostId, Title, CreationDate, Score, ViewCount, CommentCount, AnswerCount, RankScore, RankViews, Tags,
//        ROW_NUMBER() OVER (PARTITION BY CASE WHEN RankScore <= 10 THEN 'Top Score' WHEN RankViews <= 10 THEN 'Top Views' ELSE 'Others' END ORDER BY RankScore, RankViews) AS RowNum FROM RankedPosts)
// SELECT AD.PostId, AD.Title, AD.CreationDate, AD.Score, AD.ViewCount, AD.CommentCount, AD.AnswerCount, AD.Tags FROM AggregatedData AD WHERE AD.RowNum <= 10 ORDER BY AD.RankScore, AD.RankViews;
//
// The ranks read only base columns, so the posts are picked first and the joins are driven for those. The distinct tag names are listed sorted.
fn q5373(db: &'static So) -> String {
    let Post { post_type_id, closed_date, score, view_count, .. } = &db.post;
    let base = drain(db.post.with(post_type_id.eq(1)).minus(closed_date).select(view_count.opt()));
    let r = ranked(base, |&(p, _)| Reverse(score.get(p).unwrap()), true);
    let r = ranked(r, |&((_, w), _)| (w.is_none(), Reverse(w)), true);
    let cat = |s: i64, w: i64| if s <= 10 { 0 } else if w <= 10 { 1 } else { 2 };
    let pick = top_per(r, |&(((_, _), s), w)| cat(s, w), |&(((_, _), s), w)| (s, w), 10, false);
    let pv = rel(pick.into_iter().map(|(((p, _), s), w)| (p, s, w)).collect());
    type P = (Id<Post>, i64, i64);
    let tag_by_id: HashIdx<i64, Id<Tag>> = (&db.tag.origid).inv().collect();
    let cc = (&pv).map(|x: P| x.0).group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let ac = (&pv).map(|x: P| x.0).group_by(Ident::<Post>::new()).select(answers_of(db)).fold(0i64, |n, _| n + 1);
    let tg = (&pv)
        .map(|x: P| x.0)
        .group_by(Ident::<Post>::new())
        .select(links_of(db).select(&db.post_link.related_post_id).select(&tag_by_id).select(&db.tag.tag_name))
        .buf_fold(|v| join_sorted(&v));
    let v = drain((&pv).select(Same::<P>::new().and(Same::<P>::new().map(|x: P| x.0).select((&cc).opt().and((&ac).opt()).and((&tg).opt())))));
    rows(v.into_iter().map(|(_, ((p, _, _), ((c, a), t)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(c.unwrap_or(0)), V::I(a.unwrap_or(0)), ostr(t)]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, COALESCE(SUM(V.BountyAmount), 0) AS TotalBounty, RANK() OVER (ORDER BY COUNT(DISTINCT P.Id) DESC) AS ActivityRank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId WHERE U.Reputation > 1000 GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, QuestionCount, AnswerCount, TotalBounty, ActivityRank FROM UserActivity WHERE ActivityRank <= 10),
// ClosedPosts AS (SELECT PH.PostId, COUNT(PH.Id) AS CloseCount, STRING_AGG(DISTINCT C.Name, ', ') AS CloseReasons FROM PostHistory PH JOIN CloseReasonTypes C ON CAST(PH.Comment AS int) = C.Id
//     WHERE PH.PostHistoryTypeId IN (10, 11) GROUP BY PH.PostId)
// SELECT U.DisplayName AS User, U.PostCount, U.QuestionCount, U.AnswerCount, COALESCE(CP.CloseCount, 0) AS CloseCount, COALESCE(CP.CloseReasons, 'No Close Reasons') AS CloseReasons, U.TotalBounty
// FROM TopUsers U LEFT JOIN ClosedPosts CP ON U.UserId = CP.PostId ORDER BY U.ActivityRank;
//
// CP joins on the raw ids: a user id against a post id. The distinct reasons are listed sorted.
fn q1540(db: &'static So) -> String {
    let rich = || db.user.with((&db.user.reputation).gt(1000));
    let ua = rich()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select((&db.vote.bounty_amount).opt()).opt())).opt())
        .fold([0i64; 3], |a, p| match p {
            Some((t, b)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + b.flatten().unwrap_or(0)],
            None => a,
        });
    let pc = rich().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let r = ranked(drain((&ua).and(&pc)), |&(_, (_, n))| Reverse(n), false);
    let tu = rel(r.into_iter().take_while(|x| x.1 <= 10).map(|((u, (a, n)), _)| (u, a, n)).collect());
    type T = (Id<User>, [i64; 3], i64);
    let reason = close_reasons(db);
    let PostHistory { post_id, post_history_type_id, comment, .. } = &db.post_history;
    let cp = db
        .post_history
        .with(post_history_type_id.is_in([10, 11]))
        .group_by(post_id)
        .select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason))
        .buf_fold(|v| (v.len() as i64, join_sorted(&v)));
    let v = drain((&tu).select(Same::<T>::new().and(Same::<T>::new().map(|x: T| x.0).select(&db.user.origid).select(&cp).opt())));
    rows(v.into_iter().map(|(_, ((u, a, n), c))| {
        let (k, s) = c.unwrap_or((0, "No Close Reasons"));
        row(vec![user_col(db, u, "name"), V::I(n), V::I(a[0]), V::I(a[1]), V::I(k), V::S(s), V::I(a[2])])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p WHERE p.CreationDate >= CURRENT_TIMESTAMP - INTERVAL '1 year'),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounty, SUM(u.UpVotes) - SUM(u.DownVotes) AS NetVotes
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON v.PostId = p.Id GROUP BY u.Id, u.DisplayName HAVING COUNT(DISTINCT p.Id) > 10),
// UserBadges AS (SELECT ub.UserId, STRING_AGG(b.Name, ', ') AS BadgeNames FROM Badges ub JOIN Users u ON ub.UserId = u.Id
//     JOIN (SELECT Id, Name FROM PostHistoryTypes WHERE Class = 1) b ON ub.Name = b.Name GROUP BY ub.UserId)
// SELECT tu.DisplayName, tu.PostCount, tu.TotalBounty, tu.NetVotes, rb.PostId, rb.Title, rb.CreationDate, rb.ViewCount, rb.Score, COALESCE(ub.BadgeNames, 'No Badges') AS Badges
// FROM TopUsers tu LEFT JOIN RankedPosts rb ON tu.UserId = rb.PostId LEFT JOIN UserBadges ub ON tu.UserId = ub.UserId WHERE rb.PostRank = 1 ORDER BY tu.TotalBounty DESC, tu.NetVotes DESC LIMIT 10;
//
// PostHistoryTypes has no Class, so DuckDB binds it to ub.Class. rb joins on the raw ids: a user id against a post id. The badge names are listed sorted.
fn q4075(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let since = add_years(utc_to_ny(now_utc()), -1);
    let recent = drain(db.post.with(creation_date.ge(since)).select(owner_user.opt()));
    let first = top_per(recent, |&(_, u)| u, |&(p, _)| Reverse(creation_date.get(p).unwrap()), 1, false);
    let fv = rel(first.into_iter().map(|x| x.0).collect());
    let rb: HashIdx<i64, Id<Post>> = (&fv).map(|p: Id<Post>| db.post.origid.get(p).unwrap()).inv().select(&fv).collect();
    let User { up_votes, down_votes, .. } = &db.user;
    let tu = db
        .user
        .group_by(Ident::<User>::new())
        .select(up_votes.and(down_votes).and(posts_of(db).select(votes_of(db).select((&db.vote.bounty_amount).opt()).opt()).opt()))
        .fold([0i64; 2], |a, ((uv, dv), p)| [a[0] + p.flatten().flatten().unwrap_or(0), a[1] + uv - dv]);
    let pc = user_distinct_posts(db);
    let ptn: HashIdx<Str, Id<PostHistoryType>> = (&db.post_history_type.name).inv().collect();
    let Badge { user, name, class, .. } = &db.badge;
    let ub = db.badge.with(class.eq(1)).group_by(user).select(name.select(&ptn).select(&db.post_history_type.name)).buf_fold(|v| {
        let mut x: Vec<Str> = v.to_vec();
        x.sort_unstable();
        Box::leak(x.join(", ").into_boxed_str()) as Str
    });
    let v = drain((&tu).and((&pc).filt(|n| n > 10)).and((&db.user.origid).select(&rb)).and((&ub).opt()));
    let v = top_n(v, |&(u, (((a, _), _), _))| (Reverse(a[0]), Reverse(a[1]), u), 10);
    rows(v.into_iter().map(|(u, (((a, n), p), b))| {
        let mut f = vec![user_col(db, u, "name"), V::I(n), V::I(a[0]), V::I(a[1])];
        f.extend(post_fields(db, p, &["id", "title", "created", "views", "score"]));
        f.push(V::S(b.unwrap_or("No Badges")));
        row(f)
    }))
}

fn split_on(s: Str, sep: &'static str) -> std::str::Split<'static, &'static str> {
    s.split(sep)
}

// WITH PostStats AS (SELECT P.Id AS PostId, P.Title, P.Score, P.ViewCount, P.CreationDate, COUNT(DISTINCT C.Id) AS CommentCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount, ARRAY_AGG(DISTINCT T.TagName) AS Tags
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN unnest(string_to_array(P.Tags, '><')) AS T(TagName) ON TRUE
//     WHERE P.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY P.Id, P.Title, P.Score, P.ViewCount, P.CreationDate),
// UserBadges AS (SELECT U.Id AS UserId, COUNT(CASE WHEN B.Class = 1 THEN 1 END) AS GoldBadgeCount, COUNT(CASE WHEN B.Class = 2 THEN 1 END) AS SilverBadgeCount,
//        COUNT(CASE WHEN B.Class = 3 THEN 1 END) AS BronzeBadgeCount FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id)
// SELECT PS.PostId, PS.Title, PS.Score, PS.ViewCount, PS.CreationDate, PS.CommentCount, PS.UpVoteCount, PS.DownVoteCount, PS.Tags, UB.UserId, UB.GoldBadgeCount, UB.SilverBadgeCount, UB.BronzeBadgeCount
// FROM PostStats PS JOIN Users U ON U.Id = PS.PostId JOIN UserBadges UB ON U.Id = UB.UserId ORDER BY PS.Score DESC, PS.ViewCount DESC LIMIT 100;
//
// Users joins on the raw ids: a user id against a post id. The distinct tags are listed sorted.
fn q8263(db: &'static So) -> String {
    let Post { creation_date, tags_str, score, view_count, .. } = &db.post;
    let recent = || db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let tg = || tags_str.flat_map(|s: Str| split_on(s, "><")).opt();
    let pv = recent()
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(tg()))
        .fold([0i64; 2], |a, ((_, t), _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).buf_fold(|v| distinct_some(v.iter().copied()));
    let tl = recent().group_by(Ident::<Post>::new()).select(tg()).buf_fold(|v| {
        let mut x: Vec<Option<Str>> = v.to_vec();
        x.sort_unstable();
        x.dedup();
        &*Box::leak(x.into_boxed_slice())
    });
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| {
        [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]
    });
    let v = drain((&pv).and(&cc).and(&tl).and((&db.post.origid).select(&uidx).select(Ident::<User>::new().and(&ub))));
    let v = top_n(v, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 100);
    rows(v.into_iter().map(|(p, (((a, c), t), (u, b)))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "created"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1])]);
        f.push(V::L(t.iter().map(|&x| ostr(x)).collect()));
        f.push(user_col(db, u, "uid"));
        f.extend([V::I(b[0]), V::I(b[1]), V::I(b[2])]);
        row(f)
    }))
}

// WITH UserScore AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.UpVotes, U.DownVotes, (U.UpVotes - U.DownVotes) AS NetVotes, COUNT(DISTINCT P.Id) AS PostCount,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId WHERE U.Reputation > 0 GROUP BY U.Id, U.DisplayName, U.Reputation, U.UpVotes, U.DownVotes),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, NetVotes, PostCount, QuestionCount, AnswerCount, RANK() OVER (ORDER BY Reputation DESC) AS ReputationRank,
//        RANK() OVER (ORDER BY NetVotes DESC) AS NetVotesRank FROM UserScore),
// QuestionTags AS (SELECT P.Id AS QuestionId, unnest(string_to_array(P.Tags, '><')) AS Tag FROM Posts P WHERE P.PostTypeId = 1),
// TagPopularity AS (SELECT Tag, COUNT(*) AS TagCount FROM QuestionTags GROUP BY Tag ORDER BY TagCount DESC LIMIT 10)
// SELECT TU.DisplayName, TU.Reputation, TU.NetVotes, TU.PostCount, TU.QuestionCount, TU.AnswerCount, TP.Tag, TP.TagCount
// FROM TopUsers TU JOIN TagPopularity TP ON TU.QuestionCount > 0 ORDER BY TU.ReputationRank, TU.NetVotesRank;
fn q6920(db: &'static So) -> String {
    let Post { post_type_id, tags_str, .. } = &db.post;
    let us = db
        .user
        .with((&db.user.reputation).gt(0))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id).opt())
        .fold([0i64; 3], |a, t| match t {
            Some(t) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64],
            None => a,
        });
    let tc = db.post.with(post_type_id.eq(1)).group_by(tags_str.flat_map(|s: Str| split_on(s, "><"))).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let top = rel(top_n(drain(&tc), |&(t, n)| (Reverse(n), t), 10));
    let tu = rel(drain((&us).filt(|a| a[1] > 0)));
    let mut v = Vec::new();
    (&tu).cross(&top).drive(|_, ((u, a), (t, n))| v.push((u, a, t, n)));
    rows(v.into_iter().map(|(u, a, t, n)| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(db.user.up_votes.get(u).unwrap() - db.user.down_votes.get(u).unwrap()));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::S(t), V::I(n)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.Score > 0),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PopularTags AS (SELECT UNNEST(string_to_array(Tags, '<>')) AS Tag FROM Posts WHERE PostTypeId = 1 AND Tags IS NOT NULL),
// TagPopularity AS (SELECT Tag, COUNT(*) AS TagCount FROM PopularTags GROUP BY Tag ORDER BY TagCount DESC LIMIT 5)
// SELECT up.DisplayName AS UserDisplayName, r.Title AS TopPostTitle, r.CreationDate AS PostCreationDate, r.Score AS PostScore, tb.Tag AS PopularTag, ub.BadgeCount, ub.GoldBadges, ub.SilverBadges, ub.BronzeBadges
// FROM RankedPosts r JOIN Users up ON r.OwnerUserId = up.Id JOIN UserBadges ub ON up.Id = ub.UserId CROSS JOIN TagPopularity tb WHERE r.rn = 1 ORDER BY r.Score DESC, r.CreationDate DESC;
//
// The separator '<>' never occurs in Tags, so each "tag" is the whole tag string.
fn q1780(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, owner_user, tags_str, .. } = &db.post;
    let rp = drain(db.post.with(post_type_id.eq(1).and(score.gt(0))).select(owner_user.opt()));
    let first = top_per(rp, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())), 1, false);
    let fv = rel(first.into_iter().map(|x| x.0).collect());
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| {
        [a[0] + c.is_some() as i64, a[1] + (c == Some(1)) as i64, a[2] + (c == Some(2)) as i64, a[3] + (c == Some(3)) as i64]
    });
    let tc = db.post.with(post_type_id.eq(1)).group_by(tags_str.flat_map(|s: Str| split_on(s, "<>"))).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let top = rel(top_n(drain(&tc), |&(t, n)| (Reverse(n), t), 5));
    let ru = rel(drain((&fv).select(Ident::<Post>::new().and(owner_user.select(Ident::<User>::new().and(&ub))))).into_iter().map(|x| x.1).collect());
    let mut v = Vec::new();
    (&ru).cross(&top).drive(|_, ((p, (u, b)), (t, _))| v.push((p, u, b, t)));
    rows(v.into_iter().map(|(p, u, b, t)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title", "created", "score"]));
        f.push(V::S(t));
        f.extend(b.iter().map(|&x| V::I(x)));
        row(f)
    }))
}

// WITH PostTags AS (SELECT p.Id AS PostId, UNNEST(STRING_TO_ARRAY(SUBSTRING(p.Tags, 2, LENGTH(p.Tags) - 2), '><')) AS Tag FROM Posts p WHERE p.PostTypeId = 1),
// TagUsage AS (SELECT Tag, COUNT(*) AS UsageCount FROM PostTags GROUP BY Tag ORDER BY UsageCount DESC LIMIT 10),
// UserMetrics AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS QuestionCount, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositiveScoredQuestions,
//        SUM(CASE WHEN p.Score < 0 THEN 1 ELSE 0 END) AS NegativeScoredQuestions, COUNT(b.Id) AS BadgesCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId AND p.PostTypeId = 1 LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT um.DisplayName, um.QuestionCount, um.PositiveScoredQuestions, um.NegativeScoredQuestions, um.BadgesCount,
//        ROW_NUMBER() OVER (ORDER BY um.QuestionCount DESC, um.PositiveScoredQuestions DESC) AS Rank FROM UserMetrics um)
// SELECT tu.Rank, tu.DisplayName, tu.QuestionCount, tu.PositiveScoredQuestions, tu.NegativeScoredQuestions, tu.BadgesCount, tg.Tag, tg.UsageCount
// FROM TopUsers tu JOIN TagUsage tg ON tg.UsageCount = (SELECT MAX(UsageCount) FROM TagUsage) WHERE tu.Rank <= 10 ORDER BY tu.Rank, tg.Tag;
fn q25063(db: &'static So) -> String {
    let Post { post_type_id, tags_str, score, .. } = &db.post;
    let tc = db.post.with(post_type_id.eq(1)).group_by(tags_str.flat_map(tag_list)).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let tu_ = rel(top_n(drain(&tc), |&(t, n)| (Reverse(n), t), 10));
    let mx = (&tu_).fold_flat(i64::MIN, |m, (_, n)| m.max(n));
    let best = rel(drain((&tu_).filt(|(_, n)| n == mx)).into_iter().map(|x| x.1).collect());
    let qs = || posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1)));
    let um = db.user.group_by(Ident::<User>::new()).select(qs().select(score).opt().and(badges_of(db).opt())).fold([0i64; 3], |a, (s, b)| {
        [a[0] + (s.map_or(false, |s| s > 0)) as i64, a[1] + (s.map_or(false, |s| s < 0)) as i64, a[2] + b.is_some() as i64]
    });
    let qc = db.user.group_by(Ident::<User>::new()).select(qs().opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let v = top_n(drain((&um).and(&qc)), |&(_, (a, q))| (Reverse(q), Reverse(a[0])), 10);
    let tu = rel(v.into_iter().enumerate().map(|(i, x)| (i as i64 + 1, x)).collect());
    let mut out = Vec::new();
    (&tu).cross(&best).drive(|_, ((k, (u, (a, q))), (t, n))| out.push((k, u, a, q, t, n)));
    rows(out.into_iter().map(|(k, u, a, q, t, n)| row(vec![V::I(k), user_col(db, u, "name"), V::I(q), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::S(t), V::I(n)])))
}

fn join_all(v: &[Str], sep: &str) -> Str {
    let mut x: Vec<Str> = v.to_vec();
    x.sort_unstable();
    Box::leak(x.join(sep).into_boxed_str())
}

// Rewritten (rewrites/33167.sql): the STRING_AGG gets ORDER BY its own argument.
// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate > cast('2024-10-01' as date) - INTERVAL '30 days' AND p.Score > 10),
// PostAnswers AS (SELECT p.Id AS QuestionId, COUNT(a.Id) AS AnswerCount FROM Posts p LEFT JOIN Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2 WHERE p.PostTypeId = 1 GROUP BY p.Id),
// PostHistoryCTE AS (SELECT ph.PostId, COUNT(ph.Id) AS EditCount, MAX(ph.CreationDate) AS LastEditDate,
//        STRING_AGG(DISTINCT CONCAT('Type: ', pht.Name, ' by ', ph.UserDisplayName), '; ' ORDER BY CONCAT('Type: ', pht.Name, ' by ', ph.UserDisplayName)) AS EditDetails
//     FROM PostHistory ph JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, COALESCE(pa.AnswerCount, 0) AS AnswerCount, rp.ViewCount, rp.OwnerDisplayName, ph.EditCount, ph.LastEditDate, ph.EditDetails
// FROM RankedPosts rp LEFT JOIN PostAnswers pa ON rp.PostId = pa.QuestionId LEFT JOIN PostHistoryCTE ph ON rp.PostId = ph.PostId WHERE rp.Rank <= 5 ORDER BY rp.Score DESC, rp.CreationDate DESC;
fn q33167(db: &'static So) -> String {
    let Post { creation_date, score, owner_user, post_type_id, .. } = &db.post;
    let rp = drain(db.post.with(creation_date.gt(add_days(date(2024, 10, 1), -30)).and(score.gt(10))).with(owner_user).select(post_type_id));
    let rp = top_per(rp, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap())), 5, false);
    let pv = rel(rp);
    type P = (Id<Post>, i64);
    let pa = db.post.with(post_type_id.eq(1)).group_by(Ident::<Post>::new()).select(answers_of(db).opt()).fold(0i64, |n, a| n + a.is_some() as i64);
    let PostHistory { user_display_name, creation_date: hd, .. } = &db.post_history;
    let ph = (&pv)
        .map(|x: P| x.0)
        .group_by(Ident::<Post>::new())
        .select(history_of(db).select(htype_name(db).and(user_display_name.opt()).and(hd)))
        .buf_fold(|v| {
            let parts: Vec<Str> = v.iter().map(|&((n, u), _)| &*Box::leak(format!("Type: {} by {}", n, u.unwrap_or("")).into_boxed_str())).collect();
            (v.len() as i64, v.iter().map(|x| x.1).max().unwrap(), join_sorted_sep(&parts, "; "))
        });
    let v = drain((&pv).select(Same::<P>::new().map(|x: P| x.0).and(Same::<P>::new().map(|x: P| x.0).select((&pa).opt().and((&ph).opt())))));
    rows(v.into_iter().map(|(_, (p, (a, h)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.push(V::I(a.unwrap_or(0)));
        f.extend(post_fields(db, p, &["views", "owner"]));
        f.extend(match h {
            Some((n, d, s)) => [V::I(n), V::T(d), V::S(s)],
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

fn join_sorted_sep(v: &[Str], sep: &str) -> Str {
    let mut x: Vec<Str> = v.to_vec();
    x.sort_unstable();
    x.dedup();
    Box::leak(x.join(sep).into_boxed_str())
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate ASC) AS Rank
//     FROM Posts p WHERE p.Score > 0),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounty, MAX(p.LastActivityDate) AS LastActivity
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName),
// CloseReasons AS (SELECT ph.PostId, STRING_AGG(cr.Name, ', ') AS CloseReasons FROM PostHistory ph JOIN CloseReasonTypes cr ON CAST(ph.Comment AS INT) = cr.Id WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.PostId)
// SELECT u.DisplayName, ua.PostCount, ua.TotalBounty, ua.LastActivity, rp.Title, rp.Score, cr.CloseReasons
// FROM UserActivity ua JOIN Users u ON ua.UserId = u.Id LEFT JOIN RankedPosts rp ON u.Id = rp.OwnerUserId AND rp.Rank = 1 LEFT JOIN CloseReasons cr ON rp.PostId = cr.PostId
// WHERE ua.LastActivity > CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days' AND (COALESCE(cr.CloseReasons, '') <> '' OR rp.Score > 10)
// ORDER BY ua.TotalBounty DESC, ua.PostCount DESC, rp.Score DESC NULLS LAST;
//
// The reasons are listed sorted.
fn q20286(db: &'static So) -> String {
    let Post { score, creation_date, owner_user, last_activity_date, .. } = &db.post;
    let rich = || db.user.with((&db.user.reputation).gt(1000));
    let ua = rich()
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(last_activity_date.and(votes_of(db).select((&db.vote.bounty_amount).opt()).opt())).opt())
        .fold((0i64, i64::MIN), |(b, m), p| match p {
            Some((d, v)) => (b + v.flatten().unwrap_or(0), m.max(d)),
            None => (b, m),
        });
    let pc = user_distinct_posts(db);
    let first = top_per(drain(db.post.with(score.gt(0)).select(owner_user)), |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), creation_date.get(p).unwrap()), 1, false);
    let fv = rel(first);
    let rp: HashIdx<Id<User>, (Id<Post>, Id<User>)> = (&fv).map(|x: (Id<Post>, Id<User>)| x.1).inv().select(&fv).collect();
    let reason = close_reasons(db);
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let cr = db
        .post_history
        .with(post_history_type_id.is_in([10, 11]))
        .group_by(post)
        .select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason))
        .buf_fold(|v| join_all(&v, ", "));
    type J = (((i64, i64), i64), Option<(Id<Post>, Option<Str>)>);
    let t0 = add_days(ts(2024, 10, 1, 12, 34, 56), -30);
    let v = drain(
        (&ua)
            .and(&pc)
            .and((&rp).map(|x: (Id<Post>, Id<User>)| x.0).select(Ident::<Post>::new().and((&cr).opt())).opt())
            .filt(move |(((_, m), _), r): J| m != i64::MIN && m > t0 && r.map_or(false, |(p, c)| c.map_or(false, |c| !c.is_empty()) || score.get(p).unwrap() > 10)),
    );
    rows(v.into_iter().map(|(u, (((b, m), n), r))| {
        let mut f = vec![user_col(db, u, "name"), V::I(n), V::I(b), V::T(m)];
        match r {
            Some((p, c)) => {
                f.extend(post_fields(db, p, &["title", "score"]));
                f.push(ostr(c));
            }
            None => f.extend([V::Null, V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, COUNT(b.Id) AS BadgeCount
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// PostCloseReasons AS (SELECT ph.PostId, STRING_AGG(cr.Name, ', ') AS CloseReasons FROM PostHistory ph INNER JOIN CloseReasonTypes cr ON CAST(ph.Comment AS INTEGER) = cr.Id WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId)
// SELECT p.Title, p.CreationDate, u.DisplayName AS Owner, ps.UpVotes, ps.DownVotes, COALESCE(q.CloseReasons, 'Open') AS CloseReason, ranked.Score, ranked.ViewCount
// FROM RankedPosts ranked LEFT JOIN Posts p ON ranked.PostId = p.Id LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN UserStats ps ON u.Id = ps.UserId LEFT JOIN PostCloseReasons q ON p.Id = q.PostId
// WHERE ranked.rn = 1 ORDER BY ranked.CreationDate DESC LIMIT 100;
//
// rn reads only base columns, so the posts are picked first and UserStats is folded for their owners. The reasons are listed sorted.
fn q3009(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, .. } = &db.post;
    let rp = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let first = top_per(rp, |&(_, t)| t, |&(p, _)| Reverse(creation_date.get(p).unwrap()), 1, false);
    let fv: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let owners: MatSet<Id<User>> = (&fv).select(owner_user).collect();
    let us = (&owners)
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt().and(badges_of(db).opt()))
        .fold([0i64; 2], |a, (t, _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let reason = close_reasons(db);
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let q = db
        .post_history
        .with(post_history_type_id.eq(10))
        .group_by(post)
        .select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason))
        .buf_fold(|v| join_all(&v, ", "));
    let v = drain((&fv).select(Ident::<Post>::new().and(owner_user.select(Ident::<User>::new().and(&us)).opt()).and((&q).opt())));
    let v = top_n(v, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 100);
    rows(v.into_iter().map(|(_, ((p, u), c))| {
        let mut f = post_fields(db, p, &["title", "created"]);
        match u {
            Some((u, a)) => f.extend([user_col(db, u, "name"), V::I(a[0]), V::I(a[1])]),
            None => f.extend([V::Null, V::Null, V::Null]),
        }
        f.push(V::S(c.unwrap_or("Open")));
        f.extend(post_fields(db, p, &["score", "views"]));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.AnswerCount, p.ViewCount, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS UserPostRank
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.PostTypeId = 1),
// RecentUsers AS (SELECT u.Id, u.DisplayName, u.Reputation, u.CreationDate, u.Location, u.Views, u.UpVotes, u.DownVotes FROM Users u WHERE u.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '6 months'),
// ClosedPosts AS (SELECT ph.PostId, MAX(ph.CreationDate) AS LastClosedDate, STRING_AGG(DISTINCT cr.Name, ', ') AS CloseReasons FROM PostHistory ph JOIN CloseReasonTypes cr ON CAST(ph.Comment AS INTEGER) = cr.Id
//     WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId)
// SELECT u.DisplayName, u.Reputation, rp.Title, rp.CreationDate, rp.ViewCount, rp.AnswerCount, cp.LastClosedDate, cp.CloseReasons
// FROM RecentUsers u LEFT JOIN RankedPosts rp ON u.Id = rp.OwnerUserId AND rp.UserPostRank <= 5 LEFT JOIN ClosedPosts cp ON rp.Id = cp.PostId
// WHERE (u.UpVotes - u.DownVotes) > 100 OR (cp.LastClosedDate IS NOT NULL AND cp.LastClosedDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 month')
// ORDER BY u.Reputation DESC, rp.ViewCount DESC;
//
// The distinct reasons are listed sorted.
fn q3661(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, score, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let rp = drain(db.post.with(creation_date.ge(add_years(t0, -1)).and(post_type_id.eq(1))).select(owner_user));
    let rp = top_per(rp, |&(_, u)| u, |&(p, _)| Reverse(score.get(p).unwrap()), 5, true);
    let rv = rel(rp);
    let by_user: HashIdx<Id<User>, Id<Post>> = (&rv).map(|x: (Id<Post>, Id<User>)| x.1).inv().select((&rv).map(|x: (Id<Post>, Id<User>)| x.0)).collect();
    let reason = close_reasons(db);
    let PostHistory { post, post_history_type_id, comment, creation_date: hd, .. } = &db.post_history;
    let cp = db
        .post_history
        .with(post_history_type_id.eq(10))
        .group_by(post)
        .select(hd.and(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason)))
        .buf_fold(|v| (v.iter().map(|x| x.0).max().unwrap(), join_sorted(&v.iter().map(|x| x.1).collect::<Vec<_>>())));
    let User { up_votes, down_votes, .. } = &db.user;
    let m1 = add_months(t0, -1);
    type J = (i64, Option<(Id<Post>, Option<(i64, Str)>)>);
    let v = drain(
        db.user
            .with((&db.user.creation_date).ge(add_months(t0, -6)))
            .select(up_votes.and(down_votes).map(|(a, b)| a - b).and((&by_user).select(Ident::<Post>::new().and((&cp).opt())).opt()))
            .filt(move |(n, r): J| n > 100 || r.map_or(false, |(_, c)| c.map_or(false, |(d, _)| d >= m1))),
    );
    rows(v.into_iter().map(|(u, (_, r))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        match r {
            Some((p, c)) => {
                f.extend(post_fields(db, p, &["title", "created", "views", "answers"]));
                f.extend(match c {
                    Some((d, s)) => [V::T(d), V::S(s)],
                    None => [V::Null, V::Null],
                });
            }
            None => f.extend([V::Null, V::Null, V::Null, V::Null, V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.OwnerUserId, p.PostTypeId, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS RankScore,
//        COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) OVER (PARTITION BY p.Id) AS UpvoteCount, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) OVER (PARTITION BY p.Id) AS DownvoteCount
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId),
// PostTags AS (SELECT p.Id AS PostId, t.TagName FROM Posts p JOIN LATERAL unnest(string_to_array(substring(p.Tags, 2, length(p.Tags) - 2), '><')) AS t(TagName) ON true WHERE p.PostTypeId = 1),
// UserMetrics AS (SELECT u.Id AS UserId, SUM(b.Class) AS BadgeScore, AVG(u.Reputation) AS AvgReputation FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id)
// SELECT rp.PostId, rp.Title, rp.Score, rp.RankScore, rt.TagName, up.UserId, up.BadgeScore, up.AvgReputation,
//        CASE WHEN up.BadgeScore IS NULL THEN 'No Badges' WHEN up.BadgeScore > 5 THEN 'Expert' ELSE 'Novice' END AS UserStatus
// FROM RankedPosts rp JOIN PostTags rt ON rp.PostId = rt.PostId LEFT JOIN UserMetrics up ON rp.OwnerUserId = up.UserId
// WHERE (rp.UpvoteCount - rp.DownvoteCount) > 0 AND rp.RankScore <= 5 ORDER BY rp.PostTypeId, rp.Score DESC NULLS LAST LIMIT 100;
//
// RankedPosts has a row per post and vote. Only questions meet PostTags, and a question's rows are all in the PostTypeId = 1 partition, so only that one is numbered.
// The first five rows there are all votes of one post, so the tie inside ROW_NUMBER changes nothing visible.
fn q21135(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, tags_str, .. } = &db.post;
    let qrows = drain(db.post.with(post_type_id.eq(1)).select(Ident::<Post>::new().and(votes_of(db).opt())));
    let top = top_n(qrows.into_iter().map(|x| x.1).collect(), |&(p, v)| (Reverse(score.get(p).unwrap()), p, v), 5);
    let tv = rel(top.into_iter().enumerate().map(|(i, (p, _))| (p, i as i64 + 1)).collect());
    type T = (Id<Post>, i64);
    let vc = (&tv).map(|x: T| x.0).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold(0i64, |n, t| n + (t == 2) as i64 - (t == 3) as i64);
    let um = db.user.group_by(Ident::<User>::new()).select((&db.user.reputation).and(badges_of(db).select(&db.badge.class).opt())).fold((0i64, 0i64, 0i64, 0i64), |(s, k, r, n), (rep, c)| {
        (s + c.unwrap_or(0), k + c.is_some() as i64, r + rep, n + 1)
    });
    let v = drain(
        (&tv)
            .select(Same::<T>::new().and(Same::<T>::new().map(|x: T| x.0).select((&vc).filt(|d| d > 0).and(tags_str.flat_map(tag_list)).and(owner_user.select(Ident::<User>::new().and(&um)).opt())))),
    );
    let v = top_n(v, |&(_, ((p, _), _))| Reverse(score.get(p).unwrap()), 100);
    rows(v.into_iter().map(|(_, ((p, k), ((_, t), u)))| {
        let mut f = post_fields(db, p, &["id", "title", "score"]);
        f.extend([V::I(k), V::S(t)]);
        match u {
            Some((u, (s, bk, r, n))) => {
                f.extend([user_col(db, u, "uid"), if bk == 0 { V::Null } else { V::I(s) }, V::F(r as f64 / n as f64)]);
                f.push(V::S(if bk == 0 { "No Badges" } else if s > 5 { "Expert" } else { "Novice" }));
            }
            None => f.extend([V::Null, V::Null, V::Null, V::S("No Badges")]),
        }
        row(f)
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("1250", q1250),
    ("28110", q28110),
    ("28238", q28238),
    ("5373", q5373),
    ("1540", q1540),
    ("4075", q4075),
    ("8263", q8263),
    ("6920", q6920),
    ("1780", q1780),
    ("25063", q25063),
    ("33167", q33167),
    ("20286", q20286),
    ("3009", q3009),
    ("3661", q3661),
    ("21135", q21135),
];
