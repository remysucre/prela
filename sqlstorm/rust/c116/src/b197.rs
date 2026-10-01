use harness::prelude::*;
use std::cmp::Reverse;

fn leak(s: String) -> Str {
    Box::leak(s.into_boxed_str())
}

fn distinct_join(mut v: Vec<Str>, sep: &str) -> Str {
    v.sort();
    v.dedup();
    leak(v.join(sep))
}

fn cross_top<A: Copy, B: Copy, KA: Ord + Copy, KB: Ord>(a: Vec<A>, ka: impl Fn(&A) -> KA, b: Vec<B>, kb: impl Fn(&B) -> KB, n: usize) -> Vec<(A, B)> {
    let ra = rel(a);
    let rb = rel(b);
    let ranked = (&ra).map(|_| ()).inv().select(&ra).window(rank, |x: A| ka(&x), asc);
    let nb = count(&rb).max(1);
    let picked = (&ranked).filt(move |(_, r)| (r - 1) * nb < n as i64).map(|(x, _)| x);
    top_n(drain(picked.cross(&rb)), |(_, (x, y))| (ka(x), kb(y)), n).into_iter().map(|x| x.1).collect()
}

fn post_tags(db: &'static So) -> &'static HashIdx<Id<Post>, Id<Tag>> {
    static CACHE: std::sync::OnceLock<HashIdx<Id<Post>, Id<Tag>>> = std::sync::OnceLock::new();
    CACHE.get_or_init(|| (&tag_mentions(db)).map(|(p, _)| p).inv().map(|(_, t)| t).collect())
}

fn tag_posts(db: &'static So) -> &'static HashIdx<Id<Tag>, Id<Post>> {
    static CACHE: std::sync::OnceLock<HashIdx<Id<Tag>, Id<Post>>> = std::sync::OnceLock::new();
    CACHE.get_or_init(|| post_tags(db).inv().collect())
}

// SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount, COUNT(v.Id) AS VoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, p.AnswerCount, p.FavoriteCount, t.TagName
// FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
// LEFT JOIN Tags t ON p.Tags LIKE CONCAT('%', t.TagName, '%')
// WHERE p.PostTypeId = 1 GROUP BY p.Id, u.DisplayName, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, p.FavoriteCount, t.TagName
// ORDER BY p.CreationDate DESC, p.Id, t.TagName LIMIT 100;
//
// Ported from rewrites/13831.sql (`, p.Id, t.TagName` added: the cut falls inside one post's tags). The ORDER BY reads only the post and the tag,
// so the hundred (post, tag) rows are picked first and the comments x votes product is driven only for their posts.
fn q13831(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, origid, .. } = &db.post;
    let pt = post_tags(db);
    let v = drain(db.post.with(post_type_id.eq(1)).with(owner_user).select(Ident::<Post>::new().and((&pt).select(&db.tag.tag_name).opt())));
    let top = top_n(v, |&(_, (p, t))| (Reverse(creation_date.get(p).unwrap()), origid.get(p).unwrap(), t.is_none(), t), 100);
    type R = (Id<Post>, Option<Str>);
    let r = rel(top.into_iter().map(|x| x.1).collect());
    let tp: MatSet<Id<Post>> = (&r).map(|x: R| x.0).collect();
    let e = engagement(db, &tp);
    let v = drain((&r).select(Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0).select(&e))));
    rows(v.into_iter().map(|(_, ((p, t), (c, n, u, d)))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "owner"]);
        f.extend([V::I(c), V::I(n), V::I(u), V::I(d)]);
        f.extend(post_fields(db, p, &["answers", "favorites"]));
        f.push(ostr(t));
        row(f)
    }))
}

// WITH UserPostCounts AS (SELECT u.Id AS UserId, COUNT(p.Id) AS PostCount, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts,
//        SUM(CASE WHEN p.Score < 0 THEN 1 ELSE 0 END) AS NegativePosts, SUM(p.ViewCount) AS TotalViews
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId GROUP BY u.Id),
// PostStatistics AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, pt.Name AS PostTypeName, COUNT(c.Id) AS CommentCount
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id LEFT JOIN Comments c ON p.Id = c.PostId GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, pt.Name)
// SELECT up.UserId, up.PostCount, up.PositivePosts, up.NegativePosts, up.TotalViews, ps.PostId, ps.Title, ps.CreationDate, ps.Score, ps.ViewCount, ps.PostTypeName, ps.CommentCount
// FROM UserPostCounts up JOIN PostStatistics ps ON up.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = ps.PostId LIMIT 1)
// ORDER BY up.TotalViews DESC, up.PostCount DESC;
fn q14857(db: &'static So) -> String {
    let Post { score, view_count, owner_user, .. } = &db.post;
    let up = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(score.and(view_count.opt())).opt()).fold([0i64; 5], |a, p| match p {
        Some((s, v)) => [a[0] + 1, a[1] + (s > 0) as i64, a[2] + (s < 0) as i64, a[3] + v.unwrap_or(0), a[4] + v.is_some() as i64],
        None => a,
    });
    let cpp = comments_per_post(db);
    let v = drain(db.post.select(Ident::<Post>::new().and(&cpp).and(owner_user.select(Ident::<User>::new().and(&up)))));
    rows(v.into_iter().map(|(_, ((p, c), (u, a)))| {
        let mut f = ucols(db, u, &["uid"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), nullable(a[3], a[4])]);
        f.extend(post_fields(db, p, &["id", "title", "created", "score", "views", "type"]));
        f.push(V::I(c));
        row(f)
    }))
}

// WITH UserReputation AS (SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.LastAccessDate,
//        CASE WHEN U.Reputation >= 1000 THEN 'Gold' WHEN U.Reputation BETWEEN 500 AND 999 THEN 'Silver' ELSE 'Bronze' END AS ReputationTier FROM Users U),
// PostSummary AS (SELECT P.Id AS PostId, P.Title, P.ViewCount, P.AnswerCount, P.CreationDate, U.DisplayName AS OwnerDisplayName, COALESCE(PH.RevisionCount, 0) AS RevisionCount
//     FROM Posts P LEFT JOIN (SELECT PH.PostId, COUNT(*) AS RevisionCount FROM PostHistory PH GROUP BY PH.PostId) PH ON PH.PostId = P.Id JOIN Users U ON P.OwnerUserId = U.Id),
// TopPosts AS (SELECT PS.PostId, PS.Title, PS.ViewCount, PS.AnswerCount, PS.CreationDate, PS.OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY PS.OwnerDisplayName ORDER BY PS.ViewCount DESC) AS Rank
//     FROM PostSummary PS WHERE PS.ViewCount IS NOT NULL AND PS.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'))
// SELECT UP.DisplayName, UP.Reputation, TP.Title, TP.ViewCount, TP.AnswerCount, TP.CreationDate, UP.ReputationTier,
//        CASE WHEN TP.AnswerCount = 0 THEN 'No Answers Yet' ELSE 'Has Answers' END AS AnswerStatus
// FROM UserReputation UP JOIN TopPosts TP ON UP.UserId = (SELECT P.OwnerUserId FROM Posts P WHERE P.Id = TP.PostId)
// WHERE UP.Reputation >= (SELECT AVG(Reputation) FROM UserReputation) ORDER BY UP.Reputation DESC, TP.ViewCount DESC LIMIT 10;
//
// Rank and RevisionCount are never read.
fn q24751(db: &'static So) -> String {
    let Post { view_count, creation_date, owner_user, answer_count, .. } = &db.post;
    let reputation = &db.user.reputation;
    let (s, n) = reputation.fold_flat((0i64, 0i64), |(s, n), r| (s + r, n + 1));
    let users = Ident::<User>::new().with(reputation.filt(move |r| r * n >= s)).and(reputation);
    let v = drain(
        db.post
            .with(view_count)
            .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
            .select(Ident::<Post>::new().and(view_count).and(owner_user.select(users))),
    );
    let v = top_n(v, |&(_, ((_, w), (_, r)))| (Reverse(r), Reverse(w)), 10);
    rows(v.into_iter().map(|(_, ((p, _), (u, r)))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["title", "views", "answers", "created"]));
        let tier = if r >= 1000 { "Gold" } else if r >= 500 { "Silver" } else { "Bronze" };
        f.extend([V::S(tier), V::S(if answer_count.get(p) == Some(0) { "No Answers Yet" } else { "Has Answers" })]);
        row(f)
    }))
}

// WITH TagCounts AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT S.Tag) AS TagCount, SUM(p.ViewCount) AS TotalViews
//     FROM Users U JOIN Posts p ON U.Id = p.OwnerUserId
//     LEFT JOIN LATERAL (SELECT unnest(string_to_array(substring(p.Tags, 2, length(p.Tags) - 2), '><')) AS Tag) AS S ON TRUE
//     WHERE p.PostTypeId = 1 GROUP BY U.Id, U.DisplayName),
// TopUsers AS (SELECT UserId, DisplayName, TagCount, TotalViews, RANK() OVER (ORDER BY TotalViews DESC) AS ViewRank FROM TagCounts WHERE TagCount > 0),
// MostActiveTags AS (SELECT T.TagName, COUNT(P.Id) AS PostCount, SUM(P.ViewCount) AS TotalViews FROM Tags T JOIN Posts P ON P.Tags LIKE CONCAT('%', T.TagName, '%')
//     WHERE P.PostTypeId = 1 GROUP BY T.TagName ORDER BY TotalViews DESC LIMIT 10)
// SELECT U.DisplayName AS TopUser, U.TagCount, U.TotalViews, T.TagName AS MostActiveTag, T.PostCount, T.TotalViews AS TagTotalViews
// FROM TopUsers U JOIN MostActiveTags T ON TRUE WHERE U.ViewRank <= 5 ORDER BY U.TotalViews DESC, T.TotalViews DESC;
fn q26109(db: &'static So) -> String {
    let Post { post_type_id, tags_str, view_count, .. } = &db.post;
    let q = || Ident::<Post>::new().with(post_type_id.eq(1));
    let tc = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(q().select(view_count.opt().and(tags_str.flat_map(tag_list).opt()))))
        .buf_fold(|v| {
            let mut t: Vec<Str> = v.iter().filter_map(|x| x.1).collect();
            t.sort();
            t.dedup();
            (t.len() as i64, v.iter().map(|x| x.0.unwrap_or(0)).sum::<i64>(), v.iter().filter(|x| x.0.is_some()).count() as i64)
        });
    let tu = ranked(drain((&tc).filt(|a: (i64, i64, i64)| a.0 > 0)), |&(_, (_, s, k))| (k > 0, Reverse(s)), false);
    let tu: Vec<(Id<User>, (i64, i64, i64))> = drain(rel(tu).filt(|x: ((Id<User>, (i64, i64, i64)), i64)| x.1 <= 5)).into_iter().map(|x| x.1 .0).collect();
    let mt = db.tag.group_by(Ident::<Tag>::new()).select(tag_posts(db).select(q().select(view_count.opt()))).fold((0i64, 0i64, 0i64), |(n, s, k), w| {
        (n + 1, s + w.unwrap_or(0), k + w.is_some() as i64)
    });
    let mt = top_n(drain(&mt), |&(_, (_, s, k))| (k > 0, Reverse(s)), 10);
    let v = drain(rel(tu).cross(&rel(mt)));
    rows(v.into_iter().map(|(_, ((u, (n, s, k)), (t, (tn, ts, tk))))| {
        let mut f = ucols(db, u, &["name"]);
        f.extend([V::I(n), nullable(s, k), V::S(db.tag.tag_name.get(t).unwrap()), V::I(tn), nullable(ts, tk)]);
        row(f)
    }))
}

// WITH RECURSIVE UserBadgeCount AS (SELECT U.Id AS UserId, COUNT(B.Id) AS BadgeCount FROM Users U LEFT JOIN Badges B ON U.Id = B.UserId GROUP BY U.Id),
// RecentPosts AS (SELECT P.Id AS PostId, P.OwnerUserId, P.Title, P.CreationDate, P.Score, P.ViewCount,
//        ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS RecentPostRank
//     FROM Posts P WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'),
// PopularTags AS (SELECT T.TagName, SUM(P.ViewCount) AS TotalViews, COUNT(P.Id) AS PostCount FROM Tags T JOIN Posts P ON P.Tags LIKE CONCAT('%', T.TagName, '%')
//     GROUP BY T.TagName HAVING COUNT(P.Id) > 10 ORDER BY TotalViews DESC LIMIT 10)
// SELECT U.DisplayName, U.Reputation, COALESCE(UB.BadgeCount, 0) AS BadgeCount, RP.Title AS RecentPostTitle, RP.CreationDate AS RecentPostDate, PT.TagName AS PopularTag,
//        PT.TotalViews AS TagTotalViews, CASE WHEN RP.Score IS NULL THEN 'No Score' ELSE CAST(RP.Score AS TEXT) END AS RecentPostScore
// FROM Users U LEFT JOIN UserBadgeCount UB ON U.Id = UB.UserId LEFT JOIN RecentPosts RP ON U.Id = RP.OwnerUserId AND RP.RecentPostRank = 1
// LEFT JOIN PopularTags PT ON RP.Title LIKE CONCAT('%', PT.TagName, '%')
// WHERE U.Reputation > 1000 ORDER BY U.Reputation DESC, BadgeCount DESC;
//
// Not recursive: no CTE refers to itself. A CreationDate tie inside an owner goes to the smaller post id (the SQL leaves it open).
fn q31512(db: &'static So) -> String {
    let Post { creation_date, owner_user_id, owner_user, view_count, title, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30)))
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (Reverse(d), p), asc);
    let rs: MatSet<Id<Post>> = (&w).filt(|(_, n): ((Id<Post>, i64), i64)| n == 1).map(|((p, _), _)| p).collect();
    let rp: HashIdx<Id<User>, Id<Post>> = (&rs).select(owner_user).inv().collect();
    let tv = db.tag.group_by(Ident::<Tag>::new()).select(tag_posts(db).select(view_count.opt())).fold((0i64, 0i64, 0i64), |(n, s, k), w| {
        (n + 1, s + w.unwrap_or(0), k + w.is_some() as i64)
    });
    let pt = drain(db.tag.select((&db.tag.tag_name).and((&tv).filt(|a: (i64, i64, i64)| a.0 > 10))));
    let pt = top_n(pt, |&(_, (_, (_, s, k)))| (k > 0, Reverse(s)), 10);
    type T = (Str, (i64, i64, i64));
    let r = rel(pt.into_iter().map(|x| x.1).collect());
    let by: HashIdx<Str, T> = (&r).map(|x: T| x.0).inv().select(&r).collect();
    let tm = title.select_where(&by, |t: Str, n: Str| t.contains(n));
    let v = drain(db.user.with((&db.user.reputation).gt(1000)).select(Ident::<User>::new().and(badges_per_user(db)).and((&rp).select(Ident::<Post>::new().and(tm.opt())).opt())));
    rows(v.into_iter().map(|(_, ((u, b), p))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.push(V::I(b));
        match p {
            Some((p, t)) => {
                f.extend(post_fields(db, p, &["title", "created"]));
                match t {
                    Some((n, (_, s, k))) => f.extend([V::S(n), nullable(s, k)]),
                    None => f.extend([V::Null, V::Null]),
                }
                f.push(V::Owned(db.post.score.get(p).unwrap().to_string()));
            }
            None => f.extend([V::Null, V::Null, V::Null, V::Null, V::S("No Score")]),
        }
        row(f)
    }))
}

// WITH PostStats AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COUNT(c.Id) AS CommentCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount,
//        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount, COALESCE(SUM(CASE WHEN v.VoteTypeId = 1 THEN 1 ELSE 0 END), 0) AS AcceptedCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' GROUP BY p.Id, p.Title, p.CreationDate),
// TopPosts AS (SELECT ps.PostId, ps.Title, ps.CreationDate, ps.CommentCount, ps.UpVoteCount, ps.DownVoteCount, ps.AcceptedCount,
//        ROW_NUMBER() OVER (ORDER BY ps.UpVoteCount - ps.DownVoteCount DESC, ps.AcceptedCount DESC) AS Rank FROM PostStats ps)
// SELECT tp.PostId, tp.Title, tp.CreationDate, tp.CommentCount, tp.UpVoteCount, tp.DownVoteCount, tp.AcceptedCount,
//        (SELECT STRING_AGG(DISTINCT t.TagName, ', ') FROM Tags t JOIN Posts p ON p.Tags LIKE CONCAT('%<', t.TagName, '>%') WHERE p.Id = tp.PostId) AS Tags
// FROM TopPosts tp WHERE tp.Rank <= 10 ORDER BY tp.Rank;
//
// Postgres sorts the input of a DISTINCT aggregate, so the STRING_AGG is in name order.
fn q7014(db: &'static So) -> String {
    let Post { creation_date, .. } = &db.post;
    let g = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 4], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64, a[3] + (t == Some(1)) as i64]);
    let top = top_n(drain(&g), |&(_, a)| (Reverse(a[1] - a[2]), Reverse(a[3])), 10);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let tg = (&tp).group_by(Ident::<Post>::new()).select((&db.post.tags).select(&db.tag.tag_name)).buf_fold(|v| distinct_join(v.to_vec(), ", "));
    let v = drain((&tp).select(Ident::<Post>::new().and(&g).and((&tg).opt())));
    rows(v.into_iter().map(|(_, ((p, a), t))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), ostr(t)]);
        row(f)
    }))
}

// WITH PostDetails AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.ViewCount, p.AnswerCount, u.DisplayName AS AuthorName, u.Reputation AS AuthorReputation,
//        ARRAY_AGG(DISTINCT t.TagName) AS TagsList
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN unnest(string_to_array(substring(p.Tags, 2, length(p.Tags) - 2), '> <')) AS t(TagName) ON TRUE
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.Body, p.CreationDate, p.ViewCount, p.AnswerCount, u.DisplayName, u.Reputation),
// TopPosts AS (SELECT PostId, Title, Body, CreationDate, ViewCount, AnswerCount, AuthorName, AuthorReputation, TagsList, ROW_NUMBER() OVER (ORDER BY ViewCount DESC) AS Rank FROM PostDetails),
// VoteSummary AS (SELECT PostId, COUNT(CASE WHEN vt.Name = 'UpMod' THEN 1 END) AS UpVotes, COUNT(CASE WHEN vt.Name = 'DownMod' THEN 1 END) AS DownVotes,
//        COUNT(CASE WHEN vt.Name = 'AcceptedByOriginator' THEN 1 END) AS AcceptedCount FROM Votes v JOIN VoteTypes vt ON v.VoteTypeId = vt.Id GROUP BY PostId)
// SELECT tp.PostId, tp.Title, tp.Body, tp.CreationDate, tp.ViewCount, tp.AnswerCount, tp.AuthorName, tp.AuthorReputation, tp.TagsList, vs.UpVotes, vs.DownVotes, vs.AcceptedCount
// FROM TopPosts tp JOIN VoteSummary vs ON tp.PostId = vs.PostId WHERE tp.Rank <= 10 ORDER BY tp.ViewCount DESC;
//
// Rank reads only base columns, so the ten questions are picked first. '> <' never occurs in Tags, so the one element is the whole inner string.
fn q26593(db: &'static So) -> String {
    let Post { post_type_id, owner_user, view_count, tags_str, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1)).with(owner_user).select(view_count.opt()));
    let top = top_n(v, |&(_, w)| (w.is_some(), Reverse(w)), 10);
    let tp: MatSet<Id<Post>> = rel(top.into_iter().map(|x| x.0).collect()).map(|p| p).collect();
    let tl = (&tp).group_by(Ident::<Post>::new()).select(tags_str.map(|t: Str| &t[1..t.len() - 1]).opt()).buf_fold(|v| {
        let mut v: Vec<Option<Str>> = v.to_vec();
        v.sort_by(|a, b| a.is_none().cmp(&b.is_none()).then(a.cmp(b)));
        v.dedup();
        let l: &'static [Option<Str>] = Box::leak(v.into_boxed_slice());
        l
    });
    let vs = (&tp).group_by(Ident::<Post>::new()).select(votes_of(db).select(vtype_name(db))).fold([0i64; 3], |a, n| {
        [a[0] + (n == "UpMod") as i64, a[1] + (n == "DownMod") as i64, a[2] + (n == "AcceptedByOriginator") as i64]
    });
    let v = drain((&tp).select(Ident::<Post>::new().and(&tl).and(&vs)));
    rows(v.into_iter().map(|(_, ((p, l), a))| {
        let mut f = post_fields(db, p, &["id", "title", "body", "created", "views", "answers", "owner", "rep"]);
        f.push(V::L(l.iter().map(|x| ostr(*x)).collect()));
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2])]);
        row(f)
    }))
}

// WITH PostStatistics AS (SELECT P.Id AS PostId, P.PostTypeId, P.CreationDate, P.Score, P.ViewCount, COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
//        COUNT(DISTINCT V.Id) AS VoteCount, COUNT(DISTINCT A.Id) AS AnswerCount
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN Votes V ON P.Id = V.PostId LEFT JOIN Posts A ON P.Id = A.ParentId AND P.PostTypeId = 1
//     GROUP BY P.Id, P.PostTypeId, P.CreationDate, P.Score, P.ViewCount),
// BenchmarkResults AS (SELECT PostTypeId, COUNT(PostId) AS TotalPosts, AVG(ViewCount) AS AverageViews, AVG(Score) AS AverageScore, AVG(CommentCount) AS AverageComments,
//        AVG(VoteCount) AS AverageVotes, AVG(AnswerCount) AS AverageAnswers FROM PostStatistics GROUP BY PostTypeId)
// SELECT PT.Name AS PostType, BR.TotalPosts, BR.AverageViews, BR.AverageScore, BR.AverageComments, BR.AverageVotes, BR.AverageAnswers
// FROM BenchmarkResults BR JOIN PostTypes PT ON BR.PostTypeId = PT.Id ORDER BY BR.TotalPosts DESC;
fn q10754(db: &'static So) -> String {
    let Post { post_type_id, post_type, score, view_count, .. } = &db.post;
    let ps = db
        .post
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).opt()).and(Ident::<Post>::new().with(post_type_id.eq(1)).select(children_of(db)).opt()))
        .buf_fold(|v| (v.iter().filter(|x| x.0 .0.is_some()).count() as i64, distinct_some(v.iter().map(|x| x.0 .1)), distinct_some(v.iter().map(|x| x.1))));
    let br = db
        .post
        .group_by(post_type)
        .select(view_count.opt().and(score).and(&ps))
        .fold([0i64; 7], |a, ((w, s), (c, v, n))| [a[0] + 1, a[1] + w.unwrap_or(0), a[2] + w.is_some() as i64, a[3] + s, a[4] + c, a[5] + v, a[6] + n]);
    let v = drain(db.post_type.select(Ident::<PostType>::new().and(&br)));
    rows(v.into_iter().map(|(_, (t, a))| {
        row(vec![V::S(db.post_type.name.get(t).unwrap()), V::I(a[0]), avg(a[1], a[2]), avg(a[3], a[0]), avg(a[4], a[0]), avg(a[5], a[0]), avg(a[6], a[0])])
    }))
}

// WITH UserStats AS (SELECT u.Id AS UserId, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, SUM(COALESCE(vb.VoteCount, 0)) AS TotalVotes,
//        SUM(COALESCE(CASE WHEN b.Id IS NOT NULL THEN 1 ELSE 0 END, 0)) AS TotalBadges, MAX(u.CreationDate) AS AccountCreationDate
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN (SELECT v.UserId, COUNT(*) AS VoteCount FROM Votes v GROUP BY v.UserId) vb ON u.Id = vb.UserId
//     LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation, u.CreationDate),
// PostStats AS (SELECT p.Id AS PostId, pt.Name AS PostType, COUNT(c.Id) AS CommentCount, SUM(COALESCE(CASE WHEN v.Id IS NOT NULL THEN 1 ELSE 0 END, 0)) AS TotalVotes,
//        SUM(COALESCE(p.FavoriteCount, 0)) AS TotalFavorites, MAX(p.LastActivityDate) AS LastActivityDate
//     FROM Posts p LEFT JOIN PostTypes pt ON p.PostTypeId = pt.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId GROUP BY p.Id, pt.Name)
// SELECT us.UserId, us.Reputation, us.PostCount, us.TotalVotes AS UserTotalVotes, us.TotalBadges, ps.PostId, ps.PostType, ps.CommentCount, ps.TotalVotes AS PostTotalVotes,
//        ps.TotalFavorites, us.AccountCreationDate
// FROM UserStats us JOIN PostStats ps ON us.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = ps.PostId LIMIT 1)
// ORDER BY us.Reputation DESC, ps.TotalVotes DESC;
//
// SUM over the bigint VoteCount is numeric in Postgres, so UserTotalVotes prints as a float.
fn q10518(db: &'static So) -> String {
    let Post { owner_user, favorite_count, .. } = &db.post;
    let vb = (&db.vote.user).inv().fold(0i64, |n, _| n + 1);
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).opt().and((&vb).opt()).and(badges_of(db).opt()))
        .fold([0i64; 2], |a, ((_, v), b)| [a[0] + v.unwrap_or(0), a[1] + b.is_some() as i64]);
    let ps = db
        .post
        .group_by(Ident::<Post>::new())
        .select(favorite_count.opt().and(comments_of(db).opt()).and(votes_of(db).opt()))
        .fold([0i64; 3], |a, ((f, c), v)| [a[0] + c.is_some() as i64, a[1] + v.is_some() as i64, a[2] + f.unwrap_or(0)]);
    let v = drain(db.post.select(Ident::<Post>::new().and(&ps).and(owner_user.select(Ident::<User>::new().and(&us).and(user_distinct_posts(db))))));
    rows(v.into_iter().map(|(_, ((p, b), ((u, a), n)))| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend([V::I(n), V::F(a[0] as f64), V::I(a[1])]);
        f.extend(post_fields(db, p, &["id", "type"]));
        f.extend([V::I(b[0]), V::I(b[1]), V::I(b[2])]);
        f.extend(ucols(db, u, &["ucreated"]));
        row(f)
    }))
}

// WITH PostAggregates AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes, AVG(v.BountyAmount) AS AvgBounty
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.PostTypeId IN (1, 2) GROUP BY p.Id, p.Title, p.CreationDate),
// VoteDetails AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UserUpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS UserDownVotes
//     FROM Users u JOIN Votes v ON u.Id = v.UserId GROUP BY u.Id, u.DisplayName)
// SELECT pa.PostId, pa.Title, pa.CreationDate, pa.CommentCount, pa.UpVotes, pa.DownVotes, pa.AvgBounty, vd.UserId, vd.DisplayName, vd.UserUpVotes, vd.UserDownVotes
// FROM PostAggregates pa JOIN VoteDetails vd ON pa.UpVotes > 0 OR pa.DownVotes > 0 ORDER BY pa.CreationDate DESC, pa.UpVotes DESC;
fn q5851(db: &'static So) -> String {
    let Post { creation_date, post_type_id, .. } = &db.post;
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let pa = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(post_type_id.is_in([1, 2]))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(vote_type_id.and(bounty_amount.opt())).opt()))
        .fold([0i64; 5], |a, (c, v)| {
            let t = v.map(|x| x.0);
            let b = v.and_then(|x| x.1);
            [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64, a[3] + b.unwrap_or(0), a[4] + b.is_some() as i64]
        });
    let vd = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(vote_type_id)).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let v = drain((&pa).filt(|a: [i64; 5]| a[1] > 0 || a[2] > 0).cross(&vd));
    rows(v.into_iter().map(|((p, u), (a, b))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[3], a[4])]);
        f.extend(ucols(db, u, &["uid", "name"]));
        f.extend([V::I(b[0]), V::I(b[1])]);
        row(f)
    }))
}

// WITH RecentPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days') GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName),
// TagStats AS (SELECT t.Id AS TagId, t.TagName, COUNT(pt.Id) AS PostCount FROM Tags t JOIN Posts pt ON pt.Tags LIKE CONCAT('%', t.TagName, '%') GROUP BY t.Id, t.TagName),
// TopTags AS (SELECT TagId, TagName FROM TagStats ORDER BY PostCount DESC LIMIT 5)
// SELECT rp.Id AS PostId, rp.Title, rp.OwnerDisplayName, rp.CreationDate, rp.ViewCount, rp.Score, rp.CommentCount, rp.UpVotes, rp.DownVotes, tt.TagId, tt.TagName
// FROM RecentPosts rp JOIN TopTags tt ON rp.Title LIKE CONCAT('%', tt.TagName, '%') ORDER BY rp.Score DESC, rp.CreationDate DESC, rp.Id, tt.TagId LIMIT 50;
//
// Ported from rewrites/7555.sql (`, rp.Id, tt.TagId` added: one post's title matches several top tags, and the cut falls among them).
fn q7555(db: &'static So) -> String {
    let Post { creation_date, owner_user, title, score, origid, .. } = &db.post;
    let ts5 = db.tag.group_by(Ident::<Tag>::new()).select(tag_posts(db)).fold(0i64, |n, _| n + 1);
    let tt = top_n(drain(&ts5), |&(_, n)| Reverse(n), 5);
    let r = rel(tt.into_iter().map(|x| x.0).collect());
    let by: HashIdx<Str, Id<Tag>> = (&r).select(&db.tag.tag_name).inv().select(&r).collect();
    let rp = db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).with(owner_user);
    let e = engagement(db, &rp);
    let v = drain(rp.select(Ident::<Post>::new().and(title.select_where(&by, |t: Str, n: Str| t.contains(n)))));
    let v = top_n(v, |&(_, (p, t))| (Reverse(score.get(p).unwrap()), Reverse(creation_date.get(p).unwrap()), origid.get(p).unwrap(), db.tag.origid.get(t).unwrap()), 50);
    type R = (Id<Post>, Id<Tag>);
    let v = drain(rel(v.into_iter().map(|x| x.1).collect()).select(Same::<R>::new().and(Same::<R>::new().map(|x: R| x.0).select(&e))));
    rows(v.into_iter().map(|(_, ((p, t), (c, _, u, d)))| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "views", "score"]);
        f.extend([V::I(c), V::I(u), V::I(d), V::I(db.tag.origid.get(t).unwrap()), V::S(db.tag.tag_name.get(t).unwrap())]);
        row(f)
    }))
}

// WITH TaggedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.Tags, pt.Name AS PostTypeName, u.DisplayName AS OwnerDisplayName, ts.TagName
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id JOIN Users u ON p.OwnerUserId = u.Id
//     LEFT JOIN LATERAL (SELECT unnest(string_to_array(substring(p.Tags, 2, length(p.Tags) - 2), '><')) AS tag) AS tags ON TRUE LEFT JOIN Tags ts ON ts.TagName = tags.tag
//     WHERE pt.Name = 'Question' AND p.CreationDate >= DATE '2022-01-01'),
// TopTagCounts AS (SELECT TagName, COUNT(PostId) AS PostCount FROM TaggedPosts GROUP BY TagName ORDER BY PostCount DESC LIMIT 5),
// PostDetails AS (SELECT tp.PostId, tp.Title, tp.Body, tp.OwnerDisplayName, tt.PostCount, COUNT(c.Id) AS CommentCount, MAX(ph.CreationDate) AS LastEditDate
//     FROM TaggedPosts tp JOIN TopTagCounts tt ON tp.TagName = tt.TagName LEFT JOIN Comments c ON tp.PostId = c.PostId LEFT JOIN PostHistory ph ON tp.PostId = ph.PostId
//     GROUP BY tp.PostId, tp.Title, tp.Body, tp.OwnerDisplayName, tt.PostCount)
// SELECT pd.Title, pd.Body, pd.OwnerDisplayName, pd.PostCount, pd.CommentCount, pd.LastEditDate FROM PostDetails pd WHERE pd.CommentCount > 0
// ORDER BY pd.PostCount DESC, pd.LastEditDate DESC;
fn q26906(db: &'static So) -> String {
    let Post { creation_date, owner_user, tags_str, .. } = &db.post;
    let names: HashIdx<Str, Id<Tag>> = (&db.tag.tag_name).inv().collect();
    let el = tags_str.flat_map(tag_list).select((&names).select(&db.tag.tag_name).opt()).opt();
    type R = (Id<Post>, Option<Str>);
    let tp: Vec<R> = drain(
        db.post
            .with(ptype_name(db).eq("Question"))
            .with(owner_user)
            .with(creation_date.ge(date(2022, 1, 1)))
            .select(Ident::<Post>::new().and(el.map(|t: Option<Option<Str>>| t.flatten()))),
    )
    .into_iter()
    .map(|x| x.1)
    .collect();
    let tp = rel(tp);
    let tc = (&tp).group_by(Same::<R>::new().map(|x: R| x.1)).fold(0i64, |n, _| n + 1);
    let tt = top_n(drain(&tc), |&(_, n)| Reverse(n), 5);
    let tt = rel(tt);
    let tt = rel(drain((&tt).filt(|(t, _): (Option<Str>, i64)| t.is_some())).into_iter().map(|x| (x.1 .0.unwrap(), x.1 .1)).collect());
    let tti: HashIdx<Str, i64> = (&tt).map(|(t, _)| t).inv().select(&tt).map(|(_, n)| n).collect();
    let pd = (&tp)
        .group_by(Same::<R>::new().map(|x: R| x.0).and(Same::<R>::new().flat_map(|x: R| x.1).select(&tti)))
        .select(Same::<R>::new().map(|x: R| x.0).select(comments_of(db).opt().and(history_of(db).select(&db.post_history.creation_date).opt())))
        .fold((0i64, i64::MIN), |(n, m), (c, d)| (n + c.is_some() as i64, m.max(d.unwrap_or(i64::MIN))));
    let v = drain((&pd).filt(|a: (i64, i64)| a.0 > 0));
    rows(v.into_iter().map(|((p, n), (c, m))| {
        let mut f = post_fields(db, p, &["title", "body", "owner"]);
        f.extend([V::I(n), V::I(c), tmax(m)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Body, p.Tags, COUNT(a.Id) AS AnswerCount, COUNT(c.Id) AS CommentCount,
//        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes, SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
//        ROW_NUMBER() OVER (PARTITION BY p.Tags ORDER BY COUNT(a.Id) DESC) AS Rank
//     FROM Posts p LEFT JOIN Posts a ON p.Id = a.ParentId AND p.PostTypeId = 1 LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.CreationDate, p.Body, p.Tags),
// PopularTags AS (SELECT Tags, COUNT(*) AS TagCount FROM Posts WHERE PostTypeId = 1 GROUP BY Tags ORDER BY TagCount DESC LIMIT 10)
// SELECT rp.PostId, rp.Title, rp.Tags, rp.AnswerCount, rp.CommentCount, rp.UpVotes, rp.DownVotes, rp.CreationDate, rt.TagCount
// FROM RankedPosts rp JOIN PopularTags rt ON rp.Tags = rt.Tags WHERE rp.Rank <= 5 ORDER BY rt.TagCount DESC, rp.AnswerCount DESC LIMIT 100;
//
// The window is partitioned by Tags, the join key, so the product is driven only for the questions with a popular Tags string.
fn q27645(db: &'static So) -> String {
    let Post { post_type_id, tags_str, origid, .. } = &db.post;
    let q = db.post.with(post_type_id.eq(1));
    let tc = (&q).group_by(tags_str.opt()).fold(0i64, |n, _| n + 1);
    let pt = top_n(drain(&tc), |&(_, n)| Reverse(n), 10);
    let pt = rel(pt);
    let pti: HashIdx<Option<Str>, i64> = (&pt).map(|(t, _)| t).inv().select(&pt).map(|(_, n)| n).collect();
    let g = (&q)
        .with(tags_str.opt().select(&pti))
        .group_by(Ident::<Post>::new())
        .select(children_of(db).opt().and(comments_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 4], |a, ((x, c), t)| [a[0] + x.is_some() as i64, a[1] + c.is_some() as i64, a[2] + (t == Some(2)) as i64, a[3] + (t == Some(3)) as i64]);
    let w = q
        .with(tags_str.opt().select(&pti))
        .group_by(tags_str.opt())
        .select(Ident::<Post>::new().and(origid).and(&g))
        .window(row_number, |((_, o), a)| (Reverse(a[0]), o), asc);
    let v = drain((&w).filt(|(_, n): (((Id<Post>, i64), [i64; 4]), i64)| n <= 5).and(&pti));
    let v = top_n(v, |&(_, ((((_, _), a), _), n))| (Reverse(n), Reverse(a[0])), 100);
    rows(v.into_iter().map(|(_, ((((p, _), a), _), n))| {
        let mut f = post_fields(db, p, &["id", "title", "tags"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3])]);
        f.extend(post_fields(db, p, &["created"]));
        f.push(V::I(n));
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, COALESCE(upvote_count, 0) AS UpVotes, COALESCE(downvote_count, 0) AS DownVotes,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS Rank
//     FROM Posts p LEFT JOIN (SELECT PostId, COUNT(*) AS upvote_count FROM Votes WHERE VoteTypeId = 2 GROUP BY PostId) AS upvotes ON p.Id = upvotes.PostId
//     LEFT JOIN (SELECT PostId, COUNT(*) AS downvote_count FROM Votes WHERE VoteTypeId = 3 GROUP BY PostId) AS downvotes ON p.Id = downvotes.PostId),
// FilteredPosts AS (SELECT rp.Id, rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.UpVotes, rp.DownVotes FROM RankedPosts rp WHERE rp.Rank <= 3),
// UserBadges AS (SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount, MAX(b.Class) AS HighestBadgeClass FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id)
// SELECT fp.Title, fp.CreationDate, fp.ViewCount, fp.Score, fp.UpVotes, fp.DownVotes, ub.BadgeCount, ub.HighestBadgeClass
// FROM FilteredPosts fp JOIN Users u ON u.Id = (SELECT OwnerUserId FROM Posts WHERE Id = fp.Id) LEFT JOIN UserBadges ub ON u.Id = ub.UserId
// WHERE fp.Score > (SELECT AVG(Score) FROM Posts) AND (fp.UpVotes - fp.DownVotes) > 5 ORDER BY fp.Score DESC LIMIT 10;
//
// Rank reads only base columns, so the posts are ranked first; a Score tie inside an owner goes to the smaller post id (the SQL leaves it open).
fn q3518(db: &'static So) -> String {
    let Post { owner_user_id, owner_user, score, origid, .. } = &db.post;
    let (s, n) = score.fold_flat((0i64, 0i64), |(s, n), x| (s + x, n + 1));
    let w = db.post.group_by(owner_user_id.opt()).select(Ident::<Post>::new().and(score)).window(row_number, |(p, s)| (Reverse(s), origid.get(p).unwrap()), asc);
    let fp: MatSet<Id<Post>> = (&w).filt(|(_, r): ((Id<Post>, i64), i64)| r <= 3).map(|((p, _), _)| p).collect();
    let up = votes_of_type(db, 2);
    let down = votes_of_type(db, 3);
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold((0i64, None::<i64>), |(n, m), c| (n + c.is_some() as i64, m.max(c)));
    let v = drain(
        (&fp)
            .with(score.filt(move |x| x * n > s))
            .select(Ident::<Post>::new().and(score).and(&up).and(&down).and(owner_user.select(&ub)))
            .filt(|((((_, _), u), d), _)| u - d > 5),
    );
    let v = top_n(v, |&(_, ((((_, s), _), _), _))| Reverse(s), 10);
    rows(v.into_iter().map(|(_, ((((p, _), u), d), (n, m)))| {
        let mut f = post_fields(db, p, &["title", "created", "views", "score"]);
        f.extend([V::I(u), V::I(d), V::I(n), oint(m)]);
        row(f)
    }))
}

// WITH PostStats AS (SELECT p.Id AS PostId, p.Title, COUNT(c.Id) AS CommentCount, SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount,
//        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS UserPostRow
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId WHERE p.PostTypeId IN (1, 2) GROUP BY p.Id, p.Title, p.OwnerUserId, p.CreationDate),
// RankedPosts AS (SELECT ps.PostId, ps.Title, ps.CommentCount, ps.UpvoteCount, ps.DownvoteCount, u.Reputation, u.DisplayName,
//        CASE WHEN ps.CommentCount > 10 THEN 'High Activity' WHEN ps.CommentCount BETWEEN 5 AND 10 THEN 'Moderate Activity' ELSE 'Low Activity' END AS ActivityLevel
//     FROM PostStats ps JOIN Users u ON ps.PostId IN (SELECT Id FROM Posts WHERE OwnerUserId = u.Id) WHERE ps.UserPostRow = 1),
// FinalResults AS (SELECT rp.*, COALESCE(rp.UpvoteCount - rp.DownvoteCount, 0) AS NetScore FROM RankedPosts rp)
// SELECT f.DisplayName, f.Title, f.CommentCount, f.UpvoteCount, f.DownvoteCount, f.NetScore, f.ActivityLevel
// FROM FinalResults f LEFT JOIN Badges b ON f.PostId = b.UserId WHERE b.Class = 1 ORDER BY f.NetScore DESC, f.CommentCount DESC LIMIT 20;
//
// UserPostRow reads only base columns, so each owner's newest post is picked first (a tie goes to the smaller id; the SQL leaves it open).
// `f.PostId = b.UserId` compares a post id with a user id, so it goes through the raw ids.
fn q686(db: &'static So) -> String {
    let Post { post_type_id, owner_user_id, owner_user, creation_date, origid, .. } = &db.post;
    let w = db
        .post
        .with(post_type_id.is_in([1, 2]))
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(creation_date))
        .window(row_number, |(p, d)| (Reverse(d), origid.get(p).unwrap()), asc);
    let gold: HashIdx<i64, Id<Badge>> = db.badge.with((&db.badge.class).eq(1)).select(&db.badge.user_id).inv().collect();
    let fp: MatSet<Id<Post>> = (&w).filt(|(_, r): ((Id<Post>, i64), i64)| r == 1).map(|((p, _), _)| p).with(owner_user).with(origid.select(&gold)).collect();
    let e = engagement(db, &fp);
    let v = drain((&fp).select(Ident::<Post>::new().and(&e).and(origid.select(&gold))));
    let v = top_n(v, |&(_, ((_, (c, _, u, d)), _))| (Reverse(u - d), Reverse(c)), 20);
    rows(v.into_iter().map(|(_, ((p, (c, _, u, d)), _))| {
        let mut f = post_fields(db, p, &["owner", "title"]);
        let lvl = if c > 10 { "High Activity" } else if c >= 5 { "Moderate Activity" } else { "Low Activity" };
        f.extend([V::I(c), V::I(u), V::I(d), V::I(u - d), V::S(lvl)]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT Id, DisplayName, Reputation FROM Users WHERE Reputation > 1000),
// PostStatistics AS (SELECT p.Id AS PostId, p.Title, p.Score, p.ViewCount, COUNT(c.Id) AS TotalComments, COUNT(v.Id) AS TotalVotes
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId
//     WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.PostTypeId IN (1, 2) GROUP BY p.Id, p.Title, p.Score, p.ViewCount),
// TagEngagement AS (SELECT t.TagName, COUNT(DISTINCT p.Id) AS PostsTagged, SUM(ps.ViewCount) AS TotalViews, AVG(ps.Score) AS AverageScore
//     FROM Tags t JOIN Posts p ON p.Tags LIKE CONCAT('%<', t.TagName, '>%') JOIN PostStatistics ps ON p.Id = ps.PostId GROUP BY t.TagName),
// BadgeStatistics AS (SELECT b.Name AS BadgeName, COUNT(DISTINCT b.UserId) AS TotalAwarded, COUNT(DISTINCT u.Id) AS UsersWithBadge
//     FROM Badges b JOIN Users u ON b.UserId = u.Id GROUP BY b.Name)
// SELECT ur.DisplayName, ur.Reputation, ps.Title, ps.Score, ps.TotalComments, ps.TotalVotes, te.TagName, te.PostsTagged, te.TotalViews, te.AverageScore,
//        bs.BadgeName, bs.TotalAwarded, bs.UsersWithBadge
// FROM UserReputation ur JOIN PostStatistics ps ON ur.Id = ps.PostId JOIN TagEngagement te ON te.PostsTagged > 0 JOIN BadgeStatistics bs ON bs.UsersWithBadge > 0
// ORDER BY ur.Reputation DESC, ps.Score DESC;
//
// `ur.Id = ps.PostId` compares a user id with a post id, so it goes through the raw ids; the comments x votes product is driven only for the posts it keeps.
fn q7188(db: &'static So) -> String {
    let Post { creation_date, post_type_id, origid, view_count, score, .. } = &db.post;
    let psm: MatSet<Id<Post>> = db.post.with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1))).with(post_type_id.is_in([1, 2])).collect();
    let ur: HashIdx<i64, Id<User>> = db.user.with((&db.user.reputation).gt(1000)).select(&db.user.origid).inv().collect();
    let up: MatSet<Id<Post>> = (&psm).with(origid.select(&ur)).collect();
    let e = engagement(db, &up);
    let a = drain((&up).select(Ident::<Post>::new().and(&e).and(origid.select(&ur))));
    let tagp: HashIdx<Id<Tag>, Id<Post>> = (&db.post.tags).inv().collect();
    let te = db.tag.group_by(Ident::<Tag>::new()).select((&tagp).select(Ident::<Post>::new().with(&psm).and(view_count.opt()).and(score))).buf_fold(|v| {
        let d = distinct_some(v.iter().map(|x| Some(x.0 .0)));
        let w: Vec<i64> = v.iter().filter_map(|x| x.0 .1).collect();
        (d, w.iter().sum::<i64>(), w.len() as i64, v.iter().map(|x| x.1).sum::<i64>(), v.len() as i64)
    });
    let te = drain((&te).filt(|a: (i64, i64, i64, i64, i64)| a.0 > 0));
    let bs = db.badge.group_by(&db.badge.name).select((&db.badge.user_id).and(&db.badge.user)).buf_fold(|v| {
        (distinct_some(v.iter().map(|x| Some(x.0))), distinct_some(v.iter().map(|x| Some(x.1))))
    });
    let bs = drain((&bs).filt(|a: (i64, i64)| a.1 > 0));
    let v = drain(rel(a).cross(&rel(te)).cross(&rel(bs)));
    rows(v.into_iter().map(|(_, (((_, ((p, (c, n, _, _)), u)), (t, (d, ws, wk, ss, sn))), (b, (ta, uw))))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend(post_fields(db, p, &["title", "score"]));
        f.extend([V::I(c), V::I(n), V::S(db.tag.tag_name.get(t).unwrap()), V::I(d), nullable(ws, wk), avg(ss, sn), V::S(b), V::I(ta), V::I(uw)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName AS OwnerDisplay,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.ViewCount DESC) AS Rank
//     FROM Posts p JOIN Users u ON p.OwnerUserId = u.Id
//     WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.Score > 0 AND p.PostTypeId IN (1, 2)),
// TopTags AS (SELECT t.TagName, COUNT(pt.Id) AS PostCount FROM Tags t JOIN Posts pt ON pt.Tags LIKE CONCAT('%<', t.TagName, '>%') GROUP BY t.TagName ORDER BY PostCount DESC LIMIT 10)
// SELECT rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.OwnerDisplay, tt.TagName, tt.PostCount
// FROM RankedPosts rp JOIN TopTags tt ON rp.PostId IN (SELECT PostId FROM PostLinks WHERE RelatedPostId = rp.PostId) WHERE rp.Rank <= 5 ORDER BY rp.Score DESC, rp.ViewCount DESC;
//
// A ViewCount tie inside a post type goes to the smaller post id (the SQL leaves it open). The IN asks for a link from the post to itself.
fn q7575(db: &'static So) -> String {
    let Post { creation_date, score, post_type_id, owner_user, origid, view_count, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .with(score.gt(0))
        .with(post_type_id.is_in([1, 2]))
        .with(owner_user)
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(origid).and(view_count.opt()))
        .window(row_number, |((_, o), w)| (w.is_some(), Reverse(w), o), asc);
    let rp: MatSet<Id<Post>> = (&w).filt(|(_, n): (((Id<Post>, i64), Option<i64>), i64)| n <= 5).map(|(((p, _), _), _)| p).collect();
    let PostLink { post_id, related_post_id, post, .. } = &db.post_link;
    let selfl: MatSet<Id<Post>> = db.post_link.with(post_id.and(related_post_id).filt(|(a, b)| a == b)).select(post).collect();
    let tagp: HashIdx<Id<Tag>, Id<Post>> = (&db.post.tags).inv().collect();
    let tc = db.tag.group_by(Ident::<Tag>::new()).select(&tagp).fold(0i64, |n, _| n + 1);
    let tt = top_n(drain(&tc), |&(_, n)| Reverse(n), 10);
    let v = drain((&rp).with(&selfl).cross(&rel(tt)));
    rows(v.into_iter().map(|((p, _), (_, (t, n)))| {
        let mut f = post_fields(db, p, &["title", "created", "views", "score", "owner"]);
        f.extend([V::S(db.tag.tag_name.get(t).unwrap()), V::I(n)]);
        row(f)
    }))
}

// WITH TagCounts AS (SELECT LOWER(TRIM(tag.tagName)) AS tag_name, COUNT(post.id) AS post_count FROM Tags AS tag LEFT JOIN Posts AS post ON post.Tags LIKE '%' || tag.TagName || '%'
//     GROUP BY LOWER(TRIM(tag.tagName))),
// UserBadges AS (SELECT u.Id AS user_id, COUNT(b.Id) AS badge_count FROM Users AS u LEFT JOIN Badges AS b ON b.UserId = u.Id GROUP BY u.Id),
// ActiveUsers AS (SELECT u.DisplayName AS user_name, u.Reputation, u.CreationDate, u.LastAccessDate, ub.badge_count FROM Users AS u JOIN UserBadges AS ub ON u.Id = ub.user_id
//     WHERE u.LastAccessDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' AND u.Reputation > 100),
// TopPosts AS (SELECT p.Title, p.ViewCount, p.AnswerCount, p.CommentCount, u.DisplayName AS author_name, p.CreationDate, ARRAY_AGG(DISTINCT LOWER(TRIM(tag.tagName))) AS tags
//     FROM Posts AS p JOIN Users AS u ON p.OwnerUserId = u.Id LEFT JOIN Tags AS tag ON p.Tags LIKE '%' || tag.TagName || '%'
//     WHERE p.PostTypeId = 1 GROUP BY p.Title, p.ViewCount, p.AnswerCount, p.CommentCount, u.DisplayName, p.CreationDate ORDER BY p.ViewCount DESC LIMIT 10)
// SELECT au.user_name, au.Reputation, au.badge_count, tp.Title AS post_title, tp.ViewCount, tp.AnswerCount, tp.CommentCount, tp.CreationDate, tc.post_count AS related_tags_count
// FROM ActiveUsers AS au JOIN TopPosts AS tp ON au.user_name = tp.author_name JOIN TagCounts AS tc ON tc.tag_name = ANY(tp.tags)
// ORDER BY au.Reputation DESC, tp.ViewCount DESC;
fn q28659(db: &'static So) -> String {
    let Post { post_type_id, owner_user, title, view_count, answer_count, comment_count, creation_date, .. } = &db.post;
    let ln: HashIdx<Id<Tag>, Str> = (&db.tag.tag_name).map(|n: Str| leak(n.trim_matches(' ').to_ascii_lowercase())).collect();
    let tc = db.tag.group_by(&ln).select(tag_posts(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let User { last_access_date, reputation, display_name, .. } = &db.user;
    let au: HashIdx<Str, Id<User>> =
        db.user.with(last_access_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -30))).with(reputation.gt(100)).select(display_name).inv().collect();
    let bpu = badges_per_user(db);
    let g = db
        .post
        .with(post_type_id.eq(1))
        .with(owner_user)
        .group_by(title.opt().and(view_count.opt()).and(answer_count.opt()).and(comment_count).and(owner_user.select(display_name)).and(creation_date))
        .select(post_tags(db).select(&ln).opt())
        .buf_fold(|v| {
            let mut v: Vec<Option<Str>> = v.to_vec();
            v.sort();
            v.dedup();
            let l: &'static [Option<Str>] = Box::leak(v.into_boxed_slice());
            l
        });
    let top = top_n(drain(&g), |&((((((_, w), _), _), _), _), _)| (w.is_some(), Reverse(w)), 10);
    type K = (((((Option<Str>, Option<i64>), Option<i64>), i64), Str), i64);
    type R = (K, &'static [Option<Str>]);
    let v = drain(rel(top).select(
        Same::<R>::new()
            .and(Same::<R>::new().map(|x: R| x.0 .0 .1).select(&au).select(Ident::<User>::new().and(&bpu)))
            .and(Same::<R>::new().flat_map(|x: R| x.1.iter().flatten().copied()).select(&tc)),
    ));
    rows(v.into_iter().map(|(_, ((((((((t, w), a), c), _), d), _), (u, b)), n))| {
        let mut f = ucols(db, u, &["name", "rep"]);
        f.extend([V::I(b), ostr(t), oint(w), oint(a), V::I(c), V::T(d), V::I(n)]);
        row(f)
    }))
}

// WITH UserReputation AS (SELECT Id AS UserId, Reputation, ROW_NUMBER() OVER (ORDER BY Reputation DESC) AS ReputationRank FROM Users),
// PostStatistics AS (SELECT P.Id AS PostId, P.Title, P.Score, COALESCE(SUM(V.BountyAmount), 0) AS TotalBounty, COUNT(DISTINCT C.Id) AS CommentCount,
//        COUNT(DISTINCT PH.Id) FILTER (WHERE PH.PostHistoryTypeId = 10) AS CloseCount, COUNT(DISTINCT PH.Id) FILTER (WHERE PH.PostHistoryTypeId = 11) AS ReopenCount
//     FROM Posts P LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId = 8 LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN PostHistory PH ON P.Id = PH.PostId
//     GROUP BY P.Id, P.Title, P.Score),
// HighScoredPosts AS (SELECT PS.PostId, PS.Title, PS.Score, PS.TotalBounty, PS.CommentCount, PS.CloseCount, PS.ReopenCount, UR.ReputationRank
//     FROM PostStatistics PS JOIN UserReputation UR ON PS.PostId IN (SELECT A.AcceptedAnswerId FROM Posts A WHERE A.PostTypeId = 1) WHERE PS.Score > 100)
// SELECT HSP.PostId, HSP.Title, HSP.Score, HSP.TotalBounty, HSP.CommentCount, HSP.CloseCount, HSP.ReopenCount, UR.DisplayName, UR.Reputation AS CreatorReputation
// FROM HighScoredPosts HSP JOIN Users UR ON UR.Id IN (SELECT P.OwnerUserId FROM Posts P WHERE P.Id = HSP.PostId)
// ORDER BY HSP.Score DESC, HSP.CommentCount DESC LIMIT 10;
//
// The join to UserReputation names only PS, so every high-scored accepted answer meets every user; the ORDER BY reads only the post, so the
// product with the users is driven only for the best-ranked posts (`cross_top`). ReputationRank is never read.
fn q1884(db: &'static So) -> String {
    let Post { post_type_id, accepted_answer, score, owner_user, .. } = &db.post;
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let ph_type = &db.post_history.post_history_type_id;
    let hs: MatSet<Id<Post>> = db.post.with(post_type_id.eq(1)).select(accepted_answer).with(score.gt(100)).collect();
    let ps = (&hs)
        .group_by(Ident::<Post>::new())
        .select(
            votes_of(db)
                .select(Ident::<Vote>::new().with(vote_type_id.eq(8)).select(bounty_amount.opt()))
                .opt()
                .and(comments_of(db).opt())
                .and(history_of(db).select(Ident::<PostHistory>::new().and(ph_type)).opt()),
        )
        .buf_fold(|v| {
            (
                v.iter().filter_map(|x| x.0 .0.flatten()).sum::<i64>(),
                distinct_some(v.iter().map(|x| x.0 .1)),
                distinct_some(v.iter().map(|x| x.1.filter(|h| h.1 == 10).map(|h| h.0))),
                distinct_some(v.iter().map(|x| x.1.filter(|h| h.1 == 11).map(|h| h.0))),
            )
        });
    let a = drain((&hs).select(Ident::<Post>::new().and(score).and(&ps).and(owner_user)));
    let us = drain((&db.user.reputation).map(|_| ()));
    let v = cross_top(a, |&(_, (((_, s), (_, c, _, _)), _))| (Reverse(s), Reverse(c)), us, |_| (), 10);
    rows(v.into_iter().map(|((_, (((p, _), (b, c, cl, ro)), u)), _)| {
        let mut f = post_fields(db, p, &["id", "title", "score"]);
        f.extend([V::I(b), V::I(c), V::I(cl), V::I(ro)]);
        f.extend(ucols(db, u, &["name", "rep"]));
        row(f)
    }))
}

// WITH UserVoteStats AS (SELECT U.Id AS UserId, U.DisplayName, SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes, SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
//     FROM Users U LEFT JOIN Votes V ON U.Id = V.UserId GROUP BY U.Id, U.DisplayName),
// PostScoreStats AS (SELECT P.Id AS PostId, P.Title, P.Score, COUNT(C.Id) AS CommentCount, COUNT(DISTINCT H.UserId) AS EditCount, MAX(H.CreationDate) AS LastEditDate
//     FROM Posts P LEFT JOIN Comments C ON P.Id = C.PostId LEFT JOIN PostHistory H ON P.Id = H.PostId GROUP BY P.Id, P.Title, P.Score),
// TopPosts AS (SELECT PS.PostId, PS.Title, PS.Score, PS.CommentCount, PS.LastEditDate, ROW_NUMBER() OVER (ORDER BY PS.Score DESC) AS PostRank FROM PostScoreStats PS WHERE PS.Score > 0),
// UserActivity AS (SELECT U.Id AS UserId, U.DisplayName, COUNT(DISTINCT P.Id) AS PostsCreated, COUNT(DISTINCT C.Id) AS CommentsMade, COUNT(DISTINCT H.Id) AS EditsMade
//     FROM Users U LEFT JOIN Posts P ON U.Id = P.OwnerUserId LEFT JOIN Comments C ON U.Id = C.UserId LEFT JOIN PostHistory H ON U.Id = H.UserId GROUP BY U.Id, U.DisplayName)
// SELECT UVS.UserId, UVS.DisplayName, UVS.Upvotes, UVS.Downvotes, TA.Title AS TopPostTitle, TA.Score AS TopPostScore, UA.PostsCreated, UA.CommentsMade, UA.EditsMade
// FROM UserVoteStats UVS JOIN UserActivity UA ON UVS.UserId = UA.UserId LEFT JOIN TopPosts TA ON UA.PostsCreated > 0
// WHERE UVS.Upvotes - UVS.Downvotes > 0 ORDER BY UVS.Upvotes DESC, UVS.Downvotes ASC;
//
// TopPosts is one row per post with Score > 0; only its Title and Score are read. UserActivity's distinct counts come from one fold per child table.
// The LEFT JOIN names only UA, so a user with posts meets every top post.
fn q6260(db: &'static So) -> String {
    let uvs = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
    });
    let cm = db.user.group_by(Ident::<User>::new()).select(comments_by(db).opt()).buf_fold(distinct_some);
    let hb: HashIdx<Id<User>, Id<PostHistory>> = (&db.post_history.user).inv().collect();
    let ed = db.user.group_by(Ident::<User>::new()).select((&hb).opt()).buf_fold(distinct_some);
    let udp = user_distinct_posts(db);
    let us = db.user.select(&uvs).filt(|a: [i64; 2]| a[0] - a[1] > 0).and((&udp).and(&cm).and(&ed));
    let tpi: HashIdx<(), Id<Post>> = whole(db.post.with((&db.post.score).gt(0))).collect();
    let v = drain(us.and(Ident::<User>::new().with((&udp).filt(|n: i64| n > 0)).map(|_| ()).select(&tpi).opt()));
    rows(v.into_iter().map(|(u, ((a, ((p, c), e)), t))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend([V::I(a[0]), V::I(a[1])]);
        match t {
            Some(t) => f.extend(post_fields(db, t, &["title", "score"])),
            None => f.extend([V::Null, V::Null]),
        }
        f.extend([V::I(p), V::I(c), V::I(e)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.ViewCount, p.Score, U.DisplayName AS OwnerDisplayName,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS RankByScore
//     FROM Posts p LEFT JOIN Users U ON p.OwnerUserId = U.Id WHERE p.PostTypeId = 1 AND p.CreationDate >= CURRENT_DATE - INTERVAL '1 year'),
// CloseReasons AS (SELECT ph.PostId, p.Title, ph.CreationDate, ph.Comment AS CloseReason FROM PostHistory ph INNER JOIN Posts p ON ph.PostId = p.Id WHERE ph.PostHistoryTypeId = 10),
// BadgeCounts AS (SELECT b.UserId, COUNT(*) AS BadgeCount FROM Badges b GROUP BY b.UserId),
// UserActivity AS (SELECT U.Id, U.DisplayName, U.Reputation, U.Views, COALESCE(bc.BadgeCount, 0) AS BadgeCount, RANK() OVER (ORDER BY U.Reputation DESC) AS ReputationRank
//     FROM Users U LEFT JOIN BadgeCounts bc ON U.Id = bc.UserId)
// SELECT RP.PostId, RP.Title, RP.CreationDate, RP.ViewCount, RP.Score, RP.OwnerDisplayName, CR.CloseReason, UA.DisplayName AS UserName, UA.Reputation, UA.BadgeCount,
//        UA.ReputationRank, (SELECT COUNT(*) FROM Comments c WHERE c.PostId = RP.PostId) AS CommentCount
// FROM RankedPosts RP LEFT JOIN CloseReasons CR ON RP.PostId = CR.PostId
// JOIN UserActivity UA ON UA.Id IN (SELECT A.OwnerUserId FROM Posts A WHERE A.PostTypeId = 2 AND A.AcceptedAnswerId = RP.PostId)
// WHERE RP.RankByScore <= 3 ORDER BY RP.Score DESC, UA.Reputation DESC;
//
// A Score tie inside an owner goes to the smaller post id (the SQL leaves it open).
fn q32386(db: &'static So) -> String {
    let Post { post_type_id, creation_date, owner_user_id, owner_user, accepted_answer, score, origid, .. } = &db.post;
    let w = db
        .post
        .with(post_type_id.eq(1))
        .with(creation_date.ge(add_years(current_date(), -1)))
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(origid).and(score))
        .window(row_number, |((_, o), s)| (Reverse(s), o), asc);
    let rp: MatSet<Id<Post>> = (&w).filt(|(_, n): (((Id<Post>, i64), i64), i64)| n <= 3).map(|(((p, _), _), _)| p).collect();
    let acc: MatSet<(Id<Post>, Id<User>)> = db.post.with(post_type_id.eq(2)).select(accepted_answer.and(owner_user)).collect();
    let acc: HashIdx<Id<Post>, Id<User>> = (&acc).map(|(p, _)| p).inv().map(|(_, u)| u).collect();
    let ua = ranked(drain(db.user.select(&db.user.reputation)), |&(_, r)| Reverse(r), false);
    let ua = rel(ua);
    let bpu = badges_per_user(db);
    let uai: HashIdx<Id<User>, (i64, i64)> = (&ua).map(|((u, _), _)| u).inv().select(&ua).map(|(_, k)| k).and(&bpu).collect();
    let PostHistory { post_history_type_id, comment, .. } = &db.post_history;
    let cr = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.eq(10)).select(comment.opt())).opt();
    let cpp = comments_per_post(db);
    let v = drain((&rp).select(Ident::<Post>::new().and(cr).and((&acc).select(Ident::<User>::new().and(&uai))).and(&cpp)));
    rows(v.into_iter().map(|(_, (((p, c), (u, (k, b))), n))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "owner"]);
        f.push(ostr(c.flatten()));
        f.extend(ucols(db, u, &["name", "rep"]));
        f.extend([V::I(b), V::I(k), V::I(n)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT P.Id AS PostId, P.Title, P.Body, P.CreationDate, P.ViewCount, U.DisplayName AS OwnerDisplayName, COUNT(CASE WHEN A.Id IS NOT NULL THEN 1 END) AS AnswerCount,
//        (SELECT COUNT(*) FROM Comments C WHERE C.PostId = P.Id) AS CommentCount, ROW_NUMBER() OVER (ORDER BY P.ViewCount DESC) AS ViewRank
//     FROM Posts P LEFT JOIN Posts A ON A.ParentId = P.Id AND A.PostTypeId = 2 JOIN Users U ON U.Id = P.OwnerUserId WHERE P.PostTypeId = 1
//     GROUP BY P.Id, P.Title, P.Body, P.CreationDate, P.ViewCount, U.DisplayName),
// TopQuestions AS (SELECT RP.PostId, RP.Title, RP.OwnerDisplayName, RP.ViewCount, RP.CreationDate FROM RankedPosts RP WHERE RP.ViewRank <= 10),
// TagStatistics AS (SELECT T.TagName, COUNT(*) AS PostsWithTag, AVG(P.ViewCount) AS AverageViews FROM Tags T JOIN Posts P ON P.Tags LIKE CONCAT('%', T.TagName, '%') GROUP BY T.TagName)
// SELECT T.TagName, T.PostsWithTag, T.AverageViews, COUNT(DISTINCT QQ.PostId) AS RelatedQuestions
// FROM TagStatistics T LEFT JOIN TopQuestions QQ ON QQ.Title ILIKE CONCAT('%', T.TagName, '%') GROUP BY T.TagName, T.PostsWithTag, T.AverageViews
// ORDER BY T.PostsWithTag DESC, T.AverageViews DESC;
//
// ViewRank reads only base columns, so the ten questions are picked first; AnswerCount and CommentCount are never read.
fn q26868(db: &'static So) -> String {
    let Post { post_type_id, owner_user, view_count, title, .. } = &db.post;
    let v = drain(db.post.with(post_type_id.eq(1)).with(owner_user).select(view_count.opt()));
    let top = top_n(v, |&(_, w)| (w.is_some(), Reverse(w)), 10);
    let tq = rel(top.into_iter().map(|x| x.0).collect());
    let tqi: HashIdx<Str, Id<Post>> = (&tq).select(title.map(|t: Str| leak(t.to_ascii_lowercase()))).inv().select(&tq).collect();
    let ts = db.tag.group_by(Ident::<Tag>::new()).select(tag_posts(db).select(view_count.opt())).fold((0i64, 0i64, 0i64), |(n, s, k), w| {
        (n + 1, s + w.unwrap_or(0), k + w.is_some() as i64)
    });
    let ln: HashIdx<Id<Tag>, Str> = db.tag.with(&ts).select((&db.tag.tag_name).map(|n: Str| leak(n.to_ascii_lowercase()))).collect();
    let rq = db.tag.with(&ts).group_by(Ident::<Tag>::new()).select((&ln).select_where(&tqi, |n: Str, t: Str| t.contains(n)).opt()).buf_fold(distinct_some);
    let v = drain(db.tag.select(Ident::<Tag>::new().and(&ts).and(&rq)));
    rows(v.into_iter().map(|(_, ((t, (n, s, k)), q))| row(vec![V::S(db.tag.tag_name.get(t).unwrap()), V::I(n), avg(s, k), V::I(q)])))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, COALESCE(p.AcceptedAnswerId, 0) AS AcceptedAnswerId,
//        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.Id, c.Id) AS ScoreRank, COUNT(c.Id) OVER (PARTITION BY p.Id) AS CommentCount,
//        LEAD(p.Score) OVER (ORDER BY p.CreationDate, p.Id, c.Id) AS NextPostScore, p.OwnerUserId
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId WHERE p.CreationDate < TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND p.Score IS NOT NULL),
// PostDetails AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.AcceptedAnswerId, rp.ScoreRank, rp.CommentCount,
//        CASE WHEN rp.Score < COALESCE(rp.NextPostScore, 0) THEN 'Improving' WHEN rp.Score = 0 THEN 'Neutral' ELSE 'Declining' END AS ScoreTrend
//     FROM RankedPosts rp WHERE rp.ScoreRank <= 5 AND NOT EXISTS (SELECT 1 FROM Posts p WHERE p.OwnerUserId = rp.OwnerUserId AND p.Score > rp.Score)),
// BadgeCounts AS (SELECT b.UserId, COUNT(b.Id) AS TotalBadges FROM Badges b WHERE b.Class = 1 GROUP BY b.UserId)
// SELECT ud.Id AS UserId, ud.DisplayName, pd.Title, pd.Score, pd.ScoreTrend, pd.CommentCount, COALESCE(bc.TotalBadges, 0) AS GoldBadges
// FROM Users ud JOIN PostDetails pd ON ud.Id = (SELECT OwnerUserId FROM Posts WHERE Id = pd.PostId) LEFT JOIN BadgeCounts bc ON ud.Id = bc.UserId
// WHERE pd.ScoreTrend = 'Declining' AND (ud.Reputation > 1000 OR ud.Views < 50) ORDER BY pd.CommentCount DESC, bc.TotalBadges DESC, pd.PostId OFFSET 0 ROWS FETCH NEXT 10 ROWS ONLY;
//
// Ported from rewrites/24339.sql: both windows run over post x comment rows, so `, p.Id, c.Id` makes ScoreRank and the LEAD total, and `, pd.PostId` the cut.
fn q24339(db: &'static So) -> String {
    let Post { creation_date, origid, score, owner_user_id, owner_user, .. } = &db.post;
    type Rw = (Id<Post>, i64, Option<i64>, i64, i64, Option<i64>);
    let rs: MatSet<Rw> = db
        .post
        .with(creation_date.lt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .select(Ident::<Post>::new().and(origid).and(comments_of(db).select(&db.comment.origid).opt()).and(creation_date).and(score).and(owner_user_id.opt()))
        .map(|(((((p, o), c), d), s), u)| (p, o, c, d, s, u))
        .collect();
    let sr = (&rs)
        .group_by(Same::<Rw>::new().map(|x: Rw| x.5))
        .select(Same::<Rw>::new())
        .window(row_number, |x: Rw| (Reverse(x.4), x.1, x.2.is_none(), x.2), asc);
    let top5: MatSet<Rw> = (&sr).filt(|(_, n): (Rw, i64)| n <= 5).map(|(x, _)| x).collect();
    let ld = whole(&rs).window(lead, |x: Rw| (x.3, x.1, x.2.is_none(), x.2, x.4), asc);
    let ld: MatSet<(Rw, Option<i64>)> = (&ld).map(|(x, n): (Rw, Option<(i64, i64, bool, Option<i64>, i64)>)| (x, n.map(|k| k.4))).collect();
    let ldi: HashIdx<Rw, Option<i64>> = (&ld).map(|(x, _)| x).inv().map(|(_, n)| n).collect();
    let cc = (&rs).group_by(Same::<Rw>::new().map(|x: Rw| x.0)).fold(0i64, |n, x: Rw| n + x.2.is_some() as i64);
    let omax = db.post.group_by(owner_user_id).select(score).fold(i64::MIN, |m, s| m.max(s));
    let gold = db.badge.with((&db.badge.class).eq(1)).group_by(&db.badge.user).fold(0i64, |n, _| n + 1);
    let users = Ident::<User>::new().with((&db.user.reputation).and(&db.user.views).filt(|(r, v)| r > 1000 || v < 50)).and((&gold).opt());
    let p = || Same::<Rw>::new().map(|x: Rw| x.0);
    let v = drain(
        (&top5)
            .select(Same::<Rw>::new().and(&ldi).and(p().select(&cc)).and(Same::<Rw>::new().flat_map(|x: Rw| x.5).select(&omax).opt()).and(p().select(owner_user).select(users)))
            .filt(|((((x, nx), _), m), _): ((((Rw, Option<i64>), i64), Option<i64>), (Id<User>, Option<i64>))| {
                m.map_or(true, |m| x.4 >= m) && x.4 >= nx.unwrap_or(0) && x.4 != 0
            }),
    );
    let v = top_n(v, |&(_, ((((x, _), c), _), (_, g)))| (Reverse(c), g.is_some(), Reverse(g), x.1), 10);
    rows(v.into_iter().map(|(_, ((((x, _), c), _), (u, g)))| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(post_fields(db, x.0, &["title", "score"]));
        f.extend([V::S("Declining"), V::I(c), V::I(g.unwrap_or(0))]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, ROW_NUMBER() OVER (PARTITION BY pt.Name ORDER BY p.CreationDate DESC) AS rn,
//        COUNT(v.Id) OVER (PARTITION BY p.Id) AS VoteCount
//     FROM Posts p JOIN PostTypes pt ON p.PostTypeId = pt.Id LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 2
//     WHERE p.CreationDate >= (cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '365 days') AND p.Score > 0),
// UserActivity AS (SELECT u.Id AS UserId, u.DisplayName, SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges, SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
//        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges, COUNT(DISTINCT ph.PostId) AS PostCount, COUNT(DISTINCT c.Id) AS CommentCount
//     FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Comments c ON p.Id = c.PostId
//     LEFT JOIN PostHistory ph ON u.Id = ph.UserId AND ph.CreationDate >= (cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '2 years') GROUP BY u.Id, u.DisplayName),
// PopularUsers AS (SELECT ua.UserId, ua.DisplayName, ua.PostCount, rc.VoteCount, RANK() OVER (ORDER BY ua.PostCount DESC, rc.VoteCount DESC) AS PopularityRank
//     FROM UserActivity ua JOIN (SELECT UserId, COUNT(*) AS VoteCount FROM Votes WHERE VoteTypeId = 2 GROUP BY UserId) rc ON ua.UserId = rc.UserId)
// SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, pu.DisplayName AS Author, pu.PostCount AS AuthorPostCount, pu.PopularityRank
// FROM RankedPosts rp JOIN Posts p ON rp.PostId = p.Id JOIN PopularUsers pu ON p.OwnerUserId = pu.UserId WHERE rp.rn <= 3
// ORDER BY pu.PopularityRank, rp.Score DESC NULLS LAST LIMIT 100;
//
// rn is over post x upvote rows; a CreationDate tie goes to the smaller post id, then vote id (the SQL leaves it open). VoteCount and the badge sums are
// never read; PostCount is a distinct count, so it comes from one fold over the user's recent history, for the users PopularUsers keeps.
fn q22470(db: &'static So) -> String {
    let Post { creation_date, score, origid, owner_user, .. } = &db.post;
    let Vote { vote_type_id, user, .. } = &db.vote;
    type R = (Id<Post>, Option<Id<Vote>>);
    let up = votes_of(db).select(Ident::<Vote>::new().with(vote_type_id.eq(2)));
    let rs: MatSet<R> = db.post.with(creation_date.ge(add_days(ts(2024, 10, 1, 12, 34, 56), -365))).with(score.gt(0)).select(Ident::<Post>::new().and(up.opt())).collect();
    let p = || Same::<R>::new().map(|x: R| x.0);
    let w = (&rs).group_by(p().select(ptype_name(db))).select(Same::<R>::new().and(p().select(creation_date.and(origid)))).window(row_number, |(x, (d, o))| (Reverse(d), o, x.1), asc);
    let rn: MatSet<R> = (&w).filt(|(_, n): ((R, (i64, i64)), i64)| n <= 3).map(|((x, _), _)| x).collect();
    let rc = db.vote.with(vote_type_id.eq(2)).group_by(user).fold(0i64, |n, _| n + 1);
    let hb: HashIdx<Id<User>, Id<PostHistory>> = (&db.post_history.user).inv().collect();
    let PostHistory { creation_date: hd, post_id, .. } = &db.post_history;
    let pc = db
        .user
        .with(&rc)
        .group_by(Ident::<User>::new())
        .select((&hb).select(Ident::<PostHistory>::new().with(hd.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -2))).select(post_id)).opt())
        .buf_fold(distinct_some);
    let pu = ranked(drain(db.user.with(&rc).select(Ident::<User>::new().and(&pc).and(&rc))), |&(_, ((_, n), v))| (Reverse(n), Reverse(v)), false);
    let pu = rel(pu);
    let pui: HashIdx<Id<User>, (i64, i64)> = (&pu).map(|((u, _), _)| u).inv().select(&pu).map(|((_, ((_, n), _)), k)| (n, k)).collect();
    let v = drain((&rn).select(Same::<R>::new().and(p().select(score)).and(p().select(owner_user).select(Ident::<User>::new().and(&pui)))));
    let v = top_n(v, |&(_, ((_, s), (_, (_, k))))| (k, Reverse(s)), 100);
    rows(v.into_iter().map(|(_, (((q, _), _), (u, (n, k))))| {
        let mut f = post_fields(db, q, &["id", "title", "created", "score"]);
        f.extend(ucols(db, u, &["name"]));
        f.extend([V::I(n), V::I(k)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, p.Score, p.ViewCount, ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS RankScore,
//        DENSE_RANK() OVER (ORDER BY p.CreationDate DESC) AS RankAge FROM Posts p WHERE p.CreationDate >= (CAST('2024-10-01' AS DATE) - INTERVAL '1 year')),
// TopUsers AS (SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(DISTINCT p.Id) AS PostCount, COUNT(DISTINCT b.Id) AS BadgeCount, COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty
//     FROM Users u LEFT JOIN Posts p ON u.Id = p.OwnerUserId LEFT JOIN Badges b ON u.Id = b.UserId LEFT JOIN Votes v ON v.UserId = u.Id
//     GROUP BY u.Id, u.DisplayName, u.Reputation HAVING SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) > 10),
// PostHistories AS (SELECT ph.PostId, ph.PostHistoryTypeId, ph.CreationDate, ph.UserDisplayName, ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate) AS ChronologicalOrder
//     FROM PostHistory ph WHERE ph.CreationDate >= (CAST('2024-10-01' AS DATE) - INTERVAL '6 months')),
// EligiblePosts AS (SELECT rp.PostId, rp.Title, rp.CreationDate, rp.Score, rp.ViewCount, pu.DisplayName AS TopUser, ph.UserDisplayName AS LastEditor
//     FROM RankedPosts rp LEFT JOIN TopUsers pu ON pu.PostCount >= 5 LEFT JOIN PostHistories ph ON rp.PostId = ph.PostId WHERE RankScore <= 10 AND RankAge <= 5)
// SELECT ep.PostId, ep.Title, ep.Score, ep.ViewCount, COALESCE(ep.TopUser, 'No Top User') AS TopUser, COALESCE(ep.LastEditor, 'No Last Editor') AS LastEditor,
//        COUNT(DISTINCT c.Id) AS TotalComments
// FROM EligiblePosts ep LEFT JOIN Comments c ON ep.PostId = c.PostId GROUP BY ep.PostId, ep.Title, ep.Score, ep.ViewCount, ep.TopUser, ep.LastEditor
// ORDER BY ep.Score DESC, TotalComments DESC;
//
// Both ranks read only base columns, so the eligible posts are picked first; a Score tie inside a post type goes to the smaller post id (the SQL leaves it open).
// The HAVING sum can only pass for a user who cast an upvote, so the posts x badges x votes product is driven only for those users. TotalComments is
// COUNT(DISTINCT c.Id) within one post, which is that post's comment count.
fn q22792(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, origid, .. } = &db.post;
    let rp = db.post.with(creation_date.ge(add_years(date(2024, 10, 1), -1)));
    let rs = (&rp).group_by(post_type_id).select(Ident::<Post>::new().and(origid).and(score)).window(row_number, |((_, o), s)| (Reverse(s), o), asc);
    let ra = whole(&rp).select(Ident::<Post>::new().and(creation_date)).window(dense_rank, |(_, d)| Reverse(d), asc);
    let ra5: MatSet<Id<Post>> = (&ra).filt(|(_, n): ((Id<Post>, i64), i64)| n <= 5).map(|((p, _), _)| p).collect();
    let elig: MatSet<Id<Post>> = (&rs).filt(|(_, n): (((Id<Post>, i64), i64), i64)| n <= 10).map(|(((p, _), _), _)| p).with(&ra5).collect();
    let vote_type_id = &db.vote.vote_type_id;
    let cand = db.user.with(votes_by(db).select(Ident::<Vote>::new().with(vote_type_id.eq(2))));
    let tu = cand
        .group_by(Ident::<User>::new())
        .select(posts_of(db).opt().and(badges_of(db).opt()).and(votes_by(db).select(vote_type_id).opt()))
        .buf_fold(|v| (distinct_some(v.iter().map(|x| x.0 .0)), distinct_some(v.iter().map(|x| x.0 .1)), v.iter().filter(|x| x.1 == Some(2)).count() as i64));
    let names = drain(db.user.with((&tu).filt(|a: (i64, i64, i64)| a.2 > 10 && a.0 >= 5)).select(&db.user.display_name));
    let names = left_all(names.into_iter().map(|x| x.1).collect());
    let PostHistory { creation_date: hd, user_display_name, .. } = &db.post_history;
    let ph = history_of(db).select(Ident::<PostHistory>::new().with(hd.ge(add_months(date(2024, 10, 1), -6))).select(user_display_name.opt())).opt();
    type E = (Id<Post>, Option<Str>, Option<Str>);
    let ep: MatSet<E> = (&elig).select(Ident::<Post>::new().and(ph)).cross(&names).map(|((p, h), u)| (p, u, h.flatten())).collect();
    let cpp = comments_per_post(db);
    let v = drain((&ep).select(Same::<E>::new().and(Same::<E>::new().map(|x: E| x.0).select(&cpp))));
    rows(v.into_iter().map(|(_, ((p, u, h), c))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views"]);
        f.extend([V::S(u.unwrap_or("No Top User")), V::S(h.unwrap_or("No Last Editor")), V::I(c)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Tags, p.Score, p.ViewCount, p.CreationDate,
//        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC) AS Rank
//     FROM Posts p WHERE p.PostTypeId = 1 AND p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'),
// TagStatistics AS (SELECT t.TagName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts,
//        SUM(CASE WHEN p.Score <= 0 THEN 1 ELSE 0 END) AS NegativePosts, AVG(p.Score) AS AverageScore
//     FROM Tags t JOIN Posts p ON p.Tags LIKE CONCAT('%<', t.TagName, '>%') GROUP BY t.TagName),
// UserReputation AS (SELECT u.Id AS UserId, u.DisplayName, COUNT(p.Id) AS TotalPosts, SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS UpvotedPosts, AVG(u.Reputation) AS AverageReputation
//     FROM Users u JOIN Posts p ON p.OwnerUserId = u.Id GROUP BY u.Id, u.DisplayName)
// SELECT rp.PostId, rp.Title, rp.Tags, rp.Score AS PostScore, rp.ViewCount, rp.CreationDate, ts.TagName, ts.TotalPosts, ts.PositivePosts, ts.NegativePosts, ts.AverageScore,
//        ur.DisplayName AS Author, ur.TotalPosts AS AuthorTotalPosts, ur.UpvotedPosts AS AuthorUpvotedPosts, ur.AverageReputation
// FROM RankedPosts rp JOIN TagStatistics ts ON ts.TagName IN (SELECT unnest(string_to_array(rp.Tags, '> <')))
// JOIN UserReputation ur ON ur.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = rp.PostId) WHERE rp.Rank <= 10 ORDER BY ts.AverageScore DESC, rp.ViewCount DESC;
//
// Rank reads only base columns, so the ten questions are picked first; a tie goes to the smaller post id (the SQL leaves it open).
fn q28022(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, origid, tags_str, owner_user, .. } = &db.post;
    let w = db
        .post
        .with(post_type_id.eq(1))
        .with(creation_date.gt(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(post_type_id)
        .select(Ident::<Post>::new().and(origid).and(score).and(view_count.opt()))
        .window(row_number, |(((_, o), s), v)| (Reverse(s), v.is_some(), Reverse(v), o), asc);
    let rp: MatSet<Id<Post>> = (&w).filt(|(_, n): ((((Id<Post>, i64), i64), Option<i64>), i64)| n <= 10).map(|((((p, _), _), _), _)| p).collect();
    let tagp: HashIdx<Id<Tag>, Id<Post>> = (&db.post.tags).inv().collect();
    let ts = db.tag.group_by(Ident::<Tag>::new()).select((&tagp).select(score)).fold([0i64; 4], |a, s| [a[0] + 1, a[1] + (s > 0) as i64, a[2] + (s <= 0) as i64, a[3] + s]);
    let tsn: HashIdx<Str, Id<Tag>> = db.tag.with(&ts).select(&db.tag.tag_name).inv().collect();
    let pt: MatSet<(Id<Post>, Id<Tag>)> = (&rp).select(Ident::<Post>::new().and(tags_str.flat_map(|t: Str| t.split("> <")).select(&tsn))).collect();
    let ur = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(score)).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + (s > 0) as i64]);
    type P = (Id<Post>, Id<Tag>);
    let v = drain((&pt).select(Same::<P>::new().and(Same::<P>::new().map(|x: P| x.1).select(&ts)).and(Same::<P>::new().map(|x: P| x.0).select(owner_user).select(Ident::<User>::new().and(&ur)))));
    rows(v.into_iter().map(|(_, (((p, t), a), (u, b)))| {
        let mut f = post_fields(db, p, &["id", "title", "tags", "score", "views", "created"]);
        f.extend([V::S(db.tag.tag_name.get(t).unwrap()), V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[3], a[0])]);
        f.extend(ucols(db, u, &["name"]));
        f.extend([V::I(b[0]), V::I(b[1]), V::F(db.user.reputation.get(u).unwrap() as f64)]);
        row(f)
    }))
}

// WITH ProcessedTags AS (SELECT p.Id AS PostId, p.Title, p.CreationDate, unnest(string_to_array(substring(p.Tags, 2, length(p.Tags) - 2), '><')) AS Tag FROM Posts p WHERE p.PostTypeId = 1),
// TagCounts AS (SELECT Tag, COUNT(*) AS TagCount FROM ProcessedTags GROUP BY Tag HAVING COUNT(*) > 10),
// UserVotes AS (SELECT v.UserId, COUNT(v.Id) AS VoteCount FROM Votes v WHERE v.VoteTypeId IN (2, 3) GROUP BY v.UserId),
// ActiveUsers AS (SELECT u.Id, u.DisplayName, u.Reputation, COALESCE(uv.VoteCount, 0) AS VoteCount FROM Users u LEFT JOIN UserVotes uv ON u.Id = uv.UserId WHERE u.Reputation > 100),
// RelevantPosts AS (SELECT p.Id AS PostId, p.Title, p.ViewCount, COUNT(c.Id) AS CommentCount, COUNT(DISTINCT v.UserId) AS UniqueVoteCount
//     FROM Posts p LEFT JOIN Comments c ON p.Id = c.PostId LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 2 WHERE p.PostTypeId = 1 GROUP BY p.Id, p.Title, p.ViewCount)
// SELECT rp.PostId, rp.Title, rp.ViewCount, rp.CommentCount, au.DisplayName AS ActiveUser, au.VoteCount, pt.Tag, tc.TagCount
// FROM RelevantPosts rp JOIN ActiveUsers au ON au.Id IN (SELECT DISTINCT v.UserId FROM Votes v WHERE v.PostId = rp.PostId)
// JOIN ProcessedTags pt ON pt.PostId = rp.PostId JOIN TagCounts tc ON pt.Tag = tc.Tag
// JOIN (SELECT at.Tag, ARRAY_AGG(DISTINCT au.DisplayName) AS Users FROM ProcessedTags at JOIN Votes v ON at.PostId = v.PostId AND v.VoteTypeId = 2
//       JOIN ActiveUsers au ON v.UserId = au.Id GROUP BY at.Tag) AS TagUsers ON TagUsers.Tag = pt.Tag
// ORDER BY rp.ViewCount DESC LIMIT 10;
//
// TagUsers is one row per tag and only its Tag is read, so it is a semi-join on the tag. The comments x upvotes product is driven only for the
// questions an active user voted on; UniqueVoteCount is never read.
fn q29996(db: &'static So) -> String {
    let Post { post_type_id, tags_str, view_count, .. } = &db.post;
    let Vote { vote_type_id, user, .. } = &db.vote;
    let q = db.post.with(post_type_id.eq(1));
    let tc = (&q).select(tags_str.flat_map(tag_list)).inv().fold(0i64, |n, _| n + 1);
    let tc10 = (&tc).filt(|n: i64| n > 10);
    let active = || Ident::<User>::new().with((&db.user.reputation).gt(100));
    let uv = db.user.group_by(Ident::<User>::new()).select(votes_by(db).select(Ident::<Vote>::new().with(vote_type_id.is_in([2, 3])))).fold(0i64, |n, _| n + 1);
    let tu: MatSet<Str> = (&q).with(votes_of(db).select(Ident::<Vote>::new().with(vote_type_id.eq(2)).select(user).with(active()))).select(tags_str.flat_map(tag_list)).collect();
    type C = (Id<Post>, Id<User>);
    let cand: MatSet<C> = (&q).select(Ident::<Post>::new().and(votes_of(db).select(user).with(active()))).collect();
    let cp: MatSet<Id<Post>> = (&cand).map(|x: C| x.0).collect();
    let up = votes_of(db).select(Ident::<Vote>::new().with(vote_type_id.eq(2)));
    let rp = (&cp).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(up.opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let v = drain((&cand).select(
        Same::<C>::new()
            .and(Same::<C>::new().map(|x: C| x.0).select(view_count.opt().and(&rp)))
            .and(Same::<C>::new().map(|x: C| x.1).select((&uv).opt()))
            .and(Same::<C>::new().map(|x: C| x.0).select(tags_str.flat_map(tag_list)).with(&tu).select(Same::<Str>::new().and(&tc10))),
    ));
    let v = top_n(v, |&(_, (((_, (w, _)), _), _))| (w.is_some(), Reverse(w)), 10);
    rows(v.into_iter().map(|(_, ((((p, u), (w, c)), n), (t, k)))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([oint(w), V::I(c)]);
        f.extend(ucols(db, u, &["name"]));
        f.extend([V::I(n.unwrap_or(0)), V::S(t), V::I(k)]);
        row(f)
    }))
}

// WITH RankedPosts AS (SELECT p.Id AS PostId, p.Title, p.Body, p.CreationDate, p.ViewCount, p.Score, ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC NULLS LAST) AS rn
//     FROM Posts p WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND p.Score IS NOT NULL),
// UserReputation AS (SELECT u.Id AS UserId, u.Reputation, COUNT(b.Id) FILTER (WHERE b.Class = 1) AS GoldBadges, COUNT(b.Id) FILTER (WHERE b.Class = 2) AS SilverBadges,
//        COUNT(b.Id) FILTER (WHERE b.Class = 3) AS BronzeBadges FROM Users u LEFT JOIN Badges b ON u.Id = b.UserId GROUP BY u.Id, u.Reputation),
// PostHistoryClosed AS (SELECT ph.PostId, COUNT(DISTINCT ph.Id) AS CloseVotes FROM PostHistory ph WHERE ph.PostHistoryTypeId = 10 GROUP BY ph.PostId),
// PopularTags AS (SELECT unnest(string_to_array(Tags, '><')) AS Tag FROM Posts WHERE PostTypeId = 1),
// TagCounts AS (SELECT t.TagName, COUNT(p.Id) AS PostCount FROM Tags t LEFT JOIN Posts p ON p.Tags LIKE CONCAT('%', t.TagName, '%') GROUP BY t.TagName HAVING COUNT(p.Id) > 5)
// SELECT up.UserId, up.Reputation, up.GoldBadges, up.SilverBadges, up.BronzeBadges, rp.Title AS TopPostTitle, rp.ViewCount AS TopPostViews, rp.Score AS TopPostScore,
//        COALESCE(phc.CloseVotes, 0) AS TotalCloseVotes, tc.TagName, tc.PostCount
// FROM UserReputation up LEFT JOIN RankedPosts rp ON up.UserId = rp.PostId LEFT JOIN PostHistoryClosed phc ON rp.PostId = phc.PostId
// LEFT JOIN TagCounts tc ON tc.TagName IN (SELECT Tag FROM PopularTags)
// WHERE up.Reputation > 1000 AND rp.rn = 1 AND (rp.ViewCount IS NOT NULL OR rp.Score < 0) ORDER BY up.Reputation DESC, rp.ViewCount DESC LIMIT 10;
//
// rn reads only base columns, so each owner's best post is picked first; a Score tie goes to the smaller post id, and the cut, which falls among one
// user's TagCounts rows, to the tag name (the SQL leaves both open). `up.UserId = rp.PostId` compares a user id with a post id, so it goes through the raw ids.
fn q24220(db: &'static So) -> String {
    let Post { creation_date, owner_user_id, origid, score, view_count, post_type_id, tags_str, .. } = &db.post;
    let w = db
        .post
        .with(creation_date.ge(add_years(ts(2024, 10, 1, 12, 34, 56), -1)))
        .group_by(owner_user_id.opt())
        .select(Ident::<Post>::new().and(origid).and(score))
        .window(row_number, |((_, o), s)| (Reverse(s), o), asc);
    let rp1: MatSet<Id<Post>> = (&w).filt(|(_, n): (((Id<Post>, i64), i64), i64)| n == 1).map(|(((p, _), _), _)| p).collect();
    let rp: HashIdx<i64, Id<Post>> = (&rp1).select(origid).inv().collect();
    let ub = db.user.group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 3], |a, c| {
        [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]
    });
    let phc = db.post_history.with((&db.post_history.post_history_type_id).eq(10)).group_by(&db.post_history.post).fold(0i64, |n, _| n + 1);
    let pt: MatSet<Str> = db.post.with(post_type_id.eq(1)).select(tags_str.flat_map(|t: Str| t.split("><"))).collect();
    let tcf = db.tag.group_by(Ident::<Tag>::new()).select(tag_posts(db).opt()).fold(0i64, |n, p| n + p.is_some() as i64);
    let tc = drain(db.tag.with((&db.tag.tag_name).with(&pt)).select((&db.tag.tag_name).and((&tcf).filt(|n: i64| n > 5))));
    let tc = left_all(tc.into_iter().map(|x| x.1).collect());
    let rpc = Ident::<Post>::new().and(view_count.opt()).and(score).filt(|((_, w), s): ((Id<Post>, Option<i64>), i64)| w.is_some() || s < 0).and((&phc).opt());
    let v = drain(
        db.user
            .with((&db.user.reputation).gt(1000))
            .select(Ident::<User>::new().and(&db.user.reputation).and(&ub).and((&db.user.origid).select(&rp).select(rpc)))
            .cross(&tc),
    );
    let v = top_n(v, |&(_, ((((_, r), _), (((_, w), _), _)), t))| (Reverse(r), w.is_some(), Reverse(w), t.map(|t| t.0)), 10);
    rows(v.into_iter().map(|(_, ((((u, _), b), (((p, _), _), c)), t))| {
        let mut f = ucols(db, u, &["uid", "rep"]);
        f.extend([V::I(b[0]), V::I(b[1]), V::I(b[2])]);
        f.extend(post_fields(db, p, &["title", "views", "score"]));
        f.push(V::I(c.unwrap_or(0)));
        match t {
            Some((n, k)) => f.extend([V::S(n), V::I(k)]),
            None => f.extend([V::Null, V::Null]),
        }
        row(f)
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("13831", q13831),
    ("14857", q14857),
    ("24751", q24751),
    ("26109", q26109),
    ("31512", q31512),
    ("7014", q7014),
    ("26593", q26593),
    ("10754", q10754),
    ("10518", q10518),
    ("5851", q5851),
    ("7555", q7555),
    ("26906", q26906),
    ("27645", q27645),
    ("3518", q3518),
    ("686", q686),
    ("7188", q7188),
    ("7575", q7575),
    ("28659", q28659),
    ("1884", q1884),
    ("6260", q6260),
    ("32386", q32386),
    ("26868", q26868),
    ("24339", q24339),
    ("22470", q22470),
    ("22792", q22792),
    ("28022", q28022),
    ("29996", q29996),
    ("24220", q24220),
];
