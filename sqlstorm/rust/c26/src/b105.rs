use harness::prelude::*;
use std::cmp::Reverse;

fn questions(db: &'static So) -> impl Drive<D = Id<Post>, R = Id<Post>> {
    owned(db).with((&db.post.post_type_id).eq(1))
}

fn since(db: &'static So, d: i64) -> impl Drive<D = Id<Post>, R = Id<Post>> {
    db.post.with((&db.post.creation_date).ge(d))
}

fn views_desc(db: &'static So, p: Id<Post>) -> (bool, Reverse<Option<i64>>) {
    let w = db.post.view_count.get(p);
    (w.is_none(), Reverse(w))
}

// Posts grouped by `key`, folding what each post is joined to.
fn badges_by_uid(db: &'static So) -> HashIdx<i64, Id<Badge>> {
    (&db.badge.user_id).inv().collect()
}

fn cvb<Q: Drive<D = Id<Post>, R = Id<Post>>>(db: &'static So, base: Q, cols: &[&str]) -> String {
    let bidx = badges_by_uid(db);
    let f = base
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and((&db.post.owner_user_id).select(&bidx).opt()))
        .fold([0i64; 3], |a, ((c, t), _)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let b = db.post.group_by(Ident::<Post>::new()).select((&db.post.owner_user_id).select(&bidx)).count_distinct();
    let mut v = Vec::new();
    f.and((&b).opt()).drive(|p, (a, n)| v.push((p, [a[0], a[1], a[2], n.unwrap_or(0)])));
    rows(v.iter().map(|&(p, a)| {
        row(cols
            .iter()
            .map(|c| match *c {
                "#cx" => V::I(a[0]),
                "#up" => V::I(a[1]),
                "#down" => V::I(a[2]),
                "#d0" => V::I(a[3]),
                _ => post_fields(db, p, &[c]).pop().unwrap(),
            })
            .collect())
    }))
}

fn fold_by<Q, K, R, S, F>(base: Q, key: K, joined: R, init: S, f: F) -> Fold<ROf<K>, S>
where
    Q: Drive<D = Id<Post>, R = Id<Post>>,
    K: IntoQuery,
    K::Q: Probe<D = Id<Post>>,
    ROf<K>: Eq + std::hash::Hash + Copy,
    R: IntoQuery,
    R::Q: Probe<D = Id<Post>>,
    S: Copy,
    F: Fn(S, ROf<R>) -> S,
{
    base.group_by(key).select(joined).fold(init, f)
}

fn listed<K: Copy + Eq + std::hash::Hash, S: Copy>(f: &Fold<K, S>) -> Vec<(K, S)> {
    let mut v = Vec::new();
    f.drive(|k, a| v.push((k, a)));
    v
}

fn by_first<K, const N: usize>(mut v: Vec<(K, [i64; N])>) -> Vec<(K, [i64; N])> {
    v.sort_by_key(|x| Reverse(x.1[0]));
    v
}

fn name(db: &'static So) -> Compose<&'static Col<Post, Id<PostType>>, &'static Col<PostType, Str>> {
    (&db.post.post_type).select(&db.post_type.name)
}

fn distinct_by<Q, K, R>(base: Q, key: K, r: R) -> Fold<ROf<K>, i64>
where
    Q: Drive<D = Id<Post>, R = Id<Post>>,
    K: IntoQuery,
    K::Q: Probe<D = Id<Post>>,
    ROf<K>: Eq + std::hash::Hash + Copy,
    R: IntoQuery,
    R::Q: Probe<D = Id<Post>>,
    ROf<R>: Ord,
{
    base.group_by(key).select(r).count_distinct()
}

// A fold per key beside one COUNT(DISTINCT ...) per key.
fn with1<K: Copy + Eq + std::hash::Hash, const N: usize>(f: Fold<K, [i64; N]>, d: Fold<K, i64>) -> Vec<(K, [i64; N], i64)> {
    let mut v = Vec::new();
    f.and((&d).opt()).drive(|k, (a, x)| v.push((k, a, x.unwrap_or(0))));
    v.sort_by_key(|x| Reverse(x.1[0]));
    v
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// COUNT(DISTINCT p.OwnerUserId) AS UniqueUsers,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes,
// AVG(p.Score) AS AverageScore
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
fn q13325(db: &'static So) -> String {
    let f = fold_by(db.post.iq(), name(db), (&db.post.score).and(votes_of(db).select(&db.vote.vote_type_id).opt()), [0i64; 4], |a, (s, x)| {
        [a[0] + 1, a[1] + (x == Some(2)) as i64, a[2] + (x == Some(3)) as i64, a[3] + s]
    });
    let u = distinct_by(db.post.iq(), name(db), &db.post.owner_user_id);
    rows(with1(f, u).iter().map(|&(k, a, u)| row(vec![V::S(k), V::I(a[0]), V::I(u), V::I(a[1]), V::I(a[2]), avg(a[3], a[0])])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes,
// COUNT(DISTINCT c.Id) AS TotalComments
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q10737(db: &'static So) -> String {
    let f = fold_by(db.post.iq(), name(db), votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()), [0i64; 3], |a, (x, _)| {
        [a[0] + 1, a[1] + (x == Some(2)) as i64, a[2] + (x == Some(3)) as i64]
    });
    let c = distinct_by(db.post.iq(), name(db), comments_of(db));
    rows(with1(f, c).iter().map(|&(k, a, c)| row(vec![V::S(k), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(c)])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AvgScore,
// AVG(p.ViewCount) AS AvgViewCount,
// SUM(CASE WHEN b.UserId IS NOT NULL THEN 1 ELSE 0 END) AS TotalBadges,
// AVG(u.Reputation) AS AvgUserReputation
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Badges b ON b.UserId = u.Id
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q11251(db: &'static So) -> String {
    let Post { score, view_count, owner_user, .. } = &db.post;
    let f = fold_by(db.post.iq(), name(db), score.and(view_count.opt()).and(owner_user.select((&db.user.reputation).and(badges_of(db).opt())).opt()), [0i64; 7], |a, ((s, w), u)| {
        let (r, b) = match u {
            Some((r, b)) => (Some(r), b),
            None => (None, None),
        };
        [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + b.is_some() as i64, a[5] + r.is_some() as i64, a[6] + r.unwrap_or(0)]
    });
    rows(by_first(listed(&f)).iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), avg(a[3], a[2]), V::I(a[4]), avg(a[6], a[5])])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS PostCount,
// AVG(p.Score) AS AverageScore,
// SUM(CASE WHEN p.OwnerUserId IS NOT NULL THEN 1 ELSE 0 END) AS PostsByUsers,
// SUM(CASE WHEN p.OwnerUserId IS NULL THEN 1 ELSE 0 END) AS CommunityPosts,
// SUM(u.Reputation) AS TotalUserReputation
// FROM
// Posts p
// LEFT JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// GROUP BY
// pt.Name
// ORDER BY
// PostCount DESC;
fn q14420(db: &'static So) -> String {
    let Post { score, owner_user_id, owner_user, .. } = &db.post;
    let f = fold_by(db.post.iq(), name(db).opt(), score.and(owner_user_id.opt()).and(owner_user.select(&db.user.reputation).opt()), [0i64; 6], |a, ((s, o), r)| {
        [a[0] + 1, a[1] + s, a[2] + o.is_some() as i64, a[3] + o.is_none() as i64, a[4] + r.is_some() as i64, a[5] + r.unwrap_or(0)]
    });
    rows(by_first(listed(&f)).iter().map(|&(k, a)| row(vec![ostr(k), V::I(a[0]), avg(a[1], a[0]), V::I(a[2]), V::I(a[3]), nullable(a[5], a[4])])))
}

// SELECT
// p.PostTypeId,
// COUNT(p.Id) AS TotalPosts,
// SUM(p.ViewCount) AS TotalViews,
// AVG(p.Score) AS AverageScore,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes,
// COUNT(DISTINCT u.Id) AS TotalUsers
// FROM
// Posts p
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// GROUP BY
// p.PostTypeId
// ORDER BY
// TotalPosts DESC;
fn q10872(db: &'static So) -> String {
    let Post { view_count, score, owner_user, post_type_id, .. } = &db.post;
    let f = fold_by(db.post.iq(), post_type_id, view_count.opt().and(score).and(votes_of(db).select(&db.vote.vote_type_id).opt()), [0i64; 6], |a, ((w, s), x)| {
        [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s, a[4] + (x == Some(2)) as i64, a[5] + (x == Some(3)) as i64]
    });
    let u = distinct_by(db.post.iq(), post_type_id, owner_user);
    rows(with1(f, u).iter().map(|&(t, a, u)| row(vec![V::I(t), V::I(a[0]), nullable(a[2], a[1]), avg(a[3], a[0]), V::I(a[4]), V::I(a[5]), V::I(u)])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS PostCount,
// AVG(p.ViewCount) AS AverageViews,
// AVG(p.Score) AS AverageScore,
// SUM(CASE WHEN c.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalComments,
// SUM(CASE WHEN v.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalVotes
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// pt.Name
// ORDER BY
// PostCount DESC;
fn q14943(db: &'static So) -> String {
    let Post { view_count, score, .. } = &db.post;
    let f = fold_by(db.post.iq(), name(db), view_count.opt().and(score).and(comments_of(db).opt()).and(votes_of(db).opt()), [0i64; 6], |a, (((w, s), c), x)| {
        [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s, a[4] + c.is_some() as i64, a[5] + x.is_some() as i64]
    });
    rows(by_first(listed(&f)).iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), avg(a[2], a[1]), avg(a[3], a[0]), V::I(a[4]), V::I(a[5])])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.ViewCount) AS AverageViewCount,
// MAX(p.Score) AS MaxScore,
// SUM(CASE WHEN p.OwnerUserId IS NOT NULL THEN 1 ELSE 0 END) AS PostsWithOwners,
// SUM(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS QuestionsWithAcceptedAnswers
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q10435(db: &'static So) -> String {
    let Post { view_count, score, owner_user_id, accepted_answer_id, .. } = &db.post;
    let f = fold_by(db.post.iq(), name(db), view_count.opt().and(score).and(owner_user_id.opt()).and(accepted_answer_id.opt()), [0, 0, 0, i64::MIN, 0, 0], |a: [i64; 6], (((w, s), o), ac)| {
        [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3].max(s), a[4] + o.is_some() as i64, a[5] + ac.is_some() as i64]
    });
    rows(by_first(listed(&f)).iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), avg(a[2], a[1]), V::I(a[3]), V::I(a[4]), V::I(a[5])])))
}

// Types with SUM(ViewCount), AVG(Score), the mean of LastActivityDate -
// CreationDate, COUNT(DISTINCT OwnerUserId) and the accepted answers.
fn types_activity(db: &'static So) -> Vec<(Str, [i64; 5], i128, i64)> {
    let Post { score, view_count, accepted_answer_id, last_activity_date, creation_date, owner_user_id, .. } = &db.post;
    let f = fold_by(db.post.iq(), name(db), score.and(view_count.opt()).and(accepted_answer_id.opt()).and(last_activity_date).and(creation_date), ([0i64; 5], 0i128), |(a, d), ((((s, w), ac), la), c)| {
        ([a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + ac.is_some() as i64], d + (la - c) as i128)
    });
    let u = distinct_by(db.post.iq(), name(db), owner_user_id);
    let mut v = Vec::new();
    f.and((&u).opt()).drive(|k, ((a, d), u)| v.push((k, a, d, u.unwrap_or(0))));
    v.sort_by_key(|x| Reverse(x.1[0]));
    v
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AverageScore,
// SUM(p.ViewCount) AS TotalViews,
// AVG(EXTRACT(EPOCH FROM (p.LastActivityDate - p.CreationDate))) AS AveragePostAgeInSeconds,
// COUNT(DISTINCT p.OwnerUserId) AS UniqueUsers,
// SUM(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q11266(db: &'static So) -> String {
    rows(types_activity(db).iter().map(|&(k, a, d, u)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), nullable(a[3], a[2]), V::F(d as f64 / a[0] as f64 / 1e6), V::I(u), V::I(a[4])])))
}

// WITH PostStats AS (
// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AvgScore,
// SUM(p.ViewCount) AS TotalViews,
// AVG(EXTRACT(EPOCH FROM (p.LastActivityDate - p.CreationDate))) AS AvgTimeToActivity,
// COUNT(DISTINCT p.OwnerUserId) AS UniqueAuthors
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// GROUP BY
// pt.Name
// )
// SELECT
// PostType,
// TotalPosts,
// AvgScore,
// TotalViews,
// AvgTimeToActivity,
// UniqueAuthors
// FROM
// PostStats
// ORDER BY
// TotalPosts DESC;
fn q10130(db: &'static So) -> String {
    rows(types_activity(db).iter().map(|&(k, a, d, u)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), nullable(a[3], a[2]), V::F(d as f64 / a[0] as f64 / 1e6), V::I(u)])))
}

// WITH PostMetrics AS (
// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AvgScore,
// SUM(p.ViewCount) AS TotalViews,
// SUM(p.CommentCount) AS TotalComments,
// AVG(u.Reputation) AS AvgUserReputation
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// GROUP BY
// pt.Name
// )
// SELECT
// *
// FROM
// PostMetrics
// ORDER BY
// TotalPosts DESC;
fn q11479(db: &'static So) -> String {
    let Post { score, view_count, comment_count, owner_user, .. } = &db.post;
    let f = fold_by(db.post.iq(), name(db), score.and(view_count.opt()).and(comment_count).and(owner_user.select(&db.user.reputation).opt()), [0i64; 7], |a, (((s, w), c), r)| {
        [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + c, a[5] + r.is_some() as i64, a[6] + r.unwrap_or(0)]
    });
    rows(by_first(listed(&f)).iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), nullable(a[3], a[2]), V::I(a[4]), avg(a[6], a[5])])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS PostCount,
// AVG(CASE WHEN p.PostTypeId = 1 THEN p.Score END) AS AvgQuestionScore,
// AVG(p.ViewCount) AS AvgPostViewCount,
// SUM(CASE WHEN p.ClosedDate IS NOT NULL THEN 1 ELSE 0 END) AS ClosedPostCount,
// SUM(CASE WHEN c.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalComments
// FROM
// Posts p
// LEFT JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// GROUP BY
// pt.Name
// ORDER BY
// PostCount DESC;
fn q13826(db: &'static So) -> String {
    let Post { score, view_count, post_type_id, closed_date, .. } = &db.post;
    let f = fold_by(db.post.iq(), name(db).opt(), post_type_id.and(score).and(view_count.opt()).and(closed_date.opt()).and(comments_of(db).opt()), [0i64; 8], |a, ((((t, s), w), cl), c)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + if t == 1 { s } else { 0 }, a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0), a[5] + cl.is_some() as i64, a[6] + c.is_some() as i64, 0]
    });
    rows(by_first(listed(&f)).iter().map(|&(k, a)| row(vec![ostr(k), V::I(a[0]), avg(a[2], a[1]), avg(a[4], a[3]), V::I(a[5]), V::I(a[6])])))
}

// WITH PostMetrics AS (
// SELECT
// pt.Name AS PostTypeName,
// COUNT(p.Id) AS PostCount,
// AVG(p.ViewCount) AS AverageViewCount,
// SUM(u.Reputation) AS TotalUserReputation
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// GROUP BY
// pt.Name
// )
// SELECT
// PostTypeName,
// PostCount,
// AverageViewCount,
// TotalUserReputation
// FROM
// PostMetrics
// ORDER BY
// PostCount DESC;
fn q14999(db: &'static So) -> String {
    let Post { view_count, owner_user, .. } = &db.post;
    let f = fold_by(db.post.iq(), name(db), view_count.opt().and(owner_user.select(&db.user.reputation).opt()), [0i64; 5], |a, (w, r)| {
        [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + r.is_some() as i64, a[4] + r.unwrap_or(0)]
    });
    rows(by_first(listed(&f)).iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), avg(a[2], a[1]), nullable(a[4], a[3])])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN v.VoteTypeId IN (2, 3) THEN 1 ELSE 0 END) AS TotalVotes,
// SUM(CASE WHEN c.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalComments,
// AVG(u.Reputation) AS AverageUserReputation
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q10909(db: &'static So) -> String {
    let f = fold_by(
        db.post.iq(),
        name(db),
        votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()).and((&db.post.owner_user).select(&db.user.reputation).opt()),
        [0i64; 5],
        |a, ((x, c), r)| [a[0] + 1, a[1] + matches!(x, Some(2) | Some(3)) as i64, a[2] + c.is_some() as i64, a[3] + r.is_some() as i64, a[4] + r.unwrap_or(0)],
    );
    rows(by_first(listed(&f)).iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[4], a[3])])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes,
// AVG(u.Reputation) AS AvgUserReputation,
// COUNT(DISTINCT u.Id) AS TotalUsers
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.CreationDate >= '2022-01-01'
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q13114(db: &'static So) -> String {
    let base = db.post.with((&db.post.creation_date).ge(date(2022, 1, 1)));
    let f = fold_by(&base, name(db), votes_of(db).select(&db.vote.vote_type_id).opt().and((&db.post.owner_user).select(&db.user.reputation).opt()), [0i64; 5], |a, (x, r)| {
        [a[0] + 1, a[1] + (x == Some(2)) as i64, a[2] + (x == Some(3)) as i64, a[3] + r.is_some() as i64, a[4] + r.unwrap_or(0)]
    });
    let u = distinct_by(&base, name(db), &db.post.owner_user);
    rows(with1(f, u).iter().map(|&(k, a, u)| row(vec![V::S(k), V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[4], a[3]), V::I(u)])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AverageScore,
// AVG(p.ViewCount) AS AverageViews,
// SUM(CASE WHEN p.PostTypeId = 1 THEN p.AnswerCount ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN p.PostTypeId = 1 THEN CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END ELSE 0 END) AS AcceptedAnswers,
// COUNT(c.Id) AS TotalComments
// FROM
// Posts p
// LEFT JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q12194(db: &'static So) -> String {
    let Post { score, view_count, post_type_id, answer_count, accepted_answer_id, .. } = &db.post;
    let f = fold_by(
        db.post.iq(),
        name(db).opt(),
        score.and(view_count.opt()).and(post_type_id).and(answer_count.opt()).and(accepted_answer_id.opt()).and(comments_of(db).opt()),
        [0i64; 8],
        |a, (((((s, w), t), an), ac), c)| {
            let qa = if t == 1 { an } else { Some(0) };
            [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + qa.is_some() as i64, a[5] + qa.unwrap_or(0), a[6] + (t == 1 && ac.is_some()) as i64, a[7] + c.is_some() as i64]
        },
    );
    rows(by_first(listed(&f)).iter().map(|&(k, a)| row(vec![ostr(k), V::I(a[0]), avg(a[1], a[0]), avg(a[3], a[2]), nullable(a[5], a[4]), V::I(a[6]), V::I(a[7])])))
}

// WITH PostStats AS (
// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS PostCount,
// COUNT(c.Id) AS CommentCount,
// AVG(u.Reputation) AS AvgUserReputation
// FROM
// Posts p
// LEFT JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// GROUP BY
// pt.Id, pt.Name
// )
// SELECT
// ps.PostType,
// ps.PostCount,
// ps.CommentCount,
// ps.AvgUserReputation
// FROM
// PostStats ps
// ORDER BY
// ps.PostCount DESC;
fn q11702(db: &'static So) -> String {
    let f = fold_by(db.post.iq(), (&db.post.post_type).opt(), comments_of(db).opt().and((&db.post.owner_user).select(&db.user.reputation).opt()), [0i64; 4], |a, (c, r)| {
        [a[0] + 1, a[1] + c.is_some() as i64, a[2] + r.is_some() as i64, a[3] + r.unwrap_or(0)]
    });
    rows(by_first(listed(&f)).iter().map(|&(t, a)| row(vec![ostr(t.map(|t| db.post_type.name.get(t).unwrap())), V::I(a[0]), V::I(a[1]), avg(a[3], a[2])])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AvgScore,
// AVG(p.ViewCount) AS AvgViewCount,
// SUM(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS TotalAcceptedAnswers,
// SUM(CASE WHEN c.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalComments,
// SUM(CASE WHEN v.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalVotes
// FROM
// Posts p
// LEFT JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q11792(db: &'static So) -> String {
    let Post { score, view_count, accepted_answer_id, .. } = &db.post;
    let f = fold_by(db.post.iq(), name(db).opt(), score.and(view_count.opt()).and(accepted_answer_id.opt()).and(comments_of(db).opt()).and(votes_of(db).opt()), [0i64; 7], |a, ((((s, w), ac), c), x)| {
        [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + ac.is_some() as i64, a[5] + c.is_some() as i64, a[6] + x.is_some() as i64]
    });
    rows(by_first(listed(&f)).iter().map(|&(k, a)| row(vec![ostr(k), V::I(a[0]), avg(a[1], a[0]), avg(a[3], a[2]), V::I(a[4]), V::I(a[5]), V::I(a[6])])))
}

// WITH PostStats AS (
// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// COUNT(c.Id) AS TotalComments,
// COUNT(v.Id) AS TotalVotes,
// MAX(p.LastActivityDate) AS LastActivityDate
// FROM
// Posts p
// LEFT JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// pt.Name
// )
// SELECT
// PostType,
// TotalPosts,
// TotalComments,
// TotalVotes,
// LastActivityDate
// FROM
// PostStats
// ORDER BY
// TotalPosts DESC;
fn q13992(db: &'static So) -> String {
    let f = fold_by(db.post.iq(), name(db).opt(), (&db.post.last_activity_date).and(comments_of(db).opt()).and(votes_of(db).opt()), [0, 0, 0, i64::MIN], |a: [i64; 4], ((la, c), x)| {
        [a[0] + 1, a[1] + c.is_some() as i64, a[2] + x.is_some() as i64, a[3].max(la)]
    });
    rows(by_first(listed(&f)).iter().map(|&(k, a)| row(vec![ostr(k), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::T(a[3])])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// SUM(p.ViewCount) AS TotalViews,
// SUM(p.Score) AS TotalScore,
// AVG(p.AnswerCount) AS AvgAnswers,
// AVG(CASE WHEN c.Id IS NOT NULL THEN 1 ELSE 0 END) AS AvgCommentsPerPost,
// SUM(CASE WHEN b.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalBadgesEarned
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Badges b ON p.OwnerUserId = b.UserId
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q11778(db: &'static So) -> String {
    let Post { score, view_count, answer_count, owner_user_id, .. } = &db.post;
    let bidx = badges_by_uid(db);
    let f = fold_by(
        db.post.iq(),
        name(db),
        view_count.opt().and(score).and(answer_count.opt()).and(comments_of(db).opt()).and(owner_user_id.select(&bidx).opt()),
        [0i64; 8],
        |a, ((((w, s), an), c), b)| {
            [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s, a[4] + an.is_some() as i64, a[5] + an.unwrap_or(0), a[6] + c.is_some() as i64, a[7] + b.is_some() as i64]
        },
    );
    rows(by_first(listed(&f)).iter().map(|&(k, a)| {
        row(vec![V::S(k), V::I(a[0]), nullable(a[2], a[1]), V::I(a[3]), avg(a[5], a[4]), V::F(a[6] as f64 / a[0] as f64), V::I(a[7])])
    }))
}

// FROM PostTypes pt LEFT JOIN Posts p LEFT JOIN Comments c LEFT JOIN Votes v.
// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// COALESCE(AVG(p.Score), 0) AS AverageScore,
// COALESCE(SUM(c.Score), 0) AS TotalComments,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpVotes,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownVotes
// FROM
// PostTypes pt
// LEFT JOIN
// Posts p ON pt.Id = p.PostTypeId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q10356(db: &'static So) -> String {
    let of: HashIdx<Id<PostType>, Id<Post>> = (&db.post.post_type).inv().collect();
    let mut v = Vec::new();
    db.post_type
        .group_by(&db.post_type.name)
        .select((&of).select((&db.post.score).and(comments_of(db).select(&db.comment.score).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 5], |a, r| match r {
            Some(((s, c), x)) => [a[0] + 1, a[1] + s, a[2] + c.unwrap_or(0), a[3] + (x == Some(2)) as i64, a[4] + (x == Some(3)) as i64],
            None => a,
        })
        .drive(|k, a| v.push((k, a)));
    let or0 = |s: i64, n: i64| V::F(if n == 0 { 0.0 } else { s as f64 / n as f64 });
    rows(by_first(v).iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), or0(a[1], a[0]), V::I(a[2]), V::I(a[3]), V::I(a[4])])))
}

// SELECT
// pt.Name AS PostType,
// AVG(EXTRACT(EPOCH FROM (p.LastActivityDate - p.CreationDate)) / 60) AS AvgResponseTimeMin,
// AVG(u.Reputation) AS AvgUserReputation,
// COUNT(DISTINCT v.Id) AS TotalVotes,
// COUNT(DISTINCT c.Id) AS TotalComments
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// GROUP BY
// pt.Name
// ORDER BY
// AvgResponseTimeMin DESC;
fn q14942(db: &'static So) -> String {
    let Post { last_activity_date, creation_date, owner_user, .. } = &db.post;
    let f = fold_by(
        db.post.iq(),
        name(db),
        last_activity_date.and(creation_date).and(owner_user.select(&db.user.reputation).opt()).and(votes_of(db).opt()).and(comments_of(db).opt()),
        ([0i64; 3], 0i128),
        |(a, d), ((((la, c), r), _), _)| ([a[0] + 1, a[1] + r.is_some() as i64, a[2] + r.unwrap_or(0)], d + (la - c) as i128),
    );
    let x = distinct_by(db.post.iq(), name(db), votes_of(db));
    let c = distinct_by(db.post.iq(), name(db), comments_of(db));
    let mut v = Vec::new();
    f.and((&x).opt()).and((&c).opt()).drive(|k, (((a, d), x), c)| v.push((k, a, d as f64 / a[0] as f64 / 1e6 / 60.0, x.unwrap_or(0), c.unwrap_or(0))));
    v.sort_by(|a, b| b.2.total_cmp(&a.2));
    rows(v.iter().map(|&(k, a, t, x, c)| row(vec![V::S(k), V::F(t), avg(a[2], a[1]), V::I(x), V::I(c)])))
}

// A float AVG over the joined rows whose printed digits move with DuckDB's
// SET threads, so rewrites/13410.sql takes the exact-integer mean, as 5603's does.
//
// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AverageScore,
// SUM(p.ViewCount) AS TotalViews,
// AVG(EXTRACT(EPOCH FROM (p.LastActivityDate - p.CreationDate))) AS AverageDurationInSeconds,
// COUNT(DISTINCT c.Id) AS TotalComments,
// COUNT(DISTINCT v.Id) AS TotalVotes
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q13410(db: &'static So) -> String {
    let Post { last_activity_date, creation_date, score, view_count, .. } = &db.post;
    let f = fold_by(
        db.post.iq(),
        name(db),
        score.and(view_count.opt()).and(last_activity_date).and(creation_date).and(comments_of(db).opt()).and(votes_of(db).opt()),
        ([0i64; 4], 0i128),
        |(a, d), (((((s, w), la), c), _), _)| ([a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)], d + (la - c) as i128),
    );
    let c = distinct_by(db.post.iq(), name(db), comments_of(db));
    let x = distinct_by(db.post.iq(), name(db), votes_of(db));
    let mut v = Vec::new();
    f.and((&c).opt()).and((&x).opt()).drive(|k, (((a, d), c), x)| v.push((k, a, d, c.unwrap_or(0), x.unwrap_or(0))));
    v.sort_by_key(|x| Reverse(x.1[0]));
    rows(v.iter().map(|&(k, a, d, c, x)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), nullable(a[3], a[2]), V::F(d as f64 / a[0] as f64 / 1e6), V::I(c), V::I(x)])))
}

// SELECT
// COUNT(DISTINCT p.Id) AS TotalPosts,
// AVG(CASE WHEN p.PostTypeId = 1 THEN p.Score END) AS AvgQuestionScore,
// COUNT(DISTINCT u.Id) AS TotalUsers,
// COUNT(CASE WHEN p.ClosedDate IS NOT NULL THEN 1 END) AS ClosedPostsCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY
// DATE_TRUNC('month', p.CreationDate)
// ORDER BY
// DATE_TRUNC('month', p.CreationDate);
fn q13143(db: &'static So) -> String {
    let Post { post_type_id, score, closed_date, creation_date, owner_user, .. } = &db.post;
    let base = owned(db).with(creation_date.ge(ts(2023, 10, 1, 12, 34, 56)));
    let month = creation_date.map(trunc_month);
    let f = fold_by(&base, &month, post_type_id.and(score).and(closed_date.opt()), [0i64; 3], |a, ((t, s), cl)| {
        [a[0] + (t == 1) as i64, a[1] + if t == 1 { s } else { 0 }, a[2] + cl.is_some() as i64]
    });
    let p = distinct_by(&base, &month, Ident::<Post>::new());
    let u = distinct_by(&base, &month, owner_user);
    let mut v = Vec::new();
    f.and(&p).and(&u).drive(|_, ((a, p), u)| v.push((a, p, u)));
    rows(v.iter().map(|&(a, p, u)| row(vec![V::I(p), avg(a[1], a[0]), V::I(u), V::I(a[2])])))
}

// SELECT p.Title, p.CreationDate, COUNT(c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
// COUNT(DISTINCT b.Id) AS BadgeCount,
// COUNT(DISTINCT ph.Id) AS PostHistoryCount
// FROM Posts p
// LEFT JOIN Comments c ON p.Id = c.PostId
// LEFT JOIN Votes v ON p.Id = v.PostId
// LEFT JOIN Badges b ON p.OwnerUserId = b.UserId
// LEFT JOIN PostHistory ph ON p.Id = ph.PostId
// WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY p.Title, p.CreationDate
// ORDER BY p.CreationDate DESC;
fn q14567(db: &'static So) -> String {
    let Post { title, creation_date, owner_user_id, .. } = &db.post;
    let bidx = badges_by_uid(db);
    let base = db.post.with(creation_date.ge(ts(2023, 10, 1, 12, 34, 56)));
    let key = title.opt().and(creation_date);
    let f = fold_by(&base, &key, comments_of(db).opt().and(votes_of(db).opt()).and(owner_user_id.select(&bidx).opt()).and(history_of(db).opt()), [0i64; 1], |a, (((c, _), _), _)| [a[0] + c.is_some() as i64]);
    let x = distinct_by(&base, &key, votes_of(db));
    let b = distinct_by(&base, &key, owner_user_id.select(&bidx));
    let h = distinct_by(&base, &key, history_of(db));
    let mut v = Vec::new();
    f.and((&x).opt()).and((&b).opt()).and((&h).opt()).drive(|(t, c), (((a, x), b), h)| v.push((t, c, a[0], [x, b, h].map(|n| n.unwrap_or(0)))));
    rows(v.iter().map(|&(t, c, n, d)| row(vec![ostr(t), V::T(c), V::I(n), V::I(d[0]), V::I(d[1]), V::I(d[2])])))
}

// SELECT
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.AnswerCount,
// p.CommentCount,
// p.Score,
// COALESCE(u.Reputation, 0) AS OwnerReputation,
// COUNT(DISTINCT c.Id) AS TotalComments,
// COUNT(DISTINCT v.Id) AS TotalVotes
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Title, p.CreationDate, p.ViewCount, p.AnswerCount, p.CommentCount, p.Score, u.Reputation
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q13314(db: &'static So) -> String {
    let Post { title, creation_date, view_count, answer_count, comment_count, score, owner_user, .. } = &db.post;
    let key = title.opt().and(creation_date).and(view_count.opt()).and(answer_count.opt()).and(comment_count).and(score).and(owner_user.select(&db.user.reputation).opt());
    let c = distinct_by(db.post.iq(), &key, comments_of(db));
    let x = distinct_by(db.post.iq(), &key, votes_of(db));
    let groups = db.post.group_by(&key).select(Ident::<Post>::new()).fold(0i64, |a, _| a + 1);
    let mut v = Vec::new();
    groups.and((&c).opt()).and((&x).opt()).drive(|k, ((_, c), x)| v.push((k, c.unwrap_or(0), x.unwrap_or(0))));
    v.sort_by_key(|&(((((((_, cd), _), _), _), _), _), _, _)| Reverse(cd));
    rows(v.iter().take(100).map(|&(((((((t, cd), w), an), cc), s), r), c, x)| {
        row(vec![ostr(t), V::T(cd), oint(w), oint(an), V::I(cc), V::I(s), V::I(r.unwrap_or(0)), V::I(c), V::I(x)])
    }))
}

// SELECT p.Title, p.CreationDate, p.Score, u.DisplayName AS OwnerDisplayName,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM Posts p
// JOIN Users u ON p.OwnerUserId = u.Id
// LEFT JOIN Comments c ON p.Id = c.PostId
// LEFT JOIN Votes v ON p.Id = v.PostId
// WHERE p.CreationDate >= '2024-10-01 12:34:56'::timestamp - INTERVAL '1 year'
// GROUP BY p.Title, p.CreationDate, p.Score, u.DisplayName
// ORDER BY p.CreationDate DESC
// LIMIT 100;
fn q11600(db: &'static So) -> String {
    let Post { title, creation_date, score, owner_user, .. } = &db.post;
    let base = owned(db).with(creation_date.ge(ts(2023, 10, 1, 12, 34, 56)));
    let key = title.opt().and(creation_date).and(score).and(owner_user.select(&db.user.display_name));
    let mut v = group_stats(db, base, key, "cv", &[]);
    v.sort_by_key(|&((((_, cd), _), _), _)| Reverse(cd));
    rows(v.iter().take(100).map(|&((((t, cd), s), dn), ref st)| row(vec![ostr(t), V::T(cd), V::I(s), V::S(dn), V::I(st.cx), V::I(st.up), V::I(st.down)])))
}

// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// U.DisplayName AS OwnerDisplayName,
// COUNT(C.Id) AS CommentCount,
// SUM(CASE WHEN V.CreationDate IS NOT NULL THEN 1 ELSE 0 END) AS VoteCount
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// P.Id, P.Title, P.CreationDate, U.DisplayName
// ORDER BY
// P.CreationDate DESC
// LIMIT 10;
fn q16076(db: &'static So) -> String {
    stats_rows(db, stats_with(db, owned(db), "cv", &[], &[]), |p, _| newest(db, p), 10, &["id", "title", "created", "owner", "#cx", "#vx"])
}

// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// U.DisplayName AS OwnerDisplayName,
// COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
// COUNT(CASE WHEN V.Id IS NOT NULL THEN 1 END) AS VoteCount
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// WHERE
// P.PostTypeId = 1
// GROUP BY
// P.Id, P.Title, P.CreationDate, U.DisplayName
// ORDER BY
// P.CreationDate DESC;
fn q15390(db: &'static So) -> String {
    stats_rows(db, stats_with(db, questions(db), "cv", &[], &[]), |_, _| 0, 0, &["id", "title", "created", "owner", "#cx", "#vx"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// COUNT(DISTINCT c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q11235(db: &'static So) -> String {
    let c = per_post_distinct(db, comments_of(db));
    let x = per_post_distinct(db, votes_of(db));
    stats_rows(db, stats_with(db, db.post.iq(), "cv", &[], &[&c, &x]), |p, _| newest(db, p), 100, &["id", "title", "created", "views", "score", "#d0", "#d1", "#up", "#down"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 1 THEN 1 ELSE 0 END), 0) AS AcceptedAnswers
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate
// ORDER BY
// p.CreationDate DESC;
fn q11944(db: &'static So) -> String {
    stats_rows(db, stats_with(db, since(db, date(2023, 1, 1)), "cv", &[], &[]), |_, _| 0, 0, &["id", "title", "created", "#cx", "#up", "#down", "#v1"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// COUNT(DISTINCT b.Id) AS BadgeCount,
// MAX(p.CreationDate) AS LastActivityDate
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Badges b ON p.OwnerUserId = b.UserId
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title
// ORDER BY
// LastActivityDate DESC;
fn q14853(db: &'static So) -> String {
    cvb(db, db.post.with((&db.post.post_type_id).eq(1)), &["id", "title", "#cx", "#up", "#down", "#d0", "created"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
// COUNT(DISTINCT b.Id) AS BadgeCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Badges b ON p.OwnerUserId = b.UserId
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate
// ORDER BY
// p.CreationDate DESC;
fn q14673(db: &'static So) -> String {
    cvb(db, since(db, date(2023, 1, 1)), &["id", "title", "created", "#cx", "#up", "#down", "#d0"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// p.AnswerCount,
// COUNT(DISTINCT b.Id) AS BadgeCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Badges b ON p.OwnerUserId = b.UserId
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.AnswerCount
// ORDER BY
// p.ViewCount DESC;
fn q11493(db: &'static So) -> String {
    cvb(db, db.post.with((&db.post.post_type_id).eq(1)), &["id", "title", "created", "views", "#cx", "#up", "#down", "answers", "#d0"])
}

// SELECT DISTINCT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// u.DisplayName AS OwnerDisplayName,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, p.CreationDate, u.DisplayName
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q19011(db: &'static So) -> String {
    stats_rows(db, stats_with(db, questions(db), "cv", &[], &[]), |p, _| newest(db, p), 10, &["id", "title", "created", "owner", "#cx", "#up", "#down"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 6 THEN 1 ELSE 0 END), 0) AS CloseVotes,
// COUNT(c.Id) AS CommentCount
// FROM
// Posts p
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount
// ORDER BY
// p.ViewCount DESC
// LIMIT 100;
fn q14752(db: &'static So) -> String {
    let base = db.post.with((&db.post.post_type_id).eq(1));
    stats_rows(db, stats_with(db, base, "vc", &[], &[]), |p, _| views_desc(db, p), 100, &["id", "title", "created", "views", "#up", "#down", "#v6", "#cx"])
}

// WITH RankedPosts AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.AnswerCount,
// p.CreationDate,
// u.DisplayName AS OwnerDisplayName,
// COUNT(v.Id) AS VoteCount
// FROM Posts p
// JOIN Users u ON p.OwnerUserId = u.Id
// LEFT JOIN Votes v ON p.Id = v.PostId
// WHERE p.AnswerCount > 10
// GROUP BY p.Id, p.Title, p.AnswerCount, p.CreationDate, u.DisplayName
// )
// SELECT
// rp.PostId,
// rp.Title,
// rp.AnswerCount,
// rp.CreationDate,
// rp.OwnerDisplayName,
// rp.VoteCount
// FROM RankedPosts rp
// ORDER BY rp.VoteCount DESC, rp.AnswerCount DESC;
fn q14794(db: &'static So) -> String {
    let base = owned(db).with((&db.post.answer_count).gt(10));
    stats_rows(db, stats_with(db, base, "v", &[], &[]), |_, _| 0, 0, &["id", "title", "answers", "created", "owner", "#vx"])
}

// SELECT
// P.Id AS PostId,
// P.Title,
// U.DisplayName AS OwnerDisplayName,
// P.CreationDate,
// P.ViewCount,
// P.Score,
// COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
// COUNT(CASE WHEN A.Id IS NOT NULL THEN 1 END) AS AnswerCount
// FROM
// Posts P
// LEFT JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Posts A ON P.Id = A.ParentId AND A.PostTypeId = 2
// WHERE
// P.PostTypeId = 1
// GROUP BY
// P.Id, P.Title, U.DisplayName, P.CreationDate, P.ViewCount, P.Score
// ORDER BY
// P.CreationDate DESC
// LIMIT 10;
fn q19911(db: &'static So) -> String {
    let base = db.post.with((&db.post.post_type_id).eq(1));
    stats_rows(db, stats_with(db, base, "cA", &[], &[]), |p, _| newest(db, p), 10, &["id", "title", "owner", "created", "views", "score", "#cx", "#ax"])
}

// SELECT
// PH.PostHistoryTypeId,
// P.Title,
// P.CreationDate,
// P.ViewCount,
// COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVotes,
// COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVotes,
// COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount
// FROM
// Posts P
// JOIN
// PostHistory PH ON P.Id = PH.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// WHERE
// PH.CreationDate BETWEEN '2022-01-01' AND '2023-01-01'
// GROUP BY
// PH.PostHistoryTypeId, P.Title, P.CreationDate, P.ViewCount
// ORDER BY
// P.CreationDate;
fn q13020(db: &'static So) -> String {
    let PostHistory { creation_date, post_history_type_id, .. } = &db.post_history;
    let Post { title, creation_date: pcd, view_count, .. } = &db.post;
    let j: MatSet<(Id<Post>, Id<PostHistory>)> =
        db.post.select(Ident::<Post>::new().and(history_of(db).select(Ident::<PostHistory>::new().with(creation_date.between(date(2022, 1, 1), date(2023, 1, 1)))))).collect();
    let post_of = (&j).map(|(p, _)| p);
    let key = (&j).map(|(_, h)| h).select(post_history_type_id).and((&post_of).select(title.opt().and(pcd).and(view_count.opt())));
    let mut v = Vec::new();
    (&j).group_by(key)
        .select((&post_of).select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt())))
        .fold([0i64; 3], |a, (x, c)| [a[0] + (x == Some(2)) as i64, a[1] + (x == Some(3)) as i64, a[2] + c.is_some() as i64])
        .drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&((t, ((ti, c), w)), a)| row(vec![V::I(t), ostr(ti), V::T(c), oint(w), V::I(a[0]), V::I(a[1]), V::I(a[2])])))
}

// Users LEFT JOIN Posts LEFT JOIN Votes, per user: COUNT(p.Id), SUM(p.Score),
// up and down votes, over the joined rows.
fn user_post_votes(db: &'static So) -> Fold<Id<User>, [i64; 4]> {
    db.user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.score).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some((s, x)) => [a[0] + 1, a[1] + s, a[2] + (x == Some(2)) as i64, a[3] + (x == Some(3)) as i64],
            None => a,
        })
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// AVG(CASE WHEN p.PostTypeId = 1 THEN p.Score END) AS AvgQuestionScore,
// AVG(p.ViewCount) AS AvgViewCount,
// MAX(p.Score) AS MaxPostScore,
// SUM(CASE WHEN v.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalVotesReceived
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// u.Id, u.DisplayName
// ORDER BY
// TotalPosts DESC;
fn q14271(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let mut v = Vec::new();
    db.user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(view_count.opt()).and(votes_of(db).opt())).opt())
        .fold([0, 0, 0, 0, 0, i64::MIN, 0], |a: [i64; 7], p| match p {
            Some((((t, s), w), x)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + if t == 1 { s } else { 0 }, a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0), a[5].max(s), a[6] + x.is_some() as i64],
            None => a,
        })
        .drive(|u, a| v.push((u, a)));
    rows(v.iter().map(|&(u, a)| {
        row(vec![V::I(db.user.origid.get(u).unwrap()), V::S(db.user.display_name.get(u).unwrap()), V::I(a[0]), avg(a[2], a[1]), avg(a[4], a[3]), omax(a[5], a[0]), V::I(a[6])])
    }))
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpVotes,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownVotes,
// COALESCE(SUM(p.Score), 0) AS TotalScore,
// COALESCE(AVG(p.Score), 0) AS AverageScore
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// u.Id, u.DisplayName
// ORDER BY
// TotalPosts DESC, AverageScore DESC;
fn q12831(db: &'static So) -> String {
    let f = user_post_votes(db);
    let mean = |a: &[i64; 4]| if a[0] == 0 { 0.0 } else { a[1] as f64 / a[0] as f64 };
    let mut v = listed(&f);
    v.sort_by(|x, y| y.1[0].cmp(&x.1[0]).then(mean(&y.1).total_cmp(&mean(&x.1))));
    rows(v.iter().map(|(u, a)| {
        row(vec![V::I(db.user.origid.get(*u).unwrap()), V::S(db.user.display_name.get(*u).unwrap()), V::I(a[0]), V::I(a[2]), V::I(a[3]), V::I(a[1]), V::F(mean(a))])
    }))
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(p.Id) AS PostCount,
// COALESCE(SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS AnswerCount,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount,
// MAX(p.CreationDate) AS LastPostDate
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
// ORDER BY
// PostCount DESC
// LIMIT 100;
fn q13151(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let mut v = Vec::new();
    db.user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(creation_date).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0, 0, 0, 0, i64::MIN], |a: [i64; 5], p| match p {
            Some(((t, c), x)) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (x == Some(2)) as i64, a[3] + (x == Some(3)) as i64, a[4].max(c)],
            None => a,
        })
        .drive(|u, a| v.push((u, a)));
    let mut v = by_first(v);
    v.truncate(100);
    rows(v.iter().map(|&(u, a)| {
        row(vec![
            V::I(db.user.origid.get(u).unwrap()),
            V::S(db.user.display_name.get(u).unwrap()),
            V::I(db.user.reputation.get(u).unwrap()),
            V::I(a[0]),
            V::I(a[1]),
            V::I(a[2]),
            V::I(a[3]),
            if a[0] == 0 { V::Null } else { V::T(a[4]) },
        ])
    }))
}

// SELECT
// u.Id AS UserId,
// u.DisplayName AS UserName,
// u.Reputation,
// COUNT(p.Id) AS TotalPosts,
// COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS TotalQuestions,
// COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS TotalAnswers,
// SUM(COALESCE(p.Score, 0)) AS TotalScore,
// SUM(COALESCE(p.ViewCount, 0)) AS TotalViews,
// AVG(COALESCE(p.Score, 0)) AS AverageScore,
// AVG(COALESCE(p.ViewCount, 0)) AS AverageViews
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
// ORDER BY
// TotalPosts DESC
// LIMIT 100;
fn q10371(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let mut v = Vec::new();
    db.user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(view_count.opt())).opt())
        .fold([0i64; 6], |a, p| match p {
            Some(((t, s), w)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + w.unwrap_or(0), a[5] + 1],
            None => [a[0], a[1], a[2], a[3], a[4], a[5] + 1],
        })
        .drive(|u, a| v.push((u, a)));
    let mut v = by_first(v);
    v.truncate(100);
    rows(v.iter().map(|&(u, a)| {
        row(vec![
            V::I(db.user.origid.get(u).unwrap()),
            V::S(db.user.display_name.get(u).unwrap()),
            V::I(db.user.reputation.get(u).unwrap()),
            V::I(a[0]),
            V::I(a[1]),
            V::I(a[2]),
            V::I(a[3]),
            V::I(a[4]),
            V::F(a[3] as f64 / a[5] as f64),
            V::F(a[4] as f64 / a[5] as f64),
        ])
    }))
}

// SELECT
// u.DisplayName AS UserDisplayName,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN p.PostTypeId IN (4, 5) THEN 1 ELSE 0 END) AS TotalTagWikis,
// AVG(p.Score) AS AverageScore,
// SUM(CASE WHEN c.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalComments
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// GROUP BY
// u.Id, u.DisplayName
// ORDER BY
// TotalPosts DESC
// LIMIT 10;
fn q10907(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let mut v = Vec::new();
    db.user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(comments_of(db).opt())).opt())
        .fold([0i64; 6], |a, p| match p {
            Some(((t, s), c)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (t == 4 || t == 5) as i64, a[4] + s, a[5] + c.is_some() as i64],
            None => a,
        })
        .drive(|u, a| v.push((u, a)));
    let mut v = by_first(v);
    v.truncate(10);
    rows(v.iter().map(|&(u, a)| row(vec![V::S(db.user.display_name.get(u).unwrap()), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), avg(a[4], a[0]), V::I(a[5])])))
}

// SELECT
// U.DisplayName,
// COUNT(DISTINCT P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// U.DisplayName
// ORDER BY
// TotalPosts DESC
// LIMIT 10;
fn q18229(db: &'static So) -> String {
    let name = &db.user.display_name;
    let f = db.user.group_by(name).select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt()).fold([0i64; 4], |a, p| match p {
        Some((t, x)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (x == Some(2)) as i64, a[3] + (x == Some(3)) as i64],
        None => a,
    });
    let posts = db.user.group_by(name).select(posts_of(db)).count_distinct();
    let mut v = Vec::new();
    f.and((&posts).opt()).drive(|k, (a, n)| v.push((k, n.unwrap_or(0), a)));
    v.sort_by_key(|x| Reverse(x.1));
    rows(v.iter().take(10).map(|&(k, n, a)| row(vec![V::S(k), V::I(n), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3])])))
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// u.CreationDate AS UserCreationDate,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// COUNT(DISTINCT b.Id) AS BadgeCount,
// MAX(b.Date) AS LastBadgeDate
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// u.Id, u.DisplayName, u.Reputation, u.CreationDate
// ORDER BY
// u.Reputation DESC, PostCount DESC
// LIMIT 100;
fn q14008(db: &'static So) -> String {
    let g = || db.user.group_by(Ident::<User>::new());
    let f = g().select(posts_of(db).select(&db.post.post_type_id).opt().and(badges_of(db).select(&db.badge.date).opt())).fold([0, 0, 0, i64::MIN], |a: [i64; 4], (t, b)| {
        [a[0] + (t == Some(1)) as i64, a[1] + (t == Some(2)) as i64, a[2] + b.is_some() as i64, b.map_or(a[3], |d| a[3].max(d))]
    });
    let p = g().select(posts_of(db)).count_distinct();
    let b = g().select(badges_of(db)).count_distinct();
    let mut v = Vec::new();
    f.and((&p).opt()).and((&b).opt()).drive(|u, ((a, p), b)| v.push((u, a, p.unwrap_or(0), b.unwrap_or(0))));
    v.sort_by_key(|&(u, _, p, _)| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(p)));
    rows(v.iter().take(100).map(|&(u, a, p, b)| {
        let User { origid, display_name, reputation, creation_date, .. } = &db.user;
        row(vec![
            V::I(origid.get(u).unwrap()),
            V::S(display_name.get(u).unwrap()),
            V::I(reputation.get(u).unwrap()),
            V::T(creation_date.get(u).unwrap()),
            V::I(p),
            V::I(a[0]),
            V::I(a[1]),
            V::I(b),
            if a[2] == 0 { V::Null } else { V::T(a[3]) },
        ])
    }))
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// COUNT(DISTINCT c.Id) AS TotalComments,
// COUNT(DISTINCT v.Id) AS TotalVotes,
// DATE_TRUNC('month', u.CreationDate) AS UserCreationMonth,
// EXTRACT(YEAR FROM u.CreationDate) AS CreationYear
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// u.Id, u.DisplayName, DATE_TRUNC('month', u.CreationDate), EXTRACT(YEAR FROM u.CreationDate)
// ORDER BY
// UserCreationMonth, u.DisplayName;
fn q14160(db: &'static So) -> String {
    let User { creation_date, .. } = &db.user;
    let key = Ident::<User>::new().and(creation_date.map(trunc_month)).and(creation_date.map(year));
    let g = || db.user.group_by(&key);
    let p = g().select(posts_of(db)).count_distinct();
    let c = g().select(posts_of(db).select(comments_of(db))).count_distinct();
    let x = g().select(posts_of(db).select(votes_of(db))).count_distinct();
    let mut v = Vec::new();
    db.user.select(&key).select((&p).opt().and((&c).opt()).and((&x).opt()).map(|((p, c), x)| [p, c, x].map(|n| n.unwrap_or(0)))).drive(|u, n| v.push((u, n)));
    rows(v.iter().map(|&(u, n)| {
        let c = creation_date.get(u).unwrap();
        row(vec![V::I(db.user.origid.get(u).unwrap()), V::S(db.user.display_name.get(u).unwrap()), V::I(n[0]), V::I(n[1]), V::I(n[2]), V::T(trunc_month(c)), V::I(year(c))])
    }))
}

// SELECT
// u.Reputation AS UserReputation,
// COUNT(DISTINCT p.Id) AS QuestionCount,
// AVG(p.Score) AS AverageQuestionScore,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes,
// COUNT(a.Id) AS AnswerCount
// FROM
// Users u
// JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Posts a ON a.ParentId = p.Id
// LEFT JOIN
// Votes v ON v.PostId = p.Id
// WHERE
// p.PostTypeId = 1
// GROUP BY
// u.Reputation
// ORDER BY
// u.Reputation DESC;
fn q11006(db: &'static So) -> String {
    let Post { score, owner_user, .. } = &db.post;
    let base = owned(db).with((&db.post.post_type_id).eq(1));
    let key = owner_user.select(&db.user.reputation);
    let f = fold_by(&base, &key, score.and(children_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()), [0i64; 5], |a, ((s, an), x)| {
        [a[0] + 1, a[1] + s, a[2] + (x == Some(2)) as i64, a[3] + (x == Some(3)) as i64, a[4] + an.is_some() as i64]
    });
    let p = distinct_by(&base, &key, Ident::<Post>::new());
    let mut v = Vec::new();
    f.and(&p).drive(|r, (a, p)| v.push((r, a, p)));
    rows(v.iter().map(|&(r, a, p)| row(vec![V::I(r), V::I(p), avg(a[1], a[0]), V::I(a[2]), V::I(a[3]), V::I(a[4])])))
}

// WITH UserActivity AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(P.Id) AS PostsCreated,
// COUNT(PH.Id) AS EditsMade
// FROM Users U
// LEFT JOIN Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN PostHistory PH ON U.Id = PH.UserId
// GROUP BY U.Id, U.DisplayName
// )
// SELECT
// UserId,
// DisplayName,
// PostsCreated,
// EditsMade,
// (PostsCreated + EditsMade) AS TotalActivity
// FROM UserActivity
// ORDER BY TotalActivity DESC
// LIMIT 10;
fn q10898(db: &'static So) -> String {
    let edits: HashIdx<Id<User>, Id<PostHistory>> = (&db.post_history.user).inv().collect();
    let mut v = Vec::new();
    db.user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).opt().and((&edits).opt()))
        .fold((0i64, 0i64), |(p, e), (pi, ei)| (p + pi.is_some() as i64, e + ei.is_some() as i64))
        .drive(|u, a| v.push((u, a)));
    v.sort_by_key(|&(_, (p, e))| Reverse(p + e));
    rows(v.iter().take(10).map(|&(u, (p, e))| row(vec![V::I(db.user.origid.get(u).unwrap()), V::S(db.user.display_name.get(u).unwrap()), V::I(p), V::I(e), V::I(p + e)])))
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1
// WHEN v.VoteTypeId = 3 THEN -1
// ELSE 0 END) AS TotalVotes,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// COUNT(DISTINCT b.Id) AS TotalBadges
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// WHERE
// u.Reputation > 0
// GROUP BY
// u.Id, u.DisplayName
// ORDER BY
// TotalVotes DESC, TotalPosts DESC
// LIMIT 100;
fn q14324(db: &'static So) -> String {
    let g = || user_base(db, UserWhere::RepGt(0)).group_by(Ident::<User>::new());
    let f = g().select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt().and(badges_of(db).opt())).fold(0i64, |a, (x, _)| {
        a + match x.flatten() {
            Some(2) => 1,
            Some(3) => -1,
            _ => 0,
        }
    });
    let p = g().select(posts_of(db)).count_distinct();
    let b = g().select(badges_of(db)).count_distinct();
    let mut v = Vec::new();
    f.and((&p).opt()).and((&b).opt()).drive(|u, ((s, p), b)| v.push((u, s, p.unwrap_or(0), b.unwrap_or(0))));
    v.sort_by_key(|&(_, s, p, _)| (Reverse(s), Reverse(p)));
    rows(v.iter().take(100).map(|&(u, s, p, b)| row(vec![V::I(db.user.origid.get(u).unwrap()), V::S(db.user.display_name.get(u).unwrap()), V::I(s), V::I(p), V::I(b)])))
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// COUNT(DISTINCT c.Id) AS TotalComments,
// SUM(COALESCE(p.Score, 0)) AS TotalPostScore,
// SUM(COALESCE(c.Score, 0)) AS TotalCommentScore,
// SUM(COALESCE(b.Class, 0)) AS TotalBadges,
// MAX(p.CreationDate) AS LastPostDate
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
// ORDER BY
// TotalPosts DESC
// LIMIT 100;
fn q13175(db: &'static So) -> String {
    let g = || db.user.group_by(Ident::<User>::new());
    let Post { score, creation_date, .. } = &db.post;
    let f = g()
        .select(posts_of(db).select(score.and(creation_date).and(comments_of(db).select(&db.comment.score).opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0, 0, 0, i64::MIN, 0], |a: [i64; 5], (p, b)| {
            let (s, c, cs) = match p {
                Some(((s, c), cs)) => (s, Some(c), cs.unwrap_or(0)),
                None => (0, None, 0),
            };
            [a[0] + s, a[1] + cs, a[2] + b.unwrap_or(0), c.map_or(a[3], |c| a[3].max(c)), a[4] + c.is_some() as i64]
        });
    let p = g().select(posts_of(db)).count_distinct();
    let c = g().select(posts_of(db).select(comments_of(db))).count_distinct();
    let mut v = Vec::new();
    f.and((&p).opt()).and((&c).opt()).drive(|u, ((a, p), c)| v.push((u, a, p.unwrap_or(0), c.unwrap_or(0))));
    v.sort_by_key(|x| Reverse(x.2));
    rows(v.iter().take(100).map(|&(u, a, p, c)| {
        row(vec![
            V::I(db.user.origid.get(u).unwrap()),
            V::S(db.user.display_name.get(u).unwrap()),
            V::I(db.user.reputation.get(u).unwrap()),
            V::I(p),
            V::I(c),
            V::I(a[0]),
            V::I(a[1]),
            V::I(a[2]),
            if a[4] == 0 { V::Null } else { V::T(a[3]) },
        ])
    }))
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// AVG(u.Reputation) AS AvgReputation,
// SUM(p.ViewCount) AS TotalViewCount,
// COUNT(v.Id) AS TotalVotes
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN Votes v ON p.Id = v.PostId
// GROUP BY u.Id
// )
// SELECT
// COUNT(*) AS TotalUsers,
// AVG(AvgReputation) AS AverageReputation,
// SUM(TotalViewCount) AS TotalViewsAcrossUsers,
// SUM(TotalVotes) AS TotalVotesAcrossUsers
// FROM UserPostStats;
fn q10900(db: &'static So) -> String {
    let per_user = db
        .user
        .group_by(Ident::<User>::new())
        .select((&db.user.reputation).and(posts_of(db).select((&db.post.view_count).opt().and(votes_of(db).opt())).opt()))
        .fold([0i64; 5], |a, (r, p)| {
            let (w, x) = match p {
                Some((w, x)) => (w, x.is_some()),
                None => (None, false),
            };
            [a[0] + 1, a[1] + r, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + x as i64]
        });
    let (n, rs, wn, ws, x) = (&per_user).fold_flat((0i64, 0.0f64, 0i64, 0i64, 0i64), |(n, rs, wn, ws, x), a| {
        (n + 1, rs + a[1] as f64 / a[0] as f64, wn + (a[2] > 0) as i64, ws + a[3], x + a[4])
    });
    row(vec![V::I(n), if n == 0 { V::Null } else { V::F(rs / n as f64) }, nullable(ws, wn), nullable(x, n)])
}

fn count<Q: Drive>(q: Q) -> i64 {
    q.fold_flat(0i64, |a, _| a + 1)
}

fn one(f: Fold<(), i64>) -> i64 {
    (&f).fold_flat(0i64, |a, x| a + x)
}

// SELECT
// (SELECT COUNT(*) FROM Posts) AS TotalPosts,
// (SELECT COUNT(*) FROM Comments) AS TotalComments,
// (SELECT COUNT(*) FROM Votes) AS TotalVotes,
// (SELECT COUNT(*) FROM Users) AS TotalUsers,
// (SELECT AVG(Reputation) FROM Users) AS AverageUserReputation,
// (SELECT AVG(ViewCount) FROM Posts) AS AveragePostViewCount,
// (SELECT COUNT(DISTINCT PostTypeId) FROM Posts) AS DistinctPostTypes,
// (SELECT COUNT(DISTINCT VoteTypeId) FROM Votes) AS DistinctVoteTypes,
// (SELECT COUNT(DISTINCT TagName) FROM Tags) AS DistinctTags
fn q13816(db: &'static So) -> String {
    let (rn, rs) = (&db.user.reputation).fold_flat((0i64, 0i64), |(n, s), x| (n + 1, s + x));
    let (vn, vs) = db.post.select(&db.post.view_count).fold_flat((0i64, 0i64), |(n, s), x| (n + 1, s + x));
    row(vec![
        V::I(count(db.post.iq())),
        V::I(count(db.comment.iq())),
        V::I(count(db.vote.iq())),
        V::I(count(db.user.iq())),
        avg(rs, rn),
        avg(vs, vn),
        V::I(one(whole(db.post.iq()).select(&db.post.post_type_id).count_distinct())),
        V::I(one(whole(db.vote.iq()).select(&db.vote.vote_type_id).count_distinct())),
        V::I(one(whole(db.tag.iq()).select(&db.tag.tag_name).count_distinct())),
    ])
}

pub static ENTRIES: &[harness::Entry] = &[
    ("13325", q13325),
    ("10737", q10737),
    ("11251", q11251),
    ("14420", q14420),
    ("10872", q10872),
    ("14943", q14943),
    ("10435", q10435),
    ("11266", q11266),
    ("10130", q10130),
    ("11479", q11479),
    ("13826", q13826),
    ("14999", q14999),
    ("10909", q10909),
    ("13114", q13114),
    ("12194", q12194),
    ("11702", q11702),
    ("11792", q11792),
    ("13992", q13992),
    ("11778", q11778),
    ("10356", q10356),
    ("14942", q14942),
    ("13410", q13410),
    ("13143", q13143),
    ("14567", q14567),
    ("13314", q13314),
    ("11600", q11600),
    ("16076", q16076),
    ("15390", q15390),
    ("11235", q11235),
    ("11944", q11944),
    ("14853", q14853),
    ("14673", q14673),
    ("11493", q11493),
    ("19011", q19011),
    ("14752", q14752),
    ("14794", q14794),
    ("19911", q19911),
    ("13020", q13020),
    ("14271", q14271),
    ("12831", q12831),
    ("13151", q13151),
    ("10371", q10371),
    ("10907", q10907),
    ("18229", q18229),
    ("14008", q14008),
    ("14160", q14160),
    ("11006", q11006),
    ("10898", q10898),
    ("14324", q14324),
    ("13175", q13175),
    ("10900", q10900),
    ("13816", q13816),
];
