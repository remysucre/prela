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

fn score_then_newest(db: &'static So, p: Id<Post>) -> (Reverse<i64>, Reverse<i64>) {
    (Reverse(db.post.score.get(p).unwrap()), newest(db, p))
}

// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// U.DisplayName AS OwnerName,
// P.AnswerCount,
// P.CommentCount,
// P.ViewCount,
// (SELECT COUNT(*) FROM Votes V WHERE V.PostId = P.Id AND V.VoteTypeId = 2) AS UpVotes,
// (SELECT COUNT(*) FROM Votes V WHERE V.PostId = P.Id AND V.VoteTypeId = 3) AS DownVotes,
// (SELECT COUNT(*) FROM Comments C WHERE C.PostId = P.Id) AS TotalComments,
// (SELECT COUNT(*) FROM PostHistory PH WHERE PH.PostId = P.Id) AS EditHistoryCount
// FROM
// Posts P
// INNER JOIN
// Users U ON P.OwnerUserId = U.Id
// WHERE
// P.PostTypeId = 1
// ORDER BY
// P.CreationDate DESC
// LIMIT 100;
fn q11735(db: &'static So) -> String {
    let mut v = Vec::new();
    questions(db).select(votes_of_type(db, 2).and(votes_of_type(db, 3)).and(comments_per_post(db)).and(history_per_post(db))).drive(|p, a| v.push((newest(db, p), p, a)));
    v.sort_by_key(|x| x.0);
    rows(v.iter().take(100).map(|&(_, p, (((u, d), c), h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner", "answers", "comments", "views"]);
        f.extend([V::I(u), V::I(d), V::I(c), V::I(h)]);
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
// p.CommentCount,
// p.FavoriteCount,
// u.DisplayName AS OwnerDisplayName,
// (SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id) AS TotalVotes,
// (SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id) AS TotalComments,
// pt.Name AS PostType,
// (SELECT COUNT(*) FROM PostHistory ph WHERE ph.PostId = p.Id) AS HistoryCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// WHERE
// p.CreationDate >= '2023-01-01'
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q10300(db: &'static So) -> String {
    let mut v = Vec::new();
    owned_since(db, date(2023, 1, 1)).select(votes_per_post(db).and(comments_per_post(db)).and(history_per_post(db))).drive(|p, a| v.push((newest(db, p), p, a)));
    v.sort_by_key(|x| x.0);
    rows(v.iter().take(100).map(|&(_, p, ((x, c), h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "favorites", "owner"]);
        f.extend([V::I(x), V::I(c)]);
        f.extend(post_fields(db, p, &["type"]));
        f.push(V::I(h));
        row(f)
    }))
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
// p.PostTypeId IN (1, 2)
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q14153(db: &'static So) -> String {
    let base = owned(db).with((&db.post.post_type_id).in_v(vec![1, 2]));
    stats_rows(db, stats_with(db, base, "cv", &[], &[]), |p, _| newest(db, p), 100, &["id", "title", "created", "score", "views", "owner", "#cx", "#vx", "#up", "#down"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// pt.Name AS PostType,
// u.DisplayName AS Author,
// COUNT(v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// p.CreationDate,
// p.LastActivityDate,
// p.Score,
// p.ViewCount
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.Title, pt.Name, u.DisplayName, p.CreationDate, p.LastActivityDate, p.Score, p.ViewCount
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q12085(db: &'static So) -> String {
    stats_rows(db, stats_with(db, owned(db), "v", &[], &[]), |p, _| newest(db, p), 100, &["id", "title", "type", "owner", "#vx", "#up", "#down", "created", "activity", "score", "views"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// u.DisplayName AS OwnerDisplayName,
// COUNT(v.Id) AS VoteCount,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN 1 ELSE 0 END) AS CloseReopenCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// WHERE
// p.CreationDate >= '2021-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName
// ORDER BY
// p.ViewCount DESC;
fn q14267(db: &'static So) -> String {
    let v = stats_with(db, owned_since(db, date(2021, 1, 1)), "vch", &[], &[]);
    stats_rows(db, v, |_, _| 0, 0, &["id", "title", "created", "views", "score", "owner", "#vx", "#cx", "#h1011"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.Body,
// p.CreationDate,
// u.DisplayName AS OwnerDisplayName,
// u.Reputation AS OwnerReputation,
// COUNT(v.Id) AS VoteCount,
// COUNT(c.Id) AS CommentCount,
// COUNT(CASE WHEN p.PostTypeId = 1 THEN a.Id END) AS AnswerCount
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Posts a ON p.Id = a.ParentId
// WHERE
// p.CreationDate >= DATE '2020-01-01'
// GROUP BY
// p.Id, p.Title, p.Body, p.CreationDate, u.DisplayName, u.Reputation
// ORDER BY
// p.CreationDate DESC;
fn q10717(db: &'static So) -> String {
    let v = stats_with(db, since(db, date(2020, 1, 1)), "vca", &[], &[]);
    rows(v.iter().map(|(p, s, _)| {
        let mut f = stat_fields(db, *p, s, &["id", "title", "body", "created", "owner", "rep", "#vx", "#cx"]);
        f.push(V::I(if db.post.post_type_id.get(*p).unwrap() == 1 { s.ax } else { 0 }));
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate AS PostCreationDate,
// p.ViewCount,
// p.Score,
// pt.Name AS PostType,
// u.DisplayName AS OwnerDisplayName,
// COUNT(v.Id) AS VoteCount,
// p.AnswerCount,
// p.CommentCount
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= '2023-10-01 12:34:56'::timestamp - INTERVAL '1 YEAR'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, pt.Name, u.DisplayName,
// p.AnswerCount, p.CommentCount
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q14551(db: &'static So) -> String {
    stats_rows(db, stats_with(db, owned_since(db, ts(2022, 10, 1, 12, 34, 56)), "v", &[], &[]), |p, _| newest(db, p), 100, &["id", "title", "created", "views", "score", "type", "owner", "#vx", "answers", "comments"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// u.DisplayName AS OwnerDisplayName,
// COUNT(v.Id) AS TotalVotes,
// SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes,
// p.ViewCount,
// p.Score,
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
// p.Id, p.Title, p.CreationDate, u.DisplayName, p.ViewCount, p.Score, p.AnswerCount, p.CommentCount
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q13727(db: &'static So) -> String {
    stats_rows(db, stats_with(db, owned(db), "v", &[], &[]), |p, _| newest(db, p), 100, &["id", "title", "created", "owner", "#vx", "#upn", "#downn", "views", "score", "answers", "comments"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// COUNT(c.Id) AS CommentCount,
// COUNT(DISTINCT v.UserId) AS VoteCount,
// COUNT(DISTINCT b.Id) AS BadgeCount,
// u.DisplayName AS OwnerDisplayName,
// u.Reputation AS OwnerReputation
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// WHERE
// p.CreationDate >= '2020-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName, u.Reputation
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q11636(db: &'static So) -> String {
    let voters = per_post_distinct(db, votes_of(db).select(&db.vote.user_id));
    let badges = per_post_distinct(db, (&db.post.owner_user).select(badges_of(db)));
    stats_rows(db, stats_with(db, since(db, date(2020, 1, 1)), "cvb", &[], &[&voters, &badges]), |p, _| newest(db, p), 100, &["id", "title", "created", "score", "views", "#cx", "#d0", "#d1", "owner", "rep"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes,
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
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, u.Reputation, u.DisplayName
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q11133(db: &'static So) -> String {
    let base = db.post.with((&db.post.post_type_id).eq(1));
    stats_rows(db, stats_with(db, base, "cv", &[], &[]), |p, _| newest(db, p), 100, &["id", "title", "created", "views", "#cx", "#vx", "#up", "#down", "rep", "owner"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// u.Reputation AS OwnerReputation,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// CASE
// WHEN p.PostTypeId = 1 THEN 'Question'
// WHEN p.PostTypeId = 2 THEN 'Answer'
// ELSE 'Other'
// END AS PostType
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.Reputation, p.PostTypeId
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q12010(db: &'static So) -> String {
    let v = stats_with(db, owned_since(db, date(2023, 1, 1)), "cv", &[], &[]);
    let mut v: Vec<_> = v.into_iter().map(|(p, s, _)| (newest(db, p), p, s)).collect();
    v.sort_by_key(|x| x.0);
    rows(v.iter().take(100).map(|(_, p, s)| {
        let mut f = stat_fields(db, *p, s, &["id", "title", "created", "score", "views", "rep", "#cx", "#vx"]);
        f.push(V::S(match db.post.post_type_id.get(*p).unwrap() {
            1 => "Question",
            2 => "Answer",
            _ => "Other",
        }));
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate AS PostCreationDate,
// p.ViewCount,
// p.Score,
// u.Id AS UserId,
// u.DisplayName AS UserDisplayName,
// u.Reputation,
// COUNT(c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
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
// p.CreationDate >= '2020-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score,
// u.Id, u.DisplayName, u.Reputation
// ORDER BY
// p.CreationDate DESC;
fn q11487(db: &'static So) -> String {
    let votes = per_post_distinct(db, votes_of(db));
    stats_rows(db, stats_with(db, owned_since(db, date(2020, 1, 1)), "cvb", &[], &[&votes]), |_, _| 0, 0, &["id", "title", "created", "views", "score", "uid", "owner", "rep", "#cx", "#d0", "bmax"])
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
// SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes
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
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName
// ORDER BY
// p.Score DESC, p.CreationDate DESC
// LIMIT 100;
fn q13053(db: &'static So) -> String {
    stats_rows(db, stats_with(db, owned(db), "cv", &[], &[]), |p, _| score_then_newest(db, p), 100, &["id", "title", "created", "score", "views", "owner", "#cx", "#vx", "#upn", "#downn"])
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
// AVG(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS AverageUpVotes,
// AVG(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS AverageDownVotes
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName
// ORDER BY
// p.Score DESC, p.CreationDate DESC
// LIMIT 100;
fn q14343(db: &'static So) -> String {
    stats_rows(db, stats_with(db, owned_since(db, date(2023, 1, 1)), "cv", &[], &[]), |p, _| score_then_newest(db, p), 100, &["id", "title", "created", "score", "views", "owner", "#cx", "#vx", "up_frac", "down_frac"])
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
// COUNT(v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.PostTypeId IN (1, 2)
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.FavoriteCount, u.DisplayName
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q12235(db: &'static So) -> String {
    let base = owned(db).with((&db.post.post_type_id).in_v(vec![1, 2]));
    stats_rows(db, stats_with(db, base, "v", &[], &[]), |p, _| newest(db, p), 100, &["id", "title", "created", "score", "views", "answers", "comments", "favorites", "owner", "#vx", "#up", "#down"])
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
// MAX(ph.CreationDate) AS LastEdited,
// MAX(CASE WHEN ph.PostHistoryTypeId = 10 THEN ph.CreationDate END) AS ClosedDate
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
// p.PostTypeId IN (1, 2)
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q12578(db: &'static So) -> String {
    let base = owned(db).with((&db.post.post_type_id).in_v(vec![1, 2]));
    stats_rows(db, stats_with(db, base, "cvh", &[], &[]), |p, _| newest(db, p), 100, &["id", "title", "created", "score", "views", "owner", "#cx", "#vx", "hmax", "h10max"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// p.AnswerCount,
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(c.Id) AS CommentCount,
// MAX(v.CreationDate) AS LastVoteDate,
// COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes,
// COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount,
// u.Id, u.DisplayName, u.Reputation
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q10573(db: &'static So) -> String {
    stats_rows(db, stats_with(db, owned(db), "cv", &[], &[]), |p, _| newest(db, p), 100, &["id", "title", "created", "views", "score", "answers", "uid", "owner", "rep", "#cx", "vmax", "#up", "#down"])
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
// COUNT(DISTINCT v.Id) AS VoteCount,
// COUNT(DISTINCT c.Id) AS CommentCount,
// MAX(v.CreationDate) AS LastVoteDate,
// MAX(c.CreationDate) AS LastCommentDate
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, p.CommentCount, u.DisplayName
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q13018(db: &'static So) -> String {
    let votes = per_post_distinct(db, votes_of(db));
    let comments = per_post_distinct(db, comments_of(db));
    stats_rows(db, stats_with(db, owned_since(db, date(2023, 1, 1)), "cv", &[], &[&votes, &comments]), |p, _| newest(db, p), 100, &["id", "title", "created", "views", "score", "answers", "comments", "owner", "#d0", "#d1", "vmax", "cmax"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// u.DisplayName AS OwnerDisplayName,
// COUNT(c.Id) AS CommentCount,
// COUNT(ph.Id) AS HistoryCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.PostTypeId IN (1, 2)
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName
// ORDER BY
// p.CreationDate DESC;
fn q11227(db: &'static So) -> String {
    let base = db.post.with((&db.post.post_type_id).in_v(vec![1, 2]));
    stats_rows(db, stats_with(db, base, "chv", &[], &[]), |_, _| 0, 0, &["id", "title", "created", "score", "views", "owner", "#cx", "#hx", "#up", "#down"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate AS PostCreationDate,
// u.DisplayName AS OwnerDisplayName,
// p.Score,
// p.ViewCount,
// COUNT(v.Id) AS TotalVotes,
// SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes,
// COUNT(c.Id) AS TotalComments
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// VoteTypes vt ON v.VoteTypeId = vt.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, u.DisplayName, p.Score, p.ViewCount
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q11349(db: &'static So) -> String {
    stats_rows(db, stats_with(db, db.post.iq(), "vc", &[], &[]), |p, _| newest(db, p), 100, &["id", "title", "created", "owner", "score", "views", "#vx", "#upn", "#downn", "#cx"])
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
// pt.Name AS PostType,
// COALESCE(SUM(b.Class), 0) AS BadgeCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// WHERE
// p.CreationDate > '2022-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName, pt.Name
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q12312(db: &'static So) -> String {
    let base = db.post.with((&db.post.creation_date).gt(date(2022, 1, 1)));
    let v = stats_with(db, base, "cvb", &[], &[]);
    let mut v: Vec<_> = v.into_iter().map(|(p, s, _)| (newest(db, p), p, s)).collect();
    v.sort_by_key(|x| x.0);
    rows(v.iter().take(100).map(|(_, p, s)| {
        let mut f = stat_fields(db, *p, s, &["id", "title", "created", "score", "views", "#cx", "#vx", "owner", "type"]);
        f.push(V::I(s.bclass));
        row(f)
    }))
}

// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// U.DisplayName AS OwnerDisplayName,
// COUNT(C.Id) AS CommentCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// PT.Name AS PostType,
// MAX(PH.CreationDate) AS LastEditDate
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// LEFT JOIN
// PostTypes PT ON P.PostTypeId = PT.Id
// LEFT JOIN
// PostHistory PH ON P.Id = PH.PostId
// GROUP BY
// P.Id, P.Title, P.CreationDate, U.DisplayName, PT.Name
// ORDER BY
// P.CreationDate DESC;
fn q14215(db: &'static So) -> String {
    stats_rows(db, stats_with(db, owned(db), "cvh", &[], &[]), |_, _| 0, 0, &["id", "title", "created", "owner", "#cx", "#up", "#down", "type", "hmax"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate AS PostCreationDate,
// p.Score,
// p.ViewCount,
// u.Id AS UserId,
// u.Reputation,
// u.DisplayName AS UserDisplayName,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// COUNT(b.Id) AS BadgeCount
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
// p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.Id, u.Reputation, u.DisplayName
// ORDER BY
// p.CreationDate DESC;
fn q12963(db: &'static So) -> String {
    let base = owned(db).with((&db.post.creation_date).gt(ts(2023, 10, 1, 12, 34, 56)));
    stats_rows(db, stats_with(db, base, "cvb", &[], &[]), |_, _| 0, 0, &["id", "title", "created", "score", "views", "uid", "rep", "owner", "#cx", "#vx", "#bx"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate AS PostCreationDate,
// p.ViewCount,
// p.Score,
// p.AnswerCount,
// u.DisplayName AS OwnerDisplayName,
// COUNT(DISTINCT v.Id) AS VoteCount,
// COUNT(DISTINCT c.Id) AS CommentCount,
// MAX(b.Date) AS LastBadgeDate,
// COUNT(DISTINCT b.Id) AS BadgeCount
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
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, u.DisplayName
// ORDER BY
// p.CreationDate DESC;
fn q13717(db: &'static So) -> String {
    let votes = per_post_distinct(db, votes_of(db));
    let comments = per_post_distinct(db, comments_of(db));
    let badges = per_post_distinct(db, (&db.post.owner_user).select(badges_of(db)));
    let v = stats_with(db, owned_since(db, date(2023, 1, 1)), "b", &[], &[&votes, &comments, &badges]);
    stats_rows(db, v, |_, _| 0, 0, &["id", "title", "created", "views", "score", "answers", "owner", "#d0", "#d1", "bmax", "#d2"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// u.DisplayName AS OwnerDisplayName,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// AVG(p.ViewCount) AS AvgViewCount,
// SUM(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS CloseCount,
// SUM(CASE WHEN ph.PostHistoryTypeId = 11 THEN 1 ELSE 0 END) AS ReopenCount
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// WHERE
// p.CreationDate >= '2020-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, u.DisplayName
// ORDER BY
// p.CreationDate DESC;
fn q11383(db: &'static So) -> String {
    let v = stats_with(db, since(db, date(2020, 1, 1)), "cvh", &[], &[]);
    rows(v.iter().map(|(p, s, _)| {
        let mut f = stat_fields(db, *p, s, &["id", "title", "created", "owner", "#cx", "#vx"]);
        f.push(ofloat(db.post.view_count.get(*p).map(|w| w as f64)));
        f.extend([V::I(s.h10), V::I(s.h11)]);
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// u.DisplayName AS OwnerDisplayName,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
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
// p.CreationDate >= DATE('2023-01-01')
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q14993(db: &'static So) -> String {
    stats_rows(db, stats_with(db, owned_since(db, date(2023, 1, 1)), "cvb", &[], &[]), |p, _| newest(db, p), 100, &["id", "title", "created", "score", "views", "owner", "#cx", "#up", "#down", "bmax"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// pt.Name AS PostType,
// u.Reputation AS OwnerReputation,
// COUNT(v.Id) AS VoteCount,
// COUNT(c.Id) AS CommentCount,
// MAX(p.CreationDate) AS PostCreationDate,
// MAX(p.LastActivityDate) AS LastActivityDate,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
// FROM
// Posts p
// INNER JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// GROUP BY
// p.Id, p.Title, pt.Name, u.Reputation
// ORDER BY
// PostCreationDate DESC
// LIMIT 100;
fn q10530(db: &'static So) -> String {
    stats_rows(db, stats_with(db, db.post.iq(), "vc", &[], &[]), |p, _| newest(db, p), 100, &["id", "title", "type", "rep", "#vx", "#cx", "created", "activity", "#up", "#down"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score AS PostScore,
// p.ViewCount,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// u.Reputation AS UserReputation,
// u.DisplayName AS UserDisplayName,
// u.CreationDate AS UserCreationDate
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.Id, u.Reputation, u.DisplayName, u.CreationDate
// ORDER BY
// p.CreationDate DESC
// FETCH FIRST 100 ROWS ONLY;
fn q12976(db: &'static So) -> String {
    stats_rows(db, stats_with(db, owned(db), "cv", &[], &[]), |p, _| newest(db, p), 100, &["id", "title", "created", "score", "views", "#cx", "#up", "#down", "rep", "owner", "ucreated"])
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
// U.CreationDate AS OwnerCreationDate,
// COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVotes,
// COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVotes
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// WHERE
// P.CreationDate >= '2023-01-01'
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.Score, P.ViewCount, P.AnswerCount, P.CommentCount,
// U.DisplayName, U.Reputation, U.CreationDate
// ORDER BY
// P.Score DESC, P.CreationDate DESC;
fn q10076(db: &'static So) -> String {
    stats_rows(db, stats_with(db, owned_since(db, date(2023, 1, 1)), "v", &[], &[]), |_, _| 0, 0, &["id", "title", "created", "score", "views", "answers", "comments", "owner", "rep", "ucreated", "#up", "#down"])
}

// SELECT
// P.Id AS PostId,
// P.Title,
// U.Id AS UserId,
// U.DisplayName AS Author,
// COUNT(C.ID) AS CommentCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// P.CreationDate,
// P.LastActivityDate,
// P.Score,
// P.ViewCount,
// P.AnswerCount
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// LEFT JOIN
// Users U ON P.OwnerUserId = U.Id
// WHERE
// P.PostTypeId = 1
// GROUP BY
// P.Id, P.Title, U.Id, U.DisplayName, P.CreationDate, P.LastActivityDate, P.Score, P.ViewCount, P.AnswerCount
// ORDER BY
// P.Score DESC, P.CreationDate DESC
// LIMIT 100;
fn q11462(db: &'static So) -> String {
    let base = db.post.with((&db.post.post_type_id).eq(1));
    stats_rows(db, stats_with(db, base, "cv", &[], &[]), |p, _| score_then_newest(db, p), 100, &["id", "title", "uid", "owner", "#cx", "#up", "#down", "created", "activity", "score", "views", "answers"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// COALESCE(pt.Name, 'Unknown') AS PostType,
// COALESCE(ut.Reputation, 0) AS OwnerReputation,
// COALESCE(ut.DisplayName, 'Anonymous') AS OwnerDisplayName
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Users ut ON p.OwnerUserId = ut.Id
// WHERE
// p.CreationDate >= DATE '2023-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, pt.Name, ut.Reputation, ut.DisplayName
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q11516(db: &'static So) -> String {
    let v = stats_with(db, since(db, date(2023, 1, 1)), "cv", &[], &[]);
    let mut v: Vec<_> = v.into_iter().map(|(p, s, _)| (newest(db, p), p, s)).collect();
    v.sort_by_key(|x| x.0);
    rows(v.iter().take(100).map(|(_, p, s)| {
        let mut f = stat_fields(db, *p, s, &["id", "title", "created", "views", "score", "#cx", "#vx", "type"]);
        let u = db.post.owner_user.get(*p);
        f.push(V::I(u.map_or(0, |u| db.user.reputation.get(u).unwrap())));
        f.push(V::S(u.map_or("Anonymous", |u| db.user.display_name.get(u).unwrap())));
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.ViewCount,
// p.Score,
// p.CreationDate,
// pt.Name AS PostType,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// u.DisplayName AS OwnerDisplayName,
// u.Reputation AS OwnerReputation
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// JOIN
// Users u ON p.OwnerUserId = u.Id
// GROUP BY
// p.Id, p.Title, p.ViewCount, p.Score, p.CreationDate, pt.Name, u.DisplayName, u.Reputation
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q13301(db: &'static So) -> String {
    stats_rows(db, stats_with(db, owned(db), "cv", &[], &[]), |p, _| newest(db, p), 100, &["id", "title", "views", "score", "created", "type", "#cx", "#vx", "#up", "#down", "owner", "rep"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate AS PostCreationDate,
// p.Score,
// p.ViewCount,
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
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
// AND p.CreationDate >= '2022-01-01'
// AND p.CreationDate < '2023-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.Id, u.DisplayName, u.Reputation
// ORDER BY
// p.CreationDate DESC;
fn q14951(db: &'static So) -> String {
    let base = owned(db).with((&db.post.post_type_id).eq(1).and((&db.post.creation_date).ge(date(2022, 1, 1))).and((&db.post.creation_date).lt(date(2023, 1, 1))));
    stats_rows(db, stats_with(db, base, "cv", &[], &[]), |_, _| 0, 0, &["id", "title", "created", "score", "views", "uid", "owner", "rep", "#cx", "#up", "#down"])
}

// SELECT
// p.Title AS PostTitle,
// p.CreationDate AS PostCreationDate,
// p.ViewCount,
// p.Score,
// u.DisplayName AS OwnerDisplayName,
// COUNT(cm.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// AVG(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS AverageUpVotes,
// AVG(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS AverageDownVotes,
// MAX(ph.CreationDate) AS LastEditDate
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments cm ON cm.PostId = p.Id
// LEFT JOIN
// Votes v ON v.PostId = p.Id
// LEFT JOIN
// PostHistory ph ON ph.PostId = p.Id
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q11737(db: &'static So) -> String {
    stats_rows(db, stats_with(db, questions(db), "cvh", &[], &[]), |p, _| newest(db, p), 100, &["title", "created", "views", "score", "owner", "#cx", "#vx", "up_frac", "down_frac", "hmax"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// COUNT(c.Id) AS CommentCount,
// COUNT(a.Id) AS AnswerCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// u.DisplayName AS OwnerDisplayName,
// u.Reputation AS OwnerReputation
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName, u.Reputation
// ORDER BY
// p.CreationDate DESC;
fn q12445(db: &'static So) -> String {
    stats_rows(db, stats_with(db, since(db, date(2023, 1, 1)), "cAv", &[], &[]), |_, _| 0, 0, &["id", "title", "created", "views", "score", "#cx", "#ax", "#up", "#down", "owner", "rep"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate AS PostCreationDate,
// p.Score,
// p.ViewCount,
// u.DisplayName AS AuthorDisplayName,
// u.Reputation AS AuthorReputation,
// u.CreationDate AS AuthorCreationDate,
// COALESCE(COUNT(c.Id), 0) AS CommentCount,
// COALESCE(COUNT(v.Id), 0) AS VoteCount,
// COALESCE(MAX(b.Name), 'No Badge') AS BadgeName
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
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName, u.Reputation, u.CreationDate
// ORDER BY
// p.Score DESC, p.CreationDate DESC;
fn q13387(db: &'static So) -> String {
    let v = stats_with(db, owned_since(db, date(2023, 1, 1)), "cvb", &[], &[]);
    rows(v.iter().map(|(p, s, _)| {
        let mut f = stat_fields(db, *p, s, &["id", "title", "created", "score", "views", "owner", "rep", "ucreated", "#cx", "#vx"]);
        f.push(V::S(s.bname.unwrap_or("No Badge")));
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate AS PostCreationDate,
// p.ViewCount,
// p.Score,
// COUNT(DISTINCT c.Id) AS CommentCount,
// COUNT(DISTINCT a.Id) AS AnswerCount,
// u.DisplayName AS OwnerDisplayName,
// u.Reputation AS OwnerReputation,
// COUNT(DISTINCT v.Id) AS VoteCount,
// COUNT(DISTINCT b.Id) AS BadgeCount
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Posts a ON p.Id = a.ParentId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// WHERE
// p.CreationDate >= DATE('2023-01-01')
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName, u.Reputation
// ORDER BY
// p.CreationDate DESC;
fn q14045(db: &'static So) -> String {
    let c = per_post_distinct(db, comments_of(db));
    let a = per_post_distinct(db, children_of(db));
    let x = per_post_distinct(db, votes_of(db));
    let b = per_post_distinct(db, (&db.post.owner_user).select(badges_of(db)));
    let mut v = Vec::new();
    since(db, date(2023, 1, 1)).select((&c).opt().and((&a).opt()).and((&x).opt()).and((&b).opt())).drive(|p, (((c, a), x), b)| v.push((p, [c, a, x, b].map(|n| n.unwrap_or(0)))));
    rows(v.iter().map(|&(p, n)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([V::I(n[0]), V::I(n[1])]);
        f.extend(post_fields(db, p, &["owner", "rep"]));
        f.extend([V::I(n[2]), V::I(n[3])]);
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// pt.Name AS PostType,
// u.DisplayName AS OwnerDisplayName,
// u.Reputation AS OwnerReputation,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
// MAX(ph.CreationDate) AS LastEditDate
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
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, pt.Name, u.DisplayName, u.Reputation
// ORDER BY
// p.CreationDate DESC;
fn q10792(db: &'static So) -> String {
    stats_rows(db, stats_with(db, owned(db), "cvh", &[], &[]), |_, _| 0, 0, &["id", "title", "created", "type", "owner", "rep", "#cx", "#vx", "#up", "#down", "hmax"])
}

// GROUP BY a tuple of post columns.
// SELECT
// p.Title AS PostTitle,
// u.DisplayName AS AuthorName,
// p.CreationDate AS PostCreationDate,
// p.ViewCount,
// p.Score,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// AVG(CASE WHEN v.VoteTypeId = 2 THEN 1.0 ELSE 0 END) AS AverageUpVotes,
// AVG(CASE WHEN v.VoteTypeId = 3 THEN 1.0 ELSE 0 END) AS AverageDownVotes
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Title, u.DisplayName, p.CreationDate, p.ViewCount, p.Score
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q10008(db: &'static So) -> String {
    let Post { title, creation_date, view_count, score, owner_user, .. } = &db.post;
    let key = title.opt().and(owner_user.select(&db.user.display_name)).and(creation_date).and(view_count.opt()).and(score);
    let mut v = group_stats(db, owned_since(db, date(2023, 1, 1)), key, "cv", &[]);
    v.sort_by_key(|&(((((_, _), c), _), _), _)| Reverse(c));
    rows(v.iter().take(100).map(|&(((((t, dn), c), w), s), ref st)| {
        row(vec![ostr(t), V::S(dn), V::T(c), oint(w), V::I(s), V::I(st.cx), V::I(st.vx), stat_field(st, "up_frac").unwrap(), stat_field(st, "down_frac").unwrap()])
    }))
}

// GROUP BY a post and one of its child's columns: the joined rows are
// materialised and grouped from there.
// SELECT
// ph.PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// ph.PostHistoryTypeId,
// p.OwnerUserId,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
// FROM
// Posts p
// JOIN
// PostHistory ph ON p.Id = ph.PostId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// ph.CreationDate >= '2023-01-01'
// GROUP BY
// ph.PostId, p.Title, p.CreationDate, p.Score, ph.PostHistoryTypeId, p.OwnerUserId
// ORDER BY
// p.CreationDate DESC;
fn q13730(db: &'static So) -> String {
    let PostHistory { creation_date, post_history_type_id, .. } = &db.post_history;
    let j: MatSet<(Id<Post>, Id<PostHistory>)> = db.post.select(Ident::<Post>::new().and(history_of(db).select(Ident::<PostHistory>::new().with(creation_date.ge(date(2023, 1, 1)))))).collect();
    let post_of = (&j).map(|(p, _)| p);
    let mut v = Vec::new();
    (&j).group_by((&post_of).and((&j).map(|(_, h)| h).select(post_history_type_id)))
        .select((&post_of).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())))
        .fold([0i64; 4], |a, (c, x)| [a[0] + c.is_some() as i64, a[1] + x.is_some() as i64, a[2] + (x == Some(2)) as i64, a[3] + (x == Some(3)) as i64])
        .drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&((p, t), a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.push(V::I(t));
        f.extend(post_fields(db, p, &["owner_id"]));
        f.extend(a.iter().map(|&x| V::I(x)));
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// u.DisplayName AS Owner,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount,
// ph.PostHistoryTypeId,
// ph.CreationDate AS HistoryCreationDate
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
// p.CreationDate >= '2022-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName, ph.PostHistoryTypeId, ph.CreationDate
// ORDER BY
// p.CreationDate DESC;
fn q13941(db: &'static So) -> String {
    let PostHistory { creation_date, post_history_type_id, .. } = &db.post_history;
    let j: MatSet<(Id<Post>, Option<Id<PostHistory>>)> = owned_since(db, date(2022, 1, 1)).select(Ident::<Post>::new().and(history_of(db).opt())).collect();
    let post_of = (&j).map(|(p, _)| p);
    let hist_of = (&j).flat_map(|(_, h)| h);
    let mut v = Vec::new();
    (&j).group_by((&post_of).and((&hist_of).select(post_history_type_id.and(creation_date)).opt()))
        .select((&post_of).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())))
        .fold([0i64; 3], |a, (c, x)| [a[0] + c.is_some() as i64, a[1] + (x == Some(2)) as i64, a[2] + (x == Some(3)) as i64])
        .drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&((p, h), a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "owner"]);
        f.extend(a.iter().map(|&x| V::I(x)));
        f.extend([oint(h.map(|h| h.0)), ots(h.map(|h| h.1))]);
        row(f)
    }))
}

// SELECT
// U.DisplayName AS UserName,
// P.Title AS PostTitle,
// P.CreationDate AS PostDate,
// P.Score AS PostScore,
// C.Text AS CommentText,
// C.CreationDate AS CommentDate,
// COUNT(V.Id) AS VoteCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
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
// AND P.CreationDate >= '2022-01-01'
// GROUP BY
// U.DisplayName, P.Title, P.CreationDate, P.Score, C.Text, C.CreationDate
// ORDER BY
// P.CreationDate DESC;
fn q12750(db: &'static So) -> String {
    let Post { title, creation_date, owner_user, score, post_type_id, .. } = &db.post;
    let base = owned(db).with(post_type_id.eq(1).and(creation_date.ge(date(2022, 1, 1))));
    let j: MatSet<(Id<Post>, Option<Id<Comment>>)> = base.select(Ident::<Post>::new().and(comments_of(db).opt())).collect();
    let post_of = (&j).map(|(p, _)| p);
    let comment_of = (&j).flat_map(|(_, c)| c);
    let key = (&post_of)
        .select(owner_user.select(&db.user.display_name).and(title.opt()).and(creation_date).and(score))
        .and((&comment_of).select((&db.comment.text).and(&db.comment.creation_date)).opt());
    let mut v = Vec::new();
    (&j).group_by(key)
        .select((&post_of).select(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, x| [a[0] + x.is_some() as i64, a[1] + (x == Some(2)) as i64, a[2] + (x == Some(3)) as i64])
        .drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&(((((dn, t), c), s), cm), a)| {
        let mut f = vec![V::S(dn), ostr(t), V::T(c), V::I(s), ostr(cm.map(|x| x.0)), ots(cm.map(|x| x.1))];
        f.extend(a.iter().map(|&x| V::I(x)));
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// p.CommentCount,
// p.AnswerCount,
// u.Id AS UserId,
// u.DisplayName AS UserDisplayName,
// u.Reputation,
// v.VoteTypeId,
// COUNT(v.Id) AS VoteCount,
// AVG(EXTRACT(EPOCH FROM (v.CreationDate - p.CreationDate))) AS AvgTimeToVote
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= CAST('2024-10-01 12:34:56' AS timestamp) - INTERVAL '1 year'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount,
// p.CommentCount, p.AnswerCount, u.Id, u.DisplayName,
// u.Reputation, v.VoteTypeId
// ORDER BY
// p.CreationDate DESC;
fn q11252(db: &'static So) -> String {
    let base = owned(db).with((&db.post.creation_date).ge(ts(2023, 10, 1, 12, 34, 56)));
    let j: MatSet<(Id<Post>, Option<Id<Vote>>)> = base.select(Ident::<Post>::new().and(votes_of(db).opt())).collect();
    let post_of = (&j).map(|(p, _)| p);
    let vote_of = (&j).flat_map(|(_, v)| v);
    let mut v = Vec::new();
    (&j).group_by((&post_of).and((&vote_of).select(&db.vote.vote_type_id).opt()))
        .select((&post_of).select(&db.post.creation_date).and((&vote_of).select(&db.vote.creation_date).opt()))
        .fold((0i64, 0.0f64), |(n, s), (pc, vc)| match vc {
            Some(vc) => (n + 1, s + (vc - pc) as f64 / 1e6),
            None => (n, s),
        })
        .drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&((p, t), (n, s))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "comments", "answers", "uid", "owner", "rep"]);
        f.extend([oint(t), V::I(n), if n == 0 { V::Null } else { V::F(s / n as f64) }]);
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
// COUNT(DISTINCT v.Id) AS VoteCount,
// u.DisplayName AS OwnerDisplayName,
// u.Reputation AS OwnerReputation,
// b.Name AS BadgeName,
// COUNT(DISTINCT pl.RelatedPostId) AS RelatedPostCount
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
// LEFT JOIN
// PostLinks pl ON p.Id = pl.PostId
// WHERE
// p.CreationDate >= '2023-01-01 00:00:00'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount,
// u.DisplayName, u.Reputation, b.Name
// ORDER BY
// p.Score DESC;
fn q13263(db: &'static So) -> String {
    let base = owned(db).with((&db.post.creation_date).ge(date(2023, 1, 1)));
    let j: MatSet<(Id<Post>, Option<Id<Badge>>)> = base.select(Ident::<Post>::new().and((&db.post.owner_user).select(badges_of(db)).opt())).collect();
    let post_of = (&j).map(|(p, _)| p);
    let key = (&post_of).and((&j).flat_map(|(_, b)| b).select(&db.badge.name).opt());
    let links: HashIdx<Id<Post>, Id<PostLink>> = (&db.post_link.post).inv().collect();
    let c = (&j).group_by(&key).select((&post_of).select(comments_of(db).opt().and(votes_of(db).opt()).and((&links).opt()))).fold(0i64, |a, ((c, _), _)| a + c.is_some() as i64);
    let x = (&j).group_by(&key).select((&post_of).select(votes_of(db))).count_distinct();
    let r = (&j).group_by(&key).select((&post_of).select((&links).select(&db.post_link.related_post_id))).count_distinct();
    let mut v = Vec::new();
    c.and((&x).opt()).and((&r).opt()).drive(|(p, b), ((c, x), r)| v.push((p, b, c, x.unwrap_or(0), r.unwrap_or(0))));
    rows(v.iter().map(|&(p, b, c, x, r)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(c), V::I(x)]);
        f.extend(post_fields(db, p, &["owner", "rep"]));
        f.extend([ostr(b), V::I(r)]);
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// u.DisplayName AS OwnerDisplayName,
// p.ViewCount,
// p.Score,
// p.AnswerCount,
// p.CommentCount,
// p.FavoriteCount,
// t.TagName,
// v.VoteTypeId,
// COUNT(v.Id) AS VoteCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Tags t ON t.ExcerptPostId = p.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= CURRENT_DATE - INTERVAL '6 months'
// GROUP BY
// p.Id, p.Title, p.CreationDate, u.DisplayName, p.ViewCount, p.Score,
// p.AnswerCount, p.CommentCount, p.FavoriteCount, t.TagName, v.VoteTypeId
// ORDER BY
// p.CreationDate DESC;
fn q10422(db: &'static So) -> String {
    let base = owned(db).with((&db.post.creation_date).ge(add_months(current_date(), -6)));
    let j: MatSet<(Id<Post>, Option<Id<Vote>>)> = base.select(Ident::<Post>::new().and(votes_of(db).opt())).collect();
    let post_of = (&j).map(|(p, _)| p);
    let vote_of = (&j).flat_map(|(_, v)| v);
    let tag: HashIdx<Id<Post>, Str> = (&db.tag.excerpt_post).inv().select(&db.tag.tag_name).collect();
    let mut v = Vec::new();
    (&j).group_by((&post_of).and((&post_of).select((&tag).opt())).and((&vote_of).select(&db.vote.vote_type_id).opt()))
        .select((&vote_of).opt())
        .fold(0i64, |a, x| a + x.is_some() as i64)
        .drive(|k, n| v.push((k, n)));
    rows(v.iter().map(|&(((p, tn), t), n)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner", "views", "score", "answers", "comments", "favorites"]);
        f.extend([ostr(tn), oint(t), V::I(n)]);
        row(f)
    }))
}

// Users LEFT JOIN Posts [LEFT JOIN PostTypes | Votes | Badges ...]: folds
// over a user's joined rows.
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = pt.Id THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN p.PostTypeId = 3 THEN 1 ELSE 0 END) AS WikiCount,
// SUM(CASE WHEN p.PostTypeId = 4 THEN 1 ELSE 0 END) AS TagWikiExcerptCount,
// SUM(CASE WHEN p.PostTypeId = 5 THEN 1 ELSE 0 END) AS TagWikiCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
// ORDER BY
// u.Reputation DESC;
fn q14421(db: &'static So) -> String {
    let Post { post_type_id, post_type, .. } = &db.post;
    let mut v = Vec::new();
    db.user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(post_type.select(&db.post_type.origid).opt())).opt())
        .fold([0i64; 6], |a, p| match p {
            Some((t, pt)) => [a[0] + 1, a[1] + (pt == Some(t)) as i64, a[2] + (t == 2) as i64, a[3] + (t == 3) as i64, a[4] + (t == 4) as i64, a[5] + (t == 5) as i64],
            None => a,
        })
        .drive(|u, a| v.push((u, a)));
    rows(v.iter().map(|&(u, a)| {
        let mut f = vec![V::I(db.user.origid.get(u).unwrap()), V::S(db.user.display_name.get(u).unwrap()), V::I(db.user.reputation.get(u).unwrap())];
        f.extend(a.iter().map(|&x| V::I(x)));
        row(f)
    }))
}

// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// COUNT(V.Id) AS TotalVotes,
// SUM(CASE WHEN VT.Name = 'UpMod' THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN VT.Name = 'DownMod' THEN 1 ELSE 0 END) AS TotalDownVotes
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// LEFT JOIN
// VoteTypes VT ON V.VoteTypeId = VT.Id
// GROUP BY
// U.Id, U.DisplayName, U.Reputation
// ORDER BY
// U.Reputation DESC
// LIMIT 10;
fn q13315(db: &'static So) -> String {
    let mut v = Vec::new();
    db.user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select((&db.vote.vote_type).select(&db.vote_type.name).opt()).opt())).opt())
        .fold([0i64; 6], |a, p| match p {
            Some((t, x)) => {
                let n = x.flatten();
                [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + x.is_some() as i64, a[4] + (n == Some("UpMod")) as i64, a[5] + (n == Some("DownMod")) as i64]
            }
            None => a,
        })
        .drive(|u, a| v.push((u, a)));
    v.sort_by_key(|&(u, _)| Reverse(db.user.reputation.get(u).unwrap()));
    rows(v.iter().take(10).map(|&(u, a)| {
        let mut f = vec![V::I(db.user.origid.get(u).unwrap()), V::S(db.user.display_name.get(u).unwrap()), V::I(db.user.reputation.get(u).unwrap())];
        f.extend(a.iter().map(|&x| V::I(x)));
        row(f)
    }))
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges,
// AVG(u.Reputation) AS AverageReputation
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// u.Id, u.DisplayName
// ORDER BY
// TotalPosts DESC, AverageReputation DESC
// LIMIT 100;
fn q14400(db: &'static So) -> String {
    let mut v = Vec::new();
    db.user
        .group_by(Ident::<User>::new())
        .select((&db.user.reputation).and(posts_of(db).select(&db.post.post_type_id).opt()).and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 8], |a, ((r, t), b)| {
            [a[0] + t.is_some() as i64, a[1] + (t == Some(1)) as i64, a[2] + (t == Some(2)) as i64, a[3] + (b == Some(1)) as i64, a[4] + (b == Some(2)) as i64, a[5] + (b == Some(3)) as i64, a[6] + r, a[7] + 1]
        })
        .drive(|u, a| v.push((u, a)));
    v.sort_by(|x, y| y.1[0].cmp(&x.1[0]).then((y.1[6] as f64 / y.1[7] as f64).total_cmp(&(x.1[6] as f64 / x.1[7] as f64))));
    rows(v.iter().take(100).map(|&(u, a)| {
        let mut f = vec![V::I(db.user.origid.get(u).unwrap()), V::S(db.user.display_name.get(u).unwrap())];
        f.extend(a[..6].iter().map(|&x| V::I(x)));
        f.push(avg(a[6], a[7]));
        row(f)
    }))
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersCount,
// SUM(CASE WHEN p.PostTypeId IN (4, 5) THEN 1 ELSE 0 END) AS TagWikiCount,
// AVG(p.Score) AS AveragePostScore,
// SUM(c.Score) AS TotalCommentsScore,
// COUNT(b.Id) AS TotalBadges
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
// u.Reputation DESC, TotalPosts DESC
// LIMIT 10;
fn q11362(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let mut v = Vec::new();
    db.user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(comments_of(db).select(&db.comment.score).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 8], |a, (p, b)| {
            let mut a = a;
            if let Some(((t, s), c)) = p {
                a[0] += 1;
                a[1] += (t == 1) as i64;
                a[2] += (t == 2) as i64;
                a[3] += (t == 4 || t == 5) as i64;
                a[4] += s;
                a[5] += c.is_some() as i64;
                a[6] += c.unwrap_or(0);
            }
            a[7] += b.is_some() as i64;
            a
        })
        .drive(|u, a| v.push((u, a)));
    v.sort_by_key(|&(u, a)| (Reverse(db.user.reputation.get(u).unwrap()), Reverse(a[0])));
    rows(v.iter().take(10).map(|&(u, a)| {
        row(vec![
            V::I(db.user.origid.get(u).unwrap()),
            V::S(db.user.display_name.get(u).unwrap()),
            V::I(db.user.reputation.get(u).unwrap()),
            V::I(a[0]),
            V::I(a[1]),
            V::I(a[2]),
            V::I(a[3]),
            avg(a[4], a[0]),
            nullable(a[6], a[5]),
            V::I(a[7]),
        ])
    }))
}

// SELECT
// u.DisplayName AS UserName,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN p.PostTypeId IN (4, 5) THEN 1 ELSE 0 END) AS TotalTagWikis,
// AVG(p.Score) AS AveragePostScore,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// u.Reputation > 1000
// GROUP BY
// u.Id, u.DisplayName
// ORDER BY
// TotalPosts DESC
// LIMIT 10;
fn q10010(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 7], |a, p| match p {
            Some(((t, s), x)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (t == 4 || t == 5) as i64, a[4] + s, a[5] + (x == Some(2)) as i64, a[6] + (x == Some(3)) as i64],
            None => a,
        })
        .drive(|u, a| v.push((u, a)));
    v.sort_by_key(|x| Reverse(x.1[0]));
    rows(v.iter().take(10).map(|&(u, a)| {
        row(vec![V::S(db.user.display_name.get(u).unwrap()), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), avg(a[4], a[0]), V::I(a[5]), V::I(a[6])])
    }))
}

// SELECT
// U.DisplayName AS UserDisplayName,
// U.Reputation,
// U.Views,
// U.UpVotes,
// U.DownVotes,
// P.Title AS PostTitle,
// P.CreationDate AS PostCreationDate,
// P.ViewCount AS PostViewCount,
// COUNT(V.Id) AS VoteCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesCount,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesCount
// FROM
// Users U
// JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// WHERE
// U.Reputation > 1000
// GROUP BY
// U.DisplayName, U.Reputation, U.Views, U.UpVotes, U.DownVotes,
// P.Title, P.CreationDate, P.ViewCount, P.Id, U.Id
// ORDER BY
// U.Reputation DESC, P.CreationDate DESC
// FETCH FIRST 100 ROWS ONLY;
fn q12853(db: &'static So) -> String {
    let base = owned(db).with((&db.post.owner_user).select(&db.user.reputation).gt(1000));
    let mut v = post_stats(db, base, "v", &[]);
    v.sort_by_key(|&(p, _)| (Reverse(db.post.owner_user.get(p).map(|u| db.user.reputation.get(u).unwrap())), newest(db, p)));
    rows(v.iter().take(100).map(|(p, s)| {
        let u = db.post.owner_user.get(*p).unwrap();
        let User { display_name, reputation, views, up_votes, down_votes, .. } = &db.user;
        let mut f = vec![V::S(display_name.get(u).unwrap()), V::I(reputation.get(u).unwrap()), V::I(views.get(u).unwrap()), V::I(up_votes.get(u).unwrap()), V::I(down_votes.get(u).unwrap())];
        f.extend(stat_fields(db, *p, s, &["title", "created", "views", "#vx", "#up", "#down"]));
        row(f)
    }))
}

// WITH PostMetrics AS (
// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// SUM(p.ViewCount) AS TotalViews,
// SUM(p.Score) AS TotalScore,
// AVG(p.ViewCount) AS AverageViews,
// AVG(p.Score) AS AverageScore
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// pt.Name
// )
// SELECT
// PostType,
// TotalPosts,
// TotalViews,
// TotalScore,
// AverageViews,
// AverageScore,
// ROUND(TotalViews / NULLIF(TotalPosts, 0), 2) AS ViewsPerPost,
// ROUND(TotalScore / NULLIF(TotalPosts, 0), 2) AS ScorePerPost
// FROM
// PostMetrics
// ORDER BY
// TotalPosts DESC;
fn q12328(db: &'static So) -> String {
    let Post { post_type, score, view_count, creation_date, .. } = &db.post;
    let mut v = Vec::new();
    db.post
        .with(creation_date.ge(date(2023, 1, 1)))
        .group_by(post_type.select(&db.post_type.name))
        .select(score.and(view_count.opt()))
        .fold([0i64; 4], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)])
        .drive(|k, a| v.push((k, a)));
    v.sort_by_key(|x| Reverse(x.1[0]));
    let round2 = |x: f64| V::F((x * 100.0).round() / 100.0);
    rows(v.iter().map(|&(k, a)| {
        let per = |s: i64, has: bool| if has { round2(s as f64 / a[0] as f64) } else { V::Null };
        row(vec![V::S(k), V::I(a[0]), nullable(a[3], a[2]), V::I(a[1]), avg(a[3], a[2]), avg(a[1], a[0]), per(a[3], a[2] > 0), per(a[1], true)])
    }))
}

// WITH PostMetrics AS (
// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AvgScore,
// SUM(p.ViewCount) AS TotalViews,
// SUM(p.AnswerCount) AS TotalAnswers,
// AVG(p.CommentCount) AS AvgComments,
// AVG(p.FavoriteCount) AS AvgFavorites,
// AVG(EXTRACT(EPOCH FROM (p.LastActivityDate - p.CreationDate))) AS AvgTimeToActivitySeconds
// FROM
// Posts p
// INNER JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// GROUP BY
// pt.Name
// )
// SELECT
// PostType,
// TotalPosts,
// AvgScore,
// TotalViews,
// TotalAnswers,
// AvgComments,
// AvgFavorites,
// AvgTimeToActivitySeconds
// FROM
// PostMetrics
// ORDER BY
// TotalPosts DESC;
fn q13017(db: &'static So) -> String {
    let Post { post_type, score, view_count, answer_count, comment_count, favorite_count, last_activity_date, creation_date, .. } = &db.post;
    let mut v = Vec::new();
    db.post
        .group_by(post_type.select(&db.post_type.name))
        .select(score.and(view_count.opt()).and(answer_count.opt()).and(comment_count).and(favorite_count.opt()).and(last_activity_date).and(creation_date))
        .fold(([0i64; 9], 0i128), |(a, d), ((((((s, w), an), c), fv), la), cd)| {
            (
                [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + an.is_some() as i64, a[5] + an.unwrap_or(0), a[6] + c, a[7] + fv.is_some() as i64, a[8] + fv.unwrap_or(0)],
                d + (la - cd) as i128,
            )
        })
        .drive(|k, a| v.push((k, a)));
    v.sort_by_key(|x| Reverse(x.1.0[0]));
    rows(v.iter().map(|&(k, (a, d))| {
        row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), nullable(a[3], a[2]), nullable(a[5], a[4]), avg(a[6], a[0]), avg(a[8], a[7]), V::F(d as f64 / a[0] as f64 / 1e6)])
    }))
}

fn count<Q: Drive>(q: Q) -> i64 {
    q.fold_flat(0i64, |a, _| a + 1)
}

// SELECT
// (SELECT COUNT(*) FROM Posts) AS TotalPosts,
// (SELECT COUNT(*) FROM Comments) AS TotalComments,
// (SELECT COUNT(*) FROM Votes) AS TotalVotes,
// (SELECT COUNT(*) FROM Users) AS TotalUsers,
// (SELECT COUNT(*) FROM Tags) AS TotalTags,
// (SELECT COUNT(*) FROM Badges) AS TotalBadges,
// (SELECT COUNT(*) FROM PostHistory) AS TotalPostHistory,
// (SELECT COUNT(*) FROM PostLinks) AS TotalPostLinks,
// (SELECT COUNT(*) FROM LinkTypes) AS TotalLinkTypes,
// (SELECT COUNT(*) FROM PostTypes) AS TotalPostTypes,
// (SELECT COUNT(*) FROM CloseReasonTypes) AS TotalCloseReasons,
// (SELECT COUNT(*) FROM VoteTypes) AS TotalVoteTypes,
// (SELECT COUNT(*) FROM PostHistoryTypes) AS TotalPostHistoryTypes;
fn q13485(db: &'static So) -> String {
    row(vec![
        V::I(count(db.post.iq())),
        V::I(count(db.comment.iq())),
        V::I(count(db.vote.iq())),
        V::I(count(db.user.iq())),
        V::I(count(db.tag.iq())),
        V::I(count(db.badge.iq())),
        V::I(count(db.post_history.iq())),
        V::I(count(db.post_link.iq())),
        V::I(count(db.link_type.iq())),
        V::I(count(db.post_type.iq())),
        V::I(count(db.close_reason_type.iq())),
        V::I(count(db.vote_type.iq())),
        V::I(count(db.post_history_type.iq())),
    ])
}

// SELECT
// COUNT(*) AS TotalPosts,
// AVG(ViewCount) AS AvgViewCount,
// AVG(Score) AS AvgScore,
// SUM(CASE WHEN PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN PostTypeId = 3 THEN 1 ELSE 0 END) AS TotalWikis,
// SUM(CASE WHEN ClosedDate IS NOT NULL THEN 1 ELSE 0 END) AS TotalClosedPosts,
// SUM(CASE WHEN FavoriteCount > 0 THEN 1 ELSE 0 END) AS TotalFavoritePosts,
// COUNT(DISTINCT OwnerUserId) AS TotalUniqueAuthors,
// COUNT(DISTINCT Tags) AS TotalUniqueTags,
// MIN(CreationDate) AS EarliestPostDate,
// MAX(CreationDate) AS LatestPostDate
// FROM
// Posts
// WHERE
// CreationDate >= '2023-01-01' AND CreationDate < '2024-01-01';
fn q13734(db: &'static So) -> String {
    let Post { post_type_id, view_count, score, closed_date, favorite_count, owner_user_id, tags_str, creation_date, .. } = &db.post;
    let base = db.post.with(creation_date.ge(date(2023, 1, 1)).and(creation_date.lt(date(2024, 1, 1))));
    let a = (&base).select(view_count.opt().and(score).and(post_type_id).and(closed_date.opt()).and(favorite_count.opt()).and(creation_date)).fold_flat(
        [0, 0, 0, 0, 0, 0, 0, 0, 0, i64::MAX, i64::MIN],
        |a: [i64; 11], (((((w, s), t), cl), fv), c)| {
            [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s, a[4] + (t == 1) as i64, a[5] + (t == 2) as i64, a[6] + (t == 3) as i64, a[7] + cl.is_some() as i64, a[8] + fv.is_some_and(|x| x > 0) as i64, a[9].min(c), a[10].max(c)]
        },
    );
    let one = |f: Fold<(), i64>| (&f).fold_flat(0i64, |a, x| a + x);
    let authors = one(whole(&base).select(owner_user_id).count_distinct());
    let tags = one(whole(&base).select(tags_str).count_distinct());
    row(vec![
        V::I(a[0]),
        avg(a[2], a[1]),
        avg(a[3], a[0]),
        nullable(a[4], a[0]),
        nullable(a[5], a[0]),
        nullable(a[6], a[0]),
        nullable(a[7], a[0]),
        nullable(a[8], a[0]),
        V::I(authors),
        V::I(tags),
        if a[0] == 0 { V::Null } else { V::T(a[9]) },
        if a[0] == 0 { V::Null } else { V::T(a[10]) },
    ])
}

pub static ENTRIES: &[harness::Entry] = &[
    ("11735", q11735),
    ("10300", q10300),
    ("14153", q14153),
    ("12085", q12085),
    ("14267", q14267),
    ("10717", q10717),
    ("14551", q14551),
    ("13727", q13727),
    ("11636", q11636),
    ("11133", q11133),
    ("12010", q12010),
    ("11487", q11487),
    ("13053", q13053),
    ("14343", q14343),
    ("12235", q12235),
    ("12578", q12578),
    ("10573", q10573),
    ("13018", q13018),
    ("11227", q11227),
    ("11349", q11349),
    ("12312", q12312),
    ("14215", q14215),
    ("12963", q12963),
    ("13717", q13717),
    ("11383", q11383),
    ("14993", q14993),
    ("10530", q10530),
    ("12976", q12976),
    ("10076", q10076),
    ("11462", q11462),
    ("11516", q11516),
    ("13301", q13301),
    ("14951", q14951),
    ("11737", q11737),
    ("12445", q12445),
    ("13387", q13387),
    ("14045", q14045),
    ("10792", q10792),
    ("10008", q10008),
    ("13730", q13730),
    ("13941", q13941),
    ("12750", q12750),
    ("11252", q11252),
    ("13263", q13263),
    ("10422", q10422),
    ("14421", q14421),
    ("13315", q13315),
    ("14400", q14400),
    ("11362", q11362),
    ("10010", q10010),
    ("12853", q12853),
    ("12328", q12328),
    ("13017", q13017),
    ("13485", q13485),
    ("13734", q13734),
];
