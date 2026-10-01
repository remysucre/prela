use harness::prelude::*;
use std::cmp::Reverse;

fn cd(db: &'static So, p: Id<Post>) -> Reverse<i64> {
    Reverse(db.post.creation_date.get(p).unwrap())
}

fn score(db: &'static So, p: Id<Post>) -> Reverse<i64> {
    Reverse(db.post.score.get(p).unwrap())
}

fn views(db: &'static So, p: Id<Post>) -> (bool, Reverse<Option<i64>>) {
    let w = db.post.view_count.get(p);
    (w.is_none(), Reverse(w))
}

fn since(db: &'static So, d: i64) -> Restrict<Key<Post>, Filter<&'static Col<Post, i64>, impl Fn(i64) -> bool>> {
    db.post.with((&db.post.creation_date).ge(d))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// u.DisplayName AS OwnerDisplayName,
// p.Score,
// p.ViewCount,
// pd.Name AS PostTypeName,
// COUNT(c.Id) AS CommentCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// JOIN
// PostTypes pd ON p.PostTypeId = pd.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, u.DisplayName, p.Score, p.ViewCount, pd.Name
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q19938(db: &'static So) -> String {
    stat_rows(db, post_stats(db, owned(db), "c", &[]), |p, _| cd(db, p), 10, &["id", "title", "created", "owner", "score", "views", "type", "#cx"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// COUNT(b.Id) AS BadgeCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Badges b ON p.OwnerUserId = b.UserId
// WHERE
// p.CreationDate >= '2022-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount
// ORDER BY
// p.CreationDate DESC;
fn q14582(db: &'static So) -> String {
    stat_rows(db, post_stats(db, since(db, date(2022, 1, 1)), "cvb", &[]), |p, _| cd(db, p), 0, &["id", "title", "created", "score", "views", "#cx", "#vx", "#bx"])
}

// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.Score,
// P.ViewCount,
// U.DisplayName AS OwnerDisplayName,
// COUNT(C.Id) AS CommentCount,
// SUM(V.BountyAmount) AS TotalBounty
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
// P.Id, P.Title, P.CreationDate, P.Score, P.ViewCount, U.DisplayName
// ORDER BY
// P.CreationDate DESC
// LIMIT 100;
fn q11823(db: &'static So) -> String {
    let base = owned(db).with((&db.post.post_type_id).eq(1));
    stat_rows(db, post_stats(db, base, "cv", &[]), |p, _| cd(db, p), 100, &["id", "title", "created", "score", "views", "owner", "#cx", "bounty_sum"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// AVG(u.Reputation) AS AverageUserReputation
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q10896(db: &'static So) -> String {
    stat_rows(db, post_stats(db, since(db, date(2023, 1, 1)), "cv", &[]), |p, _| cd(db, p), 100, &["id", "title", "created", "score", "views", "#cx", "#vx", "rep_avg"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// u.DisplayName AS OwnerDisplayName,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS UpvoteCount
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId AND v.VoteTypeId = 2
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, u.DisplayName, p.CreationDate, p.Score, p.ViewCount
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q16296(db: &'static So) -> String {
    let base = db.post.with((&db.post.post_type_id).eq(1));
    stat_rows(db, post_stats(db, base, "cv", &[2]), |p, _| cd(db, p), 10, &["id", "title", "owner", "created", "score", "views", "#cx", "#vx"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// COUNT(c.Id) AS CommentCount,
// COUNT(ph.Id) AS PostHistoryCount,
// SUM(v.BountyAmount) AS TotalBountyAmount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId AND v.VoteTypeId = 8
// WHERE
// p.CreationDate >= '2022-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score
// ORDER BY
// p.Score DESC, p.ViewCount DESC;
fn q10709(db: &'static So) -> String {
    stat_rows(db, post_stats(db, since(db, date(2022, 1, 1)), "chv", &[8]), |p, _| (score(db, p), views(db, p)), 0, &["id", "title", "created", "views", "score", "#cx", "#hx", "bounty_sum"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// p.AnswerCount,
// p.CommentCount,
// u.DisplayName AS OwnerDisplayName,
// COUNT(v.Id) AS TotalVotes,
// AVG(v.BountyAmount) AS AverageBounty
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, p.CommentCount, u.DisplayName
// ORDER BY
// p.CreationDate DESC;
fn q13240(db: &'static So) -> String {
    stat_rows(db, post_stats(db, since(db, date(2023, 1, 1)), "v", &[]), |p, _| cd(db, p), 0, &["id", "title", "created", "views", "score", "answers", "comments", "owner", "#vx", "bounty_avg"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// u.DisplayName AS OwnerDisplayName,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// AVG(v.BountyAmount) AS AvgBountyAmount
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
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName
// ORDER BY
// p.CreationDate DESC;
fn q14462(db: &'static So) -> String {
    let base = db.post.with((&db.post.post_type_id).in_v(vec![1, 2]));
    stat_rows(db, post_stats(db, base, "cv", &[]), |p, _| cd(db, p), 0, &["id", "title", "created", "score", "views", "owner", "#cx", "#vx", "bounty_avg"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// u.DisplayName AS OwnerDisplayName,
// COUNT(c.Id) AS CommentCount,
// AVG(vt.Id) AS AverageVoteType
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// VoteTypes vt ON v.VoteTypeId = vt.Id
// WHERE
// p.CreationDate >= '2022-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, u.DisplayName
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q11527(db: &'static So) -> String {
    let base = owned(db).with((&db.post.creation_date).ge(date(2022, 1, 1)));
    stat_rows(db, post_stats(db, base, "cv", &[]), |p, _| cd(db, p), 100, &["id", "title", "created", "views", "owner", "#cx", "vtj_avg"])
}

// SELECT
// p.Title AS PostTitle,
// p.CreationDate AS PostCreationDate,
// u.DisplayName AS OwnerDisplayName,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// AVG(v.BountyAmount) AS AvgBountyAmount,
// MAX(p.LastActivityDate) AS LastActivityDate
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Id, u.DisplayName, p.Title, p.CreationDate
// ORDER BY
// p.CreationDate DESC;
fn q11766(db: &'static So) -> String {
    stat_rows(db, post_stats(db, since(db, date(2023, 1, 1)), "cv", &[]), |p, _| cd(db, p), 0, &["title", "created", "owner", "#cx", "#vx", "bounty_avg", "activity"])
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
// MAX(p.LastActivityDate) AS LastActivityDate,
// MAX(p.LastEditDate) AS LastEditDate
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q14134(db: &'static So) -> String {
    stat_rows(db, post_stats(db, db.post.iq(), "cv", &[]), |p, _| cd(db, p), 100, &["id", "title", "created", "views", "score", "owner", "#cx", "#vx", "activity", "edited"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// COUNT(c.Id) AS TotalComments,
// AVG(v.VoteTypeId) AS AverageVoteType,
// u.Reputation AS UserReputation,
// u.DisplayName AS UserDisplayName
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.Reputation, u.DisplayName
// ORDER BY
// p.ViewCount DESC;
fn q10016(db: &'static So) -> String {
    stat_rows(db, post_stats(db, since(db, date(2023, 1, 1)), "cv", &[]), |p, _| views(db, p), 0, &["id", "title", "created", "views", "score", "#cx", "vt_avg", "rep", "owner"])
}

// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.Score,
// P.ViewCount,
// U.DisplayName AS OwnerDisplayName,
// U.Reputation AS OwnerReputation,
// COUNT(C.Id) AS CommentCount,
// COUNT(V.Id) AS VoteCount,
// AVG(V.BountyAmount) AS AverageBountyAmount
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// WHERE
// P.PostTypeId IN (1, 2)
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.Score, P.ViewCount, U.DisplayName, U.Reputation
// ORDER BY
// P.CreationDate DESC;
fn q11460(db: &'static So) -> String {
    let base = owned(db).with((&db.post.post_type_id).in_v(vec![1, 2]));
    stat_rows(db, post_stats(db, base, "cv", &[]), |p, _| cd(db, p), 0, &["id", "title", "created", "score", "views", "owner", "rep", "#cx", "#vx", "bounty_avg"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// pt.Name AS PostType,
// u.DisplayName AS OwnerDisplayName,
// u.Reputation AS OwnerReputation,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, pt.Name, u.DisplayName, u.Reputation
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q11873(db: &'static So) -> String {
    stat_rows(db, post_stats(db, owned(db), "cv", &[]), |p, _| cd(db, p), 100, &["id", "title", "created", "score", "views", "type", "owner", "rep", "#cx", "#vx"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// COUNT(c.Id) AS CommentCount,
// COUNT(a.Id) AS AnswerCount,
// u.DisplayName AS OwnerDisplayName,
// p.LastActivityDate,
// p.Tags
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName, p.LastActivityDate, p.Tags
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q14348(db: &'static So) -> String {
    stat_rows(db, post_stats(db, since(db, date(2023, 1, 1)), "cA", &[]), |p, _| cd(db, p), 100, &["id", "title", "created", "score", "views", "#cx", "#ax", "owner", "activity", "tags"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// pt.Name AS PostType,
// u.DisplayName AS OwnerDisplayName,
// u.Reputation AS OwnerReputation,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, pt.Name, u.DisplayName, u.Reputation
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q12524(db: &'static So) -> String {
    stat_rows(db, post_stats(db, db.post.iq(), "cv", &[]), |p, _| cd(db, p), 100, &["id", "title", "created", "score", "views", "type", "owner", "rep", "#cx", "#vx"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// u.DisplayName AS OwnerDisplayName,
// u.Reputation AS OwnerReputation,
// pt.Name AS PostType,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName, u.Reputation, pt.Name
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q10310(db: &'static So) -> String {
    stat_rows(db, post_stats(db, db.post.iq(), "cv", &[]), |p, _| cd(db, p), 100, &["id", "title", "created", "score", "views", "owner", "rep", "type", "#cx", "#vx"])
}

// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.ViewCount,
// P.Score,
// P.AnswerCount,
// P.CommentCount,
// U.DisplayName AS OwnerDisplayName,
// COUNT(C.Id) AS CommentCount,
// SUM(V.BountyAmount) AS TotalBountyReceived
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// WHERE
// P.CreationDate >= '2022-01-01'
// AND P.PostTypeId = 1
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.ViewCount, P.Score, P.AnswerCount, P.CommentCount, U.DisplayName
// ORDER BY
// P.ViewCount DESC;
fn q12094(db: &'static So) -> String {
    let base = owned(db).with((&db.post.creation_date).ge(date(2022, 1, 1)).and((&db.post.post_type_id).eq(1)));
    stat_rows(db, post_stats(db, base, "cv", &[]), |p, _| views(db, p), 0, &["id", "title", "created", "views", "score", "answers", "comments", "owner", "#cx", "bounty_sum"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate AS PostCreationDate,
// p.ViewCount,
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// u.CreationDate AS UserCreationDate,
// u.LastAccessDate,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.ViewCount > 1000
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, u.Id, u.DisplayName, u.Reputation, u.CreationDate, u.LastAccessDate
// ORDER BY
// p.ViewCount DESC
// LIMIT 10;
fn q14368(db: &'static So) -> String {
    let base = owned(db).with((&db.post.view_count).gt(1000));
    stat_rows(db, post_stats(db, base, "cv", &[]), |p, _| views(db, p), 10, &["id", "title", "created", "views", "uid", "owner", "rep", "ucreated", "last_access", "#cx", "#vx"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate AS PostCreationDate,
// p.ViewCount,
// p.Score,
// p.AnswerCount,
// p.CommentCount,
// u.Id AS UserId,
// u.DisplayName AS UserDisplayName,
// u.Reputation AS UserReputation,
// COUNT(v.Id) AS VoteCount,
// MAX(v.CreationDate) AS LastVoteDate
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, p.CommentCount,
// u.Id, u.DisplayName, u.Reputation
// ORDER BY
// p.CreationDate DESC;
fn q12242(db: &'static So) -> String {
    let base = owned(db).with((&db.post.creation_date).ge(date(2023, 1, 1)));
    stat_rows(db, post_stats(db, base, "v", &[]), |p, _| cd(db, p), 0, &["id", "title", "created", "views", "score", "answers", "comments", "uid", "owner", "rep", "#vx", "vmax"])
}

// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate AS PostCreationDate,
// P.ViewCount,
// P.Score,
// COUNT(CM.Id) AS CommentCount,
// SUM(V.BountyAmount) AS TotalBounty,
// U.Reputation AS UserReputation,
// U.CreationDate AS UserCreationDate,
// U.DisplayName AS UserDisplayName
// FROM
// Posts P
// LEFT JOIN
// Comments CM ON P.Id = CM.PostId
// LEFT JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// Votes V ON P.Id = V.PostId AND V.VoteTypeId IN (8, 9)
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.ViewCount, P.Score, U.Reputation, U.CreationDate, U.DisplayName
// ORDER BY
// P.CreationDate DESC
// LIMIT 100;
fn q10093(db: &'static So) -> String {
    stat_rows(db, post_stats(db, db.post.iq(), "cv", &[8, 9]), |p, _| cd(db, p), 100, &["id", "title", "created", "views", "score", "#cx", "bounty_sum", "rep", "ucreated", "owner"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// u.DisplayName AS OwnerDisplayName,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// p.Score,
// p.ViewCount,
// p.AnswerCount,
// p.FavoriteCount,
// p.ClosedDate,
// p.LastActivityDate
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
// p.Id, p.Title, p.CreationDate, u.DisplayName, p.Score, p.ViewCount, p.AnswerCount, p.FavoriteCount, p.ClosedDate, p.LastActivityDate
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q13610(db: &'static So) -> String {
    let base = owned(db).with((&db.post.post_type_id).eq(1));
    stat_rows(db, post_stats(db, base, "cv", &[]), |p, _| cd(db, p), 100, &["id", "title", "created", "owner", "#cx", "#vx", "score", "views", "answers", "favorites", "closed", "activity"])
}

// SELECT
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// u.DisplayName AS OwnerDisplayName,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// AVG(v.BountyAmount) AS AverageBounty
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// u.DisplayName
// ORDER BY
// p.CreationDate DESC;
fn q13119(db: &'static So) -> String {
    let Post { title, creation_date, view_count, score, owner_user, post_type_id, .. } = &db.post;
    let key = title.opt().and(creation_date).and(view_count.opt()).and(score).and(owner_user.select(&db.user.display_name).opt());
    let v = group_stats(db, db.post.with(post_type_id.eq(1)), key, "cv", &[]);
    rows(v.iter().map(|&(((((t, c), w), s), dn), ref st)| {
        let mut f = vec![ostr(t), V::T(c), oint(w), V::I(s), ostr(dn)];
        f.extend(["#cx", "#vx", "bounty_avg"].iter().map(|x| stat_field(st, x).unwrap()));
        row(f)
    }))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(u.Reputation) AS AverageUserReputation,
// SUM(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers,
// SUM(p.ViewCount) AS TotalViewCount,
// MAX(p.CreationDate) AS MostRecentPost,
// MIN(p.CreationDate) AS OldestPost
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// JOIN
// Users u ON p.OwnerUserId = u.Id
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q13050(db: &'static So) -> String {
    let Post { post_type, view_count, owner_user, accepted_answer_id, creation_date, .. } = &db.post;
    let mut v = Vec::new();
    owned(db)
        .group_by(post_type.select(&db.post_type.name))
        .select(owner_user.select(&db.user.reputation).and(accepted_answer_id.opt()).and(view_count.opt()).and(creation_date))
        .fold([0, 0, 0, 0, 0, i64::MIN, i64::MAX], |a: [i64; 7], (((r, ac), w), c)| {
            [a[0] + 1, a[1] + r, a[2] + ac.is_some() as i64, a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0), a[5].max(c), a[6].min(c)]
        })
        .drive(|k, a| v.push((k, a)));
    v.sort_by(|a, b| b.1[0].cmp(&a.1[0]));
    rows(v.iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), V::I(a[2]), nullable(a[4], a[3]), V::T(a[5]), V::T(a[6])])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// COUNT(CASE WHEN p.Score > 0 THEN 1 END) AS PositiveScorePosts,
// COUNT(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 END) AS AcceptedAnswers,
// AVG(u.Reputation) AS AverageUserReputation,
// SUM(u.UpVotes) AS TotalUpVotes,
// SUM(u.DownVotes) AS TotalDownVotes
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// JOIN
// Users u ON p.OwnerUserId = u.Id
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q11833(db: &'static So) -> String {
    let Post { post_type, score, owner_user, accepted_answer_id, .. } = &db.post;
    let User { reputation, up_votes, down_votes, .. } = &db.user;
    let mut v = Vec::new();
    owned(db)
        .group_by(post_type.select(&db.post_type.name))
        .select(score.and(accepted_answer_id.opt()).and(owner_user.select(reputation.and(up_votes).and(down_votes))))
        .fold((0i64, 0i64, 0i64, 0i64, 0i64, 0i64), |(n, pos, acc, r, u, d), ((s, ac), ((rep, up), dn))| {
            (n + 1, pos + (s > 0) as i64, acc + ac.is_some() as i64, r + rep, u + up, d + dn)
        })
        .drive(|k, a| v.push((k, a)));
    v.sort_by(|a, b| b.1.0.cmp(&a.1.0));
    rows(v.iter().map(|&(k, (n, pos, acc, r, u, d))| row(vec![V::S(k), V::I(n), V::I(pos), V::I(acc), avg(r, n), V::I(u), V::I(d)])))
}

// COUNT, AVG(u.Reputation), SUM(Score), SUM(ViewCount), AVG(Score),
// COUNT(DISTINCT p.OwnerUserId) per type, over owned posts in `base`.
fn owner_type_stats<Q>(db: &'static So, base: Q) -> Vec<(i64, Str, (i64, i64, i64, i64, i64), i64)>
where
    Q: IntoQuery + Copy,
    Q::Q: Drive<D = Id<Post>, R = Id<Post>>,
{
    let Post { post_type, score, view_count, owner_user, owner_user_id, .. } = &db.post;
    let vals = score.and(view_count.opt()).and(owner_user.select(&db.user.reputation));
    let fold = |(n, r, s, vn, vs): (i64, i64, i64, i64, i64), ((x, w), rep): ((i64, Option<i64>), i64)| {
        (n + 1, r + rep, s + x, vn + w.is_some() as i64, vs + w.unwrap_or(0))
    };
    let main = base.group_by(post_type).select(vals).fold((0i64, 0i64, 0i64, 0i64, 0i64), fold);
    let uniq = base.group_by(post_type).select(owner_user_id).count_distinct();
    let mut v = Vec::new();
    main.and((&uniq).opt()).drive(|t, (a, u)| {
        v.push((db.post_type.origid.get(t).unwrap(), db.post_type.name.get(t).unwrap(), a, u.unwrap_or(0)))
    });
    v.sort_by(|a, b| b.2.0.cmp(&a.2.0));
    v
}

// WITH PostStats AS (
// SELECT
// p.PostTypeId,
// COUNT(p.Id) AS PostCount,
// AVG(u.Reputation) AS AverageUserReputation,
// SUM(p.ViewCount) AS TotalViews,
// SUM(p.Score) AS TotalScore
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// GROUP BY
// p.PostTypeId
// )
// SELECT
// pt.Name AS PostTypeName,
// ps.PostCount,
// ps.AverageUserReputation,
// ps.TotalViews,
// ps.TotalScore
// FROM
// PostStats ps
// JOIN
// PostTypes pt ON ps.PostTypeId = pt.Id
// ORDER BY
// ps.PostCount DESC;
fn q12804(db: &'static So) -> String {
    let base = owned(db);
    rows(owner_type_stats(db, &base).iter().map(|&(_, k, (n, r, s, vn, vs), _)| row(vec![V::S(k), V::I(n), avg(r, n), nullable(vs, vn), V::I(s)])))
}

// WITH PostStats AS (
// SELECT
// p.PostTypeId,
// COUNT(p.Id) AS PostCount,
// AVG(p.Score) AS AvgScore,
// SUM(p.ViewCount) AS TotalViewCount,
// AVG(u.Reputation) AS AvgUserReputation
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.PostTypeId
// )
// SELECT
// pt.Name AS PostTypeName,
// ps.PostCount,
// ps.AvgScore,
// ps.TotalViewCount,
// ps.AvgUserReputation
// FROM
// PostStats ps
// JOIN
// PostTypes pt ON ps.PostTypeId = pt.Id
// ORDER BY
// ps.PostCount DESC;
fn q13931(db: &'static So) -> String {
    let base = owned(db).with((&db.post.creation_date).ge(date(2023, 1, 1)));
    rows(owner_type_stats(db, &base).iter().map(|&(_, k, (n, r, s, vn, vs), _)| row(vec![V::S(k), V::I(n), avg(s, n), nullable(vs, vn), avg(r, n)])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(u.Reputation) AS AverageUserReputation,
// SUM(p.Score) AS TotalScore,
// SUM(p.ViewCount) AS TotalViews,
// COUNT(DISTINCT p.OwnerUserId) AS UniqueUsers
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
// TotalPosts DESC;
fn q14836(db: &'static So) -> String {
    let Post { post_type, score, view_count, owner_user, owner_user_id, .. } = &db.post;
    let base = owned(db).with((&db.post.creation_date).ge(ts(2024, 9, 1, 12, 34, 56)));
    let name = post_type.select(&db.post_type.name);
    let main = (&base).group_by(&name).select(score.and(view_count.opt()).and(owner_user.select(&db.user.reputation))).fold(
        (0i64, 0i64, 0i64, 0i64, 0i64),
        |(n, r, s, vn, vs), ((x, w), rep)| (n + 1, r + rep, s + x, vn + w.is_some() as i64, vs + w.unwrap_or(0)),
    );
    let uniq = (&base).group_by(&name).select(owner_user_id).count_distinct();
    let mut v = Vec::new();
    main.and(&uniq).drive(|k, (a, u)| v.push((k, a, u)));
    v.sort_by(|a, b| b.1.0.cmp(&a.1.0));
    rows(v.iter().map(|&(k, (n, r, s, vn, vs), u)| row(vec![V::S(k), V::I(n), avg(r, n), V::I(s), nullable(vs, vn), V::I(u)])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.OwnerUserId IS NOT NULL THEN 1 ELSE 0 END) AS UserCreatedPosts,
// AVG(p.Score) AS AverageScore,
// SUM(CASE WHEN p.AnswerCount > 0 THEN 1 ELSE 0 END) AS QuestionsWithAnswers,
// SUM(p.ViewCount) AS TotalViews,
// SUM(p.CommentCount) AS TotalComments,
// SUM(p.FavoriteCount) AS TotalFavorites
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q12148(db: &'static So) -> String {
    let Post { post_type, score, view_count, owner_user_id, answer_count, comment_count, favorite_count, .. } = &db.post;
    let mut v = Vec::new();
    db.post
        .group_by(post_type.select(&db.post_type.name))
        .select(score.and(view_count.opt()).and(owner_user_id.opt()).and(answer_count.opt()).and(comment_count).and(favorite_count.opt()))
        .fold([0i64; 9], |a, (((((s, w), o), ac), cc), f)| {
            [a[0] + 1, a[1] + o.is_some() as i64, a[2] + s, a[3] + ac.is_some_and(|x| x > 0) as i64, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0), a[6] + cc, a[7] + f.is_some() as i64, a[8] + f.unwrap_or(0)]
        })
        .drive(|k, a| v.push((k, a)));
    v.sort_by(|a, b| b.1[0].cmp(&a.1[0]));
    rows(v.iter().map(|&(k, a)| {
        row(vec![V::S(k), V::I(a[0]), V::I(a[1]), avg(a[2], a[0]), V::I(a[3]), nullable(a[5], a[4]), V::I(a[6]), nullable(a[8], a[7])])
    }))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// COUNT(c.Id) AS TotalComments,
// COUNT(v.Id) AS TotalVotes,
// SUM(u.Reputation) AS TotalUserReputation,
// AVG(p.Score) AS AveragePostScore,
// AVG(p.ViewCount) AS AverageViewCount
// FROM
// Posts p
// LEFT JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q12463(db: &'static So) -> String {
    let Post { post_type, score, view_count, owner_user, .. } = &db.post;
    let mut v = Vec::new();
    db.post
        .group_by(post_type.select(&db.post_type.name).opt())
        .select(score.and(view_count.opt()).and(comments_of(db).opt()).and(votes_of(db).opt()).and(owner_user.select(&db.user.reputation).opt()))
        .fold([0i64; 8], |a, ((((s, w), c), x), r)| {
            [a[0] + 1, a[1] + c.is_some() as i64, a[2] + x.is_some() as i64, a[3] + r.is_some() as i64, a[4] + r.unwrap_or(0), a[5] + s, a[6] + w.is_some() as i64, a[7] + w.unwrap_or(0)]
        })
        .drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&(k, a)| row(vec![ostr(k), V::I(a[0]), V::I(a[1]), V::I(a[2]), nullable(a[4], a[3]), avg(a[5], a[0]), avg(a[7], a[6])])))
}

// WITH QuestionStats AS (
// SELECT
// p.Id AS QuestionId,
// p.Score AS QuestionScore,
// p.ViewCount AS QuestionViews,
// u.Reputation AS UserReputation
// FROM
// Posts AS p
// JOIN
// Users AS u ON p.OwnerUserId = u.Id
// WHERE
// p.PostTypeId = 1
// )
// SELECT
// COUNT(*) AS TotalQuestions,
// AVG(QuestionScore) AS AverageScore,
// AVG(QuestionViews) AS AverageViews,
// AVG(UserReputation) AS AverageUserReputation
// FROM
// QuestionStats;
fn q10532(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, owner_user, .. } = &db.post;
    let (n, s, vn, vs, r) = owned(db)
        .with(post_type_id.eq(1))
        .select(score.and(view_count.opt()).and(owner_user.select(&db.user.reputation)))
        .fold_flat((0i64, 0i64, 0i64, 0i64, 0i64), |(n, s, vn, vs, r), ((x, w), rep)| {
            (n + 1, s + x, vn + w.is_some() as i64, vs + w.unwrap_or(0), r + rep)
        });
    row(vec![V::I(n), avg(s, n), avg(vs, vn), avg(r, n)])
}

// SELECT
// u.DisplayName AS UserName,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(CASE WHEN pt.Name = 'Question' THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN pt.Name = 'Answer' THEN 1 ELSE 0 END) AS AnswerCount,
// AVG(u.Reputation) AS AverageReputation,
// MAX(p.CreationDate) AS LatestPostDate
// FROM
// Users u
// JOIN
// Posts p ON u.Id = p.OwnerUserId
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// GROUP BY
// u.DisplayName, u.Reputation
// ORDER BY
// PostCount DESC
// LIMIT 10;
fn q11614(db: &'static So) -> String {
    let Post { post_type, owner_user, creation_date, .. } = &db.post;
    let key = owner_user.select((&db.user.display_name).and(&db.user.reputation));
    let mut v = Vec::new();
    db.post
        .group_by(key)
        .select(post_type.select(&db.post_type.name).and(creation_date))
        .fold((0i64, 0i64, 0i64, i64::MIN), |(n, q, a, mx), (t, c)| (n + 1, q + (t == "Question") as i64, a + (t == "Answer") as i64, mx.max(c)))
        .drive(|k, a| v.push((k, a)));
    v.sort_by(|a, b| b.1.0.cmp(&a.1.0));
    rows(v.iter().take(10).map(|&((dn, r), (n, q, a, mx))| row(vec![V::S(dn), V::I(n), V::I(q), V::I(a), V::F(r as f64), V::T(mx)])))
}

// SELECT
// u.DisplayName AS UserName,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN p.PostTypeId = 10 THEN 1 ELSE 0 END) AS TotalClosedPosts,
// AVG(p.Score) AS AverageScore,
// MAX(p.ViewCount) AS MaxViews,
// MIN(p.CreationDate) AS EarliestPostDate,
// MAX(p.LastActivityDate) AS RecentActivityDate
// FROM
// Users u
// JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.DisplayName
// ORDER BY
// TotalPosts DESC
// LIMIT 10;
fn q12897(db: &'static So) -> String {
    let Post { post_type_id, owner_user, creation_date, score, view_count, last_activity_date, .. } = &db.post;
    let mut v = Vec::new();
    db.post
        .group_by(owner_user.select(&db.user.display_name))
        .select(post_type_id.and(score).and(view_count.opt()).and(creation_date).and(last_activity_date))
        .fold([0, 0, 0, 0, 0, i64::MIN, 0, i64::MAX, i64::MIN], |a: [i64; 9], ((((t, s), w), c), la)| {
            [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (t == 10) as i64, a[4] + s, w.map_or(a[5], |x| a[5].max(x)), a[6] + w.is_some() as i64, a[7].min(c), a[8].max(la)]
        })
        .drive(|k, a| v.push((k, a)));
    v.sort_by(|a, b| b.1[0].cmp(&a.1[0]));
    rows(v.iter().take(10).map(|&(k, a)| {
        row(vec![V::S(k), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), avg(a[4], a[0]), omax(a[5], a[6]), V::T(a[7]), V::T(a[8])])
    }))
}

// WITH BenchmarkedQueries AS (
// SELECT
// PH.PostHistoryTypeId,
// COUNT(*) AS Count,
// MIN(PH.CreationDate) AS FirstOccurrence,
// MAX(PH.CreationDate) AS LastOccurrence,
// MAX(PH.CreationDate) - MIN(PH.CreationDate) AS Duration
// FROM
// PostHistory PH
// JOIN
// Posts P ON PH.PostId = P.Id
// WHERE
// PH.CreationDate > '2023-01-01'
// GROUP BY
// PH.PostHistoryTypeId
// )
// SELECT
// PHT.Name,
// BQ.Count,
// BQ.FirstOccurrence,
// BQ.LastOccurrence,
// BQ.Duration
// FROM
// BenchmarkedQueries BQ
// JOIN
// PostHistoryTypes PHT ON BQ.PostHistoryTypeId = PHT.Id
// ORDER BY
// BQ.Count DESC;
fn q13769(db: &'static So) -> String {
    let PostHistory { post_history_type, creation_date, post, .. } = &db.post_history;
    let mut v = Vec::new();
    db.post_history
        .with(creation_date.gt(date(2023, 1, 1)).and(post))
        .group_by(post_history_type)
        .select(creation_date)
        .fold((0i64, i64::MAX, i64::MIN), |(n, mn, mx), d| (n + 1, mn.min(d), mx.max(d)))
        .drive(|t, a| v.push((db.post_history_type.name.get(t).unwrap(), a)));
    v.sort_by(|a, b| b.1.0.cmp(&a.1.0));
    rows(v.iter().map(|&(k, (n, mn, mx))| row(vec![V::S(k), V::I(n), V::T(mn), V::T(mx), V::Iv(mx - mn)])))
}

// Posts joined to PostHistory (an inner join) with a history-type filter:
// the group holds a history column, so the (post, history) rows are
// materialised and grouped from there.
type PH = MatSet<(Id<Post>, Id<PostHistory>)>;

fn post_history_rows<Q>(db: &'static So, base: Q, types: &'static [i64]) -> PH
where
    Q: Drive<D = Id<Post>, R = Id<Post>>,
{
    let t = (&db.post_history.post_history_type_id).filt(move |t| types.contains(&t)).map(|_| ());
    base.select(Ident::<Post>::new().and(history_of(db).select(Ident::<PostHistory>::new().and(t)).map(|(h, _)| h))).collect()
}

// SELECT
// u.DisplayName AS UserName,
// p.Title AS PostTitle,
// ph.CreationDate AS HistoryDate,
// p.Score AS PostScore,
// COUNT(c.Id) AS CommentCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// JOIN
// PostHistory ph ON p.Id = ph.PostId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// WHERE
// ph.PostHistoryTypeId IN (1, 2, 4)
// GROUP BY
// u.DisplayName,
// p.Title,
// ph.CreationDate,
// p.Score
// ORDER BY
// ph.CreationDate DESC;
fn q17434(db: &'static So) -> String {
    let j = post_history_rows(db, owned(db), &[1, 2, 4]);
    let post_of = (&j).map(|(p, _)| p);
    let hist_of = (&j).map(|(_, h)| h);
    let Post { title, owner_user, score, .. } = &db.post;
    let key = (&post_of).select(owner_user.select(&db.user.display_name).and(title.opt()).and(score)).and((&hist_of).select(&db.post_history.creation_date));
    let mut v = Vec::new();
    (&j).group_by(key)
        .select((&post_of).select(comments_of(db).opt()))
        .fold(0i64, |a, c| a + c.is_some() as i64)
        .drive(|k, n| v.push((k, n)));
    rows(v.iter().map(|&((((dn, t), s), hd), n)| row(vec![V::S(dn), ostr(t), V::T(hd), V::I(s), V::I(n)])))
}

fn history_24(db: &'static So) -> Vec<(((((Str, Option<Str>), i64), i64), (i64, Option<i64>)), i64)> {
    let j = post_history_rows(db, owned(db), &[24]);
    let post_of = (&j).map(|(p, _)| p);
    let hist_of = (&j).map(|(_, h)| h);
    let Post { title, owner_user, score, creation_date, view_count, .. } = &db.post;
    let key = (&post_of)
        .select(owner_user.select(&db.user.display_name).and(title.opt()).and(creation_date))
        .and((&hist_of).select(&db.post_history.creation_date))
        .and((&post_of).select(score.and(view_count.opt())));
    let mut v = Vec::new();
    (&j).group_by(key)
        .select((&post_of).select(comments_of(db).opt()))
        .fold(0i64, |a, c| a + c.is_some() as i64)
        .drive(|k, n| v.push((k, n)));
    v.sort_by(|a, b| b.0.0.1.cmp(&a.0.0.1));
    v
}

// SELECT
// U.DisplayName AS UserName,
// P.Title AS PostTitle,
// P.CreationDate AS PostCreationDate,
// PH.CreationDate AS HistoryCreationDate,
// P.Score AS PostScore,
// COUNT(c.Id) AS CommentCount
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// JOIN
// PostHistory PH ON P.Id = PH.PostId
// LEFT JOIN
// Comments c ON P.Id = c.PostId
// WHERE
// PH.PostHistoryTypeId = 24
// GROUP BY
// U.DisplayName, P.Title, P.CreationDate, PH.CreationDate, P.Score
// ORDER BY
// PH.CreationDate DESC
// LIMIT 10;
fn q15153(db: &'static So) -> String {
    rows(history_24(db).iter().take(10).map(|&(((((dn, t), c), hd), (s, _)), n)| row(vec![V::S(dn), ostr(t), V::T(c), V::T(hd), V::I(s), V::I(n)])))
}

// SELECT
// u.DisplayName AS UserDisplayName,
// p.Title AS PostTitle,
// p.CreationDate AS PostCreationDate,
// ph.CreationDate AS HistoryCreationDate,
// p.Score AS PostScore,
// p.ViewCount AS PostViewCount,
// COUNT(c.Id) AS CommentCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// JOIN
// PostHistory ph ON p.Id = ph.PostId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// WHERE
// ph.PostHistoryTypeId = 24
// GROUP BY
// u.DisplayName, p.Title, p.CreationDate, ph.CreationDate, p.Score, p.ViewCount
// ORDER BY
// ph.CreationDate DESC;
fn q19065(db: &'static So) -> String {
    rows(history_24(db).iter().map(|&(((((dn, t), c), hd), (s, w)), n)| row(vec![V::S(dn), ostr(t), V::T(c), V::T(hd), V::I(s), oint(w), V::I(n)])))
}

// SELECT
// U.DisplayName AS UserDisplayName,
// P.Title AS PostTitle,
// P.CreationDate AS PostCreationDate,
// PH.CreationDate AS HistoryCreationDate,
// P.Body AS PostBody,
// COUNT(C.ID) AS CommentCount
// FROM
// Users U
// JOIN
// Posts P ON U.Id = P.OwnerUserId
// JOIN
// PostHistory PH ON P.Id = PH.PostId
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// WHERE
// PH.PostHistoryTypeId IN (4, 6)
// GROUP BY
// U.DisplayName, P.Title, P.CreationDate, PH.CreationDate, P.Body
// ORDER BY
// PH.CreationDate DESC;
fn q17120(db: &'static So) -> String {
    let j = post_history_rows(db, owned(db), &[4, 6]);
    let post_of = (&j).map(|(p, _)| p);
    let hist_of = (&j).map(|(_, h)| h);
    let Post { title, owner_user, creation_date, body, .. } = &db.post;
    let key = (&post_of)
        .select(owner_user.select(&db.user.display_name).and(title.opt()).and(creation_date).and(body))
        .and((&hist_of).select(&db.post_history.creation_date));
    let mut v = Vec::new();
    (&j).group_by(key)
        .select((&post_of).select(comments_of(db).opt()))
        .fold(0i64, |a, c| a + c.is_some() as i64)
        .drive(|k, n| v.push((k, n)));
    rows(v.iter().map(|&(((((dn, t), c), b), hd), n)| row(vec![V::S(dn), ostr(t), V::T(c), V::T(hd), V::S(b), V::I(n)])))
}

// `COUNT(C)` counts the alias, not a column: DuckDB gives an unmatched LEFT
// JOIN a struct of NULLs rather than a NULL struct, so it counts every joined
// row, comment or not.
//
// SELECT
// U.DisplayName AS UserName,
// P.Title AS PostTitle,
// PH.CreationDate AS PostHistoryDate,
// PH.Comment AS EditComment,
// COUNT(C) AS CommentCount,
// SUM(V.BountyAmount) AS TotalBounty,
// AVG(P.Score) AS AverageScore
// FROM
// Users U
// JOIN
// Posts P ON U.Id = P.OwnerUserId
// JOIN
// PostHistory PH ON P.Id = PH.PostId
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// WHERE
// PH.CreationDate BETWEEN '2023-01-01' AND '2023-12-31'
// AND PH.PostHistoryTypeId IN (4, 5, 6)
// GROUP BY
// U.DisplayName, P.Title, PH.CreationDate, PH.Comment
// ORDER BY
// U.DisplayName, PH.CreationDate DESC;
fn q10316(db: &'static So) -> String {
    let PostHistory { post_history_type_id, creation_date, comment, .. } = &db.post_history;
    let j: PH = owned(db)
        .select(Ident::<Post>::new().and(history_of(db).select(
            Ident::<PostHistory>::new().and(post_history_type_id.in_v(vec![4, 5, 6]).and(creation_date.between(date(2023, 1, 1), date(2023, 12, 31)))),
        )))
        .map(|(p, (h, _))| (p, h))
        .collect();
    let post_of = (&j).map(|(p, _)| p);
    let hist_of = (&j).map(|(_, h)| h);
    let Post { title, owner_user, score, .. } = &db.post;
    let key = (&post_of)
        .select(owner_user.select(&db.user.display_name).and(title.opt()))
        .and((&hist_of).select(creation_date.and(comment.opt())));
    let mut v = Vec::new();
    (&j).group_by(key)
        .select((&post_of).select(score.and(comments_of(db).opt()).and(votes_of(db).select((&db.vote.bounty_amount).opt()).opt())))
        .fold((0i64, 0i64, 0i64, 0i64), |(n, bn, bs, s), ((x, _), b)| {
            let b = b.flatten();
            (n + 1, bn + b.is_some() as i64, bs + b.unwrap_or(0), s + x)
        })
        .drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&(((dn, t), (hd, hc)), (n, bn, bs, s))| {
        row(vec![V::S(dn), ostr(t), V::T(hd), ostr(hc), V::I(n), nullable(bs, bn), avg(s, n)])
    }))
}

// SELECT
// P.Id AS PostId,
// P.Title,
// U.DisplayName AS OwnerDisplayName,
// P.CreationDate,
// P.Score,
// P.ViewCount,
// P.AnswerCount,
// P.CommentCount,
// P.FavoriteCount,
// P.LastActivityDate,
// PH.UserDisplayName AS LastEditorDisplayName,
// PH.CreationDate AS LastEditDate
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
fn q13421(db: &'static So) -> String {
    let mut v = Vec::new();
    owned(db)
        .with((&db.post.post_type_id).eq(1))
        .select(history_of(db).opt())
        .drive(|p, h| v.push((cd(db, p), p, h)));
    v.sort_by_key(|x| x.0);
    rows(v.iter().take(100).map(|&(_, p, h)| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "score", "views", "answers", "comments", "favorites", "activity"]);
        f.push(ostr(h.and_then(|h| db.post_history.user_display_name.get(h))));
        f.push(ots(h.map(|h| db.post_history.creation_date.get(h).unwrap())));
        row(f)
    }))
}

// CAST(ph.Comment AS INT) = ct.Id, as a relation history -> reason name.
fn close_reasons(db: &'static So) -> HashIdx<i64, Str> {
    (&db.close_reason_type.origid).inv().select(&db.close_reason_type.name).collect()
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// u.DisplayName AS OwnerDisplayName,
// ct.Name AS CloseReason
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId AND ph.PostHistoryTypeId = 10
// LEFT JOIN
// CloseReasonTypes ct ON CAST(ph.Comment AS INTEGER) = ct.Id
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, p.CreationDate, u.DisplayName, ct.Name
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q16677(db: &'static So) -> String {
    let PostHistory { post_history_type_id, comment, .. } = &db.post_history;
    let reason = close_reasons(db);
    let closes = db.post_history.with(post_history_type_id.eq(10)).select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason).opt());
    let groups: MatSet<(Id<Post>, Option<Str>)> =
        db.post.with((&db.post.post_type_id).eq(1)).select(Ident::<Post>::new().and(history_of(db).select(closes).opt())).map(|(p, r)| (p, r.flatten())).collect();
    let mut v = Vec::new();
    (&groups).drive(|_, (p, r)| v.push((cd(db, p), p, r)));
    v.sort_by_key(|x| x.0);
    rows(v.iter().take(10).map(|&(_, p, r)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.push(ostr(r));
        row(f)
    }))
}

// SELECT
// p.Title,
// p.CreationDate,
// u.DisplayName AS OwnerDisplayName,
// p.Score,
// p.ViewCount,
// ct.Name AS CloseReason
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId AND ph.PostHistoryTypeId IN (10, 11)
// LEFT JOIN
// CloseReasonTypes ct ON CAST(ph.Comment AS INT) = ct.Id
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Title, p.CreationDate, u.DisplayName, p.Score, p.ViewCount, ct.Name
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q16250(db: &'static So) -> String {
    let PostHistory { post_history_type_id, comment, .. } = &db.post_history;
    let Post { title, creation_date, owner_user, score, view_count, post_type_id, .. } = &db.post;
    let reason = close_reasons(db);
    let closes = db.post_history.with(post_history_type_id.in_v(vec![10, 11])).select(comment.flat_map(|s: Str| s.trim().parse::<i64>().ok()).select(&reason).opt());
    let groups: MatSet<_> = owned(db)
        .with(post_type_id.eq(1))
        .select(title.opt().and(creation_date).and(owner_user.select(&db.user.display_name)).and(score).and(view_count.opt()).and(history_of(db).select(closes).opt()))
        .map(|(k, r)| (k, r.flatten()))
        .collect();
    let mut v = Vec::new();
    (&groups).drive(|_, x| v.push(x));
    v.sort_by(|a, b| b.0.0.0.0.1.cmp(&a.0.0.0.0.1));
    rows(v.iter().take(10).map(|&(((((t, c), dn), s), w), r)| row(vec![ostr(t), V::T(c), V::S(dn), V::I(s), oint(w), ostr(r)])))
}

// SELECT
// p.Id AS PostID,
// p.Title,
// p.CreationDate AS PostCreationDate,
// p.Score,
// p.ViewCount,
// c.Id AS CommentID,
// c.Text AS CommentText,
// c.CreationDate AS CommentCreationDate,
// u.DisplayName AS UserDisplayName,
// u.Reputation AS UserReputation
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.CreationDate >= '2023-01-01'
// ORDER BY
// p.CreationDate DESC, c.CreationDate DESC
// LIMIT 1000;
fn q11307(db: &'static So) -> String {
    let mut v = Vec::new();
    owned(db)
        .with((&db.post.creation_date).ge(date(2023, 1, 1)))
        .select(comments_of(db).opt())
        .drive(|p, c| v.push((cd(db, p), c.map(|c| db.comment.creation_date.get(c).unwrap()), p, c)));
    v.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| desc_nulls_last(a.1, b.1)));
    rows(v.iter().take(1000).map(|&(_, ccd, p, c)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([oint(c.map(|c| db.comment.origid.get(c).unwrap())), ostr(c.map(|c| db.comment.text.get(c).unwrap())), ots(ccd)]);
        f.extend(post_fields(db, p, &["owner", "rep"]));
        row(f)
    }))
}

// SELECT
// u.Id AS UserId,
// u.Reputation,
// u.CreationDate AS UserCreationDate,
// p.Id AS PostId,
// p.Title,
// p.CreationDate AS PostCreationDate,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// u.Id, u.Reputation, u.CreationDate, p.Id, p.Title, p.CreationDate
// ORDER BY
// u.Reputation DESC, CommentCount DESC, VoteCount DESC;
fn q14481(db: &'static So) -> String {
    let j: MatSet<(Id<User>, Option<Id<Post>>)> = db.user.select(Ident::<User>::new().and(posts_of(db).opt())).collect();
    let post_of = (&j).flat_map(|(_, p)| p);
    let mut v = Vec::new();
    (&j).group_by(Same::new())
        .select((&post_of).select(comments_of(db).opt().and(votes_of(db).opt())).opt())
        .fold((0i64, 0i64), |(c, x), r| match r {
            Some((ci, vi)) => (c + ci.is_some() as i64, x + vi.is_some() as i64),
            None => (c, x),
        })
        .drive(|(u, p), (c, x)| v.push((u, p, c, x)));
    rows(v.iter().map(|&(u, p, c, x)| {
        let User { origid, reputation, creation_date, .. } = &db.user;
        let mut f = vec![V::I(origid.get(u).unwrap()), V::I(reputation.get(u).unwrap()), V::T(creation_date.get(u).unwrap())];
        match p {
            Some(p) => f.extend(post_fields(db, p, &["id", "title", "created"])),
            None => f.extend([V::Null, V::Null, V::Null]),
        }
        f.extend([V::I(c), V::I(x)]);
        row(f)
    }))
}

// GROUP BY p.Id, ..., v.VoteTypeId: a post and the type of each vote it got.
fn post_vote_types<Q>(db: &'static So, base: Q) -> Vec<((Id<Post>, Option<i64>), i64)>
where
    Q: Drive<D = Id<Post>, R = Id<Post>>,
{
    let j: MatSet<(Id<Post>, Option<Id<Vote>>)> = base.select(Ident::<Post>::new().and(votes_of(db).opt())).collect();
    let post_of = (&j).map(|(p, _)| p);
    let vote_of = (&j).flat_map(|(_, v)| v);
    let mut v = Vec::new();
    (&j).group_by((&post_of).and((&vote_of).select(&db.vote.vote_type_id).opt()))
        .select((&vote_of).opt())
        .fold(0i64, |a, x| a + x.is_some() as i64)
        .drive(|k, n| v.push((k, n)));
    v
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
// u.Reputation AS OwnerReputation,
// v.VoteTypeId,
// COUNT(v.Id) AS VoteCount
// FROM
// Posts p
// INNER JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, u.DisplayName, u.Reputation, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, v.VoteTypeId
// ORDER BY
// p.Score DESC, p.CreationDate DESC;
fn q10965(db: &'static So) -> String {
    rows(post_vote_types(db, owned(db)).iter().map(|&((p, t), n)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "owner", "rep"]);
        f.extend([oint(t), V::I(n)]);
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate AS PostCreationDate,
// p.ViewCount,
// p.Score,
// p.AnswerCount,
// p.CommentCount,
// u.DisplayName AS OwnerDisplayName,
// u.Reputation AS OwnerReputation,
// v.VoteTypeId,
// COUNT(v.Id) AS VoteCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score,
// p.AnswerCount, p.CommentCount, u.DisplayName,
// u.Reputation, v.VoteTypeId
// ORDER BY
// p.CreationDate DESC;
fn q11312(db: &'static So) -> String {
    rows(post_vote_types(db, owned(db)).iter().map(|&((p, t), n)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "answers", "comments", "owner", "rep"]);
        f.extend([oint(t), V::I(n)]);
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate AS PostCreationDate,
// p.ViewCount,
// p.Score,
// p.AnswerCount,
// u.DisplayName AS OwnerDisplayName,
// u.Reputation AS OwnerReputation,
// vt.Name AS VoteType,
// COUNT(v.Id) AS VoteCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// VoteTypes vt ON v.VoteTypeId = vt.Id
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount,
// u.DisplayName, u.Reputation, vt.Name
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q12928(db: &'static So) -> String {
    let base = owned(db).with((&db.post.creation_date).ge(date(2023, 1, 1)));
    let j: MatSet<(Id<Post>, Option<Id<Vote>>)> = base.select(Ident::<Post>::new().and(votes_of(db).opt())).collect();
    let post_of = (&j).map(|(p, _)| p);
    let vote_of = (&j).flat_map(|(_, v)| v);
    let mut v = Vec::new();
    (&j).group_by((&post_of).and((&vote_of).select((&db.vote.vote_type).select(&db.vote_type.name)).opt()))
        .select((&vote_of).opt())
        .fold(0i64, |a, x| a + x.is_some() as i64)
        .drive(|k, n| v.push((k, n)));
    v.sort_by_key(|&((p, t), _)| (cd(db, p), db.post.origid.get(p).unwrap(), t.is_none(), t));
    rows(v.iter().take(100).map(|&((p, t), n)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "answers", "owner", "rep"]);
        f.extend([ostr(t), V::I(n)]);
        row(f)
    }))
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// p.Id AS PostId,
// p.Title,
// p.CreationDate AS PostCreationDate,
// ph.CreationDate AS PostHistoryDate,
// ph.Comment AS PostHistoryComment,
// v.CreationDate AS VoteCreationDate,
// vt.Name AS VoteTypeName
// FROM
// Users u
// JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// VoteTypes vt ON v.VoteTypeId = vt.Id
// WHERE
// u.Reputation > 1000
// ORDER BY
// u.Id, p.Id, ph.CreationDate DESC;
fn q12522(db: &'static So) -> String {
    let base = db.user.with((&db.user.reputation).gt(1000));
    let mut v = Vec::new();
    base.select(posts_of(db).select(Ident::<Post>::new().and(history_of(db).opt()).and(votes_of(db).opt())))
        .drive(|u, ((p, h), x)| v.push((u, p, h, x)));
    rows(v.iter().map(|&(u, p, h, x)| {
        let mut f = vec![V::I(db.user.origid.get(u).unwrap()), V::S(db.user.display_name.get(u).unwrap())];
        f.extend(post_fields(db, p, &["id", "title", "created"]));
        f.push(ots(h.map(|h| db.post_history.creation_date.get(h).unwrap())));
        f.push(ostr(h.and_then(|h| db.post_history.comment.get(h))));
        f.push(ots(x.map(|x| db.vote.creation_date.get(x).unwrap())));
        f.push(ostr(x.map(|x| db.vote_type.name.get(db.vote.vote_type.get(x).unwrap()).unwrap())));
        row(f)
    }))
}

// SELECT
// p.Title AS PostTitle,
// p.CreationDate AS PostCreationDate,
// p.ViewCount,
// c.Text AS CommentText,
// c.CreationDate AS CommentCreationDate,
// u.DisplayName AS CommenterDisplayName,
// v.CreationDate AS VoteCreationDate,
// vt.Name AS VoteTypeName
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// VoteTypes vt ON v.VoteTypeId = vt.Id
// LEFT JOIN
// Users u ON c.UserId = u.Id
// WHERE
// p.PostTypeId = 1
// ORDER BY
// p.CreationDate DESC,
// c.CreationDate DESC,
// v.CreationDate DESC
// LIMIT 100;
fn q11543(db: &'static So) -> String {
    let mut v = Vec::new();
    db.post
        .with((&db.post.post_type_id).eq(1))
        .select(comments_of(db).opt().and(votes_of(db).opt()))
        .drive(|p, (c, x)| {
            v.push((cd(db, p), c.map(|c| db.comment.creation_date.get(c).unwrap()), x.map(|x| db.vote.creation_date.get(x).unwrap()), p, c, x))
        });
    v.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| desc_nulls_last(a.1, b.1)).then_with(|| desc_nulls_last(a.2, b.2)));
    rows(v.iter().take(100).map(|&(_, ccd, vcd, p, c, x)| {
        let mut f = post_fields(db, p, &["title", "created", "views"]);
        f.extend([
            ostr(c.map(|c| db.comment.text.get(c).unwrap())),
            ots(ccd),
            ostr(c.and_then(|c| db.comment.user.get(c)).map(|u| db.user.display_name.get(u).unwrap())),
            ots(vcd),
            ostr(x.map(|x| db.vote_type.name.get(db.vote.vote_type.get(x).unwrap()).unwrap())),
        ]);
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate AS PostCreationDate,
// p.ViewCount,
// p.Score,
// u.DisplayName AS OwnerDisplayName,
// u.Reputation,
// t.TagName,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// JOIN
// Tags t ON p.Tags LIKE CONCAT('%', t.TagName, '%')
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= '2020-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName, u.Reputation, t.TagName
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q10486(db: &'static So) -> String {
    let mentions = tag_mentions(db);
    let post_of = (&mentions).map(|(p, _)| p);
    let rows_ = (&mentions)
        .with((&post_of).select(owned(db).with((&db.post.creation_date).ge(date(2020, 1, 1)))))
        .group_by(Same::new())
        .select((&post_of).select(comments_of(db).opt().and(votes_of(db).opt())))
        .fold((0i64, 0i64), |(c, x), (ci, vi)| (c + ci.is_some() as i64, x + vi.is_some() as i64));
    let mut v = Vec::new();
    (&rows_).drive(|(p, t), (c, x)| v.push((cd(db, p), p, t, c, x)));
    v.sort_by_key(|&(k, p, t, _, _)| (k, db.post.origid.get(p).unwrap(), db.tag.tag_name.get(t).unwrap()));
    rows(v.iter().take(100).map(|&(_, p, t, c, x)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "owner", "rep"]);
        f.extend([V::S(db.tag.tag_name.get(t).unwrap()), V::I(c), V::I(x)]);
        row(f)
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("19938", q19938),
    ("14582", q14582),
    ("11823", q11823),
    ("10896", q10896),
    ("16296", q16296),
    ("10709", q10709),
    ("13240", q13240),
    ("14462", q14462),
    ("11527", q11527),
    ("11766", q11766),
    ("14134", q14134),
    ("10016", q10016),
    ("11460", q11460),
    ("11873", q11873),
    ("14348", q14348),
    ("12524", q12524),
    ("10310", q10310),
    ("12094", q12094),
    ("14368", q14368),
    ("12242", q12242),
    ("10093", q10093),
    ("13610", q13610),
    ("13119", q13119),
    ("13050", q13050),
    ("11833", q11833),
    ("12804", q12804),
    ("13931", q13931),
    ("14836", q14836),
    ("12148", q12148),
    ("12463", q12463),
    ("10532", q10532),
    ("11614", q11614),
    ("12897", q12897),
    ("13769", q13769),
    ("17434", q17434),
    ("15153", q15153),
    ("19065", q19065),
    ("17120", q17120),
    ("10316", q10316),
    ("13421", q13421),
    ("16677", q16677),
    ("16250", q16250),
    ("11307", q11307),
    ("14481", q14481),
    ("10965", q10965),
    ("11312", q11312),
    ("12928", q12928),
    ("12522", q12522),
    ("11543", q11543),
    ("10486", q10486),
];
