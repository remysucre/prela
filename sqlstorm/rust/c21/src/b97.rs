use harness::prelude::*;

fn types(db: &'static So, cols: &[&str]) -> String {
    rows(by_count(db).iter().map(|a| row(type_fields(a, cols))))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// SUM(p.ViewCount) AS TotalViews,
// AVG(p.Score) AS AverageScore,
// SUM(CASE WHEN p.OwnerUserId IS NOT NULL THEN 1 ELSE 0 END) AS TotalPostsByUsers,
// SUM(p.AnswerCount) AS TotalAnswers
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q10941(db: &'static So) -> String {
    types(db, &["name", "n", "views_sum", "score_avg", "owner_n", "answers_sum"])
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.Score IS NOT NULL THEN p.Score ELSE 0 END) AS TotalScore,
// SUM(p.ViewCount) AS TotalViews,
// AVG(p.CommentCount) AS AverageComments,
// AVG(p.AnswerCount) AS AverageAnswers
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q13468(db: &'static So) -> String {
    types(db, &["name", "n", "score_sum", "views_sum", "comment_avg", "answers_avg"])
}

// WITH PostStatistics AS (
// SELECT
// pt.Name AS PostTypeName,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.ViewCount) AS AverageViewCount
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// GROUP BY
// pt.Name
// )
// SELECT
// PostTypeName,
// TotalPosts,
// AverageViewCount
// FROM
// PostStatistics
// ORDER BY
// TotalPosts DESC;
fn q12046(db: &'static So) -> String {
    types(db, &["name", "n", "views_avg"])
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AverageScore,
// SUM(CASE WHEN p.OwnerUserId IS NOT NULL THEN 1 ELSE 0 END) AS TotalOwnedPosts,
// SUM(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS TotalAcceptedAnswers
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q12472(db: &'static So) -> String {
    types(db, &["name", "n", "score_avg", "owner_n", "accepted_n"])
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS PostCount,
// AVG(p.Score) AS AverageScore,
// SUM(p.ViewCount) AS TotalViews,
// SUM(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers,
// SUM(CASE WHEN p.ClosedDate IS NOT NULL THEN 1 ELSE 0 END) AS ClosedPosts
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// GROUP BY
// pt.Name
// ORDER BY
// PostCount DESC;
fn q11286(db: &'static So) -> String {
    types(db, &["name", "n", "score_avg", "views_sum", "accepted_n", "closed_n"])
}

// WITH PostMetrics AS (
// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS PostCount,
// AVG(p.Score) AS AvgScore,
// SUM(p.ViewCount) AS TotalViewCount
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// GROUP BY
// pt.Name
// )
// SELECT
// PostType,
// PostCount,
// AvgScore,
// TotalViewCount
// FROM
// PostMetrics
// ORDER BY
// PostCount DESC;
fn q14548(db: &'static So) -> String {
    types(db, &["name", "n", "score_avg", "views_sum"])
}

// WITH PostStats AS (
// SELECT
// pt.Name AS PostTypeName,
// COUNT(p.Id) AS PostCount,
// AVG(p.ViewCount) AS AvgViewCount,
// AVG(p.Score) AS AvgScore
// FROM
// Posts p
// INNER JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// GROUP BY
// pt.Name
// )
// SELECT
// PostTypeName,
// PostCount,
// AvgViewCount,
// AvgScore
// FROM
// PostStats
// ORDER BY
// PostCount DESC;
fn q13880(db: &'static So) -> String {
    types(db, &["name", "n", "views_avg", "score_avg"])
}

// WITH PostStats AS (
// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AverageScore,
// AVG(p.ViewCount) AS AverageViewCount
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
// AverageScore,
// AverageViewCount
// FROM
// PostStats
// ORDER BY
// TotalPosts DESC;
fn q12903(db: &'static So) -> String {
    types(db, &["name", "n", "score_avg", "views_avg"])
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AverageScore,
// AVG(p.ViewCount) AS AverageViewCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN p.AnswerCount ELSE 0 END) AS TotalAnswers,
// AVG(EXTRACT(EPOCH FROM (p.LastActivityDate - p.CreationDate)) / 3600) AS AverageTimeToActivityHours
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q12196(db: &'static So) -> String {
    let Post { post_type, score, view_count, answer_count, post_type_id, creation_date, last_activity_date, .. } = &db.post;
    let mut v = Vec::new();
    db.post
        .group_by(post_type.select(&db.post_type.name))
        .select(score.and(view_count.opt()).and(post_type_id).and(answer_count.opt()).and(last_activity_date).and(creation_date))
        .fold((0i64, 0i64, 0i64, 0i64, 0i64, 0i64, 0i128), |(n, s, vn, vs, an, as_, d), (((((x, w), t), a), la), cd)| {
            let qa = if t == 1 { a } else { Some(0) };
            (n + 1, s + x, vn + w.is_some() as i64, vs + w.unwrap_or(0), an + qa.is_some() as i64, as_ + qa.unwrap_or(0), d + (la - cd) as i128)
        })
        .drive(|k, a| v.push((k, a)));
    v.sort_by(|a, b| b.1.0.cmp(&a.1.0));
    rows(v.iter().map(|&(k, (n, s, vn, vs, an, as_, d))| {
        row(vec![V::S(k), V::I(n), avg(s, n), avg(vs, vn), nullable(as_, an), V::F(d as f64 / 1e6 / 3600.0 / n as f64)])
    }))
}

// GROUP BY p.PostTypeId, then JOIN PostTypes for the name.
fn by_type_id(db: &'static So) -> Vec<(i64, Str, i64, i64, i64, i64)> {
    let Post { post_type, score, view_count, .. } = &db.post;
    let mut v = Vec::new();
    db.post
        .group_by(post_type)
        .select(score.and(view_count.opt()))
        .fold((0i64, 0i64, 0i64, 0i64), |(n, s, vn, vs), (x, w)| (n + 1, s + x, vn + w.is_some() as i64, vs + w.unwrap_or(0)))
        .drive(|t, (n, s, vn, vs)| {
            v.push((db.post_type.origid.get(t).unwrap(), db.post_type.name.get(t).unwrap(), n, s, vn, vs))
        });
    v
}

// WITH PostStats AS (
// SELECT
// p.PostTypeId,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AverageScore,
// SUM(p.ViewCount) AS TotalViews
// FROM
// Posts p
// GROUP BY
// p.PostTypeId
// )
// SELECT
// pt.Name AS PostTypeName,
// ps.TotalPosts,
// ps.AverageScore,
// ps.TotalViews
// FROM
// PostTypes pt
// JOIN
// PostStats ps ON pt.Id = ps.PostTypeId
// ORDER BY
// pt.Id;
fn q12632(db: &'static So) -> String {
    let mut v = by_type_id(db);
    v.sort_by_key(|x| x.0);
    rows(v.iter().map(|&(_, k, n, s, vn, vs)| row(vec![V::S(k), V::I(n), avg(s, n), nullable(vs, vn)])))
}

// WITH PostStats AS (
// SELECT
// p.PostTypeId,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.ViewCount) AS AverageViews,
// AVG(p.Score) AS AverageScore
// FROM
// Posts p
// GROUP BY
// p.PostTypeId
// )
// SELECT
// pt.Name AS PostType,
// ps.TotalPosts,
// ps.AverageViews,
// ps.AverageScore
// FROM
// PostTypes pt
// JOIN
// PostStats ps ON pt.Id = ps.PostTypeId
// ORDER BY
// ps.TotalPosts DESC;
fn q10113(db: &'static So) -> String {
    let mut v = by_type_id(db);
    v.sort_by(|a, b| b.2.cmp(&a.2));
    rows(v.iter().map(|&(_, k, n, s, vn, vs)| row(vec![V::S(k), V::I(n), avg(vs, vn), avg(s, n)])))
}

// WITH PostMetrics AS (
// SELECT
// pt.Id AS PostTypeId,
// pt.Name AS PostTypeName,
// COUNT(p.Id) AS PostCount,
// AVG(p.Score) AS AverageScore,
// SUM(p.ViewCount) AS TotalViews
// FROM
// Posts p
// INNER JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// GROUP BY
// pt.Id, pt.Name
// )
// SELECT
// PostTypeId,
// PostTypeName,
// PostCount,
// AverageScore,
// TotalViews
// FROM
// PostMetrics
// ORDER BY
// PostCount DESC;
fn q11514(db: &'static So) -> String {
    let mut v = by_type_id(db);
    v.sort_by(|a, b| b.2.cmp(&a.2));
    rows(v.iter().map(|&(id, k, n, s, vn, vs)| row(vec![V::I(id), V::S(k), V::I(n), avg(s, n), nullable(vs, vn)])))
}

// COUNT, AVG(Score), AVG(ViewCount) and COUNT(DISTINCT p.OwnerUserId) per
// type over the posts created since `since`.
fn recent_types(db: &'static So, since: i64) -> Vec<(i64, Str, i64, i64, i64, i64)> {
    let Post { post_type, score, view_count, owner_user_id, creation_date, .. } = &db.post;
    let base = db.post.with(creation_date.ge(since));
    let name = post_type.select(&db.post_type.name);
    let main = (&base).group_by(&name).select(score.and(view_count.opt())).fold((0i64, 0i64, 0i64, 0i64), |(n, s, vn, vs), (x, w)| {
        (n + 1, s + x, vn + w.is_some() as i64, vs + w.unwrap_or(0))
    });
    let uniq = (&base).group_by(&name).select(owner_user_id).count_distinct();
    let mut v = Vec::new();
    main.and((&uniq).opt()).drive(|k, ((n, s, vn, vs), u)| v.push((n, k, s, vn, vs, u.unwrap_or(0))));
    v.sort_by(|a, b| b.0.cmp(&a.0));
    v
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS PostCount,
// AVG(p.Score) AS AverageScore,
// AVG(p.ViewCount) AS AverageViewCount,
// COUNT(DISTINCT p.OwnerUserId) AS ActiveUsers
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// WHERE
// p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'
// GROUP BY
// pt.Name
// ORDER BY
// PostCount DESC;
fn q13587(db: &'static So) -> String {
    rows(recent_types(db, date(2023, 10, 1)).iter().map(|&(n, k, s, vn, vs, u)| row(vec![V::S(k), V::I(n), avg(s, n), avg(vs, vn), V::I(u)])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AverageScore,
// AVG(p.ViewCount) AS AverageViewCount,
// COUNT(DISTINCT p.OwnerUserId) AS UniqueUsers
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q11077(db: &'static So) -> String {
    rows(recent_types(db, ts(2023, 10, 1, 12, 34, 56)).iter().map(|&(n, k, s, vn, vs, u)| row(vec![V::S(k), V::I(n), avg(s, n), avg(vs, vn), V::I(u)])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS PostCount,
// SUM(p.ViewCount) AS TotalViews,
// AVG(p.Score) AS AverageScore,
// COUNT(DISTINCT u.Id) AS UniqueUsers,
// AVG(u.Reputation) AS AverageUserReputation
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'
// GROUP BY
// pt.Name
// ORDER BY
// PostCount DESC;
fn q11782(db: &'static So) -> String {
    let Post { post_type, score, view_count, owner_user, creation_date, .. } = &db.post;
    let base = owned(db).with(creation_date.ge(ts(2024, 9, 1, 12, 34, 56)));
    let name = post_type.select(&db.post_type.name);
    let main = (&base).group_by(&name).select(score.and(view_count.opt()).and(owner_user.select(&db.user.reputation))).fold(
        (0i64, 0i64, 0i64, 0i64, 0i64),
        |(n, s, vn, vs, r), ((x, w), rep)| (n + 1, s + x, vn + w.is_some() as i64, vs + w.unwrap_or(0), r + rep),
    );
    let uniq = (&base).group_by(&name).select(owner_user).count_distinct();
    let mut v = Vec::new();
    main.and(&uniq).drive(|k, ((n, s, vn, vs, r), u)| v.push((n, k, s, vn, vs, r, u)));
    v.sort_by(|a, b| b.0.cmp(&a.0));
    rows(v.iter().map(|&(n, k, s, vn, vs, r, u)| row(vec![V::S(k), V::I(n), nullable(vs, vn), avg(s, n), V::I(u), avg(r, n)])))
}

// SELECT
// COUNT(*) AS TotalPosts,
// SUM(CASE WHEN PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// AVG(ViewCount) AS AverageViews,
// AVG(Score) AS AverageScore,
// MAX(CreationDate) AS MostRecentPost,
// MIN(CreationDate) AS OldestPost
// FROM
// Posts
// WHERE
// CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year';
fn q12713(db: &'static So) -> String {
    let Post { post_type_id, view_count, score, creation_date, .. } = &db.post;
    let (n, q, a, vn, vs, s, mx, mn) = db
        .post
        .with(creation_date.ge(ts(2023, 10, 1, 12, 34, 56)))
        .select(post_type_id.and(view_count.opt()).and(score).and(creation_date))
        .fold_flat((0i64, 0i64, 0i64, 0i64, 0i64, 0i64, i64::MIN, i64::MAX), |(n, q, a, vn, vs, s, mx, mn), (((t, w), x), cd)| {
            (n + 1, q + (t == 1) as i64, a + (t == 2) as i64, vn + w.is_some() as i64, vs + w.unwrap_or(0), s + x, mx.max(cd), mn.min(cd))
        });
    row(vec![V::I(n), V::I(q), V::I(a), avg(vs, vn), avg(s, n), V::T(mx), V::T(mn)])
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// COUNT(c.Id) AS TotalComments,
// COUNT(v.Id) AS TotalVotes,
// AVG(p.Score) AS AvgScore,
// MAX(p.Score) AS MaxScore
// FROM
// PostTypes pt
// LEFT JOIN
// Posts p ON pt.Id = p.PostTypeId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// pt.Id, pt.Name
// ORDER BY
// pt.Id;
fn q12088(db: &'static So) -> String {
    let Post { score, .. } = &db.post;
    let posts_of_type: HashIdx<Id<PostType>, Id<Post>> = (&db.post.post_type).inv().collect();
    let mut v = Vec::new();
    db.post_type
        .group_by(Ident::<PostType>::new())
        .select((&posts_of_type).select(score.and(comments_of(db).opt()).and(votes_of(db).opt())).opt())
        .fold((0i64, 0i64, 0i64, 0i64, i64::MIN), |(n, c, w, s, mx), r| match r {
            Some(((x, ci), vi)) => (n + 1, c + ci.is_some() as i64, w + vi.is_some() as i64, s + x, mx.max(x)),
            None => (n, c, w, s, mx),
        })
        .drive(|t, (n, c, w, s, mx)| v.push((db.post_type.origid.get(t).unwrap(), db.post_type.name.get(t).unwrap(), n, c, w, s, mx)));
    v.sort_by_key(|x| x.0);
    rows(v.iter().map(|&(_, k, n, c, w, s, mx)| row(vec![V::S(k), V::I(n), V::I(c), V::I(w), avg(s, n), omax(mx, n)])))
}

// GROUP BY pt.Name, u.Reputation over Posts LEFT JOIN Users LEFT JOIN Badges:
// a post counts once per badge its owner holds.
fn type_rep_badges(db: &'static So) -> Vec<(Str, Option<i64>, i64, i64, i64, i64, i64)> {
    let Post { post_type, score, view_count, owner_user, .. } = &db.post;
    let key = post_type.select(&db.post_type.name).and(owner_user.select(&db.user.reputation).opt());
    let mut v = Vec::new();
    db.post
        .group_by(key)
        .select(score.and(view_count.opt()).and(owner_user.select(badges_of(db)).opt()))
        .fold((0i64, 0i64, 0i64, 0i64, 0i64), |(n, s, vn, vs, b), ((x, w), bi)| {
            (n + 1, s + x, vn + w.is_some() as i64, vs + w.unwrap_or(0), b + bi.is_some() as i64)
        })
        .drive(|(k, r), (n, s, vn, vs, b)| v.push((k, r, n, s, vn, vs, b)));
    v
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS PostCount,
// AVG(p.Score) AS AverageScore,
// u.Reputation AS UserReputation,
// COUNT(b.Id) AS BadgeCount
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// pt.Name, u.Reputation
// ORDER BY
// PostCount DESC;
fn q11965(db: &'static So) -> String {
    rows(type_rep_badges(db).iter().map(|&(k, r, n, s, _, _, b)| row(vec![V::S(k), V::I(n), avg(s, n), oint(r), V::I(b)])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS PostCount,
// AVG(p.Score) AS AverageScore,
// SUM(p.ViewCount) AS TotalViews,
// u.Reputation AS UserReputation,
// COUNT(b.Id) AS BadgeCount
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// pt.Name, u.Reputation
// ORDER BY
// PostCount DESC;
fn q14523(db: &'static So) -> String {
    rows(type_rep_badges(db).iter().map(|&(k, r, n, s, vn, vs, b)| row(vec![V::S(k), V::I(n), avg(s, n), nullable(vs, vn), oint(r), V::I(b)])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AvgScore,
// AVG(p.ViewCount) AS AvgViewCount,
// u.Reputation AS UserReputation,
// u.CreationDate AS UserCreationDate
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// GROUP BY
// pt.Name, u.Reputation, u.CreationDate
// ORDER BY
// TotalPosts DESC, AvgScore DESC;
fn q10166(db: &'static So) -> String {
    let Post { post_type, score, view_count, owner_user, .. } = &db.post;
    let key = post_type.select(&db.post_type.name).and(owner_user.select((&db.user.reputation).and(&db.user.creation_date)).opt());
    let mut v = Vec::new();
    db.post
        .group_by(key)
        .select(score.and(view_count.opt()))
        .fold((0i64, 0i64, 0i64, 0i64), |(n, s, vn, vs), (x, w)| (n + 1, s + x, vn + w.is_some() as i64, vs + w.unwrap_or(0)))
        .drive(|(k, u), (n, s, vn, vs)| v.push((k, u, n, s, vn, vs)));
    rows(v.iter().map(|&(k, u, n, s, vn, vs)| {
        row(vec![V::S(k), V::I(n), avg(s, n), avg(vs, vn), oint(u.map(|x| x.0)), ots(u.map(|x| x.1))])
    }))
}

// SELECT
// pt.Name AS PostType,
// u.Reputation AS UserReputation,
// COUNT(p.Id) AS PostCount,
// AVG(p.ViewCount) AS AvgViewCount,
// AVG(p.Score) AS AvgScore,
// SUM(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswersCount
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// JOIN
// Users u ON p.OwnerUserId = u.Id
// GROUP BY
// pt.Name, u.Reputation
// ORDER BY
// PostCount DESC, UserReputation DESC;
fn q14367(db: &'static So) -> String {
    let Post { post_type, score, view_count, owner_user, accepted_answer_id, .. } = &db.post;
    let key = post_type.select(&db.post_type.name).and(owner_user.select(&db.user.reputation));
    let mut v = Vec::new();
    db.post
        .group_by(key)
        .select(score.and(view_count.opt()).and(accepted_answer_id.opt()))
        .fold((0i64, 0i64, 0i64, 0i64, 0i64), |(n, s, vn, vs, a), ((x, w), ac)| {
            (n + 1, s + x, vn + w.is_some() as i64, vs + w.unwrap_or(0), a + ac.is_some() as i64)
        })
        .drive(|(k, r), (n, s, vn, vs, a)| v.push((k, r, n, s, vn, vs, a)));
    rows(v.iter().map(|&(k, r, n, s, vn, vs, a)| row(vec![V::S(k), V::I(r), V::I(n), avg(vs, vn), avg(s, n), V::I(a)])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AverageScore,
// AVG(p.ViewCount) AS AverageViewCount,
// COUNT(c.Id) AS TotalComments,
// COUNT(v.Id) AS TotalVotes
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
fn q14259(db: &'static So) -> String {
    let Post { post_type, score, view_count, .. } = &db.post;
    let mut v = Vec::new();
    db.post
        .group_by(post_type.select(&db.post_type.name))
        .select(score.and(view_count.opt()).and(comments_of(db).opt()).and(votes_of(db).opt()))
        .fold((0i64, 0i64, 0i64, 0i64, 0i64, 0i64), |(n, s, vn, vs, c, w), (((x, vw), ci), vi)| {
            (n + 1, s + x, vn + vw.is_some() as i64, vs + vw.unwrap_or(0), c + ci.is_some() as i64, w + vi.is_some() as i64)
        })
        .drive(|k, a| v.push((k, a)));
    v.sort_by(|a, b| b.1.0.cmp(&a.1.0));
    rows(v.iter().map(|&(k, (n, s, vn, vs, c, w))| row(vec![V::S(k), V::I(n), avg(s, n), avg(vs, vn), V::I(c), V::I(w)])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AverageScore,
// SUM(p.ViewCount) AS TotalViews,
// COUNT(c.Id) AS TotalComments,
// AVG(u.Reputation) AS AverageUserReputation
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q14730(db: &'static So) -> String {
    let Post { post_type, score, view_count, owner_user, .. } = &db.post;
    let mut v = Vec::new();
    db.post
        .group_by(post_type.select(&db.post_type.name))
        .select(score.and(view_count.opt()).and(comments_of(db).opt()).and(owner_user.select(&db.user.reputation).opt()))
        .fold((0i64, 0i64, 0i64, 0i64, 0i64, 0i64, 0i64), |(n, s, vn, vs, c, rn, rs), (((x, vw), ci), r)| {
            (n + 1, s + x, vn + vw.is_some() as i64, vs + vw.unwrap_or(0), c + ci.is_some() as i64, rn + r.is_some() as i64, rs + r.unwrap_or(0))
        })
        .drive(|k, a| v.push((k, a)));
    v.sort_by(|a, b| b.1.0.cmp(&a.1.0));
    rows(v.iter().map(|&(k, (n, s, vn, vs, c, rn, rs))| row(vec![V::S(k), V::I(n), avg(s, n), nullable(vs, vn), V::I(c), avg(rs, rn)])))
}

// SELECT
// Users.DisplayName,
// Posts.Title,
// Posts.CreationDate,
// COUNT(Comments.Id) AS CommentCount
// FROM
// Posts
// JOIN
// Users ON Posts.OwnerUserId = Users.Id
// LEFT JOIN
// Comments ON Comments.PostId = Posts.Id
// WHERE
// Posts.PostTypeId = 1
// GROUP BY
// Users.DisplayName, Posts.Title, Posts.CreationDate
// ORDER BY
// Posts.CreationDate DESC
// LIMIT 10;
fn q18441(db: &'static So) -> String {
    tuple_rows(by_name_title_date(db, true, "c", PostWhere::All), "created", 10, &["owner", "title", "created", "#c"])
}

// SELECT
// p.Title,
// p.CreationDate,
// u.DisplayName AS Owner,
// p.Score,
// p.ViewCount,
// COUNT(c.Id) AS CommentCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON c.PostId = p.Id
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Title, p.CreationDate, u.DisplayName, p.Score, p.ViewCount
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q19727(db: &'static So) -> String {
    tuple_rows(by_name_title_date_score_views(db, true, "c", PostWhere::All), "created", 10, &["title", "created", "owner", "score", "views", "#c"])
}

// SELECT
// Users.DisplayName,
// Posts.Title,
// Posts.CreationDate,
// Posts.Score,
// COUNT(Comments.Id) AS CommentCount
// FROM
// Posts
// JOIN
// Users ON Posts.OwnerUserId = Users.Id
// LEFT JOIN
// Comments ON Comments.PostId = Posts.Id
// WHERE
// Posts.PostTypeId = 1
// GROUP BY
// Users.DisplayName, Posts.Title, Posts.CreationDate, Posts.Score
// ORDER BY
// Posts.Score DESC
// LIMIT 10;
fn q15963(db: &'static So) -> String {
    tuple_rows(by_name_title_date_score(db, true, "c", PostWhere::All), "score", 10, &["owner", "title", "created", "score", "#c"])
}

// SELECT
// Posts.Title,
// Users.DisplayName,
// Posts.CreationDate,
// Posts.Score,
// COUNT(Comments.Id) AS CommentCount
// FROM
// Posts
// JOIN
// Users ON Posts.OwnerUserId = Users.Id
// LEFT JOIN
// Comments ON Comments.PostId = Posts.Id
// WHERE
// Posts.PostTypeId = 1
// GROUP BY
// Posts.Title, Users.DisplayName, Posts.CreationDate, Posts.Score
// ORDER BY
// Posts.Score DESC
// LIMIT 10;
fn q17752(db: &'static So) -> String {
    tuple_rows(by_name_title_date_score(db, true, "c", PostWhere::All), "score", 10, &["title", "owner", "created", "score", "#c"])
}

// SELECT
// Users.DisplayName,
// Posts.Title,
// Posts.CreationDate,
// Posts.Score,
// COUNT(Comments.Id) AS CommentCount
// FROM
// Posts
// JOIN
// Users ON Posts.OwnerUserId = Users.Id
// LEFT JOIN
// Comments ON Comments.PostId = Posts.Id
// WHERE
// Posts.PostTypeId = 1
// GROUP BY
// Users.DisplayName, Posts.Title, Posts.CreationDate, Posts.Score
// ORDER BY
// Posts.CreationDate DESC
// LIMIT 10;
fn q18278(db: &'static So) -> String {
    tuple_rows(by_name_title_date_score(db, true, "c", PostWhere::All), "created", 10, &["owner", "title", "created", "score", "#c"])
}

// SELECT
// Users.DisplayName,
// Posts.Title,
// Posts.CreationDate,
// Posts.ViewCount,
// COUNT(Comments.Id) AS CommentCount
// FROM
// Posts
// JOIN
// Users ON Posts.OwnerUserId = Users.Id
// LEFT JOIN
// Comments ON Comments.PostId = Posts.Id
// WHERE
// Posts.PostTypeId = 1
// GROUP BY
// Users.DisplayName, Posts.Title, Posts.CreationDate, Posts.ViewCount
// ORDER BY
// Posts.ViewCount DESC
// LIMIT 10;
fn q17363(db: &'static So) -> String {
    tuple_rows(by_name_title_date_views(db, true, "c", PostWhere::All), "views", 10, &["owner", "title", "created", "views", "#c"])
}

// SELECT
// P.Title,
// P.CreationDate,
// U.DisplayName AS OwnerDisplayName,
// P.Score,
// P.ViewCount,
// COUNT(A.Id) AS AnswerCount
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// Posts A ON A.ParentId = P.Id AND A.PostTypeId = 2
// WHERE
// P.PostTypeId = 1
// GROUP BY
// P.Title, P.CreationDate, U.DisplayName, P.Score, P.ViewCount
// ORDER BY
// P.CreationDate DESC
// LIMIT 10;
fn q16383(db: &'static So) -> String {
    tuple_rows(by_name_title_date_score_views(db, true, "A", PostWhere::All), "created", 10, &["title", "created", "owner", "score", "views", "#a"])
}

// SELECT
// p.Title,
// p.CreationDate,
// u.DisplayName AS OwnerDisplayName,
// COUNT(c.Id) AS CommentCount,
// COUNT(a.Id) AS AnswerCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Title, p.CreationDate, u.DisplayName
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q16219(db: &'static So) -> String {
    tuple_rows(by_name_title_date(db, true, "cA", PostWhere::All), "created", 10, &["title", "created", "owner", "#cx", "#ax"])
}

// SELECT
// p.Title,
// p.CreationDate,
// u.DisplayName AS Author,
// COUNT(c.Id) AS CommentCount,
// COUNT(a.Id) AS AnswerCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Title, p.CreationDate, u.DisplayName
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q19667(db: &'static So) -> String {
    tuple_rows(by_name_title_date(db, true, "cA", PostWhere::All), "created", 10, &["title", "created", "owner", "#cx", "#ax"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.ViewCount,
// p.CreationDate,
// p.Score,
// COUNT(v.Id) AS VoteCount,
// COUNT(c.Id) AS CommentCount
// FROM
// Posts p
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Id, p.Title, p.ViewCount, p.CreationDate, p.Score
// ORDER BY
// VoteCount DESC, p.Score DESC;
fn q10379(db: &'static So) -> String {
    render_posts(db, posts_where(db, false, true, "cv", PostWhere::CreatedGe(date(2023, 1, 1))), "#vx,score", 0, &["id", "title", "views", "created", "score", "#vx", "#cx"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// MAX(ph.CreationDate) AS LastEditDate
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score
// ORDER BY
// p.Score DESC, COUNT(c.Id) DESC
// LIMIT 100;
fn q14140(db: &'static So) -> String {
    render_posts(db, posts_with_counts(db, false, true, "cvh"), "score,#cx", 100, &["id", "title", "created", "score", "#cx", "#vx", "#hmax"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate AS PostCreationDate,
// p.ViewCount AS PostViewCount,
// p.Score AS PostScore,
// COUNT(c.Id) AS CommentCount,
// u.Reputation AS UserReputation
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.Reputation
// ORDER BY
// p.Score DESC, CommentCount DESC;
fn q11275(db: &'static So) -> String {
    render_posts(db, posts_with_counts(db, false, true, "c"), "score,#c", 0, &["id", "title", "created", "views", "score", "#c", "rep"])
}

// The groups are (post columns, t.TagName) where the tag is the post's
// excerpt tag, at most one per post, so the key is a function of the post.
fn excerpt_tag(db: &'static So) -> HashIdx<Id<Post>, Str> {
    (&db.tag.excerpt_post).inv().select(&db.tag.tag_name).collect()
}

// SELECT
// p.Title,
// p.CreationDate,
// u.DisplayName AS Owner,
// COUNT(a.Id) AS AnswerCount,
// t.TagName
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Posts a ON p.Id = a.ParentId
// LEFT JOIN
// Tags t ON t.ExcerptPostId = p.Id
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Title, p.CreationDate, u.DisplayName, t.TagName
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q17156(db: &'static So) -> String {
    let Post { title, creation_date, owner_user, post_type_id, .. } = &db.post;
    let tag = excerpt_tag(db);
    let key = title.opt().and(creation_date).and(owner_user.select(&db.user.display_name)).and((&tag).opt());
    let base = owned(db).with(post_type_id.eq(1));
    let mut v = group_posts(db, &base, &key, "a");
    v.sort_by(|a, b| b.0.0.0.1.cmp(&a.0.0.0.1));
    rows(v.iter().take(10).map(|&((((t, cd), dn), tn), a)| row(vec![ostr(t), V::T(cd), V::S(dn), V::I(a.ax), ostr(tn)])))
}

// SELECT
// Users.DisplayName,
// Posts.Title,
// Posts.CreationDate,
// Tags.TagName,
// COUNT(Comments.Id) AS CommentCount
// FROM
// Users
// JOIN
// Posts ON Users.Id = Posts.OwnerUserId
// LEFT JOIN
// Comments ON Posts.Id = Comments.PostId
// JOIN
// Tags ON Posts.Id = Tags.ExcerptPostId
// GROUP BY
// Users.DisplayName, Posts.Title, Posts.CreationDate, Tags.TagName
// ORDER BY
// CommentCount DESC;
fn q17257(db: &'static So) -> String {
    let Post { title, creation_date, owner_user, .. } = &db.post;
    let tag = excerpt_tag(db);
    let key = owner_user.select(&db.user.display_name).and(title.opt()).and(creation_date).and(&tag);
    let base = owned(db);
    let v = group_posts(db, &base, &key, "c");
    rows(v.iter().map(|&((((dn, t), cd), tn), a)| row(vec![V::S(dn), ostr(t), V::T(cd), V::S(tn), V::I(a.cx)])))
}

// SELECT
// U.DisplayName AS UserName,
// P.Title AS PostTitle,
// PH.CreationDate AS HistoryDate,
// P.Score AS PostScore,
// C.Score AS CommentScore
// FROM
// Users U
// JOIN
// Posts P ON U.Id = P.OwnerUserId
// JOIN
// PostHistory PH ON P.Id = PH.PostId
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// WHERE
// P.CreationDate >= '2023-01-01'
// ORDER BY
// PH.CreationDate DESC, C.Score DESC;
fn q17041(db: &'static So) -> String {
    let Post { creation_date, owner_user, score, title, .. } = &db.post;
    let mut v = Vec::new();
    owned(db)
        .with(creation_date.ge(date(2023, 1, 1)))
        .select(
            owner_user
                .select(&db.user.display_name)
                .and(score)
                .and(history_of(db).select(&db.post_history.creation_date))
                .and(comments_of(db).select(&db.comment.score).opt()),
        )
        .drive(|p, (((dn, s), hd), cs)| v.push(row(vec![V::S(dn), ostr(title.get(p)), V::T(hd), V::I(s), oint(cs)])));
    rows(v)
}

// SELECT
// u.DisplayName AS UserDisplayName,
// p.Title AS PostTitle,
// p.CreationDate AS PostCreationDate,
// p.Score AS PostScore,
// c.Text AS CommentText,
// c.CreationDate AS CommentCreationDate
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// WHERE
// p.PostTypeId = 1
// ORDER BY
// p.Score DESC,
// c.CreationDate DESC
// LIMIT 10;
fn q18453(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, .. } = &db.post;
    let mut v = Vec::new();
    owned(db)
        .with(post_type_id.eq(1))
        .select(score.and(creation_date).and(comments_of(db).opt()))
        .drive(|p, ((s, cd), c)| v.push((s, c.map(|c| db.comment.creation_date.get(c).unwrap()), p, cd, c)));
    v.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| desc_nulls_last(a.1, b.1)));
    rows(v.iter().take(10).map(|&(s, ccd, p, cd, c)| {
        row(vec![
            post_fields(db, p, &["owner"]).pop().unwrap(),
            title(db, p),
            V::T(cd),
            V::I(s),
            ostr(c.map(|c| db.comment.text.get(c).unwrap())),
            ots(ccd),
        ])
    }))
}

// SELECT
// p.Title,
// u.DisplayName AS Owner,
// p.CreationDate,
// p.Score,
// ct.Name AS CloseReason
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId AND ph.PostHistoryTypeId = 10
// LEFT JOIN
// CloseReasonTypes ct ON CAST(ph.Comment AS int) = ct.Id
// WHERE
// p.PostTypeId = 1
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q19904(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let PostHistory { post_history_type_id, comment, .. } = &db.post_history;
    let reason: HashIdx<i64, Str> = (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect();
    let closes = db.post_history.with(post_history_type_id.eq(10)).select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason).opt());
    let mut v = Vec::new();
    owned(db)
        .with(post_type_id.eq(1))
        .select(creation_date.and(history_of(db).select(closes).opt()))
        .drive(|p, (cd, r)| v.push((cd, p, r.flatten())));
    v.sort_by(|a, b| b.0.cmp(&a.0));
    rows(v.iter().take(10).map(|&(_, p, r)| {
        let mut f = post_fields(db, p, &["title", "owner", "created", "score"]);
        f.push(ostr(r));
        row(f)
    }))
}

// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.ViewCount,
// P.AnswerCount,
// U.DisplayName AS OwnerDisplayName,
// PH.PostHistoryTypeId,
// PH.CreationDate AS HistoryCreationDate
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// PostHistory PH ON P.Id = PH.PostId
// WHERE
// PH.CreationDate >= '2023-01-01'
// ORDER BY
// P.ViewCount DESC,
// PH.CreationDate DESC
// LIMIT 100;
fn q13619(db: &'static So) -> String {
    let PostHistory { creation_date, post_history_type_id, .. } = &db.post_history;
    let mut v = Vec::new();
    owned(db)
        .select((&db.post.view_count).opt().and(history_of(db).select(creation_date.ge(date(2023, 1, 1)).and(post_history_type_id))))
        .drive(|p, (w, (hd, ht))| v.push((w, hd, p, ht)));
    v.sort_by(|a, b| desc_nulls_last(a.0, b.0).then_with(|| b.1.cmp(&a.1)));
    rows(v.iter().take(100).map(|&(_, hd, p, ht)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "answers", "owner"]);
        f.extend([V::I(ht), V::T(hd)]);
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// u.DisplayName AS OwnerDisplayName,
// p.Score,
// p.ViewCount,
// p.AnswerCount,
// p.CommentCount,
// ph.CreationDate AS LastHistoryDate,
// ph.Comment AS LastHistoryComment
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// WHERE
// p.CreationDate >= '2023-01-01'
// ORDER BY
// p.CreationDate DESC, p.Id, ph.Id
// LIMIT 100;
// rewrites/12544.sql: the ORDER BY gains p.Id, ph.Id as a tiebreak.
fn q12544(db: &'static So) -> String {
    let Post { creation_date, .. } = &db.post;
    let mut v = Vec::new();
    owned(db)
        .with(creation_date.ge(date(2023, 1, 1)))
        .select(creation_date.and(history_of(db).opt()))
        .drive(|p, (cd, h)| v.push((cd, p, h)));
    let hid = |h: Option<Id<PostHistory>>| h.map(|h| db.post_history.origid.get(h).unwrap());
    v.sort_by_key(|&(cd, p, h)| (-cd, db.post.origid.get(p).unwrap(), hid(h).is_none(), hid(h)));
    rows(v.iter().take(100).map(|&(_, p, h)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner", "score", "views", "answers", "comments"]);
        f.push(ots(h.map(|h| db.post_history.creation_date.get(h).unwrap())));
        f.push(ostr(h.and_then(|h| db.post_history.comment.get(h))));
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.Body,
// p.CreationDate AS PostCreationDate,
// p.Score,
// p.ViewCount,
// u.DisplayName AS OwnerDisplayName,
// u.Reputation AS OwnerReputation,
// v.VoteTypeId,
// v.CreationDate AS VoteCreationDate
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= '2023-01-01'
// ORDER BY
// p.CreationDate DESC
// LIMIT 1000;
fn q13993(db: &'static So) -> String {
    let Post { creation_date, .. } = &db.post;
    let mut v = Vec::new();
    owned(db)
        .with(creation_date.ge(date(2023, 1, 1)))
        .select(creation_date.and(votes_of(db).opt()))
        .drive(|p, (cd, x)| v.push((cd, p, x)));
    v.sort_by(|a, b| b.0.cmp(&a.0));
    rows(v.iter().take(1000).map(|&(_, p, x)| {
        let mut f = post_fields(db, p, &["id", "title", "body", "created", "score", "views", "owner", "rep"]);
        f.push(oint(x.map(|x| db.vote.vote_type_id.get(x).unwrap())));
        f.push(ots(x.map(|x| db.vote.creation_date.get(x).unwrap())));
        row(f)
    }))
}

// WITH UserPostStats AS (
// SELECT
// U.Id AS UserId,
// COUNT(P.Id) AS PostCount,
// AVG(P.Score) AS AvgScore,
// U.Reputation
// FROM
// Users U
// JOIN
// Posts P ON U.Id = P.OwnerUserId
// WHERE
// P.PostTypeId = 1
// GROUP BY
// U.Id, U.Reputation
// )
// SELECT
// U.UserId,
// U.PostCount,
// U.AvgScore,
// U.Reputation
// FROM
// UserPostStats U
// ORDER BY
// U.Reputation DESC
// LIMIT 10;
fn q14872(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, .. } = &db.post;
    let mut v = Vec::new();
    db.post
        .with(post_type_id.eq(1))
        .group_by(owner_user)
        .select(score)
        .fold((0i64, 0i64), |(n, s), x| (n + 1, s + x))
        .drive(|u, (n, s)| v.push((db.user.reputation.get(u).unwrap(), u, n, s)));
    v.sort_by(|a, b| b.0.cmp(&a.0));
    rows(v.iter().take(10).map(|&(r, u, n, s)| row(vec![V::I(db.user.origid.get(u).unwrap()), V::I(n), avg(s, n), V::I(r)])))
}

// SELECT
// PH.PostId,
// COUNT(PH.Id) AS RevisionCount,
// MAX(PH.CreationDate) AS LastRevisionDate,
// MIN(PH.CreationDate) AS FirstRevisionDate,
// U.DisplayName AS LastEditor,
// P.Title,
// P.Score,
// P.ViewCount
// FROM
// PostHistory PH
// JOIN
// Posts P ON PH.PostId = P.Id
// LEFT JOIN
// Users U ON PH.UserId = U.Id
// GROUP BY
// PH.PostId, U.DisplayName, P.Title, P.Score, P.ViewCount
// ORDER BY
// RevisionCount DESC, LastRevisionDate DESC;
fn q10136(db: &'static So) -> String {
    let PostHistory { post, user, creation_date, .. } = &db.post_history;
    let mut v = Vec::new();
    db.post_history
        .group_by(post.and(user.select(&db.user.display_name).opt()))
        .select(creation_date)
        .fold((0i64, i64::MIN, i64::MAX), |(n, mx, mn), d| (n + 1, mx.max(d), mn.min(d)))
        .drive(|(p, dn), (n, mx, mn)| v.push((n, mx, mn, p, dn)));
    v.sort_by(|a, b| (b.0, b.1).cmp(&(a.0, a.1)));
    rows(v.iter().map(|&(n, mx, mn, p, dn)| {
        let mut f = post_fields(db, p, &["id"]);
        f.extend([V::I(n), V::T(mx), V::T(mn), ostr(dn)]);
        f.extend(post_fields(db, p, &["title", "score", "views"]));
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// u.DisplayName AS OwnerDisplayName,
// COUNT(c.Id) AS CommentCount,
// COUNT(co.Id) AS CommentUsersCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Users co ON c.UserId = co.Id
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, p.CreationDate, u.DisplayName
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q17410(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let mut v = Vec::new();
    owned(db)
        .with(post_type_id.eq(1))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).select((&db.comment.user).opt()).opt())
        .fold((0i64, 0i64), |(c, u), x| (c + x.is_some() as i64, u + x.flatten().is_some() as i64))
        .drive(|p, (c, u)| v.push((creation_date.get(p).unwrap(), p, c, u)));
    v.sort_by(|a, b| b.0.cmp(&a.0));
    rows(v.iter().take(10).map(|&(_, p, c, u)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend([V::I(c), V::I(u)]);
        row(f)
    }))
}

// COUNT(c.Id) and SUM(v.BountyAmount) where Votes is joined only on the
// vote types in `types`.
fn bounty_rows<Q>(db: &'static So, base: Q, types: &'static [i64]) -> Vec<(Id<Post>, (i64, i64, i64))>
where
    Q: Drive<D = Id<Post>, R = Id<Post>>,
{
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let mut v = Vec::new();
    base.group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(vote_type_id.filt(move |t| types.contains(&t)).map(|_| ()).and(bounty_amount.opt())).opt()))
        .fold((0i64, 0i64, 0i64), |(c, bn, bs), (ci, vi)| {
            let b = vi.and_then(|(_, b)| b);
            (c + ci.is_some() as i64, bn + b.is_some() as i64, bs + b.unwrap_or(0))
        })
        .drive(|p, a| v.push((p, a)));
    v
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// u.DisplayName AS OwnerDisplayName,
// COUNT(c.Id) AS CommentCount,
// SUM(v.BountyAmount) AS TotalBounty
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (8, 9)
// GROUP BY
// p.Id, p.Title, p.CreationDate, u.DisplayName
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q17127(db: &'static So) -> String {
    let mut v = bounty_rows(db, owned(db), &[8, 9]);
    v.sort_by_key(|&(p, _)| -db.post.creation_date.get(p).unwrap());
    rows(v.iter().take(10).map(|&(p, (c, bn, bs))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend([V::I(c), nullable(bs, bn)]);
        row(f)
    }))
}

// SELECT
// p.Title,
// p.CreationDate,
// u.DisplayName AS OwnerDisplayName,
// COUNT(c.Id) AS CommentCount,
// SUM(v.BountyAmount) AS TotalBountyAmount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId AND v.VoteTypeId = 8
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Title, p.CreationDate, u.DisplayName
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q19453(db: &'static So) -> String {
    let Post { title, creation_date, owner_user, post_type_id, .. } = &db.post;
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let key = title.opt().and(creation_date).and(owner_user.select(&db.user.display_name));
    let mut v = Vec::new();
    owned(db)
        .with(post_type_id.eq(1))
        .group_by(key)
        .select(comments_of(db).opt().and(votes_of(db).select(vote_type_id.eq(8).map(|_| ()).and(bounty_amount.opt())).opt()))
        .fold((0i64, 0i64, 0i64), |(c, bn, bs), (ci, vi)| {
            let b = vi.and_then(|(_, b)| b);
            (c + ci.is_some() as i64, bn + b.is_some() as i64, bs + b.unwrap_or(0))
        })
        .drive(|k, a| v.push((k, a)));
    v.sort_by(|a, b| b.0.0.1.cmp(&a.0.0.1));
    rows(v.iter().take(10).map(|&(((t, cd), dn), (c, bn, bs))| row(vec![ostr(t), V::T(cd), V::S(dn), V::I(c), nullable(bs, bn)])))
}

// SELECT
// p.Title,
// p.CreationDate,
// u.DisplayName AS OwnerDisplayName,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS UpVoteCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId AND v.VoteTypeId = 2
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Title, p.CreationDate, u.DisplayName
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q19755(db: &'static So) -> String {
    let Post { title, creation_date, owner_user, post_type_id, .. } = &db.post;
    let key = owner_user.select(&db.user.display_name).and(title.opt()).and(creation_date);
    let mut v = Vec::new();
    owned(db)
        .with(post_type_id.eq(1))
        .group_by(key)
        .select(comments_of(db).opt().and(votes_of(db).select((&db.vote.vote_type_id).eq(2)).opt()))
        .fold((0i64, 0i64), |(c, u), (ci, vi)| (c + ci.is_some() as i64, u + vi.is_some() as i64))
        .drive(|k, a| v.push((k, a)));
    v.sort_by(|a, b| b.0.1.cmp(&a.0.1));
    rows(v.iter().take(10).map(|&(((dn, t), cd), (c, u))| row(vec![ostr(t), V::T(cd), V::S(dn), V::I(c), V::I(u)])))
}

// GROUP BY a post and a column of one of its joined children. The group is
// not a function of the post, so the joined rows are materialised first —
// (post, vote, comment), unique by construction — and grouped from there.
type Joined = MatSet<(Id<Post>, Option<Id<Vote>>, Option<Id<Comment>>)>;

fn questions_votes_comments(db: &'static So, outer_users: bool) -> Joined {
    let q = db.post.with((&db.post.post_type_id).eq(1));
    let rows = Ident::<Post>::new().and(votes_of(db).opt()).and(comments_of(db).opt());
    if outer_users {
        q.select(rows).map(|((p, v), c)| (p, v, c)).collect()
    } else {
        q.with(&db.post.owner_user).select(rows).map(|((p, v), c)| (p, v, c)).collect()
    }
}

// SELECT
// p.Title,
// p.CreationDate,
// u.DisplayName AS OwnerDisplayName,
// v.VoteTypeId,
// COUNT(c.Id) AS CommentCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Title, p.CreationDate, u.DisplayName, v.VoteTypeId
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q16138(db: &'static So) -> String {
    let j = questions_votes_comments(db, false);
    let post_of = (&j).map(|(p, _, _)| p);
    let vote_of = (&j).flat_map(|(_, v, _)| v);
    let comment_of = (&j).flat_map(|(_, _, c)| c);
    let Post { title, creation_date, owner_user, .. } = &db.post;
    let key = (&post_of).select(title.opt().and(creation_date).and(owner_user.select(&db.user.display_name))).and((&vote_of).select(&db.vote.vote_type_id).opt());
    let mut v = Vec::new();
    (&j).group_by(key)
        .select((&comment_of).opt())
        .fold(0i64, |a, c| a + c.is_some() as i64)
        .drive(|k, n| v.push((k, n)));
    v.sort_by(|a, b| b.0.0.0.1.cmp(&a.0.0.0.1));
    rows(v.iter().take(10).map(|&((((t, cd), dn), vt), n)| row(vec![ostr(t), V::T(cd), V::S(dn), oint(vt), V::I(n)])))
}

fn post_and_vote_type(db: &'static So) -> Vec<((Id<Post>, Option<i64>), i64)> {
    let j = questions_votes_comments(db, false);
    let post_of = (&j).map(|(p, _, _)| p);
    let vote_of = (&j).flat_map(|(_, v, _)| v);
    let comment_of = (&j).flat_map(|(_, _, c)| c);
    let key = (&post_of).and((&vote_of).select(&db.vote.vote_type_id).opt());
    let mut v = Vec::new();
    (&j).group_by(key)
        .select((&comment_of).opt())
        .fold(0i64, |a, c| a + c.is_some() as i64)
        .drive(|k, n| v.push((k, n)));
    v.sort_by_key(|&((p, vt), _)| (-db.post.creation_date.get(p).unwrap(), db.post.origid.get(p).unwrap(), vt.is_none(), vt));
    v
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// u.DisplayName AS OwnerDisplayName,
// v.VoteTypeId,
// COUNT(c.Id) AS CommentCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, p.CreationDate, u.DisplayName, v.VoteTypeId
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q19881(db: &'static So) -> String {
    rows(post_and_vote_type(db).iter().take(10).map(|&((p, vt), n)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend([oint(vt), V::I(n)]);
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// u.DisplayName AS OwnerName,
// v.VoteTypeId,
// COUNT(c.Id) AS CommentCount
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
// p.Id, p.Title, p.CreationDate, u.DisplayName, v.VoteTypeId
// ORDER BY
// p.CreationDate DESC
// FETCH FIRST 10 ROWS ONLY;
fn q15448(db: &'static So) -> String {
    rows(post_and_vote_type(db).iter().take(10).map(|&((p, vt), n)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend([oint(vt), V::I(n)]);
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// u.DisplayName AS OwnerDisplayName,
// p.CreationDate,
// v.CreationDate AS VoteDate,
// COUNT(c.Id) AS CommentCount
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, u.DisplayName, p.CreationDate, v.CreationDate
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q15915(db: &'static So) -> String {
    let j = questions_votes_comments(db, true);
    let post_of = (&j).map(|(p, _, _)| p);
    let vote_of = (&j).flat_map(|(_, v, _)| v);
    let comment_of = (&j).flat_map(|(_, _, c)| c);
    let key = (&post_of).and((&vote_of).select(&db.vote.creation_date).opt());
    let mut v = Vec::new();
    (&j).group_by(key)
        .select((&comment_of).opt())
        .fold(0i64, |a, c| a + c.is_some() as i64)
        .drive(|k, n| v.push((k, n)));
    v.sort_by_key(|&((p, vd), _)| (-db.post.creation_date.get(p).unwrap(), db.post.origid.get(p).unwrap(), vd.is_none(), vd));
    rows(v.iter().take(10).map(|&((p, vd), n)| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created"]);
        f.extend([ots(vd), V::I(n)]);
        row(f)
    }))
}

// SELECT
// u.DisplayName AS UserDisplayName,
// p.Title AS PostTitle,
// p.CreationDate AS PostCreationDate,
// v.VoteTypeId AS VoteType,
// COUNT(v.Id) AS VoteCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// u.DisplayName, p.Title, p.CreationDate, v.VoteTypeId
// ORDER BY
// PostCreationDate DESC
// LIMIT 10;
fn q15203(db: &'static So) -> String {
    let j: MatSet<(Id<Post>, Option<Id<Vote>>)> = owned(db).select(Ident::<Post>::new().and(votes_of(db).opt())).collect();
    let post_of = (&j).map(|(p, _)| p);
    let vote_of = (&j).flat_map(|(_, v)| v);
    let Post { title, creation_date, owner_user, .. } = &db.post;
    let key = (&post_of).select(owner_user.select(&db.user.display_name).and(title.opt()).and(creation_date)).and((&vote_of).select(&db.vote.vote_type_id).opt());
    let mut v = Vec::new();
    (&j).group_by(key)
        .select((&vote_of).opt())
        .fold(0i64, |a, x| a + x.is_some() as i64)
        .drive(|k, n| v.push((k, n)));
    v.sort_by(|a, b| b.0.0.1.cmp(&a.0.0.1));
    rows(v.iter().take(10).map(|&((((dn, t), cd), vt), n)| row(vec![V::S(dn), ostr(t), V::T(cd), oint(vt), V::I(n)])))
}

// SELECT
// P.Id AS PostId,
// P.Title,
// U.DisplayName AS OwnerDisplayName,
// P.CreationDate,
// P.ViewCount,
// P.Score,
// C.Text AS CommentText,
// COUNT(C.Id) AS CommentCount
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// WHERE
// P.PostTypeId = 1
// GROUP BY
// P.Id, U.DisplayName, P.Title, P.CreationDate, P.ViewCount, P.Score, C.Text
// ORDER BY
// P.CreationDate DESC;
fn q19936(db: &'static So) -> String {
    let j: MatSet<(Id<Post>, Option<Id<Comment>>)> =
        owned(db).with((&db.post.post_type_id).eq(1)).select(Ident::<Post>::new().and(comments_of(db).opt())).collect();
    let post_of = (&j).map(|(p, _)| p);
    let comment_of = (&j).flat_map(|(_, c)| c);
    let key = (&post_of).and((&comment_of).select(&db.comment.text).opt());
    let mut v = Vec::new();
    (&j).group_by(key)
        .select((&comment_of).opt())
        .fold(0i64, |a, x| a + x.is_some() as i64)
        .drive(|k, n| v.push((k, n)));
    rows(v.iter().map(|&((p, txt), n)| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "views", "score"]);
        f.extend([ostr(txt), V::I(n)]);
        row(f)
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("10941", q10941),
    ("13468", q13468),
    ("12046", q12046),
    ("12472", q12472),
    ("11286", q11286),
    ("14548", q14548),
    ("13880", q13880),
    ("12903", q12903),
    ("12196", q12196),
    ("12632", q12632),
    ("10113", q10113),
    ("11514", q11514),
    ("13587", q13587),
    ("11077", q11077),
    ("11782", q11782),
    ("12713", q12713),
    ("12088", q12088),
    ("11965", q11965),
    ("14523", q14523),
    ("10166", q10166),
    ("14367", q14367),
    ("14259", q14259),
    ("14730", q14730),
    ("18441", q18441),
    ("19727", q19727),
    ("15963", q15963),
    ("17752", q17752),
    ("18278", q18278),
    ("17363", q17363),
    ("16383", q16383),
    ("16219", q16219),
    ("19667", q19667),
    ("10379", q10379),
    ("14140", q14140),
    ("11275", q11275),
    ("17156", q17156),
    ("17257", q17257),
    ("17041", q17041),
    ("18453", q18453),
    ("19904", q19904),
    ("13619", q13619),
    ("12544", q12544),
    ("13993", q13993),
    ("14872", q14872),
    ("10136", q10136),
    ("17410", q17410),
    ("17127", q17127),
    ("19453", q19453),
    ("19755", q19755),
    ("16138", q16138),
    ("19881", q19881),
    ("15448", q15448),
    ("15915", q15915),
    ("15203", q15203),
    ("19936", q19936),
];
