use harness::prelude::*;
use std::cmp::Reverse;

fn by_first<A: Copy + Eq + std::hash::Hash, B: Copy + Eq + std::hash::Hash>(m: &MatSet<(A, B)>) -> HashIdx<A, B> {
    m.map(|(a, _)| a).inv().map(|(_, b): (A, B)| b).collect()
}

/// SQL `s LIKE '%' || x || '%'`: `%` and `_` in `x` stay wildcards; no escape character.
fn like_sub(s: &str, x: &str) -> bool {
    if !x.contains('%') && !x.contains('_') {
        return s.contains(x);
    }
    let pat = format!("%{x}%");
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
    let w = base().group_by(post_type_id).select(Ident::<Post>::new().and(score)).window(rank, |(_, s)| Reverse(s), asc);
    let rv: MatSet<(Id<Post>, i64)> = (&w).map(|((p, _), r)| (p, r)).collect();
    let ridx: HashIdx<Id<Post>, (Id<Post>, i64)> = (&rv).map(|(p, _)| p).inv().collect();
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
    let tus: HashIdx<(), (Id<User>, i64)> = whole((&tu).filt(|s| s > 0)).select(Ident::<User>::new().and(&tu)).collect();
    let top_users = Same::<(Id<Post>, i64)>::new().filt(|(_, r): (Id<Post>, i64)| r == 1).map(|_: (Id<Post>, i64)| ()).select(&tus).opt();
    let v = drain(base().with(score.gt(0)).select((&ridx).and(&cc).and((&cp).filt(|s: Str| !s.is_empty())).and(Ident::<Post>::new().select(&ridx).select(top_users))));
    rows(v.into_iter().map(|(_, ((((p, _), c), s), t))| {
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
    let qc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let qs = || db.post.with(post_type_id.eq(1));
    let ts_ = qs().group_by(tags_str.flat_map(tag_list)).select(owner_user.select((&qc).and(&up))).fold([0i64; 3], |a, (q, u)| [a[0] + 1, a[1] + q, a[2] + u]);
    let dc = qs().group_by(tags_str.flat_map(tag_list)).select(Ident::<Post>::new().with(owner_user)).count_distinct();
    let et = (&dc).filt(|d| d > 10);
    let w = whole(&et).select(Same::<Str>::new().and((&et).and(&ts_))).window(rank, |(_, (_, a))| Reverse(a[2]), asc);
    rows(drain((&w).filt(|(_, k)| k <= 10)).into_iter().map(|(_, ((t, (d, a)), k))| row(vec![V::S(t), V::I(d), avg(a[1], a[0]), V::I(a[2]), V::I(k)])))
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
    type R = ((Id<Post>, i64), Option<i64>);
    let w = whole(db.post.with(post_type_id.eq(1)).minus(closed_date)).select(Ident::<Post>::new().and(score).and(view_count.opt())).window(dense_rank, |((_, s), _)| Reverse(s), asc);
    let w = (&w).window(dense_rank, |(((_, _), v), _): (R, i64)| (v.is_none(), Reverse(v)), asc);
    let rk: MatSet<(Id<Post>, i64, i64)> = (&w).map(|((((p, _), _), s), v)| (p, s, v)).collect();
    let cat = |(_, s, w): (Id<Post>, i64, i64)| if s <= 10 { 0 } else if w <= 10 { 1 } else { 2 };
    let pw = whole(&rk).group_by(Same::<(Id<Post>, i64, i64)>::new().map(cat)).select(Same::<(Id<Post>, i64, i64)>::new()).window(row_number, |(p, s, w)| (s, w, p), asc);
    let pv: MatSet<(Id<Post>, i64, i64)> = (&pw).filt(|(_, r)| r <= 10).map(|(x, _)| x).collect();
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
    let w = whole(&ua).select(Ident::<User>::new().and((&ua).and(&pc))).window(rank, |(_, (_, n))| Reverse(n), asc);
    type T = (Id<User>, [i64; 3], i64);
    let tu: MatSet<T> = (&w).filt(|(_, k)| k <= 10).map(|((u, (a, n)), _)| (u, a, n)).collect();
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
    let Post { creation_date, owner_user_id, .. } = &db.post;
    let since = add_years(utc_to_ny(now_utc()), -1);
    let w = db.post.with(creation_date.ge(since)).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let fv: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let rb: HashIdx<i64, Id<Post>> = (&fv).select(&db.post.origid).inv().collect();
    let User { up_votes, down_votes, .. } = &db.user;
    let tu = db
        .user
        .group_by(Ident::<User>::new())
        .select(up_votes.and(down_votes).and(posts_of(db).select(votes_of(db).select((&db.vote.bounty_amount).opt()).opt()).opt()))
        .fold([0i64; 2], |a, ((uv, dv), p)| [a[0] + p.flatten().flatten().unwrap_or(0), a[1] + uv - dv]);
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
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
    let cc = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
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
    let Post { post_type_id, score, creation_date, owner_user, owner_user_id, tags_str, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1).and(score.gt(0))).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(score).and(creation_date)).window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let fv: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|(((p, _), _), _)| p).collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| {
        [a[0] + c.is_some() as i64, a[1] + (c == Some(1)) as i64, a[2] + (c == Some(2)) as i64, a[3] + (c == Some(3)) as i64]
    });
    let tc = db.post.with(post_type_id.eq(1)).group_by(tags_str.flat_map(|s: Str| split_on(s, "<>"))).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let top = rel(top_n(drain(&tc), |&(t, n)| (Reverse(n), t), 5));
    let mut v = Vec::new();
    (&fv).select(Ident::<Post>::new().and(owner_user.select(Ident::<User>::new().and(&ub)))).cross(&top).drive(|_, ((p, (u, b)), (t, _))| v.push((p, u, b, t)));
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
    let best = (&tu_).filt(|(_, n)| n == mx);
    let qs = || posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1)));
    let um = db.user.group_by(Ident::<User>::new()).select(qs().select(score).opt().and(badges_of(db).opt())).fold([0i64; 3], |a, (s, b)| {
        [a[0] + (s.map_or(false, |s| s > 0)) as i64, a[1] + (s.map_or(false, |s| s < 0)) as i64, a[2] + b.is_some() as i64]
    });
    let qc = db.user.group_by(Ident::<User>::new()).select(qs().opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let w = whole(&um).select(Ident::<User>::new().and((&um).and(&qc))).window(row_number, |(u, (a, q))| (Reverse(q), Reverse(a[0]), u), asc);
    let mut out = Vec::new();
    (&w).filt(|(_, k)| k <= 10).cross(&best).drive(|_, (((u, (a, q)), k), (t, n))| out.push((k, u, a, q, t, n)));
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
    let w = db
        .post
        .with(creation_date.gt(add_days(date(2024, 10, 1), -30)).and(score.gt(10)))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    type P = (Id<Post>, i64);
    let pv: MatSet<P> = (&w).filt(|(_, r)| r <= 5).map(|(((p, _), _), r)| (p, r)).collect();
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
    let pc = rich().group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let w = db.post.with(score.gt(0)).group_by(owner_user).select(Ident::<Post>::new().and(score).and(creation_date)).window(row_number, |((p, s), d)| (Reverse(s), d, p), asc);
    let rp: HashIdx<Id<User>, (Id<Post>, i64)> = (&w).filt(|(_, r)| r == 1).map(|(((p, s), _), _)| (p, s)).collect();
    let reason = close_reasons(db);
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let cr = db
        .post_history
        .with(post_history_type_id.is_in([10, 11]))
        .group_by(post)
        .select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason))
        .buf_fold(|v| join_all(&v, ", "));
    type J = (((i64, i64), i64), Option<((Id<Post>, i64), Option<Str>)>);
    let t0 = add_days(ts(2024, 10, 1, 12, 34, 56), -30);
    let v = drain(
        (&ua)
            .and(&pc)
            .and((&rp).select(Same::<(Id<Post>, i64)>::new().and(Same::<(Id<Post>, i64)>::new().map(|x: (Id<Post>, i64)| x.0).select((&cr).opt()))).opt())
            .filt(move |(((_, m), _), r): J| m != i64::MIN && m > t0 && r.map_or(false, |((_, s), c)| c.map_or(false, |c| !c.is_empty()) || s > 10)),
    );
    rows(v.into_iter().map(|(u, (((b, m), n), r))| {
        let mut f = vec![user_col(db, u, "name"), V::I(n), V::I(b), V::T(m)];
        match r {
            Some(((p, _), c)) => {
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
    let w = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(post_type_id).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let fv: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
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
    let w = db.post.with(creation_date.ge(add_years(t0, -1)).and(post_type_id.eq(1))).group_by(owner_user).select(Ident::<Post>::new().and(score)).window(rank, |(_, s)| Reverse(s), asc);
    let by_user: HashIdx<Id<User>, Id<Post>> = (&w).filt(|(_, k)| k <= 5).map(|((p, _), _)| p).collect();
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
    let w = db.post.with(post_type_id.eq(1)).group_by(post_type_id).select(Ident::<Post>::new().and(score).and(votes_of(db).opt())).window(row_number, |((p, s), v)| (Reverse(s), p, v), asc);
    type T = (Id<Post>, i64);
    let tv: MatSet<T> = (&w).filt(|(_, r)| r <= 5).map(|(((p, _), _), r)| (p, r)).collect();
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

// WITH RECURSIVE UserReputationCTE AS (SELECT Id, Reputation, CreationDate, DisplayName, 1 AS Level FROM Users WHERE Reputation > 0 UNION ALL
//     SELECT u.Id, u.Reputation, u.CreationDate, u.DisplayName, CTE.Level + 1 FROM Users AS u INNER JOIN UserReputationCTE AS CTE ON u.Reputation > CTE.Reputation WHERE CTE.Level < 5),
// PostStatistics AS (SELECT p.Id AS PostId, COUNT(v.Id) AS VoteCount, COUNT(DISTINCT c.Id) AS CommentCount, AVG(COALESCE(Length(p.Body), 0)) AS AvgBodyLength
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY p.Id),
// PopularPosts AS (SELECT ps.PostId, ps.VoteCount, ps.CommentCount, ps.AvgBodyLength FROM PostStatistics ps WHERE ps.VoteCount > 10)
// SELECT u.DisplayName, u.Reputation, u.CreationDate, pp.PostId, pp.VoteCount, pp.CommentCount, pp.AvgBodyLength, RANK() OVER (PARTITION BY u.Id ORDER BY pp.VoteCount DESC) AS PostRank,
//        CASE WHEN pp.CommentCount > 5 THEN 'Highly Discussed' WHEN pp.CommentCount BETWEEN 1 AND 5 THEN 'Moderately Discussed' ELSE 'Not Discussed' END AS DiscussionLevel,
//        SUM(b.Class) OVER (PARTITION BY u.Id) AS TotalBadgeClass
// FROM Users u JOIN PopularPosts pp ON u.Id = pp.PostId LEFT JOIN Badges b ON u.Id = b.UserId WHERE u.Reputation IS NOT NULL ORDER BY u.Reputation DESC, pp.VoteCount DESC;
//
// The recursive CTE is never read. pp joins on the raw ids: a user id against a post id, so a user meets at most one post and PostRank is 1 on every row.
fn q34549(db: &'static So) -> String {
    let Post { body, .. } = &db.post;
    let ps = db
        .post
        .group_by(Ident::<Post>::new())
        .select(body.and(votes_of(db).opt()).and(comments_of(db).opt()))
        .fold((0i64, 0i64, 0i64), |(v, l, n), ((b, x), _)| (v + x.is_some() as i64, l + b.chars().count() as i64, n + 1));
    let cc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let pp: HashIdx<i64, Id<Post>> = db.post.with((&ps).filt(|(v, _, _)| v > 10)).select(&db.post.origid).inv().collect();
    let bs = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class)).fold(0i64, |s, c| s + c);
    let v = drain(db.user.select((&db.user.origid).select(&pp).select(Ident::<Post>::new().and(&ps).and(&cc)).and((&bs).opt()).and(badges_of(db).opt())));
    rows(v.into_iter().map(|(u, ((((p, (n, l, k)), c), s), _))| {
        let mut f = ucols(db, u, &["name", "rep", "ucreated"]);
        f.extend(post_fields(db, p, &["id"]));
        f.extend([V::I(n), V::I(c), V::F(l as f64 / k as f64), V::I(1)]);
        f.push(V::S(if c > 5 { "Highly Discussed" } else if c >= 1 { "Moderately Discussed" } else { "Not Discussed" }));
        f.push(oint(s));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS TotalComments, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' GROUP BY p.Id, p.Title, p.CreationDate, u.DisplayName, p.PostTypeId),
// TopPosts AS (SELECT PostId, Title, CreationDate, OwnerDisplayName, TotalComments, TotalUpvotes, TotalDownvotes FROM RankedPosts WHERE Rank <= 5)
// SELECT tp.Title, tp.CreationDate, tp.OwnerDisplayName, tp.TotalComments, tp.TotalUpvotes, tp.TotalDownvotes,
//        CASE WHEN tp.TotalUpvotes > tp.TotalDownvotes THEN 'Positive' WHEN tp.TotalDownvotes > tp.TotalUpvotes THEN 'Negative' ELSE 'Neutral' END AS Sentiment,
//        (SELECT STRING_AGG(DISTINCT t.TagName, ', ') FROM Tags t WHERE t.ExcerptPostId = tp.PostId) AS Tags
// FROM TopPosts tp ORDER BY tp.TotalUpvotes DESC, tp.CreationDate ASC;
//
// Rank reads only base columns, so the posts are picked first. The distinct tag names are listed sorted.
fn q9758(db: &'static So) -> String {
    let Post { creation_date, post_type_id, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).group_by(post_type_id).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let agg = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 3], |a, (c, t)| {
        [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let excerpt: HashIdx<Id<Post>, Id<Tag>> = (&db.tag.excerpt_post).inv().collect();
    let tg = (&tp).group_by(Ident::<Post>::new()).select((&excerpt).select(&db.tag.tag_name)).buf_fold(|v| join_sorted(&v));
    let v = drain((&agg).and((&tg).opt()));
    rows(v.into_iter().map(|(p, (a, t))| {
        let mut f = post_fields(db, p, &["title", "created", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        f.push(V::S(if a[1] > a[2] { "Positive" } else if a[2] > a[1] { "Negative" } else { "Neutral" }));
        f.push(ostr(t));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserRank, p.OwnerUserId
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND u.Reputation > 1000),
// UserBadges AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount, STRING_AGG(b.Name, ', ') AS BadgeNames FROM Badges b WHERE b.Class = 1 GROUP BY b.UserId),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c GROUP BY c.PostId)
// SELECT rp.Title, rp.CreationDate, COALESCE(ub.BadgeCount, 0) AS GoldBadgeCount, COALESCE(pc.CommentCount, 0) AS Comments, rp.ViewCount, rp.Score,
//        CASE WHEN rp.ViewCount IS NULL THEN 'No Views' ELSE CAST(rp.ViewCount AS CHAR) END AS ViewCountText,
//        CASE WHEN rp.AnswerCount > 0 THEN 'Has Answers' ELSE 'No Answers' END AS AnswerStatus,
//        CASE WHEN rp.Score > 0 THEN 'Positive Score' WHEN rp.Score < 0 THEN 'Negative Score' ELSE 'Neutral Score' END AS ScoreStatus
// FROM RankedPosts rp LEFT JOIN UserBadges ub ON rp.OwnerUserId = ub.UserId LEFT JOIN PostComments pc ON rp.PostId = pc.PostId
// WHERE rp.UserRank = 1 ORDER BY rp.CreationDate DESC FETCH FIRST 50 ROWS ONLY;
fn q2118(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, view_count, answer_count, score, .. } = &db.post;
    let w = db
        .post
        .with(post_type_id.eq(1))
        .with(owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(1000))))
        .group_by(owner_user)
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (Reverse(d), p), asc);
    let first = top_n(drain((&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p)).into_iter().map(|(u, p)| (p, u)).collect(), |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 50);
    let fv = rel(first);
    type F = (Id<Post>, Id<User>);
    let ub = db.badge.with((&db.badge.class).eq(1)).group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let pc = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&fv).select(Same::<F>::new().map(|x: F| x.0).and(Same::<F>::new().map(|x: F| x.1).select(&ub).opt()).and(Same::<F>::new().map(|x: F| x.0).select(&pc).opt())));
    rows(v.into_iter().map(|(_, ((p, b), c))| {
        let mut f = post_fields(db, p, &["title", "created"]);
        f.extend([V::I(b.unwrap_or(0)), V::I(c.unwrap_or(0))]);
        f.extend(post_fields(db, p, &["views", "score"]));
        f.push(match view_count.get(p) {
            None => V::S("No Views"),
            Some(w) => V::Owned(w.to_string()),
        });
        f.push(V::S(if answer_count.get(p).map_or(false, |a| a > 0) { "Has Answers" } else { "No Answers" }));
        let s = score.get(p).unwrap();
        f.push(V::S(if s > 0 { "Positive Score" } else if s < 0 { "Negative Score" } else { "Neutral Score" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS QuestionCount, COUNT(DISTINCT c.Id) AS CommentCount, SUM(v.BountyAmount) AS TotalBounty
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName),
// PostCloseReasons AS (SELECT ph.PostId, STRING_AGG(crt.Name, ', ') AS CloseReasons FROM PostHistory ph JOIN CloseReasonTypes crt ON CAST(ph.Comment AS INT) = crt.Id WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId)
// SELECT us.DisplayName, us.QuestionCount, us.CommentCount, us.TotalBounty, rp.Title, rp.CreationDate, rp.Score, PCR.CloseReasons
// FROM UserStats us INNER JOIN RankedPosts rp ON us.UserId = rp.PostId LEFT JOIN PostCloseReasons PCR ON rp.PostId = PCR.PostId
// WHERE us.QuestionCount > 5 AND (us.TotalBounty IS NOT NULL OR us.CommentCount > 10) AND rp.PostRank = 1 ORDER BY us.TotalBounty DESC, rp.CreationDate DESC LIMIT 100;
//
// rp joins on the raw ids: a user id against a post id. The rank-1 posts are picked first and UserStats is folded only for the users they meet. The reasons are listed sorted.
fn q121(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user_id, .. } = &db.post;
    let w = db
        .post
        .with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))))
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (Reverse(d), p), asc);
    let fv: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let by_id: HashIdx<i64, Id<Post>> = (&fv).select(&db.post.origid).inv().collect();
    let met: MatSet<Id<User>> = db.user.with((&db.user.origid).select(&by_id)).map(|u| u).collect();
    let bounty = (&met)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(comments_of(db).opt()).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold((0i64, 0i64), |(n, s), (_, b)| match b.flatten() {
            Some(b) => (n + 1, s + b),
            None => (n, s),
        });
    let qc = (&met).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let cc = (&met).group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db))).fold(0i64, |n, _| n + 1);
    let reason = close_reasons(db);
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let pcr = db
        .post_history
        .with(post_history_type_id.eq(10))
        .group_by(post)
        .select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason))
        .buf_fold(|v| join_all(&v, ", "));
    type J = (((i64, i64), i64), Option<i64>);
    let v = drain(
        (&bounty)
            .and(&qc)
            .and((&cc).opt())
            .filt(|(((n, _), q), c): J| q > 5 && (n > 0 || c.unwrap_or(0) > 10))
            .and((&db.user.origid).select(&by_id).select(Ident::<Post>::new().and((&pcr).opt()))),
    );
    let v = top_n(v, |&(u, (((b, _), _), (p, _)))| (b.0 == 0, Reverse(b.1), Reverse(creation_date.get(p).unwrap()), u), 100);
    rows(v.into_iter().map(|(u, (((b, q), c), (p, r)))| {
        let mut f = vec![user_col(db, u, "name"), V::I(q), V::I(c.unwrap_or(0)), nullable(b.1, b.0)];
        f.extend(post_fields(db, p, &["title", "created", "score"]));
        f.push(ostr(r));
        row(f)
    }))
}

// WITH PostStats AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT CASE WHEN v.VoteTypeId = 2 THEN v.Id END) AS UpVotes,
//        COUNT(DISTINCT CASE WHEN v.VoteTypeId = 3 THEN v.Id END) AS DownVotes, ARRAY_AGG(DISTINCT t.TagName) AS Tags
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN unnest(string_to_array(p.Tags, '><')) AS t(TagName) ON TRUE
//     WHERE p.PostTypeId = 1 AND p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount),
// UserRankings AS (SELECT u.Id AS UserId, u.DisplayName, SUM(p.Score) AS TotalScore, COUNT(DISTINCT p.Id) AS PostsCount, COUNT(DISTINCT b.Id) AS BadgesCount
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName),
// TopPosts AS (SELECT ps.*, ur.DisplayName AS OwnerDisplayName, ur.TotalScore, ur.PostsCount, ur.BadgesCount FROM PostStats ps JOIN UserRankings ur ON ps.PostId = ur.UserId ORDER BY ps.Score DESC LIMIT 10)
// SELECT PostId, Title, CreationDate, Score, ViewCount, CommentCount, UpVotes, DownVotes, Tags, OwnerDisplayName, TotalScore, PostsCount, BadgesCount FROM TopPosts ORDER BY Score DESC, CreationDate ASC;
//
// ur joins on the raw ids: a post id against a user id, and is folded only for the users the posts meet. The distinct tags are listed sorted.
fn q7576(db: &'static So) -> String {
    let Post { post_type_id, creation_date, tags_str, score, .. } = &db.post;
    let recent = || db.post.with(post_type_id.eq(1).and(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))));
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let met: MatSet<Id<User>> = recent().select((&db.post.origid).select(&uidx)).collect();
    let ur = (&met).group_by(Ident::<User>::new()).select(posts_of(db).select(score).opt().and(badges_of(db).opt())).fold((0i64, 0i64), |(s, n), (p, _)| (s + p.unwrap_or(0), n + p.is_some() as i64));
    let pc = (&met).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let bc = (&met).group_by(Ident::<User>::new()).select(badges_of(db)).fold(0i64, |n, _| n + 1);
    let cc = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let vc = recent().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let tl = recent().group_by(Ident::<Post>::new()).select(tags_str.flat_map(|s: Str| split_on(s, "><")).opt()).buf_fold(|v| {
        let mut x: Vec<Option<Str>> = v.to_vec();
        x.sort_unstable();
        x.dedup();
        &*Box::leak(x.into_boxed_slice())
    });
    let v = drain((&cc).and(&vc).and(&tl).and((&db.post.origid).select(&uidx).select(Ident::<User>::new().and(&ur).and(&pc).and((&bc).opt()))));
    let v = top_n(v, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 10);
    rows(v.into_iter().map(|(p, (((c, a), t), (((u, (s, sn)), n), b)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(c), V::I(a[0]), V::I(a[1])]);
        f.push(V::L(t.iter().map(|&x| ostr(x)).collect()));
        f.extend([user_col(db, u, "name"), nullable(s, sn), V::I(n), V::I(b.unwrap_or(0))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.Body, U.DisplayName AS OwnerDisplayName, P.CreationDate, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount, COUNT(DISTINCT A.Id) AS AnswerCount,
//        ROW_NUMBER() OVER (PARTITION BY P.Id ORDER BY P.CreationDate DESC) AS Rank
//     FROM Posts P LEFT JOIN Users U ON P.OwnerUserId = U.Id LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Posts A ON P.Id = A.ParentId AND A.PostTypeId = 2
//     WHERE P.PostTypeId = 1 GROUP BY P.Id, P.Title, P.Body, U.DisplayName, P.CreationDate),
// TopPosts AS (SELECT RP.PostId, RP.Title, RP.OwnerDisplayName, RP.CreationDate, RP.CommentCount, RP.AnswerCount, ROW_NUMBER() OVER (ORDER BY RP.CreationDate DESC) AS NewRank FROM RankedPosts RP WHERE RP.Rank = 1)
// SELECT TP.PostId, TP.Title, TP.OwnerDisplayName, CAST(TP.CreationDate AS VARCHAR) AS FormattedCreationDate, TP.CommentCount, TP.AnswerCount,
//        CASE WHEN TP.AnswerCount > 0 THEN 'Has Answers' ELSE 'No Answers' END AS AnswerStatus, STRING_AGG(PT.Name, ', ') AS PostTypeNames
// FROM TopPosts TP LEFT JOIN PostTypes PT ON PT.Id = (SELECT PostTypeId FROM Posts WHERE Id = TP.PostId LIMIT 1)
// GROUP BY TP.PostId, TP.Title, TP.OwnerDisplayName, TP.CreationDate, TP.CommentCount, TP.AnswerCount HAVING TP.AnswerCount >= 2 ORDER BY TP.CreationDate DESC LIMIT 10;
//
// Rank partitions by the post itself, so it is 1 on every row. The HAVING and the LIMIT read only the answer count and the date, so the ten posts are picked first
// and the comment × answer product is driven for them.
fn q29536(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let ac = db.post.with(post_type_id.eq(1)).group_by(Ident::<Post>::new()).select(answers_of(db)).fold(0i64, |n, _| n + 1);
    let top = top_n(drain((&ac).filt(|n| n >= 2)), |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 10);
    let tp = rel(top);
    type T = (Id<Post>, i64);
    let cc = (&tp).map(|x: T| x.0).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(answers_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let v = drain((&tp).select(Same::<T>::new().and(Same::<T>::new().map(|x: T| x.0).select((&cc).and(ptype_name(db))))));
    rows(v.into_iter().map(|(_, ((p, a), (c, t)))| {
        let mut f = post_fields(db, p, &["id", "title", "owner"]);
        f.push(V::Owned(ts_text(creation_date.get(p).unwrap())));
        f.extend([V::I(c), V::I(a), V::S(if a > 0 { "Has Answers" } else { "No Answers" }), V::S(t)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.OwnerUserId, p.CreationDate, p.Score, ROW_NUMBER() OVER(PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.CreationDate >= CURRENT_DATE - INTERVAL '1 YEAR'),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// CloseReasonDetails AS (SELECT ph.PostId, STRING_AGG(cr.Name, ', ') AS Reasons FROM PostHistory ph JOIN CloseReasonTypes cr ON CAST(ph.Comment AS INTEGER) = cr.Id WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId)
// SELECT p.Id AS PostId, p.Title, u.DisplayName AS Author, vb.BadgeCount AS TotalBadges, vb.GoldBadges, vb.SilverBadges, vb.BronzeBadges, COALESCE(crd.Reasons, 'No reasons provided') AS CloseReasons,
//        rp.Rank, p.Score, p.ViewCount
// FROM RankedPosts rp JOIN Posts p ON rp.Id = p.Id JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN UserBadges vb ON u.Id = vb.UserId LEFT JOIN CloseReasonDetails crd ON p.Id = crd.PostId
// WHERE p.Score > 10 ORDER BY p.Score DESC, p.CreationDate DESC LIMIT 10 OFFSET 10;
//
// The reasons are listed sorted.
fn q33976(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, owner_user_id, score, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(current_date(), -1)))).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    type R = (Id<Post>, i64);
    let rv: MatSet<R> = (&w).map(|((p, _), r)| (p, r)).collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| {
        [a[0] + c.is_some() as i64, a[1] + (c == Some(1)) as i64, a[2] + (c == Some(2)) as i64, a[3] + (c == Some(3)) as i64]
    });
    let reason = close_reasons(db);
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let crd = db
        .post_history
        .with(post_history_type_id.eq(10))
        .group_by(post)
        .select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason))
        .buf_fold(|v| join_all(&v, ", "));
    let v = drain((&rv).select(Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0).with(score.gt(10)).select(owner_user.select(Ident::<User>::new().and(&ub)).and((&crd).opt())))));
    let v = top_n(v, |&(_, ((p, _), _))| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 20);
    rows(v.into_iter().skip(10).map(|(_, ((p, r), ((u, b), c)))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.push(user_col(db, u, "name"));
        f.extend(b.iter().map(|&x| V::I(x)));
        f.extend([V::S(c.unwrap_or("No reasons provided")), V::I(r)]);
        f.extend(post_fields(db, p, &["score", "views"]));
        row(f)
    }))
}

// WITH UserTags AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT T.TagName) AS TagCount, STRING_AGG(DISTINCT T.TagName, ', ') AS Tags
//     FROM Users U JOIN Posts P ON U.Id = P.OwnerUserId JOIN unnest(string_to_array(substring(P.Tags, 2, length(P.Tags) - 2), '><')) AS T(TagName) ON TRUE WHERE P.PostTypeId = 1 GROUP BY U.Id, U.DisplayName),
// PopularTags AS (SELECT T.TagName, COUNT(T.TagName) AS Frequency FROM Posts P JOIN unnest(string_to_array(substring(P.Tags, 2, length(P.Tags) - 2), '><')) AS T(TagName) ON TRUE
//     GROUP BY T.TagName ORDER BY Frequency DESC LIMIT 10),
// UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS PostsCreated, SUM(CASE WHEN PH.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS PostsClosed
//     FROM Users U JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN PostHistory PH ON P.Id = PH.PostId GROUP BY U.Id, U.DisplayName),
// FinalReport AS (SELECT U.DisplayName, U.PostsCreated, U.PostsClosed, UT.Tags, PT.TagName AS MostPopularTag FROM UserActivity U JOIN UserTags UT ON U.UserId = UT.UserId
//     LEFT JOIN PopularTags PT ON TRUE ORDER BY U.PostsCreated DESC)
// SELECT DisplayName, PostsCreated, PostsClosed, Tags, MostPopularTag FROM FinalReport WHERE MostPopularTag IS NULL;
//
// PT is joined ON TRUE, so every user row meets every popular tag and only an empty PopularTags leaves a NULL. The distinct tags are listed sorted.
fn q28650(db: &'static So) -> String {
    let Post { post_type_id, tags_str, .. } = &db.post;
    let pt = db.post.group_by(tags_str.flat_map(tag_list)).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let ptv = rel(top_n(drain(&pt), |&(t, n)| (Reverse(n), t), 10));
    let pt: HashIdx<(), (Str, i64)> = whole(&ptv).select(&ptv).collect();
    let ut = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1))).select(tags_str.flat_map(tag_list)))
        .buf_fold(|v| join_sorted(&v));
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(history_of(db).select(&db.post_history.post_history_type_id).opt()))
        .fold(0i64, |n, t| n + (t == Some(10)) as i64);
    let pc = db.user.group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    type J = (((i64, i64), Str), Option<(Str, i64)>);
    let v = drain((&ua).and(&pc).and(&ut).and(Ident::<User>::new().map(|_: Id<User>| ()).select(&pt).opt()).filt(|(_, t): J| t.is_none()));
    rows(v.into_iter().map(|(u, (((c, n), s), _))| row(vec![user_col(db, u, "name"), V::I(n), V::I(c), V::S(s), V::Null])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.OwnerUserId, p.Title, p.CreationDate, p.Score, p.ViewCount, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank,
//        COUNT(v.Id) AS VoteCount, STRING_AGG(DISTINCT t.TagName, ', ') AS Tags
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 2 LEFT JOIN Tags t ON p.Tags LIKE CONCAT('%', t.TagName, '%')
//     WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year' GROUP BY p.Id, p.OwnerUserId, p.Title, p.CreationDate, p.Score, p.ViewCount),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.CreationDate FROM Users u WHERE u.Reputation >= (SELECT AVG(Reputation) FROM Users) ORDER BY u.Reputation DESC LIMIT 10)
// SELECT tu.UserId, tu.DisplayName, COALESCE(rp.Title, 'No Posts') AS TopPostTitle, COALESCE(rp.Score, 0) AS TopPostScore, COALESCE(rp.ViewCount, 0) AS TopPostViewCount,
//        COALESCE(rp.Tags, 'No Tags') AS TopPostTags, COALESCE(CAST(rp.Rank AS VARCHAR), 'N/A') AS PostRank, COALESCE(SUM(b.Class), 0) AS TotalBadges
// FROM TopUsers tu LEFT JOIN RankedPosts rp ON tu.UserId = rp.OwnerUserId AND rp.Rank = 1 LEFT JOIN Badges b ON tu.UserId = b.UserId
// GROUP BY tu.UserId, tu.DisplayName, rp.Title, rp.Score, rp.ViewCount, rp.Tags, rp.Rank ORDER BY TotalBadges DESC, TopPostScore DESC LIMIT 5;
//
// The Tags LIKE goes through tag_mentions. The distinct tag names are listed sorted.
fn q20379(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, view_count, .. } = &db.post;
    let (s, n) = (&db.user.reputation).fold_flat((0i64, 0i64), |(s, n), r| (s + r, n + 1));
    let mean = s as f64 / n as f64;
    let tu = top_n(drain(db.user.with((&db.user.reputation).filt(move |r| r as f64 >= mean)).select(&db.user.reputation)), |&(u, r)| (Reverse(r), u), 10);
    let tu: MatSet<Id<User>> = rel(tu.into_iter().map(|x| x.0).collect()).map(|u| u).collect();
    let w = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(owner_user).select(Ident::<Post>::new().and(score).and(creation_date)).window(rank, |((_, s), d)| (Reverse(s), Reverse(d)), asc);
    let first: HashIdx<Id<User>, Id<Post>> = (&w).filt(|(_, k)| k == 1).map(|(((p, _), _), _)| p).collect();
    let tm = tag_mentions(db);
    let by_post: HashIdx<Id<Post>, (Id<Post>, Id<Tag>)> = (&tm).map(|(p, _)| p).inv().collect();
    let tg = (&first).group_by(Ident::<Post>::new()).select((&by_post).map(|(_, t)| t).select(&db.tag.tag_name)).buf_fold(|v| join_sorted(&v));
    type R = Id<Post>;
    let v = drain(
        (&tu)
            .group_by(Ident::<User>::new().and((&first).select(Ident::<Post>::new().and((&tg).opt())).opt()))
            .select(badges_of(db).select(&db.badge.class).opt())
            .fold((0i64, 0i64), |(s, n), c| (s + c.unwrap_or(0), n + c.is_some() as i64)),
    );
    let sc = |r: &Option<(R, Option<Str>)>| r.map_or(0, |(x, _)| score.get(x).unwrap());
    let v = top_n(v, |&((u, r), (s, _))| (Reverse(s), Reverse(sc(&r)), u), 5);
    rows(v.into_iter().map(|((u, r), (s, _))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        match r {
            Some((p, t)) => {
                f.push(V::S(db.post.title.get(p).unwrap_or("No Posts")));
                f.extend([V::I(score.get(p).unwrap()), V::I(view_count.get(p).unwrap_or(0)), V::S(t.unwrap_or("No Tags")), V::S("1")]);
            }
            None => f.extend([V::S("No Posts"), V::I(0), V::I(0), V::S("No Tags"), V::S("N/A")]),
        }
        f.push(V::I(s));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.CreationDate, p.OwnerUserId, u.DisplayName AS OwnerName, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.ViewCount DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1),
// PostTagCounts AS (SELECT p.Id AS PostId, COUNT(DISTINCT t.TagName) AS TagCount FROM Posts p JOIN LATERAL (SELECT unnest(string_to_array(substring(p.Tags, 2, length(p.Tags) - 2), '><')) AS tag) AS tag ON TRUE
//     JOIN Tags t ON t.TagName = tag GROUP BY p.Id),
// PostHistorySummary AS (SELECT ph.PostId, COUNT(*) AS EditCount, MAX(ph.CreationDate) AS LastEditDate FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (4, 5, 6) GROUP BY ph.PostId),
// UserReputation AS (SELECT u.Id AS UserId, SUM(COALESCE(b.Class, 0)) AS TotalBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id)
// SELECT rp.PostId, rp.Title, rp.ViewCount, rp.CreationDate, rp.OwnerName, COALESCE(ptc.TagCount, 0) AS TagCount, COALESCE(phs.EditCount, 0) AS EditCount, COALESCE(phs.LastEditDate, NULL) AS LastEditDate,
//        COALESCE(ur.TotalBadges, 0) AS OwnerTotalBadges
// FROM RankedPosts rp LEFT JOIN PostTagCounts ptc ON rp.PostId = ptc.PostId LEFT JOIN PostHistorySummary phs ON rp.PostId = phs.PostId LEFT JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId
// WHERE rp.Rank = 1 ORDER BY rp.ViewCount DESC FETCH FIRST 10 ROWS ONLY;
//
// Rank and the LIMIT read only base columns, so the ten posts are picked first. The tags joined to Tags are the post's tag edges.
fn q25857(db: &'static So) -> String {
    let Post { post_type_id, owner_user, view_count, tags, .. } = &db.post;
    let w = db.post.with(post_type_id.eq(1)).group_by(owner_user).select(Ident::<Post>::new().and(view_count.opt())).window(row_number, |(p, w)| (w.is_none(), Reverse(w), p), asc);
    let first = drain((&w).filt(|(_, r)| r == 1).map(|(x, _)| x));
    let top: MatSet<Id<Post>> = rel(top_n(first, |&(_, (p, w))| (w.is_none(), Reverse(w), p), 10).into_iter().map(|x| x.1 .0).collect()).collect();
    let ptc = (&top).group_by(Ident::<Post>::new()).select(tags.select(&db.tag.tag_name)).count_distinct();
    let PostHistory { post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let phs = (&top).group_by(Ident::<Post>::new()).select(history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.is_in([4, 5, 6]))).select(hd)).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let ur = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class)).fold(0i64, |s, c| s + c);
    let v = drain((&top).select(Ident::<Post>::new().and((&ptc).opt()).and((&phs).opt()).and(owner_user.select((&ur).opt()))));
    rows(v.into_iter().map(|(_, (((p, t), h), b))| {
        let mut f = post_fields(db, p, &["id", "title", "views", "created", "owner"]);
        f.push(V::I(t.unwrap_or(0)));
        f.extend(match h {
            Some((n, m)) => [V::I(n), V::T(m)],
            None => [V::I(0), V::Null],
        });
        f.push(V::I(b.unwrap_or(0)));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.OwnerUserId),
// PostDetails AS (SELECT rp.Id, rp.Title, rp.CreationDate, rp.Score, rp.CommentCount, rp.UpvoteCount, rp.DownvoteCount,
//        CASE WHEN rp.Score > 0 THEN 'Positive' WHEN rp.Score < 0 THEN 'Negative' ELSE 'Neutral' END AS ScoreType FROM RankedPosts rp WHERE rp.UserPostRank <= 5),
// RecentBadges AS (SELECT b.UserId, STRING_AGG(b.Name, ', ') AS BadgeNames FROM Badges b WHERE b.Date >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY b.UserId),
// FilteredPosts AS (SELECT pd.*, rb.BadgeNames FROM PostDetails pd LEFT JOIN RecentBadges rb ON pd.Id = rb.UserId)
// SELECT fp.Id, fp.Title, fp.CreationDate, fp.Score, fp.CommentCount, fp.UpvoteCount, fp.DownvoteCount, fp.ScoreType, COALESCE(fp.BadgeNames, 'No Badges') AS BadgeInfo
// FROM FilteredPosts fp WHERE fp.ScoreType = 'Positive' AND fp.CommentCount > 0 ORDER BY fp.Score DESC, fp.CreationDate ASC LIMIT 10;
//
// UserPostRank reads only base columns, so the posts are picked first. rb joins on the raw ids: a post id against a user id. The badge names are listed sorted.
fn q20396(db: &'static So) -> String {
    let Post { creation_date, owner_user_id, score, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let pd: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let agg = (&pd)
        .with(score.gt(0))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let Badge { user_id, name, date: bd, .. } = &db.badge;
    let rb = db.badge.with(bd.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(user_id).select(name).buf_fold(|v| join_all(&v, ", "));
    let v = drain((&agg).filt(|a| a[0] > 0).and((&db.post.origid).select(&rb).opt()));
    let v = top_n(v, |&(p, _)| (Reverse(score.get(p).unwrap()), creation_date.get(p).unwrap(), p), 10);
    rows(v.into_iter().map(|(p, (a, b))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::S("Positive"), V::S(b.unwrap_or("No Badges"))]);
        row(f)
    }))
}

// Rewritten (rewrites/2163.sql): PostRank gets p.Id DESC as a tiebreak.
// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC, p.Id DESC) AS PostRank
//     FROM Posts p WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'),
// UserStats AS (SELECT u.Id AS UserId, u.Reputation, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounties
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 8 GROUP BY u.Id, u.Reputation),
// PostLinksCounts AS (SELECT pl.PostId, COUNT(pl.RelatedPostId) AS LinkCount FROM PostLinks pl GROUP BY pl.PostId),
// RecentCloseReasons AS (SELECT ph.PostId, STRING_AGG(cr.Name, ', ') AS CloseReasons FROM PostHistory ph JOIN CloseReasonTypes cr ON CAST(ph.Comment AS INTEGER) = cr.Id WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId)
// SELECT rp.Title, ps.UserId, ps.Reputation, ps.TotalPosts, ps.TotalBounties, pl.LinkCount, rc.CloseReasons
// FROM RankedPosts rp JOIN UserStats ps ON rp.OwnerUserId = ps.UserId LEFT JOIN PostLinksCounts pl ON rp.PostId = pl.PostId LEFT JOIN RecentCloseReasons rc ON rp.PostId = rc.PostId
// WHERE rp.PostRank = 1 ORDER BY ps.Reputation DESC, rp.CreationDate DESC LIMIT 10;
//
// PostRank and the ORDER BY read only base columns, so the ten posts are picked first and UserStats is folded for their owners. The reasons are listed sorted.
fn q2163(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(owner_user).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), Reverse(p)), asc);
    let first = top_n(drain((&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p)).into_iter().map(|(u, p)| (p, u)).collect(), |&(p, u)| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10);
    let fv = rel(first);
    type F = (Id<Post>, Id<User>);
    let us: MatSet<Id<User>> = (&fv).map(|x: F| x.1).collect();
    let v8 = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8)));
    let ub = (&us).group_by(Ident::<User>::new()).select(posts_of(db).select(v8.select((&db.vote.bounty_amount).opt()).opt()).opt()).fold(0i64, |s, b| s + b.flatten().flatten().unwrap_or(0));
    let pc = (&us).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let pl = db.post_link.group_by(&db.post_link.post).select(Ident::<PostLink>::new()).fold(0i64, |n, _| n + 1);
    let reason = close_reasons(db);
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let rc = db
        .post_history
        .with(post_history_type_id.eq(10))
        .group_by(post)
        .select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason))
        .buf_fold(|v| join_all(&v, ", "));
    let v = drain((&fv).select(Same::<F>::new().and(Same::<F>::new().map(|x: F| x.1).select((&ub).and(&pc))).and(Same::<F>::new().map(|x: F| x.0).select((&pl).opt().and((&rc).opt())))));
    let v = top_n(v, |&(_, (((p, u), _), _))| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10);
    rows(v.into_iter().map(|(_, (((p, u), (b, n)), (l, c)))| {
        let mut f = post_fields(db, p, &["title"]);
        f.extend(ucols(db, u, &["uid", "rep"]));
        f.extend([V::I(n), V::I(b), oint(l), ostr(c)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, COALESCE(pt.Name, 'Unknown') AS PostType, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank,
//        SUM(v.BountyAmount) OVER (PARTITION BY p.Id) AS TotalBounty, COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount
//     FROM Posts p LEFT JOIN PostTypes pt ON p.PostTypeId = pt.Id LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (8, 9) LEFT JOIN Comments c ON p.Id = c.PostId
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '6 months'),
// ClosedPosts AS (SELECT ph.PostId, COUNT(ph.Id) AS CloseCount, STRING_AGG(DISTINCT crt.Name) AS CloseReasons FROM PostHistory ph JOIN CloseReasonTypes crt ON ph.Comment::int = crt.Id WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId),
// PostStatistics AS (SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.PostType, rp.Rank, rp.TotalBounty, rp.CommentCount, COALESCE(cp.CloseCount, 0) AS CloseCount,
//        COALESCE(cp.CloseReasons, 'No Close Reasons') AS CloseReasons FROM RankedPosts rp LEFT JOIN ClosedPosts cp ON rp.PostId = cp.PostId)
// SELECT ps.PostId, ps.Title, ps.Score, ps.ViewCount, ps.PostType, ps.Rank, ps.TotalBounty, ps.CommentCount, ps.CloseCount, ps.CloseReasons
// FROM PostStatistics ps WHERE ps.Rank <= 5 AND ps.TotalBounty > 0 AND ps.CommentCount > 5 ORDER BY ps.PostType, ps.Score DESC;
//
// RankedPosts has a row per post, bounty vote and comment, and Rank numbers those rows. The distinct reasons are listed sorted.
fn q2002(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -6)));
    let bv = || votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([8, 9])));
    let w = recent().group_by(post_type_id).select(Ident::<Post>::new().and(score).and(bv().opt()).and(comments_of(db).opt())).window(row_number, |(((p, s), v), c)| (Reverse(s), p, v, c), asc);
    type R = (Id<Post>, i64);
    let rv: MatSet<R> = (&w).filt(|(_, r)| r <= 5).map(|((((p, _), _), _), r)| (p, r)).collect();
    let agg = recent()
        .group_by(Ident::<Post>::new())
        .select(bv().select((&db.vote.bounty_amount).opt()).opt().and(comments_of(db).opt()))
        .fold((0i64, 0i64, 0i64), |(n, s, k), (b, c)| match b.flatten() {
            Some(b) => (n + 1, s + b, k + c.is_some() as i64),
            None => (n, s, k + c.is_some() as i64),
        });
    let reason = close_reasons(db);
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let cp = db
        .post_history
        .with(post_history_type_id.eq(10))
        .group_by(post)
        .select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason))
        .buf_fold(|v| (v.len() as i64, join_sorted_sep(&v, ",")));
    let v = drain((&rv).select(Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0).select((&agg).filt(|(n, s, k)| n > 0 && s > 0 && k > 5).and((&cp).opt())))));
    rows(v.into_iter().map(|(_, ((p, r), ((_, s, k), c)))| {
        let (n, cr) = c.unwrap_or((0, "No Close Reasons"));
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "type"]);
        f.extend([V::I(r), V::I(s), V::I(k), V::I(n), V::S(cr)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank,
//        COALESCE((SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 2), 0) AS UpVoteCount, COALESCE((SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 3), 0) AS DownVoteCount
//     FROM Posts p WHERE p.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 month')),
// TaggedPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.CreationDate, rp.Rank, tp.TagName FROM RankedPosts rp
//     LEFT JOIN (SELECT pt.PostId, t.TagName, COUNT(t.TagName) AS TagCount FROM (SELECT PostId, unnest(string_to_array(Tags, '><')) AS TagName FROM Posts) pt JOIN Tags t ON pt.TagName = t.TagName
//                GROUP BY pt.PostId, t.TagName HAVING COUNT(t.TagName) > 2) tp ON rp.PostId = tp.PostId WHERE rp.Rank <= 5)
// SELECT t.TagName, COUNT(t.PostId) AS PostCount, AVG(rp.ViewCount) AS AvgViewCount, SUM(rp.UpVoteCount - rp.DownVoteCount) AS NetVoteCount
// FROM TaggedPosts t JOIN RankedPosts rp ON t.PostId = rp.PostId GROUP BY t.TagName, rp.ViewCount, rp.UpVoteCount, rp.DownVoteCount
// HAVING AVG(rp.ViewCount) > 100 AND COUNT(t.PostId) > 1 ORDER BY NetVoteCount DESC LIMIT 10;
//
// Posts has no PostId, so DuckDB binds pt.PostId to rp.PostId and runs tp as a LATERAL subquery: each post meets the (tag, count) groups of the whole table's split tags.
fn q20630(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, view_count, tags_str, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(post_type_id).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let rv: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let names: HashIdx<Str, Id<Tag>> = (&db.tag.tag_name).inv().collect();
    let tp = db.post.group_by(tags_str.flat_map(|s: Str| split_on(s, "><")).select(&names).select(&db.tag.tag_name)).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let tpf = (&tp).filt(|n| n > 2);
    let tpu: HashIdx<(), Str> = whole(&tpf).select(Same::<Str>::new()).collect();
    let tagged: MatSet<(Id<Post>, Option<Str>)> = (&rv).select(Ident::<Post>::new().and(Ident::<Post>::new().map(|_: Id<Post>| ()).select(&tpu).opt())).collect();
    let net = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    type T = (Id<Post>, Option<Str>);
    let g = (&tagged)
        .group_by(Same::<T>::new().map(|x: T| x.1).and(Same::<T>::new().map(|x: T| x.0).select(view_count.opt().and((&net).opt()).map(|(w, a)| (w, a.unwrap_or([0, 0]))))))
        .select(Same::<T>::new().map(|x: T| x.0).select(view_count.opt().and((&net).opt())))
        .fold((0i64, 0i64, 0i64, 0i64), |(n, k, s, d), (w, a)| (n + 1, k + w.is_some() as i64, s + w.unwrap_or(0), d + a.map_or(0, |a| a[0] - a[1])));
    let v = drain((&g).filt(|(n, k, s, _)| k > 0 && (s as f64 / k as f64) > 100.0 && n > 1));
    let v = top_n(v, |&(_, (_, _, _, d))| Reverse(d), 10);
    rows(v.into_iter().map(|((t, _), (n, k, s, d))| row(vec![ostr(t), V::I(n), V::F(s as f64 / k as f64), V::I(d)])))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.OwnerUserId, COUNT(c.Id) AS CommentCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS OwnerPostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.OwnerUserId),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, SUM(u.Reputation) AS TotalReputation, RANK() OVER (ORDER BY SUM(u.Reputation) DESC) AS UserRank
//     FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId WHERE u.CreationDate < TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '2 years' GROUP BY u.Id, u.DisplayName)
// SELECT tu.UserId, tu.DisplayName, tu.TotalReputation, rp.Title, rp.CreationDate, rp.Score, CASE WHEN rp.CommentCount > 0 THEN 'Has Comments' ELSE 'No Comments' END AS CommentStatus,
//        COALESCE((SELECT STRING_AGG(pt.Name, ',' ORDER BY pt.Name) FROM PostHistory ph JOIN PostHistoryTypes pt ON ph.PostHistoryTypeId = pt.Id
//                  WHERE ph.PostId = rp.Id AND ph.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 month'), 'No Recent History') AS RecentPostHistory
// FROM TopUsers tu JOIN RankedPosts rp ON tu.UserId = rp.OwnerUserId WHERE tu.UserRank <= 10 ORDER BY tu.TotalReputation DESC, rp.CreationDate DESC LIMIT 50;
fn q3387(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let tu = db.user.with((&db.user.creation_date).lt(add_years(t0, -2))).group_by(Ident::<User>::new()).select((&db.user.reputation).and(posts_of(db))).fold(0i64, |s, (r, _)| s + r);
    let w = whole(&tu).select(Ident::<User>::new().and(&tu)).window(rank, |(_, s)| Reverse(s), asc);
    let top: MatSet<(Id<User>, i64)> = (&w).filt(|(_, k)| k <= 10).map(|(x, _)| x).collect();
    let tidx: HashIdx<Id<User>, (Id<User>, i64)> = (&top).map(|x: (Id<User>, i64)| x.0).inv().collect();
    let rp = drain(db.post.with(creation_date.gt(add_years(t0, -1))).select(owner_user.select(&tidx)));
    let rp = top_n(rp, |&(p, (u, s))| (Reverse(s), Reverse(creation_date.get(p).unwrap()), u, p), 50);
    let rv = rel(rp);
    type R = (Id<Post>, (Id<User>, i64));
    let PostHistory { creation_date: hd, .. } = &db.post_history;
    let rh = (&rv)
        .map(|x: R| x.0)
        .group_by(Ident::<Post>::new())
        .select(history_of(db).select(Ident::<PostHistory>::new().with(hd.gt(add_months(t0, -1)))).select(htype_name(db)))
        .buf_fold(|v| join_all(&v, ","));
    let cc = (&rv).map(|x: R| x.0).group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let v = drain((&rv).select(Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0).select((&cc).opt().and((&rh).opt())))));
    rows(v.into_iter().map(|(_, ((p, (u, s)), (c, h)))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.push(V::I(s));
        f.extend(post_fields(db, p, &["title", "created", "score"]));
        f.push(V::S(if c.unwrap_or(0) > 0 { "Has Comments" } else { "No Comments" }));
        f.push(V::S(h.unwrap_or("No Recent History")));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.Score, p.OwnerUserId, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS ScoreRank
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserStats AS (SELECT u.Id AS UserId, u.Reputation, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(COALESCE(b.Class, 0)) AS TotalBadgeClass
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation),
// ClosedPosts AS (SELECT ph.PostId, STRING_AGG(DISTINCT crt.Name, ', ') AS CloseReasons FROM PostHistory ph JOIN CloseReasonTypes crt ON CAST(ph.Comment AS INTEGER) = crt.Id
//     WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.PostId)
// SELECT up.DisplayName AS UserName, up.Reputation AS UserReputation, rp.Title AS PostTitle, rp.ViewCount AS PostViewCount, rp.Score AS PostScore, ups.TotalPosts AS UserTotalPosts,
//        ups.TotalBadgeClass AS UserTotalBadgeClass, cp.CloseReasons AS ClosedReasons
// FROM RankedPosts rp JOIN Users up ON rp.OwnerUserId = up.Id JOIN UserStats ups ON up.Id = ups.UserId LEFT JOIN ClosedPosts cp ON rp.PostId = cp.PostId
// WHERE rp.ScoreRank = 1 AND ups.TotalPosts > 5 AND (rp.ViewCount > (SELECT AVG(ViewCount) FROM Posts WHERE CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'))
// ORDER BY rp.Score DESC, up.Reputation DESC;
//
// UserStats is folded only for the owners of the rank-1 posts. The distinct reasons are listed sorted.
fn q4490(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, view_count, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let (s, n) = recent().select(view_count).fold_flat((0i64, 0i64), |(s, n), w| (s + w, n + 1));
    let mean = s as f64 / n as f64;
    let w = recent().group_by(owner_user).select(Ident::<Post>::new().and(score)).window(rank, |(_, s)| Reverse(s), asc);
    let first: MatSet<Id<Post>> = (&w).filt(|(_, k)| k == 1).map(|((p, _), _)| p).with(view_count.filt(move |w| w as f64 > mean)).collect();
    let owners: MatSet<Id<User>> = (&first).select(owner_user).collect();
    let bc = (&owners).group_by(Ident::<User>::new()).select(posts_of(db).opt().and(badges_of(db).select(&db.badge.class).opt())).fold(0i64, |s, (_, c)| s + c.unwrap_or(0));
    let pc = (&owners).group_by(Ident::<User>::new()).select(posts_of(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let reason = close_reasons(db);
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let cp = db
        .post_history
        .with(post_history_type_id.is_in([10, 11]))
        .group_by(post)
        .select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason))
        .buf_fold(|v| join_sorted(&v));
    let v = drain((&first).select(Ident::<Post>::new().and(owner_user.select(Ident::<User>::new().and((&pc).filt(|n| n > 5)).and(&bc))).and((&cp).opt())));
    rows(v.into_iter().map(|(_, ((p, ((u, n), b)), c))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["title", "views", "score"]));
        f.extend([V::I(n), V::I(b), ostr(c)]);
        row(f)
    }))
}

// WITH PostStats AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank, p.OwnerUserId
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.OwnerUserId),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id)
// SELECT ps.PostId, ps.Title, ps.CreationDate, ps.ViewCount, ps.Score, ps.CommentCount, ub.BadgeCount, (SELECT COUNT(*) FROM Posts p2 WHERE p2.OwnerUserId = ps.OwnerUserId) AS UserPostsCount,
//        CASE WHEN ps.UpVoteCount > ps.DownVoteCount THEN 'Positive' WHEN ps.UpVoteCount < ps.DownVoteCount THEN 'Negative' ELSE 'Neutral' END AS PostSentiment, COALESCE(t.TagName, 'No Tags') AS TagName
// FROM PostStats ps LEFT JOIN Users u ON ps.OwnerUserId = u.Id LEFT JOIN UserBadges ub ON u.Id = ub.UserId
// LEFT JOIN (SELECT Id, UNNEST(string_to_array(substring(Tags, 2, length(Tags)-2), '><')) AS TagName FROM Posts) t ON ps.PostId = t.Id
// WHERE ps.UserPostRank <= 3 ORDER BY ps.Score DESC, ps.CreationDate DESC LIMIT 100;
//
// UserPostRank and the ORDER BY read only base columns, so the posts are picked first and the joins are driven for them.
fn q2137(db: &'static So) -> String {
    let Post { creation_date, owner_user, owner_user_id, score, tags_str, .. } = &db.post;
    let w = db.post.group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let key = |p: Id<Post>| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p);
    let pick: MatSet<Id<Post>> = rel(top_n(drain((&w).filt(|(_, r)| r <= 3).map(|((p, _), _)| p)), |&(_, p)| key(p), 100).into_iter().map(|x| x.1).collect()).collect();
    let agg = (&pick).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 3], |a, (c, t)| {
        [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let upc = db.post.group_by(owner_user_id).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let v = drain((&agg).and(owner_user.select(&ub).opt()).and(owner_user_id.select(&upc).opt()).and(tags_str.flat_map(tag_list).opt()));
    let v = top_n(v, |&(p, (_, t))| (key(p), t), 100);
    rows(v.into_iter().map(|(p, (((a, b), n), t))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([V::I(a[0]), oint(b), V::I(n.unwrap_or(0))]);
        f.push(V::S(if a[1] > a[2] { "Positive" } else if a[1] < a[2] { "Negative" } else { "Neutral" }));
        f.push(V::S(t.unwrap_or("No Tags")));
        row(f)
    }))
}

// WITH UserVoteStats AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes,
//        COUNT(v.Id) AS TotalVotes FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName),
// PostStats AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, COALESCE(uc.UserCount, 0) AS UniqueCommentCount, COALESCE(uc.UserNames, 'None') AS CommentUserNames,
//        ROW_NUMBER() OVER (ORDER BY p.Score DESC) AS Rank
//     FROM Posts p LEFT JOIN (SELECT PostId, COUNT(DISTINCT UserId) AS UserCount, STRING_AGG(DISTINCT UserDisplayName, ', ') AS UserNames FROM Comments GROUP BY PostId) uc ON p.Id = uc.PostId),
// RankedPosts AS (SELECT ps.*, RANK() OVER (ORDER BY ps.ViewCount DESC) AS ViewRank FROM PostStats ps)
// SELECT uvs.UserId, uvs.DisplayName, p.Title AS PostTitle, p.ViewCount, p.UniqueCommentCount, p.CommentUserNames,
//        CASE WHEN p.Rank = 1 THEN 'Top Post' WHEN p.Rank <= 10 THEN 'Trending Post' ELSE 'Regular Post' END AS PostCategory, uvs.TotalUpVotes, uvs.TotalDownVotes
// FROM UserVoteStats uvs JOIN RankedPosts p ON uvs.UserId = p.PostId WHERE (uvs.TotalUpVotes + uvs.TotalDownVotes > 0) AND p.ViewCount > 50 ORDER BY p.ViewCount DESC, uvs.TotalUpVotes DESC LIMIT 100;
//
// p joins on the raw ids: a user id against a post id. The distinct names are listed sorted.
fn q2174(db: &'static So) -> String {
    let Post { score, view_count, .. } = &db.post;
    let uvs = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let w = whole(db.post.iq()).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let rk: MatSet<(Id<Post>, i64)> = (&w).map(|((p, _), r)| (p, r)).collect();
    let ridx = by_first(&rk);
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let Comment { post, user_id, user_display_name, .. } = &db.comment;
    let ucnt = db.comment.group_by(post).select(user_id).count_distinct();
    let unames = db.comment.group_by(post).select(user_display_name).buf_fold(|v| join_sorted(&v));
    let v = drain(
        (&uvs)
            .filt(|a| a[0] + a[1] > 0)
            .and((&db.user.origid).select(&pidx).select(Ident::<Post>::new().with(view_count.gt(50)).and(&ridx).and((&ucnt).opt().and((&unames).opt())))),
    );
    let v = top_n(v, |&(u, (a, ((p, _), _)))| (Reverse(view_count.get(p).unwrap()), Reverse(a[0]), u), 100);
    rows(v.into_iter().map(|(u, (a, ((p, r), c)))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(post_fields(db, p, &["title", "views"]));
        let (n, s) = c;
        f.extend([V::I(n.unwrap_or(0)), V::S(s.unwrap_or("None"))]);
        f.push(V::S(if r == 1 { "Top Post" } else if r <= 10 { "Trending Post" } else { "Regular Post" }));
        f.extend([V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS rn, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN vote.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN vote.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
//     FROM Posts p LEFT JOIN Comments c ON c.PostId = p.Id LEFT JOIN Votes vote ON vote.PostId = p.Id GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.PostTypeId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.CommentCount, rp.UpVoteCount, rp.DownVoteCount,
//        CASE WHEN rp.CommentCount > 10 THEN 'Highly Discussed' WHEN rp.CommentCount BETWEEN 5 AND 10 THEN 'Moderately Discussed' ELSE 'Less Discussed' END AS DiscussionStatus,
//        CASE WHEN rp.Score > 100 THEN 'Hot' WHEN rp.Score BETWEEN 50 AND 100 THEN 'Trending' ELSE 'Cold' END AS PopularityStatus,
//        (SELECT COUNT(DISTINCT b.UserId) FROM Badges b WHERE b.UserId IN (SELECT DISTINCT p.OwnerUserId FROM Posts p WHERE p.Id = rp.PostId)) AS UniqueBadgeCount,
//        (SELECT STRING_AGG(DISTINCT lt.Name, ', ') FROM LinkTypes lt JOIN PostLinks pl ON pl.LinkTypeId = lt.Id WHERE pl.PostId = rp.PostId) AS RelatedPostTypes
// FROM RankedPosts rp WHERE rp.rn = 1 AND rp.Score IS NOT NULL AND rp.ViewCount > (SELECT AVG(ViewCount) FROM Posts WHERE ViewCount IS NOT NULL) ORDER BY rp.Score DESC, rp.ViewCount DESC;
//
// rn reads only base columns, so the posts are picked first. The distinct link type names are listed sorted.
fn q24594(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user_id, view_count, .. } = &db.post;
    let (s, n) = view_count.fold_flat((0i64, 0i64), |(s, n), w| (s + w, n + 1));
    let mean = s as f64 / n as f64;
    let w = db.post.group_by(post_type_id).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let fs: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let agg = (&fs).with(view_count.filt(move |w| w as f64 > mean)).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 3], |a, (c, t)| {
        [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let by_uid: HashIdx<i64, Id<Badge>> = (&db.badge.user_id).inv().collect();
    let ubc = (&fs).group_by(Ident::<Post>::new()).select(owner_user_id.select(&by_uid).select(&db.badge.user_id)).count_distinct();
    let lt = (&fs).group_by(Ident::<Post>::new()).select(links_of(db).select(&db.post_link.link_type).select(&db.link_type.name)).buf_fold(|v| join_sorted(&v));
    let v = drain((&agg).and((&ubc).opt()).and((&lt).opt()));
    rows(v.into_iter().map(|(p, ((a, b), l))| {
        let s = db.post.score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        f.push(V::S(if a[0] > 10 { "Highly Discussed" } else if a[0] >= 5 { "Moderately Discussed" } else { "Less Discussed" }));
        f.push(V::S(if s > 100 { "Hot" } else if s >= 50 { "Trending" } else { "Cold" }));
        f.extend([V::I(b.unwrap_or(0)), ostr(l)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.ViewCount, u.DisplayName AS OwnerDisplayName, ROW_NUMBER() OVER (PARTITION BY p.Tags ORDER BY p.CreationDate DESC) AS rn, p.Tags
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'),
// PostStats AS (SELECT rp.PostId, rp.Title, rp.OwnerDisplayName, rp.ViewCount, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount FROM RankedPosts rp LEFT JOIN Comments c ON rp.PostId = c.PostId LEFT JOIN Votes v ON rp.PostId = v.PostId
//     WHERE rp.rn = 1 GROUP BY rp.PostId, rp.Title, rp.OwnerDisplayName, rp.ViewCount),
// PopularTags AS (SELECT unnest(string_to_array(Tags, '><')) AS TagName FROM Posts WHERE PostTypeId = 1),
// TagPopularity AS (SELECT TagName, COUNT(*) AS TagCount FROM PopularTags GROUP BY TagName ORDER BY TagCount DESC LIMIT 10)
// SELECT ps.PostId, ps.Title, ps.OwnerDisplayName, ps.ViewCount, ps.CommentCount, ps.UpVoteCount, ps.DownVoteCount, tg.TagName
// FROM PostStats ps JOIN TagPopularity tg ON ps.Title LIKE '%' || tg.TagName || '%' ORDER BY ps.UpVoteCount DESC, ps.ViewCount DESC;
fn q28949(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, tags_str, title, .. } = &db.post;
    let w = db
        .post
        .with(post_type_id.eq(1).and(creation_date.ge(add_years(date(2024, 10, 1), -1))))
        .with(owner_user)
        .group_by(tags_str.opt())
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (Reverse(d), p), asc);
    let fs: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let agg = (&fs).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 3], |a, (c, t)| {
        [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let tc = db.post.with(post_type_id.eq(1)).group_by(tags_str.flat_map(|s: Str| split_on(s, "><"))).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let top = rel(top_n(drain(&tc), |&(t, n)| (Reverse(n), t), 10));
    let tn: HashIdx<Str, (Str, i64)> = (&top).map(|x: (Str, i64)| x.0).inv().select(&top).collect();
    let v = drain((&agg).and(title.select_where(&tn, |t: Str, n: Str| like_sub(t, n))));
    rows(v.into_iter().map(|(p, (a, (t, _)))| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::S(t)]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT Id, DisplayName, Reputation, CASE WHEN Reputation > 1000 THEN 'High' WHEN Reputation BETWEEN 500 AND 1000 THEN 'Medium' ELSE 'Low' END AS ReputationLevel FROM Users),
// PostStatistics AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, COALESCE(p.AnswerCount, 0) AS AnswerCount, COALESCE(p.ViewCount, 0) AS ViewCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank, p.OwnerUserId FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// ClosedPosts AS (SELECT ph.PostId, ph.CreationDate, ph.Comment FROM PostHistory ph JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id WHERE pht.Name = 'Post Closed'),
// CombinedResults AS (SELECT ur.DisplayName, ur.ReputationLevel, ps.Title, ps.CreationDate, ps.Score, ps.AnswerCount, ps.ViewCount, cp.Comment AS ClosureComments
//     FROM PostStatistics ps JOIN UserReputation ur ON ps.OwnerUserId = ur.Id LEFT JOIN ClosedPosts cp ON ps.PostId = cp.PostId WHERE ur.ReputationLevel = 'High')
// SELECT DisplayName, COUNT(Title) AS TotalPosts, AVG(Score) AS AvgScore, SUM(COALESCE(AnswerCount, 0)) AS TotalAnswers, MAX(ViewCount) AS MaxViews, STRING_AGG(ClosureComments, ', ') AS ClosureRemarks
// FROM CombinedResults GROUP BY DisplayName HAVING COUNT(Title) > 5 ORDER BY AvgScore DESC LIMIT 10;
//
// PostRank is never read. The remarks are listed sorted.
fn q3672(db: &'static So) -> String {
    let Post { creation_date, owner_user, title, score, answer_count, view_count, .. } = &db.post;
    let closed = history_of(db).select(Ident::<PostHistory>::new().with(htype_name(db).eq("Post Closed"))).select((&db.post_history.comment).opt());
    let g = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(1000))))
        .group_by(owner_user.select(&db.user.display_name))
        .select(title.opt().and(score).and(answer_count.opt()).and(view_count.opt()).and(closed.opt()))
        .buf_fold(|v| {
            let rem: Vec<Str> = v.iter().filter_map(|x| x.1.flatten()).collect();
            (
                v.iter().filter(|x| x.0 .0 .0 .0.is_some()).count() as i64,
                v.iter().map(|x| x.0 .0 .0 .1).sum::<i64>(),
                v.len() as i64,
                v.iter().map(|x| x.0 .0 .1.unwrap_or(0)).sum::<i64>(),
                v.iter().map(|x| x.0 .1.unwrap_or(0)).max().unwrap(),
                if rem.is_empty() { None } else { Some(join_all(&rem, ", ")) },
            )
        });
    let v = drain((&g).filt(|x| x.0 > 5));
    let v = top_n(v, |&(k, (_, s, n, _, _, _))| (Reverse(fkey(s as f64 / n as f64)), k), 10);
    rows(v.into_iter().map(|(k, (t, s, n, a, m, r))| row(vec![V::S(k), V::I(t), avg(s, n), V::I(a), V::I(m), ostr(r)])))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U),
// PostStatistics AS (SELECT P.OwnerUserId, COUNT(P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
//        AVG(P.Score) AS AvgScore FROM Posts P GROUP BY P.OwnerUserId),
// TopUsers AS (SELECT UR.UserId, UR.DisplayName, PS.TotalPosts, PS.QuestionCount, PS.AnswerCount, PS.AvgScore FROM UserReputation UR JOIN PostStatistics PS ON UR.UserId = PS.OwnerUserId WHERE UR.ReputationRank <= 10),
// RecentBadges AS (SELECT B.UserId, COUNT(B.Id) AS BadgeCount FROM Badges B WHERE B.Date > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' GROUP BY B.UserId)
// SELECT TU.DisplayName, TU.TotalPosts, TU.QuestionCount, TU.AnswerCount, COALESCE(RB.BadgeCount, 0) AS RecentBadgeCount, TU.AvgScore,
//        (SELECT COUNT(*) FROM Comments C WHERE C.UserId = TU.UserId) AS TotalComments,
//        (SELECT STRING_AGG(CAST(PH.UserDisplayName AS VARCHAR), ', ') FROM PostHistory PH WHERE PH.UserId = TU.UserId AND PH.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year') AS RecentEditors
// FROM TopUsers TU LEFT JOIN RecentBadges RB ON TU.UserId = RB.UserId ORDER BY TU.AvgScore DESC, TU.TotalPosts DESC;
//
// The joins on UserId are on the raw ids. The editor names are listed sorted.
fn q2024(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let w = whole(db.user.iq()).select(Ident::<User>::new().and(&db.user.reputation)).window(rank, |(_, r)| Reverse(r), asc);
    let tu: MatSet<Id<User>> = (&w).filt(|(_, k)| k <= 10).map(|((u, _), _)| u).collect();
    let Post { owner_user_id, post_type_id, score, .. } = &db.post;
    let ps = db.post.group_by(owner_user_id).select(post_type_id.and(score)).fold([0i64; 4], |a, (t, s)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s]);
    let rb = db.badge.with((&db.badge.date).gt(add_days(t0, -30))).group_by(&db.badge.user_id).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let tc = db.comment.group_by(&db.comment.user_id).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let PostHistory { user_id, creation_date: hd, user_display_name, .. } = &db.post_history;
    let re = db.post_history.with(hd.ge(add_years(t0, -1))).group_by(user_id).select(user_display_name.opt()).buf_fold(|v| {
        let n: Vec<Str> = v.iter().filter_map(|&x| x).collect();
        if n.is_empty() { None } else { Some(join_all(&n, ", ")) }
    });
    let v = drain((&tu).select((&db.user.origid).select((&ps).and((&rb).opt()).and((&tc).opt()).and((&re).opt()))));
    rows(v.into_iter().map(|(u, (((a, b), c), e))| {
        row(vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(b.unwrap_or(0)), V::F(a[3] as f64 / a[0] as f64), V::I(c.unwrap_or(0)), ostr(e.flatten())])
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AcceptedAnswerId, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount, SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounties, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName),
// ClosedPosts AS (SELECT ph.PostId, ph.CreationDate, STRING_AGG(CONCAT('Closed by: ', ph.UserDisplayName, ' at ', ph.CreationDate), '; ') AS CloseDetails FROM PostHistory ph
//     WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId, ph.CreationDate)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, ua.DisplayName, ua.BadgeCount, ua.TotalBounties, ua.Upvotes, ua.Downvotes, cp.CloseDetails
// FROM RankedPosts rp LEFT JOIN UserActivity ua ON rp.AcceptedAnswerId = ua.UserId LEFT JOIN ClosedPosts cp ON rp.PostId = cp.PostId
// WHERE rp.Rank <= 5 AND (ua.BadgeCount > 2 OR ua.TotalBounties > 0) AND COALESCE(cp.CloseDetails, '') != '' ORDER BY rp.Score DESC, ua.BadgeCount DESC;
//
// ua joins on the raw ids: an accepted answer id against a user id. Rank reads only base columns, so the posts are picked first and UserActivity is folded for the users they meet.
fn q23077(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, accepted_answer_id, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(post_type_id).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let top: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let met: MatSet<Id<User>> = (&top).select(accepted_answer_id.select(&uidx)).collect();
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let ua = (&met)
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(votes_by(db).select(vote_type_id.and(bounty_amount.opt())).opt()))
        .fold([0i64; 4], |a, (b, v)| match v {
            Some((t, x)) => [a[0] + b.is_some() as i64, a[1] + x.unwrap_or(0), a[2] + (t == 2) as i64, a[3] + (t == 3) as i64],
            None => [a[0] + b.is_some() as i64, a[1], a[2], a[3]],
        });
    let PostHistory { post, post_history_type_id, creation_date: hd, user_display_name, .. } = &db.post_history;
    let cp = db.post_history.with(post_history_type_id.eq(10)).group_by(post.and(hd)).select(user_display_name.opt().and(hd)).buf_fold(|v| {
        let parts: Vec<Str> = v.iter().map(|&(n, d)| &*Box::leak(format!("Closed by: {} at {}", n.unwrap_or(""), ts_text(d)).into_boxed_str())).collect();
        join_all(&parts, "; ")
    });
    let cpv = rel(drain(&cp));
    let by_post: HashIdx<Id<Post>, Str> = (&cpv).map(|x: ((Id<Post>, i64), Str)| x.0 .0).inv().select((&cpv).map(|x: ((Id<Post>, i64), Str)| x.1)).collect();
    let v = drain((&top).select(Ident::<Post>::new().and(accepted_answer_id.select(&uidx).select(Ident::<User>::new().and((&ua).filt(|a| a[0] > 2 || a[1] > 0)))).and((&by_post).filt(|s: Str| !s.is_empty()))));
    rows(v.into_iter().map(|(_, ((p, (u, a)), c))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.push(user_col(db, u, "name"));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::S(c)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, STRING_AGG(DISTINCT t.TagName, ', ') AS Tags, COUNT(c.Id) AS CommentCount, COUNT(a.Id) AS AnswerCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes, p.OwnerUserId, p.CreationDate
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2 LEFT JOIN PostLinks pl ON pl.PostId = p.Id LEFT JOIN Tags t ON t.Id = pl.RelatedPostId
//     LEFT JOIN Votes v ON v.PostId = p.Id WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Body, p.OwnerUserId, p.CreationDate),
// UserRankings AS (SELECT u.Id AS UserId, u.DisplayName, RANK() OVER (ORDER BY SUM(rp.TotalUpvotes) - SUM(rp.TotalDownvotes) DESC) AS UserRank FROM Users u INNER JOIN RankedPosts rp ON u.Id = rp.OwnerUserId GROUP BY u.Id, u.DisplayName)
// SELECT ur.UserId, ur.DisplayName, ur.UserRank, rp.PostId, rp.Title, rp.Tags, rp.CommentCount, rp.AnswerCount, rp.TotalUpvotes, rp.TotalDownvotes
// FROM UserRankings ur JOIN RankedPosts rp ON ur.UserId = rp.OwnerUserId WHERE ur.UserRank <= 10 ORDER BY ur.UserRank, rp.CreationDate DESC;
//
// The distinct tag names are listed sorted.
fn q29378(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, .. } = &db.post;
    let rp = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.eq(1)));
    let tag_by_id: HashIdx<i64, Id<Tag>> = (&db.tag.origid).inv().collect();
    let lt = || links_of(db).select((&db.post_link.related_post_id).select(&tag_by_id).opt()).opt();
    let agg = rp()
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(answers_of(db).opt()).and(lt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 4], |a, (((c, x), _), t)| [a[0] + c.is_some() as i64, a[1] + x.is_some() as i64, a[2] + (t == Some(2)) as i64, a[3] + (t == Some(3)) as i64]);
    let tg = rp().group_by(Ident::<Post>::new()).select(links_of(db).select(&db.post_link.related_post_id).select(&tag_by_id).select(&db.tag.tag_name)).buf_fold(|v| join_sorted(&v));
    let ur = rp().group_by(owner_user).select(&agg).fold(0i64, |s, a| s + a[2] - a[3]);
    let w = whole(&ur).select(Ident::<User>::new().and(&ur)).window(rank, |(_, s)| Reverse(s), asc);
    let top: MatSet<(Id<User>, i64)> = (&w).filt(|(_, k)| k <= 10).map(|((u, _), k)| (u, k)).collect();
    let tidx: HashIdx<Id<User>, (Id<User>, i64)> = (&top).map(|x: (Id<User>, i64)| x.0).inv().collect();
    let v = drain(rp().select(Ident::<Post>::new().and(owner_user.select(&tidx)).and(&agg).and((&tg).opt())));
    rows(v.into_iter().map(|(_, (((p, (u, k)), a), t))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.push(V::I(k));
        f.extend(post_fields(db, p, &["id", "title"]));
        f.push(ostr(t));
        f.extend(a.iter().map(|&x| V::I(x)));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.ViewCount, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        CASE WHEN p.PostTypeId = 1 THEN 'Question' WHEN p.PostTypeId = 2 THEN 'Answer' ELSE 'Other' END AS PostType, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.ViewCount DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.Body, p.CreationDate, p.ViewCount, u.DisplayName, p.PostTypeId),
// TopPosts AS (SELECT PostId, Title, Body, CreationDate, ViewCount, OwnerDisplayName, CommentCount, PostType FROM RankedPosts WHERE Rank <= 5),
// TopTags AS (SELECT unnest(string_to_array(Tags, '><')) AS TagName FROM Posts WHERE Id IN (SELECT PostId FROM TopPosts)),
// TagUsage AS (SELECT TagName, COUNT(*) AS UsageCount FROM TopTags GROUP BY TagName ORDER BY UsageCount DESC)
// SELECT tp.Title, tp.OwnerDisplayName, tp.ViewCount, tp.CommentCount, tu.TagName, tu.UsageCount
// FROM TopPosts tp JOIN TagUsage tu ON EXISTS (SELECT 1 FROM Posts WHERE Tags LIKE '%' || tu.TagName || '%' AND Id = tp.PostId) ORDER BY tp.ViewCount DESC, tu.UsageCount DESC;
fn q29583(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, view_count, tags_str, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(current_date(), -1))).with(owner_user).group_by(post_type_id).select(Ident::<Post>::new().and(view_count.opt())).window(row_number, |(p, w)| (w.is_none(), Reverse(w), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let tu = (&tp).group_by(tags_str.flat_map(|s: Str| split_on(s, "><"))).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let tv: MatSet<(Str, i64)> = whole(&tu).select(Same::<Str>::new().and(&tu)).collect();
    let tn: HashIdx<Str, (Str, i64)> = (&tv).map(|x: (Str, i64)| x.0).inv().collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let v = drain((&cc).and(tags_str.select_where(&tn, |t: Str, n: Str| like_sub(t, n))));
    rows(v.into_iter().map(|(p, (c, (t, n)))| {
        let mut f = post_fields(db, p, &["title", "owner", "views"]);
        f.extend([V::I(c), V::S(t), V::I(n)]);
        row(f)
    }))
}

// WITH UserVoteStats AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        COUNT(DISTINCT p.Id) AS TotalPosts, COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS TotalQuestions, COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS TotalAnswers
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId WHERE u.Reputation > 100 GROUP BY u.Id, u.DisplayName),
// PostHistoryDetails AS (SELECT ph.PostId, STRING_AGG(DISTINCT pht.Name, ', ') AS ChangeTypes, COUNT(DISTINCT ph.Id) AS HistoryCount, MAX(ph.CreationDate) AS LastChangeDate
//     FROM PostHistory ph JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id GROUP BY ph.PostId),
// TopPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS rnk FROM Posts p
//     WHERE p.CreationDate >= CURRENT_TIMESTAMP - INTERVAL '1 year')
// SELECT uvs.UserId, uvs.DisplayName, uvs.UpVotes, uvs.DownVotes, uvs.TotalPosts, uvs.TotalQuestions, uvs.TotalAnswers, ph.ChangeTypes, ph.HistoryCount, ph.LastChangeDate, tp.PostId, tp.Title, tp.Score, tp.ViewCount
// FROM UserVoteStats uvs LEFT JOIN PostHistoryDetails ph ON uvs.UserId = ph.PostId JOIN TopPosts tp ON tp.PostId = ph.PostId WHERE tp.rnk <= 5 ORDER BY uvs.UpVotes DESC, uvs.DownVotes ASC, uvs.TotalPosts DESC;
//
// ph joins on the raw ids: a user id against a post id. rnk reads only base columns, so the posts are picked first and UserVoteStats is folded for the users they meet.
// The distinct change types are listed sorted.
fn q24009(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, view_count, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(utc_to_ny(now_utc()), -1)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(view_count.opt()))
        .window(row_number, |((p, s), w)| (Reverse(s), w.is_none(), Reverse(w), p), asc);
    let tp: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|(((p, _), _), _)| p).collect();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let met: MatSet<Id<User>> = (&tp).with(history_of(db)).select((&db.post.origid).select(&uidx).select(Ident::<User>::new().with((&db.user.reputation).gt(100)))).collect();
    let uv = (&met).group_by(Ident::<User>::new()).select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt()).fold([0i64; 2], |a, t| {
        let t = t.flatten();
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
    });
    let pc = (&met).group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id).opt()).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64],
        None => a,
    });
    let phd = (&tp).group_by(Ident::<Post>::new()).select(history_of(db).select(htype_name(db).and(&db.post_history.creation_date))).buf_fold(|v| {
        (join_sorted(&v.iter().map(|x| x.0).collect::<Vec<_>>()), v.len() as i64, v.iter().map(|x| x.1).max().unwrap())
    });
    let v = drain((&tp).select(Ident::<Post>::new().and(&phd).and((&db.post.origid).select(&uidx).select(Ident::<User>::new().and(&uv).and(&pc)))));
    rows(v.into_iter().map(|(_, ((p, (s, n, m)), ((u, a), c)))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c[0]), V::I(c[1]), V::I(c[2]), V::S(s), V::I(n), V::T(m)]);
        f.extend(post_fields(db, p, &["id", "title", "score", "views"]));
        row(f)
    }))
}

// WITH ProcessedTags AS (SELECT p.Id AS PostId, unnest(string_to_array(substring(p.Tags, 2, length(p.Tags) - 2), '><')) AS Tag FROM Posts p WHERE p.PostTypeId = 1),
// TagFrequency AS (SELECT Tag, COUNT(*) AS Frequency FROM ProcessedTags GROUP BY Tag HAVING COUNT(*) > 5),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalQuestions, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS UpvotedQuestions,
//        SUM(CASE WHEN p.Score < 0 THEN 1 ELSE 0 END) AS DownvotedQuestions, COUNT(DISTINCT c.Id) AS TotalComments
//     FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON c.PostId = p.Id WHERE p.PostTypeId = 1 GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT ua.UserId, ua.DisplayName, ua.TotalQuestions, ua.UpvotedQuestions, ua.DownvotedQuestions, ua.TotalComments, ROW_NUMBER() OVER (ORDER BY ua.TotalQuestions DESC) AS UserRank
//     FROM UserActivity ua WHERE ua.TotalQuestions > 10)
// SELECT tu.DisplayName, tu.TotalQuestions, tu.UpvotedQuestions, tu.DownvotedQuestions, tu.TotalComments, tf.Tag, tf.Frequency
// FROM TopUsers tu JOIN TagFrequency tf ON tf.Tag IN (SELECT Tag FROM ProcessedTags pt WHERE pt.PostId IN (SELECT p.Id FROM Posts p WHERE p.OwnerUserId = tu.UserId))
// ORDER BY tu.UserRank, tf.Frequency DESC;
//
// UserRank only orders the output, which has no LIMIT.
fn q26111(db: &'static So) -> String {
    let Post { post_type_id, tags_str, score, .. } = &db.post;
    let qs = || posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1)));
    let tf = db.post.with(post_type_id.eq(1)).group_by(tags_str.flat_map(tag_list)).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(qs().select(score.and(comments_of(db).opt())))
        .fold([0i64; 3], |a, (s, c)| [a[0] + (s > 0) as i64, a[1] + (s < 0) as i64, a[2] + c.is_some() as i64]);
    let qc = db.user.group_by(Ident::<User>::new()).select(qs()).fold(0i64, |n, _| n + 1);
    let ut: MatSet<(Id<User>, Str)> = db.user.select(Ident::<User>::new().and(qs().select(tags_str.flat_map(tag_list)))).collect();
    let ut_by: HashIdx<Id<User>, (Id<User>, Str)> = (&ut).map(|x: (Id<User>, Str)| x.0).inv().collect();
    let v = drain((&qc).filt(|n| n > 10).and(&ua).and((&ut_by).map(|x: (Id<User>, Str)| x.1).select(Same::<Str>::new().and((&tf).filt(|n| n > 5)))));
    rows(v.into_iter().map(|(u, ((n, a), (t, f)))| row(vec![user_col(db, u, "name"), V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::S(t), V::I(f)])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, u.DisplayName),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, OwnerDisplayName, CommentCount, UpVotes, DownVotes FROM RankedPosts WHERE rn = 1 ORDER BY Score DESC, CreationDate DESC LIMIT 10),
// PostStats AS (SELECT tp.Title, tp.OwnerDisplayName, tp.CreationDate, tp.Score, tp.CommentCount, tp.UpVotes, tp.DownVotes, COALESCE(h.Comment, 'No history') AS PostHistory
//     FROM TopPosts tp LEFT JOIN PostHistory h ON tp.PostId = h.PostId WHERE h.CreationDate >= CURRENT_TIMESTAMP - INTERVAL '30 days')
// SELECT Title, OwnerDisplayName, CreationDate, Score, CommentCount, UpVotes, DownVotes, STRING_AGG(PostHistory, ', ') AS RecentActions
// FROM PostStats GROUP BY Title, OwnerDisplayName, CreationDate, Score, CommentCount, UpVotes, DownVotes ORDER BY Score DESC, CreationDate DESC;
//
// rn partitions by the post itself, so it is 1 on every row. TopPosts reads only base columns, so the ten posts are picked first. The actions are listed sorted.
fn q6726(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, title, owner_user, .. } = &db.post;
    let top = top_n(drain(db.post.with(post_type_id.eq(1)).select(score)), |&(p, s)| (Reverse(s), Reverse(creation_date.get(p).unwrap()), p), 10);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let agg = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 3], |a, (c, t)| {
        [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let PostHistory { creation_date: hd, comment, .. } = &db.post_history;
    let since = add_days(utc_to_ny(now_utc()), -30);
    type K = (((Option<Str>, Option<Str>), (i64, i64)), [i64; 3]);
    let g = (&tp)
        .group_by(title.opt().and(owner_user.select(&db.user.display_name).opt()).and(creation_date.and(score)).and(&agg))
        .select(history_of(db).select(Ident::<PostHistory>::new().with(hd.ge(since))).select(comment.opt()))
        .buf_fold(|v| join_all(&v.iter().map(|c| c.unwrap_or("No history")).collect::<Vec<_>>(), ", "));
    let v = drain(&g);
    rows(v.into_iter().map(|((((t, o), (d, sc)), a), s): (K, Str)| {
        row(vec![ostr(t), ostr(o), V::T(d), V::I(sc), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::S(s)])
    }))
}

// WITH PostTags AS (SELECT p.Id AS PostId, unnest(string_to_array(substring(p.Tags, 2, length(p.Tags) - 2), '><')) AS Tag FROM Posts p WHERE p.PostTypeId = 1),
// UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TagActivity AS (SELECT pt.Tag, COUNT(DISTINCT p.Id) AS PostCount, COUNT(DISTINCT c.Id) AS CommentCount, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews, SUM(COALESCE(p.Score, 0)) AS TotalScore
//     FROM PostTags pt JOIN Posts p ON pt.PostId = p.Id LEFT JOIN Comments c ON c.PostId = p.Id GROUP BY pt.Tag)
// SELECT ta.Tag, ta.PostCount, ta.CommentCount, ta.TotalViews, ta.TotalScore, COALESCE(ur.DisplayName, 'No Users') AS TopUser, COALESCE(ur.Reputation, 0) AS TopUserReputation, ur.BadgeCount AS TopUserBadgeCount
// FROM TagActivity ta LEFT JOIN (SELECT pt.Tag, u.DisplayName, u.Reputation, b.BadgeCount, ROW_NUMBER() OVER(PARTITION BY pt.Tag ORDER BY u.Reputation DESC) AS rn
//     FROM PostTags pt JOIN Posts p ON pt.PostId = p.Id JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN UserReputation b ON u.Id = b.UserId) ur ON ta.Tag = ur.Tag AND ur.rn = 1
// ORDER BY ta.TotalViews DESC, ta.TotalScore DESC FETCH FIRST 10 ROWS ONLY;
//
// The ORDER BY reads only TagActivity, so the ten tags are picked first and their top user found. A tie on the top reputation goes to the smaller user id.
fn q25439(db: &'static So) -> String {
    let Post { post_type_id, tags_str, view_count, score, owner_user, .. } = &db.post;
    let qs = || db.post.with(post_type_id.eq(1));
    let ta = qs().group_by(tags_str.flat_map(tag_list)).select(view_count.opt().and(score).and(comments_of(db).opt())).fold([0i64; 2], |a, ((w, s), _)| [a[0] + w.unwrap_or(0), a[1] + s]);
    let pc = qs().group_by(tags_str.flat_map(tag_list)).select(Ident::<Post>::new()).count_distinct();
    let cc = qs().group_by(tags_str.flat_map(tag_list)).select(comments_of(db)).count_distinct();
    let top = top_n(drain((&ta).and(&pc).and((&cc).opt())), |&(t, ((a, _), _))| (Reverse(a[0]), Reverse(a[1]), t), 10);
    let tv = rel(top);
    type T = (Str, (([i64; 2], i64), Option<i64>));
    let names: MatSet<Str> = (&tv).map(|x: T| x.0).collect();
    let tq: HashIdx<Str, Id<Post>> = qs().select(tags_str.flat_map(tag_list)).inv().collect();
    let uw = (&names).group_by(Same::<Str>::new()).select((&tq).select(owner_user).select(Ident::<User>::new().and(&db.user.reputation))).window(row_number, |(u, r)| (Reverse(r), u), asc);
    let ur: HashIdx<Str, Id<User>> = (&uw).filt(|(_, k)| k == 1).map(|((u, _), _)| u).collect();
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = drain((&tv).select(Same::<T>::new().and(Same::<T>::new().map(|x: T| x.0).select((&ur).select(Ident::<User>::new().and(&bc))).opt())));
    rows(v.into_iter().map(|(_, ((t, ((a, n), c)), u))| {
        let mut f = vec![V::S(t), V::I(n), V::I(c.unwrap_or(0)), V::I(a[0]), V::I(a[1])];
        f.extend(match u {
            Some((u, b)) => [user_col(db, u, "name"), user_col(db, u, "rep"), V::I(b)],
            None => [V::S("No Users"), V::I(0), V::Null],
        });
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.Tags, u.DisplayName AS OwnerDisplayName, p.CreationDate, p.Score, RANK() OVER (PARTITION BY p.Tags ORDER BY p.Score DESC) AS TagRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.Body, rp.Tags, rp.OwnerDisplayName, rp.CreationDate, rp.Score FROM RankedPosts rp WHERE rp.TagRank = 1),
// CommentStatistics AS (SELECT PostId, COUNT(*) AS CommentCount, MAX(CreationDate) AS LastCommentDate FROM Comments GROUP BY PostId),
// PostWithComments AS (SELECT tp.*, cs.CommentCount, cs.LastCommentDate FROM TopPosts tp LEFT JOIN CommentStatistics cs ON tp.PostId = cs.PostId)
// SELECT pwc.PostId, pwc.Title, pwc.Body, pwc.Tags, pwc.OwnerDisplayName, pwc.CreationDate, pwc.Score, COALESCE(pwc.CommentCount, 0) AS TotalComments, pwc.LastCommentDate, STRING_AGG(DISTINCT pt.Name, ', ') AS PostTypeNames
// FROM PostWithComments pwc JOIN PostTypes pt ON pt.Id = (SELECT PostTypeId FROM Posts WHERE Id = pwc.PostId)
// GROUP BY pwc.PostId, pwc.Title, pwc.Body, pwc.Tags, pwc.OwnerDisplayName, pwc.CreationDate, pwc.Score, pwc.CommentCount, pwc.LastCommentDate ORDER BY pwc.Score DESC, pwc.CreationDate DESC LIMIT 50;
fn q26202(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, tags_str, score, .. } = &db.post;
    let w = db
        .post
        .with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))))
        .with(owner_user)
        .group_by(tags_str.opt())
        .select(Ident::<Post>::new().and(score))
        .window(rank, |(_, s)| Reverse(s), asc);
    let first = top_n(drain((&w).filt(|(_, k)| k == 1).map(|((p, _), _)| p)).into_iter().map(|x| x.1).collect(), |&p| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 50);
    let tp: MatSet<Id<Post>> = rel(first).collect();
    let cs = db.comment.group_by(&db.comment.post).select(&db.comment.creation_date).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let v = drain((&tp).select(Ident::<Post>::new().and((&cs).opt()).and(ptype_name(db))));
    rows(v.into_iter().map(|(_, ((p, c), t))| {
        let mut f = post_fields(db, p, &["id", "title", "body", "tags", "owner", "created", "score"]);
        f.extend(match c {
            Some((n, m)) => [V::I(n), V::T(m)],
            None => [V::I(0), V::Null],
        });
        f.push(V::S(t));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.ViewCount, P.Score, P.OwnerUserId, U.DisplayName AS OwnerDisplayName, ROW_NUMBER() OVER (PARTITION BY P.PostTypeId ORDER BY P.CreationDate DESC) AS Rank
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// TopPosts AS (SELECT PostId, Title, CreationDate, ViewCount, Score, OwnerDisplayName FROM RankedPosts WHERE Rank <= 10),
// UserVotes AS (SELECT V.PostId, COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVotes FROM Votes V GROUP BY V.PostId),
// PostHistory AS (SELECT PH.PostId, ARRAY_AGG(PH.Comment) AS HistoryComments, COUNT(PH.Id) AS EditCount FROM PostHistory PH WHERE PH.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY PH.PostId)
// SELECT TP.PostId, TP.Title, TP.CreationDate, TP.ViewCount, TP.Score, TP.OwnerDisplayName, COALESCE(UV.UpVotes, 0) AS UpVotes, COALESCE(UV.DownVotes, 0) AS DownVotes,
//        COALESCE(PH.HistoryComments, ARRAY[]::TEXT[]) AS HistoryComments, COALESCE(PH.EditCount, 0) AS EditCount
// FROM TopPosts TP LEFT JOIN UserVotes UV ON TP.PostId = UV.PostId LEFT JOIN PostHistory PH ON TP.PostId = PH.PostId WHERE TP.Score >= 10 ORDER BY TP.ViewCount DESC, TP.Score DESC;
//
// The comments in each list are sorted.
fn q32702(db: &'static So) -> String {
    let Post { creation_date, owner_user, post_type_id, score, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let w = db.post.with(creation_date.ge(add_years(t0, -1))).with(owner_user).group_by(post_type_id).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let top: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|((p, _), _)| p).collect();
    let uv = db.vote.group_by(&db.vote.post_id).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let PostHistory { post, creation_date: hd, comment, .. } = &db.post_history;
    let ph = db.post_history.with(hd.ge(add_years(t0, -1))).group_by(post).select(comment.opt()).buf_fold(|v| {
        let mut x: Vec<Option<Str>> = v.to_vec();
        x.sort_unstable();
        &*Box::leak(x.into_boxed_slice())
    });
    let v = drain((&top).with(score.ge(10)).select(Ident::<Post>::new().and((&db.post.origid).select(&uv).opt()).and((&ph).opt())));
    rows(v.into_iter().map(|(_, ((p, a), h))| {
        let a = a.unwrap_or([0, 0]);
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1])]);
        let h: &[Option<Str>] = h.unwrap_or(&[]);
        f.push(V::L(h.iter().map(|&x| ostr(x)).collect()));
        f.push(V::I(h.len() as i64));
        row(f)
    }))
}

// Rewritten (rewrites/26678.sql): the comment STRING_AGG gets ORDER BY c.Id.
// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Tags, p.CreationDate, p.OwnerUserId, u.DisplayName AS OwnerDisplayName, COUNT(a.Id) AS AnswerCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Posts a ON a.ParentId = p.Id LEFT JOIN Votes v ON v.PostId = p.Id WHERE p.PostTypeId = 1
//     GROUP BY p.Id, p.Title, p.Tags, p.CreationDate, p.OwnerUserId, u.DisplayName),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.Tags, rp.CreationDate, rp.OwnerUserId, rp.OwnerDisplayName, rp.AnswerCount, rp.UpVotes, rp.DownVotes FROM RankedPosts rp
//     WHERE rp.rn = 1 AND rp.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days')
// SELECT p.PostId, p.Title, p.Tags, p.CreationDate, p.OwnerDisplayName, p.AnswerCount, p.UpVotes, p.DownVotes,
//        COALESCE(ROUND((CAST(p.UpVotes AS FLOAT) / NULLIF((p.UpVotes + p.DownVotes), 0)) * 100, 2), 0) AS UpVotePercentage,
//        COALESCE(ROUND((CAST(p.DownVotes AS FLOAT) / NULLIF((p.UpVotes + p.DownVotes), 0)) * 100, 2), 0) AS DownVotePercentage,
//        (SELECT STRING_AGG(c.Text, ' | ' ORDER BY c.Id) FROM Comments c WHERE c.PostId = p.PostId) AS CommentSummary
// FROM FilteredPosts p ORDER BY p.UpVotes DESC, p.CreationDate DESC;
//
// rn partitions by the post itself, so it is 1 on every row. CAST AS FLOAT is f32, so the percentages are f32.
fn q26678(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let fp = || db.post.with(post_type_id.eq(1).and(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))));
    let agg = fp().group_by(Ident::<Post>::new()).select(children_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 3], |a, (c, t)| {
        [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let cs = fp().group_by(Ident::<Post>::new()).select(comments_of(db).select((&db.comment.origid).and(&db.comment.text))).buf_fold(|v| {
        let mut x: Vec<(i64, Str)> = v.to_vec();
        x.sort_unstable();
        join_seq(&x.iter().map(|y| y.1).collect::<Vec<_>>(), " | ")
    });
    let v = drain((&agg).and((&cs).opt()));
    let pct = |x: i64, n: i64| if n == 0 { V::F(0.0) } else { V::F(((((x as f32) / (n as f32) * 100.0f32) as f64 * 100.0).round() / 100.0) as f32 as f64) };
    rows(v.into_iter().map(|(p, (a, c))| {
        let mut f = post_fields(db, p, &["id", "title", "tags", "created", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), pct(a[1], a[1] + a[2]), pct(a[2], a[1] + a[2]), ostr(c)]);
        row(f)
    }))
}

fn join_seq(v: &[Str], sep: &str) -> Str {
    Box::leak(v.join(sep).into_boxed_str())
}

// WITH RECURSIVE UserScoreCTE AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.CreationDate, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COUNT(p.Id) AS PostsCount
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation, u.CreationDate),
// RankedUsers AS (SELECT UserId, DisplayName, Reputation, CreationDate, UpVotes - DownVotes AS NetVotes, PostsCount, RANK() OVER (ORDER BY UpVotes DESC) AS VoteRank FROM UserScoreCTE),
// ClosedPostInfo AS (SELECT h.UserDisplayName, h.CreationDate, COUNT(h.Id) AS ClosedPostsCount, STRING_AGG(DISTINCT pt.Name, ', ') AS CloseReasons FROM PostHistory h
//     JOIN CloseReasonTypes pt ON CAST(h.Comment AS INTEGER) = pt.Id WHERE h.PostHistoryTypeId IN (10, 11) GROUP BY h.UserDisplayName, h.CreationDate),
// UserEngagement AS (SELECT u.UserId, u.DisplayName, u.NetVotes, COALESCE(c.ClosedPostsCount, 0) AS ClosedPostsCount, c.CloseReasons FROM RankedUsers u
//     LEFT JOIN ClosedPostInfo c ON u.DisplayName = c.UserDisplayName AND u.CreationDate = c.CreationDate)
// SELECT ue.DisplayName, ue.NetVotes, ue.ClosedPostsCount, ue.CloseReasons, ROUND(CAST(ue.NetVotes AS numeric) / NULLIF(ue.ClosedPostsCount, 0), 2) AS VotesPerClosedPostRatio
// FROM UserEngagement ue WHERE ue.NetVotes > 0 ORDER BY VotesPerClosedPostRatio DESC LIMIT 10;
//
// The CTE is not recursive. The distinct reasons are listed sorted.
fn q34051(db: &'static So) -> String {
    let us = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt().and(posts_of(db).opt())).fold(0i64, |n, (t, _)| n + (t == Some(2)) as i64 - (t == Some(3)) as i64);
    let reason = close_reasons(db);
    let PostHistory { post_history_type_id, comment, user_display_name, creation_date: hd, .. } = &db.post_history;
    let cpi = db
        .post_history
        .with(post_history_type_id.is_in([10, 11]))
        .group_by(user_display_name.and(hd))
        .select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason))
        .buf_fold(|v| (v.len() as i64, join_sorted(&v)));
    let v = drain((&us).filt(|n| n > 0).and((&db.user.display_name).and(&db.user.creation_date).select(&cpi).opt()));
    let ratio = |n: i64, c: Option<(i64, Str)>| c.map(|(k, _)| (n as f64 / k as f64 * 100.0).round() / 100.0);
    let v = top_n(v, |&(u, (n, c))| {
        let r = ratio(n, c);
        (r.is_none(), Reverse(r.map(fkey)), u)
    }, 10);
    rows(v.into_iter().map(|(u, (n, c))| {
        let mut f = vec![user_col(db, u, "name"), V::I(n)];
        match c {
            Some((k, s)) => f.extend([V::I(k), V::S(s)]),
            None => f.extend([V::I(0), V::Null]),
        }
        f.push(ratio(n, c).map_or(V::Null, V::F));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, U.DisplayName AS Author, ROW_NUMBER() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC, P.CreationDate DESC) AS Rank
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id WHERE P.CreationDate >= '2024-10-01 12:34:56'::timestamp - INTERVAL '1 year'),
// PostVoteSummary AS (SELECT V.PostId, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes V GROUP BY V.PostId),
// PostCloseReasons AS (SELECT PH.PostId, STRING_AGG(CRT.Name, ', ') AS CloseReasons FROM PostHistory PH JOIN CloseReasonTypes CRT ON CAST(PH.Comment AS INTEGER) = CRT.Id WHERE PH.PostHistoryTypeId IN (10, 11) GROUP BY PH.PostId),
// RecentPostsWithDetails AS (SELECT RP.PostId, RP.Title, RP.Author, RP.CreationDate, RP.Score, PS.UpVotes, PS.DownVotes, PCR.CloseReasons FROM RankedPosts RP
//     LEFT JOIN PostVoteSummary PS ON RP.PostId = PS.PostId LEFT JOIN PostCloseReasons PCR ON RP.PostId = PCR.PostId WHERE RP.Rank <= 5)
// SELECT RPD.PostId, RPD.Title, RPD.Author, RPD.CreationDate, RPD.Score, COALESCE(RPD.UpVotes, 0) AS UpVotes, COALESCE(RPD.DownVotes, 0) AS DownVotes, COALESCE(RPD.CloseReasons, 'No close reasons') AS CloseReasons
// FROM RecentPostsWithDetails RPD ORDER BY RPD.CreationDate DESC;
//
// The reasons are listed sorted.
fn q30078(db: &'static So) -> String {
    let Post { creation_date, owner_user, post_type_id, score, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let top: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|(((p, _), _), _)| p).collect();
    let pvs = db.vote.group_by(&db.vote.post_id).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let reason = close_reasons(db);
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let pcr = db
        .post_history
        .with(post_history_type_id.is_in([10, 11]))
        .group_by(post)
        .select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason))
        .buf_fold(|v| join_all(&v, ", "));
    let v = drain((&top).select(Ident::<Post>::new().and((&db.post.origid).select(&pvs).opt()).and((&pcr).opt())));
    rows(v.into_iter().map(|(_, ((p, a), c))| {
        let a = a.unwrap_or([0, 0]);
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "score"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::S(c.unwrap_or("No close reasons"))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.ViewCount, COUNT(c.Id) AS CommentCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.Score, p.CreationDate, p.ViewCount, p.PostTypeId),
// UserBadges AS (SELECT b.UserId, COUNT(*) AS BadgeCount, STRING_AGG(b.Name, ', ') AS BadgeNames FROM Badges b GROUP BY b.UserId),
// PostVoteDetails AS (SELECT v.PostId, COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes, COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes, COUNT(CASE WHEN v.VoteTypeId = 10 THEN 1 END) AS DeletionVotes
//     FROM Votes v GROUP BY v.PostId)
// SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.CommentCount, COALESCE(ub.BadgeCount, 0) AS UserBadgeCount, COALESCE(ub.BadgeNames, 'No Badges') AS UserBadges, pvd.UpVotes, pvd.DownVotes, pvd.DeletionVotes,
//        CASE WHEN rp.ViewCount > 1000 THEN 'Popular' WHEN rp.Score > 50 THEN 'Highly Voted' ELSE 'Regular' END AS PostType,
//        CASE WHEN EXISTS (SELECT 1 FROM Posts p2 WHERE p2.AcceptedAnswerId = rp.PostId) THEN 'Answered' ELSE 'Unanswered' END AS AnswerStatus
// FROM RankedPosts rp LEFT JOIN Users u ON rp.PostId = u.Id LEFT JOIN UserBadges ub ON u.Id = ub.UserId LEFT JOIN PostVoteDetails pvd ON rp.PostId = pvd.PostId
// WHERE rp.Rank <= 10 ORDER BY rp.Score DESC, rp.CreationDate DESC;
//
// u joins on the raw ids: a post id against a user id. The badge names are listed sorted.
fn q33053(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, view_count, accepted_answer, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(current_date(), -1)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(score).and(creation_date))
        .window(row_number, |((p, s), d)| (Reverse(s), Reverse(d), p), asc);
    let top: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|(((p, _), _), _)| p).collect();
    let cc = (&top).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.name).buf_fold(|v| (v.len() as i64, join_all(&v, ", ")));
    let pvd = db.vote.group_by(&db.vote.post_id).select(&db.vote.vote_type_id).fold([0i64; 3], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + (t == 10) as i64]);
    let answered: MatSet<Id<Post>> = db.post.select(accepted_answer).collect();
    let v = drain((&cc).and((&db.post.origid).select(&uidx).select(&ub).opt()).and((&db.post.origid).select(&pvd).opt()).and(Ident::<Post>::new().with(&answered).opt()));
    rows(v.into_iter().map(|(p, (((c, b), d), a))| {
        let (n, s) = b.unwrap_or((0, "No Badges"));
        let mut f = post_fields(db, p, &["id", "title", "score", "views"]);
        f.extend([V::I(c), V::I(n), V::S(s)]);
        f.extend(match d {
            Some(d) => [V::I(d[0]), V::I(d[1]), V::I(d[2])],
            None => [V::Null, V::Null, V::Null],
        });
        let w = view_count.get(p);
        f.push(V::S(if w.map_or(false, |w| w > 1000) { "Popular" } else if score.get(p).unwrap() > 50 { "Highly Voted" } else { "Regular" }));
        f.push(V::S(if a.is_some() { "Answered" } else { "Unanswered" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.PostTypeId IN (1, 2)
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId),
// PostStats AS (SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.CommentCount, rp.UserPostRank, (rp.UpVoteCount - rp.DownVoteCount) AS NetVoteScore FROM RankedPosts rp WHERE rp.UserPostRank = 1),
// ClosedPostReasons AS (SELECT ph.PostId, STRING_AGG(cr.Name, ', ') AS CloseReasons FROM PostHistory ph INNER JOIN CloseReasonTypes cr ON cr.Id = CAST(ph.Comment AS int) WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId)
// SELECT ps.Title, ps.Score, ps.ViewCount, ps.CommentCount, ps.NetVoteScore, COALESCE(cpr.CloseReasons, 'No closure reasons') AS CloseReason
// FROM PostStats ps LEFT JOIN ClosedPostReasons cpr ON ps.PostId = cpr.PostId WHERE ps.NetVoteScore > 0 AND ps.CommentCount > 10 ORDER BY ps.NetVoteScore DESC, ps.ViewCount DESC LIMIT 10;
//
// UserPostRank reads only base columns, so the posts are picked first. The reasons are listed sorted.
fn q24850(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user_id, view_count, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.is_in([1, 2])))
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (Reverse(d), Reverse(p)), asc);
    let first: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let agg = (&first).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 2], |a, (c, t)| {
        [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64 - (t == Some(3)) as i64]
    });
    let reason = close_reasons(db);
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let cpr = db
        .post_history
        .with(post_history_type_id.eq(10))
        .group_by(post)
        .select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason))
        .buf_fold(|v| join_all(&v, ", "));
    let v = drain((&agg).filt(|a| a[1] > 0 && a[0] > 10).and((&cpr).opt()));
    let v = top_n(v, |&(p, (a, _))| {
        let w = view_count.get(p);
        (Reverse(a[1]), w.is_none(), Reverse(w), p)
    }, 10);
    rows(v.into_iter().map(|(p, (a, c))| {
        let mut f = post_fields(db, p, &["title", "score", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::S(c.unwrap_or("No closure reasons"))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, COALESCE((SELECT SUM(v.BountyAmount) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId IN (8, 9)), 0) AS TotalBounty,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS PostRank FROM Posts p WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'),
// UserBadges AS (SELECT u.Id AS UserId, STRING_AGG(b.Name, ', ') AS BadgeNames FROM Users u JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// ClosedPosts AS (SELECT p.Id, ph.Comment AS CloseReason, ph.CreationDate AS ClosedDate FROM Posts p JOIN PostHistory ph ON p.Id = ph.PostId WHERE ph.PostHistoryTypeId = 10),
// PostLinkCounts AS (SELECT pl.PostId, COUNT(pl.RelatedPostId) AS RelatedPostCount FROM PostLinks pl GROUP BY pl.PostId)
// SELECT rp.PostId, rp.Title, rp.Score, rp.TotalBounty, ub.BadgeNames, COALESCE(c.CloseReason, 'Not Closed') AS CloseReason, COALESCE(c.ClosedDate::date, NULL) AS ClosedDate, plc.RelatedPostCount
// FROM RankedPosts rp LEFT JOIN UserBadges ub ON ub.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = rp.PostId) LEFT JOIN ClosedPosts c ON c.Id = rp.PostId LEFT JOIN PostLinkCounts plc ON plc.PostId = rp.PostId
// WHERE rp.PostRank <= 5 AND (rp.TotalBounty > 0 OR c.ClosedDate IS NOT NULL) ORDER BY rp.Score DESC, rp.TotalBounty DESC;
//
// The badge names are listed sorted.
fn q24981(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, owner_user_id, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1))).group_by(post_type_id).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let top: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let tb = (&top).group_by(Ident::<Post>::new()).select(votes_of(db).select(Ident::<Vote>::new().with(vote_type_id.is_in([8, 9]))).select(bounty_amount)).fold(0i64, |s, b| s + b);
    let ub = db.badge.group_by(&db.badge.user_id).select(&db.badge.name).buf_fold(|v| join_all(&v, ", "));
    let PostHistory { post_history_type_id, comment, creation_date: hd, .. } = &db.post_history;
    let cp = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.eq(10))).select(comment.opt().and(hd));
    let plc = db.post_link.group_by(&db.post_link.post).select(Ident::<PostLink>::new()).fold(0i64, |n, _| n + 1);
    type J = ((Option<i64>, Option<(Option<Str>, i64)>), Option<i64>);
    let v = drain(
        (&top)
            .select((&tb).opt().and(cp.opt()).and((&plc).opt()))
            .filt(|((b, c), _): J| b.unwrap_or(0) > 0 || c.is_some())
            .and(owner_user_id.select(&ub).opt()),
    );
    rows(v.into_iter().map(|(p, (((b, c), l), u))| {
        let mut f = post_fields(db, p, &["id", "title", "score"]);
        f.extend([V::I(b.unwrap_or(0)), ostr(u)]);
        f.extend(match c {
            Some((r, d)) => [V::S(r.unwrap_or("Not Closed")), V::D(trunc_day(d))],
            None => [V::S("Not Closed"), V::Null],
        });
        f.push(oint(l));
        row(f)
    }))
}

// WITH PostDetails AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COALESCE(p.AcceptedAnswerId, -1) AS AcceptedAnswerId, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RecentPostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 YEAR'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.AcceptedAnswerId, p.OwnerUserId),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, STRING_AGG(b.Name, ', ') AS BadgeNames FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// TopPosts AS (SELECT pd.PostId, pd.Title, pd.CreationDate, pd.CommentCount, pd.UpVotes, pd.DownVotes, u.DisplayName, ub.BadgeCount, ub.BadgeNames,
//        RANK() OVER (ORDER BY pd.UpVotes - pd.DownVotes DESC, pd.CommentCount DESC) AS PostRank
//     FROM PostDetails pd JOIN Users u ON pd.AcceptedAnswerId = u.Id LEFT JOIN UserBadges ub ON u.Id = ub.UserId WHERE pd.RecentPostRank = 1)
// SELECT tp.Title, tp.CommentCount, tp.UpVotes, tp.DownVotes, tp.DisplayName, tp.BadgeCount, tp.BadgeNames FROM TopPosts tp WHERE tp.PostRank <= 10 ORDER BY tp.UpVotes - tp.DownVotes DESC, tp.CommentCount DESC;
//
// u joins on the raw ids: an accepted answer id (or the -1 sentinel, which is the Community user) against a user id. The badge names are listed sorted.
fn q938(db: &'static So) -> String {
    let Post { creation_date, owner_user_id, accepted_answer_id, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), Reverse(p)), asc);
    let first: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let agg = (&first).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 3], |a, (c, t)| {
        [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.name).opt()).buf_fold(|v| {
        let n: Vec<Str> = v.iter().filter_map(|&x| x).collect();
        (n.len() as i64, if n.is_empty() { None } else { Some(join_all(&n, ", ")) })
    });
    let aid = accepted_answer_id.opt().map(|a: Option<i64>| a.unwrap_or(-1));
    let tp = (&agg).and(aid.select(&uidx).select(Ident::<User>::new().and(&ub)));
    let w = whole(&tp).select(Ident::<Post>::new().and(&tp)).window(rank, |(_, (a, _))| (Reverse(a[1] - a[2]), Reverse(a[0])), asc);
    rows(drain((&w).filt(|(_, k)| k <= 10)).into_iter().map(|(_, ((p, (a, (u, (n, s)))), _))| {
        let mut f = post_fields(db, p, &["title"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), user_col(db, u, "name"), V::I(n), ostr(s)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn,
//        COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) OVER (PARTITION BY p.Id) AS UpvoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) OVER (PARTITION BY p.Id) AS DownvoteCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserBadges AS (SELECT b.UserId, COUNT(*) AS BadgeCount, STRING_AGG(b.Name, ', ') AS BadgeNames FROM Badges b GROUP BY b.UserId),
// CloseReasons AS (SELECT ph.PostId, STRING_AGG(cr.Name, ', ' ORDER BY ph.CreationDate DESC) AS CloseReasons FROM PostHistory ph JOIN CloseReasonTypes cr ON CAST(ph.Comment AS INT) = cr.Id
//     WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId)
// SELECT up.DisplayName, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.CommentCount, rp.UpvoteCount, rp.DownvoteCount, ub.BadgeCount, ub.BadgeNames, cr.CloseReasons
// FROM RankedPosts rp JOIN Users up ON rp.PostId = up.Id LEFT JOIN UserBadges ub ON up.Id = ub.UserId LEFT JOIN CloseReasons cr ON rp.PostId = cr.PostId
// WHERE (rp.Score > 10 OR rp.ViewCount > 100) AND (ub.BadgeCount > 1 OR cr.CloseReasons IS NOT NULL) ORDER BY rp.Score DESC, rp.ViewCount DESC;
//
// RankedPosts keeps a row per post, comment and vote; rn is never read. Users joins on the raw ids: a post id against a user id. The badge names are listed sorted.
fn q33974(db: &'static So) -> String {
    let Post { creation_date, score, view_count, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(score.gt(10).or(view_count.gt(100)));
    let agg = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 3], |a, (c, t)| {
        [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.name).buf_fold(|v| (v.len() as i64, join_all(&v, ", ")));
    let reason = close_reasons(db);
    let PostHistory { post, post_history_type_id, comment, creation_date: hd, .. } = &db.post_history;
    let cr = db
        .post_history
        .with(post_history_type_id.eq(10))
        .group_by(post)
        .select(hd.and(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason)))
        .buf_fold(|v| {
            let mut x: Vec<(i64, Str)> = v.to_vec();
            x.sort_by_key(|y| Reverse(y.0));
            join_seq(&x.iter().map(|y| y.1).collect::<Vec<_>>(), ", ")
        });
    type J = ((Id<User>, Option<(i64, Str)>), Option<Str>);
    let v = drain(
        recent()
            .select(Ident::<Post>::new().and(comments_of(db).opt()).and(votes_of(db).opt()))
            .select(
                Same::<((Id<Post>, Option<Id<Comment>>), Option<Id<Vote>>)>::new()
                    .map(|x: ((Id<Post>, Option<Id<Comment>>), Option<Id<Vote>>)| x.0 .0)
                    .select(Ident::<Post>::new().and(&agg).and((&db.post.origid).select(&uidx).select(Ident::<User>::new().and((&ub).opt())).and((&cr).opt()).filt(|(( _, b), c): J| b.map_or(false, |b| b.0 > 1) || c.is_some()))),
            ),
    );
    let v = top_n(v, |&(_, ((p, _), _))| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w))
    }, 0);
    rows(v.into_iter().map(|(_, ((p, a), ((u, b), c)))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title", "created", "score", "views"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        f.extend(match b {
            Some((n, s)) => [V::I(n), V::S(s)],
            None => [V::Null, V::Null],
        });
        f.push(ostr(c));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.ViewCount, p.Score, STRING_AGG(t.TagName, ', ') AS Tags,
//        ROW_NUMBER() OVER (PARTITION BY CASE WHEN pt.Name = 'Question' THEN 'Questions' WHEN pt.Name = 'Answer' THEN 'Answers' ELSE 'Other' END ORDER BY p.CreationDate DESC, p.Id) AS RN
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id LEFT JOIN Tags t ON t.ExcerptPostId = p.Id WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 YEAR'
//     GROUP BY p.Id, p.Title, p.Body, p.CreationDate, p.ViewCount, p.Score, pt.Name),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN pt.Name = 'Answer' THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN pt.Name = 'Question' THEN 1 ELSE 0 END) AS TotalQuestions
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN PostTypes pt ON p.PostTypeId = pt.Id WHERE u.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '3 MONTH' GROUP BY u.Id, u.DisplayName)
// SELECT u.DisplayName, u.TotalPosts, u.TotalAnswers, u.TotalQuestions, rp.PostId, rp.Title, rp.Tags, rp.ViewCount, rp.Score
// FROM UserActivity u JOIN RankedPosts rp ON u.TotalPosts > 0 WHERE rp.RN <= 5 ORDER BY u.TotalPosts DESC, rp.ViewCount DESC;
//
// RN reads only base columns, so the posts are picked first. The tag names are listed sorted.
// (uses rewrites/26105.sql: p.Id breaks the CreationDate tie in RN)
fn q26105(db: &'static So) -> String {
    let Post { creation_date, .. } = &db.post;
    let cat = |n: Str| if n == "Question" { 0 } else if n == "Answer" { 1 } else { 2 };
    let w = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(ptype_name(db).map(cat)).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let top: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let excerpt: HashIdx<Id<Post>, Id<Tag>> = (&db.tag.excerpt_post).inv().collect();
    let tg = (&top).group_by(Ident::<Post>::new()).select((&excerpt).select(&db.tag.tag_name)).buf_fold(|v| join_all(&v, ", "));
    let ua = db
        .user
        .with((&db.user.creation_date).ge(add_months(ts(2024, 10, 1, 12, 34, 56), -3)))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(ptype_name(db)))
        .fold([0i64; 3], |a, n| [a[0] + 1, a[1] + (n == "Answer") as i64, a[2] + (n == "Question") as i64]);
    let mut v = Vec::new();
    (&ua).cross((&top).select(Ident::<Post>::new().and((&tg).opt()))).drive(|(u, _), (a, (p, t))| v.push((u, a, p, t)));
    rows(v.into_iter().map(|(u, a, p, t)| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2])];
        f.extend(post_fields(db, p, &["id", "title"]));
        f.push(ostr(t));
        f.extend(post_fields(db, p, &["views", "score"]));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.ViewCount DESC) AS Rank FROM Posts p
//     WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year' AND p.PostTypeId = 1),
// VoteSummary AS (SELECT v.PostId, SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes,
//        COUNT(CASE WHEN vt.Name IN ('Close', 'Reopen') THEN 1 END) AS CloseVotes, COUNT(*) AS TotalVotes FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY v.PostId),
// PostTags AS (SELECT p.Id AS PostId, ARRAY_AGG(t.TagName) AS Tags FROM Posts p LEFT JOIN Tags t ON t.ExcerptPostId = p.Id WHERE p.PostTypeId = 1 GROUP BY p.Id)
// SELECT rp.PostId, rp.Title, rp.ViewCount, ps.UpVotes, ps.DownVotes, ps.CloseVotes, ps.TotalVotes, COALESCE(pt.Tags, ARRAY['No Tags']) AS Tags,
//        CASE WHEN ps.TotalVotes = 0 THEN 'No Votes' ELSE CASE WHEN ps.UpVotes > ps.DownVotes THEN 'Positive' WHEN ps.UpVotes < ps.DownVotes THEN 'Negative' ELSE 'Neutral' END END AS VoteSentiment,
//        CASE WHEN rp.Rank <= 5 THEN 'Top Post' ELSE 'Regular Post' END AS PostCategory
// FROM RankedPosts rp LEFT JOIN VoteSummary ps ON rp.PostId = ps.PostId LEFT JOIN PostTags pt ON rp.PostId = pt.PostId
// WHERE COALESCE(ps.CloseVotes, 0) < 2 ORDER BY rp.ViewCount DESC, rp.Title ASC OFFSET 3 LIMIT 10;
fn q20100(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user_id, view_count, title, .. } = &db.post;
    let vk = |p: Id<Post>| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w))
    };
    let w = db
        .post
        .with(creation_date.ge(add_years(date(2024, 10, 1), -1)).and(post_type_id.eq(1)))
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(view_count.opt()))
        .window(row_number, |(p, w)| (w.is_none(), Reverse(w), p), asc);
    type R = (Id<Post>, i64);
    let rv: MatSet<R> = (&w).map(|((p, _), r)| (p, r)).collect();
    let vs = db.vote.group_by(&db.vote.post_id).select(vtype_name(db)).fold([0i64; 4], |a, n| {
        [a[0] + (n == "UpMod") as i64, a[1] + (n == "DownMod") as i64, a[2] + (n == "Close" || n == "Reopen") as i64, a[3] + 1]
    });
    let excerpt: HashIdx<Id<Post>, Id<Tag>> = (&db.tag.excerpt_post).inv().collect();
    let pt = db.post.with(post_type_id.eq(1)).group_by(Ident::<Post>::new()).select((&excerpt).select(&db.tag.tag_name).opt()).buf_fold(|v| &*Box::leak(v.to_vec().into_boxed_slice()));
    type J = (Option<[i64; 4]>, Option<&'static [Option<Str>]>);
    let v = drain((&rv).select(Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0).select((&db.post.origid).select(&vs).opt().and((&pt).opt())).filt(|(a, _): J| a.map_or(0, |a| a[2]) < 2))));
    let v = top_n(v, |&(_, ((p, _), _))| (vk(p), title.get(p).is_none(), title.get(p), p), 13);
    rows(v.into_iter().skip(3).map(|(_, ((p, r), (a, t)))| {
        let mut f = post_fields(db, p, &["id", "title", "views"]);
        match a {
            Some(a) => f.extend(a.iter().map(|&x| V::I(x))),
            None => f.extend([V::Null, V::Null, V::Null, V::Null]),
        }
        f.push(match t {
            Some(t) => V::L(t.iter().map(|&x| ostr(x)).collect()),
            None => V::L(vec![V::S("No Tags")]),
        });
        f.push(V::S(match a {
            Some(a) if a[3] == 0 => "No Votes",
            Some(a) if a[0] > a[1] => "Positive",
            Some(a) if a[0] < a[1] => "Negative",
            _ => "Neutral",
        }));
        f.push(V::S(if r <= 5 { "Top Post" } else { "Regular Post" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, P.AnswerCount, P.CommentCount, U.DisplayName AS OwnerDisplayName, RANK() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC) AS PostRank
//     FROM Posts P JOIN Users U ON P.OwnerUserId = U.Id WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT PostId, Title, OwnerDisplayName, Score, ViewCount, AnswerCount, CommentCount FROM RankedPosts WHERE PostRank <= 10),
// PostComments AS (SELECT C.PostId, COUNT(C.Id) AS TotalComments, STRING_AGG(C.Text, ' | ' ORDER BY C.CreationDate) AS CommentsText FROM Comments C GROUP BY C.PostId),
// PostVoteSummary AS (SELECT P.Id AS PostId, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY P.Id)
// SELECT TP.Title, TP.OwnerDisplayName, TP.Score, TP.ViewCount, TP.AnswerCount, TP.CommentCount, COALESCE(PC.TotalComments, 0) AS TotalComments, COALESCE(PC.CommentsText, 'No comments') AS CommentsText,
//        COALESCE(PVS.UpVotes, 0) AS UpVotes, COALESCE(PVS.DownVotes, 0) AS DownVotes
// FROM TopPosts TP LEFT JOIN PostComments PC ON TP.PostId = PC.PostId LEFT JOIN PostVoteSummary PVS ON TP.PostId = PVS.PostId WHERE TP.Score > 0 ORDER BY TP.Score DESC, TP.ViewCount DESC;
fn q33204(db: &'static So) -> String {
    let Post { creation_date, owner_user, post_type_id, score, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user).group_by(post_type_id).select(Ident::<Post>::new().and(score)).window(rank, |(_, s)| Reverse(s), asc);
    let top: MatSet<Id<Post>> = (&w).filt(|(_, k)| k <= 10).map(|((p, _), _)| p).collect();
    let Comment { creation_date: cd, text, origid, .. } = &db.comment;
    let pc = (&top).group_by(Ident::<Post>::new()).select(comments_of(db).select(cd.and(origid).and(text))).buf_fold(|v| {
        let mut x: Vec<((i64, i64), Str)> = v.to_vec();
        x.sort_unstable();
        (x.len() as i64, join_seq(&x.iter().map(|y| y.1).collect::<Vec<_>>(), " | "))
    });
    let pvs = (&top).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let v = drain((&top).with(score.gt(0)).select(Ident::<Post>::new().and((&pc).opt()).and(&pvs)));
    rows(v.into_iter().map(|(_, ((p, c), a))| {
        let mut f = post_fields(db, p, &["title", "owner", "score", "views", "answers", "comments"]);
        let (n, s) = c.unwrap_or((0, "No comments"));
        f.extend([V::I(n), V::S(s), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH UserScoreSummary AS (SELECT U.Id AS UserId, U.DisplayName, SUM(V.BountyAmount) AS TotalBountyAmount, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts,
//        AVG(P.Score) AS AverageScore FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId IN (8, 9) GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, RANK() OVER (ORDER BY TotalBountyAmount DESC, TotalPosts DESC) AS UserRank FROM UserScoreSummary),
// RecentPostHistory AS (SELECT PH.PostId, PH.UserId, PH.CreationDate, PH.Comment, ROW_NUMBER() OVER (PARTITION BY PH.PostId ORDER BY PH.CreationDate DESC) AS rn FROM PostHistory PH WHERE PH.PostHistoryTypeId IN (10, 12, 14)),
// AggregateComments AS (SELECT C.PostId, STRING_AGG(C.Text, ' | ') AS AllComments FROM Comments C GROUP BY C.PostId)
// SELECT U.DisplayName, U.TotalBountyAmount, U.TotalPosts, U.PositivePosts, U.AverageScore, COALESCE(RPH.Comment, 'No Recent Actions') AS RecentActivity, COALESCE(AC.AllComments, 'No Comments') AS PostComments
// FROM UserScoreSummary U LEFT JOIN TopUsers T ON U.UserId = T.UserId LEFT JOIN RecentPostHistory RPH ON U.UserId = RPH.UserId AND RPH.rn = 1 LEFT JOIN AggregateComments AC ON RPH.PostId = AC.PostId
// WHERE U.TotalPosts > 5 AND U.AverageScore <= 0 AND U.UserId IN (SELECT UserId FROM Badges WHERE Class = 1) ORDER BY U.TotalBountyAmount DESC, U.TotalPosts DESC;
//
// TopUsers has a row for every user, so its join changes nothing. The comments are listed in id order.
fn q24704(db: &'static So) -> String {
    let gold: MatSet<Id<User>> = db.badge.with((&db.badge.class).eq(1)).select(&db.badge.user).collect();
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let bv = votes_of(db).select(Ident::<Vote>::new().with(vote_type_id.is_in([8, 9]))).select(bounty_amount.opt());
    let us = (&gold).group_by(Ident::<User>::new()).select(posts_of(db).select((&db.post.score).and(bv.opt()))).fold([0i64; 5], |a, (s, b)| {
        let b = b.flatten();
        [a[0] + b.is_some() as i64, a[1] + b.unwrap_or(0), a[2] + (s > 0) as i64, a[3] + s, a[4] + 1]
    });
    let pc = (&gold).group_by(Ident::<User>::new()).select(posts_of(db)).fold(0i64, |n, _| n + 1);
    let PostHistory { post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let w = db.post_history.with(post_history_type_id.is_in([10, 12, 14])).group_by(&db.post_history.post).select(Ident::<PostHistory>::new().and(hd)).window(row_number, |(h, d)| (Reverse(d), h), asc);
    let last: MatSet<Id<PostHistory>> = (&w).filt(|(_, r)| r == 1).map(|((h, _), _)| h).collect();
    let rph: HashIdx<Id<User>, Id<PostHistory>> = (&last).select(&db.post_history.user).inv().collect();
    let ac = db.comment.group_by(&db.comment.post).select((&db.comment.origid).and(&db.comment.text)).buf_fold(|v| {
        let mut x: Vec<(i64, Str)> = v.to_vec();
        x.sort_unstable();
        join_seq(&x.iter().map(|y| y.1).collect::<Vec<_>>(), " | ")
    });
    let v = drain(
        (&us)
            .filt(|a| a[3] <= 0)
            .and((&pc).filt(|n| n > 5))
            .and((&rph).select((&db.post_history.comment).opt().and((&db.post_history.post).select(&ac).opt())).opt()),
    );
    let v = top_n(v, |&(u, ((a, n), _))| (a[0] == 0, Reverse(a[1]), Reverse(n), u), 0);
    rows(v.into_iter().map(|(u, ((a, n), r))| {
        let mut f = vec![user_col(db, u, "name"), nullable(a[1], a[0]), V::I(n), V::I(a[2]), V::F(a[3] as f64 / a[4] as f64)];
        let (c, t) = r.map_or((None, None), |(c, t)| (c, t));
        f.extend([V::S(c.unwrap_or("No Recent Actions")), V::S(t.unwrap_or("No Comments"))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.PostTypeId),
// PostAnalytics AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.UpVotes, rp.DownVotes, CASE WHEN rp.Score > 0 THEN 'Positive' WHEN rp.Score < 0 THEN 'Negative' ELSE 'Neutral' END AS Score_Status,
//        PERCENT_RANK() OVER (ORDER BY rp.Score DESC) AS Score_Rank FROM RankedPosts rp WHERE rp.Rank = 1),
// RecentComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount, STRING_AGG(c.Text, '; ') AS CommentTexts FROM Comments c WHERE c.CreationDate > CURRENT_TIMESTAMP - INTERVAL '30 days' GROUP BY c.PostId)
// SELECT pa.PostId, pa.Title, pa.CreationDate, pa.Score, pa.UpVotes, pa.DownVotes, pa.Score_Status, pa.Score_Rank, COALESCE(rc.CommentCount, 0) AS RecentCommentCount, COALESCE(rc.CommentTexts, 'No comments') AS RecentComments
// FROM PostAnalytics pa LEFT JOIN RecentComments rc ON pa.PostId = rc.PostId WHERE pa.Score_Status = 'Positive' AND (pa.CreationDate >= '2023-01-01' OR pa.Score_Rank < 0.1) AND pa.UpVotes > 5
// ORDER BY pa.Score DESC, pa.UpVotes DESC LIMIT 100;
//
// Rank reads only base columns, so the posts are picked first. The recent comments are listed in id order.
fn q24165(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, .. } = &db.post;
    let w = db.post.group_by(post_type_id).select(Ident::<Post>::new().and(score).and(creation_date)).window(row_number, |((p, s), _)| (Reverse(s), p), asc);
    type P = (Id<Post>, i64, i64);
    let first: MatSet<P> = (&w).filt(|(_, r)| r == 1).map(|(((p, s), d), _)| (p, s, d)).collect();
    let n = count(&first) as f64;
    let pr = move |k: i64| if n > 1.0 { (k - 1) as f64 / (n - 1.0) } else { 0.0 };
    let rw = whole(&first).select(Same::<P>::new()).window(rank, |(_, s, _)| Reverse(s), asc);
    type R = (P, i64);
    let pa: MatSet<R> = (&rw).collect();
    let up = (&pa).map(|x: R| x.0 .0).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let Comment { creation_date: cd, post, origid, text, .. } = &db.comment;
    let rc = db.comment.with(cd.gt(add_days(utc_to_ny(now_utc()), -30))).group_by(post).select(origid.and(text)).buf_fold(|v| {
        let mut x: Vec<(i64, Str)> = v.to_vec();
        x.sort_unstable();
        (x.len() as i64, join_seq(&x.iter().map(|y| y.1).collect::<Vec<_>>(), "; "))
    });
    let t23 = ts(2023, 1, 1, 0, 0, 0);
    type J = (R, [i64; 2]);
    let v = drain(
        (&pa).select(
            Same::<R>::new()
                .and(Same::<R>::new().map(|x: R| x.0 .0).select(&up))
                .filt(move |(((_, s, d), k), a): J| s > 0 && (d >= t23 || pr(k) < 0.1) && a[0] > 5)
                .and(Same::<R>::new().map(|x: R| x.0 .0).select(&rc).opt()),
        ),
    );
    let v = top_n(v, |&(_, ((((p, s, _), _), a), _))| (Reverse(s), Reverse(a[0]), p), 100);
    rows(v.into_iter().map(|(_, ((((p, _, _), k), a), c))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        let (n, s) = c.unwrap_or((0, "No comments"));
        f.extend([V::I(a[0]), V::I(a[1]), V::S("Positive"), V::F(pr(k)), V::I(n), V::S(s)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS Rank, COALESCE(NULLIF(p.Body, ''), 'No content available') AS BodySnippet
//     FROM Posts p WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '1 year' AND p.PostTypeId IN (1, 2)),
// PostBadges AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount, STRING_AGG(b.Name, ', ') AS BadgeNames FROM Badges b GROUP BY b.UserId),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(pb.BadgeCount, 0) AS BadgeCount, pb.BadgeNames FROM Users u LEFT JOIN PostBadges pb ON u.Id = pb.UserId WHERE u.Reputation > 1000),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.ViewCount, rp.BodySnippet, tu.DisplayName, tu.Reputation, tu.BadgeCount FROM RankedPosts rp JOIN TopUsers tu ON rp.Score > 5 WHERE rp.Rank <= 10)
// SELECT tp.Title, tp.ViewCount, tp.BodySnippet, tp.DisplayName, tp.Reputation, CASE WHEN tp.BadgeCount > 0 THEN 'Has Badges' ELSE 'No Badges' END AS BadgeStatus,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpvotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownvotes
// FROM TopPosts tp LEFT JOIN Votes v ON tp.PostId = v.PostId GROUP BY tp.PostId, tp.Title, tp.ViewCount, tp.BodySnippet, tp.DisplayName, tp.Reputation, tp.BadgeCount ORDER BY tp.ViewCount DESC, tp.Title;
//
// The ON names no key of tu: a post with Score > 5 meets every top user. Each (post, user) group's votes are the post's votes.
fn q3858(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, body, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(current_date(), -1)).and(post_type_id.is_in([1, 2]))).group_by(post_type_id).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let top: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 10).map(|((p, _), _)| p).collect();
    let vs = (&top).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let bc = db.user.with((&db.user.reputation).gt(1000)).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let mut v = Vec::new();
    (&top).with(score.gt(5)).select(&vs).cross(&bc).drive(|(p, u), (a, b)| v.push((p, a, u, b)));
    rows(v.into_iter().map(|(p, a, u, b)| {
        let mut f = post_fields(db, p, &["title", "views"]);
        let s = body.get(p).unwrap();
        f.push(V::S(if s.is_empty() { "No content available" } else { s }));
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::S(if b > 0 { "Has Badges" } else { "No Badges" }), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, u.DisplayName AS OwnerDisplayName, COUNT(DISTINCT a.Id) AS AnswerCount, COUNT(DISTINCT c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Posts a ON a.ParentId = p.Id AND a.PostTypeId = 2 LEFT JOIN Comments c ON c.PostId = p.Id LEFT JOIN Votes v ON v.PostId = p.Id
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Body, p.CreationDate, u.DisplayName),
// PostLinksData AS (SELECT pl.PostId, STRING_AGG(CONCAT('Related Post ID: ', pl.RelatedPostId, ' (Link Type: ', lt.Name, ')'), '; ') AS RelatedPosts FROM PostLinks pl JOIN LinkTypes lt ON pl.LinkTypeId = lt.Id GROUP BY pl.PostId),
// PostHistoryCount AS (SELECT ph.PostId, COUNT(*) AS EditCount FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (4, 5, 6) GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.Body, rp.CreationDate, rp.OwnerDisplayName, rp.AnswerCount, rp.CommentCount, rp.UpVotes, rp.DownVotes, COALESCE(pl.RelatedPosts, 'No related posts') AS RelatedPosts,
//        COALESCE(phc.EditCount, 0) AS EditCount
// FROM RankedPosts rp LEFT JOIN PostLinksData pl ON rp.PostId = pl.PostId LEFT JOIN PostHistoryCount phc ON rp.PostId = phc.PostId WHERE rp.rn = 1 ORDER BY rp.CreationDate DESC FETCH FIRST 10 ROWS ONLY;
//
// rn partitions by the post itself, so it is 1 on every row. The ORDER BY reads only base columns, so the ten questions are picked first. The links are listed sorted.
fn q27689(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let top = top_n(drain(db.post.with(post_type_id.eq(1)).select(creation_date)), |&(p, d)| (Reverse(d), p), 10);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let agg = (&tp).group_by(Ident::<Post>::new()).select(answers_of(db).opt().and(comments_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 2], |a, (_, t)| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
    });
    let ac = (&tp).group_by(Ident::<Post>::new()).select(answers_of(db).opt()).fold(0i64, |n, a| n + a.is_some() as i64);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let PostLink { related_post_id, link_type, .. } = &db.post_link;
    let pl = (&tp).group_by(Ident::<Post>::new()).select(links_of(db).select(related_post_id.and(link_type.select(&db.link_type.name)))).buf_fold(|v| {
        let parts: Vec<Str> = v.iter().map(|&(r, n)| &*Box::leak(format!("Related Post ID: {r} (Link Type: {n})").into_boxed_str())).collect();
        join_all(&parts, "; ")
    });
    let phc = (&tp).group_by(Ident::<Post>::new()).select(history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).is_in([4, 5, 6])))).fold(0i64, |n, _| n + 1);
    let v = drain((&agg).and(&ac).and(&cc).and((&pl).opt()).and((&phc).opt()));
    rows(v.into_iter().map(|(p, ((((a, n), c), l), h))| {
        let mut f = post_fields(db, p, &["id", "title", "body", "created", "owner"]);
        f.extend([V::I(n), V::I(c), V::I(a[0]), V::I(a[1]), V::S(l.unwrap_or("No related posts")), V::I(h.unwrap_or(0))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS rn,
//        COALESCE((SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 2), 0) AS UpVoteCount, COALESCE((SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 3), 0) AS DownVoteCount
//     FROM Posts p WHERE p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// PostStatistics AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.UpVoteCount, rp.DownVoteCount, (rp.UpVoteCount - rp.DownVoteCount) AS NetVotes,
//        CASE WHEN rp.Score > 5 THEN 'Hot' WHEN rp.Score BETWEEN 3 AND 5 THEN 'Trending' ELSE 'Normal' END AS Popularity FROM RankedPosts rp WHERE rp.rn = 1),
// ClosedPosts AS (SELECT ph.PostId, ph.CreationDate, STRING_AGG(cr.Name, ', ') AS CloseReasons FROM PostHistory ph JOIN CloseReasonTypes cr ON CAST(ph.Comment AS INTEGER) = cr.Id
//     WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.PostId, ph.CreationDate)
// SELECT ps.PostId, ps.Title, ps.CreationDate, ps.ViewCount, ps.Score, ps.UpVoteCount, ps.DownVoteCount, ps.NetVotes, ps.Popularity, COALESCE(cp.CloseReasons, 'No closure reasons') AS CloseReasons
// FROM PostStatistics ps LEFT JOIN ClosedPosts cp ON ps.PostId = cp.PostId WHERE ps.Popularity = 'Hot' ORDER BY ps.ViewCount DESC, ps.CreationDate DESC LIMIT 100;
//
// The reasons are listed sorted.
fn q20913(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, view_count, .. } = &db.post;
    let w = db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(post_type_id).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let top: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let vc = (&top).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let reason = close_reasons(db);
    let PostHistory { post, post_history_type_id, comment, creation_date: hd, .. } = &db.post_history;
    let cp = db
        .post_history
        .with(post_history_type_id.is_in([10, 11]))
        .group_by(post.and(hd))
        .select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason))
        .buf_fold(|v| join_all(&v, ", "));
    let cpv = rel(drain(&cp));
    type C = ((Id<Post>, i64), Str);
    let by_post: HashIdx<Id<Post>, Str> = (&cpv).map(|x: C| x.0 .0).inv().select((&cpv).map(|x: C| x.1)).collect();
    let v = drain((&top).with(score.gt(5)).select(Ident::<Post>::new().and((&vc).and((&by_post).opt()))));
    let v = top_n(v.into_iter().map(|x| x.1).collect(), |&(p, _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), Reverse(creation_date.get(p).unwrap()), p)
    }, 100);
    rows(v.into_iter().map(|(p, (a, c))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[0] - a[1]), V::S("Hot"), V::S(c.unwrap_or("No closure reasons"))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.Tags, u.DisplayName AS OwnerDisplayName, p.CreationDate, EXTRACT(EPOCH FROM (CURRENT_TIMESTAMP - p.CreationDate)) / 60 AS AgeInMinutes,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, COUNT(c.Id) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.Tags ORDER BY EXTRACT(EPOCH FROM (CURRENT_TIMESTAMP - p.CreationDate)) / 60 DESC) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= CURRENT_DATE - INTERVAL '30 DAYS' GROUP BY p.Id, p.Title, p.Body, p.Tags, u.DisplayName, p.CreationDate),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.Body, rp.Tags, rp.OwnerDisplayName, rp.CreationDate, rp.AgeInMinutes, rp.UpVotes, rp.DownVotes, rp.CommentCount FROM RankedPosts rp WHERE rp.Rank = 1)
// SELECT fp.PostId, fp.Title, fp.OwnerDisplayName, fp.CreationDate, fp.AgeInMinutes, fp.UpVotes - fp.DownVotes AS NetVotes, fp.CommentCount, STRING_AGG(DISTINCT t.TagName, ', ') AS TagsList
// FROM FilteredPosts fp LEFT JOIN Tags t ON POSITION(t.TagName IN TRIM(BOTH ',' FROM fp.Tags)) > 0
// GROUP BY fp.PostId, fp.Title, fp.OwnerDisplayName, fp.CreationDate, fp.AgeInMinutes, fp.UpVotes, fp.DownVotes, fp.CommentCount ORDER BY NetVotes DESC, fp.CreationDate DESC FETCH FIRST 100 ROWS ONLY;
//
// The age is a TIMESTAMPTZ difference (tz_sub). Rank reads only base columns, so the posts are picked first. The distinct tag names are listed sorted.
fn q28279(db: &'static So) -> String {
    let Post { post_type_id, creation_date, tags_str, .. } = &db.post;
    let now = utc_to_ny(now_utc());
    let w = db.post.with(post_type_id.eq(1).and(creation_date.ge(add_days(current_date(), -30)))).group_by(tags_str.opt()).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (d, p), asc);
    let first: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let agg = (&first).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt())).fold([0i64; 2], |a, (t, c)| {
        [a[0] + (t == Some(2)) as i64 - (t == Some(3)) as i64, a[1] + c.is_some() as i64]
    });
    let tn: HashIdx<Str, Id<Tag>> = (&db.tag.tag_name).inv().collect();
    let tl = (&first)
        .group_by(Ident::<Post>::new())
        .select(tags_str.map(|s: Str| s.trim_matches(',')).select_where(&tn, |s: Str, n: Str| s.contains(n)).select(&db.tag.tag_name))
        .buf_fold(|v| join_sorted(&v));
    let v = drain((&agg).and((&tl).opt()));
    let v = top_n(v, |&(p, (a, _))| (Reverse(a[0]), Reverse(creation_date.get(p).unwrap()), p), 100);
    rows(v.into_iter().map(|(p, (a, t))| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created"]);
        f.push(V::F(secs(tz_sub(now, creation_date.get(p).unwrap())) / 60.0));
        f.extend([V::I(a[0]), V::I(a[1]), ostr(t)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS rn, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '6 months'
//     GROUP BY p.Id, p.OwnerUserId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount),
// UserBadges AS (SELECT b.UserId, STRING_AGG(b.Name, ', ') AS BadgeNames FROM Badges b GROUP BY b.UserId),
// PostHistoryWithReasons AS (SELECT ph.PostId, MAX(CASE WHEN ph.PostHistoryTypeId = 10 THEN ph.Comment END) AS CloseReason, MAX(CASE WHEN ph.PostHistoryTypeId = 11 THEN ph.Comment END) AS ReopenReason
//     FROM PostHistory ph GROUP BY ph.PostId)
// SELECT u.DisplayName AS UserDisplayName, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, rp.CommentCount, rp.UpVotes, rp.DownVotes, ub.BadgeNames, phwr.CloseReason, phwr.ReopenReason
// FROM RankedPosts rp JOIN Users u ON rp.PostId = u.Id LEFT JOIN UserBadges ub ON u.Id = ub.UserId LEFT JOIN PostHistoryWithReasons phwr ON rp.PostId = phwr.PostId
// WHERE rp.rn = 1 AND rp.Score > 10 AND (rp.CommentCount IS NULL OR rp.CommentCount < 5) ORDER BY rp.ViewCount DESC, rp.Score DESC LIMIT 100 OFFSET 0;
//
// Users joins on the raw ids: a post id against a user id. rn reads only base columns, so the posts are picked first. The badge names are listed sorted.
fn q23079(db: &'static So) -> String {
    let Post { creation_date, owner_user_id, score, view_count, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -6))).group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let first: MatSet<Id<Post>> = (&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p).collect();
    let agg = (&first).with(score.gt(10)).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 3], |a, (c, t)| {
        [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.name).buf_fold(|v| join_all(&v, ", "));
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let ph = db.post_history.group_by(post).select(post_history_type_id.and(comment.opt())).buf_fold(|v| {
        let mx = |k: i64| v.iter().filter(|x| x.0 == k).filter_map(|x| x.1).max();
        (mx(10), mx(11))
    });
    let v = drain((&agg).filt(|a| a[0] < 5).and((&db.post.origid).select(&uidx).select(Ident::<User>::new().and((&ub).opt()))).and((&ph).opt()));
    let v = top_n(v, |&(p, _)| {
        let w = view_count.get(p);
        (w.is_none(), Reverse(w), Reverse(score.get(p).unwrap()), p)
    }, 100);
    rows(v.into_iter().map(|(p, ((a, (u, b)), h))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title", "created", "score", "views", "answers"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), ostr(b)]);
        let (c, r) = h.unwrap_or((None, None));
        f.extend([ostr(c), ostr(r)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS rn FROM Posts p
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// VoteSummary AS (SELECT v.PostId, COUNT(CASE WHEN vt.Name = 'UpMod' THEN 1 END) AS UpVotes, COUNT(CASE WHEN vt.Name = 'DownMod' THEN 1 END) AS DownVotes FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY v.PostId),
// UserBadges AS (SELECT b.UserId, STRING_AGG(b.Name, ', ') AS Badges, COUNT(b.Id) AS BadgeCount FROM Badges b GROUP BY b.UserId),
// PostHistoryStats AS (SELECT ph.PostId, COUNT(*) AS EditCount, MAX(CASE WHEN ph.PostHistoryTypeId IN (4, 5) THEN ph.CreationDate END) AS LastEditDate, MIN(ph.CreationDate) AS FirstHistoryDate
//     FROM PostHistory ph GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, COALESCE(ps.EditCount, 0) AS EditCount, ps.LastEditDate, ps.FirstHistoryDate, COALESCE(vs.UpVotes, 0) AS TotalUpVotes, COALESCE(vs.DownVotes, 0) AS TotalDownVotes,
//        ub.Badges, CASE WHEN ub.BadgeCount = 0 THEN 'No Badges' WHEN ub.BadgeCount <= 3 THEN 'Few Badges' ELSE 'Many Badges' END AS BadgeCategory
// FROM RankedPosts rp LEFT JOIN VoteSummary vs ON rp.PostId = vs.PostId LEFT JOIN PostHistoryStats ps ON rp.PostId = ps.PostId LEFT JOIN UserBadges ub ON rp.PostId = ub.UserId
// WHERE rp.rn <= 5 ORDER BY rp.CreationDate DESC, TotalUpVotes DESC, rp.Title ASC;
//
// ub joins on the raw ids: a post id against a user id. The badge names are listed sorted.
fn q24067(db: &'static So) -> String {
    let Post { creation_date, post_type_id, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(post_type_id).select(Ident::<Post>::new().and(creation_date)).window(row_number, |(p, d)| (Reverse(d), p), asc);
    let top: MatSet<Id<Post>> = (&w).filt(|(_, r)| r <= 5).map(|((p, _), _)| p).collect();
    let vs = (&top).group_by(Ident::<Post>::new()).select(votes_of(db).select(vtype_name(db))).fold([0i64; 2], |a, n| [a[0] + (n == "UpMod") as i64, a[1] + (n == "DownMod") as i64]);
    let PostHistory { post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let ps = (&top).group_by(Ident::<Post>::new()).select(history_of(db).select(post_history_type_id.and(hd))).fold((0i64, i64::MIN, i64::MAX), |(n, m, f), (t, d)| {
        (n + 1, if t == 4 || t == 5 { m.max(d) } else { m }, f.min(d))
    });
    let ub = db.badge.group_by(&db.badge.user_id).select(&db.badge.name).buf_fold(|v| (join_all(&v, ", "), v.len() as i64));
    let v = drain((&top).select(Ident::<Post>::new().and((&vs).opt()).and((&ps).opt()).and((&db.post.origid).select(&ub).opt())));
    rows(v.into_iter().map(|(_, (((p, a), h), b))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        match h {
            Some((n, m, d)) => f.extend([V::I(n), tmax(m), V::T(d)]),
            None => f.extend([V::I(0), V::Null, V::Null]),
        }
        let a = a.unwrap_or([0, 0]);
        f.extend([V::I(a[0]), V::I(a[1])]);
        match b {
            Some((s, n)) => f.extend([V::S(s), V::S(if n == 0 { "No Badges" } else if n <= 3 { "Few Badges" } else { "Many Badges" })]),
            None => f.extend([V::Null, V::S("Many Badges")]),
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS PostRank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.Score IS NOT NULL),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// RecentComments AS (SELECT c.PostId, STRING_AGG(c.Text, ' | ') AS CommentTexts, COUNT(c.Id) AS CommentCount FROM Comments c WHERE c.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '6 months' GROUP BY c.PostId),
// PostHistoryAggregates AS (SELECT ph.PostId, MAX(CASE WHEN ph.PostHistoryTypeId = 10 THEN ph.CreationDate END) AS LastClosedDate, COUNT(*) AS HistoryCount FROM PostHistory ph GROUP BY ph.PostId)
// SELECT r.PostId, r.Title, r.Score, r.ViewCount, r.CreationDate, u.DisplayName AS OwnerDisplayName, ub.BadgeCount, rc.CommentTexts, rc.CommentCount, pqa.LastClosedDate, pqa.HistoryCount,
//        CASE WHEN pqa.LastClosedDate IS NOT NULL THEN 'Closed' ELSE 'Open' END AS PostStatus
// FROM RankedPosts r JOIN Users u ON r.OwnerUserId = u.Id LEFT JOIN UserBadges ub ON u.Id = ub.UserId LEFT JOIN RecentComments rc ON r.PostId = rc.PostId
// LEFT JOIN PostHistoryAggregates pqa ON r.PostId = pqa.PostId WHERE r.PostRank = 1 ORDER BY r.Score DESC, r.ViewCount DESC LIMIT 10;
//
// PostRank and the ORDER BY read only base columns, so the posts are picked first. The comments are listed in id order.
fn q34629(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, view_count, .. } = &db.post;
    let w = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(owner_user).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), p), asc);
    let first = top_n(drain((&w).filt(|(_, r)| r == 1).map(|((p, _), _)| p)).into_iter().map(|(u, p)| (p, u)).collect(), |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 10);
    let tp: MatSet<Id<Post>> = rel(first.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let Comment { creation_date: cd, origid, text, .. } = &db.comment;
    let rc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).select(Ident::<Comment>::new().with(cd.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -6)))).select(origid.and(text))).buf_fold(|v| {
        let mut x: Vec<(i64, Str)> = v.to_vec();
        x.sort_unstable();
        (join_seq(&x.iter().map(|y| y.1).collect::<Vec<_>>(), " | "), x.len() as i64)
    });
    let PostHistory { post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let pqa = (&tp).group_by(Ident::<Post>::new()).select(history_of(db).select(post_history_type_id.and(hd))).fold((i64::MIN, 0i64), |(m, n), (t, d)| (if t == 10 { m.max(d) } else { m }, n + 1));
    let v = drain((&tp).select(Ident::<Post>::new().and(owner_user.select(&bc)).and((&rc).opt()).and((&pqa).opt())));
    rows(v.into_iter().map(|(_, (((p, b), c), h))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "created", "owner"]);
        f.push(V::I(b));
        match c {
            Some((s, n)) => f.extend([V::S(s), V::I(n)]),
            None => f.extend([V::Null, V::Null]),
        }
        let (m, n) = match h {
            Some((m, n)) => (tmax(m), V::I(n)),
            None => (V::Null, V::Null),
        };
        let closed = matches!(m, V::T(_));
        f.extend([m, n, V::S(if closed { "Closed" } else { "Open" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, P.OwnerUserId, RANK() OVER (PARTITION BY P.OwnerUserId ORDER BY P.Score DESC) AS RankScore,
//        COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) OVER (PARTITION BY P.Id) AS UpVotes, COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) OVER (PARTITION BY P.Id) AS DownVotes
//     FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId WHERE P.PostTypeId IN (1, 2)),
// PostMetrics AS (SELECT RP.PostId, RP.Title, RP.CreationDate, RP.Score, RP.ViewCount, RP.UpVotes, RP.DownVotes,
//        CASE WHEN RP.RankScore = 1 THEN 'Top Post' WHEN RP.RankScore IS NULL THEN 'No Posts' ELSE 'Other Posts' END AS RankCategory, COALESCE(U.DisplayName, 'Deleted User') AS OwnerDisplayName
//     FROM RankedPosts RP LEFT JOIN Users U ON RP.OwnerUserId = U.Id),
// PostHistoryInfo AS (SELECT PH.PostId, PH.Comment, PH.CreationDate, PHT.Name AS HistoryType FROM PostHistory PH JOIN PostHistoryTypes PHT ON PH.PostHistoryTypeId = PHT.Id WHERE PHT.Id IN (10, 11, 12))
// SELECT PM.*, COUNT(PHI.Comment) AS HistoryCount, STRING_AGG(PHI.Comment, '; ') FILTER (WHERE PHI.Comment IS NOT NULL) AS HistoryComments
// FROM PostMetrics PM LEFT JOIN PostHistoryInfo PHI ON PM.PostId = PHI.PostId
// GROUP BY PM.PostId, PM.Title, PM.CreationDate, PM.Score, PM.ViewCount, PM.UpVotes, PM.DownVotes, PM.RankCategory, PM.OwnerDisplayName
// HAVING SUM(PM.UpVotes) - SUM(PM.DownVotes) > 0 ORDER BY PM.Score DESC, PM.ViewCount DESC LIMIT 100;
//
// PostMetrics has a row per post and vote, each carrying the post's counts, so the HAVING holds exactly when UpVotes > DownVotes, and each group is one post.
// RankScore over those rows is 1 exactly when no post of the owner scores higher. The groups are picked first and the history product is driven for them.
// The comments are listed sorted.
fn q20538(db: &'static So) -> String {
    let Post { post_type_id, owner_user, owner_user_id, score, view_count, .. } = &db.post;
    let base = || db.post.with(post_type_id.is_in([1, 2]));
    let vc = base().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let w = base().group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(score)).window(rank, |(_, s)| Reverse(s), asc);
    let rv: MatSet<(Id<Post>, i64)> = (&w).map(|((p, _), r)| (p, r)).collect();
    let ridx = by_first(&rv);
    let v = drain((&vc).filt(|a| a[0] > a[1]).and(&ridx));
    let v = top_n(v, |&(p, _)| {
        let w = view_count.get(p);
        (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p)
    }, 100);
    let tv = rel(v);
    type T = (Id<Post>, ([i64; 2], i64));
    let phi = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type).select(&db.post_history_type.origid).is_in([10, 11, 12]))).select((&db.post_history.comment).opt());
    let hc = (&tv).map(|x: T| x.0).group_by(Ident::<Post>::new()).select(votes_of(db).opt().and(phi.opt())).buf_fold(|v| {
        let c: Vec<Str> = v.iter().filter_map(|x| x.1.flatten()).collect();
        (c.len() as i64, if c.is_empty() { None } else { Some(join_all(&c, "; ")) })
    });
    let d = drain((&tv).select(Same::<T>::new().and(Same::<T>::new().map(|x: T| x.0).select((&hc).and(owner_user.select(&db.user.display_name).opt())))));
    rows(d.into_iter().map(|(_, ((p, (a, r)), ((n, s), o)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::S(if r == 1 { "Top Post" } else { "Other Posts" })]);
        f.push(V::S(o.unwrap_or("Deleted User")));
        f.extend([V::I(n), ostr(s)]);
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
    ("34549", q34549),
    ("9758", q9758),
    ("2118", q2118),
    ("121", q121),
    ("7576", q7576),
    ("29536", q29536),
    ("33976", q33976),
    ("28650", q28650),
    ("20379", q20379),
    ("25857", q25857),
    ("20396", q20396),
    ("2163", q2163),
    ("2002", q2002),
    ("20630", q20630),
    ("3387", q3387),
    ("4490", q4490),
    ("2137", q2137),
    ("2174", q2174),
    ("24594", q24594),
    ("28949", q28949),
    ("3672", q3672),
    ("2024", q2024),
    ("23077", q23077),
    ("29378", q29378),
    ("29583", q29583),
    ("24009", q24009),
    ("26111", q26111),
    ("6726", q6726),
    ("25439", q25439),
    ("26202", q26202),
    ("32702", q32702),
    ("26678", q26678),
    ("34051", q34051),
    ("30078", q30078),
    ("33053", q33053),
    ("24850", q24850),
    ("24981", q24981),
    ("938", q938),
    ("33974", q33974),
    ("26105", q26105),
    ("20100", q20100),
    ("33204", q33204),
    ("24704", q24704),
    ("24165", q24165),
    ("3858", q3858),
    ("27689", q27689),
    ("20913", q20913),
    ("28279", q28279),
    ("23079", q23079),
    ("24067", q24067),
    ("34629", q34629),
    ("20538", q20538),
];
