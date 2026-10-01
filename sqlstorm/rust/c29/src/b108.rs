use harness::prelude::*;
use std::cmp::Reverse;

fn questions(db: &'static So) -> impl Drive<D = Id<Post>, R = Id<Post>> {
    owned(db).with((&db.post.post_type_id).eq(1))
}

fn owned_since(db: &'static So, d: i64) -> impl Drive<D = Id<Post>, R = Id<Post>> {
    owned(db).with((&db.post.creation_date).ge(d))
}

fn since(db: &'static So, d: i64) -> impl Drive<D = Id<Post>, R = Id<Post>> {
    db.post.with((&db.post.creation_date).ge(d))
}

fn views_desc(db: &'static So, p: Id<Post>) -> (bool, Reverse<Option<i64>>) {
    let w = db.post.view_count.get(p);
    (w.is_none(), Reverse(w))
}

fn year_ago() -> i64 {
    ts(2023, 10, 1, 12, 34, 56)
}

fn month_ago() -> i64 {
    ts(2024, 9, 1, 12, 34, 56)
}

// (SELECT PostId, COUNT(*) FROM Comments GROUP BY PostId): grouped by the raw
// PostId column and joined back on p.Id, so a post with no comments gets NULL.
fn comments_by_raw_id(db: &'static So) -> Fold<i64, i64> {
    db.comment.group_by(&db.comment.post_id).fold(0i64, |a, _| a + 1)
}

fn votes_by_raw_id(db: &'static So) -> Fold<i64, i64> {
    db.vote.group_by(&db.vote.post_id).fold(0i64, |a, _| a + 1)
}

// The newest (or best) questions with the grouped comment count beside them.
fn with_comment_count(db: &'static So, by_score: bool, cols: &[&str]) -> String {
    let cc = comments_by_raw_id(db);
    let mut v = Vec::new();
    questions(db).select((&db.post.origid).select((&cc).opt())).drive(|p, c| v.push((p, c)));
    if by_score {
        v.sort_by_key(|&(p, _)| Reverse(db.post.score.get(p).unwrap()));
    } else {
        v.sort_by_key(|&(p, _)| newest(db, p));
    }
    rows(v.iter().take(10).map(|&(p, c)| {
        row(cols.iter().map(|x| if *x == "cc" { oint(c) } else { post_fields(db, p, &[x]).pop().unwrap() }).collect())
    }))
}

// SELECT
// P.Title,
// P.CreationDate,
// U.DisplayName AS Author,
// C.CommentCount,
// P.Score
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount
// FROM Comments
// GROUP BY PostId) C ON P.Id = C.PostId
// WHERE
// P.PostTypeId = 1
// ORDER BY
// P.CreationDate DESC
// LIMIT 10;
fn q15146(db: &'static So) -> String {
    with_comment_count(db, false, &["title", "created", "owner", "cc", "score"])
}

// SELECT
// p.Title,
// p.CreationDate,
// u.DisplayName AS OwnerDisplayName,
// p.Score,
// c.CommentCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount
// FROM Comments
// GROUP BY PostId) c ON p.Id = c.PostId
// WHERE
// p.PostTypeId = 1
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q18658(db: &'static So) -> String {
    with_comment_count(db, false, &["title", "created", "owner", "score", "cc"])
}

// SELECT
// P.Title,
// U.DisplayName AS OwnerDisplayName,
// P.Score,
// P.ViewCount,
// C.CommentCount,
// P.CreationDate
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount
// FROM Comments
// GROUP BY PostId) C ON P.Id = C.PostId
// WHERE
// P.PostTypeId = 1
// ORDER BY
// P.CreationDate DESC
// LIMIT 10;
fn q17889(db: &'static So) -> String {
    with_comment_count(db, false, &["title", "owner", "score", "views", "cc", "created"])
}

// SELECT
// p.Title,
// u.DisplayName AS OwnerDisplayName,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// c.CommentCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
// WHERE
// p.PostTypeId = 1
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q19579(db: &'static So) -> String {
    with_comment_count(db, false, &["title", "owner", "created", "score", "views", "cc"])
}

// SELECT
// p.Title,
// p.CreationDate,
// p.ViewCount,
// u.DisplayName AS OwnerDisplayName,
// c.CommentCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount
// FROM Comments
// GROUP BY PostId) c ON p.Id = c.PostId
// WHERE
// p.PostTypeId = 1
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q15810(db: &'static So) -> String {
    with_comment_count(db, false, &["title", "created", "views", "owner", "cc"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// u.DisplayName AS OwnerDisplayName,
// c.CommentCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount
// FROM Comments
// GROUP BY PostId) c ON p.Id = c.PostId
// WHERE
// p.PostTypeId = 1
// ORDER BY
// p.Score DESC
// LIMIT 10;
fn q19670(db: &'static So) -> String {
    with_comment_count(db, true, &["id", "title", "created", "score", "owner", "cc"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// u.DisplayName AS OwnerDisplayName,
// p.Score,
// p.ViewCount,
// c.CommentCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// (
// SELECT
// PostId,
// COUNT(*) AS CommentCount
// FROM
// Comments
// GROUP BY
// PostId
// ) c ON p.Id = c.PostId
// WHERE
// p.PostTypeId = 1
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q15171(db: &'static So) -> String {
    with_comment_count(db, false, &["id", "title", "created", "owner", "score", "views", "cc"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// u.DisplayName AS OwnerDisplayName,
// p.Score,
// p.ViewCount,
// c.CommentCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount
// FROM Comments
// GROUP BY PostId) c ON p.Id = c.PostId
// WHERE
// p.PostTypeId = 1
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q15384(db: &'static So) -> String {
    with_comment_count(db, false, &["id", "title", "created", "owner", "score", "views", "cc"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// u.DisplayName AS Owner,
// p.Score,
// p.ViewCount,
// c.CommentCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount
// FROM Comments
// GROUP BY PostId) c ON p.Id = c.PostId
// WHERE
// p.PostTypeId = 1
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q15480(db: &'static So) -> String {
    with_comment_count(db, false, &["id", "title", "created", "owner", "score", "views", "cc"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// u.DisplayName AS OwnerDisplayName,
// c.CommentCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount
// FROM Comments
// GROUP BY PostId) c ON p.Id = c.PostId
// WHERE
// p.PostTypeId = 1
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q15598(db: &'static So) -> String {
    with_comment_count(db, false, &["id", "title", "created", "score", "views", "owner", "cc"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// u.DisplayName AS OwnerDisplayName,
// p.CreationDate,
// p.Score,
// c.CommentCount,
// p.ViewCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
// WHERE
// p.PostTypeId = 1
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q15861(db: &'static So) -> String {
    with_comment_count(db, false, &["id", "title", "owner", "created", "score", "cc", "views"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// u.DisplayName AS OwnerDisplayName,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// c.CommentCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// (SELECT PostId, COUNT(*) as CommentCount
// FROM Comments
// GROUP BY PostId) c ON p.Id = c.PostId
// WHERE
// p.PostTypeId = 1
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q16345(db: &'static So) -> String {
    with_comment_count(db, false, &["id", "title", "owner", "created", "score", "views", "cc"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// u.DisplayName AS OwnerDisplayName,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// c.CommentCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
// WHERE
// p.PostTypeId = 1
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q16727(db: &'static So) -> String {
    with_comment_count(db, false, &["id", "title", "owner", "created", "views", "score", "cc"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// u.DisplayName AS OwnerDisplayName,
// p.Score,
// c.CommentCount,
// p.ViewCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// (SELECT
// PostId,
// COUNT(*) AS CommentCount
// FROM
// Comments
// GROUP BY
// PostId) c ON p.Id = c.PostId
// WHERE
// p.PostTypeId = 1
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q19397(db: &'static So) -> String {
    with_comment_count(db, false, &["id", "title", "created", "owner", "score", "cc", "views"])
}

// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// U.DisplayName AS OwnerDisplayName,
// P.Score,
// P.ViewCount,
// C.CommentCount
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount
// FROM Comments
// GROUP BY PostId) C ON P.Id = C.PostId
// WHERE
// P.PostTypeId = 1
// ORDER BY
// P.Score DESC
// LIMIT 10;
fn q17928(db: &'static So) -> String {
    with_comment_count(db, true, &["id", "title", "created", "owner", "score", "views", "cc"])
}

// SELECT
// P.Id AS PostId,
// P.Title,
// U.DisplayName AS OwnerDisplayName,
// P.CreationDate,
// P.Score,
// P.ViewCount,
// C.Count AS CommentCount
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS Count
// FROM Comments
// GROUP BY PostId) C ON P.Id = C.PostId
// WHERE
// P.PostTypeId = 1
// ORDER BY
// P.CreationDate DESC
// LIMIT 10;
fn q19254(db: &'static So) -> String {
    with_comment_count(db, false, &["id", "title", "owner", "created", "score", "views", "cc"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// u.DisplayName AS OwnerDisplayName,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// p.AnswerCount,
// c.CommentCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount
// FROM Comments
// GROUP BY PostId) c ON p.Id = c.PostId
// WHERE
// p.PostTypeId = 1
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q15114(db: &'static So) -> String {
    with_comment_count(db, false, &["id", "title", "owner", "created", "score", "views", "answers", "cc"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// u.DisplayName AS OwnerDisplayName,
// p.Score,
// p.ViewCount,
// p.AnswerCount,
// c.CommentCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
// WHERE
// p.PostTypeId = 1
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q17045(db: &'static So) -> String {
    with_comment_count(db, false, &["id", "title", "created", "owner", "score", "views", "answers", "cc"])
}

// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// U.DisplayName AS OwnerDisplayName,
// P.Score,
// P.ViewCount,
// C.CommentCount,
// P.AnswerCount
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) C ON P.Id = C.PostId
// WHERE
// P.PostTypeId = 1
// ORDER BY
// P.CreationDate DESC
// LIMIT 10;
fn q17604(db: &'static So) -> String {
    with_comment_count(db, false, &["id", "title", "created", "owner", "score", "views", "cc", "answers"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// u.DisplayName AS OwnerDisplayName,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// c.CommentCount,
// p.FavoriteCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount
// FROM Comments
// GROUP BY PostId) c ON p.Id = c.PostId
// WHERE
// p.PostTypeId = 1
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q15363(db: &'static So) -> String {
    with_comment_count(db, false, &["id", "title", "owner", "created", "score", "views", "cc", "favorites"])
}

// SELECT
// U.DisplayName,
// P.Title,
// P.CreationDate,
// P.Score,
// COUNT(C.CommentId) AS CommentCount
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// (SELECT PostId, COUNT(Id) AS CommentId FROM Comments GROUP BY PostId) C ON P.Id = C.PostId
// WHERE
// P.PostTypeId = 1
// GROUP BY
// U.DisplayName, P.Title, P.CreationDate, P.Score
// ORDER BY
// P.CreationDate DESC
// LIMIT 10;
fn q16192(db: &'static So) -> String {
    let cc = comments_by_raw_id(db);
    let Post { title, creation_date, score, owner_user, origid, .. } = &db.post;
    let key = owner_user.select(&db.user.display_name).and(title.opt()).and(creation_date).and(score);
    let mut v = Vec::new();
    questions(db).group_by(key).select(origid.select((&cc).opt())).fold(0i64, |n, c| n + c.is_some() as i64).drive(|k, n| v.push((k, n)));
    v.sort_by_key(|x| Reverse(x.0.0.1));
    rows(v.iter().take(10).map(|&((((dn, t), c), s), n)| row(vec![V::S(dn), ostr(t), V::T(c), V::I(s), V::I(n)])))
}

// Types with a fold over each post joined to something.
fn by_type<Q, R, S, F>(db: &'static So, base: Q, joined: R, init: S, f: F) -> Fold<Str, S>
where
    Q: Drive<D = Id<Post>, R = Id<Post>>,
    R: IntoQuery,
    R::Q: Probe<D = Id<Post>>,
    S: Copy,
    F: Fn(S, ROf<R>) -> S,
{
    base.group_by((&db.post.post_type).select(&db.post_type.name)).select(joined).fold(init, f)
}

fn listed<K: Copy + Eq + std::hash::Hash, const N: usize>(f: &Fold<K, [i64; N]>) -> Vec<(K, [i64; N])> {
    let mut v = Vec::new();
    f.drive(|k, a| v.push((k, a)));
    v.sort_by_key(|x| Reverse(x.1[0]));
    v
}

// Types with SUM(c.CommentCount) from the grouped subquery.
fn types_comment_sum(db: &'static So) -> Vec<(Str, [i64; 6])> {
    let cc = comments_by_raw_id(db);
    let Post { score, view_count, origid, .. } = &db.post;
    let f = by_type(db, db.post.iq(), score.and(view_count.opt()).and(origid.select((&cc).opt())), [0i64; 6], |a, ((s, w), c)| {
        [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + c.is_some() as i64, a[5] + c.unwrap_or(0)]
    });
    listed(&f)
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AverageScore,
// SUM(c.CommentCount) AS TotalComments
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// (SELECT PostId, COUNT(Id) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q14375(db: &'static So) -> String {
    rows(types_comment_sum(db).iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), nullable(a[5], a[4])])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS PostCount,
// AVG(p.Score) AS AverageScore,
// SUM(c.CommentCount) AS TotalComments
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// (SELECT
// PostId, COUNT(Id) AS CommentCount
// FROM
// Comments
// GROUP BY
// PostId) c ON p.Id = c.PostId
// GROUP BY
// pt.Name
// ORDER BY
// PostCount DESC;
fn q10099(db: &'static So) -> String {
    rows(types_comment_sum(db).iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), nullable(a[5], a[4])])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS PostCount,
// AVG(p.Score) AS AverageScore,
// SUM(c.CommentCount) AS TotalComments
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN (
// SELECT
// PostId,
// COUNT(*) AS CommentCount
// FROM
// Comments
// GROUP BY
// PostId
// ) c ON p.Id = c.PostId
// GROUP BY
// pt.Name
// ORDER BY
// PostCount DESC;
fn q10347(db: &'static So) -> String {
    rows(types_comment_sum(db).iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), nullable(a[5], a[4])])))
}

// SELECT
// pt.Name AS PostType,
// AVG(p.Score) AS AverageScore,
// AVG(p.ViewCount) AS AverageViewCount,
// SUM(c.CommentCount) AS TotalComments
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// (SELECT
// PostId, COUNT(*) AS CommentCount
// FROM
// Comments
// GROUP BY
// PostId) c ON p.Id = c.PostId
// GROUP BY
// pt.Name
// ORDER BY
// pt.Name;
fn q12872(db: &'static So) -> String {
    rows(types_comment_sum(db).iter().map(|&(k, a)| row(vec![V::S(k), avg(a[1], a[0]), avg(a[3], a[2]), nullable(a[5], a[4])])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(voteCount) AS AvgVotesPerPost
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// (SELECT PostId, COUNT(Id) AS voteCount
// FROM Votes
// GROUP BY PostId) v ON p.Id = v.PostId
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q11014(db: &'static So) -> String {
    let vc = votes_by_raw_id(db);
    let f = by_type(db, db.post.iq(), (&db.post.origid).select((&vc).opt()), [0i64; 3], |a, c| [a[0] + 1, a[1] + c.is_some() as i64, a[2] + c.unwrap_or(0)]);
    rows(listed(&f).iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), avg(a[2], a[1])])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(v.vote_count) AS AverageVotes,
// SUM(v.vote_count) AS TotalVotes
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN (
// SELECT
// PostId,
// COUNT(*) AS vote_count
// FROM
// Votes
// GROUP BY
// PostId
// ) v ON p.Id = v.PostId
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q13836(db: &'static So) -> String {
    let vc = votes_by_raw_id(db);
    let f = by_type(db, db.post.iq(), (&db.post.origid).select((&vc).opt()), [0i64; 3], |a, c| [a[0] + 1, a[1] + c.is_some() as i64, a[2] + c.unwrap_or(0)]);
    rows(listed(&f).iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), avg(a[2], a[1]), nullable(a[2], a[1])])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AvgScore,
// AVG(p.ViewCount) AS AvgViewCount,
// COALESCE(SUM(CASE WHEN v.PostId IS NOT NULL THEN 1 ELSE 0 END), 0) AS TotalVotes
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q14455(db: &'static So) -> String {
    let Post { score, view_count, .. } = &db.post;
    let f = by_type(db, db.post.iq(), score.and(view_count.opt()).and(votes_of(db).opt()), [0i64; 5], |a, ((s, w), x)| {
        [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + x.is_some() as i64]
    });
    rows(listed(&f).iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), avg(a[3], a[2]), V::I(a[4])])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AverageScore,
// AVG(p.ViewCount) AS AverageViewCount,
// (SELECT COUNT(*) FROM Users) AS TotalUsers,
// AVG(u.Reputation) AS AverageReputation
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// GROUP BY
// pt.Id, pt.Name
// ORDER BY
// TotalPosts DESC;
fn q13241(db: &'static So) -> String {
    let users = db.user.iq().fold_flat(0i64, |a, _| a + 1);
    let Post { score, view_count, owner_user, post_type, .. } = &db.post;
    let mut v = Vec::new();
    db.post
        .group_by(post_type)
        .select(score.and(view_count.opt()).and(owner_user.select(&db.user.reputation).opt()))
        .fold([0i64; 6], |a, ((s, w), r)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + r.is_some() as i64, a[5] + r.unwrap_or(0)])
        .drive(|t, a| v.push((db.post_type.name.get(t).unwrap(), a)));
    v.sort_by_key(|x| Reverse(x.1[0]));
    rows(v.iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), avg(a[3], a[2]), V::I(users), avg(a[5], a[4])])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AverageScore,
// COUNT(DISTINCT u.Id) AS TotalUsers,
// SUM(CASE WHEN b.UserId IS NOT NULL THEN 1 ELSE 0 END) AS TotalBadges
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// pt.Name
// ORDER BY
// pt.Name;
fn q10869(db: &'static So) -> String {
    let Post { score, owner_user, .. } = &db.post;
    let f = by_type(db, db.post.iq(), score.and(owner_user.select(badges_of(db)).opt()), [0i64; 3], |a, (s, b)| [a[0] + 1, a[1] + s, a[2] + b.is_some() as i64]);
    let u = db.post.group_by((&db.post.post_type).select(&db.post_type.name)).select(owner_user).count_distinct();
    let mut v = Vec::new();
    f.and((&u).opt()).drive(|k, (a, u)| v.push((k, a, u.unwrap_or(0))));
    rows(v.iter().map(|&(k, a, u)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), V::I(u), V::I(a[2])])))
}

// SELECT
// EXTRACT(YEAR FROM p.CreationDate) AS Year,
// COUNT(p.Id) AS TotalPosts,
// AVG(CASE WHEN p.PostTypeId = 1 THEN p.Score END) AS AverageQuestionScore,
// COUNT(DISTINCT b.UserId) AS TotalUsersWithBadges
// FROM
// Posts p
// LEFT JOIN
// Badges b ON p.OwnerUserId = b.UserId
// GROUP BY
// Year
// ORDER BY
// Year DESC;
fn q11719(db: &'static So) -> String {
    let Post { creation_date, post_type_id, score, owner_user, .. } = &db.post;
    let yr = creation_date.map(year);
    let f = db.post.group_by(&yr).select(post_type_id.and(score).and(owner_user.select(badges_of(db)).opt())).fold([0i64; 3], |a, ((t, s), _)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + if t == 1 { s } else { 0 }]
    });
    let u = db.post.group_by(&yr).select(owner_user.select(badges_of(db)).select(&db.badge.user_id)).count_distinct();
    let mut v = Vec::new();
    f.and((&u).opt()).drive(|y, (a, u)| v.push((y, a, u.unwrap_or(0))));
    rows(v.iter().map(|&(y, a, u)| row(vec![V::I(y), V::I(a[0]), avg(a[2], a[1]), V::I(u)])))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// p.AnswerCount,
// p.CommentCount,
// u.Id AS UserId,
// u.DisplayName AS OwnerDisplayName,
// u.Reputation AS OwnerReputation,
// COUNT(v.Id) AS TotalVotes,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount,
// u.Id, u.DisplayName, u.Reputation
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q10171(db: &'static So) -> String {
    stats_rows(db, stats_with(db, owned_since(db, year_ago()), "v", &[], &[]), |p, _| newest(db, p), 100, &["id", "title", "created", "score", "views", "answers", "comments", "uid", "owner", "rep", "#vx", "#up", "#down"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// u.DisplayName AS OwnerDisplayName,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// COALESCE(SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges,
// COALESCE(SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadges,
// COALESCE(SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadges
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, u.DisplayName
// ORDER BY
// p.ViewCount DESC
// LIMIT 100;
fn q13282(db: &'static So) -> String {
    stats_rows(db, stats_with(db, questions(db), "cvb", &[], &[]), |p, _| views_desc(db, p), 100, &["id", "title", "created", "views", "owner", "#cx", "#up", "#down", "#gold", "#silver", "#bronze"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate AS PostCreationDate,
// p.Score AS PostScore,
// p.ViewCount AS PostViewCount,
// u.Id AS UserId,
// u.DisplayName AS UserDisplayName,
// u.Reputation AS UserReputation,
// COUNT(v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// COUNT(c.Id) AS CommentCount,
// COALESCE(SUM(b.Class), 0) AS BadgeCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// WHERE
// p.CreationDate >= '2020-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.Id, u.DisplayName, u.Reputation
// ORDER BY
// p.Score DESC, p.ViewCount DESC;
fn q12548(db: &'static So) -> String {
    let v = stats_with(db, owned_since(db, date(2020, 1, 1)), "vcb", &[], &[]);
    let v: Vec<_> = v.into_iter().collect();
    rows(v.iter().map(|(p, s, _)| {
        let mut f = stat_fields(db, *p, s, &["id", "title", "created", "score", "views", "uid", "owner", "rep", "#vx", "#up", "#down", "#cx"]);
        f.push(V::I(s.bclass));
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate AS PostCreationDate,
// u.DisplayName AS OwnerDisplayName,
// u.Reputation AS OwnerReputation,
// COUNT(v.Id) AS VoteCount,
// COALESCE(SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END), 0) AS UpVotes,
// COALESCE(SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END), 0) AS DownVotes,
// COALESCE(SUM(CASE WHEN vt.Name = 'AcceptedByOriginator' THEN 1 ELSE 0 END), 0) AS AcceptedCount,
// p.ViewCount,
// p.Score,
// p.Tags,
// p.AnswerCount,
// p.CommentCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// VoteTypes vt ON v.VoteTypeId = vt.Id
// GROUP BY
// p.Id, p.Title, p.CreationDate, u.DisplayName, u.Reputation, p.ViewCount, p.Score, p.Tags, p.AnswerCount, p.CommentCount
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q10386(db: &'static So) -> String {
    stats_rows(db, stats_with(db, owned(db), "v", &[], &[]), |p, _| newest(db, p), 100, &["id", "title", "created", "owner", "rep", "#vx", "#upn", "#downn", "#accn", "views", "score", "tags", "answers", "comments"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// p.AnswerCount,
// p.CommentCount,
// u.DisplayName AS OwnerDisplayName,
// pt.Name AS PostType,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// COUNT(DISTINCT bh.Id) AS HistoryCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// PostHistory bh ON p.Id = bh.PostId
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount,
// p.CommentCount, u.DisplayName, pt.Name
// ORDER BY
// p.Score DESC, p.CreationDate DESC;
fn q13906(db: &'static So) -> String {
    let h = per_post_distinct(db, history_of(db));
    stats_rows(db, stats_with(db, owned_since(db, date(2023, 1, 1)), "cvh", &[], &[&h]), |_, _| 0, 0, &["id", "title", "created", "score", "views", "answers", "comments", "owner", "type", "#cx", "#up", "#down", "#d0"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// p.AnswerCount,
// p.CommentCount,
// p.FavoriteCount,
// u.DisplayName AS OwnerDisplayName,
// u.Reputation AS OwnerReputation,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// COUNT(h.Id) AS HistoryCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// PostHistory h ON p.Id = h.PostId
// WHERE
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount,
// p.CommentCount, p.FavoriteCount, u.DisplayName, u.Reputation
// ORDER BY
// p.Score DESC;
fn q10078(db: &'static So) -> String {
    stats_rows(db, stats_with(db, owned_since(db, year_ago()), "cvh", &[], &[]), |_, _| 0, 0, &["id", "title", "created", "score", "views", "answers", "comments", "favorites", "owner", "rep", "#cx", "#up", "#down", "#hx"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// p.AnswerCount,
// p.CommentCount,
// COALESCE(u.DisplayName, 'Community User') AS OwnerDisplayName,
// u.Reputation AS OwnerReputation,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes,
// MAX(b.Date) AS LastBadgeDate
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// VoteTypes vt ON v.VoteTypeId = vt.Id
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, p.CommentCount, u.DisplayName, u.Reputation
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q13192(db: &'static So) -> String {
    let v = stats_with(db, db.post.iq(), "cvb", &[], &[]);
    let mut v: Vec<_> = v.into_iter().map(|(p, s, _)| (newest(db, p), p, s)).collect();
    v.sort_by_key(|x| x.0);
    rows(v.iter().take(100).map(|(_, p, s)| {
        let mut f = stat_fields(db, *p, s, &["id", "title", "created", "views", "score", "answers", "comments"]);
        f.push(V::S(db.post.owner_user.get(*p).map_or("Community User", |u| db.user.display_name.get(u).unwrap())));
        f.extend(stat_fields(db, *p, s, &["rep", "#cx", "#vx", "#upn", "#downn", "bmax"]));
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// p.AnswerCount,
// p.CommentCount,
// p.FavoriteCount,
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(DISTINCT c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
// MAX(b.Date) AS LastBadgeDate
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// WHERE
// p.CreationDate >= '2022-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, p.CommentCount, p.FavoriteCount,
// u.Id, u.DisplayName, u.Reputation
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q11872(db: &'static So) -> String {
    let c = per_post_distinct(db, comments_of(db));
    let x = per_post_distinct(db, votes_of(db));
    stats_rows(db, stats_with(db, owned_since(db, date(2022, 1, 1)), "cvb", &[], &[&c, &x]), |p, _| newest(db, p), 100, &["id", "title", "created", "views", "score", "answers", "comments", "favorites", "uid", "owner", "rep", "#d0", "#d1", "#up", "#down", "bmax"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.Score,
// p.ViewCount,
// p.AnswerCount,
// p.CommentCount,
// p.FavoriteCount,
// p.CreationDate AS PostCreationDate,
// COUNT(c.Id) AS TotalComments,
// U.DisplayName AS UserDisplayName,
// U.Reputation AS UserReputation,
// U.CreationDate AS UserCreationDate,
// COALESCE(SUM(v.BountyAmount), 0) AS TotalBounties,
// AVG(v.VoteTypeId) AS AverageVoteType,
// COUNT(DISTINCT ph.Id) AS PostHistoryCount
// FROM
// Posts p
// LEFT JOIN
// Users U ON p.OwnerUserId = U.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// WHERE
// p.CreationDate >= '2020-01-01'
// GROUP BY
// p.Id, p.Title, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.FavoriteCount, p.CreationDate,
// U.DisplayName, U.Reputation, U.CreationDate
// ORDER BY
// p.ViewCount DESC;
fn q12953(db: &'static So) -> String {
    let h = per_post_distinct(db, history_of(db));
    let v = stats_with(db, since(db, date(2020, 1, 1)), "cvh", &[], &[&h]);
    rows(v.iter().map(|(p, s, d)| {
        let mut f = stat_fields(db, *p, s, &["id", "title", "score", "views", "answers", "comments", "favorites", "created", "#cx", "owner", "rep", "ucreated"]);
        f.extend([V::I(s.bounty_sum), stat_field(s, "vt_avg").unwrap(), V::I(d[0])]);
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// u.DisplayName AS OwnerDisplayName,
// u.Reputation AS OwnerReputation,
// MAX(p.LastActivityDate) AS LastActivity,
// SUM(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS CloseCount,
// SUM(CASE WHEN ph.PostHistoryTypeId = 11 THEN 1 ELSE 0 END) AS ReopenCount,
// AVG(EXTRACT(EPOCH FROM (p.LastActivityDate - p.CreationDate))) AS AvgResponseTimeInSeconds
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName, u.Reputation
// ORDER BY
// p.CreationDate DESC;
fn q11545(db: &'static So) -> String {
    let Post { last_activity_date, creation_date, .. } = &db.post;
    let mut v = Vec::new();
    since(db, date(2023, 1, 1))
        .group_by(Ident::<Post>::new())
        .select(last_activity_date.and(creation_date).and(comments_of(db).opt()).and(votes_of(db).opt()).and(history_of(db).select(&db.post_history.post_history_type_id).opt()))
        .fold((0i64, 0i64, 0i64, 0i64, 0i64, i64::MIN, 0.0f64), |(n, c, x, h10, h11, la, d), ((((l, cd), ci), vi), h)| {
            (n + 1, c + ci.is_some() as i64, x + vi.is_some() as i64, h10 + (h == Some(10)) as i64, h11 + (h == Some(11)) as i64, la.max(l), d + (l - cd) as f64 / 1e6)
        })
        .drive(|p, a| v.push((p, a)));
    rows(v.iter().map(|&(p, (n, c, x, h10, h11, la, d))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(c), V::I(x)]);
        f.extend(post_fields(db, p, &["owner", "rep"]));
        f.extend([V::T(la), V::I(h10), V::I(h11), V::F(d / n as f64)]);
        row(f)
    }))
}

// A float AVG whose printed digits move with DuckDB's SET threads, so
// rewrites/14185.sql takes the exact-integer mean, as 5603's does.
//
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// p.AnswerCount,
// p.CommentCount,
// u.DisplayName AS OwnerDisplayName,
// u.Reputation AS OwnerReputation,
// COUNT(c.Id) AS CommentCount,
// COUNT(CASE WHEN v.VoteTypeId = 2 THEN v.Id END) AS UpVoteCount,
// COUNT(CASE WHEN v.VoteTypeId = 3 THEN v.Id END) AS DownVoteCount,
// AVG(EXTRACT(EPOCH FROM (ph.CreationDate - p.CreationDate))) AS AvgTimeToEdit
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, u.DisplayName, u.Reputation
// ORDER BY
// p.CreationDate DESC;
fn q14185(db: &'static So) -> String {
    let Post { creation_date, .. } = &db.post;
    let mut v = Vec::new();
    questions(db)
        .group_by(Ident::<Post>::new())
        .select(creation_date.and(comments_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(history_of(db).select(&db.post_history.creation_date).opt()))
        .fold((0i64, 0i64, 0i64, 0i64, 0i128), |(c, u, d, hn, hs), (((pc, ci), x), h)| {
            (c + ci.is_some() as i64, u + (x == Some(2)) as i64, d + (x == Some(3)) as i64, hn + h.is_some() as i64, hs + h.map_or(0, |h| (h - pc) as i128))
        })
        .drive(|p, a| v.push((p, a)));
    rows(v.iter().map(|&(p, (c, u, d, hn, hs))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "owner", "rep"]);
        f.extend([V::I(c), V::I(u), V::I(d), if hn == 0 { V::Null } else { V::F(hs as f64 / hn as f64 / 1e6) }]);
        row(f)
    }))
}

// GROUP BY a tuple of the post's columns.
// SELECT
// p.Title AS PostTitle,
// p.CreationDate AS PostCreationDate,
// p.ViewCount AS PostViewCount,
// p.Score AS PostScore,
// u.DisplayName AS OwnerDisplayName,
// u.Reputation AS OwnerReputation,
// COUNT(v.Id) AS TotalVotes,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// COUNT(c.Id) AS CommentCount,
// COALESCE(MAX(ph.CreationDate), p.CreationDate) AS LastEditedDate
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName, u.Reputation
// ORDER BY
// PostViewCount DESC;
fn q11563(db: &'static So) -> String {
    let Post { title, creation_date, view_count, score, owner_user, .. } = &db.post;
    let key = title.opt().and(creation_date).and(view_count.opt()).and(score).and(owner_user.select((&db.user.display_name).and(&db.user.reputation)).opt());
    let v = group_stats(db, since(db, date(2023, 1, 1)), key, "vch", &[]);
    rows(v.iter().map(|&(((((t, c), w), sc), u), ref s)| {
        row(vec![
            ostr(t),
            V::T(c),
            oint(w),
            V::I(sc),
            ostr(u.map(|u| u.0)),
            oint(u.map(|u| u.1)),
            V::I(s.vx),
            V::I(s.up),
            V::I(s.down),
            V::I(s.cx),
            V::T(if s.hx == 0 { c } else { s.hmax }),
        ])
    }))
}

// Users grouped with their posts and badges.
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN c.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalComments,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS TotalGoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS TotalSilverBadges,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS TotalBronzeBadges
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// u.Id, u.DisplayName
// ORDER BY
// TotalPosts DESC
// LIMIT 100;
fn q12821(db: &'static So) -> String {
    let v = users_stats_with(db, UserWhere::All, "cvb", any_post, &[], &[]);
    users_rows(db, v, |_, s, _| Reverse(s.n), 100, &["uid", "name", "#n", "#q", "#a", "#cx", "#up", "#down", "#gold", "#silver", "#bronze"])
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(p.Id) AS PostCount,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId AND p.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year')
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// WHERE
// u.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year')
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
// ORDER BY
// PostCount DESC, UpVotes DESC;
fn q12335(db: &'static So) -> String {
    fn recent(_: i64, c: i64) -> bool {
        c >= ts(2023, 10, 1, 12, 34, 56)
    }
    let v = users_stats_with(db, UserWhere::CreatedGe(year_ago()), "cvb", recent, &[], &[]);
    users_rows(db, v, |_, _, _| 0, 0, &["uid", "name", "rep", "#n", "#cx", "#up", "#down", "#gold", "#silver", "#bronze"])
}

// SELECT
// u.DisplayName AS UserDisplayName,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// AVG(u.Reputation) AS AvgReputation,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS TotalGoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS TotalSilverBadges,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS TotalBronzeBadges,
// COUNT(DISTINCT ph.Id) AS TotalPostHistoryEntries
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// WHERE
// u.CreationDate >= '2020-01-01'
// GROUP BY
// u.DisplayName, u.Reputation
// HAVING
// COUNT(DISTINCT p.Id) > 10
// ORDER BY
// AvgReputation DESC, TotalPosts DESC;
fn q6050(db: &'static So) -> String {
    let w = UserWhere::CreatedGe(date(2020, 1, 1));
    let key = (&db.user.display_name).and(&db.user.reputation);
    let f = user_stats_fold(db, &key, w, "bh", any_post);
    let p = user_distinct(db, &key, w, posts_of(db));
    let h = user_distinct(db, &key, w, posts_of(db).select(history_of(db)));
    let mut v = Vec::new();
    f.and((&p).opt()).and((&h).opt()).filt(|((_, p), _)| p.unwrap_or(0) > 10).drive(|k, ((s, p), h)| v.push((k, s, p.unwrap_or(0), h.unwrap_or(0))));
    rows(v.iter().map(|&((dn, _), ref s, p, h)| {
        let mut f = vec![V::S(dn), V::I(p)];
        f.extend(["#q", "#a", "rep_avg", "#gold", "#silver", "#bronze"].iter().map(|c| ustat_field(s, c)));
        f.push(V::I(h));
        row(f)
    }))
}


// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.Score,
// P.ViewCount,
// P.AnswerCount,
// P.CommentCount,
// U.DisplayName AS OwnerDisplayName,
// U.Reputation AS OwnerReputation,
// PH.PostHistoryTypeId,
// PH.CreationDate AS PostHistoryDate,
// COUNT(V.Id) AS VoteCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// SUM(CASE WHEN B.Id IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// PostHistory PH ON P.Id = PH.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// WHERE
// P.CreationDate >= DATE '2023-01-01'
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.Score, P.ViewCount, P.AnswerCount, P.CommentCount,
// U.Id, U.DisplayName, U.Reputation, PH.PostHistoryTypeId, PH.CreationDate
// ORDER BY
// P.CreationDate DESC;
fn q12174(db: &'static So) -> String {
    let PostHistory { post_history_type_id, creation_date, .. } = &db.post_history;
    let j: MatSet<(Id<Post>, Option<Id<PostHistory>>)> = owned_since(db, date(2023, 1, 1)).select(Ident::<Post>::new().and(history_of(db).opt())).collect();
    let post_of = (&j).map(|(p, _)| p);
    let key = (&post_of).and((&j).flat_map(|(_, h)| h).select(post_history_type_id.and(creation_date)).opt());
    let mut v = Vec::new();
    (&j).group_by(key)
        .select((&post_of).select(votes_of(db).select(&db.vote.vote_type_id).opt().and((&db.post.owner_user).select(badges_of(db)).opt())))
        .fold([0i64; 4], |a, (x, b)| [a[0] + x.is_some() as i64, a[1] + (x == Some(2)) as i64, a[2] + (x == Some(3)) as i64, a[3] + b.is_some() as i64])
        .drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&((p, h), a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "owner", "rep"]);
        f.extend([oint(h.map(|h| h.0)), ots(h.map(|h| h.1))]);
        f.extend(a.iter().map(|&x| V::I(x)));
        row(f)
    }))
}

// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// U.DisplayName AS OwnerDisplayName,
// COUNT(CM.Id) AS TotalComments,
// COUNT(V.Id) AS TotalVotes,
// SUM(CASE WHEN VT.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN VT.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes,
// P.ViewCount,
// P.Score,
// P.AnswerCount,
// P.CommentCount,
// PH.CreationDate AS LastEditDate,
// PH.UserDisplayName AS LastEditor
// FROM
// Posts P
// LEFT JOIN
// Comments CM ON P.Id = CM.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// LEFT JOIN
// VoteTypes VT ON V.VoteTypeId = VT.Id
// LEFT JOIN
// PostHistory PH ON P.Id = PH.PostId
// LEFT JOIN
// Users U ON P.OwnerUserId = U.Id
// WHERE
// P.CreationDate > (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days')
// GROUP BY
// P.Id,
// P.Title,
// P.CreationDate,
// U.DisplayName,
// P.ViewCount,
// P.Score,
// P.AnswerCount,
// P.CommentCount,
// PH.CreationDate,
// PH.UserDisplayName
// ORDER BY
// P.CreationDate DESC;
fn q14233(db: &'static So) -> String {
    let PostHistory { user_display_name, creation_date, .. } = &db.post_history;
    let base = db.post.with((&db.post.creation_date).gt(month_ago()));
    let j: MatSet<(Id<Post>, Option<Id<PostHistory>>)> = base.select(Ident::<Post>::new().and(history_of(db).opt())).collect();
    let post_of = (&j).map(|(p, _)| p);
    let key = (&post_of).and((&j).flat_map(|(_, h)| h).select(creation_date.and(user_display_name.opt())).opt());
    let mut v = Vec::new();
    (&j).group_by(key)
        .select((&post_of).select(comments_of(db).opt().and(votes_of(db).select((&db.vote.vote_type).select(&db.vote_type.name).opt()).opt())))
        .fold([0i64; 4], |a, (c, x)| {
            let n = x.flatten();
            [a[0] + c.is_some() as i64, a[1] + x.is_some() as i64, a[2] + (n == Some("UpMod")) as i64, a[3] + (n == Some("DownMod")) as i64]
        })
        .drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&((p, h), a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend(a.iter().map(|&x| V::I(x)));
        f.extend(post_fields(db, p, &["views", "score", "answers", "comments"]));
        f.extend([ots(h.map(|h| h.0)), ostr(h.and_then(|h| h.1))]);
        row(f)
    }))
}

// SELECT
// P.Id AS PostId,
// P.Title,
// U.DisplayName AS OwnerDisplayName,
// P.CreationDate,
// P.ViewCount,
// P.Score,
// COUNT(CASE WHEN C.PostId = P.Id THEN 1 END) AS CommentCount,
// COUNT(CASE WHEN V.PostId = P.Id THEN 1 END) AS VoteCount,
// T.TagName,
// PT.Name AS PostType,
// BH.UserDisplayName AS LastEditedBy,
// BH.CreationDate AS LastEditedDate
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// LEFT JOIN
// Tags T ON T.ExcerptPostId = P.Id
// LEFT JOIN
// PostTypes PT ON P.PostTypeId = PT.Id
// LEFT JOIN
// PostHistory BH ON P.Id = BH.PostId
// WHERE
// P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY
// P.Id, U.DisplayName, P.Title, P.CreationDate, P.ViewCount, P.Score, T.TagName, PT.Name, BH.UserDisplayName, BH.CreationDate
// ORDER BY
// P.CreationDate DESC;
fn q10786(db: &'static So) -> String {
    let PostHistory { user_display_name, creation_date, .. } = &db.post_history;
    let tag: HashIdx<Id<Post>, Str> = (&db.tag.excerpt_post).inv().select(&db.tag.tag_name).collect();
    let j: MatSet<(Id<Post>, Option<Id<PostHistory>>)> = owned_since(db, year_ago()).select(Ident::<Post>::new().and(history_of(db).opt())).collect();
    let post_of = (&j).map(|(p, _)| p);
    let key = (&post_of).and((&post_of).select((&tag).opt())).and((&j).flat_map(|(_, h)| h).select(user_display_name.opt().and(creation_date)).opt());
    let mut v = Vec::new();
    (&j).group_by(key)
        .select((&post_of).select(comments_of(db).opt().and(votes_of(db).opt())))
        .fold([0i64; 2], |a, (c, x)| [a[0] + c.is_some() as i64, a[1] + x.is_some() as i64])
        .drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&(((p, tn), h), a)| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "views", "score"]);
        f.extend([V::I(a[0]), V::I(a[1]), ostr(tn)]);
        f.extend(post_fields(db, p, &["type"]));
        f.extend([ostr(h.and_then(|h| h.0)), ots(h.map(|h| h.1))]);
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// p.AnswerCount,
// u.DisplayName AS OwnerDisplayName,
// u.Reputation AS OwnerReputation,
// t.TagName,
// ph.CreationDate AS HistoryCreationDate,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// JOIN
// Tags t ON t.ExcerptPostId = p.Id
// LEFT JOIN
// Comments c ON c.PostId = p.Id
// LEFT JOIN
// Votes v ON v.PostId = p.Id
// LEFT JOIN
// PostHistory ph ON ph.PostId = p.Id
// WHERE
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount,
// u.DisplayName, u.Reputation, t.TagName, ph.CreationDate
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q12975(db: &'static So) -> String {
    let tag: HashIdx<Id<Post>, Str> = (&db.tag.excerpt_post).inv().select(&db.tag.tag_name).collect();
    let base = owned_since(db, year_ago());
    let j: MatSet<(Id<Post>, Option<Id<PostHistory>>)> = base.with(&tag).select(Ident::<Post>::new().and(history_of(db).opt())).collect();
    let post_of = (&j).map(|(p, _)| p);
    let key = (&post_of).and((&post_of).select(&tag)).and((&j).flat_map(|(_, h)| h).select(&db.post_history.creation_date).opt());
    let mut v = Vec::new();
    (&j).group_by(key)
        .select((&post_of).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())))
        .fold([0i64; 3], |a, (c, x)| [a[0] + c.is_some() as i64, a[1] + (x == Some(2)) as i64, a[2] + (x == Some(3)) as i64])
        .drive(|k, a| v.push((k, a)));
    v.sort_by_key(|&(((p, _), h), _)| (newest(db, p), db.post.origid.get(p).unwrap(), h.is_none(), h));
    rows(v.iter().take(100).map(|&(((p, tn), h), a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "owner", "rep"]);
        f.extend([V::S(tn), ots(h)]);
        f.extend(a.iter().map(|&x| V::I(x)));
        row(f)
    }))
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.PostTypeId,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// p.AnswerCount,
// COUNT(c.Id) AS CommentCount,
// p.FavoriteCount,
// u.Reputation AS OwnerReputation,
// p.Tags,
// h.CreationDate AS LastEditDate,
// AVG(vot.BountyAmount) AS AverageBounty
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// PostHistory h ON p.Id = h.PostId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes vot ON p.Id = vot.PostId
// GROUP BY
// p.Id, u.Reputation, p.PostTypeId, p.CreationDate, p.Score,
// p.ViewCount, p.AnswerCount, p.FavoriteCount,
// p.Tags, h.CreationDate
// )
// SELECT
// PostId,
// PostTypeId,
// CreationDate,
// Score,
// ViewCount,
// AnswerCount,
// CommentCount,
// FavoriteCount,
// OwnerReputation,
// Tags,
// LastEditDate,
// AverageBounty
// FROM
// PostStats
// ORDER BY
// Score DESC,
// ViewCount DESC;
fn q13369(db: &'static So) -> String {
    let j: MatSet<(Id<Post>, Option<Id<PostHistory>>)> = db.post.select(Ident::<Post>::new().and(history_of(db).opt())).collect();
    let post_of = (&j).map(|(p, _)| p);
    let key = (&post_of).and((&j).flat_map(|(_, h)| h).select(&db.post_history.creation_date).opt());
    let mut v = Vec::new();
    (&j).group_by(key)
        .select((&post_of).select(comments_of(db).opt().and(votes_of(db).select((&db.vote.bounty_amount).opt()).opt())))
        .fold([0i64; 3], |a, (c, x)| {
            let b = x.flatten();
            [a[0] + c.is_some() as i64, a[1] + b.is_some() as i64, a[2] + b.unwrap_or(0)]
        })
        .drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&((p, h), a)| {
        let mut f = post_fields(db, p, &["id", "type_id", "created", "score", "views", "answers"]);
        f.push(V::I(a[0]));
        f.extend(post_fields(db, p, &["favorites", "rep", "tags"]));
        f.extend([ots(h), avg(a[2], a[1])]);
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.Score,
// p.ViewCount,
// COUNT(DISTINCT c.Id) AS CommentCount,
// COUNT(DISTINCT a.Id) AS AnswerCount,
// u.DisplayName AS OwnerDisplayName,
// u.Reputation AS OwnerReputation,
// MAX(v.CreationDate) AS LastVoteDate,
// ph.CreationDate AS LastEditDate,
// pt.Name AS PostType,
// bt.Name AS BadgeName
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// LEFT JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id
// LEFT JOIN
// Badges bt ON b.Id = bt.Id
// WHERE
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY
// p.Id, p.Title, p.Score, p.ViewCount, u.DisplayName, u.Reputation, ph.CreationDate, pt.Name, bt.Name
// ORDER BY
// p.Score DESC, p.ViewCount DESC;
fn q10589(db: &'static So) -> String {
    let j: MatSet<(Id<Post>, Option<Id<PostHistory>>, Option<Id<Badge>>)> = owned_since(db, year_ago())
        .select(Ident::<Post>::new().and(history_of(db).opt()).and((&db.post.owner_user).select(badges_of(db)).opt()))
        .map(|((p, h), b)| (p, h, b))
        .collect();
    let post_of = (&j).map(|(p, _, _)| p);
    let key = (&post_of)
        .and((&j).flat_map(|(_, h, _)| h).select(&db.post_history.creation_date).opt())
        .and((&post_of).select((&db.post.post_type).select(&db.post_type.name)).opt())
        .and((&j).flat_map(|(_, _, b)| b).select(&db.badge.name).opt());
    let f = (&j).group_by(&key).select((&post_of).select(votes_of(db).select(&db.vote.creation_date).opt())).fold((0i64, i64::MIN), |(n, m), d| match d {
        Some(d) => (n + 1, m.max(d)),
        None => (n, m),
    });
    let c = (&j).group_by(&key).select((&post_of).select(comments_of(db))).count_distinct();
    let a = (&j).group_by(&key).select((&post_of).select(answers_of(db))).count_distinct();
    let mut v = Vec::new();
    f.and((&c).opt()).and((&a).opt()).drive(|k, ((f, c), a)| v.push((k, f, c.unwrap_or(0), a.unwrap_or(0))));
    rows(v.iter().map(|&((((p, hd), pt), bn), (n, m), c, a)| {
        let mut r = post_fields(db, p, &["id", "title", "score", "views"]);
        r.extend([V::I(c), V::I(a)]);
        r.extend(post_fields(db, p, &["owner", "rep"]));
        r.extend([if n == 0 { V::Null } else { V::T(m) }, ots(hd), ostr(pt), ostr(bn)]);
        row(r)
    }))
}

// WITH PostStats AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.LastActivityDate,
// P.Score,
// P.AnswerCount,
// P.ViewCount,
// P.CommentCount,
// U.DisplayName AS OwnerDisplayName,
// T.TagName,
// PH.CreationDate AS HistoryCreationDate,
// PH.PostHistoryTypeId
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// JOIN
// Tags T ON T.Id = P.Id
// LEFT JOIN
// PostHistory PH ON P.Id = PH.PostId
// )
// SELECT
// PS.OwnerDisplayName,
// PS.Title,
// PS.CreationDate,
// PS.LastActivityDate,
// PS.Score,
// PS.AnswerCount,
// PS.ViewCount,
// PS.CommentCount,
// PS.TagName,
// COUNT(PH.PostHistoryTypeId) AS HistoryCount
// FROM
// PostStats PS
// LEFT JOIN
// PostHistory PH ON PS.PostId = PH.PostId
// GROUP BY
// PS.OwnerDisplayName, PS.Title, PS.CreationDate, PS.LastActivityDate, PS.Score, PS.AnswerCount, PS.ViewCount, PS.CommentCount, PS.TagName
// ORDER BY
// PS.LastActivityDate DESC
// LIMIT 100;
fn q12670(db: &'static So) -> String {
    let Post { title, creation_date, last_activity_date, score, answer_count, view_count, comment_count, owner_user, origid, .. } = &db.post;
    let tag_by_raw: HashIdx<i64, Str> = (&db.tag.origid).inv().select(&db.tag.tag_name).collect();
    let key = owner_user
        .select(&db.user.display_name)
        .and(title.opt())
        .and(creation_date)
        .and(last_activity_date)
        .and(score)
        .and(answer_count.opt())
        .and(view_count.opt())
        .and(comment_count)
        .and(origid.select(&tag_by_raw));
    let mut v = Vec::new();
    owned(db)
        .group_by(key)
        .select(history_of(db).opt().and(history_of(db).opt()))
        .fold(0i64, |n, (_, h)| n + h.is_some() as i64)
        .drive(|k, n| v.push((k, n)));
    v.sort_by_key(|x| Reverse(x.0.0.0.0.0.0.1));
    rows(v.iter().take(100).map(|&(((((((((dn, t), c), la), s), an), w), cc), tn), n)| {
        row(vec![V::S(dn), ostr(t), V::T(c), V::T(la), V::I(s), oint(an), oint(w), V::I(cc), V::S(tn), V::I(n)])
    }))
}

// A per-post fold, then an aggregate with no group over it.
// WITH BenchmarkData AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// p.AnswerCount,
// p.CommentCount,
// p.FavoriteCount,
// u.Reputation AS AuthorReputation,
// u.CreationDate AS AuthorCreationDate,
// COUNT(c.Id) AS TotalComments,
// COUNT(v.Id) AS TotalVotes
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.PostTypeId IN (1, 2)
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount,
// p.AnswerCount, p.CommentCount, p.FavoriteCount,
// u.Reputation, u.CreationDate
// )
// SELECT
// AVG(Score) AS AvgScore,
// AVG(ViewCount) AS AvgViewCount,
// AVG(AnswerCount) AS AvgAnswerCount,
// AVG(CommentCount) AS AvgCommentCount,
// AVG(FavoriteCount) AS AvgFavoriteCount,
// AVG(AuthorReputation) AS AvgAuthorReputation,
// MIN(AuthorCreationDate) AS EarliestAuthorCreationDate,
// MAX(CreationDate) AS MostRecentPostDate,
// COUNT(PostId) AS TotalPosts
// FROM
// BenchmarkData
fn q11308(db: &'static So) -> String {
    let Post { score, view_count, answer_count, comment_count, favorite_count, owner_user, creation_date, post_type_id, .. } = &db.post;
    let base = db.post.with(post_type_id.in_v(vec![1, 2]));
    let per_post = (&base).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(votes_of(db).opt())).fold(0i64, |n, _| n + 1);
    let a = (&per_post)
        .and(score.and(view_count.opt()).and(answer_count.opt()).and(comment_count).and(favorite_count.opt()).and(owner_user.select((&db.user.reputation).and(&db.user.creation_date)).opt()).and(creation_date))
        .fold_flat([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, i64::MAX, i64::MIN], |a: [i64; 13], (_, ((((((s, w), an), c), f), u), cd))| {
            [
                a[0] + 1,
                a[1] + s,
                a[2] + w.is_some() as i64,
                a[3] + w.unwrap_or(0),
                a[4] + an.is_some() as i64,
                a[5] + an.unwrap_or(0),
                a[6] + c,
                a[7] + f.is_some() as i64,
                a[8] + f.unwrap_or(0),
                a[9] + u.is_some() as i64,
                a[10] + u.map_or(0, |u| u.0),
                u.map_or(a[11], |u| a[11].min(u.1)),
                a[12].max(cd),
            ]
        });
    row(vec![avg(a[1], a[0]), avg(a[3], a[2]), avg(a[5], a[4]), avg(a[6], a[0]), avg(a[8], a[7]), avg(a[10], a[9]), if a[9] == 0 { V::Null } else { V::T(a[11]) }, if a[0] == 0 { V::Null } else { V::T(a[12]) }, V::I(a[0])])
}

// WITH Benchmark AS (
// SELECT
// PH.PostId,
// COUNT(CASE WHEN PH.PostHistoryTypeId = 10 THEN 1 END) AS CloseCount,
// COUNT(CASE WHEN PH.PostHistoryTypeId = 11 THEN 1 END) AS ReopenCount,
// COUNT(CASE WHEN PH.PostHistoryTypeId IN (24, 25) THEN 1 END) AS EditCount,
// COUNT(CASE WHEN PH.PostHistoryTypeId = 12 THEN 1 END) AS DeleteCount,
// COUNT(CASE WHEN PH.PostHistoryTypeId = 13 THEN 1 END) AS UndeleteCount,
// COUNT(CASE WHEN PH.PostHistoryTypeId = 14 THEN 1 END) AS LockCount,
// COUNT(CASE WHEN PH.PostHistoryTypeId = 15 THEN 1 END) AS UnlockCount,
// COUNT(CASE WHEN PH.PostHistoryTypeId = 19 THEN 1 END) AS ProtectCount,
// COUNT(CASE WHEN PH.PostHistoryTypeId = 20 THEN 1 END) AS UnprotectCount,
// COUNT(CASE WHEN PH.PostHistoryTypeId IN (1, 4, 5) THEN 1 END) AS TitleOrBodyEditCount,
// COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS QuestionCount,
// COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS AnswerCount,
// COUNT(DISTINCT V.UserId) AS UniqueVoters,
// AVG(U.Reputation) AS AverageUserReputation
// FROM
// PostHistory PH
// JOIN
// Posts P ON PH.PostId = P.Id
// JOIN
// Votes V ON V.PostId = P.Id
// JOIN
// Users U ON V.UserId = U.Id
// GROUP BY
// PH.PostId
// )
// SELECT
// PostId,
// CloseCount,
// ReopenCount,
// EditCount,
// DeleteCount,
// UndeleteCount,
// LockCount,
// UnlockCount,
// ProtectCount,
// UnprotectCount,
// TitleOrBodyEditCount,
// QuestionCount,
// AnswerCount,
// UniqueVoters,
// AverageUserReputation
// FROM
// Benchmark
// ORDER BY
// PostId;
fn q14901(db: &'static So) -> String {
    let Vote { user, .. } = &db.vote;
    let voters = votes_of(db).select(user.select((&db.user.reputation).and(&db.user.origid)));
    let f = db.post.group_by(Ident::<Post>::new()).select((&db.post.post_type_id).and(history_of(db).select(&db.post_history.post_history_type_id)).and(&voters)).fold(
        [0i64; 14],
        |a, ((t, h), (r, _))| {
            let c = |x: bool| x as i64;
            [
                a[0] + c(h == 10),
                a[1] + c(h == 11),
                a[2] + c(h == 24 || h == 25),
                a[3] + c(h == 12),
                a[4] + c(h == 13),
                a[5] + c(h == 14),
                a[6] + c(h == 15),
                a[7] + c(h == 19),
                a[8] + c(h == 20),
                a[9] + c(h == 1 || h == 4 || h == 5),
                a[10] + c(t == 1),
                a[11] + c(t == 2),
                a[12] + r,
                a[13] + 1,
            ]
        },
    );
    let u = db.post.group_by(Ident::<Post>::new()).select(history_of(db).and(&voters).map(|(_, (_, id))| id)).count_distinct();
    let mut v = Vec::new();
    f.and(&u).drive(|p, (a, u)| v.push((p, a, u)));
    rows(v.iter().map(|&(p, a, u)| {
        let mut r = post_fields(db, p, &["id"]);
        r.extend(a[..12].iter().map(|&x| V::I(x)));
        r.extend([V::I(u), avg(a[12], a[13])]);
        row(r)
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("15146", q15146),
    ("18658", q18658),
    ("17889", q17889),
    ("19579", q19579),
    ("15810", q15810),
    ("19670", q19670),
    ("15171", q15171),
    ("15384", q15384),
    ("15480", q15480),
    ("15598", q15598),
    ("15861", q15861),
    ("16345", q16345),
    ("16727", q16727),
    ("19397", q19397),
    ("17928", q17928),
    ("19254", q19254),
    ("15114", q15114),
    ("17045", q17045),
    ("17604", q17604),
    ("15363", q15363),
    ("16192", q16192),
    ("14375", q14375),
    ("10099", q10099),
    ("10347", q10347),
    ("12872", q12872),
    ("11014", q11014),
    ("13836", q13836),
    ("14455", q14455),
    ("13241", q13241),
    ("10869", q10869),
    ("11719", q11719),
    ("10171", q10171),
    ("13282", q13282),
    ("12548", q12548),
    ("10386", q10386),
    ("13906", q13906),
    ("10078", q10078),
    ("13192", q13192),
    ("11872", q11872),
    ("12953", q12953),
    ("11545", q11545),
    ("14185", q14185),
    ("11563", q11563),
    ("12821", q12821),
    ("12335", q12335),
    ("6050", q6050),
    ("12174", q12174),
    ("14233", q14233),
    ("10786", q10786),
    ("12975", q12975),
    ("13369", q13369),
    ("10589", q10589),
    ("12670", q12670),
    ("11308", q11308),
    ("14901", q14901),
];
