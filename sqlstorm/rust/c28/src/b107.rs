use harness::prelude::*;
use std::cmp::Reverse;

fn questions(db: &'static So) -> impl Drive<D = Id<Post>, R = Id<Post>> {
    owned(db).with((&db.post.post_type_id).eq(1))
}

fn all_questions(db: &'static So) -> impl Drive<D = Id<Post>, R = Id<Post>> {
    db.post.with((&db.post.post_type_id).eq(1))
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

fn badges_distinct(db: &'static So) -> Fold<Id<Post>, i64> {
    per_post_distinct(db, (&db.post.owner_user).select(badges_of(db)))
}

fn ud<R>(db: &'static So, w: UserWhere, r: R) -> Fold<Id<User>, i64>
where
    R: IntoQuery,
    R::Q: Probe<D = Id<User>>,
    ROf<R>: Ord,
{
    user_distinct(db, Ident::<User>::new(), w, r)
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// p.AnswerCount,
// u.DisplayName AS OwnerDisplayName,
// COALESCE(COUNT(c.Id), 0) AS CommentCount,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 6 THEN 1 ELSE 0 END), 0) AS CloseVotes
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, u.DisplayName
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q13674(db: &'static So) -> String {
    stats_rows(db, stats_with(db, db.post.iq(), "cv", &[], &[]), |p, _| newest(db, p), 100, &["id", "title", "created", "score", "views", "answers", "owner", "#cx", "#up", "#down", "#v6"])
}

// SELECT
// p.Id AS PostID,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// u.DisplayName AS Author,
// u.Reputation AS AuthorReputation,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
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
// p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName, u.Reputation
// ORDER BY
// p.CreationDate DESC;
fn q11489(db: &'static So) -> String {
    stats_rows(db, stats_with(db, owned_since(db, year_ago()), "cv", &[], &[]), |_, _| 0, 0, &["id", "title", "created", "score", "views", "owner", "rep", "#cx", "#vx", "#up", "#down"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.Score,
// p.ViewCount,
// p.CreationDate,
// u.DisplayName AS OwnerDisplayName,
// COUNT(c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
// AVG(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS AverageUpvotes,
// AVG(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS AverageDownvotes,
// COUNT(DISTINCT ph.Id) AS HistoryCount
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
// p.Id, p.Title, p.Score, p.ViewCount, p.CreationDate, u.DisplayName
// ORDER BY
// p.CreationDate DESC;
fn q10150(db: &'static So) -> String {
    let x = per_post_distinct(db, votes_of(db));
    let h = per_post_distinct(db, history_of(db));
    stats_rows(db, stats_with(db, questions(db), "cvh", &[], &[&x, &h]), |_, _| 0, 0, &["id", "title", "score", "views", "created", "owner", "#cx", "#d0", "up_frac", "down_frac", "#d1"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate AS PostCreationDate,
// u.DisplayName AS AuthorDisplayName,
// u.Reputation AS AuthorReputation,
// COUNT(v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount,
// p.ViewCount,
// p.AnswerCount,
// p.CommentCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'
// GROUP BY
// p.Id, p.Title, p.CreationDate, u.DisplayName, u.Reputation, p.ViewCount, p.AnswerCount, p.CommentCount
// ORDER BY
// p.CreationDate DESC;
fn q10899(db: &'static So) -> String {
    stats_rows(db, stats_with(db, owned_since(db, year_ago()), "v", &[], &[]), |_, _| 0, 0, &["id", "title", "created", "owner", "rep", "#vx", "#up", "#down", "views", "answers", "comments"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// u.DisplayName AS OwnerDisplayName,
// u.Reputation AS OwnerReputation,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName, u.Reputation
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q11649(db: &'static So) -> String {
    stats_rows(db, stats_with(db, since(db, year_ago()), "cv", &[], &[]), |p, _| newest(db, p), 100, &["id", "title", "created", "score", "views", "owner", "rep", "#cx", "#vx", "#up", "#down"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// u.DisplayName AS OwnerDisplayName,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
// p.AnswerCount,
// p.FavoriteCount
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 YEAR'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName,
// p.AnswerCount, p.FavoriteCount
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q10846(db: &'static So) -> String {
    stats_rows(db, stats_with(db, since(db, year_ago()), "cv", &[], &[]), |p, _| newest(db, p), 100, &["id", "title", "created", "views", "score", "owner", "#cx", "#up", "#down", "answers", "favorites"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// u.DisplayName AS OwnerDisplayName,
// COUNT(c.Id) AS CommentCount,
// COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty,
// COUNT(DISTINCT ph.Id) AS RevisionCount,
// COUNT(DISTINCT b.Id) AS BadgeCount
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (8, 9)
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// WHERE
// p.CreationDate >= '2022-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q14952(db: &'static So) -> String {
    let h = per_post_distinct(db, history_of(db));
    let b = badges_distinct(db);
    let v = stats_with(db, since(db, date(2022, 1, 1)), "cvhb", &[8, 9], &[&h, &b]);
    let mut v: Vec<_> = v.into_iter().map(|(p, s, d)| (newest(db, p), p, s, d)).collect();
    v.sort_by_key(|x| x.0);
    rows(v.iter().take(100).map(|(_, p, s, d)| {
        let mut f = stat_fields(db, *p, s, &["id", "title", "created", "score", "views", "owner", "#cx"]);
        f.extend([V::I(s.bounty_sum), V::I(d[0]), V::I(d[1])]);
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
// COUNT(DISTINCT b.Id) AS BadgeCount,
// u.DisplayName AS OwnerDisplayName,
// u.Reputation AS OwnerReputation
// FROM Posts p
// JOIN Users u ON p.OwnerUserId = u.Id
// LEFT JOIN Comments c ON c.PostId = p.Id
// LEFT JOIN Votes v ON v.PostId = p.Id
// LEFT JOIN Badges b ON b.UserId = u.Id
// WHERE p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Id,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// u.DisplayName,
// u.Reputation
// ORDER BY p.Score DESC, p.ViewCount DESC;
fn q11892(db: &'static So) -> String {
    let b = badges_distinct(db);
    stats_rows(db, stats_with(db, owned_since(db, date(2023, 1, 1)), "cvb", &[], &[&b]), |_, _| 0, 0, &["id", "title", "created", "views", "score", "#cx", "#up", "#down", "#d0", "owner", "rep"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.Score,
// p.ViewCount,
// p.CreationDate,
// p.LastActivityDate,
// u.DisplayName AS OwnerDisplayName,
// COUNT(DISTINCT c.Id) AS CommentCount,
// COUNT(DISTINCT a.Id) AS AcceptedAnswerCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Posts a ON p.AcceptedAnswerId = a.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, p.Score, p.ViewCount, p.CreationDate, p.LastActivityDate, u.DisplayName
// ORDER BY
// p.ViewCount DESC
// LIMIT 100;
fn q10351(db: &'static So) -> String {
    let c = per_post_distinct(db, comments_of(db));
    let a = per_post_distinct(db, &db.post.accepted_answer);
    stats_rows(db, stats_with(db, all_questions(db), "cv", &[], &[&c, &a]), |p, _| views_desc(db, p), 100, &["id", "title", "score", "views", "created", "activity", "owner", "#d0", "#d1", "#up", "#down"])
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
// pt.Name AS PostTypeName,
// CASE
// WHEN p.AcceptedAnswerId IS NOT NULL THEN 'Has Accepted Answer'
// ELSE 'No Accepted Answer'
// END AS AnswerStatus
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
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount,
// u.DisplayName, u.Reputation, pt.Name, p.AcceptedAnswerId
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q11712(db: &'static So) -> String {
    let v = stats_with(db, owned(db), "cv", &[], &[]);
    let mut v: Vec<_> = v.into_iter().map(|(p, s, _)| (newest(db, p), p, s)).collect();
    v.sort_by_key(|x| x.0);
    rows(v.iter().take(100).map(|(_, p, s)| {
        let mut f = stat_fields(db, *p, s, &["id", "title", "created", "score", "views", "#cx", "#vx", "owner", "rep", "type"]);
        f.push(V::S(if db.post.accepted_answer_id.get(*p).is_some() { "Has Accepted Answer" } else { "No Accepted Answer" }));
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// u.DisplayName AS OwnerName,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// COUNT(DISTINCT ph.Id) AS EditCount,
// MIN(ph.CreationDate) AS FirstEditDate,
// MAX(ph.CreationDate) AS LastEditDate
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
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q13124(db: &'static So) -> String {
    let h = per_post_distinct(db, history_of(db));
    stats_rows(db, stats_with(db, questions(db), "cvh", &[], &[&h]), |p, _| newest(db, p), 100, &["id", "title", "created", "score", "views", "owner", "#cx", "#up", "#down", "#d0", "hmin", "hmax"])
}

// WITH AggregatedData AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// u.DisplayName AS OwnerDisplayName,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// MAX(ph.CreationDate) AS LastEditDate
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
// p.CreationDate >= '2023-01-01 00:00:00'
// GROUP BY
// p.Id, p.Title, u.DisplayName
// )
// SELECT
// PostId,
// Title,
// OwnerDisplayName,
// CommentCount,
// VoteCount,
// LastEditDate
// FROM
// AggregatedData
// ORDER BY
// VoteCount DESC, CommentCount DESC
// LIMIT 100;
fn q12617(db: &'static So) -> String {
    let v = stats_with(db, since(db, date(2023, 1, 1)), "cvh", &[], &[]);
    stats_rows(db, v, |p, s| (Reverse(s.vx), Reverse(s.cx), db.post.origid.get(p).unwrap()), 100, &["id", "title", "owner", "#cx", "#vx", "hmax"])
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// p.Id AS PostId,
// p.Title,
// p.CreationDate AS PostCreationDate,
// p.Score AS PostScore,
// p.ViewCount AS PostViewCount,
// COALESCE(COUNT(c.Id), 0) AS CommentCount,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount
// FROM
// Users u
// JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// u.CreationDate >= '2021-01-01'
// AND p.PostTypeId IN (1, 2)
// GROUP BY
// u.Id, u.DisplayName, u.Reputation,
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount
// ORDER BY
// u.Reputation DESC, p.Score DESC;
fn q10193(db: &'static So) -> String {
    let base = owned(db).with((&db.post.post_type_id).in_v(vec![1, 2]).and((&db.post.owner_user).select(&db.user.creation_date).ge(date(2021, 1, 1))));
    stats_rows(db, stats_with(db, base, "cv", &[], &[]), |_, _| 0, 0, &["uid", "owner", "rep", "id", "title", "created", "score", "views", "#cx", "#up", "#down"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// p.AnswerCount,
// p.CommentCount,
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation AS UserReputation,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= '2024-10-01 12:34:56'::timestamp - INTERVAL '1 year'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, p.CommentCount,
// u.Id, u.DisplayName, u.Reputation
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q14785(db: &'static So) -> String {
    stats_rows(db, stats_with(db, since(db, year_ago()), "cv", &[], &[]), |p, _| newest(db, p), 100, &["id", "title", "created", "views", "score", "answers", "comments", "uid", "owner", "rep", "#cx", "#up", "#down"])
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
// u.DisplayName AS OwnerDisplayName,
// u.Reputation AS OwnerReputation,
// u.Views AS OwnerViews,
// COUNT(v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, p.CommentCount, p.FavoriteCount,
// u.DisplayName, u.Reputation, u.Views
// ORDER BY
// p.CreationDate DESC;
fn q10427(db: &'static So) -> String {
    let v = stats_with(db, owned_since(db, year_ago()), "v", &[], &[]);
    rows(v.iter().map(|(p, s, _)| {
        let mut f = stat_fields(db, *p, s, &["id", "title", "created", "views", "score", "answers", "comments", "favorites", "owner", "rep"]);
        f.push(V::I(db.user.views.get(db.post.owner_user.get(*p).unwrap()).unwrap()));
        f.extend(["#vx", "#up", "#down"].iter().map(|c| stat_field(s, c).unwrap()));
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.LastActivityDate,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// u.DisplayName AS OwnerDisplayName,
// u.Reputation AS OwnerReputation,
// pt.Name AS PostType,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Id, u.DisplayName, u.Reputation, pt.Name, p.Title, p.CreationDate, p.LastActivityDate
// ORDER BY
// p.LastActivityDate DESC;
fn q14931(db: &'static So) -> String {
    stats_rows(db, stats_with(db, since(db, date(2023, 1, 1)), "cv", &[], &[]), |_, _| 0, 0, &["id", "title", "created", "activity", "#cx", "#vx", "owner", "rep", "type", "#up", "#down"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate AS PostCreationDate,
// p.Score,
// p.ViewCount,
// p.AnswerCount,
// u.DisplayName AS AuthorDisplayName,
// u.Reputation AS AuthorReputation,
// COUNT(v.Id) AS TotalVotes,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
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
// p.CreationDate >= '2024-10-01 12:34:56'::timestamp - INTERVAL '1 year'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount,
// u.DisplayName, u.Reputation
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q10539(db: &'static So) -> String {
    stats_rows(db, stats_with(db, owned_since(db, year_ago()), "vc", &[], &[]), |p, _| newest(db, p), 100, &["id", "title", "created", "score", "views", "answers", "owner", "rep", "#vx", "#up", "#down", "#cx"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// u.DisplayName AS OwnerDisplayName,
// u.Reputation AS OwnerReputation,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes,
// p.ViewCount,
// p.AcceptedAnswerId
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
// p.CreationDate >= '2024-10-01 12:34:56'::timestamp - INTERVAL '30 days'
// GROUP BY
// p.Id, p.Title, p.CreationDate, u.DisplayName, u.Reputation, p.ViewCount, p.AcceptedAnswerId
// ORDER BY
// p.CreationDate DESC;
fn q10288(db: &'static So) -> String {
    stats_rows(db, stats_with(db, owned_since(db, month_ago()), "cv", &[], &[]), |_, _| 0, 0, &["id", "title", "created", "owner", "rep", "#cx", "#vx", "#upn", "#downn", "views", "accepted"])
}

// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate AS PostCreationDate,
// U.Id AS UserId,
// U.DisplayName AS UserDisplayName,
// U.Reputation,
// P.ViewCount,
// P.Score,
// COALESCE(COUNT(V.Id), 0) AS VoteCount,
// COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
// COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
// COALESCE(P.AnswerCount, 0) AS AnswerCount,
// COALESCE(P.CommentCount, 0) AS CommentCount
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// WHERE
// P.CreationDate >= '2023-01-01'
// GROUP BY
// P.Id, P.Title, P.CreationDate, U.Id, U.DisplayName, U.Reputation, P.ViewCount, P.Score, P.AnswerCount, P.CommentCount
// ORDER BY
// P.CreationDate DESC
// LIMIT 100;
fn q12261(db: &'static So) -> String {
    let v = stats_with(db, owned_since(db, date(2023, 1, 1)), "v", &[], &[]);
    let mut v: Vec<_> = v.into_iter().map(|(p, s, _)| (newest(db, p), p, s)).collect();
    v.sort_by_key(|x| x.0);
    rows(v.iter().take(100).map(|(_, p, s)| {
        let mut f = stat_fields(db, *p, s, &["id", "title", "created", "uid", "owner", "rep", "views", "score", "#vx", "#up", "#down"]);
        f.push(V::I(db.post.answer_count.get(*p).unwrap_or(0)));
        f.extend(post_fields(db, *p, &["comments"]));
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
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
// pt.Name AS PostTypeName,
// COUNT(DISTINCT b.Id) AS BadgeCount
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// WHERE
// p.CreationDate BETWEEN '2023-01-01' AND '2023-10-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName, pt.Name
// ORDER BY
// p.CreationDate DESC;
fn q13426(db: &'static So) -> String {
    let b = badges_distinct(db);
    let base = db.post.with((&db.post.creation_date).between(date(2023, 1, 1), date(2023, 10, 1)));
    stats_rows(db, stats_with(db, base, "cvb", &[], &[&b]), |_, _| 0, 0, &["id", "title", "created", "views", "score", "owner", "#cx", "#up", "#down", "type", "#d0"])
}

// WITH PostDetails AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// pt.Name AS PostType,
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
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, pt.Name, u.Reputation
// )
// SELECT
// PostId,
// Title,
// CreationDate,
// PostType,
// OwnerReputation,
// Score,
// ViewCount,
// CommentCount,
// VoteCount
// FROM
// PostDetails
// ORDER BY
// Score DESC, ViewCount DESC;
fn q10747(db: &'static So) -> String {
    stats_rows(db, stats_with(db, owned(db), "cv", &[], &[]), |_, _| 0, 0, &["id", "title", "created", "type", "rep", "score", "views", "#cx", "#vx"])
}

// SELECT
// p.Title AS PostTitle,
// p.CreationDate AS PostCreationDate,
// u.DisplayName AS OwnerDisplayName,
// u.Reputation AS OwnerReputation,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// pt.Name AS PostTypeName,
// MAX(ph.CreationDate) AS LastEditDate,
// COUNT(DISTINCT ph.Id) AS EditCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, u.DisplayName, u.Reputation, pt.Name
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q13629(db: &'static So) -> String {
    let h = per_post_distinct(db, history_of(db));
    stats_rows(db, stats_with(db, owned(db), "cvh", &[], &[&h]), |p, _| newest(db, p), 100, &["title", "created", "owner", "rep", "#cx", "#vx", "#up", "#down", "type", "hmax", "#d0"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// u.DisplayName AS OwnerDisplayName,
// COUNT(v.Id) AS VoteCount,
// AVG(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteAverage,
// AVG(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteAverage,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN b.UserId IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount,
// p.Score,
// p.ViewCount
// FROM
// Posts p
// LEFT JOIN
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
// p.Id, p.Title, p.CreationDate, u.DisplayName, p.Score, p.ViewCount
// ORDER BY
// p.CreationDate DESC;
fn q14378(db: &'static So) -> String {
    let v = stats_with(db, since(db, date(2020, 1, 1)), "vcb", &[], &[]);
    stats_rows(db, v, |_, _| 0, 0, &["id", "title", "created", "owner", "#vx", "up_frac", "down_frac", "#cx", "#bx", "score", "views"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// u.DisplayName AS OwnerDisplayName,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// AVG(COALESCE(p2.ViewCount, 0)) AS AverageViewCount,
// MAX(p.LastEditDate) AS LastEdit,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Posts p2 ON p.ParentId = p2.Id
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, u.DisplayName
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q11765(db: &'static So) -> String {
    let Post { parent, view_count, .. } = &db.post;
    let mut v = Vec::new();
    questions(db)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(parent.select(view_count.opt()).opt()))
        .fold([0i64; 6], |a, ((c, x), pv)| {
            [a[0] + c.is_some() as i64, a[1] + x.is_some() as i64, a[2] + pv.flatten().unwrap_or(0), a[3] + 1, a[4] + (x == Some(2)) as i64, a[5] + (x == Some(3)) as i64]
        })
        .drive(|p, a| v.push((newest(db, p), p, a)));
    v.sort_by_key(|x| x.0);
    rows(v.iter().take(100).map(|&(_, p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::F(a[2] as f64 / a[3] as f64)]);
        f.extend(post_fields(db, p, &["edited"]));
        f.extend([V::I(a[4]), V::I(a[5])]);
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// COUNT(v.Id) AS VoteCount,
// COALESCE(SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END), 0) AS Upvotes,
// COALESCE(SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END), 0) AS Downvotes,
// COALESCE(SUM(CASE WHEN vt.Name = 'BountyStart' THEN v.BountyAmount ELSE 0 END), 0) AS TotalBounty,
// u.DisplayName AS AuthorDisplayName,
// u.Reputation AS AuthorReputation
// FROM
// Posts p
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// VoteTypes vt ON v.VoteTypeId = vt.Id
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, u.DisplayName, u.Reputation
// ORDER BY
// VoteCount DESC, p.CreationDate DESC;
fn q14687(db: &'static So) -> String {
    let Vote { vote_type, bounty_amount, .. } = &db.vote;
    let mut v = Vec::new();
    since(db, date(2023, 1, 1))
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(vote_type.select(&db.vote_type.name).opt().and(bounty_amount.opt())).opt())
        .fold([0i64; 4], |a, x| match x {
            Some((n, b)) => [a[0] + 1, a[1] + (n == Some("UpMod")) as i64, a[2] + (n == Some("DownMod")) as i64, a[3] + if n == Some("BountyStart") { b.unwrap_or(0) } else { 0 }],
            None => a,
        })
        .drive(|p, a| v.push((p, a)));
    rows(v.iter().map(|&(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3])]);
        f.extend(post_fields(db, p, &["owner", "rep"]));
        row(f)
    }))
}

// A per-post fold of every post, then an aggregate with no group over it.
// WITH Benchmark AS (
// SELECT
// P.Id AS PostId,
// U.DisplayName AS OwnerDisplayName,
// P.Title,
// P.CreationDate,
// P.LastEditDate,
// P.ViewCount,
// P.Score,
// COUNT(C.Id) AS CommentCount,
// COUNT(V.Id) AS VoteCount
// FROM
// Posts P
// LEFT JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// P.Id, U.DisplayName, P.Title, P.CreationDate, P.LastEditDate, P.ViewCount, P.Score
// )
// SELECT
// COUNT(*) AS TotalPosts,
// AVG(ViewCount) AS AverageViews,
// AVG(Score) AS AverageScore,
// SUM(CommentCount) AS TotalComments,
// SUM(VoteCount) AS TotalVotes
// FROM
// Benchmark;
fn q12919(db: &'static So) -> String {
    let per_post = stats_fold(db, db.post.iq(), Ident::<Post>::new(), "cv", &[]);
    let Post { view_count, score, .. } = &db.post;
    let (n, vn, vs, s, c, x) = (&per_post).and(view_count.opt()).and(score).fold_flat((0i64, 0i64, 0i64, 0i64, 0i64, 0i64), |(n, vn, vs, sc, c, x), ((st, w), s)| {
        (n + 1, vn + w.is_some() as i64, vs + w.unwrap_or(0), sc + s, c + st.cx, x + st.vx)
    });
    row(vec![V::I(n), avg(vs, vn), avg(s, n), nullable(c, n), nullable(x, n)])
}

// WITH BenchmarkData AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// p.AnswerCount,
// p.CommentCount,
// u.Reputation AS OwnerReputation,
// p2.Title AS AcceptedAnswerTitle
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Posts p2 ON p.AcceptedAnswerId = p2.Id
// WHERE
// p.PostTypeId = 1
// )
// SELECT
// COUNT(*) AS TotalQuestions,
// AVG(Score) AS AvgScore,
// AVG(ViewCount) AS AvgViewCount,
// AVG(AnswerCount) AS AvgAnswerCount,
// AVG(CommentCount) AS AvgCommentCount,
// MAX(OwnerReputation) AS MaxOwnerReputation,
// MIN(OwnerReputation) AS MinOwnerReputation
// FROM
// BenchmarkData;
fn q14423(db: &'static So) -> String {
    let Post { score, view_count, answer_count, comment_count, owner_user, accepted_answer, .. } = &db.post;
    let a = all_questions(db).select(score.and(view_count.opt()).and(answer_count.opt()).and(comment_count).and(owner_user.select(&db.user.reputation).opt()).and(accepted_answer.opt())).fold_flat(
        [0, 0, 0, 0, 0, 0, 0, i64::MIN, i64::MAX, 0],
        |a: [i64; 10], (((((s, w), an), c), r), _)| {
            [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + an.is_some() as i64, a[5] + an.unwrap_or(0), a[6] + c, r.map_or(a[7], |x| a[7].max(x)), r.map_or(a[8], |x| a[8].min(x)), a[9] + r.is_some() as i64]
        },
    );
    row(vec![V::I(a[0]), avg(a[1], a[0]), avg(a[3], a[2]), avg(a[5], a[4]), avg(a[6], a[0]), omax(a[7], a[9]), omax(a[8], a[9])])
}

// WITH PostMetrics AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.ViewCount,
// p.CreationDate,
// p.PostTypeId,
// u.DisplayName AS OwnerDisplayName,
// COUNT(c.Id) AS CommentCount
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// WHERE
// p.CreationDate >= '2023-01-01' AND
// p.CreationDate < '2023-12-31'
// GROUP BY
// p.Id, p.Title, p.ViewCount, p.CreationDate, p.PostTypeId, u.DisplayName
// )
// SELECT
// pt.Name AS PostType,
// COUNT(pm.PostId) AS TotalPosts,
// SUM(pm.ViewCount) AS TotalViews,
// AVG(pm.CommentCount) AS AverageComments
// FROM
// PostMetrics pm
// JOIN
// PostTypes pt ON pm.PostTypeId = pt.Id
// GROUP BY
// pt.Name
// ORDER BY
// TotalViews DESC;
fn q14935(db: &'static So) -> String {
    let Post { creation_date, view_count, post_type, .. } = &db.post;
    let base = db.post.with(creation_date.ge(date(2023, 1, 1)).and(creation_date.lt(date(2023, 12, 31))));
    let per_post = (&base).group_by(Ident::<Post>::new()).select(comments_of(db).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let mut v = Vec::new();
    (&base)
        .group_by(post_type.select(&db.post_type.name))
        .select(view_count.opt().and(&per_post))
        .fold([0i64; 4], |a, (w, c)| [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + c])
        .drive(|k, a| v.push((k, a)));
    v.sort_by_key(|x| (x.1[1] == 0, Reverse(x.1[2])));
    rows(v.iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), nullable(a[2], a[1]), avg(a[3], a[0])])))
}

// WITH PostMetrics AS (
// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// SUM(p.Score) AS TotalScore,
// AVG(p.ViewCount) AS AverageViewCount,
// SUM(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS TotalAcceptedAnswers,
// SUM(CASE WHEN p.CommentCount > 0 THEN 1 ELSE 0 END) AS PostsWithComments,
// SUM(CASE WHEN p.FavoriteCount > 0 THEN 1 ELSE 0 END) AS PostsWithFavorites
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
// TotalScore,
// AverageViewCount,
// TotalAcceptedAnswers,
// PostsWithComments,
// PostsWithFavorites
// FROM
// PostMetrics
// ORDER BY
// TotalPosts DESC;
fn q12811(db: &'static So) -> String {
    let Post { post_type, score, view_count, accepted_answer_id, comment_count, favorite_count, .. } = &db.post;
    let mut v = Vec::new();
    db.post
        .group_by(post_type.select(&db.post_type.name))
        .select(score.and(view_count.opt()).and(accepted_answer_id.opt()).and(comment_count).and(favorite_count.opt()))
        .fold([0i64; 7], |a, ((((s, w), ac), c), f)| {
            [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + ac.is_some() as i64, a[5] + (c > 0) as i64, a[6] + f.is_some_and(|x| x > 0) as i64]
        })
        .drive(|k, a| v.push((k, a)));
    v.sort_by_key(|x| Reverse(x.1[0]));
    rows(v.iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), V::I(a[1]), avg(a[3], a[2]), V::I(a[4]), V::I(a[5]), V::I(a[6])])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// SUM(COALESCE(p.ViewCount, 0)) AS TotalViews,
// AVG(p.Score) AS AverageScore,
// COUNT(DISTINCT p.OwnerUserId) AS UniqueUsers,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN p.PostTypeId = 4 THEN 1 ELSE 0 END) AS TotalTagWikis,
// SUM(CASE WHEN p.PostTypeId = 10 THEN 1 ELSE 0 END) AS TotalClosedPosts,
// AVG(u.Reputation) AS AverageUserReputation
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q10380(db: &'static So) -> String {
    let Post { post_type, view_count, score, post_type_id, owner_user, owner_user_id, creation_date, .. } = &db.post;
    let base = owned(db).with(creation_date.ge(year_ago()));
    let name = post_type.select(&db.post_type.name);
    let f = (&base).group_by(&name).select(view_count.opt().and(score).and(post_type_id).and(owner_user.select(&db.user.reputation))).fold([0i64; 8], |a, (((w, s), t), r)| {
        [a[0] + 1, a[1] + w.unwrap_or(0), a[2] + s, a[3] + (t == 1) as i64, a[4] + (t == 2) as i64, a[5] + (t == 4) as i64, a[6] + (t == 10) as i64, a[7] + r]
    });
    let u = (&base).group_by(&name).select(owner_user_id).count_distinct();
    let mut v = Vec::new();
    f.and(&u).drive(|k, (a, u)| v.push((k, a, u)));
    v.sort_by_key(|x| Reverse(x.1[0]));
    rows(v.iter().map(|&(k, a, u)| row(vec![V::S(k), V::I(a[0]), V::I(a[1]), avg(a[2], a[0]), V::I(u), V::I(a[3]), V::I(a[4]), V::I(a[5]), V::I(a[6]), avg(a[7], a[0])])))
}

// GROUP BY a tuple of post columns.
// SELECT
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// u.DisplayName AS OwnerDisplayName,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// COUNT(DISTINCT ph.Id) AS EditCount,
// COUNT(DISTINCT b.Id) AS BadgeCount
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
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q13720(db: &'static So) -> String {
    let Post { title, creation_date, view_count, score, owner_user, .. } = &db.post;
    let key = title.opt().and(creation_date).and(view_count.opt()).and(score).and(owner_user.select(&db.user.display_name));
    let s = stats_fold(db, questions(db), &key, "cvhb", &[]);
    let h = questions(db).group_by(&key).select(history_of(db)).count_distinct();
    let b = questions(db).group_by(&key).select(owner_user.select(badges_of(db))).count_distinct();
    let mut v = Vec::new();
    s.and((&h).opt()).and((&b).opt()).drive(|k, ((s, h), b)| v.push((k, s, h.unwrap_or(0), b.unwrap_or(0))));
    v.sort_by_key(|x| Reverse(x.0.0.0.0.1));
    rows(v.iter().take(100).map(|&(((((t, c), w), sc), dn), ref s, h, b)| {
        row(vec![ostr(t), V::T(c), oint(w), V::I(sc), V::S(dn), V::I(s.cx), V::I(s.up), V::I(s.down), V::I(h), V::I(b)])
    }))
}

// SELECT
// U.DisplayName AS UserDisplayName,
// U.Reputation,
// P.Title AS PostTitle,
// P.CreationDate AS PostCreationDate,
// P.Score AS PostScore,
// COUNT(C.ID) AS CommentCount,
// SUM(V.BountyAmount) AS TotalBounty,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes,
// COUNT(DISTINCT B.Id) AS BadgeCount
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// WHERE
// P.PostTypeId = 1
// GROUP BY
// U.DisplayName, U.Reputation, P.Title, P.CreationDate, P.Score
// ORDER BY
// P.CreationDate DESC
// LIMIT 100;
fn q10618(db: &'static So) -> String {
    let Post { title, creation_date, score, owner_user, .. } = &db.post;
    let key = owner_user.select((&db.user.display_name).and(&db.user.reputation)).and(title.opt()).and(creation_date).and(score);
    let s = stats_fold(db, questions(db), &key, "cvb", &[]);
    let b = questions(db).group_by(&key).select(owner_user.select(badges_of(db))).count_distinct();
    let mut v = Vec::new();
    s.and((&b).opt()).drive(|k, (s, b)| v.push((k, s, b.unwrap_or(0))));
    v.sort_by_key(|&(((_, c), _), _, _)| Reverse(c));
    rows(v.iter().take(100).map(|&(((((dn, r), t), c), sc), ref s, b)| {
        row(vec![V::S(dn), V::I(r), ostr(t), V::T(c), V::I(sc), V::I(s.cx), nullable(s.bounty_sum, s.bounty_n), V::I(s.up), V::I(s.down), V::I(b)])
    }))
}

// SELECT
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// u.DisplayName AS OwnerDisplayName,
// u.Reputation AS OwnerReputation,
// pt.Name AS PostTypeName,
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
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName, u.Reputation, pt.Name
// ORDER BY
// p.CreationDate DESC;
fn q14409(db: &'static So) -> String {
    let Post { title, creation_date, score, view_count, owner_user, post_type, .. } = &db.post;
    let key = title.opt().and(creation_date).and(score).and(view_count.opt()).and(owner_user.select((&db.user.display_name).and(&db.user.reputation)).opt()).and(post_type.select(&db.post_type.name).opt());
    let v = group_stats(db, since(db, date(2023, 1, 1)), key, "cvb", &[]);
    rows(v.iter().map(|&((((((t, c), sc), w), u), pt), ref s)| {
        row(vec![ostr(t), V::T(c), V::I(sc), oint(w), V::I(s.cx), V::I(s.up), V::I(s.down), ostr(u.map(|u| u.0)), oint(u.map(|u| u.1)), ostr(pt), V::I(s.bclass)])
    }))
}

// SELECT
// p.Title AS PostTitle,
// u.DisplayName AS Author,
// p.CreationDate AS PostDate,
// p.Score,
// p.ViewCount,
// p.AnswerCount,
// p.CommentCount,
// COUNT(v.Id) AS NumberOfVotes,
// AVG(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS AverageUpVotes,
// AVG(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS AverageDownVotes,
// COUNT(ph.Id) AS RevisionCount,
// MAX(ph.CreationDate) AS LastEditedDate
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// WHERE
// p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'
// GROUP BY
// p.Title, u.DisplayName, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q13182(db: &'static So) -> String {
    let Post { title, creation_date, score, view_count, answer_count, comment_count, owner_user, .. } = &db.post;
    let key = title.opt().and(owner_user.select(&db.user.display_name)).and(creation_date).and(score).and(view_count.opt()).and(answer_count.opt()).and(comment_count);
    let mut v = group_stats(db, owned_since(db, year_ago()), key, "vh", &[]);
    v.sort_by_key(|x| Reverse(x.0.0.0.0.0.1));
    rows(v.iter().take(100).map(|&(((((((t, dn), c), sc), w), an), cc), ref s)| {
        let mut f = vec![ostr(t), V::S(dn), V::T(c), V::I(sc), oint(w), oint(an), V::I(cc)];
        f.extend(["#vx", "up_frac", "down_frac", "#hx", "hmax"].iter().map(|x| stat_field(s, x).unwrap()));
        row(f)
    }))
}

// GROUP BY a post and a column of one of its history rows.
fn post_hist<Q: Drive<D = Id<Post>, R = Id<Post>>>(db: &'static So, base: Q) -> MatSet<(Id<Post>, Option<Id<PostHistory>>)> {
    base.select(Ident::<Post>::new().and(history_of(db).opt())).collect()
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// COUNT(DISTINCT c.Id) AS CommentCount,
// COUNT(DISTINCT a.Id) AS AnswerCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// ph.CreationDate AS LastEditDate,
// u.DisplayName AS OwnerDisplayName
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, ph.CreationDate, u.DisplayName
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q13834(db: &'static So) -> String {
    let j = post_hist(db, all_questions(db));
    let post_of = (&j).map(|(p, _)| p);
    let hist_of = (&j).flat_map(|(_, h)| h);
    let key = (&post_of).and((&hist_of).select(&db.post_history.creation_date).opt());
    let f = (&j)
        .group_by(&key)
        .select((&post_of).select(comments_of(db).opt().and(answers_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())))
        .fold([0i64; 2], |a, ((_, _), x)| [a[0] + (x == Some(2)) as i64, a[1] + (x == Some(3)) as i64]);
    let c = (&j).group_by(&key).select((&post_of).select(comments_of(db))).count_distinct();
    let a = (&j).group_by(&key).select((&post_of).select(answers_of(db))).count_distinct();
    let mut v = Vec::new();
    f.and((&c).opt()).and((&a).opt()).drive(|(p, h), ((f, c), a)| v.push((p, h, f, c.unwrap_or(0), a.unwrap_or(0))));
    v.sort_by_key(|&(p, h, _, _, _)| (newest(db, p), db.post.origid.get(p).unwrap(), h.is_none(), h));
    rows(v.iter().take(100).map(|&(p, h, f, c, a)| {
        let mut r = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        r.extend([V::I(c), V::I(a), V::I(f[0]), V::I(f[1]), ots(h)]);
        r.extend(post_fields(db, p, &["owner"]));
        row(r)
    }))
}

// SELECT
// u.DisplayName AS UserDisplayName,
// p.Title AS PostTitle,
// p.CreationDate AS PostCreationDate,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// pt.Name AS PostTypeName,
// ph.CreationDate AS LastEditDate,
// COUNT(DISTINCT b.Id) AS BadgeCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// u.DisplayName, p.Title, p.CreationDate, pt.Name, ph.CreationDate
// ORDER BY
// p.CreationDate DESC;
fn q13736(db: &'static So) -> String {
    let Post { title, creation_date, owner_user, post_type, .. } = &db.post;
    let j = post_hist(db, owned_since(db, date(2023, 1, 1)));
    let post_of = (&j).map(|(p, _)| p);
    let hist_of = (&j).flat_map(|(_, h)| h);
    let key = (&post_of)
        .select(owner_user.select(&db.user.display_name).and(title.opt()).and(creation_date).and(post_type.select(&db.post_type.name).opt()))
        .and((&hist_of).select(&db.post_history.creation_date).opt());
    let f = (&j)
        .group_by(&key)
        .select((&post_of).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(owner_user.select(badges_of(db)).opt())))
        .fold([0i64; 3], |a, ((c, x), _)| [a[0] + c.is_some() as i64, a[1] + (x == Some(2)) as i64, a[2] + (x == Some(3)) as i64]);
    let b = (&j).group_by(&key).select((&post_of).select(owner_user.select(badges_of(db)))).count_distinct();
    let mut v = Vec::new();
    f.and((&b).opt()).drive(|k, (a, b)| v.push((k, a, b.unwrap_or(0))));
    rows(v.iter().map(|&(((((dn, t), c), pt), hd), a, b)| row(vec![V::S(dn), ostr(t), V::T(c), V::I(a[0]), V::I(a[1]), V::I(a[2]), ostr(pt), ots(hd), V::I(b)])))
}

// SELECT
// U.DisplayName AS UserDisplayName,
// P.Title AS PostTitle,
// P.CreationDate AS PostCreationDate,
// PH.CreationDate AS PostHistoryDate,
// P.Body AS PostBody,
// PH.Comment AS EditComment,
// P.Score AS PostScore,
// COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVotes,
// COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVotes,
// COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// PostHistory PH ON P.Id = PH.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// WHERE
// PH.PostHistoryTypeId IN (4, 5, 6)
// GROUP BY
// U.DisplayName, P.Title, P.CreationDate, PH.CreationDate, P.Body, PH.Comment, P.Score
// ORDER BY
// PH.CreationDate DESC
// LIMIT 100;
fn q11325(db: &'static So) -> String {
    let PostHistory { post_history_type_id, creation_date, comment, .. } = &db.post_history;
    let Post { title, creation_date: pcd, body, score, owner_user, .. } = &db.post;
    let j: MatSet<(Id<Post>, Id<PostHistory>)> =
        owned(db).select(Ident::<Post>::new().and(history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.in_v(vec![4, 5, 6]))))).collect();
    let post_of = (&j).map(|(p, _)| p);
    let key = (&post_of)
        .select(owner_user.select(&db.user.display_name).and(title.opt()).and(pcd).and(body).and(score))
        .and((&j).map(|(_, h)| h).select(creation_date.and(comment.opt())));
    let mut v = Vec::new();
    (&j).group_by(key)
        .select((&post_of).select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt())))
        .fold([0i64; 3], |a, (x, c)| [a[0] + (x == Some(2)) as i64, a[1] + (x == Some(3)) as i64, a[2] + c.is_some() as i64])
        .drive(|k, a| v.push((k, a)));
    v.sort_by_key(|&((_, (hd, _)), _)| Reverse(hd));
    rows(v.iter().take(100).map(|&(((((( dn, t), c), b), sc), (hd, hc)), a)| {
        row(vec![V::S(dn), ostr(t), V::T(c), V::T(hd), V::S(b), ostr(hc), V::I(sc), V::I(a[0]), V::I(a[1]), V::I(a[2])])
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// COUNT(DISTINCT c.Id) AS CommentCount,
// COUNT(DISTINCT b.Id) AS BadgeCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// t.TagName
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// LEFT JOIN
// PostLinks pl ON p.Id = pl.PostId
// LEFT JOIN
// Tags t ON pl.RelatedPostId = t.ExcerptPostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, t.TagName
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q13988(db: &'static So) -> String {
    let excerpt: HashIdx<i64, Id<Tag>> = (&db.tag.excerpt_post_id).inv().collect();
    let tag_of_link = (&db.post_link.related_post_id).select(&excerpt);
    let j: MatSet<(Id<Post>, Option<Id<PostLink>>, Option<Id<Tag>>)> = all_questions(db)
        .select(Ident::<Post>::new().and(links_of(db).select(Ident::<PostLink>::new().and(tag_of_link.opt())).opt()))
        .map(|(p, l)| match l {
            Some((l, t)) => (p, Some(l), t),
            None => (p, None, None),
        })
        .collect();
    let post_of = (&j).map(|(p, _, _)| p);
    let key = (&post_of).and((&j).flat_map(|(_, _, t)| t).select(&db.tag.tag_name).opt());
    let rows_ = (&post_of).select(comments_of(db).opt().and((&db.post.owner_user).select(badges_of(db)).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()));
    let f = (&j).group_by(&key).select(rows_).fold([0i64; 2], |a, (_, x)| [a[0] + (x == Some(2)) as i64, a[1] + (x == Some(3)) as i64]);
    let c = (&j).group_by(&key).select((&post_of).select(comments_of(db))).count_distinct();
    let b = (&j).group_by(&key).select((&post_of).select((&db.post.owner_user).select(badges_of(db)))).count_distinct();
    let mut v = Vec::new();
    f.and((&c).opt()).and((&b).opt()).drive(|(p, t), ((a, c), b)| v.push((p, t, a, c.unwrap_or(0), b.unwrap_or(0))));
    v.sort_by_key(|&(p, t, _, _, _)| (newest(db, p), db.post.origid.get(p).unwrap(), t.is_none(), t));
    rows(v.iter().take(100).map(|&(p, t, a, c, b)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([V::I(c), V::I(b), V::I(a[0]), V::I(a[1]), ostr(t)]);
        row(f)
    }))
}

// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// U.CreationDate,
// P.PostTypeId,
// PT.Name AS PostTypeName,
// COUNT(P.Id) AS TotalPosts,
// SUM(CASE WHEN P.Score IS NOT NULL THEN 1 ELSE 0 END) AS TotalScore,
// SUM(CASE WHEN P.ViewCount IS NOT NULL THEN P.ViewCount ELSE 0 END) AS TotalViews,
// SUM(CASE WHEN P.AnswerCount IS NOT NULL THEN P.AnswerCount ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN P.CommentCount IS NOT NULL THEN P.CommentCount ELSE 0 END) AS TotalComments
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// PostTypes PT ON P.PostTypeId = PT.Id
// GROUP BY
// U.Id, U.DisplayName, U.Reputation, U.CreationDate, P.PostTypeId, PT.Name
// ORDER BY
// U.Reputation DESC, TotalPosts DESC;
fn q11180(db: &'static So) -> String {
    let Post { post_type_id, post_type, score, view_count, answer_count, comment_count, .. } = &db.post;
    let j: MatSet<(Id<User>, Option<Id<Post>>)> = db.user.select(Ident::<User>::new().and(posts_of(db).opt())).collect();
    let post_of = (&j).flat_map(|(_, p)| p);
    let key = (&j).map(|(u, _)| u).and((&post_of).select(post_type_id.and(post_type.select(&db.post_type.name).opt())).opt());
    let mut v = Vec::new();
    (&j).group_by(key)
        .select((&post_of).select(score.and(view_count.opt()).and(answer_count.opt()).and(comment_count)).opt())
        .fold([0i64; 5], |a, p| match p {
            Some((((_, w), an), cc)) => [a[0] + 1, a[1] + 1, a[2] + w.unwrap_or(0), a[3] + an.unwrap_or(0), a[4] + cc],
            None => a,
        })
        .drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&((u, t), a)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep"), user_col(db, u, "ucreated"), oint(t.map(|t| t.0)), ostr(t.and_then(|t| t.1))];
        f.extend(a.iter().map(|&x| V::I(x)));
        row(f)
    }))
}

// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// U.CreationDate,
// P.Id AS PostId,
// P.Title,
// P.PostTypeId,
// PT.Name AS PostTypeName,
// P.Score AS PostScore,
// P.CreationDate AS PostCreationDate,
// P.ViewCount,
// COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
// COUNT(CASE WHEN V.Id IS NOT NULL THEN 1 END) AS VoteCount
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// PostTypes PT ON P.PostTypeId = PT.Id
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// U.Id, U.DisplayName, U.Reputation, U.CreationDate, P.Id, P.Title, P.PostTypeId, PT.Name, P.Score, P.CreationDate, P.ViewCount
// ORDER BY
// U.Reputation DESC, P.Score DESC;
fn q14729(db: &'static So) -> String {
    let j: MatSet<(Id<User>, Option<Id<Post>>)> = db.user.select(Ident::<User>::new().and(posts_of(db).opt())).collect();
    let post_of = (&j).flat_map(|(_, p)| p);
    let mut v = Vec::new();
    (&j).group_by(Same::new())
        .select((&post_of).select(comments_of(db).opt().and(votes_of(db).opt())).opt())
        .fold([0i64; 2], |a, r| match r {
            Some((c, x)) => [a[0] + c.is_some() as i64, a[1] + x.is_some() as i64],
            None => a,
        })
        .drive(|(u, p), a| v.push((u, p, a)));
    rows(v.iter().map(|&(u, p, a)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep"), user_col(db, u, "ucreated")];
        match p {
            Some(p) => f.extend(post_fields(db, p, &["id", "title", "type_id", "type", "score", "created", "views"])),
            None => f.extend((0..7).map(|_| V::Null)),
        }
        f.extend([V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// P.Id AS PostId,
// P.Title,
// P.CreationDate AS PostCreationDate,
// P.Score,
// P.ViewCount,
// COUNT(C.ID) AS CommentCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount,
// SUM(CASE WHEN B.Id IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// WHERE
// U.Reputation > 0
// GROUP BY
// U.Id, U.DisplayName, U.Reputation, P.Id, P.Title, P.CreationDate, P.Score, P.ViewCount
// ORDER BY
// U.Reputation DESC, P.CreationDate DESC
// LIMIT 100;
fn q14113(db: &'static So) -> String {
    let j: MatSet<(Id<User>, Option<Id<Post>>)> = user_base(db, UserWhere::RepGt(0)).select(Ident::<User>::new().and(posts_of(db).opt())).collect();
    let post_of = (&j).flat_map(|(_, p)| p);
    let user_of = (&j).map(|(u, _)| u);
    let mut v = Vec::new();
    (&j).group_by(Same::new())
        .select((&post_of).select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and((&user_of).select(badges_of(db)).opt()))
        .fold([0i64; 4], |a, (r, b)| {
            let (c, x) = match r {
                Some((c, x)) => (c.is_some(), x),
                None => (false, None),
            };
            [a[0] + c as i64, a[1] + (x == Some(2)) as i64, a[2] + (x == Some(3)) as i64, a[3] + b.is_some() as i64]
        })
        .drive(|(u, p), a| v.push((u, p, a)));
    v.sort_by_key(|&(u, p, _)| (Reverse(db.user.reputation.get(u).unwrap()), p.is_none(), Reverse(p.map(|p| db.post.creation_date.get(p).unwrap()))));
    rows(v.iter().take(100).map(|&(u, p, a)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep")];
        match p {
            Some(p) => f.extend(post_fields(db, p, &["id", "title", "created", "score", "views"])),
            None => f.extend((0..5).map(|_| V::Null)),
        }
        f.extend(a.iter().map(|&x| V::I(x)));
        row(f)
    }))
}

// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(DISTINCT P.Id) AS TotalPosts,
// COUNT(DISTINCT C.Id) AS TotalComments,
// SUM(V.BountyAmount) AS TotalBounties,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// AVG(P.Score) AS AvgPostScore,
// AVG(P.ViewCount) AS AvgPostViews,
// MAX(P.CreationDate) AS LastPostDate
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId AND V.VoteTypeId = 8
// WHERE
// U.Reputation > 100
// GROUP BY
// U.Id, U.DisplayName, U.Reputation
// ORDER BY
// TotalPosts DESC;
fn q13204(db: &'static So) -> String {
    let w = UserWhere::RepGt(100);
    let p = ud(db, w, posts_of(db));
    let c = ud(db, w, posts_of(db).select(comments_of(db)));
    let v = users_stats_with(db, w, "cv", any_post, &[8], &[&p, &c]);
    users_rows(db, v, |_, _, _| 0, 0, &["uid", "name", "rep", "#d0", "#d1", "bounty_sum", "#q", "#a", "score_avg", "views_avg", "created_max"])
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS TotalQuestions,
// COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS TotalAnswers,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes,
// SUM(CASE WHEN v.VoteTypeId = 7 THEN 1 ELSE 0 END) AS TotalReopenVotes,
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
// TotalPosts DESC
// FETCH FIRST 100 ROWS ONLY;
fn q13257(db: &'static So) -> String {
    let w = UserWhere::All;
    let Post { post_type_id, .. } = &db.post;
    let p = ud(db, w, posts_of(db));
    let q = ud(db, w, posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(1))));
    let a = ud(db, w, posts_of(db).select(Ident::<Post>::new().with(post_type_id.eq(2))));
    let v = users_stats_with(db, w, "v", any_post, &[], &[&p, &q, &a]);
    users_rows(db, v, |_, _, d| Reverse(d[0]), 100, &["uid", "name", "rep", "#d0", "#d1", "#d2", "#up", "#down", "#v7", "created_max"])
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// u.CreationDate AS UserCreationDate,
// u.LastAccessDate,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// COUNT(DISTINCT c.Id) AS TotalComments,
// COUNT(DISTINCT v.Id) AS TotalUpvotes,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// MAX(p.CreationDate) AS LastPostDate
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId AND v.VoteTypeId = 2
// GROUP BY
// u.Id, u.DisplayName, u.Reputation, u.CreationDate, u.LastAccessDate
// ORDER BY
// u.Reputation DESC, TotalPosts DESC;
fn q11427(db: &'static So) -> String {
    let w = UserWhere::All;
    let p = ud(db, w, posts_of(db));
    let c = ud(db, w, posts_of(db).select(comments_of(db)));
    let x = ud(db, w, posts_of(db).select(votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2)))));
    let v = users_stats_with(db, w, "cv", any_post, &[2], &[&p, &c, &x]);
    users_rows(db, v, |_, _, _| 0, 0, &["uid", "name", "rep", "ucreated", "last_access", "#d0", "#d1", "#d2", "#q", "#a", "created_max"])
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes,
// SUM(CASE WHEN PH.PostId IS NOT NULL THEN 1 ELSE 0 END) AS TotalPostHistories,
// AVG(p.Score) AS AverageScore,
// SUM(p.ViewCount) AS TotalViews
// FROM
// Users AS u
// LEFT JOIN
// Posts AS p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Votes AS v ON p.Id = v.PostId
// LEFT JOIN
// PostHistory AS PH ON p.Id = PH.PostId
// GROUP BY
// u.Id, u.DisplayName
// ORDER BY
// TotalPosts DESC;
fn q11969(db: &'static So) -> String {
    let v = users_stats_with(db, UserWhere::All, "vh", any_post, &[], &[]);
    users_rows(db, v, |_, _, _| 0, 0, &["uid", "name", "#n", "#q", "#a", "#up", "#down", "#hx", "score_avg", "views_sum"])
}

// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(DISTINCT P.Id) AS TotalPosts,
// COUNT(DISTINCT C.Id) AS TotalComments,
// SUM(V.BountyAmount) AS TotalBounty,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes,
// SUM(CASE WHEN PH.PostHistoryTypeId IN (10, 11, 12, 13) THEN 1 ELSE 0 END) AS TotalCloseActions
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// LEFT JOIN
// PostHistory PH ON P.Id = PH.PostId
// WHERE
// U.CreationDate >= '2023-01-01'
// GROUP BY
// U.Id, U.DisplayName, U.Reputation
// ORDER BY
// TotalPosts DESC, TotalComments DESC;
fn q14079(db: &'static So) -> String {
    let w = UserWhere::CreatedGe(date(2023, 1, 1));
    let p = ud(db, w, posts_of(db));
    let c = ud(db, w, posts_of(db).select(comments_of(db)));
    let v = users_stats_with(db, w, "cvh", any_post, &[], &[&p, &c]);
    users_rows(db, v, |_, _, _| 0, 0, &["uid", "name", "rep", "#d0", "#d1", "bounty_sum", "#up", "#down", "#hclose"])
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(p.Id) AS TotalPosts,
// COUNT(c.Id) AS TotalComments,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN b.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalBadges,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
// ORDER BY
// TotalPosts DESC, u.Reputation DESC
// LIMIT 100;
fn q10962(db: &'static So) -> String {
    let mut v = Vec::new();
    user_counts(db, UserWhere::All, "cbv").drive(|u, a| v.push((u, a)));
    out(v, |&(u, a)| (Reverse(a.n), Reverse(db.user.reputation.get(u).unwrap())), 100, |&(u, a)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend([a.n, a.cx, a.q, a.a, a.bx, a.up, a.down].map(V::I));
        f
    })
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// COUNT(DISTINCT c.Id) AS TotalComments,
// COUNT(DISTINCT v.Id) AS TotalVotes,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN p.PostTypeId IN (4, 5) THEN 1 ELSE 0 END) AS TotalTagWikis,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes,
// AVG(u.Reputation) AS AvgReputation
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// u.Id, u.DisplayName
// ORDER BY
// TotalPosts DESC
// LIMIT 100;
fn q13978(db: &'static So) -> String {
    let w = UserWhere::All;
    let p = ud(db, w, posts_of(db));
    let c = ud(db, w, posts_of(db).select(comments_of(db)));
    let x = ud(db, w, posts_of(db).select(votes_of(db)));
    let v = users_stats_with(db, w, "cv", any_post, &[], &[&p, &c, &x]);
    users_rows(db, v, |_, _, d| Reverse(d[0]), 100, &["uid", "name", "#d0", "#d1", "#d2", "#q", "#a", "#45", "#up", "#down", "rep_avg"])
}

// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// U.CreationDate,
// U.LastAccessDate,
// COUNT(P.Id) AS TotalPosts,
// COUNT(C.Id) AS TotalComments,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// COALESCE(MAX(P.LastActivityDate), '1970-01-01 00:00:00') AS LastActivity,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// U.Id, U.DisplayName, U.Reputation, U.CreationDate, U.LastAccessDate
// ORDER BY
// TotalPosts DESC
// LIMIT 100;
fn q14520(db: &'static So) -> String {
    let v = users_stats_with(db, UserWhere::All, "cv", any_post, &[], &[]);
    let mut v = v;
    v.sort_by_key(|x| Reverse(x.1.n));
    rows(v.iter().take(100).map(|(u, s, _)| {
        let mut f: Vec<V> = ["uid", "name", "rep", "ucreated", "last_access"].iter().map(|c| user_col(db, *u, c)).collect();
        f.extend(["#n", "#cx", "#q", "#a"].iter().map(|c| ustat_field(s, c)));
        f.push(V::T(if s.n == 0 { 0 } else { s.lamax }));
        f.extend(["#up", "#down"].iter().map(|c| ustat_field(s, c)));
        row(f)
    }))
}

// SELECT
// u.DisplayName AS UserDisplayName,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(p.ViewCount) AS TotalViews,
// SUM(p.Score) AS TotalScore,
// COUNT(DISTINCT c.Id) AS TotalComments,
// COUNT(DISTINCT b.Id) AS TotalBadges,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// u.Reputation > 1000
// GROUP BY
// u.DisplayName
// ORDER BY
// TotalPosts DESC
// LIMIT 100;
fn q11434(db: &'static So) -> String {
    let w = UserWhere::RepGt(1000);
    let name = &db.user.display_name;
    let f = user_stats_fold(db, name, w, "cbv", any_post);
    let p = user_distinct(db, name, w, posts_of(db));
    let c = user_distinct(db, name, w, posts_of(db).select(comments_of(db)));
    let b = user_distinct(db, name, w, badges_of(db));
    let mut v = Vec::new();
    f.and((&p).opt()).and((&c).opt()).and((&b).opt()).drive(|k, (((s, p), c), b)| v.push((k, s, [p, c, b].map(|x| x.unwrap_or(0)))));
    v.sort_by_key(|x| Reverse(x.2[0]));
    rows(v.iter().take(100).map(|(k, s, d)| {
        let mut r = vec![V::S(k), V::I(d[0])];
        r.extend(["#q", "#a", "views_sum", "score_sum"].iter().map(|c| ustat_field(s, c)));
        r.extend([V::I(d[1]), V::I(d[2])]);
        r.extend(["#up", "#down"].iter().map(|c| ustat_field(s, c)));
        row(r)
    }))
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(DISTINCT p.Id) AS PostCount,
// COUNT(DISTINCT a.Id) AS AcceptedAnswers,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN p.PostTypeId IN (4, 5) THEN 1 ELSE 0 END) AS TagWikiCount,
// AVG(p.Score) AS AvgPostScore,
// SUM(p.ViewCount) AS TotalViews
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Posts a ON p.Id = a.AcceptedAnswerId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
// ORDER BY
// PostCount DESC;
fn q14779(db: &'static So) -> String {
    let accepting: HashIdx<Id<Post>, Id<Post>> = (&db.post.accepted_answer).inv().collect();
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let rows_ = user_base(db, UserWhere::All)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(view_count.opt()).and((&accepting).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 9], |a, r| match r {
            Some(((((t, s), w), _), x)) => [
                a[0] + 1,
                a[1] + (x == Some(2)) as i64,
                a[2] + (x == Some(3)) as i64,
                a[3] + (t == 1) as i64,
                a[4] + (t == 2) as i64,
                a[5] + (t == 4 || t == 5) as i64,
                a[6] + s,
                a[7] + w.unwrap_or(0),
                a[8] + w.is_some() as i64,
            ],
            None => a,
        });
    let p = ud(db, UserWhere::All, posts_of(db));
    let acc = ud(db, UserWhere::All, posts_of(db).select(&accepting));
    let mut v = Vec::new();
    rows_.and((&p).opt()).and((&acc).opt()).drive(|u, ((a, p), c)| v.push((u, a, p.unwrap_or(0), c.unwrap_or(0))));
    rows(v.iter().map(|&(u, a, p, c)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep"), V::I(p), V::I(c)];
        f.extend(a[1..6].iter().map(|&x| V::I(x)));
        f.extend([avg(a[6], a[0]), nullable(a[7], a[8])]);
        row(f)
    }))
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes,
// COUNT(DISTINCT t.Id) AS TotalUniqueTags,
// AVG(u.Reputation) AS AverageReputation,
// MAX(p.CreationDate) AS LastPostDate
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// PostLinks pl ON p.Id = pl.PostId
// LEFT JOIN
// Tags t ON pl.RelatedPostId = t.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// u.Id, u.DisplayName
// ORDER BY
// TotalPosts DESC, AverageReputation DESC;
fn q12603(db: &'static So) -> String {
    let tag_by_raw: HashIdx<i64, Id<Tag>> = (&db.tag.origid).inv().collect();
    let tag_of_link = (&db.post_link.related_post_id).select(&tag_by_raw);
    let Post { post_type_id, creation_date, .. } = &db.post;
    let rows_ = user_base(db, UserWhere::All)
        .group_by(Ident::<User>::new())
        .select((&db.user.reputation).and(posts_of(db).select(post_type_id.and(creation_date).and(links_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt()))
        .fold([0, 0, 0, 0, 0, 0, i64::MIN, 0], |a: [i64; 8], (r, p)| {
            let mut a = a;
            a[4] += r;
            a[5] += 1;
            if let Some((((t, c), _), x)) = p {
                a[0] += (t == 1) as i64;
                a[1] += (t == 2) as i64;
                a[2] += (x == Some(2)) as i64;
                a[3] += (x == Some(3)) as i64;
                a[6] = a[6].max(c);
                a[7] += 1;
            }
            a
        });
    let p = ud(db, UserWhere::All, posts_of(db));
    let t = ud(db, UserWhere::All, posts_of(db).select(links_of(db).select(&tag_of_link)));
    let mut v = Vec::new();
    rows_.and((&p).opt()).and((&t).opt()).drive(|u, ((a, p), t)| v.push((u, a, p.unwrap_or(0), t.unwrap_or(0))));
    rows(v.iter().map(|&(u, a, p, t)| {
        row(vec![
            user_col(db, u, "uid"),
            user_col(db, u, "name"),
            V::I(p),
            V::I(a[0]),
            V::I(a[1]),
            V::I(a[2]),
            V::I(a[3]),
            V::I(t),
            avg(a[4], a[5]),
            if a[7] == 0 { V::Null } else { V::T(a[6]) },
        ])
    }))
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// COUNT(DISTINCT c.Id) AS TotalComments,
// COUNT(DISTINCT v.Id) AS TotalVotes,
// COUNT(DISTINCT b.Id) AS TotalBadges
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// LEFT JOIN
// Comments c ON u.Id = c.UserId AND c.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// LEFT JOIN
// Votes v ON u.Id = v.UserId AND v.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// LEFT JOIN
// Badges b ON u.Id = b.UserId AND b.Date >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
// ORDER BY
// u.Reputation DESC;
fn q14381(db: &'static So) -> String {
    let t = year_ago();
    let w = UserWhere::All;
    let p = ud(db, w, posts_of(db).select(Ident::<Post>::new().with((&db.post.creation_date).ge(t))));
    let c = ud(db, w, comments_by(db).select(Ident::<Comment>::new().with((&db.comment.creation_date).ge(t))));
    let x = ud(db, w, votes_by(db).select(Ident::<Vote>::new().with((&db.vote.creation_date).ge(t))));
    let b = ud(db, w, badges_of(db).select(Ident::<Badge>::new().with((&db.badge.date).ge(t))));
    let mut v = Vec::new();
    db.user.select((&p).opt().and((&c).opt()).and((&x).opt()).and((&b).opt())).drive(|u, (((p, c), x), b)| v.push((u, [p, c, x, b].map(|n| n.unwrap_or(0)))));
    rows(v.iter().map(|&(u, d)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(d.iter().map(|&n| V::I(n)));
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate AS PostCreationDate,
// u.Id AS UserId,
// u.DisplayName AS UserName,
// u.Reputation,
// p.ViewCount,
// p.Score,
// p.AnswerCount,
// p.CommentCount,
// COUNT(v.Id) AS TotalVotes,
// AVG(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS AvgUpvotes,
// AVG(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS AvgDownvotes
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= CURRENT_DATE - INTERVAL '6 month'
// GROUP BY
// p.Id, p.Title, p.CreationDate, u.Id, u.DisplayName, u.Reputation, p.ViewCount, p.Score, p.AnswerCount, p.CommentCount
// ORDER BY
// p.CreationDate DESC
// FETCH FIRST 100 ROWS ONLY;
fn q12058(db: &'static So) -> String {
    stats_rows(db, stats_with(db, owned_since(db, add_months(current_date(), -6)), "v", &[], &[]), |p, _| newest(db, p), 100, &["id", "title", "created", "uid", "owner", "rep", "views", "score", "answers", "comments", "#vx", "up_frac", "down_frac"])
}

#[derive(Clone, Copy, Default)]
struct UC {
    n: i64,
    q: i64,
    a: i64,
    t38: i64,
    cx: i64,
    up: i64,
    down: i64,
    bx: i64,
    gold: i64,
    silver: i64,
    bronze: i64,
    views_sum: i64,
}

fn user_counts(db: &'static So, w: UserWhere, joins: &str) -> DenseFold<Id<User>, UC> {
    let (c, v, b) = (joins.contains('c'), joins.contains('v'), joins.contains('b'));
    let post = (&db.post.post_type_id)
        .and((&db.post.view_count).opt())
        .and(comments_of_if(db, c).opt())
        .and(votes_of_if(db, v).select(&db.vote.vote_type_id).opt());
    user_base(db, w)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post).opt().and(badges_of_if(db, b).select(&db.badge.class).opt()))
        .dense_fold(db.user.id.n, UC::default(), |mut a, (p, b)| {
            if let Some((((t, w), c), v)) = p {
                a.n += 1;
                a.q += (t == 1) as i64;
                a.a += (t == 2) as i64;
                a.t38 += (3..=8).contains(&t) as i64;
                a.views_sum += w.unwrap_or(0);
                a.cx += c.is_some() as i64;
                a.up += (v == Some(2)) as i64;
                a.down += (v == Some(3)) as i64;
            }
            if let Some(cls) = b {
                a.bx += 1;
                a.gold += (cls == 1) as i64;
                a.silver += (cls == 2) as i64;
                a.bronze += (cls == 3) as i64;
            }
            a
        })
}

fn out<X, T: Ord>(mut v: Vec<X>, key: impl Fn(&X) -> T, n: usize, f: impl Fn(&X) -> Vec<V>) -> String {
    v.sort_by_key(|x| key(x));
    let n = if n == 0 { v.len() } else { n };
    if n < v.len() && key(&v[n - 1]) == key(&v[n]) {
        eprintln!("tie at the LIMIT cut");
    }
    rows(v.iter().take(n).map(|x| row(f(x))))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("13674", q13674),
    ("11489", q11489),
    ("10150", q10150),
    ("10899", q10899),
    ("11649", q11649),
    ("10846", q10846),
    ("14952", q14952),
    ("11892", q11892),
    ("10351", q10351),
    ("11712", q11712),
    ("13124", q13124),
    ("12617", q12617),
    ("10193", q10193),
    ("14785", q14785),
    ("10427", q10427),
    ("14931", q14931),
    ("10539", q10539),
    ("10288", q10288),
    ("12261", q12261),
    ("13426", q13426),
    ("10747", q10747),
    ("13629", q13629),
    ("14378", q14378),
    ("11765", q11765),
    ("14687", q14687),
    ("12919", q12919),
    ("14423", q14423),
    ("14935", q14935),
    ("12811", q12811),
    ("10380", q10380),
    ("13720", q13720),
    ("10618", q10618),
    ("14409", q14409),
    ("13182", q13182),
    ("13834", q13834),
    ("13736", q13736),
    ("11325", q11325),
    ("13988", q13988),
    ("11180", q11180),
    ("14729", q14729),
    ("14113", q14113),
    ("13204", q13204),
    ("13257", q13257),
    ("11427", q11427),
    ("11969", q11969),
    ("14079", q14079),
    ("10962", q10962),
    ("13978", q13978),
    ("14520", q14520),
    ("11434", q11434),
    ("14779", q14779),
    ("12603", q12603),
    ("14381", q14381),
    ("12058", q12058),
];
