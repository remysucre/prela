use harness::prelude::*;
use std::cmp::Reverse;

fn questions(db: &'static So) -> impl Drive<D = Id<Post>, R = Id<Post>> {
    owned(db).with((&db.post.post_type_id).eq(1))
}

fn since(db: &'static So, d: i64) -> impl Drive<D = Id<Post>, R = Id<Post>> {
    db.post.with((&db.post.creation_date).ge(d))
}

fn owned_since(db: &'static So, d: i64) -> impl Drive<D = Id<Post>, R = Id<Post>> {
    owned(db).with((&db.post.creation_date).ge(d))
}

fn count<Q: Drive>(q: Q) -> i64 {
    q.fold_flat(0i64, |a, _| a + 1)
}

fn mean<Q: Drive<R = i64>>(q: Q) -> V {
    let (n, s) = q.fold_flat((0i64, 0i64), |(n, s), x| (n + 1, s + x));
    avg(s, n)
}

fn one(f: Fold<(), i64>) -> i64 {
    (&f).fold_flat(0i64, |a, x| a + x)
}

// (SELECT PostId, COUNT(*) FROM <child> GROUP BY PostId), keyed by the raw id.
fn comments_by_raw(db: &'static So) -> Fold<i64, i64> {
    db.comment.group_by(&db.comment.post_id).fold(0i64, |a, _| a + 1)
}

fn votes_by_raw(db: &'static So) -> Fold<i64, i64> {
    db.vote.group_by(&db.vote.post_id).fold(0i64, |a, _| a + 1)
}

fn links_by_raw(db: &'static So) -> Fold<i64, i64> {
    db.post_link.group_by(&db.post_link.post_id).fold(0i64, |a, _| a + 1)
}

// (SELECT ParentId, COUNT(*) FROM Posts WHERE PostTypeId = 2 GROUP BY ParentId).
fn answers_by_raw(db: &'static So) -> Fold<i64, i64> {
    db.post.with((&db.post.post_type_id).eq(2)).group_by(&db.post.parent_id).fold(0i64, |a, _| a + 1)
}

// The newest questions with nullable subquery counts beside them, `c0`, `c1`.
fn newest_with(db: &'static So, a: &Fold<i64, i64>, b: Option<&Fold<i64, i64>>, cols: &[&str]) -> String {
    let mut v = Vec::new();
    match b {
        Some(b) => questions(db).select((&db.post.origid).select(a.opt().and(b.opt()))).drive(|p, (x, y)| v.push((p, x, y))),
        None => questions(db).select((&db.post.origid).select(a.opt())).drive(|p, x| v.push((p, x, None))),
    }
    v.sort_by_key(|&(p, _, _)| newest(db, p));
    rows(v.iter().take(10).map(|&(p, x, y)| {
        row(cols
            .iter()
            .map(|c| match *c {
                "c0" => oint(x),
                "c1" => oint(y),
                _ => post_fields(db, p, &[c]).pop().unwrap(),
            })
            .collect())
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// u.DisplayName AS OwnerDisplayName,
// p.CreationDate,
// p.Score,
// c.CommentCount,
// p.ViewCount,
// p.LastActivityDate
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
fn q19370(db: &'static So) -> String {
    newest_with(db, &comments_by_raw(db), None, &["id", "title", "owner", "created", "score", "c0", "views", "activity"])
}

// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// U.DisplayName AS OwnerDisplayName,
// P.Score,
// C.CommentCount,
// V.VoteCount
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) C ON P.Id = C.PostId
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS VoteCount FROM Votes GROUP BY PostId) V ON P.Id = V.PostId
// WHERE
// P.PostTypeId = 1
// ORDER BY
// P.CreationDate DESC
// LIMIT 10;
fn q16609(db: &'static So) -> String {
    newest_with(db, &comments_by_raw(db), Some(&votes_by_raw(db)), &["id", "title", "created", "owner", "score", "c0", "c1"])
}

// SELECT
// P.Id AS PostId,
// P.Title,
// U.DisplayName AS OwnerDisplayName,
// P.CreationDate,
// P.Score,
// P.ViewCount,
// C.CommentCount,
// V.VoteCount
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) C ON P.Id = C.PostId
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS VoteCount FROM Votes GROUP BY PostId) V ON P.Id = V.PostId
// WHERE
// P.PostTypeId = 1
// ORDER BY
// P.CreationDate DESC
// LIMIT 10;
fn q17332(db: &'static So) -> String {
    newest_with(db, &comments_by_raw(db), Some(&votes_by_raw(db)), &["id", "title", "owner", "created", "score", "views", "c0", "c1"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// u.DisplayName AS OwnerDisplayName,
// p.ViewCount,
// c.CommentCount,
// t.Count AS TagCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS Count FROM PostLinks GROUP BY PostId) t ON p.Id = t.PostId
// WHERE
// p.PostTypeId = 1
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q19841(db: &'static So) -> String {
    newest_with(db, &comments_by_raw(db), Some(&links_by_raw(db)), &["id", "title", "created", "score", "owner", "views", "c0", "c1"])
}

// SELECT
// P.Title,
// P.CreationDate,
// U.DisplayName AS OwnerDisplayName,
// P.Score,
// P.ViewCount,
// C.CommentCount,
// A.AnswerCount
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) C ON P.Id = C.PostId
// LEFT JOIN
// (SELECT ParentId, COUNT(*) AS AnswerCount FROM Posts WHERE PostTypeId = 2 GROUP BY ParentId) A ON P.Id = A.ParentId
// WHERE
// P.PostTypeId = 1
// ORDER BY
// P.CreationDate DESC
// LIMIT 10;
fn q15016(db: &'static So) -> String {
    newest_with(db, &comments_by_raw(db), Some(&answers_by_raw(db)), &["title", "created", "owner", "score", "views", "c0", "c1"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// u.DisplayName AS Author,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// c.CommentCount,
// a.AnswerCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
// LEFT JOIN
// (SELECT ParentId, COUNT(*) AS AnswerCount FROM Posts WHERE PostTypeId = 2 GROUP BY ParentId) a ON p.Id = a.ParentId
// WHERE
// p.PostTypeId = 1
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q17078(db: &'static So) -> String {
    newest_with(db, &comments_by_raw(db), Some(&answers_by_raw(db)), &["id", "title", "owner", "created", "score", "views", "c0", "c1"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// u.DisplayName AS OwnerDisplayName,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// c.CommentCount,
// a.AnswerCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
// LEFT JOIN
// (SELECT ParentId, COUNT(*) AS AnswerCount FROM Posts WHERE PostTypeId = 2 GROUP BY ParentId) a ON p.Id = a.ParentId
// WHERE
// p.PostTypeId = 1
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q17475(db: &'static So) -> String {
    newest_with(db, &comments_by_raw(db), Some(&answers_by_raw(db)), &["id", "title", "owner", "created", "score", "views", "c0", "c1"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// u.DisplayName AS OwnerDisplayName,
// p.Score,
// p.ViewCount,
// c.CommentCount,
// a.AnswerCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
// LEFT JOIN
// (SELECT ParentId, COUNT(*) AS AnswerCount FROM Posts WHERE PostTypeId = 2 GROUP BY ParentId) a ON p.Id = a.ParentId
// WHERE
// p.PostTypeId = 1
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q17715(db: &'static So) -> String {
    newest_with(db, &comments_by_raw(db), Some(&answers_by_raw(db)), &["id", "title", "created", "owner", "score", "views", "c0", "c1"])
}

// SELECT
// P.Id AS PostId,
// P.Title,
// P.Body,
// U.DisplayName AS OwnerDisplayName,
// P.CreationDate,
// P.Score,
// P.ViewCount,
// C.CommentCount,
// A.AnswerCount
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) C ON P.Id = C.PostId
// LEFT JOIN
// (SELECT ParentId, COUNT(*) AS AnswerCount FROM Posts WHERE PostTypeId = 2 GROUP BY ParentId) A ON P.Id = A.ParentId
// WHERE
// P.PostTypeId = 1
// ORDER BY
// P.CreationDate DESC
// LIMIT 10;
fn q17849(db: &'static So) -> String {
    newest_with(db, &comments_by_raw(db), Some(&answers_by_raw(db)), &["id", "title", "body", "owner", "created", "score", "views", "c0", "c1"])
}

// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.Score,
// P.ViewCount,
// C.CommentCount,
// V.VoteCount,
// U.DisplayName AS OwnerDisplayName,
// U.Reputation AS OwnerReputation
// FROM
// Posts P
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount
// FROM Comments
// GROUP BY PostId) C ON P.Id = C.PostId
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS VoteCount
// FROM Votes
// GROUP BY PostId) V ON P.Id = V.PostId
// LEFT JOIN
// Users U ON P.OwnerUserId = U.Id
// WHERE
// P.CreationDate >= '2023-01-01 00:00:00'
// ORDER BY
// P.CreationDate DESC
// LIMIT 100;
fn q11010(db: &'static So) -> String {
    let (c, x) = (comments_by_raw(db), votes_by_raw(db));
    let mut v = Vec::new();
    since(db, date(2023, 1, 1)).select((&db.post.origid).select((&c).opt().and((&x).opt()))).drive(|p, a| v.push((newest(db, p), p, a)));
    v.sort_by_key(|x| x.0);
    rows(v.iter().take(100).map(|&(_, p, (c, x))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([oint(c), oint(x)]);
        f.extend(post_fields(db, p, &["owner", "rep"]));
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// u.DisplayName AS Author,
// p.Score,
// c.CommentCount,
// t.TagName
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
// LEFT JOIN
// PostLinks pl ON p.Id = pl.PostId
// LEFT JOIN
// Tags t ON pl.RelatedPostId = t.Id
// WHERE
// p.PostTypeId = 1
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q15601(db: &'static So) -> String {
    let cc = comments_by_raw(db);
    let tag_by_raw: HashIdx<i64, Str> = (&db.tag.origid).inv().select(&db.tag.tag_name).collect();
    let mut v = Vec::new();
    questions(db)
        .select((&db.post.origid).select((&cc).opt()).and(links_of(db).select((&db.post_link.related_post_id).select((&tag_by_raw).opt())).opt()))
        .drive(|p, (c, t)| v.push((newest(db, p), p, c, t.flatten())));
    v.sort_by_key(|x| x.0);
    rows(v.iter().take(10).map(|&(_, p, c, t)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner", "score"]);
        f.extend([oint(c), ostr(t)]);
        row(f)
    }))
}

// SELECT
// U.DisplayName AS UserDisplayName,
// Post.Title AS PostTitle,
// Post.CreationDate AS PostCreationDate,
// Post.Body AS PostBody,
// COUNT(C.CommentId) AS CommentCount
// FROM
// Posts Post
// JOIN
// Users U ON Post.OwnerUserId = U.Id
// LEFT JOIN
// (SELECT
// Id as CommentId,
// PostId
// FROM
// Comments) C ON Post.Id = C.PostId
// WHERE
// Post.PostTypeId = 1
// GROUP BY
// U.DisplayName, Post.Title, Post.CreationDate, Post.Body
// ORDER BY
// Post.CreationDate DESC
// LIMIT 10;
fn q17648(db: &'static So) -> String {
    tuple_rows(by_name_title_date_body(db, true, "c", PostWhere::All), "created", 10, &["owner", "title", "created", "body", "#c"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// a.AcceptedAnswerId,
// u.DisplayName AS OwnerDisplayName,
// (SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id) AS VoteCount,
// (SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id) AS CommentCount,
// (SELECT COUNT(*) FROM PostLinks pl WHERE pl.PostId = p.Id) AS RelatedPostCount
// FROM
// Posts p
// LEFT JOIN
// Posts a ON p.AcceptedAnswerId = a.Id
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.PostTypeId = 1
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q12541(db: &'static So) -> String {
    let lpp = (&db.post_link.post).inv().dense_fold_outer(db.post.id.n, 0i64, |a, _| a + 1);
    let mut v = Vec::new();
    db.post
        .with((&db.post.post_type_id).eq(1))
        .select((&db.post.accepted_answer).select((&db.post.accepted_answer_id).opt()).opt().and(votes_per_post(db)).and(comments_per_post(db)).and(&lpp))
        .drive(|p, a| v.push((newest(db, p), p, a)));
    v.sort_by_key(|x| x.0);
    rows(v.iter().take(100).map(|&(_, p, (((a, x), c), l))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.push(oint(a.flatten()));
        f.extend(post_fields(db, p, &["owner"]));
        f.extend([V::I(x), V::I(c), V::I(l)]);
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// u.DisplayName AS OwnerDisplayName,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// (SELECT COUNT(*) FROM Posts p2 WHERE p2.ParentId = p.Id) AS AnswerCount
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
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q12936(db: &'static So) -> String {
    let apc = answers_per_post(db);
    let v = stats_fold(db, questions(db), Ident::<Post>::new(), "cv", &[]);
    let mut out = Vec::new();
    v.and(&apc).drive(|p, (s, a)| out.push((newest(db, p), p, s, a)));
    out.sort_by_key(|x| x.0);
    rows(out.iter().take(100).map(|&(_, p, ref s, a)| {
        let mut f = stat_fields(db, p, s, &["id", "title", "created", "views", "score", "owner", "#cx", "#vx"]);
        f.push(V::I(a));
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// u.DisplayName AS OwnerDisplayName,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// (SELECT COUNT(*) FROM PostHistory ph WHERE ph.PostId = p.Id) AS RevisionCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= DATE '2023-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q12410(db: &'static So) -> String {
    let hpp = history_per_post(db);
    let v = stats_fold(db, owned_since(db, date(2023, 1, 1)), Ident::<Post>::new(), "cv", &[]);
    let mut out = Vec::new();
    v.and(&hpp).drive(|p, (s, h)| out.push((newest(db, p), p, s, h)));
    out.sort_by_key(|x| x.0);
    rows(out.iter().take(100).map(|&(_, p, ref s, h)| {
        let mut f = stat_fields(db, p, s, &["id", "title", "created", "views", "score", "owner", "#cx", "#vx"]);
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
// U.DisplayName AS OwnerDisplayName,
// U.Reputation AS OwnerReputation,
// (SELECT COUNT(*) FROM Comments C WHERE C.PostId = P.Id) AS CommentCount,
// (SELECT COUNT(*) FROM Votes V WHERE V.PostId = P.Id) AS VoteCount,
// P.LastActivityDate,
// PH.CreationDate AS LastEditDate,
// PH.UserDisplayName AS LastEditor
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// PostHistory PH ON P.Id = PH.PostId
// WHERE
// P.PostTypeId = 1
// ORDER BY
// P.CreationDate DESC
// LIMIT 100;
fn q10895(db: &'static So) -> String {
    let mut v = Vec::new();
    questions(db)
        .select(comments_per_post(db).and(votes_per_post(db)).and(history_of(db).opt()))
        .drive(|p, a| v.push((p, a)));
    let hid = |h: Option<Id<PostHistory>>| h.map(|h| db.post_history.origid.get(h).unwrap());
    v.sort_by_key(|&(p, ((_, _), h))| (newest(db, p), db.post.origid.get(p).unwrap(), hid(h).is_none(), hid(h)));
    rows(v.iter().take(100).map(|&(p, ((c, x), h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner", "rep"]);
        f.extend([V::I(c), V::I(x)]);
        f.extend(post_fields(db, p, &["activity"]));
        f.extend([ots(h.map(|h| db.post_history.creation_date.get(h).unwrap())), ostr(h.and_then(|h| db.post_history.user_display_name.get(h)))]);
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// COUNT(CASE WHEN c.Id IS NOT NULL THEN 1 END) AS CommentCount,
// COUNT(DISTINCT v.UserId) AS VoteCount,
// u.DisplayName AS OwnerDisplayName,
// COUNT(DISTINCT b.Id) AS BadgeCount
// FROM Posts p
// LEFT JOIN Comments c ON p.Id = c.PostId
// LEFT JOIN Votes v ON p.Id = v.PostId
// LEFT JOIN Users u ON p.OwnerUserId = u.Id
// LEFT JOIN Badges b ON u.Id = b.UserId
// WHERE p.CreationDate >= '2023-01-01'
// GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName
// ORDER BY p.CreationDate DESC;
fn q14907(db: &'static So) -> String {
    let voters = per_post_distinct(db, votes_of(db).select(&db.vote.user_id));
    let b = per_post_distinct(db, (&db.post.owner_user).select(badges_of(db)));
    stats_rows(db, stats_with(db, since(db, date(2023, 1, 1)), "cvb", &[], &[&voters, &b]), |_, _| 0, 0, &["id", "title", "created", "views", "score", "#cx", "#d0", "owner", "#d1"])
}

// SELECT
// P.Id AS PostId,
// P.Title,
// U.DisplayName AS OwnerDisplayName,
// P.CreationDate,
// P.Score,
// P.ViewCount,
// COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
// COUNT(CASE WHEN V.Id IS NOT NULL THEN 1 END) AS VoteCount,
// COUNT(DISTINCT B.Id) AS BadgeCount
// FROM
// Posts P
// LEFT JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// GROUP BY
// P.Id, P.Title, U.DisplayName, P.CreationDate, P.Score, P.ViewCount
// ORDER BY
// P.CreationDate DESC
// LIMIT 100;
fn q11255(db: &'static So) -> String {
    let b = per_post_distinct(db, (&db.post.owner_user).select(badges_of(db)));
    stats_rows(db, stats_with(db, db.post.iq(), "cvb", &[], &[&b]), |p, _| newest(db, p), 100, &["id", "title", "owner", "created", "score", "views", "#cx", "#vx", "#d0"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// SUM(CASE WHEN b.Id IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Badges b ON p.OwnerUserId = b.UserId
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'
// GROUP BY
// p.Id, p.Title, p.CreationDate
// ORDER BY
// p.CreationDate DESC;
fn q12665(db: &'static So) -> String {
    let bidx: HashIdx<i64, Id<Badge>> = (&db.badge.user_id).inv().collect();
    let mut v = Vec::new();
    since(db, ts(2024, 9, 1, 12, 34, 56))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and((&db.post.owner_user_id).select(&bidx).opt()))
        .fold([0i64; 4], |a, ((c, t), b)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64, a[3] + b.is_some() as i64])
        .drive(|p, a| v.push((p, a)));
    rows(v.iter().map(|&(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend(a.iter().map(|&x| V::I(x)));
        row(f)
    }))
}

// Users LEFT JOIN Posts [...] per user.
fn users(db: &'static So, w: UserWhere, joins: &str, d: &[&Fold<Id<User>, i64>]) -> Vec<(Id<User>, UStats, [i64; 4])> {
    users_stats_with(db, w, joins, any_post, &[], d)
}

fn ud<R>(db: &'static So, w: UserWhere, r: R) -> Fold<Id<User>, i64>
where
    R: IntoQuery,
    R::Q: Probe<D = Id<User>>,
    ROf<R>: Ord,
{
    user_distinct(db, Ident::<User>::new(), w, r)
}

fn rep_desc(db: &'static So, u: Id<User>) -> Reverse<i64> {
    Reverse(db.user.reputation.get(u).unwrap())
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.Score IS NOT NULL THEN p.Score ELSE 0 END) AS TotalScore,
// COUNT(DISTINCT b.Id) AS BadgeCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// WHERE
// u.Reputation > 0
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
// ORDER BY
// PostCount DESC, TotalScore DESC
// LIMIT 100;
fn q10377(db: &'static So) -> String {
    let w = UserWhere::RepGt(0);
    let b = ud(db, w, badges_of(db));
    users_rows(db, users(db, w, "b", &[&b]), |_, s, _| (Reverse(s.n), Reverse(s.score_sum)), 100, &["uid", "name", "rep", "#n", "score_sum0", "#d0"])
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.Score IS NOT NULL THEN p.Score ELSE 0 END) AS TotalPostScore,
// AVG(CASE WHEN p.Score IS NOT NULL THEN p.Score ELSE NULL END) AS AvgPostScore,
// COUNT(DISTINCT b.Id) AS TotalBadges
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
// ORDER BY
// u.Reputation DESC, TotalPostScore DESC;
fn q14517(db: &'static So) -> String {
    let b = ud(db, UserWhere::All, badges_of(db));
    users_rows(db, users(db, UserWhere::All, "b", &[&b]), |_, _, _| 0, 0, &["uid", "name", "rep", "#n", "score_sum0", "score_avg", "#d0"])
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(COALESCE(p.ViewCount, 0)) AS TotalViews,
// AVG(p.Score) AS AverageScore,
// SUM(CASE WHEN b.Id IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
// ORDER BY
// PostCount DESC, Reputation DESC;
fn q14645(db: &'static So) -> String {
    users_rows(db, users(db, UserWhere::All, "b", &[]), |_, _, _| 0, 0, &["uid", "name", "rep", "#n", "#q", "#a", "views_sum0", "score_avg", "#bx"])
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(p.Id) AS PostCount,
// COALESCE(SUM(p.Score), 0) AS TotalScore
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id,
// u.DisplayName,
// u.Reputation
// )
// SELECT
// ups.UserId,
// ups.DisplayName,
// ups.Reputation,
// ups.PostCount,
// ups.TotalScore
// FROM
// UserPostStats ups
// ORDER BY
// ups.Reputation DESC
// LIMIT 10;
fn q12492(db: &'static So) -> String {
    users_rows(db, users(db, UserWhere::All, "", &[]), |u, _, _| rep_desc(db, u), 10, &["uid", "name", "rep", "#n", "score_sum0"])
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS PostCount,
// AVG(COALESCE(p.Score, 0)) AS AverageScore
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName
// )
// SELECT
// ups.UserId,
// ups.DisplayName,
// ups.PostCount,
// ups.AverageScore,
// u.Reputation
// FROM
// UserPostStats ups
// JOIN
// Users u ON ups.UserId = u.Id
// ORDER BY
// u.Reputation DESC
// LIMIT 10;
fn q11256(db: &'static So) -> String {
    let v = users(db, UserWhere::All, "", &[]);
    let mut v = v;
    v.sort_by_key(|&(u, _, _)| rep_desc(db, u));
    rows(v.iter().take(10).map(|(u, s, _)| {
        row(vec![user_col(db, *u, "uid"), user_col(db, *u, "name"), V::I(s.n), V::F(s.score_sum as f64 / s.rows as f64), user_col(db, *u, "rep")])
    }))
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS TotalQuestions,
// COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS TotalAnswers,
// SUM(COALESCE(p.Score, 0)) AS TotalScore,
// SUM(COALESCE(p.ViewCount, 0)) AS TotalViews,
// AVG(COALESCE(p.Score, 0)) AS AvgScorePerPost,
// AVG(COALESCE(p.ViewCount, 0)) AS AvgViewsPerPost
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName
// ORDER BY
// TotalPosts DESC;
fn q10415(db: &'static So) -> String {
    let Post { post_type_id, .. } = &db.post;
    let q = ud(db, UserWhere::All, posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1))));
    let a = ud(db, UserWhere::All, posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(2))));
    let v = users(db, UserWhere::All, "", &[&q, &a]);
    rows(v.iter().map(|(u, s, d)| {
        row(vec![
            user_col(db, *u, "uid"),
            user_col(db, *u, "name"),
            V::I(s.n),
            V::I(d[0]),
            V::I(d[1]),
            V::I(s.score_sum),
            V::I(s.views_sum),
            V::F(s.score_sum as f64 / s.rows as f64),
            V::F(s.views_sum as f64 / s.rows as f64),
        ])
    }))
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS TotalQuestions,
// COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS TotalAnswers,
// SUM(CASE WHEN v.CreationDate IS NOT NULL THEN 1 ELSE 0 END) AS TotalVotes,
// AVG(u.Reputation) AS AverageReputation,
// MAX(u.CreationDate) AS MostRecentAccountCreation
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// u.Id, u.DisplayName
// ORDER BY
// TotalPosts DESC
// LIMIT 100;
fn q14782(db: &'static So) -> String {
    let Post { post_type_id, .. } = &db.post;
    let w = UserWhere::All;
    let p = ud(db, w, posts_of(db));
    let q = ud(db, w, posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1))));
    let a = ud(db, w, posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(2))));
    let v = users(db, w, "v", &[&p, &q, &a]);
    users_rows(db, v, |_, _, d| Reverse(d[0]), 100, &["uid", "name", "#d0", "#d1", "#d2", "#vx", "rep_avg", "ucreated"])
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(CASE WHEN p.Score IS NOT NULL THEN p.Score ELSE 0 END) AS TotalScore,
// COUNT(DISTINCT v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
// AVG(u.Reputation) AS AvgUserReputation
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// u.Id, u.DisplayName
// ORDER BY
// PostCount DESC, TotalScore DESC
// LIMIT 100;
fn q13155(db: &'static So) -> String {
    let w = UserWhere::All;
    let p = ud(db, w, posts_of(db));
    let x = ud(db, w, posts_of(db).select(votes_of(db)));
    let v = users(db, w, "v", &[&p, &x]);
    users_rows(db, v, |_, s, d| (Reverse(d[0]), Reverse(s.score_sum)), 100, &["uid", "name", "#d0", "score_sum0", "#d1", "#up", "#down", "rep_avg"])
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// COUNT(DISTINCT c.Id) AS TotalComments,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN p.ParentId IS NOT NULL THEN 1 ELSE 0 END) AS TotalChildPosts,
// AVG(p.Score) AS AveragePostScore,
// AVG(u.Reputation) AS AverageUserReputation
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
// LIMIT 100;
fn q11197(db: &'static So) -> String {
    let w = UserWhere::All;
    let p = ud(db, w, posts_of(db));
    let c = ud(db, w, posts_of(db).select(comments_of(db)));
    let Post { post_type_id, parent_id, score, .. } = &db.post;
    let f = user_base(db, w)
        .group_by(Ident::<User>::new())
        .select((&db.user.reputation).and(posts_of(db).select(post_type_id.and(parent_id.opt()).and(score).and(comments_of(db).opt())).opt()))
        .fold([0i64; 7], |a, (r, p)| {
            let mut a = a;
            a[5] += r;
            a[6] += 1;
            if let Some((((t, par), s), _)) = p {
                a[0] += 1;
                a[1] += (t == 1) as i64;
                a[2] += (t == 2) as i64;
                a[3] += par.is_some() as i64;
                a[4] += s;
            }
            a
        });
    let mut v = Vec::new();
    f.and((&p).opt()).and((&c).opt()).drive(|u, ((a, p), c)| v.push((u, a, p.unwrap_or(0), c.unwrap_or(0))));
    v.sort_by_key(|x| Reverse(x.2));
    rows(v.iter().take(100).map(|&(u, a, p, c)| {
        row(vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(p), V::I(c), V::I(a[1]), V::I(a[2]), V::I(a[3]), avg(a[4], a[0]), avg(a[5], a[6])])
    }))
}

// SELECT
// U.Reputation AS UserReputation,
// COUNT(P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN V.CreationDate IS NOT NULL THEN 1 ELSE 0 END) AS TotalVotes,
// AVG(P.Score) AS AveragePostScore,
// MAX(P.CreationDate) AS MostRecentPost,
// COUNT(DISTINCT V.UserId) AS UniqueVoters
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// U.Id, U.Reputation
// ORDER BY
// UserReputation DESC;
fn q13386(db: &'static So) -> String {
    let w = UserWhere::All;
    let voters = ud(db, w, posts_of(db).select(votes_of(db).select(&db.vote.user_id)));
    let v = users(db, w, "v", &[&voters]);
    users_rows(db, v, |_, _, _| 0, 0, &["rep", "#n", "#q", "#a", "#vx", "score_avg", "created_max", "#d0"])
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// COUNT(DISTINCT p.Id) AS PostCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
// SUM(v.BountyAmount) AS TotalBounty
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Votes v ON u.Id = v.UserId
// GROUP BY
// u.Id, u.Reputation
// )
// SELECT
// us.UserId,
// us.Reputation,
// us.PostCount,
// us.VoteCount,
// us.TotalBounty
// FROM
// UserStats us
// ORDER BY
// us.Reputation DESC;
fn q11374(db: &'static So) -> String {
    let w = UserWhere::All;
    let p = ud(db, w, posts_of(db));
    let x = ud(db, w, votes_by(db));
    let f = user_base(db, w).group_by(Ident::<User>::new()).select(posts_of(db).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt())).fold((0i64, 0i64), |(n, s), (_, b)| {
        let b = b.flatten();
        (n + b.is_some() as i64, s + b.unwrap_or(0))
    });
    let mut v = Vec::new();
    f.and((&p).opt()).and((&x).opt()).drive(|u, ((a, p), x)| v.push((u, a, p.unwrap_or(0), x.unwrap_or(0))));
    rows(v.iter().map(|&(u, (n, s), p, x)| row(vec![user_col(db, u, "uid"), user_col(db, u, "rep"), V::I(p), V::I(x), nullable(s, n)])))
}

// WITH UserPostStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(P.Id) AS TotalPosts,
// COALESCE(AVG(P.Score), 0) AS AverageScore,
// COALESCE(SUM(V.Id), 0) AS TotalVotes
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// U.Id, U.DisplayName
// )
// SELECT
// UserId,
// DisplayName,
// TotalPosts,
// AverageScore,
// TotalVotes
// FROM
// UserPostStats
// ORDER BY
// TotalPosts DESC, AverageScore DESC;
fn q11648(db: &'static So) -> String {
    let mut v = Vec::new();
    user_base(db, UserWhere::All)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.score).and(votes_of(db).select(&db.vote.origid).opt())).opt())
        .fold([0i64; 3], |a, p| match p {
            Some((s, x)) => [a[0] + 1, a[1] + s, a[2] + x.unwrap_or(0)],
            None => a,
        })
        .drive(|u, a| v.push((u, a)));
    rows(v.iter().map(|&(u, a)| {
        row(vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(a[0]), V::F(if a[0] == 0 { 0.0 } else { a[1] as f64 / a[0] as f64 }), V::I(a[2])])
    }))
}

// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(P.Id) AS NumberOfPosts,
// AVG(U.Reputation) AS AverageReputation,
// SUM(V.TotalVotes) AS TotalVotes
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// (SELECT
// PostId,
// COUNT(*) AS TotalVotes
// FROM
// Votes
// GROUP BY
// PostId) V ON P.Id = V.PostId
// GROUP BY
// U.Id, U.DisplayName
// ORDER BY
// NumberOfPosts DESC;
fn q13557(db: &'static So) -> String {
    let vc = votes_by_raw(db);
    let mut v = Vec::new();
    user_base(db, UserWhere::All)
        .group_by(Ident::<User>::new())
        .select((&db.user.reputation).and(posts_of(db).select((&db.post.origid).select((&vc).opt())).opt()))
        .fold([0i64; 5], |a, (r, p)| {
            let x = p.flatten();
            [a[0] + p.is_some() as i64, a[1] + r, a[2] + 1, a[3] + x.is_some() as i64, a[4] + x.unwrap_or(0)]
        })
        .drive(|u, a| v.push((u, a)));
    rows(v.iter().map(|&(u, a)| row(vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(a[0]), avg(a[1], a[2]), nullable(a[4], a[3])])))
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS PostCount,
// AVG(p.Score) AS AverageScore,
// AVG(p.ViewCount) AS AverageViewCount,
// MAX(v.VoteCount) AS MaxVoteCount,
// MIN(v.VoteCount) AS MinVoteCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN (
// SELECT
// PostId,
// COUNT(Id) AS VoteCount
// FROM
// Votes
// GROUP BY
// PostId
// ) v ON p.Id = v.PostId
// GROUP BY
// u.Id, u.DisplayName
// ORDER BY
// PostCount DESC;
fn q11353(db: &'static So) -> String {
    let vc = votes_by_raw(db);
    let Post { score, view_count, origid, .. } = &db.post;
    let mut v = Vec::new();
    user_base(db, UserWhere::All)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score.and(view_count.opt()).and(origid.select((&vc).opt()))).opt())
        .fold([0, 0, 0, 0, 0, i64::MIN, i64::MAX], |a: [i64; 7], p| match p {
            Some(((s, w), x)) => [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + x.is_some() as i64, x.map_or(a[5], |x| a[5].max(x)), x.map_or(a[6], |x| a[6].min(x))],
            None => a,
        })
        .drive(|u, a| v.push((u, a)));
    rows(v.iter().map(|&(u, a)| row(vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(a[0]), avg(a[1], a[0]), avg(a[3], a[2]), omax(a[5], a[4]), omax(a[6], a[4])])))
}

// GROUP BY p.OwnerUserId, the raw column, joined to Users on it.
// WITH PostStats AS (
// SELECT
// COUNT(*) AS TotalPosts,
// AVG(Score) AS AverageScore,
// OwnerUserId
// FROM
// Posts
// GROUP BY
// OwnerUserId
// ),
// UserReputation AS (
// SELECT
// U.Id AS UserId,
// U.Reputation
// FROM
// Users U
// JOIN
// PostStats PS ON U.Id = PS.OwnerUserId
// )
// SELECT
// PS.TotalPosts,
// PS.AverageScore,
// UR.UserId,
// UR.Reputation
// FROM
// PostStats PS
// JOIN
// UserReputation UR ON PS.OwnerUserId = UR.UserId
// ORDER BY
// UR.Reputation DESC
// LIMIT 10;
fn q14112(db: &'static So) -> String {
    let stats = db.post.group_by(&db.post.owner_user_id).select(&db.post.score).fold((0i64, 0i64), |(n, s), x| (n + 1, s + x));
    let mut v = Vec::new();
    db.user.select((&db.user.origid).select(&stats)).drive(|u, a| v.push((u, a)));
    v.sort_by_key(|&(u, _)| rep_desc(db, u));
    rows(v.iter().take(10).map(|&(u, (n, s))| row(vec![V::I(n), avg(s, n), user_col(db, u, "uid"), user_col(db, u, "rep")])))
}

// Posts grouped by type with a fold over what each is joined to.
fn by_key<Q, K, R, S, F>(base: Q, key: K, joined: R, init: S, f: F) -> Fold<ROf<K>, S>
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

fn name(db: &'static So) -> Compose<&'static Col<Post, Id<PostType>>, &'static Col<PostType, Str>> {
    (&db.post.post_type).select(&db.post_type.name)
}

fn with_d<K: Copy + Eq + std::hash::Hash, const N: usize>(f: &Fold<K, [i64; N]>, d: &[&Fold<K, i64>]) -> Vec<(K, [i64; N], [i64; 2])> {
    let mut v = Vec::new();
    let o = |x: Option<i64>| x.unwrap_or(0);
    match d {
        [] => f.drive(|k, a| v.push((k, a, [0; 2]))),
        [x] => f.and(x.opt()).drive(|k, (a, x)| v.push((k, a, [o(x), 0]))),
        [x, y] => f.and(x.opt()).and(y.opt()).drive(|k, ((a, x), y)| v.push((k, a, [o(x), o(y)]))),
        _ => panic!(),
    }
    v.sort_by_key(|x| Reverse(x.1[0]));
    v
}

fn dby<Q, K, R>(base: Q, key: K, r: R) -> Fold<ROf<K>, i64>
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

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AverageScore,
// SUM(CASE WHEN p.OwnerUserId IS NOT NULL THEN 1 ELSE 0 END) AS TotalOwnedPosts,
// COUNT(DISTINCT p.OwnerUserId) AS UniqueUsers,
// AVG(u.Reputation) AS AverageUserReputation
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
fn q11264(db: &'static So) -> String {
    let Post { score, owner_user_id, owner_user, .. } = &db.post;
    let f = by_key(db.post.iq(), name(db), score.and(owner_user_id.opt()).and(owner_user.select(&db.user.reputation).opt()), [0i64; 5], |a, ((s, o), r)| {
        [a[0] + 1, a[1] + s, a[2] + o.is_some() as i64, a[3] + r.is_some() as i64, a[4] + r.unwrap_or(0)]
    });
    let u = dby(db.post.iq(), name(db), owner_user_id);
    rows(with_d(&f, &[&u]).iter().map(|&(k, a, d)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), V::I(a[2]), V::I(d[0]), avg(a[4], a[3])])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.ViewCount) AS AvgViewCount,
// SUM(vote_count) AS TotalVotes
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
// JOIN
// Users u ON p.OwnerUserId = u.Id
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q10252(db: &'static So) -> String {
    let vc = votes_by_raw(db);
    let Post { view_count, origid, .. } = &db.post;
    let f = by_key(owned(db), name(db), view_count.opt().and(origid.select((&vc).opt())), [0i64; 5], |a, (w, x)| {
        [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + x.is_some() as i64, a[4] + x.unwrap_or(0)]
    });
    rows(with_d(&f, &[]).iter().map(|&(k, a, _)| row(vec![V::S(k), V::I(a[0]), avg(a[2], a[1]), nullable(a[4], a[3])])))
}

// WITH PostStats AS (
// SELECT
// pt.Name AS PostTypeName,
// COUNT(p.Id) AS PostCount,
// AVG(p.ViewCount) AS AvgViewCount,
// AVG(p.Score) AS AvgScore
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// GROUP BY
// pt.Name
// )
// SELECT
// ps.PostTypeName,
// ps.PostCount,
// ps.AvgViewCount,
// ps.AvgScore,
// (SELECT COUNT(*) FROM Posts) AS TotalPosts
// FROM
// PostStats ps
// ORDER BY
// ps.PostCount DESC;
fn q14460(db: &'static So) -> String {
    let total = count(db.post.iq());
    rows(by_count(db).iter().map(|a| {
        let mut f = type_fields(a, &["name", "n", "views_avg", "score_avg"]);
        f.push(V::I(total));
        row(f)
    }))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers,
// SUM(p.Score) AS TotalScore,
// AVG(p.ViewCount) AS AverageViews,
// COUNT(DISTINCT u.Id) AS UniqueUsers,
// COUNT(c.Id) AS TotalComments
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q14561(db: &'static So) -> String {
    let Post { accepted_answer_id, score, view_count, owner_user, .. } = &db.post;
    let f = by_key(db.post.iq(), name(db), accepted_answer_id.opt().and(score).and(view_count.opt()).and(comments_of(db).opt()), [0i64; 6], |a, (((ac, s), w), c)| {
        [a[0] + 1, a[1] + ac.is_some() as i64, a[2] + s, a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0), a[5] + c.is_some() as i64]
    });
    let u = dby(db.post.iq(), name(db), owner_user);
    rows(with_d(&f, &[&u]).iter().map(|&(k, a, d)| row(vec![V::S(k), V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[4], a[3]), V::I(d[0]), V::I(a[5])])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS PostCount,
// COUNT(DISTINCT u.Id) AS UserCount,
// AVG(u.Reputation) AS AvgReputation,
// SUM(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers,
// SUM(p.ViewCount) AS TotalViewCount,
// SUM(p.Score) AS TotalScore,
// MAX(p.CreationDate) AS LatestPostDate
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// GROUP BY
// pt.Name
// ORDER BY
// PostCount DESC;
fn q12234(db: &'static So) -> String {
    let Post { accepted_answer_id, score, view_count, owner_user, creation_date, .. } = &db.post;
    let f = by_key(db.post.iq(), name(db), owner_user.select(&db.user.reputation).opt().and(accepted_answer_id.opt()).and(view_count.opt()).and(score).and(creation_date), [0, 0, 0, 0, 0, 0, 0, i64::MIN], |a: [i64; 8], ((((r, ac), w), s), c)| {
        [a[0] + 1, a[1] + r.is_some() as i64, a[2] + r.unwrap_or(0), a[3] + ac.is_some() as i64, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0), a[6] + s, a[7].max(c)]
    });
    let u = dby(db.post.iq(), name(db), owner_user);
    rows(with_d(&f, &[&u]).iter().map(|&(k, a, d)| row(vec![V::S(k), V::I(a[0]), V::I(d[0]), avg(a[2], a[1]), V::I(a[3]), nullable(a[5], a[4]), V::I(a[6]), V::T(a[7])])))
}

// WITH PostStats AS (
// SELECT
// pt.Name AS PostTypeName,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AverageScore,
// COUNT(DISTINCT u.Id) AS TotalUsers
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// pt.Name
// )
// SELECT
// PostTypeName,
// TotalPosts,
// AverageScore,
// TotalUsers
// FROM
// PostStats
// ORDER BY
// TotalPosts DESC;
fn q10535(db: &'static So) -> String {
    let base = db.post.with((&db.post.creation_date).ge(date(2023, 1, 1)));
    let f = by_key(&base, name(db), &db.post.score, [0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let u = dby(&base, name(db), &db.post.owner_user);
    rows(with_d(&f, &[&u]).iter().map(|&(k, a, d)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), V::I(d[0])])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers,
// AVG(p.Score) AS AverageScore,
// SUM(p.ViewCount) AS TotalViews,
// COUNT(DISTINCT c.Id) AS TotalComments,
// COUNT(DISTINCT bh.Id) AS TotalPostHistoryEntries
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// PostHistory bh ON p.Id = bh.PostId
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q13486(db: &'static So) -> String {
    let Post { accepted_answer_id, score, view_count, .. } = &db.post;
    let base = db.post.with((&db.post.creation_date).ge(date(2023, 1, 1)));
    let f = by_key(&base, name(db), accepted_answer_id.opt().and(score).and(view_count.opt()).and(comments_of(db).opt()).and(history_of(db).opt()), [0i64; 5], |a, ((((ac, s), w), _), _)| {
        [a[0] + 1, a[1] + ac.is_some() as i64, a[2] + s, a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0)]
    });
    let c = dby(&base, name(db), comments_of(db));
    let h = dby(&base, name(db), history_of(db));
    rows(with_d(&f, &[&c, &h]).iter().map(|&(k, a, d)| row(vec![V::S(k), V::I(a[0]), V::I(a[1]), avg(a[2], a[0]), nullable(a[4], a[3]), V::I(d[0]), V::I(d[1])])))
}

// Types LEFT JOIN Votes: COUNT, AVG(Score), up, down, AVG(ViewCount).
fn type_votes<K>(db: &'static So, key: K) -> Vec<(ROf<K>, [i64; 6], [i64; 2])>
where
    K: IntoQuery,
    K::Q: Probe<D = Id<Post>>,
    ROf<K>: Eq + std::hash::Hash + Copy,
{
    let f = by_key(db.post.iq(), key, (&db.post.score).and((&db.post.view_count).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()), [0i64; 6], |a, ((s, w), x)| {
        [a[0] + 1, a[1] + s, a[2] + (x == Some(2)) as i64, a[3] + (x == Some(3)) as i64, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0)]
    });
    with_d(&f, &[])
}

// WITH PostMetrics AS (
// SELECT
// pt.Name AS PostTypeName,
// COUNT(p.Id) AS PostCount,
// AVG(p.Score) AS AverageScore,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// pt.Name
// )
// SELECT
// PostTypeName,
// PostCount,
// AverageScore,
// UpVotes,
// DownVotes
// FROM
// PostMetrics
// ORDER BY
// PostCount DESC;
fn q10657(db: &'static So) -> String {
    rows(type_votes(db, name(db)).iter().map(|&(k, a, _)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), V::I(a[2]), V::I(a[3])])))
}

// WITH PostStats AS (
// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AverageScore,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// pt.Name
// )
// SELECT
// PostType,
// TotalPosts,
// AverageScore,
// TotalUpvotes,
// TotalDownvotes
// FROM
// PostStats
// ORDER BY
// TotalPosts DESC;
fn q11210(db: &'static So) -> String {
    rows(type_votes(db, name(db)).iter().map(|&(k, a, _)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), V::I(a[2]), V::I(a[3])])))
}

// WITH PostSummary AS (
// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AverageScore,
// SUM(CASE WHEN vt.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN vt.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Votes vt ON p.Id = vt.PostId
// GROUP BY
// pt.Name
// )
// SELECT
// PostType,
// TotalPosts,
// AverageScore,
// TotalUpVotes,
// TotalDownVotes
// FROM
// PostSummary
// ORDER BY
// TotalPosts DESC;
fn q14709(db: &'static So) -> String {
    rows(type_votes(db, name(db)).iter().map(|&(k, a, _)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), V::I(a[2]), V::I(a[3])])))
}

// WITH PostStats AS (
// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes,
// AVG(p.ViewCount) AS AverageViews
// FROM
// Posts p
// LEFT JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// pt.Name
// )
// SELECT
// PostType,
// PostCount,
// TotalUpvotes,
// TotalDownvotes,
// AverageViews
// FROM
// PostStats
// ORDER BY
// PostCount DESC;
fn q11860(db: &'static So) -> String {
    rows(type_votes(db, name(db).opt()).iter().map(|&(k, a, _)| row(vec![ostr(k), V::I(a[0]), V::I(a[2]), V::I(a[3]), avg(a[5], a[4])])))
}

// WITH PostStats AS (
// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// COUNT(DISTINCT p.OwnerUserId) AS UniqueUsers,
// AVG(u.Reputation) AS AverageUserReputation,
// SUM(p.ViewCount) AS TotalViews,
// SUM(p.Score) AS TotalScore
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
// PostType,
// TotalPosts,
// UniqueUsers,
// AverageUserReputation,
// TotalViews,
// TotalScore
// FROM
// PostStats
// ORDER BY
// TotalPosts DESC;
fn q10888(db: &'static So) -> String {
    let Post { owner_user, owner_user_id, view_count, score, .. } = &db.post;
    let f = by_key(db.post.iq(), name(db), owner_user.select(&db.user.reputation).opt().and(view_count.opt()).and(score), [0i64; 6], |a, ((r, w), s)| {
        [a[0] + 1, a[1] + r.is_some() as i64, a[2] + r.unwrap_or(0), a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0), a[5] + s]
    });
    let u = dby(db.post.iq(), name(db), owner_user_id);
    rows(with_d(&f, &[&u]).iter().map(|&(k, a, d)| row(vec![V::S(k), V::I(a[0]), V::I(d[0]), avg(a[2], a[1]), nullable(a[4], a[3]), V::I(a[5])])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// SUM(p.ViewCount) AS TotalViews,
// SUM(p.Score) AS TotalScore,
// AVG(CASE WHEN p.OwnerUserId IS NOT NULL THEN u.Reputation ELSE 0 END) AS AvgUserReputation,
// COUNT(DISTINCT v.UserId) AS UniqueVoters,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q12222(db: &'static So) -> String {
    let Post { owner_user, owner_user_id, view_count, score, .. } = &db.post;
    let f = by_key(
        db.post.iq(),
        name(db),
        view_count.opt().and(score).and(owner_user_id.opt()).and(owner_user.select(&db.user.reputation).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()),
        [0i64; 8],
        |a, ((((w, s), o), r), x)| {
            // CASE WHEN p.OwnerUserId IS NOT NULL THEN u.Reputation ELSE 0 END
            let rep = if o.is_some() { r } else { Some(0) };
            [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s, a[4] + rep.is_some() as i64, a[5] + rep.unwrap_or(0), a[6] + (x == Some(2)) as i64, a[7] + (x == Some(3)) as i64]
        },
    );
    let voters = dby(db.post.iq(), name(db), votes_of(db).select(&db.vote.user_id));
    rows(with_d(&f, &[&voters]).iter().map(|&(k, a, d)| {
        row(vec![V::S(k), V::I(a[0]), nullable(a[2], a[1]), V::I(a[3]), avg(a[5], a[4]), V::I(d[0]), V::I(a[6]), V::I(a[7])])
    }))
}

// GROUP BY p.PostTypeId.
// SELECT
// P.PostTypeId,
// COUNT(P.Id) AS TotalPosts,
// SUM(CASE WHEN P.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers,
// AVG(U.Reputation) AS AverageUserReputation,
// COUNT(DISTINCT U.Id) AS TotalUsers,
// AVG(P.Score) AS AveragePostScore,
// SUM(P.ViewCount) AS TotalViewCount,
// SUM(P.CommentCount) AS TotalCommentCount,
// COUNT(DISTINCT P.OwnerUserId) AS UniquePostOwners
// FROM
// Posts P
// LEFT JOIN
// Users U ON P.OwnerUserId = U.Id
// GROUP BY
// P.PostTypeId
// ORDER BY
// P.PostTypeId;
fn q10346(db: &'static So) -> String {
    let Post { post_type_id, accepted_answer_id, owner_user, owner_user_id, score, view_count, comment_count, .. } = &db.post;
    let f = by_key(db.post.iq(), post_type_id, accepted_answer_id.opt().and(owner_user.select(&db.user.reputation).opt()).and(score).and(view_count.opt()).and(comment_count), [0i64; 8], |a, ((((ac, r), s), w), c)| {
        [a[0] + 1, a[1] + ac.is_some() as i64, a[2] + r.is_some() as i64, a[3] + r.unwrap_or(0), a[4] + s, a[5] + w.is_some() as i64, a[6] + w.unwrap_or(0), a[7] + c]
    });
    let u = dby(db.post.iq(), post_type_id, owner_user);
    let o = dby(db.post.iq(), post_type_id, owner_user_id);
    let mut v = with_d(&f, &[&u, &o]);
    v.sort_by_key(|x| x.0);
    rows(v.iter().map(|&(t, a, d)| row(vec![V::I(t), V::I(a[0]), V::I(a[1]), avg(a[3], a[2]), V::I(d[0]), avg(a[4], a[0]), nullable(a[6], a[5]), V::I(a[7]), V::I(d[1])])))
}

// SELECT
// p.PostTypeId,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositiveScoreCount,
// SUM(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers,
// COUNT(DISTINCT u.Id) AS UserCount,
// COUNT(DISTINCT b.Id) AS BadgeCount,
// AVG(u.Reputation) AS AvgReputation
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.PostTypeId
// ORDER BY
// p.PostTypeId;
fn q12648(db: &'static So) -> String {
    let Post { post_type_id, accepted_answer_id, owner_user, score, creation_date, .. } = &db.post;
    let base = owned(db).with(creation_date.ge(date(2023, 1, 1)));
    let f = by_key(&base, post_type_id, score.and(accepted_answer_id.opt()).and(owner_user.select(&db.user.reputation)).and(owner_user.select(badges_of(db)).opt()), [0i64; 4], |a, (((s, ac), r), _)| {
        [a[0] + 1, a[1] + (s > 0) as i64, a[2] + ac.is_some() as i64, a[3] + r]
    });
    let u = dby(&base, post_type_id, owner_user);
    let b = dby(&base, post_type_id, owner_user.select(badges_of(db)));
    let mut v = with_d(&f, &[&u, &b]);
    v.sort_by_key(|x| x.0);
    rows(v.iter().map(|&(t, a, d)| row(vec![V::I(t), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(d[0]), V::I(d[1]), avg(a[3], a[0])])))
}

// SELECT
// p.PostTypeId,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes,
// COUNT(DISTINCT u.Id) AS TotalUsers,
// AVG(u.Reputation) AS AverageReputation,
// MIN(p.CreationDate) AS EarliestPostDate,
// MAX(p.CreationDate) AS LatestPostDate
// FROM
// Posts p
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 YEAR'
// GROUP BY
// p.PostTypeId
// ORDER BY
// TotalPosts DESC;
fn q12879(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, .. } = &db.post;
    let base = db.post.with(creation_date.ge(ts(2023, 10, 1, 12, 34, 56)));
    let f = by_key(&base, post_type_id, creation_date.and(owner_user.select(&db.user.reputation).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()), [0, 0, 0, 0, i64::MAX, i64::MIN], |a: [i64; 6], ((c, r), x)| {
        [a[0] + (x == Some(2)) as i64, a[1] + (x == Some(3)) as i64, a[2] + r.is_some() as i64, a[3] + r.unwrap_or(0), a[4].min(c), a[5].max(c)]
    });
    let p = dby(&base, post_type_id, Ident::<Post>::new());
    let u = dby(&base, post_type_id, owner_user);
    let mut v = with_d(&f, &[&p, &u]);
    v.sort_by_key(|x| Reverse(x.2[0]));
    rows(v.iter().map(|&(t, a, d)| row(vec![V::I(t), V::I(d[0]), V::I(a[0]), V::I(a[1]), V::I(d[1]), avg(a[3], a[2]), V::T(a[4]), V::T(a[5])])))
}

// WITH PostMetrics AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS PostCount,
// AVG(CASE WHEN p.PostTypeId = 1 THEN p.Score ELSE NULL END) AS AvgQuestionScore,
// COUNT(c.Id) AS TotalComments
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// WHERE
// p.CreationDate BETWEEN '2022-01-01' AND '2022-12-31'
// GROUP BY
// p.OwnerUserId
// )
// SELECT
// u.DisplayName,
// pm.PostCount,
// pm.AvgQuestionScore,
// pm.TotalComments
// FROM
// PostMetrics pm
// JOIN
// Users u ON u.Id = pm.OwnerUserId
// ORDER BY
// pm.PostCount DESC;
fn q11858(db: &'static So) -> String {
    let Post { owner_user_id, post_type_id, score, creation_date, .. } = &db.post;
    let base = db.post.with(creation_date.between(date(2022, 1, 1), date(2022, 12, 31)));
    let metrics = (&base)
        .group_by(owner_user_id)
        .select(post_type_id.and(score).and(comments_of(db).opt()))
        .fold([0i64; 4], |a, ((t, s), c)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + if t == 1 { s } else { 0 }, a[3] + c.is_some() as i64]);
    let mut v = Vec::new();
    db.user.select((&db.user.origid).select(&metrics)).drive(|u, a| v.push((u, a)));
    rows(v.iter().map(|&(u, a)| row(vec![user_col(db, u, "name"), V::I(a[0]), avg(a[2], a[1]), V::I(a[3])])))
}

fn scalar_row(db: &'static So) -> [i64; 4] {
    [count(db.post.iq()), count(db.user.iq()), count(db.vote.iq()), count(db.comment.iq())]
}

// WITH PostStats AS (
// SELECT
// COUNT(*) AS TotalPosts,
// AVG(Score) AS AverageScore
// FROM
// Posts
// ),
// UserStats AS (
// SELECT
// COUNT(*) AS TotalUsers,
// AVG(Reputation) AS AverageReputation
// FROM
// Users
// ),
// VoteStats AS (
// SELECT
// COUNT(*) AS TotalVotes
// FROM
// Votes
// )
// SELECT
// p.TotalPosts,
// p.AverageScore,
// u.TotalUsers,
// u.AverageReputation,
// v.TotalVotes
// FROM
// PostStats p, UserStats u, VoteStats v;
fn q12858(db: &'static So) -> String {
    let [p, u, x, _] = scalar_row(db);
    row(vec![V::I(p), mean(&db.post.score), V::I(u), mean(&db.user.reputation), V::I(x)])
}

// WITH PostCounts AS (
// SELECT
// COUNT(*) AS TotalPosts,
// AVG(Score) AS AvgPostScore
// FROM
// Posts
// ),
// CommentCounts AS (
// SELECT
// COUNT(*) AS TotalComments
// FROM
// Comments
// ),
// UserStats AS (
// SELECT
// COUNT(*) AS TotalUsers,
// AVG(Reputation) AS AvgUserReputation
// FROM
// Users
// )
// SELECT
// p.TotalPosts,
// c.TotalComments,
// u.TotalUsers,
// p.AvgPostScore,
// u.AvgUserReputation
// FROM
// PostCounts p,
// CommentCounts c,
// UserStats u;
fn q13785(db: &'static So) -> String {
    let [p, u, _, c] = scalar_row(db);
    row(vec![V::I(p), V::I(c), V::I(u), mean(&db.post.score), mean(&db.user.reputation)])
}

// WITH UserStats AS (
// SELECT
// COUNT(*) AS TotalUsers,
// AVG(Reputation) AS AvgReputation
// FROM Users
// ),
// PostStats AS (
// SELECT
// COUNT(*) AS TotalPosts,
// AVG(ViewCount) AS AvgViewCount
// FROM Posts
// ),
// CommentStats AS (
// SELECT
// COUNT(*) AS TotalComments
// FROM Comments
// ),
// VoteStats AS (
// SELECT
// COUNT(*) AS TotalVotes
// FROM Votes
// )
// SELECT
// u.TotalUsers,
// u.AvgReputation,
// p.TotalPosts,
// p.AvgViewCount,
// c.TotalComments,
// v.TotalVotes
// FROM
// UserStats u,
// PostStats p,
// CommentStats c,
// VoteStats v;
fn q13758(db: &'static So) -> String {
    let [p, u, x, c] = scalar_row(db);
    row(vec![V::I(u), mean(&db.user.reputation), V::I(p), mean(db.post.select(&db.post.view_count)), V::I(c), V::I(x)])
}

// WITH UserStats AS (
// SELECT COUNT(*) AS TotalUsers
// FROM Users
// ),
// PostStats AS (
// SELECT COUNT(*) AS TotalPosts, AVG(ViewCount) AS AvgViewCount
// FROM Posts
// ),
// CommentStats AS (
// SELECT COUNT(*) AS TotalComments, MAX(CreationDate) AS LastCommentDate
// FROM Comments
// )
// SELECT
// (SELECT TotalUsers FROM UserStats) AS TotalUsers,
// (SELECT TotalPosts FROM PostStats) AS TotalPosts,
// (SELECT AvgViewCount FROM PostStats) AS AvgViewCount,
// (SELECT TotalComments FROM CommentStats) AS TotalComments,
// (SELECT LastCommentDate FROM CommentStats) AS LastCommentDate
// ;
fn q13741(db: &'static So) -> String {
    let [p, u, _, c] = scalar_row(db);
    let last = (&db.comment.creation_date).fold_flat(i64::MIN, |a, d| a.max(d));
    row(vec![V::I(u), V::I(p), mean(db.post.select(&db.post.view_count)), V::I(c), if c == 0 { V::Null } else { V::T(last) }])
}

// SELECT
// (SELECT COUNT(*) FROM Posts) AS TotalPosts,
// (SELECT COUNT(*) FROM Comments) AS TotalComments,
// (SELECT COUNT(*) FROM Users) AS TotalUsers,
// (SELECT AVG(Score) FROM Posts) AS AvgPostScore,
// (SELECT AVG(Reputation) FROM Users) AS AvgUserReputation,
// (SELECT COUNT(DISTINCT OwnerUserId) FROM Posts WHERE OwnerUserId IS NOT NULL) AS UniquePostOwners,
// (SELECT COUNT(DISTINCT UserId) FROM Comments WHERE UserId IS NOT NULL) AS UniqueCommentUsers
fn q14121(db: &'static So) -> String {
    let [p, u, _, c] = scalar_row(db);
    let owners = one(whole(db.post.iq()).select(&db.post.owner_user_id).count_distinct());
    let commenters = one(whole(db.comment.iq()).select(&db.comment.user_id).count_distinct());
    row(vec![V::I(p), V::I(c), V::I(u), mean(&db.post.score), mean(&db.user.reputation), V::I(owners), V::I(commenters)])
}

// SELECT
// (SELECT COUNT(*) FROM Posts) AS Total_Posts,
// (SELECT COUNT(*) FROM Users) AS Total_Users,
// (SELECT COUNT(*) FROM Votes) AS Total_Votes,
// (SELECT COUNT(*) FROM Badges) AS Total_Badges,
// (SELECT AVG(Score) FROM Posts WHERE Score IS NOT NULL) AS Avg_Post_Score,
// (SELECT AVG(Reputation) FROM Users WHERE Reputation IS NOT NULL) AS Avg_User_Reputation,
// (SELECT AVG(VoteTypeId * 1.0) FROM Votes) AS Avg_Vote_Type,
// (SELECT COUNT(DISTINCT PostId) FROM Comments) AS Total_Commented_Posts,
// (SELECT AVG(LENGTH(Body)) FROM Posts WHERE Body IS NOT NULL) AS Avg_Post_Body_Length
fn q10966(db: &'static So) -> String {
    let [p, u, x, _] = scalar_row(db);
    let commented = one(whole(db.comment.iq()).select(&db.comment.post_id).count_distinct());
    let (n, len) = (&db.post.body).fold_flat((0i64, 0i64), |(n, l), b| (n + 1, l + b.chars().count() as i64));
    row(vec![V::I(p), V::I(u), V::I(x), V::I(count(db.badge.iq())), mean(&db.post.score), mean(&db.user.reputation), mean(&db.vote.vote_type_id), V::I(commented), avg(len, n)])
}

// WITH CommentsPerPost AS (
// SELECT
// PostId,
// COUNT(Id) AS CommentCount
// FROM
// Comments
// GROUP BY
// PostId
// ),
// VotesPerUser AS (
// SELECT
// UserId,
// COUNT(Id) AS VoteCount
// FROM
// Votes
// GROUP BY
// UserId
// ),
// BadgesPerUser AS (
// SELECT
// UserId,
// COUNT(Id) AS BadgeCount
// FROM
// Badges
// GROUP BY
// UserId
// )
// SELECT
// (SELECT AVG(CommentCount) FROM CommentsPerPost) AS AvgCommentsPerPost,
// (SELECT SUM(VoteCount) FROM VotesPerUser) AS TotalVotes,
// (SELECT SUM(BadgeCount) FROM BadgesPerUser) AS TotalBadges
fn q14821(db: &'static So) -> String {
    let per_post = db.comment.group_by(&db.comment.post_id).fold(0i64, |a, _| a + 1);
    let per_voter = db.vote.group_by((&db.vote.user_id).opt()).fold(0i64, |a, _| a + 1);
    let per_badger = db.badge.group_by(&db.badge.user_id).fold(0i64, |a, _| a + 1);
    let (n, s) = (&per_post).fold_flat((0i64, 0i64), |(n, s), c| (n + 1, s + c));
    let (vn, votes) = (&per_voter).fold_flat((0i64, 0i64), |(n, a), c| (n + 1, a + c));
    let (bn, badges) = (&per_badger).fold_flat((0i64, 0i64), |(n, a), c| (n + 1, a + c));
    row(vec![avg(s, n), nullable(votes, vn), nullable(badges, bn)])
}

// WITH PostStats AS (
// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AvgPostScore
// FROM Posts p
// JOIN PostTypes pt ON p.PostTypeId = pt.Id
// GROUP BY pt.Name
// ),
// UserStats AS (
// SELECT
// COUNT(u.Id) AS TotalUsers,
// AVG(u.Reputation) AS AvgReputation
// FROM Users u
// )
// SELECT
// ps.PostType,
// ps.TotalPosts,
// ps.AvgPostScore,
// us.TotalUsers,
// us.AvgReputation
// FROM PostStats ps, UserStats us
// ORDER BY ps.TotalPosts DESC;
fn q13068(db: &'static So) -> String {
    let (n, r) = (&db.user.reputation).fold_flat((0i64, 0i64), |(n, s), x| (n + 1, s + x));
    rows(by_count(db).iter().map(|a| {
        let mut f = type_fields(a, &["name", "n", "score_avg"]);
        f.extend([V::I(n), avg(r, n)]);
        row(f)
    }))
}

// WITH PostCounts AS (
// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS PostCount
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// GROUP BY
// pt.Name
// ),
// AverageScore AS (
// SELECT
// AVG(p.Score) AS AverageScore
// FROM
// Posts p
// WHERE
// p.PostTypeId = 1
// ),
// UserCount AS (
// SELECT
// COUNT(*) AS TotalUsers
// FROM
// Users
// )
// SELECT
// pc.PostType,
// pc.PostCount,
// avg.AverageScore,
// uc.TotalUsers
// FROM
// PostCounts pc,
// AverageScore avg,
// UserCount uc;
fn q12382(db: &'static So) -> String {
    let (n, s) = db.post.with((&db.post.post_type_id).eq(1)).select(&db.post.score).fold_flat((0i64, 0i64), |(n, s), x| (n + 1, s + x));
    let users = count(db.user.iq());
    rows(by_count(db).iter().map(|a| {
        let mut f = type_fields(a, &["name", "n"]);
        f.extend([avg(s, n), V::I(users)]);
        row(f)
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("19370", q19370),
    ("16609", q16609),
    ("17332", q17332),
    ("19841", q19841),
    ("15016", q15016),
    ("17078", q17078),
    ("17475", q17475),
    ("17715", q17715),
    ("17849", q17849),
    ("11010", q11010),
    ("15601", q15601),
    ("17648", q17648),
    ("12541", q12541),
    ("12936", q12936),
    ("12410", q12410),
    ("10895", q10895),
    ("14907", q14907),
    ("11255", q11255),
    ("12665", q12665),
    ("10377", q10377),
    ("14517", q14517),
    ("14645", q14645),
    ("12492", q12492),
    ("11256", q11256),
    ("10415", q10415),
    ("14782", q14782),
    ("13155", q13155),
    ("11197", q11197),
    ("13386", q13386),
    ("11374", q11374),
    ("11648", q11648),
    ("13557", q13557),
    ("11353", q11353),
    ("14112", q14112),
    ("11264", q11264),
    ("10252", q10252),
    ("14460", q14460),
    ("14561", q14561),
    ("12234", q12234),
    ("10535", q10535),
    ("13486", q13486),
    ("10657", q10657),
    ("11210", q11210),
    ("14709", q14709),
    ("11860", q11860),
    ("10888", q10888),
    ("12222", q12222),
    ("10346", q10346),
    ("12648", q12648),
    ("12879", q12879),
    ("11858", q11858),
    ("12858", q12858),
    ("13785", q13785),
    ("13758", q13758),
    ("13741", q13741),
    ("14121", q14121),
    ("10966", q10966),
    ("14821", q14821),
    ("13068", q13068),
    ("12382", q12382),
];
