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
];
