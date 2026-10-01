use harness::prelude::*;
use std::cmp::Reverse;

fn cd(db: &'static So, p: Id<Post>) -> Reverse<i64> {
    Reverse(db.post.creation_date.get(p).unwrap())
}

fn questions(db: &'static So) -> impl Drive<D = Id<Post>, R = Id<Post>> {
    owned(db).with((&db.post.post_type_id).eq(1))
}

fn owned_since(db: &'static So, d: i64) -> impl Drive<D = Id<Post>, R = Id<Post>> {
    owned(db).with((&db.post.creation_date).ge(d))
}

const YEAR_AGO: fn() -> i64 = || ts(2023, 10, 1, 12, 34, 56);
const MONTH_AGO: fn() -> i64 = || ts(2024, 9, 1, 12, 34, 56);

// Per-post aggregates joined to a COUNT(DISTINCT ...) per post, newest first.
fn with_distinct<Q>(db: &'static So, base: Q, joins: &str, vtypes: &'static [i64], d: &[Fold<Id<Post>, i64>]) -> Vec<(Id<Post>, Stats, [i64; 2])>
where
    Q: Drive<D = Id<Post>, R = Id<Post>>,
{
    let empty = per_post_distinct(db, children_of_if(db, false));
    let (a, b) = (d.first().unwrap_or(&empty), d.get(1).unwrap_or(&empty));
    let mut v = Vec::new();
    stats_fold(db, base, Ident::<Post>::new(), joins, vtypes).and(a.opt()).and(b.opt()).drive(|p, ((s, x), y)| v.push((p, s, [x.unwrap_or(0), y.unwrap_or(0)])));
    v
}

fn newest_rows(db: &'static So, mut v: Vec<(Id<Post>, Stats, [i64; 2])>, n: usize, cols: &[&str]) -> String {
    v.sort_by_key(|&(p, _, _)| cd(db, p));
    rows(v.iter().take(n).map(|&(p, ref s, d)| {
        row(cols.iter().map(|c| match *c {
            "#d0" => V::I(d[0]),
            "#d1" => V::I(d[1]),
            _ => stat_fields(db, p, s, &[c]).pop().unwrap(),
        }).collect())
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// COUNT(c.Id) AS CommentCount,
// u.Reputation AS OwnerReputation,
// u.DisplayName AS OwnerDisplayName
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// WHERE
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.Reputation, u.DisplayName
// ORDER BY
// p.CreationDate DESC;
fn q10551(db: &'static So) -> String {
    stat_rows(db, post_stats(db, owned_since(db, YEAR_AGO()), "c", &[]), |p, _| cd(db, p), 0, &["id", "title", "created", "score", "views", "#cx", "rep", "owner"])
}

// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.Score,
// P.ViewCount,
// P.AnswerCount,
// U.DisplayName AS OwnerDisplayName,
// U.Reputation AS OwnerReputation,
// (SELECT COUNT(*) FROM Votes V WHERE V.PostId = P.Id AND V.VoteTypeId = 2) AS UpVoteCount,
// (SELECT COUNT(*) FROM Votes V WHERE V.PostId = P.Id AND V.VoteTypeId = 3) AS DownVoteCount
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// WHERE
// P.PostTypeId = 1
// ORDER BY
// P.Score DESC, P.ViewCount DESC
// LIMIT 100;
fn q13943(db: &'static So) -> String {
    let mut v = Vec::new();
    questions(db).select(votes_of_type(db, 2).and(votes_of_type(db, 3))).drive(|p, a| v.push((p, a)));
    v.sort_by_key(|&(p, _)| (Reverse(db.post.score.get(p).unwrap()), db.post.view_count.get(p).is_none(), Reverse(db.post.view_count.get(p))));
    rows(v.iter().take(100).map(|&(p, (u, d))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "owner", "rep"]);
        f.extend([V::I(u), V::I(d)]);
        row(f)
    }))
}

// SELECT
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
// Comments c ON c.PostId = p.Id
// LEFT JOIN
// Votes v ON v.PostId = p.Id
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Title, p.CreationDate, u.DisplayName
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q19984(db: &'static So) -> String {
    tuple_rows(by_name_title_date(db, true, "cv", PostWhere::All), "created", 10, &["title", "created", "owner", "#cx", "#up", "#down"])
}

// SELECT
// p.Id AS PostID,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// COALESCE(u.DisplayName, 'Community User') AS OwnerDisplayName,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount
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
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q14830(db: &'static So) -> String {
    let base = db.post.with((&db.post.post_type_id).eq(1));
    let mut v = post_stats(db, base, "cv", &[]);
    v.sort_by_key(|&(p, _)| cd(db, p));
    rows(v.iter().take(100).map(|(p, s)| {
        let mut f = post_fields(db, *p, &["id", "title", "created", "score", "views"]);
        f.push(V::S(db.post.owner_user.get(*p).map_or("Community User", |u| db.user.display_name.get(u).unwrap())));
        f.extend([V::I(s.cx), V::I(s.vx)]);
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// u.DisplayName AS OwnerDisplayName,
// COUNT(DISTINCT c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
// MAX(b.Name) AS BadgeName
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
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, u.DisplayName
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q11928(db: &'static So) -> String {
    let d = [per_post_distinct(db, comments_of(db)), per_post_distinct(db, votes_of(db))];
    newest_rows(db, with_distinct(db, owned(db), "cvb", &[], &d), 100, &["id", "title", "created", "score", "owner", "#d0", "#d1", "bname"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// u.DisplayName AS OwnerDisplayName,
// u.Reputation AS OwnerReputation,
// COUNT(c.Id) AS CommentCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// WHERE
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName, u.Reputation
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q12816(db: &'static So) -> String {
    newest_rows(db, with_distinct(db, owned_since(db, MONTH_AGO()), "c", &[], &[]), 100, &["id", "title", "created", "views", "score", "owner", "rep", "#cx"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// COUNT(c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
// u.Reputation AS OwnerReputation,
// u.DisplayName AS OwnerDisplayName
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.Reputation, u.DisplayName
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q12255(db: &'static So) -> String {
    let d = [per_post_distinct(db, votes_of(db))];
    newest_rows(db, with_distinct(db, db.post.iq(), "cv", &[], &d), 100, &["id", "title", "created", "score", "views", "#cx", "#d0", "rep", "owner"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// COUNT(DISTINCT c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
// u.DisplayName AS OwnerDisplayName,
// p.CreationDate,
// p.LastActivityDate,
// p.ViewCount,
// p.Score
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// GROUP BY
// p.Id, p.Title, u.DisplayName, p.CreationDate, p.LastActivityDate, p.ViewCount, p.Score
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q14050(db: &'static So) -> String {
    let d = [per_post_distinct(db, comments_of(db)), per_post_distinct(db, votes_of(db))];
    newest_rows(db, with_distinct(db, db.post.iq(), "cv", &[], &d), 100, &["id", "title", "#d0", "#d1", "owner", "created", "activity", "views", "score"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// u.DisplayName AS OwnerDisplayName,
// COUNT(c.Id) AS CommentCount,
// COUNT(CASE WHEN v.VoteTypeId = 2 THEN v.Id END) AS UpVoteCount,
// COUNT(CASE WHEN v.VoteTypeId = 3 THEN v.Id END) AS DownVoteCount
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
fn q15336(db: &'static So) -> String {
    newest_rows(db, with_distinct(db, questions(db), "cv", &[], &[]), 10, &["id", "title", "created", "owner", "#cx", "#up", "#down"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// u.DisplayName AS OwnerDisplayName,
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
// GROUP BY
// p.Id, p.Title, p.CreationDate, u.DisplayName
// ORDER BY
// p.CreationDate DESC
// OFFSET 0 ROWS FETCH NEXT 10 ROWS ONLY;
fn q15590(db: &'static So) -> String {
    newest_rows(db, with_distinct(db, questions(db), "cv", &[], &[]), 10, &["id", "title", "created", "owner", "#cx", "#up", "#down"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q10531(db: &'static So) -> String {
    let base = db.post.with((&db.post.creation_date).ge(date(2023, 1, 1)));
    newest_rows(db, with_distinct(db, base, "cv", &[], &[]), 100, &["id", "title", "created", "views", "score", "#cx", "#vx", "#up", "#down"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q12547(db: &'static So) -> String {
    let base = db.post.with((&db.post.creation_date).ge(date(2023, 1, 1)));
    newest_rows(db, with_distinct(db, base, "cv", &[], &[]), 100, &["id", "title", "created", "score", "views", "#cx", "#vx", "#up", "#down"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate AS PostCreationDate,
// u.Id AS UserId,
// u.DisplayName AS UserDisplayName,
// u.Reputation,
// p.Score AS PostScore,
// p.ViewCount,
// COUNT(v.Id) AS VoteCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY
// p.Id, p.Title, p.CreationDate, u.Id, u.DisplayName, u.Reputation, p.Score, p.ViewCount
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q14124(db: &'static So) -> String {
    newest_rows(db, with_distinct(db, owned_since(db, YEAR_AGO()), "v", &[], &[]), 100, &["id", "title", "created", "uid", "owner", "rep", "score", "views", "#vx"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
// u.DisplayName AS OwnerDisplayName,
// p.CreationDate
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
// p.Id, p.Title, u.DisplayName, p.CreationDate
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q19783(db: &'static So) -> String {
    let base = db.post.with((&db.post.post_type_id).eq(1));
    newest_rows(db, with_distinct(db, base, "cv", &[], &[]), 10, &["id", "title", "#cx", "#up", "#down", "owner", "created"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// u.DisplayName AS OwnerDisplayName,
// COUNT(c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
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
// Badges b ON u.Id = b.UserId
// WHERE
// p.PostTypeId IN (1, 2)
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q12197(db: &'static So) -> String {
    let base = db.post.with((&db.post.post_type_id).in_v(vec![1, 2]));
    let d = [per_post_distinct(db, votes_of(db)), per_post_distinct(db, (&db.post.owner_user).select(badges_of(db)))];
    newest_rows(db, with_distinct(db, base, "cvb", &[], &d), 100, &["id", "title", "created", "score", "views", "owner", "#cx", "#d0", "#d1"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// p.AnswerCount,
// u.DisplayName AS OwnerDisplayName,
// COUNT(c.Id) AS CommentCount,
// MAX(v.CreationDate) AS LastVoteDate,
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
// Badges b ON u.Id = b.UserId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, u.DisplayName
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q14584(db: &'static So) -> String {
    let d = [per_post_distinct(db, (&db.post.owner_user).select(badges_of(db)))];
    newest_rows(db, with_distinct(db, owned(db), "cvb", &[], &d), 100, &["id", "title", "created", "views", "score", "answers", "owner", "#cx", "vmax", "#d0"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// u.DisplayName AS OwnerDisplayName,
// COUNT(co.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments co ON p.Id = co.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q10850(db: &'static So) -> String {
    let base = db.post.with((&db.post.post_type_id).eq(1));
    newest_rows(db, with_distinct(db, base, "cv", &[], &[]), 100, &["id", "title", "created", "score", "views", "owner", "#cx", "#up", "#down"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// u.DisplayName AS OwnerDisplayName,
// COUNT(c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
// COUNT(DISTINCT bh.Id) AS HistoryCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// PostHistory bh ON p.Id = bh.PostId
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q10948(db: &'static So) -> String {
    let d = [per_post_distinct(db, votes_of(db)), per_post_distinct(db, history_of(db))];
    newest_rows(db, with_distinct(db, owned_since(db, date(2023, 1, 1)), "cvh", &[], &d), 100, &["id", "title", "created", "score", "views", "owner", "#cx", "#d0", "#d1"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// u.DisplayName AS OwnerDisplayName,
// COUNT(DISTINCT c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
// MAX(b.Date) AS LastBadgeDate,
// p.Tags
// FROM
// Posts p
// LEFT JOIN
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
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName, p.Tags
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q11196(db: &'static So) -> String {
    let base = db.post.with((&db.post.post_type_id).eq(1));
    let d = [per_post_distinct(db, comments_of(db)), per_post_distinct(db, votes_of(db))];
    newest_rows(db, with_distinct(db, base, "cvb", &[], &d), 100, &["id", "title", "created", "score", "views", "owner", "#d0", "#d1", "bmax", "tags"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// u.DisplayName AS OwnerName,
// COUNT(DISTINCT a.Id) AS AnswerCount,
// COUNT(DISTINCT c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
// p.Score,
// p.ViewCount
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, p.CreationDate, u.DisplayName, p.Score, p.ViewCount
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q12304(db: &'static So) -> String {
    let base = db.post.with((&db.post.post_type_id).eq(1));
    let a = db.post.group_by(Ident::<Post>::new()).select(answers_of(db)).count_distinct();
    let c = per_post_distinct(db, comments_of(db));
    let x = per_post_distinct(db, votes_of(db));
    let mut v = Vec::new();
    base.select((&a).opt().and((&c).opt()).and((&x).opt())).drive(|p, ((a, c), x)| v.push((cd(db, p), p, [a, c, x].map(|n| n.unwrap_or(0)))));
    v.sort_by_key(|x| x.0);
    rows(v.iter().take(100).map(|&(_, p, n)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend(n.iter().map(|&x| V::I(x)));
        f.extend(post_fields(db, p, &["score", "views"]));
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// u.DisplayName AS OwnerDisplayName,
// COUNT(c.id) AS CommentCount,
// SUM(v.BountyAmount) AS TotalBounty,
// COUNT(DISTINCT b.Id) AS BadgeCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (8, 9)
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// WHERE
// p.CreationDate >= '2022-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, u.DisplayName
// ORDER BY
// p.Score DESC, CommentCount DESC
// LIMIT 100;
fn q13592(db: &'static So) -> String {
    let d = [per_post_distinct(db, (&db.post.owner_user).select(badges_of(db)))];
    let mut v = with_distinct(db, owned_since(db, date(2022, 1, 1)), "cvb", &[8, 9], &d);
    v.sort_by_key(|&(p, ref s, _)| (Reverse(db.post.score.get(p).unwrap()), Reverse(s.cx)));
    rows(v.iter().take(100).map(|&(p, ref s, d)| {
        let mut f = stat_fields(db, p, s, &["id", "title", "created", "score", "owner", "#cx", "bounty_sum"]);
        f.push(V::I(d[0]));
        row(f)
    }))
}

// SELECT
// P.Id AS PostId,
// P.Title,
// U.DisplayName AS Owner,
// P.CreationDate,
// COUNT(CF.Id) AS CommentCount,
// COUNT(V.Id) AS VoteCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// P.ViewCount
// FROM
// Posts P
// LEFT JOIN
// Comments CF ON P.Id = CF.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// JOIN
// Users U ON P.OwnerUserId = U.Id
// WHERE
// P.CreationDate >= '2022-01-01'
// GROUP BY
// P.Id, P.Title, U.DisplayName, P.CreationDate, P.ViewCount
// ORDER BY
// P.CreationDate DESC;
fn q12112(db: &'static So) -> String {
    let v = post_stats(db, owned_since(db, date(2022, 1, 1)), "cv", &[]);
    rows(v.iter().map(|(p, s)| row(stat_fields(db, *p, s, &["id", "title", "owner", "created", "#cx", "#vx", "#up", "#down", "views"]))))
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
// COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVotes,
// COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVotes
// FROM
// Posts P
// LEFT JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// WHERE
// P.CreationDate >= '2021-01-01'
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.ViewCount, P.Score, P.AnswerCount, P.CommentCount, U.DisplayName
// ORDER BY
// P.Score DESC, P.ViewCount DESC;
fn q13271(db: &'static So) -> String {
    let v = post_stats(db, db.post.with((&db.post.creation_date).ge(date(2021, 1, 1))), "v", &[]);
    rows(v.iter().map(|(p, s)| row(stat_fields(db, *p, s, &["id", "title", "created", "views", "score", "answers", "comments", "owner", "#up", "#down"]))))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// COALESCE(a.Id, 0) AS AcceptedAnswerId,
// u.DisplayName AS OwnerDisplayName
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Posts a ON p.AcceptedAnswerId = a.Id
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, a.Id, u.DisplayName
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q13792(db: &'static So) -> String {
    let base = db.post.with((&db.post.post_type_id).eq(1));
    let mut v = post_stats(db, base, "cv", &[]);
    v.sort_by_key(|&(p, _)| cd(db, p));
    rows(v.iter().take(100).map(|(p, s)| {
        let mut f = stat_fields(db, *p, s, &["id", "title", "created", "views", "score", "#cx", "#vx"]);
        f.push(V::I(db.post.accepted_answer.get(*p).map_or(0, |a| db.post.origid.get(a).unwrap())));
        f.extend(post_fields(db, *p, &["owner"]));
        row(f)
    }))
}

// SELECT
// p.Id AS PostID,
// p.Title,
// p.ViewCount,
// p.Score,
// p.CreationDate,
// p.AnswerCount,
// u.Reputation AS OwnerReputation,
// COUNT(v.Id) AS VoteCount,
// AVG(CASE WHEN v.VoteTypeId = 2 THEN 1.0 ELSE 0.0 END) AS AverageUpVotes,
// AVG(CASE WHEN v.VoteTypeId = 3 THEN 1.0 ELSE 0.0 END) AS AverageDownVotes
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Id, p.Title, p.ViewCount, p.Score, p.CreationDate, p.AnswerCount, u.Reputation
// ORDER BY
// p.CreationDate DESC;
fn q11409(db: &'static So) -> String {
    let v = post_stats(db, owned_since(db, date(2023, 1, 1)), "v", &[]);
    rows(v.iter().map(|(p, s)| {
        let mut f = stat_fields(db, *p, s, &["id", "title", "views", "score", "created", "answers", "rep", "#vx"]);
        f.extend([V::F(s.up as f64 / s.rows as f64), V::F(s.down as f64 / s.rows as f64)]);
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// (SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id) AS CommentCount,
// (SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId IN (2, 3)) AS VoteCount,
// (SELECT COUNT(*) FROM PostHistory ph WHERE ph.PostId = p.Id) AS HistoryCount,
// u.DisplayName AS OwnerDisplayName,
// u.Reputation AS OwnerReputation
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.CreationDate >= '2023-01-01'
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q14659(db: &'static So) -> String {
    let Vote { vote_type_id, post, .. } = &db.vote;
    let votes23 = db.vote.with(vote_type_id.in_v(vec![2, 3])).select(post).inv().dense_fold_outer(db.post.id.n, 0i64, |a, _| a + 1);
    let mut v = Vec::new();
    owned_since(db, date(2023, 1, 1)).select(comments_per_post(db).and(&votes23).and(history_per_post(db))).drive(|p, a| v.push((cd(db, p), p, a)));
    v.sort_by_key(|x| x.0);
    rows(v.iter().take(100).map(|&(_, p, ((c, x), h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(c), V::I(x), V::I(h)]);
        f.extend(post_fields(db, p, &["owner", "rep"]));
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
// (SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id) AS VoteCount,
// (SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id) AS CommentCount,
// (SELECT COUNT(*) FROM PostHistory ph WHERE ph.PostId = p.Id) AS HistoryCount,
// (SELECT COUNT(*) FROM Badges b WHERE b.UserId = p.OwnerUserId) AS BadgeCount
// FROM
// Posts p
// INNER JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.PostTypeId = 1
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q13848(db: &'static So) -> String {
    let bb = badges_per_user(db);
    let mut v = Vec::new();
    questions(db)
        .select(votes_per_post(db).and(comments_per_post(db)).and(history_per_post(db)).and((&db.post.owner_user).select(&bb)))
        .drive(|p, a| v.push((cd(db, p), p, a)));
    v.sort_by_key(|x| x.0);
    rows(v.iter().take(100).map(|&(_, p, (((x, c), h), b))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "owner"]);
        f.extend([V::I(x), V::I(c), V::I(h), V::I(b)]);
        row(f)
    }))
}

// SELECT
// P.Id AS PostId,
// P.Title,
// P.Score,
// P.ViewCount,
// P.AnswerCount,
// P.CommentCount,
// P.CreationDate,
// U.DisplayName AS OwnerDisplayName,
// (SELECT COUNT(*)
// FROM Comments C
// WHERE C.PostId = P.Id) AS TotalComments,
// (SELECT COUNT(*)
// FROM Votes V
// WHERE V.PostId = P.Id AND V.VoteTypeId = 2) AS UpVotes,
// (SELECT COUNT(*)
// FROM Votes V
// WHERE V.PostId = P.Id AND V.VoteTypeId = 3) AS DownVotes
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// WHERE
// P.PostTypeId = 1
// ORDER BY
// P.CreationDate DESC
// LIMIT 100;
fn q12531(db: &'static So) -> String {
    let mut v = Vec::new();
    questions(db).select(comments_per_post(db).and(votes_of_type(db, 2)).and(votes_of_type(db, 3))).drive(|p, a| v.push((cd(db, p), p, a)));
    v.sort_by_key(|x| x.0);
    rows(v.iter().take(100).map(|&(_, p, ((c, u), d))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "answers", "comments", "created", "owner"]);
        f.extend([V::I(c), V::I(u), V::I(d)]);
        row(f)
    }))
}

// SELECT
// u.DisplayName AS UserDisplayName,
// p.Title AS PostTitle,
// p.CreationDate AS PostCreationDate,
// ph.CreationDate AS HistoryCreationDate,
// p.Score AS PostScore,
// ph.Comment AS EditComment,
// p.ViewCount,
// p.AnswerCount,
// (SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 2) AS UpVoteCount,
// (SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 3) AS DownVoteCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// JOIN
// PostHistory ph ON p.Id = ph.PostId
// WHERE
// ph.PostHistoryTypeId IN (4, 5, 6)
// ORDER BY
// ph.CreationDate DESC
// LIMIT 100;
fn q11622(db: &'static So) -> String {
    let PostHistory { post_history_type_id, creation_date, comment, .. } = &db.post_history;
    let mut v = Vec::new();
    owned(db)
        .select(history_of(db).select(post_history_type_id.in_v(vec![4, 5, 6]).and(creation_date).and(comment.opt())).and(votes_of_type(db, 2)).and(votes_of_type(db, 3)))
        .drive(|p, a| v.push((p, a)));
    v.sort_by_key(|&(_, ((((_, d), _), _), _))| Reverse(d));
    rows(v.iter().take(100).map(|&(p, ((((_, hd), hc), u), d))| {
        row(vec![
            post_fields(db, p, &["owner"]).pop().unwrap(),
            title(db, p),
            V::T(db.post.creation_date.get(p).unwrap()),
            V::T(hd),
            V::I(db.post.score.get(p).unwrap()),
            ostr(hc),
            oint(db.post.view_count.get(p)),
            oint(db.post.answer_count.get(p)),
            V::I(u),
            V::I(d),
        ])
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
// b.Id AS BadgeId,
// b.Name AS BadgeName,
// b.Class AS BadgeClass,
// b.Date AS BadgeDate
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// ORDER BY
// p.CreationDate DESC,
// u.Reputation DESC;
fn q11144(db: &'static So) -> String {
    let Badge { origid, name, class, date, .. } = &db.badge;
    let mut v = Vec::new();
    owned_since(db, YEAR_AGO()).select((&db.post.owner_user).select(badges_of(db).select(origid.and(name).and(class).and(date)).opt())).drive(|p, b| v.push((p, b)));
    rows(v.iter().map(|&(p, b)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "uid", "owner", "rep"]);
        match b {
            Some((((i, n), c), d)) => f.extend([V::I(i), V::S(n), V::I(c), V::T(d)]),
            None => f.extend([V::Null, V::Null, V::Null, V::Null]),
        }
        row(f)
    }))
}

// Votes joined with `LEFT JOIN VoteTypes vt`, counted by vt.Name.
fn by_vote_type_name<Q, K>(db: &'static So, base: Q, key: K) -> Vec<(ROf<K>, (i64, i64, i64, i64))>
where
    Q: Drive<D = Id<Post>, R = Id<Post>>,
    K: IntoQuery,
    K::Q: Probe<D = Id<Post>>,
    ROf<K>: Eq + std::hash::Hash + Copy,
{
    let mut v = Vec::new();
    base.group_by(key)
        .select(comments_of(db).opt().and(votes_of(db).select((&db.vote.vote_type).select(&db.vote_type.name).opt()).opt()))
        .fold((0i64, 0i64, 0i64, 0i64), |(c, x, u, d), (ci, vi)| {
            let n = vi.flatten();
            (c + ci.is_some() as i64, x + vi.is_some() as i64, u + (n == Some("UpMod")) as i64, d + (n == Some("DownMod")) as i64)
        })
        .drive(|k, a| v.push((k, a)));
    v
}

// SELECT
// p.Title,
// p.CreationDate,
// u.DisplayName AS OwnerDisplayName,
// COUNT(c.Id) AS CommentCount,
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
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Title, p.CreationDate, u.DisplayName
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q18626(db: &'static So) -> String {
    let Post { title, creation_date, owner_user, .. } = &db.post;
    let mut v = by_vote_type_name(db, questions(db), title.opt().and(creation_date).and(owner_user.select(&db.user.display_name)));
    v.sort_by_key(|&(((_, c), _), _)| Reverse(c));
    rows(v.iter().take(10).map(|&(((t, c), dn), (n, _, u, d))| row(vec![ostr(t), V::T(c), V::S(dn), V::I(n), V::I(u), V::I(d)])))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// u.DisplayName AS OwnerName,
// COUNT(v.Id) AS VoteCount,
// SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes,
// AVG(u.Reputation) AS AverageUserReputation,
// MAX(p.LastActivityDate) AS LastActivity
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// VoteTypes vt ON v.VoteTypeId = vt.Id
// GROUP BY
// p.Id, p.Title, p.CreationDate, u.DisplayName
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q13328(db: &'static So) -> String {
    let Post { owner_user, last_activity_date, .. } = &db.post;
    let mut v = Vec::new();
    owned(db)
        .group_by(Ident::<Post>::new())
        .select(owner_user.select(&db.user.reputation).and(last_activity_date).and(votes_of(db).select((&db.vote.vote_type).select(&db.vote_type.name).opt()).opt()))
        .fold((0i64, 0i64, 0i64, 0i64, 0i64, i64::MIN), |(n, x, u, d, r, la), ((rep, a), vi)| {
            let t = vi.flatten();
            (n + 1, x + vi.is_some() as i64, u + (t == Some("UpMod")) as i64, d + (t == Some("DownMod")) as i64, r + rep, la.max(a))
        })
        .drive(|p, a| v.push((cd(db, p), p, a)));
    v.sort_by_key(|x| x.0);
    rows(v.iter().take(100).map(|&(_, p, (n, x, u, d, r, la))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend([V::I(x), V::I(u), V::I(d), avg(r, n), V::T(la)]);
        row(f)
    }))
}

// WITH Benchmark AS (
// SELECT
// PH.PostHistoryTypeId,
// COUNT(PH.Id) AS ActionCount,
// MIN(PH.CreationDate) AS FirstActionDate,
// MAX(PH.CreationDate) AS LastActionDate,
// EXTRACT(EPOCH FROM (MAX(PH.CreationDate) - MIN(PH.CreationDate))) AS DurationSeconds
// FROM
// PostHistory PH
// GROUP BY
// PH.PostHistoryTypeId
// )
// SELECT
// PHT.Name AS ActionName,
// B.ActionCount,
// B.FirstActionDate,
// B.LastActionDate,
// B.DurationSeconds
// FROM
// Benchmark B
// JOIN
// PostHistoryTypes PHT ON B.PostHistoryTypeId = PHT.Id
// ORDER BY
// B.ActionCount DESC;
fn q14415(db: &'static So) -> String {
    let PostHistory { post_history_type, creation_date, .. } = &db.post_history;
    let mut v = Vec::new();
    db.post_history
        .group_by(post_history_type)
        .select(creation_date)
        .fold((0i64, i64::MAX, i64::MIN), |(n, mn, mx), d| (n + 1, mn.min(d), mx.max(d)))
        .drive(|t, a| v.push((db.post_history_type.name.get(t).unwrap(), a)));
    v.sort_by_key(|x| Reverse(x.1.0));
    rows(v.iter().map(|&(k, (n, mn, mx))| row(vec![V::S(k), V::I(n), V::T(mn), V::T(mx), V::F((mx - mn) as f64 / 1e6)])))
}

// Posts per type with a fold over whatever each post is joined to.
fn by_type<Q, R, S, F>(db: &'static So, base: Q, joined: R, init: S, f: F) -> Vec<(Str, S)>
where
    Q: Drive<D = Id<Post>, R = Id<Post>>,
    R: IntoQuery,
    R::Q: Probe<D = Id<Post>>,
    S: Copy,
    F: Fn(S, ROf<R>) -> S,
{
    let mut v = Vec::new();
    base.group_by((&db.post.post_type).select(&db.post_type.name)).select(joined).fold(init, f).drive(|k, a| v.push((k, a)));
    v
}

fn most<S: Copy>(mut v: Vec<(Str, S)>, n: impl Fn(&S) -> i64) -> Vec<(Str, S)> {
    v.sort_by_key(|x| Reverse(n(&x.1)));
    v
}

// A float AVG whose printed digits move with DuckDB's SET threads, so
// rewrites/10694.sql takes the exact-integer mean, as 5603's does.
//
// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.Score IS NOT NULL THEN p.Score ELSE 0 END) AS TotalScore,
// SUM(CASE WHEN p.ViewCount IS NOT NULL THEN p.ViewCount ELSE 0 END) AS TotalViews,
// AVG(CASE WHEN p.CreationDate IS NOT NULL THEN EXTRACT(EPOCH FROM (cast('2024-10-01 12:34:56' as timestamp) - p.CreationDate)) ELSE NULL END) AS AvgPostAgeInSeconds
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q10694(db: &'static So) -> String {
    let Post { score, view_count, creation_date, .. } = &db.post;
    let now = ts(2024, 10, 1, 12, 34, 56);
    let v = by_type(db, db.post.iq(), score.and(view_count.opt()).and(creation_date), (0i64, 0i64, 0i64, 0i128), |(n, s, w, a), ((x, vw), c)| {
        (n + 1, s + x, w + vw.unwrap_or(0), a + (now - c) as i128)
    });
    rows(most(v, |a| a.0).iter().map(|&(k, (n, s, w, a))| row(vec![V::S(k), V::I(n), V::I(s), V::I(w), V::F(a as f64 / n as f64 / 1e6)])))
}

// SELECT
// pt.Name AS PostTypeName,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.ViewCount) AS AverageViews,
// AVG(p.Score) AS AverageScore,
// COUNT(DISTINCT c.Id) AS TotalComments,
// COUNT(DISTINCT a.Id) AS TotalAnswers,
// COUNT(DISTINCT v.Id) AS TotalVotes
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Posts a ON p.Id = a.ParentId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q11059(db: &'static So) -> String {
    let Post { score, view_count, post_type, .. } = &db.post;
    let name = post_type.select(&db.post_type.name);
    let main = db.post.group_by(&name).select(score.and(view_count.opt()).and(comments_of(db).opt()).and(children_of(db).opt()).and(votes_of(db).opt())).fold(
        [0i64; 4],
        |a, ((((s, w), _), _), _)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)],
    );
    let c = db.post.group_by(&name).select(comments_of(db)).count_distinct();
    let ans = db.post.group_by(&name).select(children_of(db)).count_distinct();
    let x = db.post.group_by(&name).select(votes_of(db)).count_distinct();
    let mut v = Vec::new();
    main.and((&c).opt()).and((&ans).opt()).and((&x).opt()).drive(|k, (((a, c), n), x)| v.push((k, (a, [c, n, x].map(|y| y.unwrap_or(0))))));
    rows(most(v, |a| a.0[0]).iter().map(|&(k, (a, d))| row(vec![V::S(k), V::I(a[0]), avg(a[3], a[2]), avg(a[1], a[0]), V::I(d[0]), V::I(d[1]), V::I(d[2])])))
}

// WITH PostScoreViews AS (
// SELECT
// pt.Name AS PostType,
// AVG(p.Score) AS AverageScore,
// AVG(u.Views) AS AverageViews
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
// )
// SELECT
// PostType,
// AverageScore,
// AverageViews
// FROM
// PostScoreViews
// ORDER BY
// AverageScore DESC;
fn q12984(db: &'static So) -> String {
    let Post { score, owner_user, .. } = &db.post;
    let mut v = by_type(db, owned_since(db, YEAR_AGO()), score.and(owner_user.select(&db.user.views)), [0i64; 3], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w]);
    v.sort_by(|a, b| (b.1[1] as f64 / b.1[0] as f64).total_cmp(&(a.1[1] as f64 / a.1[0] as f64)));
    rows(v.iter().map(|&(k, a)| row(vec![V::S(k), avg(a[1], a[0]), avg(a[2], a[0])])))
}

// WITH PostPerformance AS (
// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AverageScore,
// AVG(p.ViewCount) AS AverageViewCount
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY
// pt.Name
// )
// SELECT
// PostType,
// TotalPosts,
// AverageScore,
// AverageViewCount
// FROM
// PostPerformance
// ORDER BY
// TotalPosts DESC;
fn q10514(db: &'static So) -> String {
    let Post { score, view_count, creation_date, .. } = &db.post;
    let base = db.post.with(creation_date.ge(YEAR_AGO()));
    let v = by_type(db, base, score.and(view_count.opt()), [0i64; 4], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]);
    rows(most(v, |a| a[0]).iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), avg(a[3], a[2])])))
}

// WITH PostStats AS (
// SELECT
// PT.Name AS PostType,
// COUNT(P.Id) AS PostCount,
// AVG(P.Score) AS AverageScore,
// AVG(U.Reputation) AS AverageUserReputation
// FROM
// Posts P
// JOIN
// PostTypes PT ON P.PostTypeId = PT.Id
// JOIN
// Users U ON P.OwnerUserId = U.Id
// WHERE
// P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY
// PT.Name
// )
// SELECT
// PostType,
// PostCount,
// AverageScore,
// AverageUserReputation
// FROM
// PostStats
// ORDER BY
// PostCount DESC;
fn q14787(db: &'static So) -> String {
    let Post { score, owner_user, .. } = &db.post;
    let v = by_type(db, owned_since(db, YEAR_AGO()), score.and(owner_user.select(&db.user.reputation)), [0i64; 3], |a, (s, r)| [a[0] + 1, a[1] + s, a[2] + r]);
    rows(most(v, |a| a[0]).iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), avg(a[2], a[0])])))
}

// WITH PostMetrics AS (
// SELECT
// pt.Id AS PostTypeId,
// COUNT(p.Id) AS PostCount,
// AVG(p.Score) AS AverageScore,
// MAX(p.ViewCount) AS MaxViewCount
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// WHERE
// p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'
// GROUP BY
// pt.Id
// )
// SELECT
// pt.Name AS PostTypeName,
// pm.PostCount,
// pm.AverageScore,
// pm.MaxViewCount
// FROM
// PostMetrics pm
// JOIN
// PostTypes pt ON pm.PostTypeId = pt.Id
// ORDER BY
// pm.PostCount DESC;
fn q10641(db: &'static So) -> String {
    let Post { score, view_count, creation_date, post_type, .. } = &db.post;
    let mut v = Vec::new();
    db.post
        .with(creation_date.ge(date(2023, 10, 1)))
        .group_by(post_type)
        .select(score.and(view_count.opt()))
        .fold([0, 0, 0, i64::MIN], |a: [i64; 4], (s, w)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, w.map_or(a[3], |x| a[3].max(x))])
        .drive(|t, a| v.push((db.post_type.name.get(t).unwrap(), a)));
    rows(most(v, |a| a[0]).iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), omax(a[3], a[2])])))
}

// SELECT
// PT.Name AS PostType,
// COUNT(P.Id) AS TotalPosts,
// SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END) AS PositiveScoredPosts,
// SUM(P.ViewCount) AS TotalViews,
// AVG(P.Score) AS AverageScore,
// AVG(COALESCE(P.AnswerCount, 0)) AS AverageAnswerCount,
// AVG(COALESCE(P.CommentCount, 0)) AS AverageCommentCount,
// COUNT(DISTINCT P.OwnerUserId) AS UniquePostOwners,
// AVG(UP.Reputation) AS AverageUserReputation
// FROM
// Posts P
// JOIN
// PostTypes PT ON P.PostTypeId = PT.Id
// JOIN
// Users UP ON P.OwnerUserId = UP.Id
// GROUP BY
// PT.Name
// ORDER BY
// TotalPosts DESC;
fn q10261(db: &'static So) -> String {
    let Post { score, view_count, answer_count, comment_count, owner_user, .. } = &db.post;
    let name = (&db.post.post_type).select(&db.post_type.name);
    let main = owned(db).group_by(&name).select(score.and(view_count.opt()).and(answer_count.opt()).and(comment_count).and(owner_user.select(&db.user.reputation))).fold(
        [0i64; 8],
        |a, ((((s, w), an), c), r)| [a[0] + 1, a[1] + (s > 0) as i64, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + s, a[5] + an.unwrap_or(0), a[6] + c, a[7] + r],
    );
    let users = owned(db).group_by(&name).select(owner_user).count_distinct();
    let mut v = Vec::new();
    main.and(&users).drive(|k, (a, u)| v.push((k, (a, u))));
    rows(most(v, |a| a.0[0]).iter().map(|&(k, (a, u))| {
        row(vec![V::S(k), V::I(a[0]), V::I(a[1]), nullable(a[3], a[2]), avg(a[4], a[0]), avg(a[5], a[0]), avg(a[6], a[0]), V::I(u), avg(a[7], a[0])])
    }))
}


// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS PostCount,
// AVG(p.Score) AS AverageScore,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN p.PostTypeId = 10 THEN 1 ELSE 0 END) AS ClosedPostCount,
// SUM(CASE WHEN p.PostTypeId = 12 THEN 1 ELSE 0 END) AS DeletedPostCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName
// ORDER BY
// PostCount DESC;
fn q10306(db: &'static So) -> String {
    let v = user_groups(db, Ident::<User>::new(), UserWhere::All);
    rows(v.iter().map(|(u, a)| {
        let mut f = vec![V::I(db.user.origid.get(*u).unwrap()), V::S(db.user.display_name.get(*u).unwrap())];
        f.extend(["#n", "score_avg", "#q", "#a", "#10", "#12"].iter().map(|c| name_field(a, c)));
        row(f)
    }))
}

// SELECT
// U.DisplayName AS UserDisplayName,
// U.Reputation AS UserReputation,
// COUNT(P.Id) AS TotalPosts,
// AVG(P.Score) AS AverageScore,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN P.PostTypeId = 3 THEN 1 ELSE 0 END) AS TotalWikis,
// SUM(CASE WHEN P.PostTypeId IN (1, 2) THEN P.ViewCount ELSE 0 END) AS TotalViews
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// WHERE
// U.Reputation > 0
// GROUP BY
// U.Id, U.DisplayName, U.Reputation
// ORDER BY
// TotalPosts DESC
// LIMIT 100;
fn q11998(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(0))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(view_count.opt())).opt())
        .fold([0i64; 7], |a, p| match p {
            Some(((t, s), w)) => {
                let tv = if t == 1 || t == 2 { w } else { Some(0) };
                [a[0] + 1, a[1] + s, a[2] + (t == 1) as i64, a[3] + (t == 2) as i64, a[4] + (t == 3) as i64, a[5] + tv.is_some() as i64, a[6] + tv.unwrap_or(0)]
            }
            None => a,
        })
        .drive(|u, a| v.push((u, a)));
    v.sort_by_key(|x| Reverse(x.1[0]));
    rows(v.iter().take(100).map(|&(u, a)| {
        row(vec![V::S(db.user.display_name.get(u).unwrap()), V::I(db.user.reputation.get(u).unwrap()), V::I(a[0]), avg(a[1], a[0]), V::I(a[2]), V::I(a[3]), V::I(a[4]), nullable(a[6], a[5])])
    }))
}

// SELECT
// u.DisplayName,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// u.DisplayName
// ORDER BY
// PostCount DESC
// FETCH FIRST 10 ROWS ONLY;
fn q19344(db: &'static So) -> String {
    let mut v = user_vote_groups(db, &db.user.display_name, UserWhere::All, &[]);
    v.sort_by_key(|x| Reverse(x.1[0]));
    rows(v.iter().take(10).map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4])])))
}

// SELECT
// u.DisplayName AS UserName,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// AVG(p.Score) AS AverageScore,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// u.DisplayName
// ORDER BY
// TotalPosts DESC
// LIMIT 100;
fn q14908(db: &'static So) -> String {
    let mut v = user_vote_groups(db, &db.user.display_name, UserWhere::All, &[]);
    v.sort_by_key(|x| Reverse(x.1[0]));
    rows(v.iter().take(100).map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[9], a[0]), V::I(a[3]), V::I(a[4])])))
}

// SELECT
// u.DisplayName AS UserName,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(v.BountyAmount) AS TotalBountyAmount,
// AVG(u.Reputation) AS AvgUserReputation,
// MAX(p.CreationDate) AS LatestPostDate
// FROM
// Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 8
// WHERE
// u.Reputation > 1000
// GROUP BY
// u.DisplayName
// ORDER BY
// TotalPosts DESC
// LIMIT 100;
fn q14274(db: &'static So) -> String {
    let mut v = user_vote_groups(db, &db.user.display_name, UserWhere::RepGt(1000), &[8]);
    v.sort_by_key(|x| Reverse(x.1[0]));
    rows(v.iter().take(100).map(|&(k, a)| {
        row(vec![V::S(k), V::I(a[0]), V::I(a[1]), V::I(a[2]), nullable(a[8], a[7]), avg(a[11], a[12]), if a[0] == 0 { V::Null } else { V::T(a[10]) }])
    }))
}

// SELECT
// u.DisplayName AS UserName,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(c.Score) AS TotalCommentScore,
// SUM(v.BountyAmount) AS TotalBountyAmount,
// MAX(p.CreationDate) AS LastPostDate
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
// PostCount DESC
// LIMIT 100;
fn q10662(db: &'static So) -> String {
    let Post { post_type_id, creation_date, .. } = &db.post;
    let mut v = Vec::new();
    db.user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(creation_date).and(comments_of(db).select(&db.comment.score).opt()).and(votes_of(db).select((&db.vote.bounty_amount).opt()).opt())).opt())
        .fold([0, 0, 0, 0, 0, 0, 0, i64::MIN], |a: [i64; 8], p| match p {
            Some((((t, c), cs), b)) => {
                let b = b.flatten();
                [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + cs.is_some() as i64, a[4] + cs.unwrap_or(0), a[5] + b.is_some() as i64, a[6] + b.unwrap_or(0), a[7].max(c)]
            }
            None => a,
        })
        .drive(|u, a| v.push((u, a)));
    v.sort_by_key(|x| Reverse(x.1[0]));
    rows(v.iter().take(100).map(|&(u, a)| {
        row(vec![V::S(db.user.display_name.get(u).unwrap()), V::I(a[0]), V::I(a[1]), V::I(a[2]), nullable(a[4], a[3]), nullable(a[6], a[5]), if a[0] == 0 { V::Null } else { V::T(a[7]) }])
    }))
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(p.Id) AS TotalPosts,
// SUM(p.Score) AS TotalScore,
// AVG(p.Score) AS AverageScore,
// MAX(p.ViewCount) AS MaxViewCount,
// AVG(p.ViewCount) AS AverageViewCount,
// COUNT(c.Id) AS TotalComments,
// COUNT(DISTINCT v.Id) AS TotalVotes
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// u.Reputation > 0
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
// ORDER BY
// TotalScore DESC;
fn q12248(db: &'static So) -> String {
    let Post { score, view_count, .. } = &db.post;
    let main = user_base(db, UserWhere::RepGt(0))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score.and(view_count.opt()).and(comments_of(db).opt()).and(votes_of(db).opt())).opt())
        .fold([0, 0, 0, i64::MIN, 0, 0], |a: [i64; 6], p| match p {
            Some((((s, w), c), _)) => [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, w.map_or(a[3], |x| a[3].max(x)), a[4] + w.unwrap_or(0), a[5] + c.is_some() as i64],
            None => a,
        });
    let votes = user_base(db, UserWhere::RepGt(0)).group_by(Ident::<User>::new()).select(posts_of(db).select(votes_of(db))).count_distinct();
    let mut v = Vec::new();
    main.and((&votes).opt()).drive(|u, (a, x)| v.push((u, a, x.unwrap_or(0))));
    rows(v.iter().map(|&(u, a, x)| {
        row(vec![
            V::I(db.user.origid.get(u).unwrap()),
            V::S(db.user.display_name.get(u).unwrap()),
            V::I(db.user.reputation.get(u).unwrap()),
            V::I(a[0]),
            nullable(a[1], a[0]),
            avg(a[1], a[0]),
            omax(a[3], a[2]),
            avg(a[4], a[2]),
            V::I(a[5]),
            V::I(x),
        ])
    }))
}

// Users LEFT JOIN Posts LEFT JOIN Comments LEFT JOIN Votes, grouped by user,
// with COUNT(DISTINCT ...) of the posts, comments and votes.
fn user_distincts<Q>(db: &'static So, users: Q) -> (Fold<Id<User>, i64>, Fold<Id<User>, i64>, Fold<Id<User>, i64>)
where
    Q: IntoQuery + Copy,
    Q::Q: Drive<D = Id<User>, R = Id<User>>,
{
    let g = || users.group_by(Ident::<User>::new());
    (g().select(posts_of(db)).count_distinct(), g().select(posts_of(db).select(comments_of(db))).count_distinct(), g().select(posts_of(db).select(votes_of(db))).count_distinct())
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// COUNT(DISTINCT c.Id) AS TotalComments,
// COUNT(DISTINCT v.Id) AS TotalVotes,
// SUM(u.Reputation) AS TotalReputation,
// AVG(u.Views) AS AverageViews,
// MAX(p.CreationDate) AS LatestPostDate
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
// TotalPosts DESC, TotalReputation DESC
// LIMIT 100;
fn q13473(db: &'static So) -> String {
    let rows_ = db
        .user
        .group_by(Ident::<User>::new())
        .select((&db.user.reputation).and(posts_of(db).select((&db.post.creation_date).and(comments_of(db).opt()).and(votes_of(db).opt())).opt()))
        .fold((0i64, 0i64, i64::MIN, 0i64), |(n, r, mx, np), (rep, p)| match p {
            Some(((c, _), _)) => (n + 1, r + rep, mx.max(c), np + 1),
            None => (n + 1, r + rep, mx, np),
        });
    let (p, c, x) = user_distincts(db, db.user.iq());
    let mut v = Vec::new();
    rows_.and((&p).opt()).and((&c).opt()).and((&x).opt()).drive(|u, (((a, p), c), x)| v.push((u, a, [p, c, x].map(|y| y.unwrap_or(0)))));
    v.sort_by_key(|&(_, (_, r, _, _), d)| (Reverse(d[0]), Reverse(r)));
    rows(v.iter().take(100).map(|&(u, (_, r, mx, np), d)| {
        row(vec![
            V::I(db.user.origid.get(u).unwrap()),
            V::S(db.user.display_name.get(u).unwrap()),
            V::I(d[0]),
            V::I(d[1]),
            V::I(d[2]),
            V::I(r),
            V::F(db.user.views.get(u).unwrap() as f64),
            if np == 0 { V::Null } else { V::T(mx) },
        ])
    }))
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(DISTINCT p.Id) AS PostCount,
// COUNT(DISTINCT c.Id) AS CommentCount,
// COUNT(DISTINCT b.Id) AS BadgeCount,
// SUM(u.UpVotes) AS TotalUpVotes,
// SUM(u.DownVotes) AS TotalDownVotes
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// WHERE
// u.Reputation > 1000
// AND u.CreationDate < '2021-01-01'
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
// ORDER BY
// TotalUpVotes DESC
// LIMIT 100;
fn q14749(db: &'static So) -> String {
    let User { reputation, creation_date, up_votes, down_votes, .. } = &db.user;
    let base = db.user.with(reputation.gt(1000).and(creation_date.lt(date(2021, 1, 1))));
    let sums = (&base)
        .group_by(Ident::<User>::new())
        .select(up_votes.and(down_votes).and(posts_of(db).select(comments_of(db).opt()).opt()).and(badges_of(db).opt()))
        .fold((0i64, 0i64), |(u, d), (((up, dn), _), _)| (u + up, d + dn));
    let g = || (&base).group_by(Ident::<User>::new());
    let p = g().select(posts_of(db)).count_distinct();
    let c = g().select(posts_of(db).select(comments_of(db))).count_distinct();
    let b = g().select(badges_of(db)).count_distinct();
    let mut v = Vec::new();
    sums.and((&p).opt()).and((&c).opt()).and((&b).opt()).drive(|u, (((s, p), c), b)| v.push((u, s, [p, c, b].map(|y| y.unwrap_or(0)))));
    v.sort_by_key(|x| Reverse(x.1.0));
    rows(v.iter().take(100).map(|&(u, (up, dn), d)| {
        row(vec![
            V::I(db.user.origid.get(u).unwrap()),
            V::S(db.user.display_name.get(u).unwrap()),
            V::I(db.user.reputation.get(u).unwrap()),
            V::I(d[0]),
            V::I(d[1]),
            V::I(d[2]),
            V::I(up),
            V::I(dn),
        ])
    }))
}

// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(P.ViewCount) AS TotalViews,
// SUM(P.Score) AS TotalScore,
// AVG(P.Score) AS AverageScore,
// COUNT(C.Id) AS TotalComments,
// SUM(B.Class) AS TotalBadges
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// GROUP BY
// U.Id, U.DisplayName
// ORDER BY
// TotalPosts DESC;
fn q13538(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let mut v = Vec::new();
    db.user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(view_count.opt()).and(comments_of(db).opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 10], |a, (p, b)| {
            let mut a = a;
            if let Some((((t, s), w), c)) = p {
                a[0] += 1;
                a[1] += (t == 1) as i64;
                a[2] += (t == 2) as i64;
                a[3] += w.is_some() as i64;
                a[4] += w.unwrap_or(0);
                a[5] += s;
                a[6] += c.is_some() as i64;
            }
            if let Some(k) = b {
                a[7] += 1;
                a[8] += k;
            }
            a
        })
        .drive(|u, a| v.push((u, a)));
    rows(v.iter().map(|&(u, a)| {
        row(vec![
            V::I(db.user.origid.get(u).unwrap()),
            V::S(db.user.display_name.get(u).unwrap()),
            V::I(a[0]),
            V::I(a[1]),
            V::I(a[2]),
            nullable(a[4], a[3]),
            nullable(a[5], a[0]),
            avg(a[5], a[0]),
            V::I(a[6]),
            nullable(a[8], a[7]),
        ])
    }))
}


pub static ENTRIES: &[harness::Entry] = &[
    ("10551", q10551),
    ("13943", q13943),
    ("19984", q19984),
    ("14830", q14830),
    ("11928", q11928),
    ("12816", q12816),
    ("12255", q12255),
    ("14050", q14050),
    ("15336", q15336),
    ("15590", q15590),
    ("10531", q10531),
    ("12547", q12547),
    ("14124", q14124),
    ("19783", q19783),
    ("12197", q12197),
    ("14584", q14584),
    ("10850", q10850),
    ("10948", q10948),
    ("11196", q11196),
    ("12304", q12304),
    ("13592", q13592),
    ("12112", q12112),
    ("13271", q13271),
    ("13792", q13792),
    ("11409", q11409),
    ("14659", q14659),
    ("13848", q13848),
    ("12531", q12531),
    ("11622", q11622),
    ("11144", q11144),
    ("18626", q18626),
    ("13328", q13328),
    ("14415", q14415),
    ("10694", q10694),
    ("11059", q11059),
    ("12984", q12984),
    ("10514", q10514),
    ("14787", q14787),
    ("10641", q10641),
    ("10261", q10261),
    ("10306", q10306),
    ("11998", q11998),
    ("19344", q19344),
    ("14908", q14908),
    ("14274", q14274),
    ("10662", q10662),
    ("12248", q12248),
    ("13473", q13473),
    ("14749", q14749),
    ("13538", q13538),
];
