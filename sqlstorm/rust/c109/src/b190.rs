use harness::prelude::*;
use std::cmp::Reverse;

fn like(s: &str, pat: &str) -> bool {
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

fn distinct_join(v: &[Str]) -> Str {
    let mut x: Vec<Str> = v.to_vec();
    x.sort_unstable();
    x.dedup();
    Box::leak(x.join(", ").into_boxed_str())
}

fn set_of<T: Copy + Eq + std::hash::Hash>(v: Vec<T>) -> MatSet<T> {
    rel(v).map(|x| x).collect()
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.Score, p.CreationDate, p.Tags, COALESCE(u.DisplayName, 'Community User') AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.Tags ORDER BY p.Score DESC) AS Rank
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.PostTypeId = 1 AND p.Score > 0 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TagPopularity AS (SELECT UNNEST(string_to_array(substring(Tags, 2, length(Tags) - 2), '><')) AS Tag FROM Posts WHERE PostTypeId = 1 AND Tags IS NOT NULL),
// PopularTags AS (SELECT Tag, COUNT(*) AS TagFrequency FROM TagPopularity GROUP BY Tag ORDER BY TagFrequency DESC LIMIT 10),
// PopularPosts AS (SELECT rp.PostId, rp.Title, rp.OwnerDisplayName, rp.ViewCount, rp.Score, rp.Tags FROM RankedPosts rp JOIN PopularTags pt ON rp.Tags LIKE '%' || pt.Tag || '%')
// SELECT pp.PostId, pp.Title, pp.OwnerDisplayName, pp.ViewCount, pp.Score, pt.Tag
// FROM PopularPosts pp CROSS JOIN PopularTags pt ORDER BY pt.Tag, pp.Score DESC;
fn q29335(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, tags_str, .. } = &db.post;
    let tf = db.post.with(post_type_id.eq(1)).select(tags_str.flat_map(tag_list)).group_by(Same::<Str>::new()).fold(0i64, |n, _| n + 1);
    let pt = top_n(drain(&tf), |&(t, n)| (Reverse(n), t), 10);
    let pt: Vec<Str> = pt.into_iter().map(|x| x.0).collect();
    let pts = set_of(pt.clone());
    let ptr = rel(pt);
    let rp = db.post.with(post_type_id.eq(1).and(score.gt(0)).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))));
    let pp = rp.select(tags_str.select_where(&pts, |t: Str, n: Str| like(t, &format!("%{n}%"))));
    let v = drain(pp.cross(&ptr));
    rows(v.into_iter().map(|((p, _), (_, t))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.push(V::S(db.post.owner_user.get(p).map_or("Community User", |u| db.user.display_name.get(u).unwrap())));
        f.extend(post_fields(db, p, &["views", "score"]));
        f.push(V::S(t));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, COALESCE(COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 2), 0) AS UpVotes,
//        COALESCE(COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 3), 0) AS DownVotes, COALESCE(SUM(CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN 1 ELSE 0 END), 0) AS CloseReopenCount,
//        DENSE_RANK() OVER (ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN PostHistory ph ON p.Id = ph.PostId
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= '2024-10-01 12:34:56'::timestamp - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.Score),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.UpVotes, rp.DownVotes, rp.CloseReopenCount FROM RankedPosts rp WHERE rp.Rank <= 10)
// SELECT t.PostId, t.Title, t.CreationDate, t.Score, t.UpVotes, t.DownVotes, t.CloseReopenCount, STRING_AGG(DISTINCT c.UserDisplayName, ', ') AS Commenters
// FROM TopPosts t LEFT JOIN Comments c ON t.PostId = c.PostId
// GROUP BY t.PostId, t.Title, t.CreationDate, t.Score, t.UpVotes, t.DownVotes, t.CloseReopenCount ORDER BY t.Score DESC, t.CreationDate DESC;
//
// The rank reads only base columns, so the posts are ranked first and the product is driven only for them.
fn q6292(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(score.and(creation_date)));
    let r = ranked(v, |&(_, (s, d))| (Reverse(s), Reverse(d)), true);
    let tp = set_of(r.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect());
    let agg = (&tp)
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(history_of(db).select(&db.post_history.post_history_type_id).opt()))
        .fold([0i64; 3], |a, (v, h)| [a[0] + (v == Some(2)) as i64, a[1] + (v == Some(3)) as i64, a[2] + matches!(h, Some(10 | 11)) as i64]);
    let cm = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.user_display_name)).buf_fold(|v| distinct_join(&v));
    let v = drain((&agg).and((&cm).opt()));
    rows(v.into_iter().map(|(p, (a, c))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), ostr(c)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.FavoriteCount,
//        ROW_NUMBER() OVER (PARTITION BY pt.Name ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank, pt.Name AS PostTypeName, u.DisplayName AS OwnerDisplayName, u.Reputation AS OwnerReputation,
//        (SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 2) AS UpVotes, (SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 3) AS DownVotes,
//        (SELECT STRING_AGG(t.TagName, ', ') FROM Tags t WHERE t.Id IN (SELECT unnest(string_to_array(substring(p.Tags, 2, length(p.Tags)-2), '><')::int[]))) AS TagsList
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' AND p.Score > 10 AND p.ViewCount > 100)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, rp.CommentCount, rp.FavoriteCount, rp.OwnerDisplayName, rp.OwnerReputation, rp.UpVotes, rp.DownVotes, rp.TagsList
// FROM RankedPosts rp WHERE rp.Rank <= 10 ORDER BY rp.Score DESC, rp.CreationDate DESC;
//
// The `::int[]` cast of tag names would raise an error in DuckDB on any row; no row qualifies, so it is never evaluated. The port reads names that parse as integers as tag ids.
fn q9925(db: &'static So) -> String {
    let Post { score, creation_date, view_count, owner_user, tags_str, .. } = &db.post;
    let base = db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)).and(score.gt(10)).and(view_count.gt(100))).with(owner_user);
    let v = drain(base.select(ptype_name(db)));
    let tp = set_of(top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10, false).into_iter().map(|x| x.0).collect());
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let tid: HashIdx<i64, Id<Tag>> = (&db.tag.origid).inv().collect();
    let tl = (&tp)
        .group_by(Ident::<Post>::new())
        .select(tags_str.flat_map(tag_list).flat_map(|s: Str| s.parse::<i64>().ok()).select(&tid).select(&db.tag.tag_name))
        .buf_fold(|v| -> Str { Box::leak(v.to_vec().join(", ").into_boxed_str()) });
    let v = drain((&tp).select((&vc).opt().and((&tl).opt())));
    rows(v.into_iter().map(|(p, (a, t))| {
        let a = a.unwrap_or([0, 0]);
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "favorites", "owner", "rep"]);
        f.extend([V::I(a[0]), V::I(a[1]), ostr(t)]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsCount,
//        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersCount, SUM(P.Score) AS TotalScore, U.Reputation, ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS UserRank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId WHERE U.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, TotalPosts, QuestionsCount, AnswersCount, TotalScore, Reputation, UserRank FROM UserActivity WHERE UserRank <= 10)
// SELECT T.DisplayName, T.TotalPosts, T.QuestionsCount, T.AnswersCount, T.TotalScore, T.Reputation,
//        (SELECT ARRAY(SELECT DISTINCT TagName FROM Tags WHERE TagName IN (SELECT unnest(string_to_array(P.Tags, '>')) FROM Posts P WHERE P.OwnerUserId = T.UserId))) AS ActiveTags,
//        (SELECT COUNT(*) FROM Badges B WHERE B.UserId = T.UserId) AS TotalBadges
// FROM TopUsers T ORDER BY T.TotalScore DESC;
fn q9476(db: &'static So) -> String {
    let tu = top_n(drain(db.user.with((&db.user.creation_date).ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(&db.user.reputation)), |&(u, r)| (Reverse(r), u), 10);
    let tu = set_of(tu.into_iter().map(|x| x.0).collect());
    let ups = user_posts(db);
    let tn: HashIdx<Str, Id<Tag>> = (&db.tag.tag_name).inv().collect();
    let at = (&tu)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.tags_str).flat_map(|s: Str| s.split('>'))).select(&tn).select(&db.tag.tag_name))
        .buf_fold(|v| distinct_join(&v));
    let bc = (&tu).group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let v = drain((&tu).select((&ups).and((&at).opt()).and(&bc)));
    rows(v.into_iter().map(|(u, ((a, t), b))| {
        let mut f = ucols(db, u, &["name"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[3]), nullable(a[4], a[1]), user_col(db, u, "rep")]);
        f.push(V::Owned(format!("[{}]", t.unwrap_or(""))));
        f.push(V::I(b));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn FROM Posts p WHERE p.PostTypeId = 1),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(b.Id) AS BadgeCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation),
// VotesSummary AS (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesCount, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesCount FROM Votes v GROUP BY v.PostId),
// ClosedPosts AS (SELECT h.PostId, MAX(h.CreationDate) AS LastClosedDate, STRING_AGG(DISTINCT cr.Name, ', ') AS CloseReasons
//     FROM PostHistory h JOIN CloseReasonTypes cr ON CAST(h.Comment AS INT) = cr.Id WHERE h.PostHistoryTypeId = 10 GROUP BY h.PostId)
// SELECT rp.Title, rp.CreationDate, u.DisplayName, u.Reputation, us.BadgeCount, COALESCE(vs.UpVotesCount, 0) AS UpVotes, COALESCE(vs.DownVotesCount, 0) AS DownVotes, cp.LastClosedDate, cp.CloseReasons
// FROM RankedPosts rp JOIN Users u ON u.Id = rp.OwnerUserId JOIN UserReputation us ON us.UserId = u.Id LEFT JOIN VotesSummary vs ON vs.PostId = rp.Id LEFT JOIN ClosedPosts cp ON cp.PostId = rp.Id
// WHERE rp.rn = 1 AND (us.Reputation > 1000 OR cp.LastClosedDate IS NOT NULL) ORDER BY rp.CreationDate DESC LIMIT 10;
//
// Partitioned by the owner edge: a post whose OwnerUserId dangles is dropped by the Users join whatever its rn.
fn q2863(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1)).select(owner_user));
    let first = set_of(top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false).into_iter().map(|x| x.0).collect());
    let bc = db.user.group_by(Ident::<User>::new()).select(badges_of(db).opt()).fold(0i64, |n, b| n + b.is_some() as i64);
    let vs = (&first).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post, post_history_type_id, comment, creation_date: hd, .. } = &db.post_history;
    let cp = db
        .post_history
        .with(post_history_type_id.eq(10))
        .group_by(post)
        .select(hd.and(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason)))
        .buf_fold(|v| (v.iter().map(|x| x.0).max().unwrap(), distinct_join(&v.iter().map(|x| x.1).collect::<Vec<_>>())));
    let rep = |u: Id<User>| db.user.reputation.get(u).unwrap();
    type J = (((Id<User>, i64), Option<[i64; 2]>), Option<(i64, Str)>);
    let v = drain((&first).select(owner_user.select(Ident::<User>::new().and(&bc)).and((&vs).opt()).and((&cp).opt())).filt(move |(((u, _), _), c): J| rep(u) > 1000 || c.is_some()));
    let v = top_n(v, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 10);
    rows(v.into_iter().map(|(p, (((u, b), vs), c))| {
        let vs = vs.unwrap_or([0, 0]);
        let mut f = post_fields(db, p, &["title", "created"]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::I(b), V::I(vs[0]), V::I(vs[1]), ots(c.map(|c| c.0)), ostr(c.map(|c| c.1))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, COALESCE(v.UpVotes, 0) AS UpVotes, COALESCE(v.DownVotes, 0) AS DownVotes, COALESCE(c.CommentCount, 0) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank, p.PostTypeId, p.Score, p.Tags
//     FROM Posts p LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes GROUP BY PostId) v ON p.Id = v.PostId
//     LEFT JOIN (SELECT PostId, COUNT(Id) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId),
// PopularTags AS (SELECT value AS TagName, COUNT(*) AS TagCount FROM Posts, UNNEST(string_to_array(Tags, ',')) AS value WHERE Tags IS NOT NULL GROUP BY TagName ORDER BY TagCount DESC LIMIT 10)
// SELECT rp.PostId, rp.Title, rp.Body, rp.CreationDate, rp.UpVotes, rp.DownVotes, rp.CommentCount, pt.TagName
// FROM RankedPosts rp JOIN PopularTags pt ON rp.Tags LIKE '%' || pt.TagName || '%' WHERE rp.Rank <= 5 ORDER BY rp.PostTypeId, rp.Score DESC;
//
// The rank reads only base columns, so the posts are ranked first and the counts are taken only for them.
// `value` is the UNNEST's table alias, so it binds to a struct {'unnest': ...}, and the LIKE pattern is that struct's text.
fn q27731(db: &'static So) -> String {
    let Post { post_type_id, score, tags_str, .. } = &db.post;
    let tc = db.post.select(tags_str.flat_map(|s: Str| s.split(','))).group_by(Same::<Str>::new()).fold(0i64, |n, _| n + 1);
    let pt = set_of(top_n(drain(&tc), |&(t, n)| (Reverse(n), t), 10).into_iter().map(|x| x.0).collect());
    let tp = set_of(top_per(drain(db.post.select(post_type_id)), |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false).into_iter().map(|x| x.0).collect());
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let v = drain((&tp).select(tags_str.select_where(&pt, |t: Str, n: Str| like(t, &format!("%{{'unnest': {n}}}%"))).and((&vc).opt()).and((&cc).opt())));
    rows(v.into_iter().map(|(p, ((t, a), c))| {
        let a = a.unwrap_or([0, 0]);
        let mut f = post_fields(db, p, &["id", "title", "body", "created"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c.unwrap_or(0)), V::Owned(format!("{{'unnest': {t}}}"))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Tags, p.ViewCount, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRank
//     FROM Posts p WHERE p.PostTypeId = 1),
// UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.Views, COUNT(DISTINCT p.Id) AS QuestionCount, SUM(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName, u.Reputation, u.Views),
// TopTags AS (SELECT TagName, COUNT(*) AS TagCount FROM (SELECT unnest(string_to_array(Tags, '>')) AS TagName FROM Posts WHERE PostTypeId = 1) AS TagsTable GROUP BY TagName ORDER BY TagCount DESC LIMIT 10)
// SELECT ur.DisplayName AS User, ur.Reputation, ur.Views, rp.Title, rp.ViewCount, rp.CreationDate, tt.TagName, tt.TagCount, rp.UserPostRank
// FROM RankedPosts rp JOIN UserReputation ur ON rp.PostId = ur.UserId JOIN TopTags tt ON tt.TagName = ANY(string_to_array(rp.Tags, '>'))
// WHERE rp.UserPostRank <= 3 ORDER BY ur.Reputation DESC, rp.ViewCount DESC;
//
// The join is on the post's Id against the user's Id, as written.
fn q29581(db: &'static So) -> String {
    let Post { post_type_id, tags_str, creation_date, owner_user_id, .. } = &db.post;
    let qs = || db.post.with(post_type_id.eq(1));
    let split = |s: Str| s.split('>');
    let tc = qs().select(tags_str.flat_map(split)).group_by(Same::<Str>::new()).fold(0i64, |n, _| n + 1);
    let tt = rel(top_n(drain(&tc), |&(t, n)| (Reverse(n), t), 10));
    let tti: HashIdx<Str, (Str, i64)> = (&tt).map(|(t, _): (Str, i64)| t).inv().select(&tt).collect();
    let r = per_group(ranked(drain(qs().select(owner_user_id.opt())), |&(p, o)| (o, Reverse(creation_date.get(p).unwrap()), p), false), |x| x.1);
    let rp = rel(r.into_iter().filter(|x| x.1 <= 3).map(|((p, _), k)| (p, k)).collect());
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    type R = (Id<Post>, i64);
    let pr = || Same::<R>::new().map(|x: R| x.0);
    let ur = Ident::<User>::new().with((&db.user.reputation).gt(1000));
    let v = drain((&rp).select(Same::<R>::new().and(pr().select(&db.post.origid).select(&uidx).select(ur)).and(pr().select(tags_str.flat_map(split)).select(&tti))));
    rows(v.into_iter().map(|(_, (((p, k), u), (t, n)))| {
        let mut f = ucols(db, u, &["name", "rep", "uviews"]);
        f.extend(post_fields(db, p, &["title", "views", "created"]));
        f.extend([V::S(t), V::I(n), V::I(k)]);
        row(f)
    }))
}

// WITH UserBadges AS (SELECT U.Id AS UserId, U.DisplayName, B.Name AS BadgeName, B.Class, COUNT(*) OVER (PARTITION BY U.Id) AS BadgeCount FROM Users U JOIN Badges B ON U.Id = B.UserId),
// TopUsers AS (SELECT UserId, DisplayName, BadgeCount, RANK() OVER (ORDER BY BadgeCount DESC) AS Rank FROM UserBadges)
// SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS TotalPosts, SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END) AS PositiveScorePosts,
//        SUM(CASE WHEN P.AnswerCount > 0 THEN 1 ELSE 0 END) AS QuestionsWithAnswers, SUM(CASE WHEN C.Id IS NOT NULL THEN 1 ELSE 0 END) AS PostsWithComments,
//        T.TagName, COUNT(DISTINCT PT.Id) AS TotalPostTypes, MAX(T2.CreationDate) AS LastPostDate
// FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON P.Id = C.PostId
// LEFT JOIN LATERAL (SELECT unnest(string_to_array(P.Tags, '>')) AS TagName) T ON TRUE
// LEFT JOIN PostTypes PT ON P.PostTypeId = PT.Id LEFT JOIN Posts P2 ON P.AcceptedAnswerId = P2.Id
// LEFT JOIN (SELECT PostId, MAX(CreationDate) AS CreationDate FROM PostLinks GROUP BY PostId) T2 ON P.Id = T2.PostId
// GROUP BY U.Id, U.DisplayName, T.TagName HAVING COUNT(DISTINCT P.Id) > 5 ORDER BY TotalPosts DESC, UserId LIMIT 50;
//
// The CTEs are never read. A user with no posts has at most 0 distinct posts and fails the HAVING, so only owned posts are grouped;
// P2 is joined on its key and multiplies nothing.
fn q7811(db: &'static So) -> String {
    let Post { owner_user, tags_str, score, answer_count, post_type, .. } = &db.post;
    type J = (Id<Post>, Option<Str>);
    let jr = rel(drain(db.post.with(owner_user).select(Ident::<Post>::new().and(tags_str.flat_map(|s: Str| s.split('>')).opt()))).into_iter().map(|x| x.1).collect::<Vec<J>>());
    let pr = || Same::<J>::new().map(|x: J| x.0);
    let t2 = db.post_link.group_by(&db.post_link.post).select(&db.post_link.creation_date).fold(i64::MIN, |m, d| m.max(d));
    let g = (&jr)
        .group_by(pr().select(owner_user).and(Same::<J>::new().map(|x: J| x.1)))
        .select(pr().and(pr().select(score.and(answer_count.opt()).and(post_type))).and(pr().select(comments_of(db).opt())).and(pr().select((&t2).opt())))
        .buf_fold(|v| {
            let n = distinct_some(v.iter().map(|x| Some(x.0 .0 .0)));
            let t = distinct_some(v.iter().map(|x| Some(x.0 .0 .1 .1)));
            let mut a = [0i64; 3];
            let mut m = i64::MIN;
            for &(((_, ((s, ac), _)), c), d) in v.iter() {
                a[0] += (s > 0) as i64;
                a[1] += ac.map_or(false, |x| x > 0) as i64;
                a[2] += c.is_some() as i64;
                m = m.max(d.unwrap_or(i64::MIN));
            }
            (n, a, t, m)
        });
    let v = drain((&g).filt(|x: (i64, [i64; 3], i64, i64)| x.0 > 5));
    let v = top_n(v, |&((u, t), (n, _, _, _))| (Reverse(n), db.user.origid.get(u).unwrap(), t), 50);
    rows(v.into_iter().map(|((u, t), (n, a, k, m))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), ostr(t), V::I(k), tmax(m)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, COUNT(v.Id) AS VoteCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS Rank
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.PostTypeId),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.VoteCount FROM RankedPosts rp WHERE rp.Rank <= 5),
// PostCommentSummary AS (SELECT p.Id AS PostId, COUNT(c.Id) AS CommentCount, STRING_AGG(DISTINCT c.UserDisplayName, ', ') AS Commenters FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY p.Id)
// SELECT fp.PostId, fp.Title, fp.CreationDate, fp.ViewCount, fp.Score, fp.VoteCount, pcs.CommentCount, COALESCE(pcs.Commenters, 'No comments') AS Commenters,
//        CASE WHEN fp.Score > 100 THEN 'High Score' WHEN fp.Score BETWEEN 50 AND 100 THEN 'Moderate Score' ELSE 'Low Score' END AS ScoreCategory
// FROM FilteredPosts fp LEFT JOIN PostCommentSummary pcs ON fp.PostId = pcs.PostId WHERE fp.ViewCount > 50 ORDER BY fp.ViewCount DESC, fp.Score DESC;
//
// The rank reads only base columns, so the posts are ranked first and the counts are taken only for them.
fn q4999(db: &'static So) -> String {
    let Post { creation_date, post_type_id, view_count, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let tp = set_of(top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false).into_iter().map(|x| x.0).collect());
    let fp = || (&tp).with(view_count.gt(50));
    let vc = fp().group_by(Ident::<Post>::new()).select(votes_of(db).opt()).fold(0i64, |n, v| n + v.is_some() as i64);
    let cc = fp().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let cm = fp().group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.user_display_name)).buf_fold(|v| distinct_join(&v));
    let v = drain((&vc).and(&cc).and((&cm).opt()));
    rows(v.into_iter().map(|(p, ((n, c), m))| {
        let s = score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([V::I(n), V::I(c), V::S(m.unwrap_or("No comments"))]);
        f.push(V::S(if s > 100 { "High Score" } else if (50..=100).contains(&s) { "Moderate Score" } else { "Low Score" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COUNT(c.Id) AS CommentCount, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS RecentRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate, p.PostTypeId),
// UserReputation AS (SELECT u.Id AS UserId, SUM(b.Class) AS TotalBadges, AVG(u.Reputation) AS AvgReputation FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// ClosureReasons AS (SELECT ph.PostId, STRING_AGG(cr.Name, ', ') AS ClosureReasons FROM PostHistory ph JOIN CloseReasonTypes cr ON ph.Comment = CAST(cr.Id AS VARCHAR) WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.CommentCount, CASE WHEN ur.TotalBadges IS NULL THEN 0 ELSE ur.TotalBadges END AS UserTotalBadges,
//        COALESCE(ur.AvgReputation, 0) AS UserAvgReputation, COALESCE(cr.ClosureReasons, 'Not Closed') AS ClosureReasons
// FROM RankedPosts rp LEFT JOIN UserReputation ur ON rp.PostId = ur.UserId LEFT JOIN ClosureReasons cr ON rp.PostId = cr.PostId
// WHERE rp.RecentRank <= 5 AND (ur.AvgReputation IS NULL OR ur.AvgReputation > 100) OR (cr.ClosureReasons IS NOT NULL AND ur.TotalBadges >= 1)
// ORDER BY rp.CommentCount DESC, rp.CreationDate DESC LIMIT 10;
//
// The post's Id is joined to the user's Id, as written. AVG(u.Reputation) over one user's rows is that user's reputation.
fn q22222(db: &'static So) -> String {
    let Post { creation_date, post_type_id, origid, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let r = per_group(ranked(drain(recent().select(post_type_id)), |&(p, t)| (t, Reverse(creation_date.get(p).unwrap())), false), |x| x.1);
    let rv = rel(r.into_iter().map(|((p, _), k)| (p, k)).collect());
    let cc = recent().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class)).fold(0i64, |n, c| n + c);
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let reason: HashIdx<Str, Str> = (&db.close_reason_type.origid).map(|i: i64| -> Str { Box::leak(i.to_string().into_boxed_str()) }).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let cr = db.post_history.with(post_history_type_id.eq(10)).group_by(post).select(comment.select(&reason)).buf_fold(|v| -> Str { Box::leak(v.to_vec().join(", ").into_boxed_str()) });
    type R = (Id<Post>, i64);
    type J = (((R, i64), Option<(Id<User>, Option<i64>)>), Option<Str>);
    let pr = || Same::<R>::new().map(|x: R| x.0);
    let rep = |u: Id<User>| db.user.reputation.get(u).unwrap();
    let v = drain(
        (&rv)
            .select(Same::<R>::new().and(pr().select(&cc)).and(pr().select(origid).select(&uidx).select(Ident::<User>::new().and((&ub).opt())).opt()).and(pr().select(&cr).opt()))
            .filt(move |(((( _, k), _), u), c): J| (k <= 5 && u.map_or(true, |(u, _)| rep(u) > 100)) || (c.is_some() && u.and_then(|x| x.1).map_or(false, |b| b >= 1))),
    );
    let v = top_n(v, |&(_, ((((p, _), n), _), _))| (Reverse(n), Reverse(creation_date.get(p).unwrap()), p), 10);
    rows(v.into_iter().map(|(_, ((((p, _), n), u), c))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(n), V::I(u.and_then(|x| x.1).unwrap_or(0)), V::F(u.map_or(0.0, |x| rep(x.0) as f64)), V::S(c.unwrap_or("Not Closed"))]);
        row(f)
    }))
}

// WITH UserBadges AS (SELECT UserId, COUNT(*) AS BadgeCount, STRING_AGG(Name, ', ') AS BadgeNames FROM Badges GROUP BY UserId),
// PostStatistics AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT pl.RelatedPostId) AS RelatedPosts
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN PostLinks pl ON p.Id = pl.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate),
// ClosedPosts AS (SELECT ph.PostId, MAX(ph.CreationDate) AS LastClosedDate FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId),
// RankedPosts AS (SELECT ps.PostId, ps.Title, ps.CreationDate, ps.UpVotes, ps.DownVotes, ps.CommentCount, ps.RelatedPosts, cp.LastClosedDate, RANK() OVER (ORDER BY ps.UpVotes DESC) AS RankByVotes
//     FROM PostStatistics ps LEFT JOIN ClosedPosts cp ON ps.PostId = cp.PostId)
// SELECT rp.*, ub.BadgeCount, ub.BadgeNames FROM RankedPosts rp LEFT JOIN UserBadges ub ON rp.PostId = ub.UserId WHERE rp.RankByVotes <= 10 ORDER BY rp.UpVotes DESC, rp.CreationDate DESC;
//
// The post's Id is joined to the badge's UserId, as written.
fn q1625(db: &'static So) -> String {
    let Post { creation_date, origid, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let ps = recent()
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()).and(links_of(db).opt()))
        .fold([0i64; 3], |a, ((t, c), _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64]);
    let rl = recent().group_by(Ident::<Post>::new()).select(links_of(db).select(&db.post_link.related_post_id)).count_distinct();
    let r = ranked(drain(&ps), |&(_, a)| Reverse(a[0]), false);
    let tp = rel(r.into_iter().take_while(|x| x.1 <= 10).map(|((p, a), k)| (p, a, k)).collect());
    let cp = db.post_history.with((&db.post_history.post_history_type_id).eq(10)).group_by(&db.post_history.post).select(&db.post_history.creation_date).fold(i64::MIN, |m, d| m.max(d));
    let ub = db.badge.group_by(&db.badge.user_id).select(&db.badge.name).buf_fold(|v| -> (i64, Str) { (v.len() as i64, Box::leak(v.to_vec().join(", ").into_boxed_str())) });
    type R = (Id<Post>, [i64; 3], i64);
    let pr = || Same::<R>::new().map(|x: R| x.0);
    let v = drain((&tp).select(Same::<R>::new().and(pr().select(&rl).opt()).and(pr().select(&cp).opt()).and(pr().select(origid).select(&ub).opt())));
    let v = top_n(v, |&(_, ((((p, a, _), _), _), _))| (Reverse(a[0]), Reverse(creation_date.get(p).unwrap()), p), 0);
    rows(v.into_iter().map(|(_, ((((p, a, k), l), c), b))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(l.unwrap_or(0)), ots(c), V::I(k), oint(b.map(|b| b.0)), ostr(b.map(|b| b.1))]);
        row(f)
    }))
}

// Rewritten (rewrites/21867.sql): the RankByViews window gets `, p.Id`.
// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.Score, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.ViewCount DESC, p.Id) AS RankByViews,
//        DENSE_RANK() OVER (ORDER BY p.Score DESC) AS RankByScore FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// UserVoteDetails AS (SELECT v.PostId, MAX(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVote, MAX(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVote
//     FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY v.PostId),
// BadgeSummary AS (SELECT b.UserId, COUNT(*) AS BadgeCount, STRING_AGG(b.Name, ', ') AS Badges FROM Badges b GROUP BY b.UserId)
// SELECT p.Title AS PostTitle, p.ViewCount, p.Score, COALESCE(uv.UpVote, 0) AS UpVoteCount, COALESCE(uv.DownVote, 0) AS DownVoteCount, RANK() OVER (ORDER BY p.Score DESC) AS GlobalScoreRank,
//        b.BadgeCount AS UserBadgeCount, b.Badges AS UserBadgeNames
// FROM RankedPosts p LEFT JOIN UserVoteDetails uv ON p.PostId = uv.PostId LEFT JOIN Posts cp ON p.PostId = cp.AcceptedAnswerId LEFT JOIN BadgeSummary b ON cp.OwnerUserId = b.UserId
// WHERE p.RankByViews <= 5 AND (SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.PostId) > 10 AND p.PostId IS NOT NULL
// ORDER BY GlobalScoreRank, p.ViewCount DESC;
fn q21867(db: &'static So) -> String {
    let Post { creation_date, post_type_id, view_count, score, origid, accepted_answer, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let tp = set_of(top_per(v, |&(_, t)| t, |&(p, _)| { let w = view_count.get(p); (w.is_none(), Reverse(w), origid.get(p).unwrap()) }, 5, false).into_iter().map(|x| x.0).collect());
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let uv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(vtype_name(db))).fold([0i64; 2], |a, n| [a[0].max((n == "UpMod") as i64), a[1].max((n == "DownMod") as i64)]);
    let acc: HashIdx<Id<Post>, Id<Post>> = accepted_answer.inv().collect();
    let bs = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.name)).buf_fold(|v| -> (i64, Str) { (v.len() as i64, Box::leak(v.to_vec().join(", ").into_boxed_str())) });
    let v = drain((&tp).with((&cc).filt(|n| n > 10)).select((&uv).opt().and((&acc).select(owner_user.select(&bs).opt()).opt())));
    let v = ranked(v, |&(p, _)| Reverse(score.get(p).unwrap()), false);
    let v = top_n(v, |&((p, _), k)| { let w = view_count.get(p); (k, w.is_none(), Reverse(w)) }, 0);
    rows(v.into_iter().map(|((p, (u, b)), k)| {
        let u = u.unwrap_or([0, 0]);
        let b = b.flatten();
        let mut f = post_fields(db, p, &["title", "views", "score"]);
        f.extend([V::I(u[0]), V::I(u[1]), V::I(k), oint(b.map(|b| b.0)), ostr(b.map(|b| b.1))]);
        row(f)
    }))
}

// WITH UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(P.Id) AS PostCount, SUM(COALESCE(P.Score, 0)) AS TotalScore,
//        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount, SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId GROUP BY U.Id, U.DisplayName, U.Reputation),
// RecentPosts AS (SELECT P.Id, P.Title, P.CreationDate, P.OwnerUserId, RANK() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS RecentRank
//     FROM Posts P WHERE P.CreationDate >= (TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days')),
// ClosingReasons AS (SELECT PH.PostId, ARRAY_AGG(DISTINCT CR.Name) AS CloseReasons FROM PostHistory PH JOIN CloseReasonTypes CR ON CAST(PH.Comment AS INT) = CR.Id
//     WHERE PH.PostHistoryTypeId IN (10, 11) GROUP BY PH.PostId)
// SELECT UA.DisplayName, UA.Reputation, UA.PostCount, UA.TotalScore, UA.QuestionCount, UA.AnswerCount, RP.Title AS RecentPostTitle, RP.CreationDate AS RecentPostDate, CR.CloseReasons
// FROM UserActivity UA LEFT JOIN RecentPosts RP ON UA.UserId = RP.OwnerUserId AND RP.RecentRank = 1 LEFT JOIN ClosingReasons CR ON RP.Id = CR.PostId
// WHERE UA.Reputation > 1000 ORDER BY UA.TotalScore DESC, UA.DisplayName ASC LIMIT 50;
//
// Partitioned by the owner edge: only posts whose owner exists can meet a user.
fn q2827(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(owner_user));
    let r = rel(top_per(v, |&(_, u)| u, |&(p, _)| Reverse(creation_date.get(p).unwrap()), 1, true));
    let rpi: HashIdx<Id<User>, Id<Post>> = (&r).map(|x: (Id<Post>, Id<User>)| x.1).inv().select((&r).map(|x: (Id<Post>, Id<User>)| x.0)).collect();
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let cr = db
        .post_history
        .with(post_history_type_id.is_in([10, 11]))
        .group_by(post)
        .select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason))
        .buf_fold(|v| distinct_join(&v));
    let ups = user_posts(db);
    let v = drain(db.user.with((&db.user.reputation).gt(1000)).select((&ups).and((&rpi).select(Ident::<Post>::new().and((&cr).opt())).opt())));
    let v = top_n(v, |&(u, (a, p))| (Reverse(a[4]), db.user.display_name.get(u).unwrap(), u, p.map(|x| x.0)), 50);
    rows(v.into_iter().map(|(u, (a, p))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(a[1]), V::I(a[4]), V::I(a[2]), V::I(a[3])]);
        match p {
            Some((p, c)) => {
                f.extend(post_fields(db, p, &["title", "created"]));
                f.push(match c { Some(c) => V::Owned(format!("[{c}]")), None => V::Null });
            }
            None => f.extend([V::Null, V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH TagFrequency AS (SELECT trim(split_part(tag, '>', 1)) AS TagName, COUNT(*) AS Frequency
//     FROM (SELECT unnest(string_to_array(substring(Tags, 2, length(Tags) - 2), '><')) AS tag FROM Posts WHERE PostTypeId = 1) AS TagList GROUP BY TagName),
// MostFrequentTags AS (SELECT TagName, Frequency, ROW_NUMBER() OVER (ORDER BY Frequency DESC) AS Rank FROM TagFrequency),
// UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS PostCount, SUM(V.BountyAmount) AS TotalBounties,
//        SUM(CASE WHEN C.Id IS NOT NULL THEN 1 ELSE 0 END) AS CommentCount, SUM(CASE WHEN PH.Id IS NOT NULL THEN 1 ELSE 0 END) AS EditCount
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId IN (2, 3)
//     LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN PostHistory PH ON P.Id = PH.PostId GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, TotalBounties, CommentCount, EditCount, ROW_NUMBER() OVER (ORDER BY PostCount DESC, TotalBounties DESC) AS UserRank FROM UserActivity)
// SELECT T.TagName, T.Frequency AS TagFrequency, U.DisplayName AS TopUser, U.PostCount, U.TotalBounties, U.CommentCount, U.EditCount
// FROM MostFrequentTags T JOIN TopUsers U ON U.UserRank <= 5 WHERE T.Frequency > 10 ORDER BY T.Frequency DESC, U.TotalBounties DESC;
fn q25526(db: &'static So) -> String {
    let Post { post_type_id, tags_str, .. } = &db.post;
    let tf = db.post.with(post_type_id.eq(1)).select(tags_str.flat_map(tag_list).map(|t: Str| t.split('>').next().unwrap().trim())).group_by(Same::<Str>::new()).fold(0i64, |n, _| n + 1);
    let tags = rel(drain((&tf).filt(|n| n > 10)));
    let v23 = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).is_in([2, 3]))).select((&db.vote.bounty_amount).opt());
    let ua = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(v23.opt().and(comments_of(db).opt()).and(history_of(db).opt())).opt())
        .fold([0i64; 4], |a, x| match x {
            Some(((b, c), h)) => {
                let b = b.flatten();
                [a[0] + b.is_some() as i64, a[1] + b.unwrap_or(0), a[2] + c.is_some() as i64, a[3] + h.is_some() as i64]
            }
            None => a,
        });
    let pc = user_distinct_posts(db);
    let tu = top_n(drain((&pc).and(&ua)), |&(u, (n, a))| (Reverse(n), a[0] == 0, Reverse(a[1]), u), 5);
    let tu = rel(tu);
    let v = drain((&tags).cross(&tu));
    rows(v.into_iter().map(|(_, ((t, f), (u, (n, a))))| {
        let mut r = vec![V::S(t), V::I(f)];
        r.extend(ucols(db, u, &["name"]));
        r.extend([V::I(n), nullable(a[1], a[0]), V::I(a[2]), V::I(a[3])]);
        row(r)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsCount,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersCount, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty, ROW_NUMBER() OVER (PARTITION BY u.Id ORDER BY COUNT(DISTINCT p.Id) DESC) AS ActivityRank
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 8 GROUP BY u.Id, u.DisplayName, u.Reputation),
// PopularPosts AS (SELECT p.Id, p.Title, p.Score, p.ViewCount, DENSE_RANK() OVER (ORDER BY p.Score DESC, p.ViewCount DESC) AS PostRank FROM Posts p WHERE p.CreationDate >= CURRENT_TIMESTAMP - INTERVAL '30 days'),
// ClosingReasons AS (SELECT ph.PostId, STRING_AGG(cr.Name, ', ') AS ClosingReasons FROM PostHistory ph JOIN CloseReasonTypes cr ON ph.Comment::integer = cr.Id WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId)
// SELECT ua.DisplayName, ua.Reputation, ua.TotalPosts, ua.QuestionsCount, ua.AnswersCount, ua.TotalBounty, pp.Title AS PopularPostTitle, pp.Score AS PopularPostScore, pp.ViewCount AS PopularPostViews, cr.ClosingReasons
// FROM UserActivity ua LEFT JOIN PopularPosts pp ON ua.UserId = pp.Id LEFT JOIN ClosingReasons cr ON pp.Id = cr.PostId
// WHERE ua.ActivityRank <= 10 ORDER BY ua.Reputation DESC, pp.Score DESC NULLS LAST;
//
// ActivityRank partitions by the group key, so it is always 1. The user's Id is joined to the post's Id, as written;
// CreationDate is compared with CURRENT_TIMESTAMP as an instant in the session zone.
fn q4718(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let v8 = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let ua = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(v8.opt())).opt()).fold([0i64; 3], |a, x| match x {
        Some((t, b)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + b.flatten().unwrap_or(0)],
        None => a,
    });
    let pc = user_distinct_posts(db);
    let since = now_utc() - 30 * DAY_US;
    let pidx: HashIdx<i64, Id<Post>> = db.post.with(creation_date.filt(move |d| ny_to_utc(d) >= since)).select(&db.post.origid).inv().collect();
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let cr = db
        .post_history
        .with(post_history_type_id.eq(10))
        .group_by(post)
        .select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason))
        .buf_fold(|v| -> Str { Box::leak(v.to_vec().join(", ").into_boxed_str()) });
    let v = drain((&pc).and(&ua).and((&db.user.origid).select(&pidx).select(Ident::<Post>::new().and((&cr).opt())).opt()));
    rows(v.into_iter().map(|(u, ((n, a), p))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        match p {
            Some((p, c)) => {
                f.extend(post_fields(db, p, &["title", "score", "views"]));
                f.push(ostr(c));
            }
            None => f.extend([V::Null, V::Null, V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH PostStatistics AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COALESCE(c.CommentCount, 0) AS CommentCount, COALESCE(a.AnswerCount, 0) AS AnswerCount, t.TagName,
//        COUNT(DISTINCT v.Id) AS TotalVotes, ROW_NUMBER() OVER (PARTITION BY t.TagName ORDER BY p.Score DESC) AS Rank
//     FROM Posts p LEFT JOIN (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
//     LEFT JOIN (SELECT ParentId, COUNT(*) AS AnswerCount FROM Posts WHERE PostTypeId = 2 GROUP BY ParentId) a ON p.Id = a.ParentId
//     LEFT JOIN (SELECT Id, unnest(string_to_array(substring(Tags, 2, length(Tags) - 2), '><')) AS TagName FROM Posts) t ON p.Id = t.Id
//     LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, c.CommentCount, a.AnswerCount, t.TagName),
// TopPosts AS (SELECT PostId, Title, CreationDate, Score, ViewCount, CommentCount, AnswerCount, TagName, TotalVotes, DENSE_RANK() OVER (ORDER BY Score DESC, ViewCount DESC) AS ScoreRank FROM PostStatistics)
// SELECT p.PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.CommentCount, p.AnswerCount, p.TagName, p.TotalVotes, CASE WHEN p.ScoreRank <= 10 THEN 'Top 10 Posts' ELSE 'Other Posts' END AS PostCategory
// FROM TopPosts p WHERE p.ScoreRank <= 50 ORDER BY p.Score DESC, p.ViewCount DESC;
//
// Every question has at least one (post, tag) row and the dense rank reads only its Score and ViewCount, so the questions are ranked first.
fn q9285(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, tags_str, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1)).select(score.and(view_count.opt())));
    let r = rel(ranked(v, |&(_, (s, w))| (Reverse(s), w.is_none(), Reverse(w)), true).into_iter().take_while(|x| x.1 <= 50).map(|((p, _), k)| (p, k)).collect());
    type R = (Id<Post>, i64);
    let pt: MatSet<(R, Option<Str>)> = (&r).select(Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0).select(tags_str.flat_map(tag_list).opt()))).collect();
    let tp: MatSet<Id<Post>> = (&r).map(|x: R| x.0).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let ac = (&tp).group_by(Ident::<Post>::new()).select(answers_of(db)).fold(0i64, |n, _| n + 1);
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db)).count_distinct();
    type J = (R, Option<Str>);
    let pr = || Same::<J>::new().map(|x: J| x.0 .0);
    let v = drain((&pt).select(Same::<J>::new().and(pr().select((&cc).opt().and((&ac).opt()).and((&vc).opt())))));
    rows(v.into_iter().map(|(_, (((p, k), t), ((c, a), n)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(c.unwrap_or(0)), V::I(a.unwrap_or(0)), ostr(t), V::I(n.unwrap_or(0))]);
        f.push(V::S(if k <= 10 { "Top 10 Posts" } else { "Other Posts" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, u.DisplayName AS OwnerDisplayName, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.PostTypeId = 1 AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// TopPosts AS (SELECT PostId, Title, OwnerDisplayName, CreationDate, Score FROM RankedPosts WHERE PostRank <= 5),
// PostVoteCounts AS (SELECT PostId, COUNT(CASE WHEN VoteTypeId = 2 THEN 1 END) AS UpvoteCount, COUNT(CASE WHEN VoteTypeId = 3 THEN 1 END) AS DownvoteCount FROM Votes GROUP BY PostId),
// ClosedPosts AS (SELECT ph.PostId, STRING_AGG(DISTINCT crt.Name, ', ') AS CloseReasons FROM PostHistory ph JOIN CloseReasonTypes crt ON ph.Comment = CAST(crt.Id AS VARCHAR)
//     WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.PostId)
// SELECT tp.PostId, tp.Title, tp.OwnerDisplayName, tp.CreationDate, tp.Score, COALESCE(pvc.UpvoteCount, 0) AS UpvoteCount, COALESCE(pvc.DownvoteCount, 0) AS DownvoteCount,
//        COALESCE(cp.CloseReasons, 'Not Closed') AS CloseReasons
// FROM TopPosts tp LEFT JOIN PostVoteCounts pvc ON tp.PostId = pvc.PostId LEFT JOIN ClosedPosts cp ON tp.PostId = cp.PostId ORDER BY tp.Score DESC, tp.CreationDate DESC;
fn q5514(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(owner_user));
    let tp = set_of(top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 5, false).into_iter().map(|x| x.0).collect());
    let vc = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let reason: HashIdx<Str, Str> = (&db.close_reason_type.origid).map(|i: i64| -> Str { Box::leak(i.to_string().into_boxed_str()) }).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let cp = db.post_history.with(post_history_type_id.is_in([10, 11])).group_by(post).select(comment.select(&reason)).buf_fold(|v| distinct_join(&v));
    let v = drain((&tp).select((&vc).opt().and((&cp).opt())));
    rows(v.into_iter().map(|(p, (a, c))| {
        let a = a.unwrap_or([0, 0]);
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "score"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::S(c.unwrap_or("Not Closed"))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.OwnerUserId, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank,
//        COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.PostTypeId = 1 AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// FilteredPosts AS (SELECT rp.Id, rp.Title, rp.OwnerUserId, rp.CreationDate, rp.Score, rp.CommentCount FROM RankedPosts rp WHERE rp.Rank = 1 AND rp.Score > 10),
// PostHistoryStats AS (SELECT ph.PostId, COUNT(*) AS EditCount, MAX(ph.CreationDate) AS LastEditDate FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (4, 5, 6) GROUP BY ph.PostId)
// SELECT fp.Title, u.DisplayName AS Owner, fp.CreationDate, fp.Score, COALESCE(phs.EditCount, 0) AS EditCount, phs.LastEditDate,
//        (SELECT string_agg(DISTINCT c.UserDisplayName, ', ') FROM Comments c WHERE c.PostId = fp.Id) AS Commenters,
//        CASE WHEN fp.CommentCount > 0 THEN 'Has Comments' ELSE 'No Comments' END AS CommentStatus
// FROM FilteredPosts fp JOIN Users u ON fp.OwnerUserId = u.Id LEFT JOIN PostHistoryStats phs ON fp.Id = phs.PostId
// WHERE fp.Score > (SELECT AVG(Score) FROM Posts WHERE PostTypeId = 1) ORDER BY fp.Score DESC LIMIT 50;
//
// Rank = 1 keeps one post x comment row per owner, the owner's latest question. Partitioned by the owner edge, since the Users join drops the rest.
fn q4114(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).select(owner_user));
    let first = set_of(top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false).into_iter().map(|x| x.0).collect());
    let (sum, n) = db.post.with(post_type_id.eq(1)).select(score).fold_flat((0i128, 0i128), |(s, n), x| (s + x as i128, n + 1));
    let fp = || (&first).with(score.gt(10).and(score.filt(move |s| s as i128 * n > sum)));
    let cc = fp().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let phs = db.post_history.with(post_history_type_id.is_in([4, 5, 6])).group_by(post).select(hd).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let cm = fp().group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.user_display_name)).buf_fold(|v| distinct_join(&v));
    let v = drain((&cc).and((&phs).opt()).and((&cm).opt()));
    let v = top_n(v, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 50);
    rows(v.into_iter().map(|(p, ((c, h), m))| {
        let mut f = post_fields(db, p, &["title", "owner", "created", "score"]);
        f.extend([V::I(h.map_or(0, |h| h.0)), ots(h.map(|h| h.1)), ostr(m), V::S(if c > 0 { "Has Comments" } else { "No Comments" })]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U),
// PostActivity AS (SELECT P.Id AS PostId, P.Title, COUNT(C.Id) AS CommentCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY P.Id, P.Title),
// ClosedPosts AS (SELECT PH.PostId, COUNT(*) AS CloseCount, STRING_AGG(DISTINCT C.Name, ', ') AS CloseReasons FROM PostHistory PH JOIN CloseReasonTypes C ON PH.Comment::integer = C.Id
//     WHERE PH.PostHistoryTypeId = 10 GROUP BY PH.PostId),
// PostStats AS (SELECT PA.PostId, PA.Title, PA.CommentCount, PA.UpVoteCount, PA.DownVoteCount, COALESCE(CP.CloseCount, 0) AS CloseCount, COALESCE(CP.CloseReasons, 'None') AS CloseReasons
//     FROM PostActivity PA LEFT JOIN ClosedPosts CP ON PA.PostId = CP.PostId)
// SELECT UR.UserId, UR.DisplayName, PS.PostId, PS.Title, PS.CommentCount, PS.UpVoteCount, PS.DownVoteCount, PS.CloseCount, PS.CloseReasons
// FROM UserReputation UR JOIN Posts P ON UR.UserId = P.OwnerUserId JOIN PostStats PS ON P.Id = PS.PostId WHERE UR.ReputationRank <= 10
// ORDER BY UR.Reputation DESC, PS.UpVoteCount DESC, PS.CommentCount DESC;
fn q1997(db: &'static So) -> String {
    let Post { creation_date, .. } = &db.post;
    let tu = set_of(ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false).into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect());
    let recent = Ident::<Post>::new().with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let tp: MatSet<Id<Post>> = (&tu).select(posts_of(db)).select(recent).collect();
    let pa = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let cp = db
        .post_history
        .with(post_history_type_id.eq(10))
        .group_by(post)
        .select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason))
        .buf_fold(|v| (v.len() as i64, distinct_join(&v)));
    let v = drain((&pa).and((&cp).opt()));
    rows(v.into_iter().map(|(p, (a, c))| {
        let mut f = post_fields(db, p, &["uid", "owner", "id", "title"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(c.map_or(0, |c| c.0)), V::S(c.map_or("None", |c| c.1))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS Ranking,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) OVER (PARTITION BY p.Id), 0) AS Upvotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) OVER (PARTITION BY p.Id), 0) AS Downvotes
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserBadges AS (SELECT b.UserId, COUNT(b.Id) AS BadgesCount, STRING_AGG(b.Name, ', ') AS BadgeNames FROM Badges b GROUP BY b.UserId),
// CloseReasons AS (SELECT ph.PostId, STRING_AGG(CASE WHEN ph.Comment IS NOT NULL THEN CONCAT('Closed for: ', cr.Name) END, ', ') AS CloseReason
//     FROM PostHistory ph JOIN CloseReasonTypes cr ON CAST(ph.Comment AS INT) = cr.Id WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId)
// SELECT up.PostId, up.Title, up.CreationDate, up.ViewCount, up.Score, ub.BadgesCount, ub.BadgeNames, cr.CloseReason, CASE WHEN cr.CloseReason IS NULL THEN 'Open' ELSE 'Closed' END AS PostStatus,
//        (up.Upvotes - up.Downvotes) AS NetVotes
// FROM RankedPosts up LEFT JOIN UserBadges ub ON up.PostId = ub.UserId LEFT JOIN CloseReasons cr ON up.PostId = cr.PostId
// WHERE up.Ranking <= 10 ORDER BY up.Score DESC FETCH FIRST 50 ROWS ONLY;
//
// Ranking numbers the post x vote rows, so a post with several votes fills several ranks. The post's Id is joined to the badge's UserId, as written.
fn q22019(db: &'static So) -> String {
    let Post { creation_date, post_type_id, origid, score, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    type J = (Id<Post>, Option<Id<Vote>>);
    let j: Vec<J> = drain(recent().select(Ident::<Post>::new().and(votes_of(db).opt()))).into_iter().map(|x| x.1).collect();
    let top = rel(top_per(j, |&(p, _)| post_type_id.get(p).unwrap(), |&(p, v)| (Reverse(creation_date.get(p).unwrap()), p, v), 10, false));
    let vc = recent().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold(0i64, |n, t| n + (t == 2) as i64 - (t == 3) as i64);
    let ub = db.badge.group_by(&db.badge.user_id).select(&db.badge.name).buf_fold(|v| -> (i64, Str) { (v.len() as i64, Box::leak(v.to_vec().join(", ").into_boxed_str())) });
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let cr = db
        .post_history
        .with(post_history_type_id.eq(10))
        .group_by(post)
        .select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason))
        .buf_fold(|v| -> Str { Box::leak(v.iter().map(|n| format!("Closed for: {n}")).collect::<Vec<_>>().join(", ").into_boxed_str()) });
    let pr = || Same::<J>::new().map(|x: J| x.0);
    let v = drain((&top).select(pr().and(pr().select(&vc).opt()).and(pr().select(origid).select(&ub).opt()).and(pr().select(&cr).opt())));
    let v = top_n(v, |&(i, (((p, _), _), _))| (Reverse(score.get(p).unwrap()), i), 50);
    rows(v.into_iter().map(|(_, (((p, n), b), c))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([oint(b.map(|b| b.0)), ostr(b.map(|b| b.1)), ostr(c), V::S(if c.is_none() { "Open" } else { "Closed" }), V::I(n.unwrap_or(0))]);
        row(f)
    }))
}

// WITH PostDetails AS (SELECT p.Id AS PostId, p.Title, p.Score, p.AnswerCount, COALESCE(u.DisplayName, 'Anonymous') AS Owner, STRING_AGG(DISTINCT t.TagName, ', ') AS Tags, COUNT(DISTINCT c.Id) AS CommentCount
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Tags t ON t.ExcerptPostId = p.Id LEFT JOIN Comments c ON c.PostId = p.Id
//     WHERE p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.Score, p.AnswerCount, u.DisplayName),
// TopPosts AS (SELECT PostId, Title, Score, Owner, Tags, CommentCount, RANK() OVER (ORDER BY Score DESC) AS Rank FROM PostDetails WHERE AnswerCount > 0),
// FailedVotes AS (SELECT p.Id AS PostId, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id)
// SELECT tp.Title, tp.Score, tp.Owner, tp.Tags, tp.CommentCount, COALESCE(fv.DownVotes, 0) AS DownVotes, COALESCE(fv.UpVotes, 0) AS UpVotes,
//        CASE WHEN COALESCE(fv.DownVotes, 0) > COALESCE(fv.UpVotes, 0) THEN 'Needs Improvement' WHEN COALESCE(fv.DownVotes, 0) = COALESCE(fv.UpVotes, 0) THEN 'Balanced' ELSE 'Well Received' END AS PostReception
// FROM TopPosts tp LEFT JOIN FailedVotes fv ON tp.PostId = fv.PostId WHERE tp.Rank <= 10 ORDER BY tp.Score DESC;
//
// The rank reads only Score, so the posts are ranked first and the aggregates are taken only for them.
fn q2236(db: &'static So) -> String {
    let Post { creation_date, answer_count, score, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(answer_count.gt(0))).select(score));
    let tp = set_of(ranked(v, |&(_, s)| Reverse(s), false).into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect());
    let ex: HashIdx<Id<Post>, Id<Tag>> = (&db.tag.excerpt_post).inv().collect();
    let tg = (&tp).group_by(Ident::<Post>::new()).select((&ex).select(&db.tag.tag_name)).buf_fold(|v| distinct_join(&v));
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db)).count_distinct();
    let fv = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 3) as i64, a[1] + (t == 2) as i64]);
    let v = drain((&tp).select((&tg).opt().and((&cc).opt()).and((&fv).opt())));
    rows(v.into_iter().map(|(p, ((t, c), a))| {
        let a = a.unwrap_or([0, 0]);
        let mut f = post_fields(db, p, &["title", "score"]);
        f.push(V::S(owner_user.get(p).map_or("Anonymous", |u| db.user.display_name.get(u).unwrap())));
        f.extend([ostr(t), V::I(c.unwrap_or(0)), V::I(a[0]), V::I(a[1])]);
        f.push(V::S(if a[0] > a[1] { "Needs Improvement" } else if a[0] == a[1] { "Balanced" } else { "Well Received" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserRank,
//        COALESCE(u.DisplayName, 'Anonymous') AS OwnerDisplay FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// ClosedPosts AS (SELECT ph.PostId, COUNT(ph.Id) AS CloseCount, STRING_AGG(DISTINCT cr.Name, ', ') AS CloseReasons FROM PostHistory ph JOIN CloseReasonTypes cr ON CAST(ph.Comment AS INTEGER) = cr.Id
//     WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId),
// PostsWithVotes AS (SELECT p.Id AS PostId, COUNT(v.Id) AS VoteCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, COALESCE(cp.CloseCount, 0) AS CloseCount, COALESCE(cp.CloseReasons, 'No close reasons') AS CloseReasons,
//        pw.VoteCount, pw.UpVotes, pw.DownVotes, rp.OwnerDisplay
// FROM RankedPosts rp LEFT JOIN ClosedPosts cp ON rp.PostId = cp.PostId LEFT JOIN PostsWithVotes pw ON rp.PostId = pw.PostId
// WHERE rp.UserRank = 1 AND (rp.Score > 5 OR rp.ViewCount > 100) ORDER BY rp.CreationDate DESC LIMIT 100;
fn q1596(db: &'static So) -> String {
    let Post { creation_date, owner_user_id, owner_user, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user_id.opt()));
    let first = set_of(top_per(v, |&(_, u)| u, |&(p, _)| Reverse(creation_date.get(p).unwrap()), 1, true).into_iter().map(|x| x.0).collect());
    let fp = || (&first).with(score.gt(5).or(view_count.gt(100)));
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let cp = db
        .post_history
        .with(post_history_type_id.eq(10))
        .group_by(post)
        .select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason))
        .buf_fold(|v| (v.len() as i64, distinct_join(&v)));
    let pw = fp().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| [a[0] + t.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let v = drain((&pw).and((&cp).opt()));
    let v = top_n(v, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 100);
    rows(v.into_iter().map(|(p, (a, c))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers"]);
        f.extend([V::I(c.map_or(0, |c| c.0)), V::S(c.map_or("No close reasons", |c| c.1)), V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        f.push(V::S(owner_user.get(p).map_or("Anonymous", |u| db.user.display_name.get(u).unwrap())));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.OwnerUserId, p.PostTypeId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS rn
//     FROM Posts p WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '30 days' AND p.Score IS NOT NULL),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount FROM Users u JOIN Posts p ON u.Id = p.OwnerUserId WHERE u.Reputation >= 1000 GROUP BY u.Id, u.Reputation),
// CloseReasonCounts AS (SELECT ph.PostId, COUNT(*) AS CloseReasonCount FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId),
// UserBadges AS (SELECT b.UserId, STRING_AGG(b.Name, ', ') AS BadgeNames FROM Badges b GROUP BY b.UserId),
// PostAnalytics AS (SELECT rp.PostId, rp.Title, rp.Score, ur.Reputation, COALESCE(cr.CloseReasonCount, 0) AS CloseCount, ub.BadgeNames
//     FROM RankedPosts rp JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId LEFT JOIN CloseReasonCounts cr ON rp.PostId = cr.PostId LEFT JOIN UserBadges ub ON rp.OwnerUserId = ub.UserId)
// SELECT pa.PostId, pa.Title, pa.Score, pa.Reputation, pa.CloseCount, pa.BadgeNames FROM PostAnalytics pa WHERE pa.CloseCount = 0 AND pa.Reputation >= 1000
// ORDER BY pa.Score DESC, pa.Reputation DESC LIMIT 100 OFFSET 0;
//
// rn is never read. The owner of a recent post always has a post, so UserReputation is the users with Reputation >= 1000.
fn q33391(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, .. } = &db.post;
    let recent = db.post.with(creation_date.ge(add_days(current_date(), -30)));
    let cr = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10)));
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.name)).buf_fold(|v| -> Str { Box::leak(v.to_vec().join(", ").into_boxed_str()) });
    let v = drain(recent.minus(cr).select(owner_user.select(Ident::<User>::new().with((&db.user.reputation).ge(1000)).and((&ub).opt()))));
    let v = top_n(v, |&(p, (u, _))| (Reverse(score.get(p).unwrap()), Reverse(db.user.reputation.get(u).unwrap()), p), 100);
    rows(v.into_iter().map(|(p, (u, b))| {
        let mut f = post_fields(db, p, &["id", "title", "score"]);
        f.extend([user_col(db, u, "rep"), V::I(0), ostr(b)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.Score, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.ViewCount DESC) AS RankByViews,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS RankByScore
//     FROM Posts p WHERE p.CreationDate >= (cast('2024-10-01' as date) - INTERVAL '1 year') AND p.ViewCount IS NOT NULL),
// UserVoteStats AS (SELECT u.Id AS UserId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes,
//        COUNT(DISTINCT v.PostId) AS TotalVotes FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id),
// CloseReasons AS (SELECT ph.PostId, STRING_AGG(DISTINCT crt.Name, ', ') AS CloseReasonNames FROM PostHistory ph JOIN CloseReasonTypes crt ON ph.Comment::int = crt.Id WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.ViewCount, rp.Score, us.TotalUpVotes, us.TotalDownVotes, cr.CloseReasonNames,
//        CASE WHEN rp.RankByViews <= 5 THEN 'Top Viewed' WHEN rp.RankByScore <= 5 THEN 'Top Scored' ELSE 'Others' END AS ViewScoreCategory
// FROM RankedPosts rp LEFT JOIN UserVoteStats us ON rp.PostId = us.UserId LEFT JOIN CloseReasons cr ON rp.PostId = cr.PostId
// WHERE (cr.CloseReasonNames IS NOT NULL OR us.TotalVotes > 0) AND rp.RankByViews < 10 ORDER BY rp.ViewCount DESC NULLS LAST, rp.Score DESC NULLS LAST;
//
// The post's Id is joined to the user's Id, as written.
fn q21549(db: &'static So) -> String {
    let Post { creation_date, view_count, score, post_type_id, origid, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1))).with(view_count).select(post_type_id));
    let r = per_group(ranked(v, |&(p, t)| (t, Reverse(view_count.get(p)), origid.get(p).unwrap()), false), |x| x.1);
    let r = per_group(ranked(r, |&((p, t), _)| (t, Reverse(score.get(p).unwrap()), origid.get(p).unwrap()), false), |x| x.0 .1);
    let rv = rel(r.into_iter().map(|(((p, _), vr), sr)| (p, vr, sr)).collect());
    let us = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select((&db.vote.vote_type_id).and(&db.vote.post_id)).opt()).buf_fold(|v| {
        let up = v.iter().filter(|x| matches!(x, Some((2, _)))).count() as i64;
        let dn = v.iter().filter(|x| matches!(x, Some((3, _)))).count() as i64;
        (up, dn, distinct_some(v.iter().map(|x| x.map(|y| y.1))))
    });
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let cr = db
        .post_history
        .with(post_history_type_id.eq(10))
        .group_by(post)
        .select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason))
        .buf_fold(|v| distinct_join(&v));
    type R = (Id<Post>, i64, i64);
    type J = ((R, Option<(i64, i64, i64)>), Option<Str>);
    let pr = || Same::<R>::new().map(|x: R| x.0);
    let v = drain(
        (&rv)
            .filt(|x: R| x.1 < 10)
            .select(Same::<R>::new().and(pr().select(origid).select(&uidx).select(&us).opt()).and(pr().select(&cr).opt()))
            .filt(|((_, u), c): J| c.is_some() || u.map_or(false, |u| u.2 > 0)),
    );
    rows(v.into_iter().map(|(_, (((p, vr, sr), u), c))| {
        let mut f = post_fields(db, p, &["id", "title", "views", "score"]);
        f.extend([oint(u.map(|u| u.0)), oint(u.map(|u| u.1)), ostr(c)]);
        f.push(V::S(if vr <= 5 { "Top Viewed" } else if sr <= 5 { "Top Scored" } else { "Others" }));
        row(f)
    }))
}

// WITH UserBadgeCounts AS (SELECT u.Id AS UserId, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PopularPosts AS (SELECT p.Id, p.Title, p.Score, p.ViewCount, p.OwnerUserId, RANK() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS RankByPopularity
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 month' AND p.Score IS NOT NULL),
// CloseReasonCounts AS (SELECT ph.PostId, COUNT(*) AS CloseReasonCount, STRING_AGG(CASE WHEN ph.Comment IS NOT NULL THEN ph.Comment END, ', ') AS Reasons FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId)
// SELECT u.DisplayName, ubc.GoldBadges, ubc.SilverBadges, ubc.BronzeBadges, pp.Title, pp.Score, pp.ViewCount, COALESCE(crc.CloseReasonCount, 0) AS TotalCloseReasons,
//        COALESCE(crc.Reasons, 'No close reasons') AS CloseReasons,
//        CASE WHEN ubc.GoldBadges > 0 THEN 'Top Contributor' WHEN ubc.SilverBadges > 0 OR ubc.BronzeBadges > 0 THEN 'Active Contributor' ELSE 'Newcomer' END AS UserStatus
// FROM UserBadgeCounts ubc JOIN Users u ON u.Id = ubc.UserId LEFT JOIN PopularPosts pp ON pp.OwnerUserId = u.Id AND pp.RankByPopularity <= 5
// LEFT JOIN CloseReasonCounts crc ON pp.Id = crc.PostId WHERE u.Reputation >= 100 ORDER BY u.Reputation DESC, pp.Score DESC NULLS LAST;
fn q22890(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, view_count, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let pp = set_of(top_per(v, |&(_, t)| t, |&(p, _)| { let w = view_count.get(p); (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w)) }, 5, true).into_iter().map(|x| x.0).collect());
    let ppi: HashIdx<Id<User>, Id<Post>> = (&pp).select(owner_user).inv().collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]);
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let crc = db.post_history.with(post_history_type_id.eq(10)).group_by(post).select(comment.opt()).buf_fold(|v| -> (i64, Option<Str>) {
        let c: Vec<Str> = v.iter().flatten().copied().collect();
        (v.len() as i64, if c.is_empty() { None } else { Some(Box::leak(c.join(", ").into_boxed_str())) })
    });
    let v = drain(db.user.with((&db.user.reputation).ge(100)).select((&ub).and((&ppi).select(Ident::<Post>::new().and((&crc).opt())).opt())));
    rows(v.into_iter().map(|(u, (a, p))| {
        let mut f = ucols(db, u, &["name"]);
        f.extend(a.map(V::I));
        match p {
            Some((p, c)) => {
                f.extend(post_fields(db, p, &["title", "score", "views"]));
                f.extend([V::I(c.map_or(0, |c| c.0)), V::S(c.and_then(|c| c.1).unwrap_or("No close reasons"))]);
            }
            None => f.extend([V::Null, V::Null, V::Null, V::I(0), V::S("No close reasons")]),
        }
        f.push(V::S(if a[0] > 0 { "Top Contributor" } else if a[1] > 0 || a[2] > 0 { "Active Contributor" } else { "Newcomer" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Tags, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, COUNT(DISTINCT c.Id) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS rn
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
//     GROUP BY p.Id, p.Title, p.Tags, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount),
// PostDiversity AS (SELECT rp.PostId, COUNT(DISTINCT tg.TagName) AS UniqueTagsCount FROM RankedPosts rp
//     CROSS JOIN LATERAL (SELECT TRIM(both '<>' FROM unnest(string_to_array(rp.Tags, ','))) AS Tag) AS tag INNER JOIN Tags tg ON tg.TagName = tag.Tag GROUP BY rp.PostId),
// ResultSet AS (SELECT rp.PostId, rp.Title, rp.ViewCount, rp.Score, rp.AnswerCount, rp.CommentCount, pd.UniqueTagsCount, DENSE_RANK() OVER (ORDER BY rp.Score DESC) AS ScoreRank
//     FROM RankedPosts rp JOIN PostDiversity pd ON rp.PostId = pd.PostId WHERE rp.rn = 1)
// SELECT rs.PostId, rs.Title, rs.ViewCount, rs.Score, rs.AnswerCount, rs.CommentCount, rs.UniqueTagsCount, rs.ScoreRank,
//        CASE WHEN rs.UniqueTagsCount >= 5 THEN 'Highly Diverse' WHEN rs.UniqueTagsCount BETWEEN 3 AND 4 THEN 'Moderately Diverse' ELSE 'Low Diversity' END AS TagDiversity
// FROM ResultSet rs WHERE rs.ScoreRank <= 100 ORDER BY rs.Score DESC, rs.ViewCount DESC;
//
// rn partitions by the post itself, so it is always 1.
fn q25706(db: &'static So) -> String {
    let Post { creation_date, tags_str, score, .. } = &db.post;
    let recent = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let tn: HashIdx<Str, Id<Tag>> = (&db.tag.tag_name).inv().collect();
    let pd = recent
        .group_by(Ident::<Post>::new())
        .select(tags_str.flat_map(|s: Str| s.split(',')).map(|s: Str| s.trim_matches(|c: char| c == '<' || c == '>')).select(&tn).select(&db.tag.tag_name))
        .count_distinct();
    let r = ranked(drain((&pd).and(score)), |&(_, (_, s))| Reverse(s), true);
    let rs = set_of(r.iter().take_while(|x| x.1 <= 100).map(|x| (x.0 .0, x.1)).collect());
    type R = (Id<Post>, i64);
    let pr = || Same::<R>::new().map(|x: R| x.0);
    let cc = (&rs).map(|x: R| x.0).group_by(Ident::<Post>::new()).select(comments_of(db)).count_distinct();
    let v = drain((&rs).select(Same::<R>::new().and(pr().select(&pd)).and(pr().select(&cc).opt())));
    rows(v.into_iter().map(|(_, (((p, k), n), c))| {
        let mut f = post_fields(db, p, &["id", "title", "views", "score", "answers"]);
        f.extend([V::I(c.unwrap_or(0)), V::I(n), V::I(k)]);
        f.push(V::S(if n >= 5 { "Highly Diverse" } else if (3..=4).contains(&n) { "Moderately Diverse" } else { "Low Diversity" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, u.DisplayName AS Author, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS PostRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// ClosedPosts AS (SELECT ph.PostId, COUNT(*) AS CloseCount, STRING_AGG(DISTINCT cr.Name, ', ') AS CloseReasons FROM PostHistory ph JOIN CloseReasonTypes cr ON CAST(ph.Comment AS int) = cr.Id
//     WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.PostId),
// PostStatistics AS (SELECT rp.Id, rp.Title, rp.Author, COALESCE(cp.CloseCount, 0) AS CloseCount, COALESCE(cp.CloseReasons, 'No Reasons') AS CloseReasons,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, AVG(v.BountyAmount) AS AverageBounty
//     FROM RankedPosts rp LEFT JOIN ClosedPosts cp ON rp.Id = cp.PostId LEFT JOIN Votes v ON rp.Id = v.PostId GROUP BY rp.Id, rp.Title, rp.Author, cp.CloseCount, cp.CloseReasons)
// SELECT ps.Id, ps.Title, ps.Author, ps.CloseCount, ps.CloseReasons, ps.UpVotes, ps.DownVotes, ps.AverageBounty, CASE WHEN ps.CloseCount > 0 THEN 'Closed' ELSE 'Open' END AS PostStatus
// FROM PostStatistics ps WHERE ps.CloseCount > 2 OR ps.UpVotes > 10 ORDER BY ps.UpVotes DESC, ps.CloseCount ASC LIMIT 100;
fn q4698(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let rp = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(owner_user);
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let cp = db
        .post_history
        .with(post_history_type_id.is_in([10, 11]))
        .group_by(post)
        .select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason))
        .buf_fold(|v| (v.len() as i64, distinct_join(&v)));
    let ps = rp()
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select((&db.vote.vote_type_id).and((&db.vote.bounty_amount).opt())).opt())
        .fold([0i64; 4], |a, v| match v {
            Some((t, b)) => [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + b.is_some() as i64, a[3] + b.unwrap_or(0)],
            None => a,
        });
    type J = ([i64; 4], Option<(i64, Str)>);
    let v = drain((&ps).and((&cp).opt()).filt(|(a, c): J| c.map_or(0, |c| c.0) > 2 || a[0] > 10));
    let v = top_n(v, |&(p, (a, c))| (Reverse(a[0]), c.map_or(0, |c| c.0), p), 100);
    rows(v.into_iter().map(|(p, (a, c))| {
        let n = c.map_or(0, |c| c.0);
        let mut f = post_fields(db, p, &["id", "title", "owner"]);
        f.extend([V::I(n), V::S(c.map_or("No Reasons", |c| c.1)), V::I(a[0]), V::I(a[1]), avg(a[3], a[2]), V::S(if n > 0 { "Closed" } else { "Open" })]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank, p.OwnerUserId
//     FROM Posts p WHERE p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// ClosedPosts AS (SELECT ph.PostId, COUNT(*) AS CloseCount, STRING_AGG(DISTINCT cr.Name, ', ') AS CloseReasons FROM PostHistory ph JOIN CloseReasonTypes cr ON CAST(ph.Comment AS INT) = cr.Id
//     WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.PostId),
// UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT b.Id) AS BadgesCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesCount FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName)
// SELECT us.DisplayName, us.UserId, rp.PostId, rp.Title, COALESCE(cp.CloseCount, 0) AS NumberOfTimesClosed, COALESCE(cp.CloseReasons, 'None') AS ClosedReasons, us.BadgesCount, us.UpVotesCount, us.DownVotesCount
// FROM UserStatistics us JOIN RankedPosts rp ON us.UserId = rp.OwnerUserId LEFT JOIN ClosedPosts cp ON rp.PostId = cp.PostId
// WHERE rp.PostRank = 1 AND (us.UpVotesCount - us.DownVotesCount) > 10 ORDER BY us.BadgesCount DESC, rp.ViewCount DESC LIMIT 50;
//
// Partitioned by the owner edge: only posts whose owner exists meet a user.
fn q4685(db: &'static So) -> String {
    let Post { creation_date, owner_user, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user));
    let first = set_of(top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false).into_iter().map(|x| x.0).collect());
    let owners: MatSet<Id<User>> = (&first).select(owner_user).collect();
    let us = (&owners)
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(votes_by(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let bc = (&owners).group_by(Ident::<User>::new()).select(badges_of(db)).count_distinct();
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let cp = db
        .post_history
        .with(post_history_type_id.is_in([10, 11]))
        .group_by(post)
        .select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason))
        .buf_fold(|v| (v.len() as i64, distinct_join(&v)));
    let v = drain((&first).select(owner_user.select(Ident::<User>::new().and((&us).filt(|a| a[0] - a[1] > 10)).and((&bc).opt())).and((&cp).opt())));
    let v = top_n(v, |&(p, ((_, b), _))| { let w = view_count.get(p); (Reverse(b.unwrap_or(0)), w.is_none(), Reverse(w), p) }, 50);
    rows(v.into_iter().map(|(p, (((u, a), b), c))| {
        let mut f = ucols(db, u, &["name", "uid"]);
        f.extend(post_fields(db, p, &["id", "title"]));
        f.extend([V::I(c.map_or(0, |c| c.0)), V::S(c.map_or("None", |c| c.1)), V::I(b.unwrap_or(0)), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, ROW_NUMBER() OVER (PARTITION BY U.Id ORDER BY p.Score DESC) AS RankByScore,
//        COUNT(com.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount
//     FROM Posts p JOIN Users U ON p.OwnerUserId = U.Id LEFT JOIN Comments com ON p.Id = com.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '6 months' GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, U.Id),
// FilteredPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.CommentCount, rp.UpvoteCount, U.Reputation
//     FROM RankedPosts rp JOIN Users U ON rp.PostId IN (SELECT AcceptedAnswerId FROM Posts WHERE AcceptedAnswerId IS NOT NULL) WHERE rp.RankByScore <= 5)
// SELECT FP.PostId, FP.Title, FP.CreationDate, FP.ViewCount, FP.Score, FP.CommentCount, FP.UpvoteCount, U.DisplayName, U.Reputation, COUNT(DISTINCT b.Id) AS BadgeCount, ARRAY_AGG(DISTINCT b.Name) AS BadgeNames
// FROM FilteredPosts FP JOIN Users U ON FP.Reputation > 1000 LEFT JOIN Badges b ON U.Id = b.UserId WHERE U.Location IS NOT NULL
// GROUP BY FP.PostId, FP.Title, FP.CreationDate, FP.ViewCount, FP.Score, FP.CommentCount, FP.UpvoteCount, U.Id, U.DisplayName, U.Reputation ORDER BY FP.Score DESC, FP.CommentCount DESC LIMIT 10;
//
// Both Users joins name only one side, so they are cross joins; the GROUP BY drops the first user, so a group is (post, second user)
// and exists when any first user has Reputation > 1000. The rank reads only Score, so the posts are ranked first.
fn q33015(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user, score, accepted_answer, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1).and(creation_date.ge(add_months(ts(2024, 10, 1, 12, 34, 56), -6)))).select(owner_user));
    let tp = set_of(top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false).into_iter().map(|x| x.0).collect());
    let acc: MatSet<Id<Post>> = accepted_answer.map(|p: Id<Post>| p).collect();
    let fp: MatSet<Id<Post>> = (&tp).with(&acc).collect();
    let u1: MatSet<Id<User>> = db.user.with((&db.user.reputation).gt(1000)).collect();
    let u2: MatSet<Id<User>> = db.user.with(&db.user.location).collect();
    let g: MatSet<(Id<Post>, Id<User>)> = (&fp).cross(&u1).cross(&u2).map(|((p, _), u): ((Id<Post>, Id<User>), Id<User>)| (p, u)).collect();
    type G = (Id<Post>, Id<User>);
    let pa = (&fp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64]);
    let bs = (&u2).group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.name)).buf_fold(|v| (v.len() as i64, distinct_join(&v)));
    let v = drain((&g).select(Same::<G>::new().and(Same::<G>::new().map(|x: G| x.0).select(&pa)).and(Same::<G>::new().map(|x: G| x.1).select(&bs).opt())));
    let v = top_n(v, |&(_, (((p, u), a), _))| (Reverse(score.get(p).unwrap()), Reverse(a[0]), p, u), 10);
    rows(v.into_iter().map(|(_, (((p, u), a), b))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::I(b.map_or(0, |b| b.0)), V::Owned(format!("[{}]", b.map_or("NULL", |b| b.1)))]);
        row(f)
    }))
}

// WITH ProcessedTags AS (SELECT p.Id AS PostId, LOWER(t.TagName) AS ProcessedTagName FROM Posts p JOIN UNNEST(string_to_array(substring(p.Tags, 2, length(p.Tags) - 2), '><')) AS t(TagName) ON true),
// TagMetrics AS (SELECT PostId, ProcessedTagName, COUNT(PostId) AS TagFrequency FROM ProcessedTags GROUP BY PostId, ProcessedTagName),
// UserEngagement AS (SELECT u.Id AS UserId, u.DisplayName, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotesReceived,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotesReceived, COUNT(DISTINCT c.Id) AS CommentsMade
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY u.Id, u.DisplayName),
// TopPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, p.CreationDate, ROW_NUMBER() OVER (ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank, p.OwnerUserId FROM Posts p WHERE p.PostTypeId = 1)
// SELECT tp.PostId, tp.Title, tp.Score, tp.ViewCount, tm.ProcessedTagName, tm.TagFrequency, ue.DisplayName AS UserAuthor, ue.UpVotesReceived, ue.DownVotesReceived, ue.CommentsMade
// FROM TopPosts tp JOIN TagMetrics tm ON tp.PostId = tm.PostId JOIN Users u ON tp.OwnerUserId = u.Id JOIN UserEngagement ue ON u.Id = ue.UserId
// WHERE tp.Rank <= 10 ORDER BY tp.Score DESC, tm.TagFrequency DESC;
//
// The rank reads only base columns, so the questions are ranked first and the users' products are driven only for their owners.
fn q29400(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, tags_str, owner_user, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1)).select(score.and(view_count.opt())));
    let tp = set_of(top_n(v, |&(p, (s, w))| (Reverse(s), w.is_none(), Reverse(w), p), 10).into_iter().map(|x| x.0).collect());
    let tm = (&tp).select(Ident::<Post>::new().and(tags_str.flat_map(tag_list).map(|t: Str| -> Str { Box::leak(t.to_lowercase().into_boxed_str()) }))).group_by(Same::<(Id<Post>, Str)>::new()).fold(0i64, |n, _| n + 1);
    let owners: MatSet<Id<User>> = (&tp).select(owner_user).collect();
    let ue = (&owners)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt())).opt())
        .fold([0i64; 2], |a, x| {
            let t = x.and_then(|x| x.0);
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
        });
    let uc = (&owners).group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db))).count_distinct();
    type K = (Id<Post>, Str);
    let v = drain((&tm).and(Same::<K>::new().map(|x: K| x.0).select(owner_user).select(Ident::<User>::new().and(&ue).and((&uc).opt()))));
    rows(v.into_iter().map(|((p, t), (n, ((u, a), c)))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views"]);
        f.extend([V::S(t), V::I(n)]);
        f.extend(ucols(db, u, &["name"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(c.unwrap_or(0))]);
        row(f)
    }))
}

// WITH UserStatistics AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, u.Views, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
//        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation, u.Views),
// PostScores AS (SELECT p.OwnerUserId, SUM(p.Score) AS TotalScore, COUNT(p.Id) AS PostCount FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.OwnerUserId),
// UserRanking AS (SELECT us.UserId, us.DisplayName, us.Reputation, us.Views, ps.TotalScore, ps.PostCount, RANK() OVER (ORDER BY us.Reputation DESC, ps.TotalScore DESC) AS UserRank
//     FROM UserStatistics us JOIN PostScores ps ON us.UserId = ps.OwnerUserId)
// SELECT ur.UserId, ur.DisplayName, ur.Reputation, ur.Views, ur.TotalScore, ur.PostCount, ur.UserRank, COALESCE(t.TagName, 'No Tags') AS MostFrequentTag
// FROM UserRanking ur LEFT JOIN (SELECT p.OwnerUserId, t.TagName, COUNT(*) AS TagCount FROM Posts p, UNNEST(string_to_array(p.Tags, '><')) AS t(TagName)
//     GROUP BY p.OwnerUserId, t.TagName ORDER BY TagCount DESC) t ON ur.UserId = t.OwnerUserId
// WHERE ur.UserRank <= 100 ORDER BY ur.UserRank;
//
// The tag subquery is one row per (owner, tag element) over all the user's posts; TagCount is never read.
fn q2227(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, tags_str, .. } = &db.post;
    let ps = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).group_by(owner_user).select(score).fold((0i64, 0i64), |(s, n), x| (s + x, n + 1));
    let rep = |u: Id<User>| db.user.reputation.get(u).unwrap();
    let r = ranked(drain(&ps), |&(u, (s, _))| (Reverse(rep(u)), Reverse(s)), false);
    let ur = rel(r.into_iter().take_while(|x| x.1 <= 100).map(|((u, a), k)| (u, a, k)).collect());
    type R = (Id<User>, (i64, i64), i64);
    let ut: MatSet<(Id<User>, Str)> = (&ur).map(|x: R| x.0).select(Ident::<User>::new().and(posts_of(db).select(tags_str.flat_map(|s: Str| s.split("><"))))).collect();
    let uti: HashIdx<Id<User>, Str> = (&ut).map(|x: (Id<User>, Str)| x.0).inv().select((&ut).map(|x: (Id<User>, Str)| x.1)).collect();
    let v = drain((&ur).select(Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0).select(&uti).opt())));
    rows(v.into_iter().map(|(_, ((u, (s, n), k), t))| {
        let mut f = ucols(db, u, &["uid", "name", "rep", "uviews"]);
        f.extend([V::I(s), V::I(n), V::I(k), V::S(t.unwrap_or("No Tags"))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate ASC) AS Rank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.AnswerCount FROM RankedPosts rp WHERE rp.Rank <= 5),
// CommentsPerPost AS (SELECT c.PostId, COUNT(*) AS CommentCount, SUM(c.Score) AS TotalScore FROM Comments c GROUP BY c.PostId),
// PostsWithComments AS (SELECT tp.PostId, tp.Title, COALESCE(c.CommentCount, 0) AS CommentCount, COALESCE(c.TotalScore, 0) AS TotalCommentScore, CASE WHEN tp.Score IS NULL THEN -1 ELSE tp.Score END AS PostScore
//     FROM TopPosts tp LEFT JOIN CommentsPerPost c ON tp.PostId = c.PostId),
// UserBadges AS (SELECT b.UserId, STRING_AGG(b.Name, ', ') AS Badges, COUNT(b.Id) AS BadgeCount FROM Badges b GROUP BY b.UserId)
// SELECT up.Id AS UserId, up.DisplayName, p.Title, p.PostScore, p.CommentCount, p.TotalCommentScore, ub.Badges
// FROM Users up LEFT JOIN PostsWithComments p ON up.Id = p.PostId LEFT JOIN UserBadges ub ON up.Id = ub.UserId
// WHERE p.PostScore > 0 AND (ub.BadgeCount IS NULL OR ub.BadgeCount > 1) ORDER BY p.PostScore DESC, p.CommentCount DESC LIMIT 10;
//
// The user's Id is joined to the post's Id, as written; the WHERE on p makes the LEFT JOIN an inner one.
fn q20596(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, origid, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let tp = set_of(top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), creation_date.get(p).unwrap(), p), 5, false).into_iter().map(|x| x.0).collect());
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db).select(&db.comment.score)).fold((0i64, 0i64), |(n, s), x| (n + 1, s + x));
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.name).buf_fold(|v| -> (i64, Str) { (v.len() as i64, Box::leak(v.to_vec().join(", ").into_boxed_str())) });
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    type J = (Id<User>, Option<(i64, Str)>);
    let v = drain(
        (&tp)
            .with(score.gt(0))
            .select((&cc).opt().and(origid.select(&uidx).select(Ident::<User>::new().and((&ub).opt())).filt(|(_, b): J| b.map_or(true, |b| b.0 > 1)))),
    );
    let v = top_n(v, |&(p, (c, _))| (Reverse(score.get(p).unwrap()), Reverse(c.map_or(0, |c| c.0)), p), 10);
    rows(v.into_iter().map(|(p, (c, (u, b)))| {
        let c = c.unwrap_or((0, 0));
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(post_fields(db, p, &["title", "score"]));
        f.extend([V::I(c.0), V::I(c.1), ostr(b.map(|b| b.1))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, RANK() OVER (ORDER BY p.Score DESC) AS ScoreRank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year') AND p.PostTypeId IN (1, 2) GROUP BY p.Id, p.Title, p.CreationDate, p.Score, u.DisplayName),
// TopPosts AS (SELECT PostId, Title, OwnerDisplayName, Score, UpVotes, DownVotes, CommentCount FROM RankedPosts WHERE ScoreRank <= 10)
// SELECT tp.Title, tp.OwnerDisplayName, tp.Score, tp.UpVotes, tp.DownVotes, tp.CommentCount, COALESCE(AVG(b.Class), 0) AS AverageBadgeClass,
//        COALESCE(STRING_AGG(DISTINCT REPLACE(t.TagName, '<', '&lt;'), ', '), '') AS AssociatedTags
// FROM TopPosts tp LEFT JOIN Badges b ON b.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = tp.PostId) LEFT JOIN Posts p ON p.Id = tp.PostId
// LEFT JOIN (SELECT pt.Id AS PostId, t.TagName FROM Posts pt JOIN Tags t ON t.ExcerptPostId = pt.Id) AS t ON t.PostId = tp.PostId
// GROUP BY tp.PostId, tp.Title, tp.OwnerDisplayName, tp.Score, tp.UpVotes, tp.DownVotes, tp.CommentCount ORDER BY tp.Score DESC;
//
// The rank reads only Score, so the posts are ranked first. `LEFT JOIN Posts p` is on its key and multiplies nothing.
fn q9382(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(post_type_id.is_in([1, 2]))).with(owner_user).select(score));
    let tp = set_of(ranked(v, |&(_, s)| Reverse(s), false).into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect());
    let pa = (&tp)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let ex: HashIdx<Id<Post>, Id<Tag>> = (&db.tag.excerpt_post).inv().collect();
    let bt = (&tp)
        .group_by(Ident::<Post>::new())
        .select(owner_user.select(badges_of(db)).select(&db.badge.class).opt().and((&ex).select(&db.tag.tag_name).opt()))
        .buf_fold(|v| {
            let (s, n) = v.iter().fold((0i64, 0i64), |(s, n), x| (s + x.0.unwrap_or(0), n + x.0.is_some() as i64));
            let t: Vec<Str> = v.iter().flat_map(|x| x.1).map(|t| -> Str { Box::leak(t.replace('<', "&lt;").into_boxed_str()) }).collect();
            (s, n, distinct_join(&t))
        });
    let v = drain((&pa).and(&bt));
    rows(v.into_iter().map(|(p, (a, (s, n, t)))| {
        let mut f = post_fields(db, p, &["title", "owner", "score"]);
        f.extend([V::I(a[1]), V::I(a[2]), V::I(a[0]), V::F(if n == 0 { 0.0 } else { s as f64 / n as f64 }), V::S(t)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate > (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year')),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, CASE WHEN u.Reputation IS NULL THEN 0 ELSE u.Reputation END AS SafeReputation FROM Users u),
// VoteDetail AS (SELECT v.PostId, SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 WHEN vt.Name = 'DownMod' THEN -1 ELSE 0 END) AS NetVotes FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY v.PostId),
// ClosedPostReasons AS (SELECT ph.PostId, STRING_AGG(cr.Name, ', ') AS CloseReasons FROM PostHistory ph JOIN CloseReasonTypes cr ON CAST(ph.Comment AS INT) = cr.Id WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.ViewCount, rp.CreationDate, COALESCE(ud.SafeReputation, 0) AS UserReputation, COALESCE(vd.NetVotes, 0) AS NetVotes, cr.CloseReasons
// FROM RankedPosts rp LEFT JOIN Users u ON rp.PostId = u.Id LEFT JOIN UserReputation ud ON u.Id = ud.UserId LEFT JOIN VoteDetail vd ON rp.PostId = vd.PostId
// LEFT JOIN ClosedPostReasons cr ON rp.PostId = cr.PostId WHERE rp.Rank <= 10 AND (rp.ViewCount > 100 OR cr.CloseReasons IS NOT NULL) ORDER BY rp.ViewCount DESC, rp.CreationDate DESC;
//
// The post's Id is joined to the user's Id, as written.
fn q23319(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, view_count, origid, .. } = &db.post;
    let v = drain(db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(post_type_id));
    let tp = set_of(top_per(v, |&(_, t)| t, |&(p, _)| { let w = view_count.get(p); (Reverse(score.get(p).unwrap()), w.is_none(), Reverse(w), p) }, 10, false).into_iter().map(|x| x.0).collect());
    let vd = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(vtype_name(db))).fold(0i64, |n, t| n + (t == "UpMod") as i64 - (t == "DownMod") as i64);
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let cr = db
        .post_history
        .with(post_history_type_id.eq(10))
        .group_by(post)
        .select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason))
        .buf_fold(|v| -> Str { Box::leak(v.to_vec().join(", ").into_boxed_str()) });
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    type J = (((Option<i64>, Option<Id<User>>), Option<i64>), Option<Str>);
    let v = drain((&tp).select(view_count.opt().and(origid.select(&uidx).opt()).and((&vd).opt()).and((&cr).opt())).filt(|(((w, _), _), c): J| w.map_or(false, |w| w > 100) || c.is_some()));
    rows(v.into_iter().map(|(p, (((_, u), n), c))| {
        let mut f = post_fields(db, p, &["id", "title", "views", "created"]);
        f.extend([V::I(u.map_or(0, |u| db.user.reputation.get(u).unwrap())), V::I(n.unwrap_or(0)), ostr(c)]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.CreationDate, DENSE_RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U WHERE U.Reputation > 0),
// RecentPosts AS (SELECT P.Id AS PostId, P.PostTypeId, P.Title, P.Score, P.CreationDate, P.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS RecentPostRank
//     FROM Posts P WHERE P.CreationDate > cast('2024-10-01' as date) - INTERVAL '1 year'),
// PostVoteCounts AS (SELECT P.Id AS PostId, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount, COUNT(V.Id) AS TotalVotes
//     FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY P.Id),
// CloseReasons AS (SELECT PH.PostId, STRING_AGG(DISTINCT CRT.Name, ', ') AS ReasonNames FROM PostHistory PH JOIN CloseReasonTypes CRT ON PH.Comment::int = CRT.Id WHERE PH.PostHistoryTypeId = 10 GROUP BY PH.PostId)
// SELECT U.DisplayName, U.Reputation, R.Title, R.Score, R.CreationDate AS PostCreationDate, COALESCE(V.UpVoteCount, 0) AS UpVoteCount, COALESCE(V.DownVoteCount, 0) AS DownVoteCount,
//        COALESCE(CR.ReasonNames, 'Not Closed') AS CloseReasons, U.ReputationRank
// FROM UserReputation U LEFT JOIN RecentPosts R ON U.UserId = R.OwnerUserId AND R.RecentPostRank = 1 LEFT JOIN PostVoteCounts V ON R.PostId = V.PostId LEFT JOIN CloseReasons CR ON R.PostId = CR.PostId
// WHERE U.ReputationRank <= 10 ORDER BY U.Reputation DESC, R.CreationDate DESC OFFSET 5 LIMIT 10;
//
// Partitioned by the owner edge: only posts whose owner exists meet a user.
fn q21643(db: &'static So) -> String {
    let Post { creation_date, owner_user, .. } = &db.post;
    let r = ranked(drain(db.user.with((&db.user.reputation).gt(0)).select(&db.user.reputation)), |&(_, r)| Reverse(r), true);
    let tu = rel(r.into_iter().take_while(|x| x.1 <= 10).map(|((u, _), k)| (u, k)).collect());
    let v = drain(db.post.with(creation_date.gt(add_years(date(2024, 10, 1), -1))).select(owner_user));
    let first = rel(top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false));
    let fi: HashIdx<Id<User>, Id<Post>> = (&first).map(|x: (Id<Post>, Id<User>)| x.1).inv().select((&first).map(|x: (Id<Post>, Id<User>)| x.0)).collect();
    let vc = (&first).map(|x: (Id<Post>, Id<User>)| x.0).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let cr = db
        .post_history
        .with(post_history_type_id.eq(10))
        .group_by(post)
        .select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason))
        .buf_fold(|v| distinct_join(&v));
    type R = (Id<User>, i64);
    let v = drain((&tu).select(Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0).select(&fi).select(Ident::<Post>::new().and((&vc).opt()).and((&cr).opt())).opt())));
    let v = top_n(v, |&(_, ((u, _), p))| {
        let d = p.map(|x| creation_date.get(x.0 .0).unwrap());
        (Reverse(db.user.reputation.get(u).unwrap()), d.is_none(), Reverse(d), u)
    }, 15);
    rows(v.into_iter().skip(5).map(|(_, ((u, k), p))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        match p {
            Some(((p, a), c)) => {
                let a = a.unwrap_or([0, 0]);
                f.extend(post_fields(db, p, &["title", "score", "created"]));
                f.extend([V::I(a[0]), V::I(a[1]), V::S(c.unwrap_or("Not Closed"))]);
            }
            None => f.extend([V::Null, V::Null, V::Null, V::I(0), V::I(0), V::S("Not Closed")]),
        }
        f.push(V::I(k));
        row(f)
    }))
}

// WITH UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, MAX(b.Class) AS MaxBadgeClass, STRING_AGG(b.Name, ', ') AS BadgeNames FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id),
// PopularPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
//        ROW_NUMBER() OVER (ORDER BY p.ViewCount DESC) AS PopularityRank FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.ViewCount),
// RecentPostHistory AS (SELECT h.PostId, h.UserId, ph.Name AS HistoryType, h.CreationDate, ROW_NUMBER() OVER (PARTITION BY h.PostId ORDER BY h.CreationDate DESC) AS RecentHistoryRank
//     FROM PostHistory h JOIN PostHistoryTypes ph ON h.PostHistoryTypeId = ph.Id WHERE h.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year')
// SELECT u.DisplayName, u.Location, ub.BadgeCount, ub.MaxBadgeClass, pp.Title AS PopularPostTitle, pp.ViewCount, pp.UpVotes, pp.DownVotes, rp.HistoryType AS RecentActionType, rp.CreationDate AS RecentActionDate
// FROM Users u LEFT JOIN UserBadges ub ON u.Id = ub.UserId LEFT JOIN PopularPosts pp ON pp.PopularityRank = 1 LEFT JOIN RecentPostHistory rp ON u.Id = rp.UserId AND rp.RecentHistoryRank = 1
// WHERE (ub.BadgeCount IS NULL OR ub.MaxBadgeClass = 1) AND (u.Location IS NOT NULL OR (u.AboutMe IS NOT NULL AND LENGTH(u.AboutMe) > 100)) AND pp.ViewCount >= 1000
// ORDER BY u.Reputation DESC LIMIT 10;
//
// `ON pp.PopularityRank = 1` names only pp, so it is a cross join with the single top post. The rank reads only ViewCount, so it is taken first.
// ub.BadgeCount is a COUNT over a LEFT JOIN, never NULL, so only MaxBadgeClass = 1 passes.
fn q22458(db: &'static So) -> String {
    let Post { post_type_id, view_count, .. } = &db.post;
    let top = top_n(drain(db.post.with(post_type_id.eq(1)).select(view_count.opt())), |&(p, w)| (w.is_none(), Reverse(w), p), 1);
    let pp = set_of(top.into_iter().map(|x| x.0).collect());
    let ppv = (&pp).with(view_count.ge(1000)).group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let ppr = rel(drain(&ppv));
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold((0i64, i64::MIN), |(n, m), c| match c {
        Some(c) => (n + 1, m.max(c)),
        None => (n, m),
    });
    let PostHistory { user, creation_date: hd, .. } = &db.post_history;
    let hv = drain(db.post_history.with(hd.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(&db.post_history.post));
    let h1 = rel(top_per(hv, |&(_, p)| p, |&(h, _)| (Reverse(hd.get(h).unwrap()), h), 1, false).into_iter().map(|x| x.0).collect());
    let hu: HashIdx<Id<User>, Id<PostHistory>> = (&h1).select(user).inv().select(&h1).collect();
    type U = (Id<User>, (i64, i64));
    let User { location, about_me, .. } = &db.user;
    let us: Vec<U> = drain(db.user.with(location.or(about_me.filt(|s: Str| s.chars().count() > 100))).select(Ident::<User>::new().and(&ub)).filt(|(_, (_, m)): U| m == 1)).into_iter().map(|x| x.1).collect();
    let us = rel(us);
    type Y = (U, (Id<Post>, [i64; 2]));
    let v = drain((&us).cross(&ppr).select(Same::<Y>::new().and(Same::<Y>::new().map(|y: Y| y.0 .0).select(&hu).opt())));
    let v = top_n(v, |&(_, (((u, _), _), h))| (Reverse(db.user.reputation.get(u).unwrap()), u, h), 10);
    rows(v.into_iter().map(|(_, (((u, (n, m)), (p, a)), h))| {
        let mut f = ucols(db, u, &["name"]);
        f.extend([ostr(db.user.location.get(u)), V::I(n), if n == 0 { V::Null } else { V::I(m) }]);
        f.extend(post_fields(db, p, &["title", "views"]));
        f.extend([V::I(a[0]), V::I(a[1])]);
        f.extend(match h {
            Some(h) => [V::S(htype_name(db).get(h).unwrap()), V::T(hd.get(h).unwrap())],
            None => [V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH RECURSIVE UserPosts AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers, SUM(CASE WHEN p.Score > 0 THEN p.Score ELSE 0 END) AS TotalScore
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// PostStats AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, COALESCE(uh.TotalPosts, 0) AS UserTotalPosts, COALESCE(uh.TotalQuestions, 0) AS UserTotalQuestions,
//        COALESCE(uh.TotalAnswers, 0) AS UserTotalAnswers, COALESCE(uh.TotalScore, 0) AS UserTotalScore
//     FROM Posts p LEFT JOIN UserPosts uh ON p.OwnerUserId = uh.UserId WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'),
// ClosedPosts AS (SELECT ph.PostId, ph.CreationDate, STRING_AGG(DISTINCT cr.Name, ', ') AS ClosureReasons FROM PostHistory ph JOIN CloseReasonTypes cr ON CAST(ph.Comment AS INT) = cr.Id
//     WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId, ph.CreationDate),
// RankedPosts AS (SELECT ps.*, RANK() OVER (ORDER BY ps.Score DESC, ps.ViewCount DESC) AS PostRank FROM PostStats ps)
// SELECT rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.UserTotalPosts, rp.UserTotalQuestions, rp.UserTotalAnswers, rp.UserTotalScore, cp.ClosureReasons
// FROM RankedPosts rp LEFT JOIN ClosedPosts cp ON rp.PostId = cp.PostId WHERE rp.PostRank <= 10 ORDER BY rp.Score DESC, rp.ViewCount DESC;
//
// No CTE refers to itself, so the RECURSIVE does nothing. The rank reads only base columns, so the posts are ranked first.
fn q31287(db: &'static So) -> String {
    let Post { creation_date, score, view_count, owner_user, post_type_id, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(score.and(view_count.opt())));
    let tp = set_of(ranked(v, |&(_, (s, w))| (Reverse(s), w.is_none(), Reverse(w)), false).into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect());
    let owners: MatSet<Id<User>> = (&tp).select(owner_user).collect();
    let up = (&owners).group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(score))).fold([0i64; 4], |a, (t, s)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s.max(0)]);
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post, post_history_type_id, comment, creation_date: hd, .. } = &db.post_history;
    let cp = db
        .post_history
        .with(post_history_type_id.eq(10))
        .group_by(post.and(hd))
        .select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason))
        .buf_fold(|v| distinct_join(&v));
    let cpr = rel(drain(&cp));
    let cpi: HashIdx<Id<Post>, Str> = (&cpr).map(|x: ((Id<Post>, i64), Str)| x.0 .0).inv().select((&cpr).map(|x: ((Id<Post>, i64), Str)| x.1)).collect();
    let v = drain((&tp).select(owner_user.select(&up).opt().and((&cpi).opt())));
    rows(v.into_iter().map(|(p, (u, c))| {
        let u = u.unwrap_or([0; 4]);
        let mut f = post_fields(db, p, &["title", "created", "score", "views"]);
        f.extend(u.map(V::I));
        f.push(ostr(c));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.ViewCount, p.AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.CreationDate DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate >= CURRENT_DATE - INTERVAL '1 year'),
// CloseReasons AS (SELECT ph.PostId, STRING_AGG(cr.Name, ', ') AS CloseReasonNames FROM PostHistory ph JOIN CloseReasonTypes cr ON CAST(ph.Comment AS INT) = cr.Id WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
//        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, TotalPosts, TotalQuestions, TotalAnswers, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS UserRank FROM UserStats WHERE TotalPosts > 0)
// SELECT rp.PostId, rp.Title, rp.Score, rp.ViewCount, rp.AnswerCount, COALESCE(cr.CloseReasonNames, 'No Close Reasons') AS CloseReasonNames, tu.DisplayName AS TopUser, tu.Reputation, tu.TotalPosts
// FROM RankedPosts rp LEFT JOIN CloseReasons cr ON rp.PostId = cr.PostId LEFT JOIN TopUsers tu ON rp.Rank = 1 WHERE rp.Rank <= 5 ORDER BY rp.Score DESC;
//
// `ON rp.Rank = 1` names only rp: the rank-1 rows are crossed with every TopUsers row (or kept with NULLs if there is none), the others get NULLs.
fn q31504(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(current_date(), -1))).select(post_type_id));
    let r = per_group(ranked(v, |&(p, t)| (t, Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), false), |x| x.1);
    let rp = rel(r.into_iter().filter(|x| x.1 <= 5).map(|((p, _), k)| (p, k)).collect());
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let cr = db
        .post_history
        .with(post_history_type_id.eq(10))
        .group_by(post)
        .select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason))
        .buf_fold(|v| -> Str { Box::leak(v.to_vec().join(", ").into_boxed_str()) });
    let tu = left_all(drain(user_distinct_posts(db).filt(|n| n > 0)));
    type R = (Id<Post>, i64);
    let pc = || Same::<R>::new().map(|x: R| x.0).select(&cr).opt();
    type U = Option<(Id<User>, i64)>;
    let on = rel(drain(&tu).into_iter().map(|(_, u)| (true, u)).chain([(false, None)]).collect::<Vec<(bool, U)>>());
    let on_idx: HashIdx<bool, (bool, U)> = (&on).map(|(b, _)| b).inv().select(&on).collect();
    let out = drain((&rp).select(Same::<R>::new().and(pc()).and(Same::<R>::new().map(|x: R| x.1 == 1).select(&on_idx))));
    rows(out.into_iter().map(|(_, (((p, _), c), (_, u)))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "answers"]);
        f.push(V::S(c.unwrap_or("No Close Reasons")));
        f.extend(match u {
            Some((u, n)) => [user_col(db, u, "name"), user_col(db, u, "rep"), V::I(n)],
            None => [V::Null, V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH TagUsage AS (SELECT unnest(string_to_array(substring(Tags, 2, length(Tags) - 2), '><')) AS Tag, Id AS PostId FROM Posts WHERE PostTypeId = 1),
// TagStats AS (SELECT Tag, COUNT(DISTINCT PostId) AS TagCount FROM TagUsage GROUP BY Tag),
// TopTags AS (SELECT Tag, TagCount, ROW_NUMBER() OVER (ORDER BY TagCount DESC) AS Rank FROM TagStats),
// PostInteraction AS (SELECT p.Id AS PostId, COUNT(c.Id) AS CommentCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpvoteCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownvoteCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 6 THEN 1 ELSE 0 END), 0) AS CloseVoteCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId = 1 GROUP BY p.Id),
// FinalStats AS (SELECT tt.Tag, pi.PostId, p.Title, pi.CommentCount, pi.UpvoteCount, pi.DownvoteCount, pi.CloseVoteCount
//     FROM PostInteraction pi JOIN Posts p ON pi.PostId = p.Id JOIN TopTags tt ON tt.Rank <= 10 WHERE tt.Tag = ANY(string_to_array(substring(p.Tags, 2, length(p.Tags) - 2), '><')))
// SELECT fs.Tag, COUNT(fs.PostId) AS PostCount, SUM(fs.CommentCount) AS TotalComments, SUM(fs.UpvoteCount) AS TotalUpvotes, SUM(fs.DownvoteCount) AS TotalDownvotes, SUM(fs.CloseVoteCount) AS TotalCloseVotes
// FROM FinalStats fs GROUP BY fs.Tag ORDER BY PostCount DESC, TotalUpvotes DESC;
fn q29167(db: &'static So) -> String {
    let Post { post_type_id, tags_str, .. } = &db.post;
    let qs = || db.post.with(post_type_id.eq(1));
    let pt: MatSet<(Id<Post>, Str)> = qs().select(Ident::<Post>::new().and(tags_str.flat_map(tag_list))).collect();
    let tc = (&pt).group_by(Same::<(Id<Post>, Str)>::new().map(|x: (Id<Post>, Str)| x.1)).select(Same::<(Id<Post>, Str)>::new()).fold(0i64, |n, _| n + 1);
    let tt = set_of(top_n(drain(&tc), |&(t, n)| (Reverse(n), t), 10).into_iter().map(|x| x.0).collect());
    let fp: MatSet<(Id<Post>, Str)> = (&pt).with(Same::<(Id<Post>, Str)>::new().map(|x: (Id<Post>, Str)| x.1).select(&tt)).collect();
    let fq: MatSet<Id<Post>> = (&fp).map(|x: (Id<Post>, Str)| x.0).collect();
    let pi = (&fq)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 4], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64, a[3] + (t == Some(6)) as i64]);
    type K = (Id<Post>, Str);
    let g = (&fp).group_by(Same::<K>::new().map(|x: K| x.1)).select(Same::<K>::new().map(|x: K| x.0).select(&pi)).fold([0i64; 5], |a, b| [a[0] + 1, a[1] + b[0], a[2] + b[1], a[3] + b[2], a[4] + b[3]]);
    rows(drain(&g).into_iter().map(|(t, a)| {
        let mut f = vec![V::S(t)];
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.OwnerUserId, U.DisplayName AS OwnerDisplayName, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p JOIN Users U ON p.OwnerUserId = U.Id WHERE p.PostTypeId = 1),
// UserBadges AS (SELECT b.UserId, COUNT(*) AS BadgeCount, STRING_AGG(b.Name, ', ') AS BadgeNames FROM Badges b GROUP BY b.UserId),
// ClosedQuestions AS (SELECT ph.PostId, ph.CreationDate AS ClosedDate, ph.Comment AS CloseReason FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10),
// RecentPosts AS (SELECT p.Id, p.Title, p.ViewCount, COALESCE(rp.BadgeCount, 0) AS BadgeCount, COALESCE(cl.ClosedDate, NULL) AS LastClosedDate, COALESCE(cl.CloseReason, 'Not Closed') AS LastCloseReason
//     FROM Posts p LEFT JOIN UserBadges rp ON p.OwnerUserId = rp.UserId LEFT JOIN ClosedQuestions cl ON p.Id = cl.PostId WHERE p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days')
// SELECT rp.Id, rp.Title, rp.ViewCount, rp.BadgeCount, rp.LastClosedDate, rp.LastCloseReason, CASE WHEN rp.BadgeCount > 0 THEN 'Has Badges' ELSE 'No Badges' END AS BadgeStatus,
//        CASE WHEN rp.LastClosedDate IS NOT NULL THEN 'Closed' ELSE 'Open' END AS PostStatus
// FROM RecentPosts rp WHERE rp.ViewCount > (SELECT AVG(ViewCount) FROM Posts) ORDER BY rp.ViewCount DESC FETCH FIRST 10 ROWS ONLY;
//
// RankedPosts is never read.
fn q30249(db: &'static So) -> String {
    let Post { creation_date, view_count, owner_user_id, .. } = &db.post;
    let (sum, n) = db.post.select(view_count).fold_flat((0i128, 0i128), |(s, n), x| (s + x as i128, n + 1));
    let ub = db.badge.group_by(&db.badge.user_id).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let cl = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10)));
    let v = drain(
        db.post
            .with(creation_date.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30)).and(view_count.filt(move |w| w as i128 * n > sum)))
            .select(owner_user_id.select(&ub).opt().and(cl.opt())),
    );
    let v = top_n(v, |&(p, (_, h))| (Reverse(view_count.get(p)), p, h), 10);
    rows(v.into_iter().map(|(p, (b, h))| {
        let b = b.unwrap_or(0);
        let mut f = post_fields(db, p, &["id", "title", "views"]);
        f.extend([V::I(b), ots(h.map(|h| db.post_history.creation_date.get(h).unwrap())), V::S(h.and_then(|h| db.post_history.comment.get(h)).unwrap_or("Not Closed"))]);
        f.extend([V::S(if b > 0 { "Has Badges" } else { "No Badges" }), V::S(if h.is_some() { "Closed" } else { "Open" })]);
        row(f)
    }))
}

// WITH TagStatistics AS (SELECT t.TagName, COUNT(DISTINCT p.Id) AS PostCount, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews, AVG(COALESCE(p.Score, 0)) AS AverageScore,
//        COUNT(DISTINCT c.Id) AS CommentCount, COUNT(DISTINCT b.Id) AS BadgeCount
//     FROM Tags t LEFT JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%' LEFT JOIN Comments c ON c.PostId = p.Id LEFT JOIN Badges b ON b.UserId = p.OwnerUserId
//     WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year' GROUP BY t.TagName),
// ClosedPosts AS (SELECT ph.PostId, STRING_AGG(DISTINCT ctr.Name, ', ') AS CloseReasons, COUNT(*) AS CloseEventCount FROM PostHistory ph JOIN CloseReasonTypes ctr ON ph.PostHistoryTypeId IN (10, 11)
//     WHERE ph.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year' GROUP BY ph.PostId),
// AggregateData AS (SELECT ts.TagName, ts.PostCount, ts.TotalViews, ts.AverageScore, ts.CommentCount, ts.BadgeCount, COUNT(cp.PostId) AS ClosedCount, SUM(cp.CloseEventCount) AS TotalCloseEvents,
//        CASE WHEN COUNT(cp.PostId) > 0 THEN 'Yes' ELSE 'No' END AS HasClosedPosts
//     FROM TagStatistics ts LEFT JOIN ClosedPosts cp ON ts.PostCount > 0 GROUP BY ts.TagName, ts.PostCount, ts.TotalViews, ts.AverageScore, ts.CommentCount, ts.BadgeCount)
// SELECT *, RANK() OVER (ORDER BY TotalViews DESC) AS RankByViews, RANK() OVER (ORDER BY AverageScore DESC) AS RankByScore FROM AggregateData ORDER BY TotalViews DESC, AverageScore DESC;
//
// Both `ON`s that name one side are cross joins: every close event with every close reason type, and every tag with every closed post.
fn q27623(db: &'static So) -> String {
    let Post { creation_date, view_count, score, owner_user, .. } = &db.post;
    let lt = tag_mentions(db);
    let by_tag: HashIdx<Id<Tag>, Id<Post>> = (&lt).map(|x: (Id<Post>, Id<Tag>)| x.1).inv().select((&lt).map(|x: (Id<Post>, Id<Tag>)| x.0)).collect();
    let since = date(2023, 10, 1);
    let tp = || (&by_tag).select(Ident::<Post>::new().with(creation_date.ge(since)));
    let sums = db
        .tag
        .group_by(Ident::<Tag>::new())
        .select(tp().select(view_count.opt().and(score).and(comments_of(db).opt()).and(owner_user.select(badges_of(db)).opt())))
        .fold([0i64; 3], |a, (((w, s), _), _)| [a[0] + w.unwrap_or(0), a[1] + s, a[2] + 1]);
    let pc = db.tag.group_by(Ident::<Tag>::new()).select(tp()).count_distinct();
    let cc = db.tag.group_by(Ident::<Tag>::new()).select(tp().select(comments_of(db))).count_distinct();
    let bc = db.tag.group_by(Ident::<Tag>::new()).select(tp().select(owner_user).select(badges_of(db))).count_distinct();
    let PostHistory { post, post_history_type_id, creation_date: hd, .. } = &db.post_history;
    let ph: MatSet<Id<PostHistory>> = db.post_history.with(post_history_type_id.is_in([10, 11]).and(hd.ge(since))).collect();
    type C = (Id<PostHistory>, Str);
    let cp = (&ph).cross(&db.close_reason_type.name).map(|x: C| x).group_by(Same::<C>::new().map(|x: C| x.0).select(post)).select(Same::<C>::new()).fold(0i64, |n, _| n + 1);
    let cpr = rel(drain(&cp));
    type T = (Id<Tag>, ((([i64; 3], i64), Option<i64>), Option<i64>));
    let tsr = rel(drain((&sums).and(&pc).and((&cc).opt()).and((&bc).opt())));
    let ad = (&tsr).filt(|x: T| x.1 .0 .0 .1 > 0).cross(&cpr).map(|x: (T, (Id<Post>, i64))| x).group_by(Same::<(T, (Id<Post>, i64))>::new().map(|x: (T, (Id<Post>, i64))| x.0 .0)).select(Same::<(T, (Id<Post>, i64))>::new().map(|x: (T, (Id<Post>, i64))| x.1 .1)).fold((0i64, 0i64), |(n, s), e| (n + 1, s + e));
    let v = drain((&tsr).select(Same::<T>::new().and(Same::<T>::new().map(|x: T| x.0).select(&ad).opt())));
    let mean = |a: [i64; 3]| a[1] as f64 / a[2] as f64;
    let r = ranked(v, |&(_, ((_, (((a, _), _), _)), _))| Reverse(a[0]), false);
    let r = ranked(r, |&((_, ((_, (((a, _), _), _)), _)), _)| Reverse(fkey(mean(a))), false);
    rows(r.into_iter().map(|(((_, ((t, (((a, p), c), b)), d)), rv), rs)| {
        let d = d.unwrap_or((0, 0));
        let mut f = vec![V::S(db.tag.tag_name.get(t).unwrap()), V::I(p), V::I(a[0]), V::F(mean(a)), V::I(c.unwrap_or(0)), V::I(b.unwrap_or(0))];
        f.extend([V::I(d.0), nullable(d.1, d.0), V::S(if d.0 > 0 { "Yes" } else { "No" }), V::I(rv), V::I(rs)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.OwnerUserId, p.Title, p.ViewCount, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.ViewCount DESC) AS RN,
//        COUNT(*) OVER (PARTITION BY p.OwnerUserId) AS PostCount FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, CASE WHEN u.Reputation < 100 THEN 'Newbie' WHEN u.Reputation BETWEEN 100 AND 1000 THEN 'Intermediate' ELSE 'Expert' END AS ReputationTier FROM Users u),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c GROUP BY c.PostId),
// ClosedPostReasons AS (SELECT ph.PostId, STRING_AGG(CASE WHEN ph.PostHistoryTypeId = 10 THEN cr.Name END, ', ') AS CloseReasons FROM PostHistory ph LEFT JOIN CloseReasonTypes cr ON ph.Comment::integer = cr.Id
//     WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.ViewCount, ur.Reputation, ur.ReputationTier, pc.CommentCount, cr.CloseReasons, COALESCE(NULLIF(rp.PostCount, 0), 1) AS NonZeroPostCount,
//        CASE WHEN cr.CloseReasons IS NOT NULL THEN 'Closed' ELSE 'Open' END AS PostStatus
// FROM RankedPosts rp JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId LEFT JOIN PostComments pc ON rp.PostId = pc.PostId LEFT JOIN ClosedPostReasons cr ON rp.PostId = cr.PostId
// WHERE rp.RN = 1 ORDER BY rp.ViewCount DESC NULLS LAST LIMIT 50;
//
// Partitioned by the owner edge: only posts whose owner exists meet a user.
fn q21567(db: &'static So) -> String {
    let Post { creation_date, owner_user, view_count, .. } = &db.post;
    let recent = || db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)));
    let v = drain(recent().select(owner_user));
    let first = set_of(top_per(v, |&(_, u)| u, |&(p, _)| { let w = view_count.get(p); (w.is_none(), Reverse(w), p) }, 1, false).into_iter().map(|x| x.0).collect());
    let npc = recent().group_by(owner_user).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let pc = (&first).group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let cr = db
        .post_history
        .with(post_history_type_id.is_in([10, 11]))
        .group_by(post)
        .select(post_history_type_id.and(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason).opt()))
        .buf_fold(|v| -> Option<Str> {
            let n: Vec<Str> = v.iter().filter(|x| x.0 == 10).flat_map(|x| x.1).collect();
            if n.is_empty() { None } else { Some(Box::leak(n.join(", ").into_boxed_str())) }
        });
    let v = drain((&first).select(owner_user.select(Ident::<User>::new().and(&npc)).and((&pc).opt()).and((&cr).opt())));
    let v = top_n(v, |&(p, _)| { let w = view_count.get(p); (w.is_none(), Reverse(w), p) }, 50);
    rows(v.into_iter().map(|(p, (((u, n), c), cr))| {
        let rep = db.user.reputation.get(u).unwrap();
        let cr = cr.flatten();
        let mut f = post_fields(db, p, &["id", "title", "views"]);
        f.extend([V::I(rep), V::S(if rep < 100 { "Newbie" } else if rep <= 1000 { "Intermediate" } else { "Expert" }), oint(c), ostr(cr), V::I(if n == 0 { 1 } else { n })]);
        f.push(V::S(if cr.is_some() { "Closed" } else { "Open" }));
        row(f)
    }))
}

// WITH RECURSIVE UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.CreationDate, U.LastAccessDate, COALESCE(P.AnswerCount, 0) AS TotalAnswers, COALESCE(P.CommentCount, 0) AS TotalComments,
//        COALESCE(P.ViewCount, 0) AS TotalViews, ROW_NUMBER() OVER (PARTITION BY U.Id ORDER BY U.CreationDate DESC) AS ActivityRank
//     FROM Users U LEFT JOIN (SELECT OwnerUserId, SUM(CASE WHEN PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(CASE WHEN PostTypeId = 1 THEN 1 ELSE 0 END) AS CommentCount, SUM(ViewCount) AS ViewCount
//     FROM Posts GROUP BY OwnerUserId) P ON U.Id = P.OwnerUserId),
// RecentPosts AS (SELECT Id, Title, CreationDate, LastEditDate, Score, OwnerUserId, ROW_NUMBER() OVER (ORDER BY CreationDate DESC) AS RecentPostRank FROM Posts
//     WHERE LastEditDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year')
// SELECT UA.DisplayName, UA.Reputation, UA.CreationDate AS AccountCreationDate, R.Title AS RecentPostTitle, R.CreationDate AS RecentPostDate, R.Score AS RecentPostScore,
//        (SELECT COUNT(*) FROM Comments C WHERE C.PostId = R.Id) AS TotalCommentsOnPost,
//        (SELECT STRING_AGG(DISTINCT T.TagName, ', ') FROM Posts P JOIN Tags T ON P.Tags LIKE '%' || T.TagName || '%' WHERE P.Id = R.Id) AS AssociatedTags
// FROM UserActivity UA LEFT JOIN RecentPosts R ON UA.UserId = R.OwnerUserId WHERE UA.ActivityRank <= 10 ORDER BY UA.Reputation DESC, R.RecentPostRank LIMIT 20;
//
// No CTE refers to itself, so the RECURSIVE does nothing. ActivityRank partitions by the user, so it is always 1; the post aggregates are never read.
fn q32254(db: &'static So) -> String {
    let Post { creation_date, last_edit_date, owner_user, tags_str, .. } = &db.post;
    let rp = ranked(drain(db.post.with(last_edit_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(creation_date)), |&(p, d)| (Reverse(d), p), false);
    let rpr = rel(rp.into_iter().map(|((p, _), k)| (p, k)).collect());
    type R = (Id<Post>, i64);
    let byo: HashIdx<Id<User>, R> = (&rpr).map(|x: R| x.0).select(owner_user).inv().select(&rpr).collect();
    let v = drain(db.user.select((&byo).opt()));
    let v = top_n(v, |&(u, r)| (Reverse(db.user.reputation.get(u).unwrap()), r.is_none(), r.map(|x| x.1), u), 20);
    let tp: MatSet<Id<Post>> = rel(v.iter().flat_map(|x| x.1).map(|x| x.0).collect()).map(|p| p).collect();
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let tg = (&tp).group_by(Ident::<Post>::new()).select(tags_str.select_where((&db.tag.tag_name).inv(), |s: Str, t: Str| like(s, &format!("%{t}%"))).select(&db.tag.tag_name)).buf_fold(|v| distinct_join(&v));
    let tr = rel(v);
    type X = (Id<User>, Option<R>);
    let v = drain((&tr).select(Same::<X>::new().and(Same::<X>::new().flat_map(|x: X| x.1.map(|r| r.0)).select((&cc).opt().and((&tg).opt())).opt())));
    rows(v.into_iter().map(|(_, ((u, r), a))| {
        let mut f = ucols(db, u, &["name", "rep", "ucreated"]);
        match r {
            Some((p, _)) => {
                let (c, t) = a.unwrap();
                f.extend(post_fields(db, p, &["title", "created", "score"]));
                f.extend([V::I(c.unwrap_or(0)), ostr(t)]);
            }
            None => f.extend([V::Null, V::Null, V::Null, V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, p.CreationDate, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS Rank,
//        COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount, CASE WHEN p.CreationDate < (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year') THEN 'Old Post' ELSE 'Recent Post' END AS PostAge
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId),
// ClosedPosts AS (SELECT ph.PostId, COUNT(*) AS CloseCount, STRING_AGG(cr.Name, ', ') AS CloseReasons FROM PostHistory ph JOIN CloseReasonTypes cr ON CAST(ph.Comment AS INTEGER) = cr.Id WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id)
// SELECT rp.PostId, rp.Title, rp.Score, rp.CreationDate,
//        CASE WHEN ub.BadgeCount IS NOT NULL THEN CONCAT('User has ', ub.BadgeCount, ' badges (', ub.GoldBadges, ' Gold, ', ub.SilverBadges, ' Silver, ', ub.BronzeBadges, ' Bronze)') ELSE 'User has no badges' END AS BadgeStatus,
//        COALESCE(cp.CloseCount, 0) AS ClosedPostCount, cp.CloseReasons, rp.PostAge
// FROM RankedPosts rp LEFT JOIN UserBadges ub ON rp.OwnerUserId = ub.UserId LEFT JOIN ClosedPosts cp ON rp.PostId = cp.PostId
// WHERE rp.Rank = 1 AND rp.Score > (SELECT AVG(Score) FROM Posts) ORDER BY rp.Score DESC, rp.CreationDate DESC LIMIT 10;
//
// Rank = 1 keeps one post x comment row per owner (the NULL owner too), the owner's best-scored post.
fn q22653(db: &'static So) -> String {
    let Post { creation_date, owner_user_id, owner_user, score, .. } = &db.post;
    let first = set_of(top_per(drain(db.post.select(owner_user_id.opt())), |&(_, u)| u, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 1, false).into_iter().map(|x| x.0).collect());
    let (sum, n) = db.post.select(score).fold_flat((0i128, 0i128), |(s, n), x| (s + x as i128, n + 1));
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let cp = db
        .post_history
        .with(post_history_type_id.eq(10))
        .group_by(post)
        .select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason))
        .buf_fold(|v| -> (i64, Str) { (v.len() as i64, Box::leak(v.to_vec().join(", ").into_boxed_str())) });
    let v = drain((&first).with(score.filt(move |s| s as i128 * n > sum)).select(owner_user.select(&ub).opt().and((&cp).opt())));
    let v = top_n(v, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p), 10);
    let old = add_years(ts(2024, 10, 1, 12, 34, 56), -1);
    rows(v.into_iter().map(|(p, (b, c))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "created"]);
        f.push(match b {
            Some(a) => V::Owned(format!("User has {} badges ({} Gold, {} Silver, {} Bronze)", a[0], a[1], a[2], a[3])),
            None => V::S("User has no badges"),
        });
        f.extend([V::I(c.map_or(0, |c| c.0)), ostr(c.map(|c| c.1)), V::S(if creation_date.get(p).unwrap() < old { "Old Post" } else { "Recent Post" })]);
        row(f)
    }))
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

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.PostTypeId, P.AcceptedAnswerId, P.Score, P.ViewCount, U.DisplayName AS OwnerDisplayName,
//        RANK() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC NULLS LAST) AS PostRank
//     FROM Posts P LEFT JOIN Users U ON P.OwnerUserId = U.Id WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND P.ViewCount > 0),
// ClosedPostHistory AS (SELECT PH.PostId, COUNT(*) AS CloseCount, STRING_AGG(CASE WHEN PH.Comment IS NOT NULL THEN PH.Comment ELSE 'No Comment' END, '; ') AS ClosingComments, MAX(PH.CreationDate) AS LastCloseDate
//     FROM PostHistory PH WHERE PH.PostHistoryTypeId IN (10, 11) GROUP BY PH.PostId),
// ValidCloseReasons AS (SELECT PH.PostId, STRING_AGG(CASE WHEN CR.Name IS NOT NULL THEN CR.Name ELSE 'Unknown Reason' END, ', ') AS CloseReasons
//     FROM PostHistory PH LEFT JOIN CloseReasonTypes CR ON CAST(PH.Comment AS INTEGER) = CR.Id WHERE PH.PostHistoryTypeId = 10 GROUP BY PH.PostId)
// SELECT RP.PostId, RP.Title, RP.CreationDate, RP.OwnerDisplayName, RP.Score AS PostScore, RP.ViewCount, RP.PostTypeId, COALESCE(CPH.CloseCount, 0) AS TotalClose,
//        COALESCE(VCR.CloseReasons, 'No reasons provided') AS ClosureReasons, RP.PostRank
// FROM RankedPosts RP LEFT JOIN ClosedPostHistory CPH ON RP.PostId = CPH.PostId LEFT JOIN ValidCloseReasons VCR ON RP.PostId = VCR.PostId
// WHERE (RP.PostTypeId = 1 AND RP.Score >= 10) OR (RP.PostTypeId = 2 AND RP.ViewCount >= 100) ORDER BY RP.CreationDate DESC LIMIT 100;
fn q23627(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)).and(view_count.gt(0))).select(post_type_id));
    let r = per_group(ranked(v, |&(p, t)| (t, Reverse(score.get(p).unwrap())), false), |x| x.1);
    type R = ((Id<Post>, i64), i64);
    let rp = rel(r);
    let cph = db.post_history.with((&db.post_history.post_history_type_id).is_in([10, 11])).group_by(&db.post_history.post).select(Ident::<PostHistory>::new()).fold(0i64, |n, _| n + 1);
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let vcr = db
        .post_history
        .with(post_history_type_id.eq(10))
        .group_by(post)
        .select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason).opt())
        .buf_fold(|v| -> Str { Box::leak(v.iter().map(|n| n.unwrap_or("Unknown Reason")).collect::<Vec<_>>().join(", ").into_boxed_str()) });
    let pr = || Same::<R>::new().map(|x: R| x.0 .0);
    let v = drain(
        (&rp)
            .filt(move |((p, t), _): R| (t == 1 && score.get(p).unwrap() >= 10) || (t == 2 && view_count.get(p).map_or(false, |w| w >= 100)))
            .select(Same::<R>::new().and(pr().select(&cph).opt()).and(pr().select(&vcr).opt())),
    );
    let v = top_n(v, |&(_, ((((p, _), _), _), _))| (Reverse(creation_date.get(p).unwrap()), p), 100);
    rows(v.into_iter().map(|(_, ((((p, _), k), n), c))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner", "score", "views", "type_id"]);
        f.extend([V::I(n.unwrap_or(0)), V::S(c.unwrap_or("No reasons provided")), V::I(k)]);
        row(f)
    }))
}

// WITH UserVoteSummary AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(V.Id) AS TotalVotes, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.DisplayName),
// PostSummary AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes, COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
//        COUNT(C.Id) AS CommentCount, ROW_NUMBER() OVER (PARTITION BY EXTRACT(YEAR FROM P.CreationDate) ORDER BY P.CreationDate) AS YearRank
//     FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Comments C ON P.Id = C.PostId GROUP BY P.Id, P.Title, P.CreationDate),
// ClosedPostReasons AS (SELECT PH.PostId, STRING_AGG(DISTINCT CRT.Name, ', ') AS CloseReasons FROM PostHistory PH JOIN CloseReasonTypes CRT ON CAST(PH.Comment AS INT) = CRT.Id WHERE PH.PostHistoryTypeId = 10 GROUP BY PH.PostId)
// SELECT U.DisplayName, U.TotalVotes, U.UpVotes, U.DownVotes, P.PostId, P.Title, P.CreationDate, P.UpVotes AS PostUpVotes, P.DownVotes AS PostDownVotes, P.CommentCount, P.YearRank, C.CloseReasons
// FROM UserVoteSummary U JOIN PostSummary P ON U.TotalVotes > 5 LEFT JOIN ClosedPostReasons C ON P.PostId = C.PostId
// WHERE (P.CommentCount > 10 OR P.YearRank = 1) AND P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' ORDER BY U.TotalVotes DESC, P.CreationDate DESC LIMIT 100;
//
// `ON U.TotalVotes > 5` names only U, so it is a cross join. YearRank reads only CreationDate, so it is ranked over all posts before the counts are taken for the recent ones.
fn q2215(db: &'static So) -> String {
    let Post { creation_date, .. } = &db.post;
    let yr = per_group(ranked(drain(db.post.select(creation_date)), |&(p, d)| (year(d), d, p), false), |x| year(x.1));
    let since = add_years(ts(2024, 10, 1, 12, 34, 56), -1);
    type R = ((Id<Post>, i64), i64);
    let yr = rel(yr);
    let ps = db
        .post
        .with(creation_date.ge(since))
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()))
        .fold([0i64; 3], |a, (t, c)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64]);
    let pv: Vec<(R, [i64; 3])> = drain((&yr).select(Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0 .0).select(&ps))).filt(|(((_, _), k), a): (R, [i64; 3])| a[2] > 10 || k == 1)).into_iter().map(|x| x.1).collect();
    let uv = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| [a[0] + t.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let us: Vec<(Id<User>, [i64; 3])> = drain((&uv).filt(|a| a[0] > 5));
    let v = cross_top(us, |&(u, a)| (Reverse(a[0]), u), pv, |&(((p, d), _), _)| (Reverse(d), p), 100);
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let cr = db
        .post_history
        .with(post_history_type_id.eq(10))
        .group_by(post)
        .select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason))
        .buf_fold(|v| distinct_join(&v));
    type X = ((Id<User>, [i64; 3]), (R, [i64; 3]));
    let v = drain(rel(v).select(Same::<X>::new().and(Same::<X>::new().map(|x: X| x.1 .0 .0 .0).select(&cr).opt())));
    rows(v.into_iter().map(|(_, (((u, a), (((p, _), k), b)), c))| {
        let mut f = ucols(db, u, &["name"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        f.extend(post_fields(db, p, &["id", "title", "created"]));
        f.extend([V::I(b[0]), V::I(b[1]), V::I(b[2]), V::I(k), ostr(c)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.CreationDate, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.ViewCount DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.ViewCount IS NOT NULL),
// RecentVotes AS (SELECT v.PostId, v.VoteTypeId, COUNT(v.Id) AS VoteCount FROM Votes v WHERE v.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' GROUP BY v.PostId, v.VoteTypeId),
// PostWithVotes AS (SELECT rp.PostId, rp.Title, rp.ViewCount, COALESCE(rv.VoteCount, 0) AS RecentVoteCount,
//        CASE WHEN rv.VoteTypeId = 2 THEN 'Upvote' WHEN rv.VoteTypeId = 3 THEN 'Downvote' ELSE 'No recent votes' END AS VoteType
//     FROM RankedPosts rp LEFT JOIN RecentVotes rv ON rp.PostId = rv.PostId WHERE rp.Rank <= 10),
// PostHistoryDetail AS (SELECT ph.PostId, STRING_AGG(CONCAT(pht.Name, ': ', ph.Comment), '; ') AS HistoryComments FROM PostHistory ph JOIN PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id
//     WHERE ph.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '6 months' GROUP BY ph.PostId)
// SELECT pw.PostId, pw.Title, pw.ViewCount, pw.RecentVoteCount, pw.VoteType, COALESCE(phd.HistoryComments, 'No history comments') AS PostHistory
// FROM PostWithVotes pw LEFT JOIN PostHistoryDetail phd ON pw.PostId = phd.PostId WHERE pw.RecentVoteCount > 0 OR pw.VoteType != 'No recent votes' ORDER BY pw.ViewCount DESC, pw.Title LIMIT 100;
//
// A row passes the WHERE exactly when it has a RecentVotes match, since VoteCount is then at least 1.
fn q20663(db: &'static So) -> String {
    let Post { creation_date, post_type_id, view_count, title, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let v = drain(db.post.with(creation_date.ge(add_years(t0, -1))).with(view_count).select(post_type_id));
    let tp = set_of(top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(view_count.get(p)), p), 10, false).into_iter().map(|x| x.0).collect());
    let Vote { post, vote_type_id, creation_date: vd, .. } = &db.vote;
    let rv = db.vote.with(vd.ge(add_days(t0, -30))).group_by(post.and(vote_type_id)).select(Ident::<Vote>::new()).fold(0i64, |n, _| n + 1);
    let rvr = rel(drain(&rv));
    type K = ((Id<Post>, i64), i64);
    let rvi: HashIdx<Id<Post>, K> = (&rvr).map(|x: K| x.0 .0).inv().select(&rvr).collect();
    let PostHistory { post: hp, comment, creation_date: hd, .. } = &db.post_history;
    let phd = db
        .post_history
        .with(hd.ge(add_months(t0, -6)))
        .group_by(hp)
        .select(htype_name(db).and(comment.opt()))
        .buf_fold(|v| -> Str { Box::leak(v.iter().map(|(n, c)| format!("{n}: {}", c.unwrap_or(""))).collect::<Vec<_>>().join("; ").into_boxed_str()) });
    let v = drain((&tp).select((&rvi).and((&phd).opt())));
    let v = top_n(v, |&(p, (k, _))| (Reverse(view_count.get(p)), title.get(p), p, k), 100);
    rows(v.into_iter().map(|(p, (((_, t), n), h))| {
        let mut f = post_fields(db, p, &["id", "title", "views"]);
        f.extend([V::I(n), V::S(if t == 2 { "Upvote" } else if t == 3 { "Downvote" } else { "No recent votes" }), V::S(h.unwrap_or("No history comments"))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Score, COALESCE(v.UpVotes, 0) AS UpVotes, COALESCE(v.DownVotes, 0) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.CreationDate DESC) AS Rank,
//        COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount
//     FROM Posts p LEFT JOIN (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes FROM Votes GROUP BY PostId) v ON p.Id = v.PostId
//     LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.Score IS NOT NULL AND p.CreationDate > (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year')),
// CloseReasons AS (SELECT ph.PostId, STRING_AGG(cr.Name, ', ') AS CloseReasonNames FROM PostHistory ph JOIN CloseReasonTypes cr ON CAST(ph.Comment AS INTEGER) = cr.Id WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.PostId),
// MostCommentedPosts AS (SELECT PostId, COUNT(*) AS TotalComments FROM Comments GROUP BY PostId HAVING COUNT(*) > 5)
// SELECT rp.PostId, rp.Title, rp.Score, rp.UpVotes, rp.DownVotes, COALESCE(cr.CloseReasonNames, 'No close reasons') AS CloseReasons, COALESCE(mcp.TotalComments, 0) AS CommentCount,
//        CASE WHEN rp.Rank = 1 THEN 'Most Recent' ELSE NULL END AS IsTopPost
// FROM RankedPosts rp LEFT JOIN CloseReasons cr ON rp.PostId = cr.PostId LEFT JOIN MostCommentedPosts mcp ON rp.PostId = mcp.PostId
// WHERE rp.UpVotes + 3 * rp.DownVotes > 10 ORDER BY rp.Score DESC, rp.UpVotes DESC;
//
// RankedPosts has one row per post x comment, and each is kept; Rank = 1 marks one row per post type.
fn q21655(db: &'static So) -> String {
    let Post { creation_date, post_type_id, .. } = &db.post;
    type J = (Id<Post>, Option<Id<Comment>>);
    let j: Vec<J> = drain(db.post.with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(Ident::<Post>::new().and(comments_of(db).opt()))).into_iter().map(|x| x.1).collect();
    let r1: MatSet<J> = rel(top_per(j.clone(), |&(p, _)| post_type_id.get(p).unwrap(), |&(p, c)| (Reverse(creation_date.get(p).unwrap()), p, c), 1, false)).map(|x| x).collect();
    let vc = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let cr = db
        .post_history
        .with(post_history_type_id.is_in([10, 11]))
        .group_by(post)
        .select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason))
        .buf_fold(|v| -> Str { Box::leak(v.to_vec().join(", ").into_boxed_str()) });
    let mcp = db.comment.group_by(&db.comment.post).select(Ident::<Comment>::new()).fold(0i64, |n, _| n + 1);
    let pr = || Same::<J>::new().map(|x: J| x.0);
    type X = ((((J, Option<[i64; 2]>), Option<Str>), Option<i64>), Option<J>);
    let v = drain(
        rel(j)
            .select(Same::<J>::new().and(pr().select(&vc).opt()).and(pr().select(&cr).opt()).and(pr().select((&mcp).filt(|n| n > 5)).opt()).and(Same::<J>::new().with(&r1).opt()))
            .filt(|((((_, a), _), _), _): X| a.map_or(false, |a| a[0] + 3 * a[1] > 10)),
    );
    rows(v.into_iter().map(|(_, (((((p, _), a), c), m), t))| {
        let a = a.unwrap();
        let mut f = post_fields(db, p, &["id", "title", "score"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::S(c.unwrap_or("No close reasons")), V::I(m.unwrap_or(0)), if t.is_some() { V::S("Most Recent") } else { V::Null }]);
        row(f)
    }))
}

// WITH User_Reputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U WHERE U.Reputation IS NOT NULL),
// Top_Posts AS (SELECT P.Id AS PostId, P.Title, P.OwnerUserId, P.CreationDate, P.AnswerCount, P.Score, ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS PostRank
//     FROM Posts P WHERE P.Score IS NOT NULL AND P.OwnerUserId IS NOT NULL),
// Post_Statistics AS (SELECT UP.UserId, UP.DisplayName, COUNT(DISTINCT TP.PostId) AS TotalPosts, SUM(COALESCE(TP.Score, 0)) AS TotalScore, AVG(COALESCE(TP.AnswerCount, 0)) AS AvgAnswerCount
//     FROM User_Reputation UP LEFT JOIN Top_Posts TP ON UP.UserId = TP.OwnerUserId GROUP BY UP.UserId, UP.DisplayName),
// Closed_Posts AS (SELECT H.PostId, COUNT(H.Id) AS CloseVoteCount, STRING_AGG(CASE WHEN H.Comment IS NOT NULL THEN H.Comment ELSE 'No comment' END, '; ') AS CloseReasons FROM PostHistory H WHERE H.PostHistoryTypeId = 10 GROUP BY H.PostId),
// Posts_With_Closed_Info AS (SELECT P.PostId, P.Title, P.OwnerUserId, COALESCE(CP.CloseVoteCount, 0) AS CloseVoteCount, COALESCE(CP.CloseReasons, 'No Close Reasons') AS CloseReasons
//     FROM Top_Posts P LEFT JOIN Closed_Posts CP ON P.PostId = CP.PostId)
// SELECT PS.UserId, PS.DisplayName, PS.TotalPosts, PS.TotalScore, PS.AvgAnswerCount, P.CloseVoteCount, P.CloseReasons
// FROM Post_Statistics PS JOIN Posts_With_Closed_Info P ON PS.UserId = P.OwnerUserId WHERE PS.TotalPosts > 2 AND PS.AvgAnswerCount > 1 AND P.CloseVoteCount > 0 ORDER BY PS.TotalScore DESC LIMIT 50;
fn q23710(db: &'static So) -> String {
    let Post { score, answer_count, owner_user, .. } = &db.post;
    let ps = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(score.and(answer_count.opt()))).fold([0i64; 3], |a, (s, n)| [a[0] + 1, a[1] + s, a[2] + n.unwrap_or(0)]);
    let PostHistory { post_history_type_id, comment, .. } = &db.post_history;
    let cp = db.post_history.with(post_history_type_id.eq(10)).group_by(&db.post_history.post).select(comment.opt()).buf_fold(|v| -> (i64, Str) {
        (v.len() as i64, Box::leak(v.iter().map(|c| c.unwrap_or("No comment")).collect::<Vec<_>>().join("; ").into_boxed_str()))
    });
    let v = drain(db.post.with(&cp).select(owner_user.select(Ident::<User>::new().and((&ps).filt(|a| a[0] > 2 && a[2] > a[0]))).and(&cp)));
    let v = top_n(v, |&(p, ((u, a), _))| (Reverse(a[1]), u, p), 50);
    rows(v.into_iter().map(|(_, ((u, a), (n, c)))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1]), avg(a[2], a[0]), V::I(n), V::S(c)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RecentPostRank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 YEAR' AND p.OwnerUserId IS NOT NULL),
// PostVoteSummary AS (SELECT v.PostId, COUNT(v.Id) FILTER (WHERE v.VoteTypeId IN (2, 5)) AS UpVotes, COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 3) AS DownVotes,
//        SUM(CASE WHEN v.VoteTypeId = 8 THEN v.BountyAmount ELSE 0 END) AS TotalBounty FROM Votes v GROUP BY v.PostId),
// ClosedPostReasons AS (SELECT ph.PostId, STRING_AGG(c.Name, ', ') AS CloseReasons FROM PostHistory ph JOIN CloseReasonTypes c ON ph.Comment::INTEGER = c.Id WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, u.DisplayName AS OwnerDisplayName, pvs.UpVotes, pvs.DownVotes, pvs.TotalBounty, COALESCE(cpr.CloseReasons, 'Not Closed') AS CloseReasonsDetails,
//        CASE WHEN rp.RecentPostRank = 1 THEN 'Most Recent Post' WHEN rp.RecentPostRank = 2 THEN 'Second Most Recent Post' ELSE 'Older Post' END AS PostAgeCategory,
//        EXTRACT(EPOCH FROM cast('2024-10-01 12:34:56' as timestamp) - rp.CreationDate) AS AgeInSeconds
// FROM RankedPosts rp LEFT JOIN Users u ON rp.OwnerUserId = u.Id LEFT JOIN PostVoteSummary pvs ON rp.PostId = pvs.PostId LEFT JOIN ClosedPostReasons cpr ON rp.PostId = cpr.PostId
// WHERE pvs.UpVotes - pvs.DownVotes > 0 AND rp.RecentPostRank <= 2 ORDER BY pvs.UpVotes DESC, rp.CreationDate ASC LIMIT 10;
fn q22275(db: &'static So) -> String {
    let Post { creation_date, owner_user_id, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let v = drain(db.post.with(creation_date.ge(add_years(t0, -1))).select(owner_user_id));
    let r = per_group(ranked(v, |&(p, o)| (o, Reverse(creation_date.get(p).unwrap()), p), false), |x| x.1);
    let rp = rel(r.into_iter().filter(|x| x.1 <= 2).map(|((p, _), k)| (p, k)).collect());
    type R = (Id<Post>, i64);
    let tp: MatSet<Id<Post>> = (&rp).map(|x: R| x.0).collect();
    let pvs = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select((&db.vote.vote_type_id).and((&db.vote.bounty_amount).opt()))).fold([0i64; 4], |a, (t, b)| {
        let term = if t == 8 { b } else { Some(0) };
        [a[0] + matches!(t, 2 | 5) as i64, a[1] + (t == 3) as i64, a[2] + term.is_some() as i64, a[3] + term.unwrap_or(0)]
    });
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let cr = db
        .post_history
        .with(post_history_type_id.eq(10))
        .group_by(post)
        .select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason))
        .buf_fold(|v| -> Str { Box::leak(v.to_vec().join(", ").into_boxed_str()) });
    let pr = || Same::<R>::new().map(|x: R| x.0);
    let v = drain((&rp).select(Same::<R>::new().and(pr().select((&pvs).filt(|a| a[0] - a[1] > 0))).and(pr().select(&cr).opt())));
    let v = top_n(v, |&(_, (((p, _), a), _))| (Reverse(a[0]), creation_date.get(p).unwrap(), p), 10);
    rows(v.into_iter().map(|(_, (((p, k), a), c))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), nullable(a[3], a[2]), V::S(c.unwrap_or("Not Closed"))]);
        f.push(V::S(if k == 1 { "Most Recent Post" } else if k == 2 { "Second Most Recent Post" } else { "Older Post" }));
        f.push(V::F(secs(t0 - creation_date.get(p).unwrap())));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.Score, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) OVER (PARTITION BY p.Id), 0) AS UpVotes,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) OVER (PARTITION BY p.Id), 0) AS DownVotes, COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount,
//        ROW_NUMBER() OVER (ORDER BY p.Score DESC, p.CreationDate DESC) AS PostRank
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN Comments c ON p.Id = c.PostId
//     WHERE p.PostTypeId = 1 AND p.Score > 0 AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// HighestRankedPosts AS (SELECT rp.Id, rp.Title, rp.CreationDate, rp.Score, rp.UpVotes, rp.DownVotes, rp.CommentCount FROM RankedPosts rp WHERE rp.PostRank <= 10),
// PostWithHistory AS (SELECT h.PostId, h.UserId, h.CreationDate AS HistoryDate, DENSE_RANK() OVER (PARTITION BY h.PostId ORDER BY h.CreationDate DESC) AS EditRank FROM PostHistory h WHERE h.PostHistoryTypeId IN (4, 5, 10))
// SELECT hrp.Id AS PostId, hrp.Title, hrp.CreationDate, hrp.Score, hrp.UpVotes, hrp.DownVotes, hrp.CommentCount, COUNT(DISTINCT ph.UserId) AS UniqueEditors, MAX(ph.HistoryDate) AS LastEditDate,
//        STRING_AGG(DISTINCT u.DisplayName, ', ') AS EditorNames
// FROM HighestRankedPosts hrp LEFT JOIN PostWithHistory ph ON hrp.Id = ph.PostId LEFT JOIN Users u ON ph.UserId = u.Id
// GROUP BY hrp.Id, hrp.Title, hrp.CreationDate, hrp.Score, hrp.UpVotes, hrp.DownVotes, hrp.CommentCount ORDER BY hrp.Score DESC, hrp.CreationDate DESC;
//
// PostRank numbers the post x vote x comment rows, so the top ten rows can all belong to one post; the GROUP BY folds them back to one row per post.
fn q392(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, .. } = &db.post;
    let qs = || db.post.with(post_type_id.eq(1).and(score.gt(0)).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))));
    type J = (Id<Post>, (Option<Id<Vote>>, Option<Id<Comment>>));
    let j: Vec<J> = drain(qs().select(Ident::<Post>::new().and(votes_of(db).opt().and(comments_of(db).opt())))).into_iter().map(|x| x.1).collect();
    let top = top_n(j, |&(p, vc)| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), p, vc), 10);
    let hp: MatSet<Id<Post>> = rel(top).map(|x: J| x.0).collect();
    let pa = (&hp)
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()))
        .fold([0i64; 3], |a, (t, c)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64]);
    let PostHistory { post_history_type_id, user, user_id, creation_date: hd, .. } = &db.post_history;
    let ph = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.is_in([4, 5, 10])));
    let ed = (&hp)
        .group_by(Ident::<Post>::new())
        .select(ph.select(user_id.opt().and(hd).and(user.select(&db.user.display_name).opt())))
        .buf_fold(|v| {
            let names: Vec<Str> = v.iter().flat_map(|x| x.1).collect();
            (distinct_some(v.iter().map(|x| x.0 .0)), v.iter().map(|x| x.0 .1).max().unwrap(), if names.is_empty() { None } else { Some(distinct_join(&names)) })
        });
    let v = drain((&pa).and((&ed).opt()));
    rows(v.into_iter().map(|(p, (a, e))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        f.extend(match e {
            Some((n, d, s)) => [V::I(n), V::T(d), ostr(s)],
            None => [V::I(0), V::Null, V::Null],
        });
        row(f)
    }))
}

// WITH TagStatistics AS (SELECT t.TagName, COUNT(DISTINCT p.Id) AS PostCount, SUM(p.ViewCount) AS TotalViews, SUM(p.AnswerCount) AS TotalAnswers, STRING_AGG(DISTINCT u.DisplayName, ', ') AS ActiveUsers, AVG(p.Score) AS AverageScore
//     FROM Tags t LEFT JOIN Posts p ON p.Tags LIKE '%' || '<' || t.TagName || '>' || '%' LEFT JOIN Users u ON p.OwnerUserId = u.Id WHERE t.Count > 0 GROUP BY t.TagName),
// UsersWithTopTags AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT t.Id) AS TagCount, SUM(p.ViewCount) AS TotalViews, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
//        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions
//     FROM Users u LEFT JOIN Posts p ON p.OwnerUserId = u.Id LEFT JOIN LATERAL (SELECT unnest(string_to_array(p.Tags, '><')) AS TagName) AS tag ON true LEFT JOIN Tags t ON t.TagName = tag.TagName
//     GROUP BY u.Id, u.DisplayName HAVING COUNT(DISTINCT t.Id) >= 3),
// BenchmarkResult AS (SELECT ts.TagName, ts.PostCount, ts.TotalViews, ts.TotalAnswers, ts.AverageScore, u.DisplayName AS UserWithMostPosts FROM TagStatistics ts
//     LEFT JOIN (SELECT p.Tags, p.OwnerUserId, COUNT(*) AS UserPostCount FROM Posts p WHERE p.Tags IS NOT NULL GROUP BY p.Tags, p.OwnerUserId) user_post_count ON user_post_count.Tags = ts.TagName
//     JOIN Users u ON u.Id = user_post_count.OwnerUserId ORDER BY ts.TotalViews DESC LIMIT 10)
// SELECT b.TagName, b.PostCount, b.TotalViews, b.TotalAnswers, b.AverageScore, COALESCE(b.UserWithMostPosts, 'No Posts') AS UserWithMostPosts FROM BenchmarkResult b;
//
// UsersWithTopTags is never read. `LIKE '%<name>%'` is the tag being one of the post's tags (no tag name has a wildcard or a bracket), which is the Post.tags edge.
fn q25521(db: &'static So) -> String {
    let Post { view_count, answer_count, score, tags_str, owner_user_id, .. } = &db.post;
    let by_tag: HashIdx<Id<Tag>, Id<Post>> = (&db.post.tags).inv().collect();
    let ts_ = db.tag.with((&db.tag.count).gt(0)).group_by(Ident::<Tag>::new()).select((&by_tag).select(view_count.opt().and(answer_count.opt()).and(score)).opt()).fold([0i64; 6], |a, x| match x {
        Some(((w, n), s)) => [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + n.is_some() as i64, a[4] + n.unwrap_or(0), a[5] + s],
        None => a,
    });
    let upc: MatSet<(Str, i64)> = db.post.select(tags_str.and(owner_user_id)).collect();
    let upi: HashIdx<Str, i64> = (&upc).map(|x: (Str, i64)| x.0).inv().select((&upc).map(|x: (Str, i64)| x.1)).collect();
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let v = drain((&ts_).and((&db.tag.tag_name).select(&upi).select(&uidx)));
    let v = top_n(v, |&(t, (a, _))| (a[1] == 0, Reverse(a[2]), t), 10);
    rows(v.into_iter().map(|(t, (a, u))| {
        let mut f = vec![V::S(db.tag.tag_name.get(t).unwrap()), V::I(a[0]), nullable(a[2], a[1]), nullable(a[4], a[3]), avg(a[5], a[0])];
        f.extend(ucols(db, u, &["name"]));
        row(f)
    }))
}

// WITH UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions, SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpvotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownvotes, STRING_AGG(DISTINCT t.TagName, ', ') AS Tags
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId LEFT JOIN UNNEST(string_to_array(p.Tags, ', ')) AS t(TagName) ON TRUE
//     WHERE u.Reputation > 1000 GROUP BY u.Id, u.DisplayName),
// ClosedPostStats AS (SELECT p.OwnerUserId, COUNT(p.Id) AS ClosedPosts FROM Posts p WHERE p.ClosedDate IS NOT NULL GROUP BY p.OwnerUserId),
// ActivitySummary AS (SELECT ua.UserId, ua.DisplayName, ua.TotalPosts, ua.TotalQuestions, ua.TotalAnswers, ua.TotalUpvotes, ua.TotalDownvotes, COALESCE(cps.ClosedPosts, 0) AS ClosedPosts, ua.Tags
//     FROM UserActivity ua LEFT JOIN ClosedPostStats cps ON ua.UserId = cps.OwnerUserId)
// SELECT asu.UserId, asu.DisplayName, asu.TotalPosts, asu.TotalQuestions, asu.TotalAnswers, asu.TotalUpvotes, asu.TotalDownvotes, asu.ClosedPosts,
//        CASE WHEN asu.TotalPosts > 100 THEN 'Active Contributor' WHEN asu.TotalPosts > 50 THEN 'Moderate Contributor' ELSE 'New Contributor' END AS ContributionLevel
// FROM ActivitySummary asu ORDER BY asu.TotalUpvotes DESC, asu.TotalPosts DESC FETCH FIRST 20 ROWS ONLY;
fn q1370(db: &'static So) -> String {
    let Post { post_type_id, tags_str, owner_user, closed_date, .. } = &db.post;
    let ua = db
        .user
        .with((&db.user.reputation).gt(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(tags_str.flat_map(|s: Str| s.split(", ")).opt())).opt())
        .fold([0i64; 5], |a, x| match x {
            Some(((t, v), _)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (v == Some(2)) as i64, a[4] + (v == Some(3)) as i64],
            None => a,
        });
    let cps = db.post.with(closed_date).group_by(owner_user).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let v = top_n(drain((&ua).and((&cps).opt())), |&(u, (a, _))| (Reverse(a[3]), Reverse(a[0]), u), 20);
    rows(v.into_iter().map(|(u, (a, c))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        f.push(V::I(c.unwrap_or(0)));
        f.push(V::S(if a[0] > 100 { "Active Contributor" } else if a[0] > 50 { "Moderate Contributor" } else { "New Contributor" }));
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, p.Tags, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn,
//        (SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id) AS CommentCount, p.OwnerUserId FROM Posts p WHERE p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'),
// PostMetrics AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, COALESCE(u.DisplayName, 'Anonymous') AS OwnerDisplayName, rp.CommentCount,
//        CASE WHEN rp.Score > 10 THEN 'High Score' WHEN rp.Score BETWEEN 1 AND 10 THEN 'Average Score' ELSE 'Low Score' END AS ScoreCategory
//     FROM RecentPosts rp LEFT JOIN Users u ON rp.OwnerUserId = u.Id WHERE rp.rn = 1),
// ClosedPostReasons AS (SELECT ph.PostId, STRING_AGG(cr.Name, ', ') AS CloseReasons FROM PostHistory ph JOIN CloseReasonTypes cr ON CAST(ph.Comment AS INT) = cr.Id WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.PostId)
// SELECT pm.PostId, pm.Title, pm.CreationDate, pm.ViewCount, pm.Score, pm.OwnerDisplayName, pm.CommentCount, pm.ScoreCategory, COALESCE(cpr.CloseReasons, 'No closure reason') AS CloseReasons,
//        CASE WHEN pm.ViewCount = 0 THEN 'No Views' WHEN pm.ViewCount IS NULL THEN 'Unknown Views' ELSE 'Viewed' END AS ViewStatus
// FROM PostMetrics pm LEFT JOIN ClosedPostReasons cpr ON pm.PostId = cpr.PostId WHERE pm.CommentCount > 1 ORDER BY pm.Score DESC, pm.CreationDate ASC FETCH FIRST 50 ROWS ONLY;
fn q20344(db: &'static So) -> String {
    let Post { creation_date, owner_user_id, owner_user, score, view_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.gt(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(owner_user_id.opt()));
    let first = set_of(top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 1, false).into_iter().map(|x| x.0).collect());
    let cc = (&first).group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let cr = db
        .post_history
        .with(post_history_type_id.is_in([10, 11]))
        .group_by(post)
        .select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason))
        .buf_fold(|v| -> Str { Box::leak(v.to_vec().join(", ").into_boxed_str()) });
    let v = drain((&cc).filt(|n| n > 1).and((&cr).opt()));
    let v = top_n(v, |&(p, _)| (Reverse(score.get(p).unwrap()), creation_date.get(p).unwrap(), p), 50);
    rows(v.into_iter().map(|(p, (n, c))| {
        let s = score.get(p).unwrap();
        let w = view_count.get(p);
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.push(V::S(owner_user.get(p).map_or("Anonymous", |u| db.user.display_name.get(u).unwrap())));
        f.extend([V::I(n), V::S(if s > 10 { "High Score" } else if (1..=10).contains(&s) { "Average Score" } else { "Low Score" }), V::S(c.unwrap_or("No closure reason"))]);
        f.push(V::S(if w == Some(0) { "No Views" } else if w.is_none() { "Unknown Views" } else { "Viewed" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS Rank,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) OVER (PARTITION BY p.Id), 0) AS UpVotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) OVER (PARTITION BY p.Id), 0) AS DownVotes
//     FROM Posts p LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= DATE '2024-10-01' - INTERVAL '1 year'),
// ClosedPosts AS (SELECT ph.PostId, STRING_AGG(DISTINCT c.Name, ', ') AS CloseReasons, COUNT(*) AS CloseCount FROM PostHistory ph JOIN CloseReasonTypes c ON CAST(ph.Comment AS integer) = c.Id
//     WHERE ph.PostHistoryTypeId IN (10, 11) GROUP BY ph.PostId),
// FinalPosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.UpVotes - rp.DownVotes AS NetVotes, COALESCE(cp.CloseReasons, 'No Close Reasons') AS CloseReasons, COALESCE(cp.CloseCount, 0) AS CloseCount,
//        CASE WHEN rp.Rank = 1 THEN 'Latest Post' ELSE 'Older Post' END AS PostLabel FROM RankedPosts rp LEFT JOIN ClosedPosts cp ON rp.PostId = cp.PostId)
// SELECT fp.PostId, fp.Title AS PostTitle, fp.CreationDate, fp.Score, fp.NetVotes, fp.CloseReasons, fp.CloseCount,
//        CASE WHEN fp.NetVotes < 0 THEN 'Needs Attention' WHEN fp.Score > 50 THEN 'Highly Voted' ELSE 'Regular Activity' END AS PostActivity
// FROM FinalPosts fp WHERE fp.CloseCount = 0 ORDER BY fp.Score DESC NULLS LAST, fp.CreationDate ASC FETCH FIRST 20 ROWS ONLY;
//
// RankedPosts has one row per post x vote and each is kept; PostLabel is never read. CloseCount = 0 is the post having no ClosedPosts row.
fn q22153(db: &'static So) -> String {
    let Post { creation_date, score, .. } = &db.post;
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let cp: MatSet<Id<Post>> = db.post_history.with(post_history_type_id.is_in([10, 11])).with(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason)).select(post).collect();
    let rp = || db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1))).minus(&cp);
    let nv = rp().group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id)).fold(0i64, |n, t| n + (t == 2) as i64 - (t == 3) as i64);
    let v = drain(rp().select(votes_of(db).opt().and((&nv).opt())));
    let v = top_n(v, |&(p, (x, _))| (Reverse(score.get(p).unwrap()), creation_date.get(p).unwrap(), p, x), 20);
    rows(v.into_iter().map(|(p, (_, n))| {
        let n = n.unwrap_or(0);
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend([V::I(n), V::S("No Close Reasons"), V::I(0)]);
        f.push(V::S(if n < 0 { "Needs Attention" } else if score.get(p).unwrap() > 50 { "Highly Voted" } else { "Regular Activity" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, p.CreationDate, p.Score, p.OwnerUserId, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank,
//        COALESCE(u.DisplayName, 'Anonymous') AS OwnerName FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year' AND p.ViewCount IS NOT NULL),
// TopPosts AS (SELECT rp.PostId, rp.Title, rp.ViewCount, rp.CreationDate, rp.Score, rp.OwnerName FROM RankedPosts rp WHERE rp.Rank <= 5),
// PostComments AS (SELECT c.PostId, COUNT(c.Id) AS CommentCount FROM Comments c GROUP BY c.PostId),
// PostScores AS (SELECT tp.PostId, tp.Title, tp.ViewCount, tp.CreationDate, tp.Score, tp.OwnerName, COALESCE(pc.CommentCount, 0) AS TotalComments,
//        CASE WHEN tp.Score >= 0 THEN 'Positive' WHEN tp.Score < 0 THEN 'Negative' ELSE 'Neutral' END AS ScoreCategory FROM TopPosts tp LEFT JOIN PostComments pc ON tp.PostId = pc.PostId)
// SELECT ps.PostId, ps.Title, ps.ViewCount, ps.CreationDate, ps.Score, ps.OwnerName, ps.TotalComments, ps.ScoreCategory,
//        (SELECT STRING_AGG(pt.Name, ', ') FROM PostHistory ph JOIN PostHistoryTypes pt ON ph.PostHistoryTypeId = pt.Id WHERE ph.PostId = ps.PostId AND ph.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 month') AS RecentHistory
// FROM PostScores ps WHERE ps.TotalComments > 5 ORDER BY ps.Score DESC, ps.ViewCount DESC OFFSET 0 ROWS FETCH NEXT 10 ROWS ONLY;
fn q23791(db: &'static So) -> String {
    let Post { creation_date, post_type_id, view_count, score, owner_user, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1))).with(view_count).select(post_type_id));
    let tp = set_of(top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false).into_iter().map(|x| x.0).collect());
    let cc = (&tp).group_by(Ident::<Post>::new()).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let rh = (&tp)
        .group_by(Ident::<Post>::new())
        .select(history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.creation_date).ge(add_months(date(2024, 10, 1), -1)))).select(htype_name(db)))
        .buf_fold(|v| -> Str { Box::leak(v.to_vec().join(", ").into_boxed_str()) });
    let v = drain((&cc).filt(|n| n > 5).and((&rh).opt()));
    let v = top_n(v, |&(p, _)| (Reverse(score.get(p).unwrap()), Reverse(view_count.get(p)), p), 10);
    rows(v.into_iter().map(|(p, (n, h))| {
        let mut f = post_fields(db, p, &["id", "title", "views", "created", "score"]);
        f.push(V::S(owner_user.get(p).map_or("Anonymous", |u| db.user.display_name.get(u).unwrap())));
        f.extend([V::I(n), V::S(if score.get(p).unwrap() >= 0 { "Positive" } else { "Negative" }), ostr(h)]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.CreationDate, RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U),
// PostWithTagCounts AS (SELECT P.Id AS PostId, P.Title, COUNT(T.TagName) AS TagCount, P.Score, P.OwnerUserId, COALESCE(P.ClosedDate, '9999-12-31') AS CloseDate
//     FROM Posts P LEFT JOIN (SELECT P.Id AS PostId, unnest(string_to_array(substring(P.Tags, 2, length(P.Tags) - 2), '><')) AS TagName FROM Posts P) T ON P.Id = T.PostId
//     GROUP BY P.Id, P.Title, P.Score, P.OwnerUserId, P.ClosedDate),
// ActiveAndClosedPosts AS (SELECT P.PostId, P.Title, COUNT(C.Id) AS CommentCount, SUM(CASE WHEN P.CloseDate < CAST('2024-10-01 12:34:56' AS TIMESTAMP) THEN 1 ELSE 0 END) AS ClosedCount,
//        SUM(CASE WHEN P.CloseDate = '9999-12-31' THEN 1 ELSE 0 END) AS ActiveCount FROM PostWithTagCounts P LEFT JOIN Comments C ON P.PostId = C.PostId GROUP BY P.PostId, P.Title),
// UserBadges AS (SELECT B.UserId, COUNT(B.Id) AS BadgeCount, MAX(B.Class) AS HighestBadgeClass FROM Badges B GROUP BY B.UserId)
// SELECT UR.DisplayName, UR.Reputation, COALESCE(UB.BadgeCount, 0) AS BadgeCount, COALESCE(UB.HighestBadgeClass, 0) AS HighestBadgeClass, ACP.Title, ACP.CommentCount, ACP.ClosedCount, ACP.ActiveCount
// FROM UserReputation UR LEFT JOIN UserBadges UB ON UR.UserId = UB.UserId JOIN ActiveAndClosedPosts ACP ON UR.UserId = ACP.PostId
// WHERE (UR.Reputation > 1000 OR ACP.CommentCount > 5) AND (ACP.ClosedCount = 0 OR ACP.ActiveCount < 5) AND (UB.BadgeCount IS NULL OR UB.BadgeCount > 2)
// ORDER BY UR.Reputation DESC, ACP.CommentCount DESC LIMIT 100;
//
// The user's Id is joined to the post's Id, as written. TagCount is never read.
fn q21437(db: &'static So) -> String {
    let Post { closed_date, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let far = date(9999, 12, 31);
    let acp = db.post.group_by(Ident::<Post>::new()).select(closed_date.opt().and(comments_of(db).opt())).fold([0i64; 3], |a, (d, c)| {
        let d = d.unwrap_or(far);
        [a[0] + c.is_some() as i64, a[1] + (d < t0) as i64, a[2] + (d == far) as i64]
    });
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold((0i64, 0i64), |(n, m), c| (n + 1, m.max(c)));
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let rep = |u: Id<User>| db.user.reputation.get(u).unwrap();
    type J = ((Id<User>, Option<(i64, i64)>), (Id<Post>, [i64; 3]));
    let v = drain(
        db.user
            .select(Ident::<User>::new().and((&ub).opt()).and((&db.user.origid).select(&pidx).select(Ident::<Post>::new().and(&acp))))
            .filt(move |((u, b), (_, a)): J| (rep(u) > 1000 || a[0] > 5) && (a[1] == 0 || a[2] < 5) && b.map_or(true, |b| b.0 > 2)),
    );
    let v = top_n(v, |&(_, ((u, _), (p, a)))| (Reverse(rep(u)), Reverse(a[0]), u, p), 100);
    rows(v.into_iter().map(|(_, ((u, b), (p, a)))| {
        let b = b.unwrap_or((0, 0));
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(b.0), V::I(b.1)]);
        f.extend(post_fields(db, p, &["title"]));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS PostRank
//     FROM Posts p WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// UserStats AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpvotes, COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownvotes,
//        COALESCE(SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges, COALESCE(SUM(b.Class) FILTER (WHERE b.Class IS NOT NULL), 0) AS TotalBadges
//     FROM Users u LEFT JOIN Votes v ON u.Id = v.UserId LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.DisplayName, u.Reputation),
// PostHistoryWithCloseReasons AS (SELECT ph.PostId, ph.CreationDate, STRING_AGG(cr.Name, ', ') AS CloseReasons FROM PostHistory ph LEFT JOIN CloseReasonTypes cr ON CAST(ph.Comment AS INTEGER) = cr.Id
//     WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId, ph.CreationDate)
// SELECT us.DisplayName, us.Reputation, us.TotalUpvotes, us.TotalDownvotes, us.GoldBadges, us.TotalBadges, rp.Title, rp.CreationDate AS PostCreationDate, rp.Score, rp.ViewCount, rp.AnswerCount, rp.CommentCount, ph.CloseReasons
// FROM UserStats us JOIN RankedPosts rp ON us.UserId = rp.PostId LEFT JOIN PostHistoryWithCloseReasons ph ON rp.PostId = ph.PostId
// WHERE rp.PostRank <= 3 ORDER BY us.Reputation DESC, rp.ViewCount DESC LIMIT 100;
//
// The user's Id is joined to the post's Id, as written.
fn q4061(db: &'static So) -> String {
    let Post { creation_date, owner_user_id, view_count, origid, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).select(owner_user_id.opt()));
    let rp = set_of(top_per(v, |&(_, u)| u, |&(p, _)| (Reverse(creation_date.get(p).unwrap()), p), 3, false).into_iter().map(|x| x.0).collect());
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let us_ = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt().and(badges_of(db).select(&db.badge.class).opt())).fold([0i64; 4], |a, (t, c)| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + (c == Some(1)) as i64, a[3] + c.unwrap_or(0)]
    });
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post, post_history_type_id, comment, creation_date: hd, .. } = &db.post_history;
    let ph = db
        .post_history
        .with(post_history_type_id.eq(10))
        .group_by(post.and(hd))
        .select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason).opt())
        .buf_fold(|v| -> Option<Str> {
            let n: Vec<Str> = v.iter().flatten().copied().collect();
            if n.is_empty() { None } else { Some(Box::leak(n.join(", ").into_boxed_str())) }
        });
    let phr = rel(drain(&ph));
    type H = ((Id<Post>, i64), Option<Str>);
    let phi: HashIdx<Id<Post>, Option<Str>> = (&phr).map(|x: H| x.0 .0).inv().select((&phr).map(|x: H| x.1)).collect();
    let v = drain((&rp).select(origid.select(&uidx).select(Ident::<User>::new().and(&us_)).and((&phi).opt())));
    let v = top_n(v, |&(p, ((u, _), c))| { let w = view_count.get(p); (Reverse(db.user.reputation.get(u).unwrap()), w.is_none(), Reverse(w), p, c) }, 100);
    rows(v.into_iter().map(|(p, ((u, a), c))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(a.map(V::I));
        f.extend(post_fields(db, p, &["title", "created", "score", "views", "answers", "comments"]));
        f.push(ostr(c.flatten()));
        row(f)
    }))
}

// WITH UserEngagement AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, COUNT(P.Id) AS PostCount, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, ROW_NUMBER() OVER (ORDER BY COUNT(P.Id) DESC) AS EngagementRank
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Votes V ON P.Id = V.PostId GROUP BY U.Id, U.DisplayName, U.Reputation),
// TopUsers AS (SELECT UserId, DisplayName, Reputation, PostCount, UpVotes, DownVotes FROM UserEngagement WHERE EngagementRank <= 10),
// RecentPosts AS (SELECT P.Id AS PostId, P.Title, P.CreationDate, P.OwnerUserId, P.PostTypeId, RANK() OVER (PARTITION BY P.PostTypeId ORDER BY P.CreationDate DESC) AS RecentRank
//     FROM Posts P WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days')
// SELECT TU.DisplayName, TU.Reputation, COALESCE(RP.PostId, -1) AS RecentPostId, COALESCE(RP.Title, 'No recent posts') AS RecentPostTitle, COALESCE(RP.CreationDate, '1900-01-01') AS PostCreationDate,
//        CASE WHEN TU.UpVotes > TU.DownVotes THEN 'Positively Engaged' WHEN TU.UpVotes < TU.DownVotes THEN 'Negatively Engaged' ELSE 'Neutral Engagement' END AS EngagementProfile,
//        STRING_AGG(DISTINCT PT.Name, ', ') AS PostTypeNames
// FROM TopUsers TU LEFT JOIN RecentPosts RP ON TU.UserId = RP.OwnerUserId AND RP.RecentRank = 1 LEFT JOIN PostTypes PT ON RP.PostTypeId = PT.Id
// GROUP BY TU.UserId, TU.DisplayName, TU.Reputation, RP.PostId, RP.Title, RP.CreationDate, TU.UpVotes, TU.DownVotes ORDER BY TU.Reputation DESC;
//
// Each group is one user and at most one recent post, so PostTypeNames is that post's type name.
fn q21883(db: &'static So) -> String {
    let Post { creation_date, post_type_id, owner_user, .. } = &db.post;
    let ue = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt()).fold([0i64; 3], |a, x| match x {
        Some(t) => [a[0] + 1, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64],
        None => a,
    });
    let tu = set_of(top_n(drain(&ue), |&(u, a)| (Reverse(a[0]), u), 10).into_iter().map(|x| x.0).collect());
    let v = drain(db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).select(post_type_id));
    let rp = set_of(top_per(v, |&(_, t)| t, |&(p, _)| Reverse(creation_date.get(p).unwrap()), 1, true).into_iter().map(|x| x.0).collect());
    let rpi: HashIdx<Id<User>, Id<Post>> = (&rp).select(owner_user).inv().collect();
    let v = drain((&tu).select((&ue).and((&rpi).select(Ident::<Post>::new().and(ptype_name(db))).opt())));
    rows(v.into_iter().map(|(u, (a, p))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        match p {
            Some((p, n)) => {
                f.push(post_fields(db, p, &["id"]).remove(0));
                f.push(V::S(db.post.title.get(p).unwrap_or("No recent posts")));
                f.extend(post_fields(db, p, &["created"]));
                f.push(V::S(if a[1] > a[2] { "Positively Engaged" } else if a[1] < a[2] { "Negatively Engaged" } else { "Neutral Engagement" }));
                f.push(V::S(n));
            }
            None => {
                f.extend([V::I(-1), V::S("No recent posts"), V::T(date(1900, 1, 1))]);
                f.push(V::S(if a[1] > a[2] { "Positively Engaged" } else if a[1] < a[2] { "Negatively Engaged" } else { "Neutral Engagement" }));
                f.push(V::Null);
            }
        }
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS RecentPostRank, COALESCE(u.Reputation, 0) AS UserReputation
//     FROM Posts p LEFT JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.PostTypeId = 1 AND p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// PostWithTopVotes AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.CommentCount, rp.UserReputation, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount
//     FROM RankedPosts rp LEFT JOIN Votes v ON rp.PostId = v.PostId GROUP BY rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, rp.CommentCount, rp.UserReputation),
// ClosedPosts AS (SELECT ph.PostId, STRING_AGG(DISTINCT ctr.Name, ', ') AS CloseReason FROM PostHistory ph JOIN CloseReasonTypes ctr ON CAST(ph.Comment AS INTEGER) = ctr.Id WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId)
// SELECT p.Title, p.CreationDate, p.Score, p.ViewCount, p.CommentCount, p.UserReputation, p.UpVoteCount, p.DownVoteCount, COALESCE(cp.CloseReason, 'Not Closed') AS CloseReasonStatus
// FROM PostWithTopVotes p LEFT JOIN ClosedPosts cp ON p.PostId = cp.PostId WHERE p.UserReputation > 1000 AND (p.UpVoteCount - p.DownVoteCount) > 5 ORDER BY p.Score DESC, p.CommentCount DESC LIMIT 50;
//
// RankedPosts has one row per question x comment, so the vote sums are over comment x vote rows.
fn q1494(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, owner_user, .. } = &db.post;
    let qs = || db.post.with(post_type_id.eq(1).and(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))).with(owner_user.select(Ident::<User>::new().with((&db.user.reputation).gt(1000))));
    let pv = qs().group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 3], |a, (c, t)| {
        [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let cn = qs().group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post, post_history_type_id, comment, .. } = &db.post_history;
    let cp = db
        .post_history
        .with(post_history_type_id.eq(10))
        .group_by(post)
        .select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason))
        .buf_fold(|v| distinct_join(&v));
    let v = drain((&pv).filt(|a| a[1] - a[2] > 5).and(&cn).and((&cp).opt()));
    let v = top_n(v, |&(p, ((_, n), _))| (Reverse(score.get(p).unwrap()), Reverse(n), p), 50);
    rows(v.into_iter().map(|(p, ((a, n), c))| {
        let mut f = post_fields(db, p, &["title", "created", "score", "views"]);
        f.push(V::I(n));
        f.extend(post_fields(db, p, &["rep"]));
        f.extend([V::I(a[1]), V::I(a[2]), V::S(c.unwrap_or("Not Closed"))]);
        row(f)
    }))
}

// WITH UserRankings AS (SELECT U.Id AS UserId, U.DisplayName, RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank FROM Users U),
// HighReputationUsers AS (SELECT UserId, DisplayName, ReputationRank FROM UserRankings WHERE ReputationRank <= 10),
// RecentPostCounts AS (SELECT P.OwnerUserId, COUNT(P.Id) AS RecentPostsCount FROM Posts P WHERE P.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' GROUP BY P.OwnerUserId),
// CombinedData AS (SELECT U.DisplayName, COALESCE(R.RecentPostsCount, 0) AS RecentPostsCount, COALESCE(B.BadgesCount, 0) AS BadgesCount, U.ReputationRank
//     FROM HighReputationUsers U LEFT JOIN RecentPostCounts R ON U.UserId = R.OwnerUserId LEFT JOIN (SELECT UserId, COUNT(*) AS BadgesCount FROM Badges GROUP BY UserId) B ON U.UserId = B.UserId)
// SELECT CD.DisplayName, CD.RecentPostsCount, CD.BadgesCount, CASE WHEN CD.RecentPostsCount = 0 THEN 'No posts in the last 30 days' ELSE 'Active user' END AS ActivityStatus,
//        (SELECT STRING_AGG(T.TagName, ', ') FROM Tags T WHERE T.WikiPostId IS NOT NULL) AS PopularTags,
//        (SELECT COUNT(*) FROM Votes V WHERE V.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '7 days' AND V.VoteTypeId = 2) AS RecentUpVotes,
//        (SELECT COUNT(*) FROM PostHistory PH WHERE PH.UserId IN (SELECT U.Id FROM Users U WHERE U.Reputation > 1000) AND PH.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '90 days') AS HistoryCommentsFromHighReputationUsers
// FROM CombinedData CD GROUP BY CD.DisplayName, CD.RecentPostsCount, CD.BadgesCount, CD.ReputationRank ORDER BY CD.ReputationRank;
//
// The uncorrelated scalar subqueries are separate queries. STRING_AGG has no ORDER BY; the tags come in table order.
fn q20981(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let r = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let hu = rel(r.into_iter().take_while(|x| x.1 <= 10).map(|((u, _), k)| (u, k)).collect());
    let Post { creation_date, owner_user, .. } = &db.post;
    let rpc = db.post.with(creation_date.gt(add_days(t0, -30))).group_by(owner_user).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let bc = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let mut tags = drain(db.tag.with(&db.tag.wiki_post_id).select(&db.tag.tag_name));
    tags.sort_by_key(|x| x.0);
    let pt: Str = Box::leak(tags.into_iter().map(|x| x.1).collect::<Vec<_>>().join(", ").into_boxed_str());
    let ru = count(db.vote.with((&db.vote.creation_date).gt(add_days(t0, -7)).and((&db.vote.vote_type_id).eq(2))));
    let hc = count(db.post_history.with((&db.post_history.user).select(Ident::<User>::new().with((&db.user.reputation).gt(1000)))).with((&db.post_history.creation_date).gt(add_days(t0, -90))));
    type R = (Id<User>, i64);
    let v = drain((&hu).select(Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0).select((&rpc).opt().and((&bc).opt())))));
    let g: MatSet<(Str, i64, i64, i64)> = rel(v).map(|(_, ((u, k), (r, b))): (usize, (R, (Option<i64>, Option<i64>)))| (db.user.display_name.get(u).unwrap(), r.unwrap_or(0), b.unwrap_or(0), k)).collect();
    let v = drain(&g);
    rows(v.into_iter().map(|(_, (n, r, b, _))| {
        row(vec![V::S(n), V::I(r), V::I(b), V::S(if r == 0 { "No posts in the last 30 days" } else { "Active user" }), V::S(pt), V::I(ru), V::I(hc)])
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.CreationDate, COALESCE(SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) OVER (PARTITION BY U.Id), 0) AS UpVotesReceived,
//        COALESCE(SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) OVER (PARTITION BY U.Id), 0) AS DownVotesReceived FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId WHERE U.Reputation IS NOT NULL),
// UserBadges AS (SELECT UserId, COUNT(*) AS BadgeCount, STRING_AGG(Name, ', ') AS BadgeNames FROM Badges GROUP BY UserId),
// PostStatistics AS (SELECT P.OwnerUserId, COUNT(P.Id) AS TotalPosts, SUM(P.ViewCount) AS TotalViews, AVG(P.Score) AS AverageScore, COUNT(DISTINCT P.Tags) AS TotalTagsUsed FROM Posts P GROUP BY P.OwnerUserId),
// QualifiedUsers AS (SELECT UR.UserId, UR.DisplayName, UR.Reputation, COALESCE(UB.BadgeCount, 0) AS BadgeCount, PS.TotalPosts, PS.TotalViews, PS.AverageScore, PS.TotalTagsUsed
//     FROM UserReputation UR LEFT JOIN UserBadges UB ON UR.UserId = UB.UserId LEFT JOIN PostStatistics PS ON UR.UserId = PS.OwnerUserId WHERE UR.Reputation > (SELECT AVG(Reputation) FROM Users))
// SELECT QU.UserId, QU.DisplayName, QU.Reputation, QU.BadgeCount, QU.TotalPosts, QU.TotalViews, QU.AverageScore, QU.TotalTagsUsed, ROW_NUMBER() OVER (ORDER BY QU.Reputation DESC) AS Rank
// FROM QualifiedUsers QU WHERE (QU.BadgeCount > 5 OR QU.TotalPosts > 50) AND (QU.TotalViews IS NOT NULL) ORDER BY QU.Reputation DESC, QU.DisplayName ASC FETCH FIRST 10 ROWS ONLY;
//
// UserReputation has one row per user x vote they cast, so a voter repeats; Rank numbers those rows.
fn q20810(db: &'static So) -> String {
    let Post { owner_user, view_count, score, tags_str, .. } = &db.post;
    let (sum, n) = db.user.select(&db.user.reputation).fold_flat((0i128, 0i128), |(s, n), x| (s + x as i128, n + 1));
    let ub = db.badge.group_by(&db.badge.user).select(Ident::<Badge>::new()).fold(0i64, |n, _| n + 1);
    let ps = db.post.group_by(owner_user).select(view_count.opt().and(score)).fold([0i64; 4], |a, (w, s)| [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s]);
    let tu = db.post.group_by(owner_user).select(tags_str).count_distinct();
    type J = (Id<User>, (((Option<Id<Vote>>, Option<i64>), [i64; 4]), Option<i64>));
    let rep = |u: Id<User>| db.user.reputation.get(u).unwrap();
    let v: Vec<J> = drain(
        db.user
            .with((&db.user.reputation).filt(move |r| r as i128 * n > sum))
            .select(Ident::<User>::new().and(votes_by(db).opt().and((&ub).opt()).and((&ps).filt(|a| a[1] > 0)).and((&tu).opt())))
            .filt(|(_, (((_, b), a), _)): J| b.unwrap_or(0) > 5 || a[0] > 50),
    )
    .into_iter()
    .map(|x| x.1)
    .collect();
    let r = ranked(v, |&(u, (((x, _), _), _))| (Reverse(rep(u)), db.user.display_name.get(u).unwrap(), u, x), false);
    let r = top_n(r, |x| x.1, 10);
    rows(r.into_iter().map(|((u, (((_, b), a), t)), k)| {
        let mut f = ucols(db, u, &["uid", "name", "rep"]);
        f.extend([V::I(b.unwrap_or(0)), V::I(a[0]), V::I(a[2]), avg(a[3], a[0]), V::I(t.unwrap_or(0)), V::I(k)]);
        row(f)
    }))
}

fn json_close_reason(s: Str) -> Option<i64> {
    let t = s.trim();
    if !t.starts_with('{') {
        return None;
    }
    let i = t.find("\"closeReasonId\"")? + "\"closeReasonId\"".len();
    let rest = t[i..].trim_start().strip_prefix(':')?.trim_start().trim_start_matches('"');
    let end = rest.find(|c: char| !(c.is_ascii_digit() || c == '-')).unwrap_or(rest.len());
    rest[..end].parse().ok()
}

// WITH UserPostStats AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS PostCount, SUM(COALESCE(p.ViewCount, 0)) AS TotalViews, SUM(COALESCE(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END, 0)) AS QuestionCount,
//        SUM(COALESCE(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END, 0)) AS AnswerCount, SUM(COALESCE(p.Score, 0)) AS TotalScore FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id, u.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, PostCount, TotalViews, QuestionCount, AnswerCount, TotalScore, RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank FROM UserPostStats),
// ClosedPosts AS (SELECT p.Id AS PostId, p.Title, ph.CreationDate AS ClosedDate, c.Name AS CloseReason, ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC) AS ReasonRank
//     FROM Posts p JOIN PostHistory ph ON p.Id = ph.PostId AND ph.PostHistoryTypeId = 10 JOIN CloseReasonTypes c ON c.Id = CAST(JSON_VALUE(ph.Comment, '$.closeReasonId') AS INTEGER) WHERE ph.CreationDate IS NOT NULL)
// SELECT tu.DisplayName, tu.PostCount, tu.TotalViews, tu.QuestionCount, tu.AnswerCount, tu.TotalScore, ARRAY_AGG(DISTINCT cp.Title) AS ClosedPostsTitles, COUNT(DISTINCT cp.PostId) AS TotalClosedPosts,
//        COALESCE(AVG(CASE WHEN cp.ReasonRank = 1 THEN 1 ELSE 0 END), 0) AS ReopenedPostsCount
// FROM TopUsers tu LEFT JOIN ClosedPosts cp ON tu.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = cp.PostId) WHERE tu.ScoreRank <= 10
// GROUP BY tu.UserId, tu.DisplayName, tu.PostCount, tu.TotalViews, tu.QuestionCount, tu.AnswerCount, tu.TotalScore ORDER BY tu.TotalScore DESC;
//
// JSON_VALUE reads the closeReasonId member of a JSON object; the close comments here are bare numbers, so every one is NULL and ClosedPosts is empty.
fn q453(db: &'static So) -> String {
    let ups = user_posts(db);
    let r = ranked(drain(&ups), |&(_, a)| Reverse(a[4]), false);
    let tu = set_of(r.into_iter().take_while(|x| x.1 <= 10).map(|x| x.0 .0).collect());
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let PostHistory { post, post_history_type_id, comment, creation_date: hd, .. } = &db.post_history;
    let cph: Vec<(Id<PostHistory>, Id<Post>)> = drain(db.post_history.with(post_history_type_id.eq(10)).with(comment.flat_map(json_close_reason).select(&reason)).select(post));
    let cp = rel(per_group(ranked(cph, |&(h, p)| (p, Reverse(hd.get(h).unwrap()), h), false), |x| x.1));
    type C = ((Id<PostHistory>, Id<Post>), i64);
    let byu: HashIdx<Id<User>, C> = (&cp).map(|x: C| x.0 .1).select(&db.post.owner_user).inv().select(&cp).collect();
    let g = (&tu).group_by(Ident::<User>::new()).select((&byu).opt()).buf_fold(|v| {
        let t: Vec<Option<Str>> = v.iter().map(|x| x.and_then(|c| db.post.title.get(c.0 .1))).collect();
        let mut t2 = t.clone();
        t2.sort();
        t2.dedup();
        let titles: &'static [Option<Str>] = Box::leak(t2.into_boxed_slice());
        (titles, distinct_some(v.iter().map(|x| x.map(|c| c.0 .1))), v.iter().filter(|x| x.map_or(false, |c| c.1 == 1)).count() as i64, v.len() as i64)
    });
    let v = drain((&ups).and(&g));
    rows(v.into_iter().map(|(u, (a, (t, n, r1, rows_)))| {
        let mut f = ucols(db, u, &["name"]);
        f.extend([V::I(a[1]), V::I(a[6]), V::I(a[2]), V::I(a[3]), V::I(a[4])]);
        f.push(V::L(t.iter().map(|&x| ostr(x)).collect()));
        f.extend([V::I(n), V::F(r1 as f64 / rows_ as f64)]);
        row(f)
    }))
}

// WITH RecursiveTags AS (SELECT p.Id AS PostId, unnest(string_to_array(substring(p.Tags, 2, length(p.Tags)-2), '><')) AS Tag FROM Posts p WHERE p.PostTypeId = 1),
// TagCounts AS (SELECT Tag, COUNT(*) AS TagFrequency FROM RecursiveTags GROUP BY Tag),
// TopTags AS (SELECT Tag, TagFrequency FROM TagCounts ORDER BY TagFrequency DESC LIMIT 10),
// PostsWithTopTags AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, rt.Tag FROM Posts p JOIN RecursiveTags rt ON p.Id = rt.PostId JOIN TopTags t ON rt.Tag = t.Tag),
// UserEngagement AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(DISTINCT p.Id) AS TotalPosts, SUM(COALESCE(c.Score, 0)) AS TotalCommentScores, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName),
// UserActivity AS (SELECT ue.UserId, ue.DisplayName, ue.TotalPosts, ue.TotalCommentScores, ue.TotalUpVotes, ue.TotalDownVotes, ROW_NUMBER() OVER (ORDER BY ue.TotalPosts DESC) AS Rank FROM UserEngagement ue)
// SELECT p.PostId, p.Title, p.CreationDate, p.Tag, ua.DisplayName AS User, ua.TotalPosts, ua.TotalCommentScores, ua.TotalUpVotes, ua.TotalDownVotes
// FROM PostsWithTopTags p JOIN UserActivity ua ON p.PostId = ua.UserId ORDER BY p.CreationDate DESC, ua.TotalPosts DESC;
//
// The post's Id is joined to the user's Id, as written. The WITH is not RECURSIVE; Rank is never read.
fn q29102(db: &'static So) -> String {
    let Post { post_type_id, tags_str, origid, .. } = &db.post;
    let qs = || db.post.with(post_type_id.eq(1));
    let tc = qs().select(tags_str.flat_map(tag_list)).group_by(Same::<Str>::new()).fold(0i64, |n, _| n + 1);
    let tt = set_of(top_n(drain(&tc), |&(t, n)| (Reverse(n), t), 10).into_iter().map(|x| x.0).collect());
    let uidx: HashIdx<i64, Id<User>> = (&db.user.origid).inv().collect();
    let users: MatSet<Id<User>> = qs().select(origid).select(&uidx).collect();
    let ue = (&users)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(comments_of(db).select(&db.comment.score).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 3], |a, x| match x {
            Some((c, t)) => [a[0] + c.unwrap_or(0), a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64],
            None => a,
        });
    let pc = (&users).group_by(Ident::<User>::new()).select(posts_of(db).opt()).buf_fold(|v| distinct_some(v.iter().copied()));
    let v = drain(qs().select(tags_str.flat_map(tag_list).select(&tt).and(origid.select(&uidx).select(Ident::<User>::new().and(&ue).and(&pc)))));
    rows(v.into_iter().map(|(p, (t, ((u, a), n)))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.push(V::S(t));
        f.extend(ucols(db, u, &["name"]));
        f.extend([V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days'),
// PostVotes AS (SELECT v.PostId, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 WHEN v.VoteTypeId = 3 THEN -1 ELSE 0 END) AS VoteScore FROM Votes v GROUP BY v.PostId),
// PostHistories AS (SELECT ph.PostId, COUNT(*) AS EditCount, MAX(ph.CreationDate) AS LastEditDate, ARRAY_AGG(DISTINCT ph.UserDisplayName) AS Editors FROM PostHistory ph WHERE ph.PostHistoryTypeId IN (4, 5) GROUP BY ph.PostId),
// CombinedData AS (SELECT rp.Id AS PostId, rp.Title, rp.CreationDate, rp.ViewCount, COALESCE(pv.VoteScore, 0) AS VoteScore, COALESCE(ph.EditCount, 0) AS EditCount, ARRAY_LENGTH(ph.Editors, 1) AS UniqueEditorsCount,
//        CASE WHEN rp.AnswerCount > 0 THEN 'Answered' ELSE 'Unanswered' END AS PostStatus
//     FROM RankedPosts rp LEFT JOIN PostVotes pv ON rp.Id = pv.PostId LEFT JOIN PostHistories ph ON rp.Id = ph.PostId WHERE rp.Rank <= 5)
// SELECT cd.PostId, cd.Title, cd.CreationDate, cd.ViewCount, cd.VoteScore, cd.EditCount, cd.UniqueEditorsCount, cd.PostStatus,
//        CASE WHEN cd.VoteScore > 0 THEN 'Positive' WHEN cd.VoteScore < 0 THEN 'Negative' ELSE 'Neutral' END AS VoteSentiment
// FROM CombinedData cd WHERE cd.VoteScore >= (SELECT AVG(VoteScore) FROM PostVotes) ORDER BY cd.ViewCount DESC, cd.VoteScore DESC, cd.EditCount DESC;
//
// PostVotes groups on Votes.PostId, raw ids included. ARRAY_AGG(DISTINCT ...) keeps a NULL as one element, so the length counts it.
fn q24258(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, answer_count, .. } = &db.post;
    let v = drain(db.post.with(creation_date.ge(add_days(date(2024, 10, 1), -30))).select(post_type_id));
    let tp = set_of(top_per(v, |&(_, t)| t, |&(p, _)| (Reverse(score.get(p).unwrap()), p), 5, false).into_iter().map(|x| x.0).collect());
    let Vote { post_id, vote_type_id, .. } = &db.vote;
    let pv = db.vote.group_by(post_id).select(vote_type_id).fold(0i64, |n, t| n + (t == 2) as i64 - (t == 3) as i64);
    let (sum, n) = (&pv).fold_flat((0i128, 0i128), |(s, n), x| (s + x as i128, n + 1));
    let PostHistory { post_history_type_id, user_display_name, .. } = &db.post_history;
    let ph = db.post_history.with(post_history_type_id.is_in([4, 5])).group_by(&db.post_history.post).select(user_display_name.opt()).buf_fold(|v| {
        let mut x: Vec<Option<Str>> = v.to_vec();
        x.sort();
        x.dedup();
        (v.len() as i64, x.len() as i64)
    });
    let v = drain((&tp).select((&db.post.origid).select((&pv).opt()).opt().and((&ph).opt())));
    let v: Vec<(Id<Post>, (i64, Option<(i64, i64)>))> = v.into_iter().map(|(p, (s, h))| (p, (s.flatten().unwrap_or(0), h))).collect();
    let v = drain(rel(v).filt(move |(_, (s, _)): (Id<Post>, (i64, Option<(i64, i64)>))| s as i128 * n >= sum));
    rows(v.into_iter().map(|(_, (p, (s, h)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views"]);
        f.extend([V::I(s), V::I(h.map_or(0, |h| h.0)), oint(h.map(|h| h.1))]);
        f.push(V::S(if answer_count.get(p).map_or(false, |a| a > 0) { "Answered" } else { "Unanswered" }));
        f.push(V::S(if s > 0 { "Positive" } else if s < 0 { "Negative" } else { "Neutral" }));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostID, p.Title, p.CreationDate, p.PostTypeId, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS PostRank
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.CreationDate >= (TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year') GROUP BY p.Id, p.Title, p.CreationDate, p.PostTypeId, p.Score),
// TopPosts AS (SELECT rp.PostID, rp.Title, rp.CommentCount, (rp.UpVotes - rp.DownVotes) AS NetVotes, rp.PostTypeId, CASE WHEN rp.PostRank <= 10 THEN 'Top 10' WHEN rp.PostRank <= 50 THEN 'Top 50' ELSE 'Other' END AS RankGroup
//     FROM RankedPosts rp WHERE rp.CommentCount > 0),
// AggBadges AS (SELECT b.UserId, COUNT(b.Id) AS BadgeCount, STRING_AGG(b.Name, ', ') AS BadgeList FROM Badges b WHERE b.Date >= (TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year') GROUP BY b.UserId),
// PostDetails AS (SELECT p.Title, t.TagName, COALESCE(ab.BadgeCount, 0) AS UserBadgeCount, COALESCE(ab.BadgeList, 'No Badges') AS UserBadges
//     FROM TopPosts p JOIN Tags t ON t.WikiPostId = p.PostID LEFT JOIN AggBadges ab ON p.PostID = ab.UserId)
// SELECT pd.Title, pd.TagName, pd.UserBadgeCount, pd.UserBadges, CASE WHEN pd.UserBadgeCount > 5 THEN 'Veteran User' WHEN pd.UserBadgeCount > 0 THEN 'Active Contributor' ELSE 'New User' END AS UserStatus
// FROM PostDetails pd WHERE pd.UserBadgeCount IS NOT NULL AND pd.UserBadgeCount < 10 ORDER BY pd.UserBadgeCount DESC, pd.Title LIMIT 100;
//
// Only CommentCount of RankedPosts is read, and it is positive exactly when the post has a comment. The post's Id is joined to the badge's UserId, as written.
fn q21866(db: &'static So) -> String {
    let Post { creation_date, origid, title, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let tp = || db.post.with(creation_date.ge(add_years(t0, -1))).with(comments_of(db));
    let wiki: HashIdx<Id<Post>, Id<Tag>> = (&db.tag.wiki_post).inv().collect();
    let ab = db.badge.with((&db.badge.date).ge(add_years(t0, -1))).group_by(&db.badge.user_id).select(&db.badge.name).buf_fold(|v| -> (i64, Str) { (v.len() as i64, Box::leak(v.to_vec().join(", ").into_boxed_str())) });
    type J = (Id<Tag>, Option<(i64, Str)>);
    let v = drain(tp().select((&wiki).and(origid.select(&ab).opt())).filt(|(_, b): J| b.map_or(0, |b| b.0) < 10));
    let v = top_n(v, |&(p, (t, b))| (Reverse(b.map_or(0, |b| b.0)), title.get(p), p, t), 100);
    rows(v.into_iter().map(|(p, (t, b))| {
        let n = b.map_or(0, |b| b.0);
        let mut f = post_fields(db, p, &["title"]);
        f.extend([V::S(db.tag.tag_name.get(t).unwrap()), V::I(n), V::S(b.map_or("No Badges", |b| b.1))]);
        f.push(V::S(if n > 5 { "Veteran User" } else if n > 0 { "Active Contributor" } else { "New User" }));
        row(f)
    }))
}

// WITH RankedUsers AS (SELECT u.Id, u.DisplayName, u.Reputation, RANK() OVER (ORDER BY u.Reputation DESC) AS UserRank, COUNT(DISTINCT p.Id) AS PostCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY u.Id, u.DisplayName, u.Reputation),
// ActivePosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, COALESCE(p.AcceptedAnswerId, 0) AS AcceptedAnswerId, COUNT(c.Id) AS CommentCount, SUM(v.BountyAmount) AS TotalBounties
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 8 WHERE p.LastActivityDate >= CURRENT_TIMESTAMP - INTERVAL '30 days'
//     GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.AcceptedAnswerId),
// PostDetails AS (SELECT ap.PostId, ap.Title, ap.CreationDate, ap.ViewCount, ap.CommentCount, ap.TotalBounties, CASE WHEN ap.AcceptedAnswerId = 0 THEN 'No Answer Accepted' ELSE 'Answer Accepted' END AS AnswerStatus FROM ActivePosts ap)
// SELECT ru.DisplayName, ru.Reputation, ru.UserRank, pd.Title, pd.CreationDate, pd.ViewCount, pd.CommentCount, pd.TotalBounties, pd.AnswerStatus, STRING_AGG(DISTINCT pt.Name, ', ') AS PostTypeNames
// FROM RankedUsers ru JOIN PostDetails pd ON ru.Id = pd.PostId LEFT JOIN PostTypes pt ON pd.CommentCount > 0 AND pd.TotalBounties > 0
// GROUP BY ru.Id, ru.DisplayName, ru.Reputation, ru.UserRank, pd.PostId, pd.Title, pd.CreationDate, pd.ViewCount, pd.CommentCount, pd.TotalBounties, pd.AnswerStatus ORDER BY ru.UserRank, pd.ViewCount DESC LIMIT 100;
//
// The user's Id is joined to the post's Id, as written; RankedUsers' counts are never read. LastActivityDate is compared with CURRENT_TIMESTAMP as an instant in the session zone.
// `ON pd.CommentCount > 0 AND pd.TotalBounties > 0` names only pd: when it holds, the group sees every post type.
fn q1325(db: &'static So) -> String {
    let Post { last_activity_date, view_count, accepted_answer_id, .. } = &db.post;
    let since = now_utc() - 30 * DAY_US;
    let ap = db
        .post
        .with(last_activity_date.filt(move |d| ny_to_utc(d) >= since))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt()).opt()))
        .fold([0i64; 3], |a, (c, b)| {
            let b = b.flatten();
            [a[0] + c.is_some() as i64, a[1] + b.is_some() as i64, a[2] + b.unwrap_or(0)]
        });
    let r = ranked(drain(&db.user.reputation), |&(_, r)| Reverse(r), false);
    let ur = rel(r.into_iter().map(|((u, _), k)| (u, k)).collect());
    let urk: HashIdx<Id<User>, i64> = (&ur).map(|x: (Id<User>, i64)| x.0).inv().select((&ur).map(|x: (Id<User>, i64)| x.1)).collect();
    let pidx: HashIdx<i64, Id<Post>> = (&db.post.origid).inv().collect();
    let names = {
        let mut n: Vec<Str> = drain(&db.post_type.name).into_iter().map(|x| x.1).collect();
        n.sort();
        n.dedup();
        let s: Str = Box::leak(n.join(", ").into_boxed_str());
        s
    };
    let v = drain(db.user.select((&urk).and((&db.user.origid).select(&pidx).select(Ident::<Post>::new().and(&ap)))));
    let v = top_n(v, |&(u, (k, (p, _)))| (k, Reverse(view_count.get(p)), u, p), 100);
    rows(v.into_iter().map(|(u, (k, (p, a)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(k));
        f.extend(post_fields(db, p, &["title", "created", "views"]));
        f.extend([V::I(a[0]), nullable(a[2], a[1]), V::S(if accepted_answer_id.get(p).unwrap_or(0) == 0 { "No Answer Accepted" } else { "Answer Accepted" })]);
        f.push(if a[0] > 0 && a[1] > 0 && a[2] > 0 { V::S(names) } else { V::Null });
        row(f)
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("29335", q29335),
    ("6292", q6292),
    ("9925", q9925),
    ("9476", q9476),
    ("2863", q2863),
    ("27731", q27731),
    ("29581", q29581),
    ("7811", q7811),
    ("4999", q4999),
    ("22222", q22222),
    ("1625", q1625),
    ("21867", q21867),
    ("2827", q2827),
    ("25526", q25526),
    ("4718", q4718),
    ("9285", q9285),
    ("5514", q5514),
    ("4114", q4114),
    ("1997", q1997),
    ("22019", q22019),
    ("2236", q2236),
    ("1596", q1596),
    ("33391", q33391),
    ("21549", q21549),
    ("22890", q22890),
    ("25706", q25706),
    ("4698", q4698),
    ("4685", q4685),
    ("33015", q33015),
    ("29400", q29400),
    ("2227", q2227),
    ("20596", q20596),
    ("9382", q9382),
    ("23319", q23319),
    ("21643", q21643),
    ("22458", q22458),
    ("31287", q31287),
    ("31504", q31504),
    ("29167", q29167),
    ("30249", q30249),
    ("27623", q27623),
    ("21567", q21567),
    ("32254", q32254),
    ("22653", q22653),
    ("23627", q23627),
    ("2215", q2215),
    ("20663", q20663),
    ("21655", q21655),
    ("23710", q23710),
    ("22275", q22275),
    ("392", q392),
    ("25521", q25521),
    ("1370", q1370),
    ("20344", q20344),
    ("22153", q22153),
    ("23791", q23791),
    ("21437", q21437),
    ("4061", q4061),
    ("21883", q21883),
    ("1494", q1494),
    ("20981", q20981),
    ("20810", q20810),
    ("453", q453),
    ("29102", q29102),
    ("24258", q24258),
    ("21866", q21866),
    ("1325", q1325),
];
