use harness::prelude::*;

fn newest_first(db: &'static So, p: Id<Post>) -> i64 {
    -db.post.creation_date.get(p).unwrap()
}

fn best_first(db: &'static So, p: Id<Post>) -> i64 {
    -db.post.score.get(p).unwrap()
}

// JOIN Tags t ON t.ExcerptPostId = p.Id, for the questions with an owner.
fn excerpt_questions(db: &'static So, key: fn(&'static So, Id<Post>) -> i64, cols: &[&str]) -> String {
    let tag_of: HashIdx<Id<Post>, Str> = (&db.tag.excerpt_post).inv().select(&db.tag.tag_name).collect();
    let mut v = Vec::new();
    owned(db).with((&db.post.post_type_id).eq(1)).select(&tag_of).drive(|p, tn| v.push((key(db, p), p, tn)));
    v.sort_by_key(|&(k, _, _)| k);
    rows(v.iter().take(10).map(|&(_, p, tn)| {
        let mut f = post_fields(db, p, cols);
        f.push(V::S(tn));
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.Score,
// p.ViewCount,
// u.DisplayName AS OwnerDisplayName
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.CreationDate >= '2023-01-01'
// ORDER BY
// p.Score DESC
// LIMIT 10;
fn q19496(db: &'static So) -> String {
    render_posts(db, posts_where(db, false, false, "", PostWhere::CreatedGe(date(2023, 1, 1))), "score", 10, &["id", "title", "score", "views", "owner"])
}

// SELECT p.Title, p.CreationDate, u.DisplayName, t.TagName
// FROM Posts p
// JOIN Users u ON p.OwnerUserId = u.Id
// JOIN Tags t ON t.ExcerptPostId = p.Id
// WHERE p.PostTypeId = 1
// ORDER BY p.CreationDate DESC
// LIMIT 10;
fn q19012(db: &'static So) -> String {
    excerpt_questions(db, newest_first, &["title", "created", "owner"])
}

// SELECT
// p.Title,
// p.CreationDate,
// u.DisplayName AS Owner,
// t.TagName
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// JOIN
// Tags t ON t.ExcerptPostId = p.Id
// WHERE
// p.PostTypeId = 1
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q19373(db: &'static So) -> String {
    excerpt_questions(db, newest_first, &["title", "created", "owner"])
}

// SELECT
// p.Title,
// u.DisplayName AS Owner,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// t.TagName
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// JOIN
// Tags t ON t.ExcerptPostId = p.Id
// WHERE
// p.PostTypeId = 1
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q18870(db: &'static So) -> String {
    excerpt_questions(db, newest_first, &["title", "owner", "created", "score", "views"])
}

// SELECT
// p.Title,
// u.DisplayName AS OwnerDisplayName,
// p.ViewCount,
// p.CreationDate,
// p.Score,
// t.TagName
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// JOIN
// Tags t ON t.ExcerptPostId = p.Id
// WHERE
// p.PostTypeId = 1
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q19015(db: &'static So) -> String {
    excerpt_questions(db, newest_first, &["title", "owner", "views", "created", "score"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// u.DisplayName AS OwnerDisplayName,
// p.CreationDate,
// p.Score,
// t.TagName
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// JOIN
// Tags t ON t.ExcerptPostId = p.Id
// WHERE
// p.PostTypeId = 1
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q17109(db: &'static So) -> String {
    excerpt_questions(db, newest_first, &["id", "title", "owner", "created", "score"])
}

// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// U.DisplayName AS OwnerDisplayName,
// P.ViewCount,
// P.Score,
// T.TagName
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// JOIN
// Tags T ON T.ExcerptPostId = P.Id
// WHERE
// P.PostTypeId = 1
// ORDER BY
// P.CreationDate DESC
// LIMIT 10;
fn q17728(db: &'static So) -> String {
    excerpt_questions(db, newest_first, &["id", "title", "created", "owner", "views", "score"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// u.DisplayName AS Author,
// p.Score,
// p.ViewCount,
// t.TagName
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// JOIN
// Tags t ON t.ExcerptPostId = p.Id
// WHERE
// p.PostTypeId = 1
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q19384(db: &'static So) -> String {
    excerpt_questions(db, newest_first, &["id", "title", "created", "owner", "score", "views"])
}

// SELECT
// P.Id AS PostId,
// P.Title,
// U.DisplayName AS OwnerDisplayName,
// P.Score,
// P.ViewCount,
// P.CreationDate,
// T.TagName
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// JOIN
// Tags T ON T.ExcerptPostId = P.Id
// WHERE
// P.PostTypeId = 1
// ORDER BY
// P.Score DESC
// LIMIT 10;
fn q18178(db: &'static So) -> String {
    excerpt_questions(db, best_first, &["id", "title", "owner", "score", "views", "created"])
}

// JOIN Tags t ON p.Tags LIKE '%' || t.TagName || '%': one row per (question, tag).
// Both queries tie inside one question's tags at the LIMIT, so their rewrites
// order by p.Id, t.TagName after the date.
fn tagged_questions(db: &'static So, outer: bool, cols: &[&str]) -> String {
    let mentions = tag_mentions(db);
    let tags_by_post: HashIdx<Id<Post>, Id<Tag>> =
        (&mentions).map(|(p, _)| p).inv().select((&mentions).map(|(_, t)| t)).collect();
    let mut v = Vec::new();
    let qs = owned(db).with((&db.post.post_type_id).eq(1));
    if outer {
        qs.select((&tags_by_post).opt()).drive(|p, t| v.push((newest_first(db, p), p, t)));
    } else {
        qs.select(&tags_by_post).drive(|p, t| v.push((newest_first(db, p), p, Some(t))));
    }
    let name = |t: Option<Id<Tag>>| t.map(|t| db.tag.tag_name.get(t).unwrap());
    v.sort_by_key(|&(k, p, t)| (k, db.post.origid.get(p).unwrap(), name(t).is_none(), name(t)));
    rows(v.iter().take(10).map(|&(_, p, t)| {
        let mut f = post_fields(db, p, cols);
        f.push(ostr(name(t)));
        row(f)
    }))
}

// SELECT u.DisplayName, p.Title, p.CreationDate, t.TagName
// FROM Posts p
// JOIN Users u ON p.OwnerUserId = u.Id
// JOIN Tags t ON p.Tags LIKE '%' || t.TagName || '%'
// WHERE p.PostTypeId = 1
// ORDER BY p.CreationDate DESC
// LIMIT 10;
fn q16744(db: &'static So) -> String {
    tagged_questions(db, false, &["owner", "title", "created"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// u.DisplayName AS Owner,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// p.AnswerCount,
// p.CommentCount,
// t.TagName
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Tags t ON p.Tags LIKE CONCAT('%', t.TagName, '%')
// WHERE
// p.PostTypeId = 1
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q17299(db: &'static So) -> String {
    tagged_questions(db, true, &["id", "title", "owner", "created", "score", "views", "answers", "comments"])
}

// Comments on questions, newest comment first, with `author` the user the
// query joins: the comment's writer or the post's owner.
fn question_comments<A>(db: &'static So, author: A) -> String
where
    A: IntoQuery,
    A::Q: Probe<D = Id<Comment>, R = Str>,
{
    let Comment { post, creation_date, .. } = &db.comment;
    let mut v = Vec::new();
    db.comment
        .with(post.select(&db.post.post_type_id).eq(1))
        .select(post.and(creation_date).and(author))
        .drive(|c, ((p, cd), dn)| v.push((cd, c, p, dn)));
    v.sort_by(|a, b| b.0.cmp(&a.0));
    rows(v.iter().take(10).map(|&(cd, c, p, dn)| {
        let mut r = vec![V::S(dn)];
        r.extend(post_fields(db, p, &["title", "created"]));
        r.extend([V::S(db.comment.text.get(c).unwrap()), V::T(cd)]);
        row(r)
    }))
}

// SELECT
// u.DisplayName AS UserDisplayName,
// p.Title AS PostTitle,
// p.CreationDate AS PostCreationDate,
// c.Text AS CommentText,
// c.CreationDate AS CommentCreationDate
// FROM
// Posts p
// JOIN
// Comments c ON p.Id = c.PostId
// JOIN
// Users u ON c.UserId = u.Id
// WHERE
// p.PostTypeId = 1
// ORDER BY
// c.CreationDate DESC
// LIMIT 10;
fn q17488(db: &'static So) -> String {
    question_comments(db, (&db.comment.user).select(&db.user.display_name))
}

// SELECT
// U.DisplayName AS UserDisplayName,
// P.Title AS PostTitle,
// P.CreationDate AS PostCreationDate,
// C.Text AS CommentText,
// C.CreationDate AS CommentCreationDate
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// JOIN
// Comments C ON P.Id = C.PostId
// WHERE
// P.PostTypeId = 1
// ORDER BY
// C.CreationDate DESC
// LIMIT 10;
fn q16279(db: &'static So) -> String {
    question_comments(db, (&db.comment.post).select(&db.post.owner_user).select(&db.user.display_name))
}

// PostTypes grouped: COUNT, AVG(Score), AVG(ViewCount) in one fold, and
// COUNT(DISTINCT OwnerUserId) in a second.
fn type_counts(db: &'static So, views: bool) -> String {
    let Post { post_type, score, view_count, owner_user, .. } = &db.post;
    let name = post_type.select(&db.post_type.name);
    let main = db.post.group_by(&name).select(score.and(view_count.opt())).fold((0i64, 0i64, 0i64, 0i64), |(n, s, vn, vs), (x, w)| {
        (n + 1, s + x, vn + w.is_some() as i64, vs + w.unwrap_or(0))
    });
    let uniq = db.post.group_by(&name).select(owner_user).count_distinct();
    let mut v = Vec::new();
    main.and((&uniq).opt()).drive(|k, ((n, s, vn, vs), u)| v.push((n, k, s, vn, vs, u.unwrap_or(0))));
    v.sort_by(|a, b| b.0.cmp(&a.0));
    rows(v.iter().map(|&(n, k, s, vn, vs, u)| {
        if views {
            row(vec![V::S(k), V::I(n), avg(s, n), avg(vs, vn), V::I(u)])
        } else {
            row(vec![V::S(k), V::I(n), avg(s, n), V::I(u)])
        }
    }))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AverageScore,
// COUNT(DISTINCT p.OwnerUserId) AS UniqueUsers
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q12545(db: &'static So) -> String {
    type_counts(db, false)
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AverageScore,
// COUNT(DISTINCT p.OwnerUserId) AS TotalUniqueUsers
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q14403(db: &'static So) -> String {
    type_counts(db, false)
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AverageScore,
// AVG(p.ViewCount) AS AverageViewCount,
// COUNT(DISTINCT p.OwnerUserId) AS UniqueContributors
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q11186(db: &'static So) -> String {
    type_counts(db, true)
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AverageScore,
// COUNT(DISTINCT p.OwnerUserId) AS TotalUsers,
// AVG(p.ViewCount) AS AverageViewCount
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q12399(db: &'static So) -> String {
    let Post { post_type, score, view_count, owner_user, .. } = &db.post;
    let name = post_type.select(&db.post_type.name);
    let main = db.post.group_by(&name).select(score.and(view_count.opt())).fold((0i64, 0i64, 0i64, 0i64), |(n, s, vn, vs), (x, w)| {
        (n + 1, s + x, vn + w.is_some() as i64, vs + w.unwrap_or(0))
    });
    let uniq = db.post.group_by(&name).select(owner_user).count_distinct();
    let mut v = Vec::new();
    main.and((&uniq).opt()).drive(|k, ((n, s, vn, vs), u)| v.push((n, k, s, vn, vs, u.unwrap_or(0))));
    v.sort_by(|a, b| b.0.cmp(&a.0));
    rows(v.iter().map(|&(n, k, s, vn, vs, u)| row(vec![V::S(k), V::I(n), avg(s, n), V::I(u), avg(vs, vn)])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(EXTRACT(EPOCH FROM (p.LastActivityDate - p.CreationDate))) AS AvgTimeToActivity
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q14326(db: &'static So) -> String {
    let Post { post_type, creation_date, last_activity_date, .. } = &db.post;
    let mut v = Vec::new();
    db.post
        .group_by(post_type.select(&db.post_type.name))
        .select(last_activity_date.and(creation_date))
        .fold((0i64, 0i128), |(n, s), (la, cd)| (n + 1, s + (la - cd) as i128))
        .drive(|k, (n, s)| v.push((n, k, s)));
    v.sort_by(|a, b| b.0.cmp(&a.0));
    rows(v.iter().map(|&(n, k, s)| row(vec![V::S(k), V::I(n), V::F(s as f64 / 1e6 / n as f64)])))
}

// WITH PostStats AS (
// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS PostCount,
// AVG(p.Score) AS AverageScore
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// GROUP BY
// pt.Name
// )
// SELECT
// ps.PostType,
// ps.PostCount,
// ps.AverageScore
// FROM
// PostStats ps
// ORDER BY
// ps.PostCount DESC;
fn q13568(db: &'static So) -> String {
    rows(by_count(db).iter().map(|a| row(type_fields(a, &["name", "n", "score_avg"]))))
}

// SELECT
// p.Title,
// p.CreationDate,
// u.DisplayName AS OwnerDisplayName,
// COALESCE(p.AnswerCount, 0) AS AnswerCount,
// COALESCE(p.ViewCount, 0) AS ViewCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.PostTypeId = 1
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q16487(db: &'static So) -> String {
    let mut v = Vec::new();
    owned(db).with((&db.post.post_type_id).eq(1)).drive(|p, _| v.push((newest_first(db, p), p)));
    v.sort_by_key(|&(k, _)| k);
    rows(v.iter().take(10).map(|&(_, p)| {
        let mut f = post_fields(db, p, &["title", "created", "owner"]);
        f.push(V::I(db.post.answer_count.get(p).unwrap_or(0)));
        f.push(V::I(db.post.view_count.get(p).unwrap_or(0)));
        row(f)
    }))
}

// Questions LEFT JOIN Votes, newest question first: one row per vote.
fn question_votes(db: &'static So, cols: &[&str], vcols: &[&str]) -> String {
    let mut v = Vec::new();
    owned(db).with((&db.post.post_type_id).eq(1)).select(votes_of(db).opt()).drive(|p, vt| v.push((newest_first(db, p), p, vt)));
    v.sort_by_key(|&(k, _, _)| k);
    rows(v.iter().take(10).map(|&(_, p, vt)| {
        let mut f = post_fields(db, p, cols);
        for c in vcols {
            f.push(match *c {
                "type" => oint(vt.map(|x| db.vote.vote_type_id.get(x).unwrap())),
                "created" => ots(vt.map(|x| db.vote.creation_date.get(x).unwrap())),
                _ => unreachable!(),
            });
        }
        row(f)
    }))
}

// SELECT p.Title, p.CreationDate, u.DisplayName, v.VoteTypeId
// FROM Posts p
// JOIN Users u ON p.OwnerUserId = u.Id
// LEFT JOIN Votes v ON p.Id = v.PostId
// WHERE p.PostTypeId = 1
// ORDER BY p.CreationDate DESC
// LIMIT 10;
fn q19386(db: &'static So) -> String {
    question_votes(db, &["title", "created", "owner"], &["type"])
}

// SELECT
// P.Id AS PostId,
// P.Title,
// U.DisplayName AS OwnerName,
// V.VoteTypeId,
// V.CreationDate AS VoteDate
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// WHERE
// P.PostTypeId = 1
// ORDER BY
// P.CreationDate DESC
// LIMIT 10;
fn q17545(db: &'static So) -> String {
    question_votes(db, &["id", "title", "owner"], &["type", "created"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// u.DisplayName AS OwnerDisplayName,
// v.VoteTypeId
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.PostTypeId = 1
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q17766(db: &'static So) -> String {
    question_votes(db, &["id", "title", "created", "owner"], &["type"])
}

// SELECT p.Id, p.Title, u.DisplayName, p.CreationDate, v.VoteTypeId
// FROM Posts p
// JOIN Users u ON p.OwnerUserId = u.Id
// LEFT JOIN Votes v ON p.Id = v.PostId
// WHERE p.PostTypeId = 1
// ORDER BY p.CreationDate DESC
// LIMIT 10;
fn q19581(db: &'static So) -> String {
    question_votes(db, &["id", "title", "owner", "created"], &["type"])
}

// SELECT
// U.DisplayName AS UserName,
// P.Title AS PostTitle,
// P.Score AS PostScore,
// C.Text AS CommentText,
// C.CreationDate AS CommentDate
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// WHERE
// P.PostTypeId = 1
// ORDER BY
// P.Score DESC
// LIMIT 10;
fn q19893(db: &'static So) -> String {
    let mut v = Vec::new();
    owned(db).with((&db.post.post_type_id).eq(1)).select(comments_of(db).opt()).drive(|p, c| v.push((best_first(db, p), p, c)));
    v.sort_by_key(|&(k, _, _)| k);
    rows(v.iter().take(10).map(|&(_, p, c)| {
        let mut f = post_fields(db, p, &["owner", "title", "score"]);
        f.push(ostr(c.map(|c| db.comment.text.get(c).unwrap())));
        f.push(ots(c.map(|c| db.comment.creation_date.get(c).unwrap())));
        row(f)
    }))
}

// SELECT
// p.Title,
// p.CreationDate,
// u.DisplayName AS Owner,
// COUNT(ans.Id) AS AnswerCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Posts ans ON p.Id = ans.ParentId
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Title,
// p.CreationDate,
// u.DisplayName
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q19431(db: &'static So) -> String {
    tuple_rows(by_name_title_date(db, true, "a", PostWhere::All), "created", 10, &["title", "created", "owner", "#a"])
}

// SELECT
// p.Title,
// p.CreationDate,
// p.Score,
// u.DisplayName AS OwnerDisplayName,
// COUNT(co.Id) AS CommentCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments co ON p.Id = co.PostId
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Title,
// p.CreationDate,
// p.Score,
// u.DisplayName
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q17394(db: &'static So) -> String {
    tuple_rows(by_name_title_date_score(db, true, "c", PostWhere::All), "created", 10, &["title", "created", "score", "owner", "#c"])
}

// SELECT
// p.Title,
// u.DisplayName AS OwnerName,
// p.CreationDate,
// p.Score,
// COUNT(com.Id) AS CommentCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments com ON p.Id = com.PostId
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Title, u.DisplayName, p.CreationDate, p.Score
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q16669(db: &'static So) -> String {
    tuple_rows(by_name_title_date_score(db, true, "c", PostWhere::All), "created", 10, &["title", "owner", "created", "score", "#c"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// u.DisplayName AS OwnerName,
// p.CreationDate,
// p.Score,
// COUNT(co.Id) AS CommentCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments co ON p.Id = co.PostId
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, u.DisplayName, p.CreationDate, p.Score
// ORDER BY
// p.Score DESC
// LIMIT 10;
fn q18175(db: &'static So) -> String {
    post_rows(db, true, "c", "score", 10, &["id", "title", "owner", "created", "score", "#c"])
}

// SELECT
// P.Title,
// P.CreationDate,
// U.DisplayName,
// P.Score,
// P.ViewCount,
// COUNT(C.Id) AS CommentCount
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// WHERE
// P.PostTypeId = 1 /* Questions only */
// GROUP BY
// P.Title,
// P.CreationDate,
// U.DisplayName,
// P.Score,
// P.ViewCount
// ORDER BY
// P.CreationDate DESC
// LIMIT 10;
fn q19143(db: &'static So) -> String {
    tuple_rows(by_name_title_date_score_views(db, true, "c", PostWhere::All), "created", 10, &["title", "created", "owner", "score", "views", "#c"])
}

// SELECT
// p.Title,
// p.CreationDate,
// u.DisplayName AS OwnerDisplayName,
// COUNT(com.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments com ON p.Id = com.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Title, p.CreationDate, u.DisplayName
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q15903(db: &'static So) -> String {
    tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 10, &["title", "created", "owner", "#cx", "#vx"])
}

// SELECT
// p.Title,
// p.CreationDate,
// u.DisplayName AS OwnerDisplayName,
// COUNT(co.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments co ON p.Id = co.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Title, p.CreationDate, u.DisplayName
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q17563(db: &'static So) -> String {
    tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 10, &["title", "created", "owner", "#cx", "#vx"])
}

// SELECT
// p.Title,
// p.CreationDate,
// u.DisplayName AS OwnerDisplayName,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN vt.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN vt.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes vt ON p.Id = vt.PostId
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Title,
// p.CreationDate,
// u.DisplayName
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q16527(db: &'static So) -> String {
    tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 10, &["title", "created", "owner", "#cx", "#up", "#down"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// u.DisplayName AS OwnerDisplayName,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN vote.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN vote.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes vote ON p.Id = vote.PostId
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, p.CreationDate, u.DisplayName
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q17707(db: &'static So) -> String {
    post_rows(db, true, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#up", "#down"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// u.DisplayName AS Author,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN vote.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN vote.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes vote ON p.Id = vote.PostId
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, p.CreationDate, u.DisplayName
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q18454(db: &'static So) -> String {
    post_rows(db, true, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#up", "#down"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// u.DisplayName AS OwnerName,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN vote.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN vote.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes vote ON p.Id = vote.PostId
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, p.CreationDate, u.DisplayName
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q19680(db: &'static So) -> String {
    post_rows(db, true, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#up", "#down"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// u.DisplayName AS Author,
// COUNT(c.Id) AS CommentCount,
// COALESCE(SUM(CASE WHEN vote.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
// COALESCE(SUM(CASE WHEN vote.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes vote ON p.Id = vote.PostId
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, p.CreationDate, u.DisplayName
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q17779(db: &'static So) -> String {
    post_rows(db, true, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#up", "#down"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// u.DisplayName AS Owner,
// COUNT(c.Id) AS CommentCount,
// COALESCE(SUM(CASE WHEN vt.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
// COALESCE(SUM(CASE WHEN vt.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes vt ON p.Id = vt.PostId
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, p.CreationDate, u.DisplayName
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q19263(db: &'static So) -> String {
    post_rows(db, true, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#up", "#down"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// u.DisplayName AS OwnerDisplayName,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN vote.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN vote.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes vote ON p.Id = vote.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, u.DisplayName
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q19960(db: &'static So) -> String {
    post_rows(db, false, "cv", "created", 10, &["id", "title", "created", "owner", "#cx", "#up", "#down"])
}

pub static ENTRIES: &[harness::Entry] = &[
    ("19496", q19496),
    ("19012", q19012),
    ("19373", q19373),
    ("18870", q18870),
    ("19015", q19015),
    ("17109", q17109),
    ("17728", q17728),
    ("19384", q19384),
    ("18178", q18178),
    ("16744", q16744),
    ("17299", q17299),
    ("17488", q17488),
    ("16279", q16279),
    ("12545", q12545),
    ("14403", q14403),
    ("11186", q11186),
    ("12399", q12399),
    ("14326", q14326),
    ("13568", q13568),
    ("16487", q16487),
    ("19386", q19386),
    ("17545", q17545),
    ("17766", q17766),
    ("19581", q19581),
    ("19893", q19893),
    ("19431", q19431),
    ("17394", q17394),
    ("16669", q16669),
    ("18175", q18175),
    ("19143", q19143),
    ("15903", q15903),
    ("17563", q17563),
    ("16527", q16527),
    ("17707", q17707),
    ("18454", q18454),
    ("19680", q19680),
    ("17779", q17779),
    ("19263", q19263),
    ("19960", q19960),
];
