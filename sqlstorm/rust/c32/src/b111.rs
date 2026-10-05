use harness::prelude::*;
use std::cmp::Reverse;

fn since(db: &'static So, d: i64) -> impl Drive<D = Id<Post>, R = Id<Post>> {
    db.post.with((&db.post.creation_date).ge(d))
}

fn owned_since(db: &'static So, d: i64) -> impl Drive<D = Id<Post>, R = Id<Post>> {
    owned(db).with((&db.post.creation_date).ge(d))
}

fn year_ago() -> i64 {
    ts(2023, 10, 1, 12, 34, 56)
}

fn month_ago() -> i64 {
    ts(2024, 9, 1, 12, 34, 56)
}

fn ud<R>(db: &'static So, w: UserWhere, r: R) -> Fold<Id<User>, i64>
where
    R: IntoQuery,
    R::Q: Probe<D = Id<User>>,
    ROf<R>: Ord,
{
    user_distinct(db, Ident::<User>::new(), w, r)
}

fn only_type(db: &'static So, t: i64) -> Compose<&'static HashIdx<Id<User>, Id<Post>>, Restrict<Ident<Post>, Filter<&'static Col<Post, i64>, impl Fn(i64) -> bool>>> {
    posts_of(db).select(Ident::<Post>::new().with((&db.post.post_type_id).eq(t)))
}

fn views_desc(db: &'static So, p: Id<Post>) -> (bool, Reverse<Option<i64>>) {
    let w = db.post.view_count.get(p);
    (w.is_none(), Reverse(w))
}

fn score_views(db: &'static So, p: Id<Post>) -> (Reverse<i64>, (bool, Reverse<Option<i64>>)) {
    (Reverse(db.post.score.get(p).unwrap()), views_desc(db, p))
}

fn views_score(db: &'static So, p: Id<Post>) -> ((bool, Reverse<Option<i64>>), Reverse<i64>) {
    (views_desc(db, p), Reverse(db.post.score.get(p).unwrap()))
}

fn round2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

// --- posts ------------------------------------------------------------------

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// COALESCE(COUNT(DISTINCT c.Id), 0) AS CommentCount,
// COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
// U.DisplayName AS OwnerDisplayName,
// U.Reputation AS OwnerReputation
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Users U ON p.OwnerUserId = U.Id
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, U.DisplayName, U.Reputation
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q11608(db: &'static So) -> String {
    let c = per_post_distinct(db, comments_of(db));
    let v = stats_with(db, since(db, date(2023, 1, 1)), "cv", &[], &[&c]);
    let mut v: Vec<_> = v.into_iter().map(|(p, s, d)| (newest(db, p), p, s, d)).collect();
    v.sort_by_key(|x| x.0);
    rows(v.iter().take(100).map(|(_, p, s, d)| {
        let mut f = stat_fields(db, *p, s, &["id", "title", "created", "views", "score"]);
        f.extend([V::I(d[0]), V::I(s.bounty_sum), V::I(s.up), V::I(s.down)]);
        f.extend(post_fields(db, *p, &["owner", "rep"]));
        row(f)
    }))
}

// WITH PostStats AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.Score,
// P.ViewCount,
// COUNT(DISTINCT C.Id) AS CommentCount,
// COUNT(DISTINCT V.Id) AS VoteCount,
// AVG(U.Reputation) AS AverageUserReputation
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// LEFT JOIN
// Users U ON P.OwnerUserId = U.Id
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.Score, P.ViewCount
// )
// SELECT
// PS.PostId,
// PS.Title,
// PS.CreationDate,
// PS.Score,
// PS.ViewCount,
// PS.CommentCount,
// PS.VoteCount,
// PS.AverageUserReputation
// FROM
// PostStats PS
// ORDER BY
// PS.ViewCount DESC, PS.Score DESC;
fn q13278(db: &'static So) -> String {
    let c = per_post_distinct(db, comments_of(db));
    let x = per_post_distinct(db, votes_of(db));
    stats_rows(db, stats_with(db, db.post.iq(), "cv", &[], &[&c, &x]), |_, _| 0, 0, &["id", "title", "created", "score", "views", "#d0", "#d1", "rep_avg"])
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
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// COUNT(DISTINCT ph.Id) AS EditHistoryCount
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
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName
// ORDER BY
// p.CreationDate DESC;
fn q14358(db: &'static So) -> String {
    let h = per_post_distinct(db, history_of(db));
    stats_rows(db, stats_with(db, owned_since(db, year_ago()), "cvh", &[], &[&h]), |_, _| 0, 0, &["id", "title", "created", "views", "score", "owner", "#cx", "#vx", "#up", "#down", "#d0"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate AS PostCreationDate,
// p.Score,
// p.ViewCount,
// p.Tags,
// u.Id AS UserId,
// u.DisplayName AS UserDisplayName,
// u.Reputation AS UserReputation,
// c.CommentCount,
// v.VoteCount,
// ph.EditHistoryCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount
// FROM Comments
// GROUP BY PostId) c ON p.Id = c.PostId
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS VoteCount
// FROM Votes
// GROUP BY PostId) v ON p.Id = v.PostId
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS EditHistoryCount
// FROM PostHistory
// GROUP BY PostId) ph ON p.Id = ph.PostId
// WHERE
// p.CreationDate >= '2023-01-01'
// ORDER BY
// p.CreationDate DESC;
fn q10961(db: &'static So) -> String {
    let c = db.comment.group_by(&db.comment.post_id).fold(0i64, |a, _| a + 1);
    let x = db.vote.group_by(&db.vote.post_id).fold(0i64, |a, _| a + 1);
    let h = db.post_history.group_by(&db.post_history.post_id).fold(0i64, |a, _| a + 1);
    let mut v = Vec::new();
    owned_since(db, date(2023, 1, 1)).select((&db.post.origid).select((&c).opt().and((&x).opt()).and((&h).opt()))).drive(|p, a| v.push((p, a)));
    rows(v.iter().map(|&(p, ((c, x), h))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "tags", "uid", "owner", "rep"]);
        f.extend([oint(c), oint(x), oint(h)]);
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.ViewCount,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) - SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS VoteScore,
// COUNT(c.Id) AS CommentCount,
// u.DisplayName AS OwnerDisplayName,
// u.Reputation AS OwnerReputation,
// p.CreationDate,
// p.LastActivityDate,
// p.PostTypeId
// FROM
// Posts p
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.CreationDate >= '2024-10-01 12:34:56'::timestamp - INTERVAL '1 YEAR'
// GROUP BY
// p.Id, p.Title, p.ViewCount, u.DisplayName, u.Reputation, p.CreationDate, p.LastActivityDate, p.PostTypeId
// ORDER BY
// VoteScore DESC, p.LastActivityDate DESC;
fn q11832(db: &'static So) -> String {
    let v = stats_with(db, since(db, year_ago()), "vc", &[], &[]);
    let mut v: Vec<_> = v.into_iter().map(|(p, s, _)| ((Reverse(s.up - s.down), Reverse(db.post.last_activity_date.get(p).unwrap())), p, s)).collect();
    v.sort_by_key(|x| x.0);
    rows(v.iter().map(|(_, p, s)| {
        let mut f = stat_fields(db, *p, s, &["id", "title", "views"]);
        f.extend([V::I(s.up - s.down), V::I(s.cx)]);
        f.extend(post_fields(db, *p, &["owner", "rep", "created", "activity", "type_id"]));
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// u.DisplayName AS OwnerDisplayName,
// COUNT(c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
// p.ViewCount,
// p.Score,
// pt.Name AS PostTypeName
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// WHERE
// p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'
// GROUP BY
// p.Id, p.Title, p.CreationDate, u.DisplayName, p.ViewCount, p.Score, pt.Name
// ORDER BY
// p.CreationDate DESC;
fn q13531(db: &'static So) -> String {
    let x = per_post_distinct(db, votes_of(db));
    stats_rows(db, stats_with(db, owned_since(db, year_ago()), "cv", &[], &[&x]), |_, _| 0, 0, &["id", "title", "created", "owner", "#cx", "#d0", "#up", "#down", "views", "score", "type"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// COUNT(c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
// u.DisplayName AS OwnerDisplayName,
// pt.Name AS PostTypeName
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
// WHERE
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName, pt.Name
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q13500(db: &'static So) -> String {
    let x = per_post_distinct(db, votes_of(db));
    stats_rows(db, stats_with(db, owned_since(db, year_ago()), "cv", &[], &[&x]), |p, _| newest(db, p), 100, &["id", "title", "created", "views", "score", "#cx", "#d0", "#up", "#down", "owner", "type"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// u.DisplayName AS OwnerDisplayName,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
// AVG(COALESCE(p.Score, 0)) AS AverageScore,
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
// p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY
// p.Id, p.Title, p.CreationDate, u.DisplayName
// ORDER BY
// AverageScore DESC, VoteCount DESC;
fn q13788(db: &'static So) -> String {
    let base = owned(db).with((&db.post.creation_date).gt(year_ago()));
    let mut v = Vec::new();
    base.group_by(Ident::<Post>::new())
        .select((&db.post.score).and(comments_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(history_of(db).select(&db.post_history.creation_date).opt()))
        .fold([0, 0, 0, 0, 0, 0, i64::MIN, 0], |a: [i64; 8], (((s, c), x), h)| {
            [a[0] + c.is_some() as i64, a[1] + x.is_some() as i64, a[2] + (x == Some(2)) as i64, a[3] + (x == Some(3)) as i64, a[4] + s, a[5] + 1, h.map_or(a[6], |d| a[6].max(d)), a[7] + h.is_some() as i64]
        })
        .drive(|p, a| v.push((p, a)));
    rows(v.iter().map(|&(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), avg(a[4], a[5]), if a[7] == 0 { V::Null } else { V::T(a[6]) }]);
        row(f)
    }))
}

// WITH Benchmark AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// u.DisplayName AS Owner,
// p.CreationDate,
// p.ViewCount,
// p.Score,
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
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, u.DisplayName, p.CreationDate, p.ViewCount, p.Score
// )
// SELECT
// PostId,
// Title,
// Owner,
// CreationDate,
// ViewCount,
// Score,
// CommentCount,
// UpVotes,
// DownVotes,
// (UpVotes - DownVotes) AS NetVotes
// FROM
// Benchmark
// ORDER BY
// ViewCount DESC, Score DESC
// LIMIT 10;
fn q11703(db: &'static So) -> String {
    let base = db.post.with((&db.post.post_type_id).eq(1));
    let v = stats_with(db, base, "cv", &[], &[]);
    let mut v: Vec<_> = v.into_iter().map(|(p, s, _)| (views_score(db, p), p, s)).collect();
    v.sort_by_key(|x| x.0);
    rows(v.iter().take(10).map(|(_, p, s)| {
        let mut f = stat_fields(db, *p, s, &["id", "title", "owner", "created", "views", "score", "#cx", "#up", "#down"]);
        f.push(V::I(s.up - s.down));
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
// u.Reputation AS OwnerReputation,
// COUNT(DISTINCT c.Id) AS CommentCount,
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
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.FavoriteCount,
// u.DisplayName, u.Reputation
// ORDER BY
// p.CreationDate DESC;
fn q10125(db: &'static So) -> String {
    let c = per_post_distinct(db, comments_of(db));
    stats_rows(db, stats_with(db, owned_since(db, month_ago()), "cv", &[], &[&c]), |_, _| 0, 0, &["id", "title", "created", "score", "views", "answers", "comments", "favorites", "owner", "rep", "#d0", "#up", "#down"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.AnswerCount,
// p.CommentCount,
// p.Score,
// u.DisplayName AS OwnerDisplayName,
// u.Reputation AS OwnerReputation,
// COUNT(c.Id) AS TotalComments,
// COUNT(DISTINCT v.Id) AS TotalVotes,
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
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.AnswerCount, p.CommentCount, p.Score,
// u.DisplayName, u.Reputation
// ORDER BY
// p.CreationDate DESC;
fn q10403(db: &'static So) -> String {
    let x = per_post_distinct(db, votes_of(db));
    stats_rows(db, stats_with(db, owned_since(db, month_ago()), "cv", &[], &[&x]), |_, _| 0, 0, &["id", "title", "created", "views", "answers", "comments", "score", "owner", "rep", "#cx", "#d0", "#up", "#down"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.Score,
// p.ViewCount,
// COALESCE(u.DisplayName, 'Community User') AS OwnerDisplayName,
// u.Reputation AS OwnerReputation,
// pt.Name AS PostTypeName,
// COUNT(DISTINCT c.Id) AS CommentCount,
// COUNT(DISTINCT a.Id) AS AnswerCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Posts a ON p.Id = a.ParentId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.Title, p.Score, p.ViewCount, u.DisplayName, u.Reputation, pt.Name
// ORDER BY
// p.Score DESC, p.ViewCount DESC
// LIMIT 100;
fn q10574(db: &'static So) -> String {
    let c = per_post_distinct(db, comments_of(db));
    let a = per_post_distinct(db, children_of(db));
    let v = stats_with(db, db.post.iq(), "cav", &[], &[&c, &a]);
    let mut v: Vec<_> = v.into_iter().map(|(p, s, d)| (score_views(db, p), p, s, d)).collect();
    v.sort_by_key(|x| x.0);
    rows(v.iter().take(100).map(|(_, p, s, d)| {
        let mut f = post_fields(db, *p, &["id", "title", "score", "views"]);
        f.push(V::S(db.post.owner_user.get(*p).map_or("Community User", |u| db.user.display_name.get(u).unwrap())));
        f.extend(post_fields(db, *p, &["rep", "type"]));
        f.extend([V::I(d[0]), V::I(d[1]), V::I(s.up), V::I(s.down)]);
        row(f)
    }))
}

// SELECT
// P.Id AS PostId,
// P.Title,
// U.DisplayName AS OwnerDisplayName,
// P.CreationDate,
// P.Score,
// P.ViewCount,
// COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
// COUNT(CASE WHEN A.Id IS NOT NULL THEN 1 END) AS AnswerCount,
// COUNT(DISTINCT B.Id) AS BadgeCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Posts A ON P.Id = A.ParentId
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// WHERE
// P.PostTypeId = 1
// GROUP BY
// P.Id, P.Title, U.DisplayName, P.CreationDate, P.Score, P.ViewCount
// ORDER BY
// P.CreationDate DESC
// LIMIT 100;
fn q11630(db: &'static So) -> String {
    let b = per_post_distinct(db, (&db.post.owner_user).select(badges_of(db)));
    let base = owned(db).with((&db.post.post_type_id).eq(1));
    stats_rows(db, stats_with(db, base, "cabv", &[], &[&b]), |p, _| newest(db, p), 100, &["id", "title", "owner", "created", "score", "views", "#cx", "#ax", "#d0", "#up", "#down"])
}

// WITH PostAggregated AS (
// SELECT
// p.Id AS PostId,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
// AVG(u.Reputation) AS AvgUserReputation
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// GROUP BY
// p.Id
// )
// SELECT
// pa.PostId,
// pa.CommentCount,
// pa.UpVoteCount,
// pa.DownVoteCount,
// pa.AvgUserReputation,
// pt.Name AS PostType,
// p.Title,
// p.CreationDate
// FROM
// PostAggregated pa
// JOIN
// Posts p ON pa.PostId = p.Id
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// ORDER BY
// pa.UpVoteCount DESC, pa.CommentCount DESC;
fn q11138(db: &'static So) -> String {
    let v = stats_with(db, db.post.iq(), "cv", &[], &[]);
    rows(v.iter().map(|(p, s, _)| {
        let mut f = vec![post_fields(db, *p, &["id"]).pop().unwrap()];
        f.extend(["#cx", "#up", "#down", "rep_avg", "type", "title", "created"].iter().map(|c| stat_fields(db, *p, s, &[c]).pop().unwrap()));
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.PostTypeId,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// p.AnswerCount,
// p.CommentCount,
// COALESCE(u.DisplayName, 'Community User') AS OwnerDisplayName,
// COUNT(c.Id) AS TotalComments,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes,
// MAX(ph.CreationDate) AS LastEditDate,
// COUNT(DISTINCT ph.Id) AS EditCount
// FROM
// Posts p
// LEFT JOIN Users u ON p.OwnerUserId = u.Id
// LEFT JOIN Comments c ON p.Id = c.PostId
// LEFT JOIN Votes v ON p.Id = v.PostId
// LEFT JOIN PostHistory ph ON p.Id = ph.PostId
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Id, p.Title, p.PostTypeId, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, p.CommentCount, u.DisplayName
// ORDER BY
// p.Score DESC, p.ViewCount DESC;
fn q10861(db: &'static So) -> String {
    let h = per_post_distinct(db, history_of(db));
    let v = stats_with(db, since(db, date(2023, 1, 1)), "cvh", &[], &[&h]);
    rows(v.iter().map(|(p, s, d)| {
        let mut f = stat_fields(db, *p, s, &["id", "title", "type_id", "created", "views", "score", "answers", "comments"]);
        f.push(V::S(db.post.owner_user.get(*p).map_or("Community User", |u| db.user.display_name.get(u).unwrap())));
        f.extend([V::I(s.cx), V::I(s.up), V::I(s.down), stat_field(s, "hmax").unwrap(), V::I(d[0])]);
        row(f)
    }))
}

// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// U.DisplayName AS OwnerDisplayName,
// COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
// COUNT(CASE WHEN V.Id IS NOT NULL THEN 1 END) AS VoteCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// P.Score,
// P.ViewCount,
// P.AnswerCount,
// P.FavoriteCount,
// P.LastActivityDate
// FROM
// Posts P
// LEFT JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// WHERE
// P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 YEAR'
// GROUP BY
// P.Id, P.Title, P.CreationDate, U.DisplayName, P.Score, P.ViewCount, P.AnswerCount, P.FavoriteCount, P.LastActivityDate
// ORDER BY
// P.Score DESC, P.ViewCount DESC;
fn q10307(db: &'static So) -> String {
    stats_rows(db, stats_with(db, since(db, year_ago()), "cv", &[], &[]), |_, _| 0, 0, &["id", "title", "created", "owner", "#cx", "#vx", "#up", "#down", "score", "views", "answers", "favorites", "activity"])
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
// p.Tags,
// u.DisplayName AS OwnerDisplayName,
// u.Reputation AS OwnerReputation,
// CASE
// WHEN ph.PostId IS NOT NULL THEN 'Yes'
// ELSE 'No'
// END AS IsClosed,
// COUNT(v.Id) AS VoteCount,
// AVG(v.BountyAmount) AS AvgBounty
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId AND ph.PostHistoryTypeId = 10
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.FavoriteCount, p.Tags,
// u.DisplayName, u.Reputation, ph.PostId
// ORDER BY
// p.CreationDate DESC;
fn q12520(db: &'static So) -> String {
    let closes = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).eq(10)));
    let mut v = Vec::new();
    owned_since(db, year_ago())
        .group_by(Ident::<Post>::new())
        .select((&closes).opt().and(votes_of(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold([0i64; 4], |a, (h, x)| [a[0] + x.is_some() as i64, a[1] + x.flatten().is_some() as i64, a[2] + x.flatten().unwrap_or(0), a[3] + h.is_some() as i64])
        .drive(|p, a| v.push((p, a)));
    rows(v.iter().map(|&(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "favorites", "tags", "owner", "rep"]);
        f.extend([V::S(if a[3] > 0 { "Yes" } else { "No" }), V::I(a[0]), avg(a[2], a[1])]);
        row(f)
    }))
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.PostTypeId,
// p.OwnerUserId,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// AVG(v.BountyAmount) AS AverageBounty,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.PostTypeId, p.OwnerUserId
// )
// SELECT
// PostStats.PostId,
// PostStats.PostTypeId,
// Users.DisplayName AS OwnerDisplayName,
// PostStats.CommentCount,
// PostStats.VoteCount,
// PostStats.UpVotes,
// PostStats.DownVotes,
// PostStats.AverageBounty
// FROM
// PostStats
// JOIN
// Users ON PostStats.OwnerUserId = Users.Id
// ORDER BY
// PostStats.VoteCount DESC;
fn q14563(db: &'static So) -> String {
    let v = stats_with(db, owned(db), "cv", &[], &[]);
    rows(v.iter().map(|(p, s, _)| {
        let mut f = post_fields(db, *p, &["id", "type_id", "owner"]);
        f.extend(["#cx", "#vx", "#up", "#down", "bounty_avg"].iter().map(|c| stat_field(s, c).unwrap()));
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// pt.Name AS PostType,
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(v.Id) AS VoteCount,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
// p.CreationDate,
// p.LastActivityDate,
// p.ViewCount,
// p.AnswerCount,
// p.CommentCount,
// p.FavoriteCount
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'
// GROUP BY
// p.Id, p.Title, pt.Name, u.Id, u.DisplayName, u.Reputation,
// p.CreationDate, p.LastActivityDate, p.ViewCount,
// p.AnswerCount, p.CommentCount, p.FavoriteCount
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q14228(db: &'static So) -> String {
    stats_rows(db, stats_with(db, owned_since(db, year_ago()), "v", &[], &[]), |p, _| newest(db, p), 100, &["id", "title", "type", "uid", "owner", "rep", "#vx", "#up", "#down", "created", "activity", "views", "answers", "comments", "favorites"])
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
// AVG(v.BountyAmount) AS AverageBounty,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpVotes,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownVotes,
// COUNT(DISTINCT c.Id) AS TotalComments,
// COUNT(DISTINCT ph.Id) AS TotalPostHistoryEvents
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
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.FavoriteCount, u.DisplayName
// ORDER BY
// p.CreationDate DESC;
fn q13138(db: &'static So) -> String {
    let c = per_post_distinct(db, comments_of(db));
    let h = per_post_distinct(db, history_of(db));
    stats_rows(db, stats_with(db, since(db, date(2023, 1, 1)), "vch", &[], &[&c, &h]), |_, _| 0, 0, &["id", "title", "created", "score", "views", "answers", "comments", "favorites", "owner", "bounty_avg", "#up", "#down", "#d0", "#d1"])
}

// WITH PostVoteStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score AS PostScore,
// p.ViewCount,
// COUNT(v.Id) AS VoteCount,
// SUM(CASE WHEN vt.Id = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN vt.Id = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
// p.AnswerCount,
// p.CommentCount,
// p.FavoriteCount
// FROM
// Posts p
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// VoteTypes vt ON v.VoteTypeId = vt.Id
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.FavoriteCount
// )
// SELECT
// PostId,
// Title,
// CreationDate,
// PostScore,
// ViewCount,
// VoteCount,
// UpVoteCount,
// DownVoteCount,
// AnswerCount,
// CommentCount,
// FavoriteCount
// FROM
// PostVoteStats
// ORDER BY
// PostScore DESC,
// ViewCount DESC
// LIMIT 100;
fn q10480(db: &'static So) -> String {
    stats_rows(db, stats_with(db, db.post.iq(), "v", &[], &[]), |p, _| score_views(db, p), 100, &["id", "title", "created", "score", "views", "#vx", "#upj", "#downj", "answers", "comments", "favorites"])
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// pt.Name AS PostType,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
// p.Score,
// p.ViewCount,
// p.CreationDate
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// GROUP BY
// p.Id, p.Title, pt.Name, p.Score, p.ViewCount, p.CreationDate
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.PostType,
// ps.CommentCount,
// ps.VoteCount,
// ps.UpVoteCount,
// ps.DownVoteCount,
// ps.Score,
// ps.ViewCount,
// ps.CreationDate
// FROM
// PostStats ps
// ORDER BY
// ps.ViewCount DESC,
// ps.Score DESC
// LIMIT 100;
fn q14161(db: &'static So) -> String {
    stats_rows(db, stats_with(db, db.post.iq(), "cv", &[], &[]), |p, _| views_score(db, p), 100, &["id", "title", "type", "#cx", "#vx", "#up", "#down", "score", "views", "created"])
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// p.AnswerCount,
// p.CommentCount,
// COUNT(c.Id) AS TotalComments,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
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
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, u.DisplayName
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.Score,
// ps.ViewCount,
// ps.AnswerCount,
// ps.CommentCount,
// ps.TotalComments,
// ps.UpVotes,
// ps.DownVotes,
// ps.OwnerDisplayName
// FROM
// PostStats ps
// ORDER BY
// ps.Score DESC, ps.ViewCount DESC
// LIMIT 100;
fn q13362(db: &'static So) -> String {
    stats_rows(db, stats_with(db, db.post.iq(), "cv", &[], &[]), |p, _| score_views(db, p), 100, &["id", "title", "created", "score", "views", "answers", "comments", "#cx", "#up", "#down", "owner"])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// u.DisplayName AS OwnerDisplayName,
// COUNT(v.Id) AS VoteCount,
// COUNT(c.Id) AS CommentCount,
// COALESCE(NULLIF(ROUND(AVG(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END), 2), 0), 0) AS UpVotes,
// COALESCE(NULLIF(ROUND(AVG(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END), 2), 0), 0) AS DownVotes
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// VoteTypes vt ON v.VoteTypeId = vt.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// WHERE
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName
// ORDER BY
// p.CreationDate DESC;
fn q12489(db: &'static So) -> String {
    let v = stats_with(db, owned_since(db, month_ago()), "vc", &[], &[]);
    rows(v.iter().map(|(p, s, _)| {
        let mut f = stat_fields(db, *p, s, &["id", "title", "created", "score", "views", "owner", "#vx", "#cx"]);
        f.extend([V::F(round2(s.upn as f64 / s.rows as f64)), V::F(round2(s.downn as f64 / s.rows as f64))]);
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// COUNT(DISTINCT c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
// COUNT(DISTINCT ph.Id) AS EditHistoryCount,
// COALESCE(SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadgeCount,
// COALESCE(SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadgeCount,
// COALESCE(SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadgeCount,
// COUNT(DISTINCT pl.RelatedPostId) AS RelatedPostCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// LEFT JOIN
// Badges b ON p.OwnerUserId = b.UserId
// LEFT JOIN
// PostLinks pl ON p.Id = pl.PostId
// WHERE
// p.CreationDate >= '2020-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q8272(db: &'static So) -> String {
    let c = per_post_distinct(db, comments_of(db));
    let h = per_post_distinct(db, history_of(db));
    let r = per_post_distinct(db, links_of(db).select(&db.post_link.related_post_id));
    let bidx: HashIdx<i64, Id<Badge>> = (&db.badge.user_id).inv().collect();
    let f = since(db, date(2020, 1, 1))
        .group_by(Ident::<Post>::new())
        .select(
            comments_of(db)
                .opt()
                .and(votes_of(db).select(&db.vote.vote_type_id).opt())
                .and(history_of(db).opt())
                .and((&db.post.owner_user_id).select(&bidx).select(&db.badge.class).opt())
                .and(links_of(db).opt()),
        )
        .fold([0i64; 5], |a, ((((_, t), _), b), _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + (b == Some(1)) as i64, a[3] + (b == Some(2)) as i64, a[4] + (b == Some(3)) as i64]);
    let mut v = Vec::new();
    f.and((&c).opt()).and((&h).opt()).and((&r).opt()).drive(|p, (((a, c), h), r)| v.push((p, a, [c, h, r].map(|x| x.unwrap_or(0)))));
    v.sort_by_key(|&(p, _, _)| newest(db, p));
    rows(v.iter().take(100).map(|&(p, a, d)| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(d[0]), V::I(a[0]), V::I(a[1]), V::I(d[1]), V::I(a[2]), V::I(a[3]), V::I(a[4]), V::I(d[2])]);
        row(f)
    }))
}

// GROUP BY a post and a column of one of its joined history rows.
// SELECT
// U.DisplayName AS UserName,
// P.Title AS PostTitle,
// P.Score AS PostScore,
// PH.CreationDate AS HistoryCreationDate,
// P.LastEditDate AS LastEditDate,
// COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVoteCount,
// COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVoteCount,
// COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
// COUNT(DISTINCT B.Id) AS BadgeCount
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
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// WHERE
// P.CreationDate >= '2023-01-01'
// GROUP BY
// U.DisplayName, P.Title, P.Score, PH.CreationDate, P.LastEditDate
// ORDER BY
// P.Score DESC, UserName ASC;
fn q13051(db: &'static So) -> String {
    let Post { title, score, last_edit_date, owner_user, .. } = &db.post;
    let j: MatSet<(Id<Post>, Option<Id<PostHistory>>)> = owned_since(db, date(2023, 1, 1)).select(Ident::<Post>::new().and(history_of(db).opt())).collect();
    let post_of = (&j).map(|(p, _)| p);
    let key = (&post_of)
        .select(owner_user.select(&db.user.display_name).and(title.opt()).and(score).and(last_edit_date.opt()))
        .and((&j).flat_map(|(_, h)| h).select(&db.post_history.creation_date).opt());
    let f = (&j).group_by(&key).select((&post_of).select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()).and(owner_user.select(badges_of(db)).opt()))).fold([0i64; 3], |a, ((x, c), _)| {
        [a[0] + (x == Some(2)) as i64, a[1] + (x == Some(3)) as i64, a[2] + c.is_some() as i64]
    });
    let b = (&j).group_by(&key).select((&post_of).select(owner_user.select(badges_of(db)))).count_distinct();
    let mut v = Vec::new();
    f.and((&b).opt()).drive(|k, (a, b)| v.push((k, a, b.unwrap_or(0))));
    rows(v.iter().map(|&(((((dn, t), s), le), hd), a, b)| row(vec![V::S(dn), ostr(t), V::I(s), ots(hd), ots(le), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(b)])))
}

// SELECT
// DATE_TRUNC('day', p.CreationDate) AS post_date,
// COUNT(DISTINCT p.Id) AS total_posts,
// COUNT(DISTINCT c.Id) AS total_comments,
// COUNT(DISTINCT u.Id) AS active_users,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS total_questions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS total_answers,
// SUM(CASE WHEN p.PostTypeId = 10 THEN 1 ELSE 0 END) AS total_closed_posts,
// SUM(CASE WHEN p.PostTypeId = 11 THEN 1 ELSE 0 END) AS total_reopened_posts
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id OR c.UserId = u.Id
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'
// GROUP BY
// post_date
// ORDER BY
// post_date ASC;
fn q11938(db: &'static So) -> String {
    let Post { creation_date, owner_user, post_type_id, .. } = &db.post;
    let base = db.post.with(creation_date.ge(month_ago()));
    let rows_: MatSet<(Id<Post>, Option<Id<Comment>>)> = base.select(Ident::<Post>::new().and(comments_of(db).opt())).collect();
    let post_of = (&rows_).map(|(p, _)| p);
    let comment_of = (&rows_).flat_map(|(_, c)| c);
    // LEFT JOIN Users u ON p.OwnerUserId = u.Id OR c.UserId = u.Id: each row
    // meets the owner and the commenter, each user once.
    let met: MatSet<((Id<Post>, Option<Id<Comment>>), Id<User>)> =
        (&rows_).select(Same::new().and((&post_of).select(owner_user))).union((&rows_).select(Same::new().and((&comment_of).select(&db.comment.user)))).collect();
    let users_of: HashIdx<(Id<Post>, Option<Id<Comment>>), Id<User>> = (&met).map(|(r, _)| r).inv().select((&met).map(|(_, u)| u)).collect();
    let day = (&post_of).select(creation_date.map(trunc_day));
    let f = (&rows_).group_by(&day).select((&post_of).select(post_type_id).and((&users_of).opt())).fold([0i64; 4], |a, (t, _)| {
        [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (t == 10) as i64, a[3] + (t == 11) as i64]
    });
    let p = (&rows_).group_by(&day).select(&post_of).count_distinct();
    let c = (&rows_).group_by(&day).select(&comment_of).count_distinct();
    let u = (&rows_).group_by(&day).select(&users_of).count_distinct();
    let mut v = Vec::new();
    f.and(&p).and((&c).opt()).and((&u).opt()).drive(|d, (((a, p), c), u)| v.push((d, a, p, c.unwrap_or(0), u.unwrap_or(0))));
    rows(v.iter().map(|&(d, a, p, c, u)| row(vec![V::T(d), V::I(p), V::I(c), V::I(u), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3])])))
}

// --- users ------------------------------------------------------------------

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// COUNT(DISTINCT c.Id) AS TotalComments,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// COALESCE(SUM(p.ViewCount), 0) AS TotalViews,
// COALESCE(MAX(p.CreationDate), DATE '1970-01-01') AS LatestPostDate
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
// TotalPosts DESC, TotalViews DESC;
fn q10572(db: &'static So) -> String {
    let w = UserWhere::All;
    let (p, c) = (ud(db, w, posts_of(db)), ud(db, w, posts_of(db).select(comments_of(db))));
    let v = users_stats_with(db, w, "cv", any_post, &[], &[&p, &c]);
    rows(v.iter().map(|(u, s, d)| {
        let mut f = vec![user_col(db, *u, "uid"), user_col(db, *u, "name"), V::I(d[0]), V::I(d[1])];
        f.extend(["#up", "#down", "#q", "#a"].iter().map(|c| ustat_field(s, c)));
        f.extend([V::I(s.views_sum), V::T(if s.n == 0 { 0 } else { s.pmax })]);
        row(f)
    }))
}

// WITH UserPostStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// AVG(P.Score) AS AveragePostScore,
// SUM(P.ViewCount) AS TotalViews,
// SUM(P.CommentCount) AS TotalComments
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id, U.DisplayName
// )
// SELECT
// UPS.UserId,
// UPS.DisplayName,
// UPS.TotalPosts,
// UPS.TotalQuestions,
// UPS.TotalAnswers,
// UPS.AveragePostScore,
// UPS.TotalViews,
// UPS.TotalComments
// FROM
// UserPostStats UPS
// ORDER BY
// UPS.TotalPosts DESC
// LIMIT 10;
fn q12833(db: &'static So) -> String {
    let v = users_stats_with(db, UserWhere::All, "", any_post, &[], &[]);
    let mut v = v;
    v.sort_by_key(|x| Reverse(x.1.n));
    rows(v.iter().take(10).map(|(u, s, _)| {
        let mut f = vec![user_col(db, *u, "uid"), user_col(db, *u, "name")];
        f.extend(["#n", "#q", "#a", "score_avg", "views_sum"].iter().map(|c| ustat_field(s, c)));
        f.push(nullable(s.cc_sum, s.n));
        row(f)
    }))
}

// SELECT
// u.Id AS UserID,
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN pt.Name = 'Question' THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN pt.Name = 'Answer' THEN 1 ELSE 0 END) AS TotalAnswers,
// COUNT(c.Id) AS TotalComments,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpVotes,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownVotes
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY
// u.Id, u.DisplayName
// ORDER BY
// TotalPosts DESC;
fn q12706(db: &'static So) -> String {
    let Post { creation_date, post_type, .. } = &db.post;
    let recent = Ident::<Post>::new().with(creation_date.ge(year_ago()));
    let mut v = Vec::new();
    db.user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(recent.select(post_type.select(&db.post_type.name).opt().and(comments_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()))))
        .fold([0i64; 6], |a, ((t, c), x)| {
            [a[0] + 1, a[1] + (t == Some("Question")) as i64, a[2] + (t == Some("Answer")) as i64, a[3] + c.is_some() as i64, a[4] + (x == Some(2)) as i64, a[5] + (x == Some(3)) as i64]
        })
        .drive(|u, a| v.push((u, a)));
    rows(v.iter().map(|&(u, a)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
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
// SUM(CASE WHEN V.CreationDate IS NOT NULL THEN 1 ELSE 0 END) AS TotalVotes,
// SUM(CASE WHEN P.PostTypeId = 1 THEN P.AnswerCount ELSE 0 END) AS TotalAnswersToQuestions,
// SUM(P.ViewCount) AS TotalViewCount,
// SUM(P.FavoriteCount) AS TotalFavorites,
// MAX(P.LastActivityDate) AS LastActivityDate,
// MIN(P.CreationDate) AS FirstPostDate,
// AVG(P.Score) AS AveragePostScore
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// U.Id, U.DisplayName, U.Reputation
// ORDER BY
// U.Reputation DESC, TotalPosts DESC;
fn q10809(db: &'static So) -> String {
    let w = UserWhere::All;
    let (p, c) = (ud(db, w, posts_of(db)), ud(db, w, posts_of(db).select(comments_of(db))));
    let Post { post_type_id, answer_count, view_count, favorite_count, last_activity_date, creation_date, score, .. } = &db.post;
    let f = user_base(db, w)
        .group_by(Ident::<User>::new())
        .select(
            posts_of(db)
                .select(post_type_id.and(answer_count.opt()).and(view_count.opt()).and(favorite_count.opt()).and(last_activity_date).and(creation_date).and(score).and(comments_of(db).opt()).and(votes_of(db).opt()))
                .opt(),
        )
        .fold([0, 0, 0, 0, 0, 0, 0, i64::MIN, i64::MAX, 0, 0], |a: [i64; 11], r| match r {
            Some(((((((((t, an), w), fv), la), c), s), _), x)) => {
                let ans = if t == 1 { an } else { Some(0) };
                [
                    a[0] + x.is_some() as i64,
                    a[1] + ans.is_some() as i64,
                    a[2] + ans.unwrap_or(0),
                    a[3] + w.is_some() as i64,
                    a[4] + w.unwrap_or(0),
                    a[5] + fv.is_some() as i64,
                    a[6] + fv.unwrap_or(0),
                    a[7].max(la),
                    a[8].min(c),
                    a[9] + s,
                    a[10] + 1,
                ]
            }
            // CASE WHEN p.PostTypeId = 1 ... ELSE 0: a user with no posts still adds a 0
            None => {
                let mut a = a;
                a[1] += 1;
                a
            }
        });
    let mut v = Vec::new();
    f.and((&p).opt()).and((&c).opt()).drive(|u, ((a, p), c)| v.push((u, a, p.unwrap_or(0), c.unwrap_or(0))));
    rows(v.iter().map(|&(u, a, p, c)| {
        let t = |x: i64| if a[10] == 0 { V::Null } else { V::T(x) };
        row(vec![
            user_col(db, u, "uid"),
            user_col(db, u, "name"),
            user_col(db, u, "rep"),
            V::I(p),
            V::I(c),
            V::I(a[0]),
            nullable(a[2], a[1]),
            nullable(a[4], a[3]),
            nullable(a[6], a[5]),
            t(a[7]),
            t(a[8]),
            avg(a[9], a[10]),
        ])
    }))
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(DISTINCT p.Id) AS PostsCount,
// COUNT(DISTINCT a.Id) AS AnswersCount,
// SUM(COALESCE(p.ViewCount, 0)) AS TotalViews,
// SUM(COALESCE(p.Score, 0)) AS TotalScore,
// SUM(COALESCE(c.Id, 0)) AS CommentsCount,
// COUNT(DISTINCT b.Id) AS BadgesCount,
// MAX(p.CreationDate) AS LastPostDate
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Posts a ON p.AcceptedAnswerId = a.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
// ORDER BY
// TotalViews DESC, TotalScore DESC
// LIMIT 100;
fn q10645(db: &'static So) -> String {
    let w = UserWhere::All;
    let recent_posts = posts_of(db).select(Ident::<Post>::new().with((&db.post.creation_date).ge(year_ago())));
    let p = ud(db, w, &recent_posts);
    let a = ud(db, w, (&recent_posts).select(&db.post.accepted_answer));
    let b = ud(db, w, badges_of(db));
    let Post { view_count, score, creation_date, accepted_answer, .. } = &db.post;
    let f = user_base(db, w)
        .group_by(Ident::<User>::new())
        .select((&recent_posts).select(view_count.opt().and(score).and(creation_date).and(accepted_answer.opt()).and(comments_of(db).select(&db.comment.origid).opt())).and(badges_of(db).opt()))
        .fold([0, 0, 0, i64::MIN, 0], |a: [i64; 5], (((((w, s), c), _), ci), _)| [a[0] + w.unwrap_or(0), a[1] + s, a[2] + ci.unwrap_or(0), a[3].max(c), a[4] + 1]);
    let mut v = Vec::new();
    f.and((&p).opt()).and((&a).opt()).and((&b).opt()).drive(|u, (((f, p), a), b)| v.push((u, f, [p, a, b].map(|x| x.unwrap_or(0)))));
    v.sort_by_key(|&(_, f, _)| (Reverse(f[0]), Reverse(f[1])));
    rows(v.iter().take(100).map(|&(u, f, d)| {
        row(vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep"), V::I(d[0]), V::I(d[1]), V::I(f[0]), V::I(f[1]), V::I(f[2]), V::I(d[2]), V::T(f[3])])
    }))
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// COUNT(DISTINCT c.Id) AS TotalComments,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes,
// COALESCE(ROUND(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) / NULLIF(COUNT(DISTINCT p.Id), 0), 2), 0) AS AvgUpvoteScorePerPost,
// COALESCE(ROUND(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) / NULLIF(COUNT(DISTINCT p.Id), 0), 2), 0) AS AvgDownvoteScorePerPost
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
// ORDER BY
// u.Reputation DESC;
fn q14984(db: &'static So) -> String {
    let w = UserWhere::All;
    let (p, c) = (ud(db, w, posts_of(db)), ud(db, w, posts_of(db).select(comments_of(db))));
    let v = users_stats_with(db, w, "cv", any_post, &[], &[&p, &c]);
    let per = |x: i64, n: i64| V::F(if n == 0 { 0.0 } else { round2(x as f64 / n as f64) });
    rows(v.iter().map(|(u, s, d)| {
        row(vec![user_col(db, *u, "uid"), user_col(db, *u, "name"), user_col(db, *u, "rep"), V::I(d[0]), V::I(d[1]), V::I(s.up), V::I(s.down), per(s.up, d[0]), per(s.down, d[0])])
    }))
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// u.CreationDate,
// u.LastAccessDate,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS TotalQuestions,
// COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS TotalAnswers,
// SUM(COALESCE(p.ViewCount, 0)) AS TotalViews,
// SUM(COALESCE(p.Score, 0)) AS TotalScore,
// COUNT(DISTINCT c.Id) AS TotalComments,
// COUNT(DISTINCT b.Id) AS TotalBadges,
// MAX(p.LastActivityDate) AS LastActivity
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// u.Id, u.DisplayName, u.Reputation, u.CreationDate, u.LastAccessDate
// ORDER BY
// TotalPosts DESC
// LIMIT 100;
fn q10602(db: &'static So) -> String {
    let w = UserWhere::All;
    let (p, q, a) = (ud(db, w, posts_of(db)), ud(db, w, only_type(db, 1)), ud(db, w, only_type(db, 2)));
    let (c, b) = (ud(db, w, posts_of(db).select(comments_of(db))), ud(db, w, badges_of(db)));
    let f = user_stats_fold(db, Ident::<User>::new(), w, "cb", any_post);
    let mut v = Vec::new();
    f.and((&p).opt()).and((&q).opt()).and((&a).opt()).and((&c).opt()).and((&b).opt()).drive(|u, (((((s, p), q), a), c), b)| v.push((u, s, [p, q, a, c, b].map(|x| x.unwrap_or(0)))));
    v.sort_by_key(|x| Reverse(x.2[0]));
    rows(v.iter().take(100).map(|(u, s, d)| {
        let mut f: Vec<V> = ["uid", "name", "rep", "ucreated", "last_access"].iter().map(|c| user_col(db, *u, c)).collect();
        f.extend([V::I(d[0]), V::I(d[1]), V::I(d[2]), V::I(s.views_sum), V::I(s.score_sum), V::I(d[3]), V::I(d[4]), ustat_field(s, "activity_max")]);
        row(f)
    }))
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// u.CreationDate,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS TotalQuestions,
// COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS TotalAnswers,
// COUNT(DISTINCT c.Id) AS TotalComments,
// COUNT(DISTINCT v.Id) AS TotalVotes,
// SUM(p.ViewCount) AS TotalViews,
// SUM(p.Score) AS TotalScore,
// SUM(CASE WHEN b.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalBadges
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
// u.Id, u.DisplayName, u.Reputation, u.CreationDate
// ORDER BY
// TotalPosts DESC, u.Reputation DESC
// LIMIT 100;
fn q12904(db: &'static So) -> String {
    let w = UserWhere::All;
    let (p, q, a) = (ud(db, w, posts_of(db)), ud(db, w, only_type(db, 1)), ud(db, w, only_type(db, 2)));
    let (c, x) = (ud(db, w, posts_of(db).select(comments_of(db))), ud(db, w, posts_of(db).select(votes_of(db))));
    let f = user_stats_fold(db, Ident::<User>::new(), w, "cvb", any_post);
    let mut v = Vec::new();
    f.and((&p).opt()).and((&q).opt()).and((&a).opt()).and((&c).opt()).and((&x).opt()).drive(|u, (((((s, p), q), a), c), x)| v.push((u, s, [p, q, a, c, x].map(|y| y.unwrap_or(0)))));
    v.sort_by_key(|&(u, _, d)| (Reverse(d[0]), Reverse(db.user.reputation.get(u).unwrap())));
    rows(v.iter().take(100).map(|(u, s, d)| {
        let mut f: Vec<V> = ["uid", "name", "rep", "ucreated"].iter().map(|c| user_col(db, *u, c)).collect();
        f.extend(d.iter().map(|&x| V::I(x)));
        f.extend([ustat_field(s, "views_sum"), ustat_field(s, "score_sum"), V::I(s.bx)]);
        row(f)
    }))
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS TotalQuestions,
// COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS TotalAnswers,
// COUNT(DISTINCT c.Id) AS TotalComments,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes,
// SUM(CASE WHEN b.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalBadges,
// AVG(p.Score) AS AverageScore,
// MAX(p.CreationDate) AS LastPostDate
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
fn q13740(db: &'static So) -> String {
    let w = UserWhere::All;
    let (p, q, a, c) = (ud(db, w, posts_of(db)), ud(db, w, only_type(db, 1)), ud(db, w, only_type(db, 2)), ud(db, w, posts_of(db).select(comments_of(db))));
    let v = users_stats_with(db, w, "cvb", any_post, &[], &[&p, &q, &a, &c]);
    users_rows(db, v, |_, _, d| Reverse(d[0]), 100, &["uid", "name", "#d0", "#d1", "#d2", "#d3", "#up", "#down", "#bx", "score_avg", "created_max"])
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS TotalUpvotedPosts,
// SUM(CASE WHEN p.Score < 0 THEN 1 ELSE 0 END) AS TotalDownvotedPosts,
// AVG(p.ViewCount) AS AverageViewCount,
// AVG(p.AnswerCount) AS AverageAnswerCount
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// GROUP BY u.Id, u.DisplayName
// )
// SELECT
// UserId,
// DisplayName,
// TotalPosts,
// TotalQuestions,
// TotalAnswers,
// TotalUpvotedPosts,
// TotalDownvotedPosts,
// AverageViewCount,
// AverageAnswerCount
// FROM UserPostStats
// ORDER BY TotalPosts DESC
// LIMIT 10;
fn q14402(db: &'static So) -> String {
    let neg = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(&db.post.score)).fold(0i64, |a, s| a + (s < 0) as i64);
    let f = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "", any_post);
    let mut v = Vec::new();
    f.and((&neg).opt()).drive(|u, (s, n)| v.push((u, s, n.unwrap_or(0))));
    v.sort_by_key(|x| Reverse(x.1.n));
    rows(v.iter().take(10).map(|(u, s, n)| {
        let mut f = vec![user_col(db, *u, "uid"), user_col(db, *u, "name")];
        f.extend(["#n", "#q", "#a", "#pos"].iter().map(|c| ustat_field(s, c)));
        f.push(V::I(*n));
        f.extend(["views_avg", "answers_avg"].iter().map(|c| ustat_field(s, c)));
        row(f)
    }))
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(p.Score) AS TotalScore,
// SUM(p.ViewCount) AS TotalViews,
// AVG(p.Score) AS AvgScorePerPost,
// AVG(p.ViewCount) AS AvgViewsPerPost
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName
// )
// SELECT
// u.UserId,
// u.DisplayName,
// u.TotalPosts,
// u.TotalQuestions,
// u.TotalAnswers,
// u.TotalScore,
// u.TotalViews,
// u.AvgScorePerPost,
// u.AvgViewsPerPost
// FROM
// UserPostStats u
// WHERE
// u.TotalPosts > 0
// ORDER BY
// u.TotalScore DESC, u.TotalPosts DESC
// LIMIT 100;
fn q12120(db: &'static So) -> String {
    let f = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "", any_post).filt(|s| s.n > 0);
    let mut v = Vec::new();
    f.drive(|u, s| v.push((u, s, [0; 4])));
    users_rows(db, v, |_, s, _| (Reverse(s.score_sum), Reverse(s.n)), 100, &["uid", "name", "#n", "#q", "#a", "score_sum", "views_sum", "score_avg", "views_avg"])
}

// SELECT
// u.DisplayName AS UserDisplayName,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// AVG(COALESCE(p.Score, 0)) AS AveragePostScore,
// SUM(COALESCE(p.ViewCount, 0)) AS TotalViewCount,
// SUM(COALESCE(c.Id, 0)) AS TotalComments,
// SUM(CASE WHEN b.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalBadges,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes
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
// u.Id, u.DisplayName
// ORDER BY
// TotalPosts DESC
// LIMIT 100;
fn q13055(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let mut v = Vec::new();
    db.user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(view_count.opt()).and(comments_of(db).select(&db.comment.origid).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 10], |a, (p, b)| {
            let mut a = a;
            a[9] += 1;
            a[7] += b.is_some() as i64;
            if let Some(((((t, s), w), c), x)) = p {
                a[0] += 1;
                a[1] += (t == 1) as i64;
                a[2] += (t == 2) as i64;
                a[3] += s;
                a[4] += w.unwrap_or(0);
                a[5] += c.unwrap_or(0);
                a[6] += (x == Some(2)) as i64;
                a[8] += (x == Some(3)) as i64;
            }
            a
        })
        .drive(|u, a| v.push((u, a)));
    v.sort_by_key(|x| Reverse(x.1[0]));
    rows(v.iter().take(100).map(|&(u, a)| {
        row(vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::F(a[3] as f64 / a[9] as f64), V::I(a[4]), V::I(a[5]), V::I(a[7]), V::I(a[6]), V::I(a[8])])
    }))
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN p.PostTypeId IN (4, 5) THEN 1 ELSE 0 END) AS TotalTagWikis,
// COUNT(c.Id) AS TotalComments,
// COUNT(v.CreationDate) AS TotalVotes
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
// )
// SELECT
// UserId,
// DisplayName,
// TotalPosts,
// TotalQuestions,
// TotalAnswers,
// TotalTagWikis,
// TotalComments,
// TotalVotes
// FROM
// UserPostStats
// ORDER BY
// TotalPosts DESC
// LIMIT 10;
fn q10217(db: &'static So) -> String {
    let us = user_base(db, UserWhere::All)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(comments_of(db).opt()).and(votes_of(db).opt())).opt())
        .fold([0i64; 6], |a, p| match p {
            Some(((t, c), v)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (t == 4 || t == 5) as i64, a[4] + c.is_some() as i64, a[5] + v.is_some() as i64],
            None => a,
        });
    let v = top_n(drain(&us), |&(u, a)| (Reverse(a[0]), u), 10);
    rows(v.into_iter().map(|(u, a)| {
        let mut f = ucols(db, u, &["uid", "name"]);
        f.extend(a.map(V::I));
        row(f)
    }))
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(p.ViewCount) AS TotalViews,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// u.Id, u.DisplayName
// )
// SELECT
// us.UserId,
// us.DisplayName,
// us.TotalPosts,
// us.Questions,
// us.Answers,
// us.TotalViews,
// us.UpVotes,
// us.DownVotes,
// (us.UpVotes - us.DownVotes) AS NetVotes
// FROM
// UserPostStats us
// ORDER BY
// us.TotalPosts DESC;
fn q10865(db: &'static So) -> String {
    let v = users_stats_with(db, UserWhere::All, "v", any_post, &[], &[]);
    rows(v.iter().map(|(u, s, _)| {
        let mut f = vec![user_col(db, *u, "uid"), user_col(db, *u, "name")];
        f.extend(["#n", "#q", "#a", "views_sum", "#up", "#down"].iter().map(|c| ustat_field(s, c)));
        f.push(V::I(s.up - s.down));
        row(f)
    }))
}

// WITH UserPostStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END) AS PositiveScorePosts,
// SUM(CASE WHEN P.Score <= 0 THEN 1 ELSE 0 END) AS NegativeScorePosts,
// AVG(P.ViewCount) AS AverageViewCount,
// AVG(P.Score) AS AverageScore
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id, U.DisplayName
// )
// SELECT
// UserId,
// DisplayName,
// TotalPosts,
// TotalQuestions,
// TotalAnswers,
// PositiveScorePosts,
// NegativeScorePosts,
// AverageViewCount,
// AverageScore
// FROM
// UserPostStats
// ORDER BY
// TotalPosts DESC
// LIMIT 100;
fn q12029(db: &'static So) -> String {
    let nonpos = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(&db.post.score)).fold(0i64, |a, s| a + (s <= 0) as i64);
    let f = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "", any_post);
    let mut v = Vec::new();
    f.and((&nonpos).opt()).drive(|u, (s, n)| v.push((u, s, n.unwrap_or(0))));
    v.sort_by_key(|x| Reverse(x.1.n));
    rows(v.iter().take(100).map(|(u, s, n)| {
        let mut f = vec![user_col(db, *u, "uid"), user_col(db, *u, "name")];
        f.extend(["#n", "#q", "#a", "#pos"].iter().map(|c| ustat_field(s, c)));
        f.push(V::I(*n));
        f.extend(["views_avg", "score_avg"].iter().map(|c| ustat_field(s, c)));
        row(f)
    }))
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS TotalUpvotedPosts,
// SUM(CASE WHEN p.Score < 0 THEN 1 ELSE 0 END) AS TotalDownvotedPosts,
// AVG(p.Score) AS AverageScore,
// SUM(c.Score) AS TotalCommentScore
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// GROUP BY
// u.Id, u.DisplayName
// )
// SELECT
// UserId,
// DisplayName,
// TotalPosts,
// TotalQuestions,
// TotalAnswers,
// TotalUpvotedPosts,
// TotalDownvotedPosts,
// AverageScore,
// TotalCommentScore
// FROM
// UserPostStats
// ORDER BY
// TotalPosts DESC
// LIMIT 100;
fn q13766(db: &'static So) -> String {
    let neg = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select((&db.post.score).and(comments_of(db).opt()))).fold(0i64, |a, (s, _)| a + (s < 0) as i64);
    let f = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "c", any_post);
    let mut v = Vec::new();
    f.and((&neg).opt()).drive(|u, (s, n)| v.push((u, s, n.unwrap_or(0))));
    v.sort_by_key(|x| Reverse(x.1.n));
    rows(v.iter().take(100).map(|(u, s, n)| {
        let mut f = vec![user_col(db, *u, "uid"), user_col(db, *u, "name")];
        f.extend(["#n", "#q", "#a", "#pos"].iter().map(|c| ustat_field(s, c)));
        f.push(V::I(*n));
        f.extend(["score_avg", "cscore_sum"].iter().map(|c| ustat_field(s, c)));
        row(f)
    }))
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// AVG(p.Score) AS AverageScore,
// SUM(p.ViewCount) AS TotalViews,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// u.Id, u.DisplayName
// )
// SELECT
// u.DisplayName,
// ups.TotalPosts,
// ups.TotalQuestions,
// ups.TotalAnswers,
// ups.AverageScore,
// ups.TotalViews,
// ups.TotalUpVotes,
// ups.TotalDownVotes,
// u.Reputation
// FROM
// UserPostStats ups
// JOIN
// Users u ON ups.UserId = u.Id
// ORDER BY
// ups.TotalPosts DESC
// LIMIT 10;
fn q10723(db: &'static So) -> String {
    let v = users_stats_with(db, UserWhere::All, "v", any_post, &[], &[]);
    users_rows(db, v, |_, s, _| Reverse(s.n), 10, &["name", "#n", "#q", "#a", "score_avg", "views_sum", "#up", "#down", "rep"])
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount,
// SUM(CASE WHEN v.VoteTypeId = 5 THEN 1 ELSE 0 END) AS FavoriteCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// u.Id, u.DisplayName
// )
// SELECT
// ups.UserId,
// ups.DisplayName,
// ups.PostCount,
// ups.QuestionCount,
// ups.AnswerCount,
// ups.UpvoteCount,
// ups.DownvoteCount,
// ups.FavoriteCount,
// u.Reputation,
// u.CreationDate
// FROM
// UserPostStats ups
// JOIN
// Users u ON ups.UserId = u.Id
// ORDER BY
// ups.PostCount DESC
// FETCH FIRST 10 ROWS ONLY;
fn q14851(db: &'static So) -> String {
    let f = user_vote_groups(db, Ident::<User>::new(), UserWhere::All, &[]);
    let mut v = f;
    v.sort_by_key(|x| Reverse(x.1[0]));
    rows(v.iter().take(10).map(|&(u, a)| {
        row(vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4]), V::I(a[6]), user_col(db, u, "rep"), user_col(db, u, "ucreated")])
    }))
}

// --- post types and whole-table aggregates -----------------------------------

// WITH PostStats AS (
// SELECT
// p.PostTypeId,
// COUNT(*) AS TotalPosts,
// COUNT(DISTINCT p.OwnerUserId) AS TotalOwners,
// SUM(COALESCE(p.ViewCount, 0)) AS TotalViews,
// SUM(COALESCE(p.Score, 0)) AS TotalScore,
// AVG(COALESCE(p.Score, 0)) AS AvgScore,
// AVG(COALESCE(p.ViewCount, 0)) AS AvgViews,
// MAX(p.CreationDate) AS LatestPostDate
// FROM
// Posts p
// WHERE
// p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'
// GROUP BY
// p.PostTypeId
// )
// SELECT
// pt.Name AS PostType,
// ps.TotalPosts,
// ps.TotalOwners,
// ps.TotalViews,
// ps.TotalScore,
// ps.AvgScore,
// ps.AvgViews,
// ps.LatestPostDate
// FROM
// PostStats ps
// JOIN
// PostTypes pt ON ps.PostTypeId = pt.Id
// ORDER BY
// ps.TotalPosts DESC;
fn q12218(db: &'static So) -> String {
    let Post { post_type, score, view_count, owner_user_id, creation_date, .. } = &db.post;
    let base = db.post.with(creation_date.ge(date(2023, 10, 1)));
    let f = (&base).group_by(post_type).select(view_count.opt().and(score).and(creation_date)).fold([0, 0, 0, i64::MIN], |a: [i64; 4], ((w, s), c)| [a[0] + 1, a[1] + w.unwrap_or(0), a[2] + s, a[3].max(c)]);
    let o = (&base).group_by(post_type).select(owner_user_id).count_distinct();
    let mut v = Vec::new();
    f.and((&o).opt()).drive(|t, (a, o)| v.push((db.post_type.name.get(t).unwrap(), a, o.unwrap_or(0))));
    v.sort_by_key(|x| Reverse(x.1[0]));
    rows(v.iter().map(|&(k, a, o)| row(vec![V::S(k), V::I(a[0]), V::I(o), V::I(a[1]), V::I(a[2]), V::F(a[2] as f64 / a[0] as f64), V::F(a[1] as f64 / a[0] as f64), V::T(a[3])])))
}

// SELECT
// p.PostTypeId,
// pt.Name AS PostTypeName,
// COUNT(p.Id) AS TotalPosts,
// SUM(p.Score) AS TotalScore,
// SUM(p.ViewCount) AS TotalViewCount,
// AVG(p.ViewCount) AS AvgViewCount,
// COALESCE(SUM(CASE WHEN p.PostTypeId = 1 THEN 1 END), 0) AS TotalQuestions,
// COALESCE(SUM(CASE WHEN p.PostTypeId = 2 THEN 1 END), 0) AS TotalAnswers,
// COALESCE(SUM(CASE WHEN p.PostTypeId = 3 THEN 1 END), 0) AS TotalWikis,
// COUNT(DISTINCT p.OwnerUserId) AS UniqueUsers,
// SUM(u.Reputation) AS TotalReputation,
// COUNT(DISTINCT c.Id) AS TotalComments,
// COUNT(DISTINCT v.Id) AS TotalVotes
// FROM
// Posts p
// LEFT JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.PostTypeId, pt.Name
// ORDER BY
// p.PostTypeId;
fn q12639(db: &'static So) -> String {
    let Post { post_type_id, post_type, score, view_count, owner_user, owner_user_id, .. } = &db.post;
    let key = post_type_id.and(post_type.select(&db.post_type.name).opt());
    let f = db.post.group_by(&key).select(score.and(view_count.opt()).and(post_type_id).and(owner_user.select(&db.user.reputation).opt()).and(comments_of(db).opt()).and(votes_of(db).opt())).fold([0i64; 10], |a, (((((s, w), t), r), _), _)| {
        [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + (t == 1) as i64, a[5] + (t == 2) as i64, a[6] + (t == 3) as i64, a[7] + r.is_some() as i64, a[8] + r.unwrap_or(0), 0]
    });
    let o = db.post.group_by(&key).select(owner_user_id).count_distinct();
    let c = db.post.group_by(&key).select(comments_of(db)).count_distinct();
    let x = db.post.group_by(&key).select(votes_of(db)).count_distinct();
    let mut v = Vec::new();
    f.and((&o).opt()).and((&c).opt()).and((&x).opt()).drive(|k, (((a, o), c), x)| v.push((k, a, [o, c, x].map(|y| y.unwrap_or(0)))));
    rows(v.iter().map(|&((t, n), a, d)| {
        row(vec![V::I(t), ostr(n), V::I(a[0]), V::I(a[1]), nullable(a[3], a[2]), avg(a[3], a[2]), V::I(a[4]), V::I(a[5]), V::I(a[6]), V::I(d[0]), nullable(a[8], a[7]), V::I(d[1]), V::I(d[2])])
    }))
}

// WITH PostTypeCount AS (
// SELECT
// pt.Name AS PostTypeName,
// COUNT(p.Id) AS PostCount
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// GROUP BY
// pt.Name
// ),
// AvgPostScore AS (
// SELECT
// pt.Name AS PostTypeName,
// AVG(p.Score) AS AverageScore
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// GROUP BY
// pt.Name
// ),
// CommentCount AS (
// SELECT
// pt.Name AS PostTypeName,
// COUNT(c.Id) AS CommentCount
// FROM
// Comments c
// JOIN
// Posts p ON c.PostId = p.Id
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// GROUP BY
// pt.Name
// )
// SELECT
// ptc.PostTypeName,
// ptc.PostCount,
// aps.AverageScore,
// cc.CommentCount
// FROM
// PostTypeCount ptc
// JOIN
// AvgPostScore aps ON ptc.PostTypeName = aps.PostTypeName
// JOIN
// CommentCount cc ON ptc.PostTypeName = cc.PostTypeName
// ORDER BY
// ptc.PostTypeName;
fn q11314(db: &'static So) -> String {
    let name = (&db.post.post_type).select(&db.post_type.name);
    let counts = db.post.group_by(&name).fold(0i64, |a, _| a + 1);
    let scores = db.post.group_by(&name).select(&db.post.score).fold((0i64, 0i64), |(n, s), x| (n + 1, s + x));
    let comments = db.comment.group_by((&db.comment.post).select(&name)).fold(0i64, |a, _| a + 1);
    let mut v = Vec::new();
    counts.and(&scores).and(&comments).drive(|k, ((n, (sn, ss)), c)| v.push((k, n, sn, ss, c)));
    rows(v.iter().map(|&(k, n, sn, ss, c)| row(vec![V::S(k), V::I(n), avg(ss, sn), V::I(c)])))
}

// WITH PostsStats AS (
// SELECT
// PostTypeId,
// COUNT(*) AS PostCount,
// AVG(ViewCount) AS AvgViewCount,
// AVG(Score) AS AvgScore,
// AVG(CommentCount) AS AvgCommentCount,
// AVG(AnswerCount) AS AvgAnswerCount
// FROM
// Posts
// GROUP BY
// PostTypeId
// ),
// UsersStats AS (
// SELECT
// COUNT(*) AS UserCount,
// AVG(Reputation) AS AvgReputation,
// AVG(Views) AS AvgViews,
// AVG(UpVotes) AS AvgUpVotes,
// AVG(DownVotes) AS AvgDownVotes
// FROM
// Users
// )
// SELECT
// p.PostTypeId,
// p.PostCount,
// p.AvgViewCount,
// p.AvgScore,
// p.AvgCommentCount,
// p.AvgAnswerCount,
// u.UserCount,
// u.AvgReputation,
// u.AvgViews,
// u.AvgUpVotes,
// u.AvgDownVotes
// FROM
// PostsStats p,
// UsersStats u
// ORDER BY
// p.PostTypeId;
fn q14278(db: &'static So) -> String {
    let Post { post_type_id, view_count, score, comment_count, answer_count, .. } = &db.post;
    let User { reputation, views, up_votes, down_votes, .. } = &db.user;
    let types = db.post
        .group_by(post_type_id)
        .select(view_count.opt().and(score).and(comment_count).and(answer_count.opt()))
        .fold([0i64; 7], |a, (((w, s), c), an)| [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s, a[4] + c, a[5] + an.is_some() as i64, a[6] + an.unwrap_or(0)]);
    let users = whole(db.user.iq()).select(reputation.and(views).and(up_votes).and(down_votes)).fold([0i64; 5], |a, (((r, v), up), down)| [a[0] + 1, a[1] + r, a[2] + v, a[3] + up, a[4] + down]);
    let us = rel(vec![()]).select((&users).opt());
    let mut v = Vec::new();
    (&types).cross(&us).drive(|(t, _), (a, u)| v.push((t, a, u.unwrap_or([0; 5]))));
    v.sort_by_key(|x| x.0);
    rows(v.iter().map(|&(t, a, u)| {
        row(vec![V::I(t), V::I(a[0]), avg(a[2], a[1]), avg(a[3], a[0]), avg(a[4], a[0]), avg(a[6], a[5]), V::I(u[0]), avg(u[1], u[0]), avg(u[2], u[0]), avg(u[3], u[0]), avg(u[4], u[0])])
    }))
}

// WITH UserPostCounts AS (
// SELECT
// OwnerUserId,
// COUNT(*) AS PostCount
// FROM
// Posts
// WHERE
// PostTypeId = 1
// GROUP BY
// OwnerUserId
// ),
// AvgScore AS (
// SELECT
// AVG(Score) AS AverageScore
// FROM
// Posts
// WHERE
// PostTypeId = 1
// ),
// TopUsers AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// UPC.PostCount
// FROM
// Users U
// JOIN
// UserPostCounts UPC ON U.Id = UPC.OwnerUserId
// ORDER BY
// UPC.PostCount DESC
// LIMIT 10
// )
// SELECT
// (SELECT COUNT(*) FROM Posts) AS TotalPosts,
// (SELECT AverageScore FROM AvgScore) AS AverageQuestionScore,
// TU.UserId,
// TU.DisplayName,
// TU.Reputation,
// TU.PostCount
// FROM
// TopUsers TU;
fn q10868(db: &'static So) -> String {
    let Post { post_type_id, owner_user_id, score, .. } = &db.post;
    let questions = db.post.with(post_type_id.eq(1));
    let per_owner = (&questions).group_by(owner_user_id).fold(0i64, |a, _| a + 1);
    let top = db.user.select((&db.user.origid).select(&per_owner));
    let posts = whole(db.post.iq()).fold(0i64, |a, _| a + 1);
    let qscore = whole(&questions).select(score).fold((0i64, 0i64), |(n, s), x| (n + 1, s + x));
    let scalars = rel(vec![()]).select((&posts).opt().and((&qscore).opt()));
    let mut v = Vec::new();
    (&top).cross(&scalars).drive(|(u, _), (n, (p, q))| v.push((u, n, p.unwrap_or(0), q.unwrap_or((0, 0)))));
    v.sort_by_key(|&(_, n, _, _)| Reverse(n));
    rows(v.iter().take(10).map(|&(u, n, p, (qn, qs))| row(vec![V::I(p), avg(qs, qn), user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep"), V::I(n)])))
}

// WITH PostStats AS (
// SELECT
// COUNT(*) AS TotalPosts,
// AVG(ViewCount) AS AverageViewCount
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
// COUNT(*) AS TotalVotes,
// AVG(VoteCount) AS AverageVotesPerPost
// FROM (
// SELECT
// PostId,
// COUNT(*) AS VoteCount
// FROM
// Votes
// GROUP BY
// PostId
// ) AS PostVoteCounts
// )
// SELECT
// (SELECT TotalPosts FROM PostStats) AS TotalPosts,
// (SELECT AverageViewCount FROM PostStats) AS AverageViewCount,
// (SELECT TotalUsers FROM UserStats) AS TotalUsers,
// (SELECT AverageReputation FROM UserStats) AS AverageReputation,
// (SELECT TotalVotes FROM VoteStats) AS TotalVotes,
// (SELECT AverageVotesPerPost FROM VoteStats) AS AverageVotesPerPost;
fn q12060(db: &'static So) -> String {
    let posts = whole(db.post.iq()).select((&db.post.view_count).opt()).fold([0i64; 3], |a, w| [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0)]);
    let users = whole(db.user.iq()).select(&db.user.reputation).fold([0i64; 2], |a, r| [a[0] + 1, a[1] + r]);
    let per_post = db.vote.group_by(&db.vote.post_id).fold(0i64, |a, _| a + 1);
    let votes = whole(&per_post).select(&per_post).fold([0i64; 2], |a, n| [a[0] + 1, a[1] + n]);
    let mut out = Vec::new();
    rel(vec![()]).select((&posts).opt().and((&users).opt()).and((&votes).opt())).drive(|_, ((p, u), x)| {
        let (p, u, x) = (p.unwrap_or([0; 3]), u.unwrap_or([0; 2]), x.unwrap_or([0; 2]));
        out.push(row(vec![V::I(p[0]), avg(p[2], p[1]), V::I(u[0]), avg(u[1], u[0]), V::I(x[0]), avg(x[1], x[0])]));
    });
    rows(out)
}

// WITH PostMetrics AS (
// SELECT
// COUNT(*) AS TotalPosts,
// AVG(Score) AS AveragePostScore,
// SUM(ViewCount) AS TotalViews,
// SUM(AnswerCount) AS TotalAnswers,
// SUM(CommentCount) AS TotalComments,
// SUM(FavoriteCount) AS TotalFavorites
// FROM
// Posts
// ),
// UserMetrics AS (
// SELECT
// COUNT(*) AS TotalUsers,
// AVG(Reputation) AS AverageReputation,
// SUM(UpVotes) AS TotalUpVotes,
// SUM(DownVotes) AS TotalDownVotes
// FROM
// Users
// )
// SELECT
// PM.TotalPosts,
// PM.AveragePostScore,
// PM.TotalViews,
// PM.TotalAnswers,
// PM.TotalComments,
// PM.TotalFavorites,
// UM.TotalUsers,
// UM.AverageReputation,
// UM.TotalUpVotes,
// UM.TotalDownVotes
// FROM
// PostMetrics PM, UserMetrics UM;
fn q12095(db: &'static So) -> String {
    let Post { score, view_count, answer_count, comment_count, favorite_count, .. } = &db.post;
    let User { reputation, up_votes, down_votes, .. } = &db.user;
    let pm = whole(db.post.iq())
        .select(score.and(view_count.opt()).and(answer_count.opt()).and(comment_count).and(favorite_count.opt()))
        .fold([0i64; 9], |a, ((((s, w), an), c), f)| {
            [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + an.is_some() as i64, a[5] + an.unwrap_or(0), a[6] + c, a[7] + f.is_some() as i64, a[8] + f.unwrap_or(0)]
        });
    let um = whole(db.user.iq()).select(reputation.and(up_votes).and(down_votes)).fold([0i64; 4], |a, ((r, up), down)| [a[0] + 1, a[1] + r, a[2] + up, a[3] + down]);
    let mut out = Vec::new();
    rel(vec![()]).select((&pm).opt().and((&um).opt())).drive(|_, (p, u)| {
        let (p, u) = (p.unwrap_or([0; 9]), u.unwrap_or([0; 4]));
        out.push(row(vec![V::I(p[0]), avg(p[1], p[0]), nullable(p[3], p[2]), nullable(p[5], p[4]), nullable(p[6], p[0]), nullable(p[8], p[7]), V::I(u[0]), avg(u[1], u[0]), nullable(u[2], u[0]), nullable(u[3], u[0])]));
    });
    rows(out)
}

// SELECT
// (SELECT COUNT(*) FROM Posts) AS TotalPosts,
// (SELECT COUNT(*) FROM Users) AS TotalUsers,
// (SELECT COUNT(*) FROM Votes) AS TotalVotes,
// (SELECT COUNT(*) FROM Comments) AS TotalComments,
// (SELECT COUNT(*) FROM Badges) AS TotalBadges,
// (SELECT AVG(Score) FROM Posts WHERE Score IS NOT NULL) AS AvgPostScore,
// (SELECT AVG(Reputation) FROM Users WHERE Reputation IS NOT NULL) AS AvgUserReputation,
// (SELECT COUNT(DISTINCT PostId) FROM Votes WHERE VoteTypeId = 2) AS TotalUpVotes,
// (SELECT COUNT(DISTINCT PostId) FROM Votes WHERE VoteTypeId = 3) AS TotalDownVotes,
// (SELECT COUNT(DISTINCT PostId) FROM PostHistory WHERE PostHistoryTypeId IN (10, 11)) AS TotalPostClosures,
// (SELECT COUNT(DISTINCT PostId) FROM PostLinks) AS TotalPostLinks,
// (SELECT COUNT(*) FROM Tags) AS TotalTags;
fn q12862(db: &'static So) -> String {
    let Vote { vote_type_id, post_id, .. } = &db.vote;
    let posts = whole(db.post.iq()).select(&db.post.score).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let users = whole(db.user.iq()).select(&db.user.reputation).fold([0i64; 2], |a, r| [a[0] + 1, a[1] + r]);
    let votes = whole(db.vote.iq()).fold(0i64, |a, _| a + 1);
    let comments = whole(db.comment.iq()).fold(0i64, |a, _| a + 1);
    let badges = whole(db.badge.iq()).fold(0i64, |a, _| a + 1);
    let tags = whole(db.tag.iq()).fold(0i64, |a, _| a + 1);
    let up = whole(db.vote.with(vote_type_id.eq(2))).select(post_id).count_distinct();
    let down = whole(db.vote.with(vote_type_id.eq(3))).select(post_id).count_distinct();
    let closed = whole(db.post_history.with((&db.post_history.post_history_type_id).in_v(vec![10, 11]))).select(&db.post_history.post_id).count_distinct();
    let linked = whole(db.post_link.iq()).select(&db.post_link.post_id).count_distinct();
    let mut out = Vec::new();
    rel(vec![()])
        .select((&posts).opt().and((&users).opt()).and((&votes).opt()).and((&comments).opt()).and((&badges).opt()).and((&up).opt()).and((&down).opt()).and((&closed).opt()).and((&linked).opt()).and((&tags).opt()))
        .drive(|_, (((((((((p, u), x), c), b), up), down), cl), l), t)| {
            let (p, u, i) = (p.unwrap_or([0; 2]), u.unwrap_or([0; 2]), |v: Option<i64>| V::I(v.unwrap_or(0)));
            out.push(row(vec![V::I(p[0]), V::I(u[0]), i(x), i(c), i(b), avg(p[1], p[0]), avg(u[1], u[0]), i(up), i(down), i(cl), i(l), i(t)]));
        });
    rows(out)
}

// SELECT
// COUNT(DISTINCT u.Id) AS TotalUsers,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// COUNT(DISTINCT c.Id) AS TotalComments,
// COUNT(DISTINCT b.Id) AS TotalBadges,
// COUNT(DISTINCT ph.Id) AS TotalPostHistory,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// AVG(u.Reputation) AS AverageUserReputation,
// AVG(p.Score) AS AveragePostScore,
// AVG(c.Score) AS AverageCommentScore
// FROM
// Users u
// JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year';
fn q14609(db: &'static So) -> String {
    let Post { post_type_id, score, owner_user, .. } = &db.post;
    let base = owned_since(db, year_ago());
    let a = whole(&base)
        .select(post_type_id.and(score).and(owner_user.select(&db.user.reputation)).and(comments_of(db).select(&db.comment.score).opt()).and(owner_user.select(badges_of(db)).opt()).and(history_of(db).opt()))
        .fold([0i64; 7], |a, (((((t, s), r), c), _), _)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + r, a[4] + s, a[5] + c.is_some() as i64, a[6] + c.unwrap_or(0)]);
    let u = whole(&base).select(owner_user).count_distinct();
    let p = whole(&base).count_distinct();
    let c = whole(&base).select(comments_of(db)).count_distinct();
    let b = whole(&base).select(owner_user.select(badges_of(db))).count_distinct();
    let h = whole(&base).select(history_of(db)).count_distinct();
    let mut out = Vec::new();
    rel(vec![()]).select((&a).opt().and((&u).opt()).and((&p).opt()).and((&c).opt()).and((&b).opt()).and((&h).opt())).drive(|_, (((((a, u), p), c), b), h)| {
        let (a, i) = (a.unwrap_or([0; 7]), |v: Option<i64>| V::I(v.unwrap_or(0)));
        let sum = |s: i64| if a[0] == 0 { V::Null } else { V::I(s) };
        out.push(row(vec![i(u), i(p), i(c), i(b), i(h), sum(a[1]), sum(a[2]), avg(a[3], a[0]), avg(a[4], a[0]), avg(a[6], a[5])]));
    });
    rows(out)
}

// SELECT
// COUNT(P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// AVG(CASE WHEN P.PostTypeId = 1 THEN P.Score END) AS AverageQuestionScore,
// AVG(CASE WHEN P.PostTypeId = 2 THEN P.Score END) AS AverageAnswerScore,
// COUNT(DISTINCT U.Id) AS TotalUsers,
// COUNT(DISTINCT T.Id) AS TotalTags,
// SUM(V.BountyAmount) AS TotalBountyAmount
// FROM
// Posts P
// LEFT JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// Tags T ON P.Tags LIKE CONCAT('%', T.TagName, '%')
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// WHERE
// P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 YEAR'
// GROUP BY
// DATE(P.CreationDate)
// ORDER BY
// DATE(P.CreationDate) ASC;
fn q12985(db: &'static So) -> String {
    let mentions = tag_mentions(db);
    let tags_of: HashIdx<Id<Post>, Id<Tag>> = (&mentions).map(|(p, _)| p).inv().select((&mentions).map(|(_, t)| t)).collect();
    let Post { post_type_id, score, owner_user, creation_date, .. } = &db.post;
    let base = db.post.with(creation_date.ge(year_ago()));
    let day = creation_date.map(trunc_day);
    let f = (&base).group_by(&day).select(post_type_id.and(score).and((&tags_of).opt()).and(votes_of(db).select((&db.vote.bounty_amount).opt()).opt())).fold([0i64; 9], |a, (((t, s), _), x)| {
        let b = x.flatten();
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + if t == 1 { s } else { 0 }, a[4] + if t == 2 { s } else { 0 }, a[5] + b.is_some() as i64, a[6] + b.unwrap_or(0), 0, 0]
    });
    let u = (&base).group_by(&day).select(owner_user).count_distinct();
    let t = (&base).group_by(&day).select(&tags_of).count_distinct();
    let mut v = Vec::new();
    f.and((&u).opt()).and((&t).opt()).drive(|_, ((a, u), t)| v.push((a, u.unwrap_or(0), t.unwrap_or(0))));
    rows(v.iter().map(|&(a, u, t)| row(vec![V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[3], a[1]), avg(a[4], a[2]), V::I(u), V::I(t), nullable(a[6], a[5])])))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("11608", q11608),
    ("13278", q13278),
    ("14358", q14358),
    ("10961", q10961),
    ("11832", q11832),
    ("13531", q13531),
    ("13500", q13500),
    ("13788", q13788),
    ("11703", q11703),
    ("10125", q10125),
    ("10403", q10403),
    ("10574", q10574),
    ("11630", q11630),
    ("11138", q11138),
    ("10861", q10861),
    ("10307", q10307),
    ("12520", q12520),
    ("14563", q14563),
    ("14228", q14228),
    ("13138", q13138),
    ("10480", q10480),
    ("14161", q14161),
    ("13362", q13362),
    ("12489", q12489),
    ("8272", q8272),
    ("13051", q13051),
    ("11938", q11938),
    ("10572", q10572),
    ("12833", q12833),
    ("12706", q12706),
    ("10809", q10809),
    ("10645", q10645),
    ("14984", q14984),
    ("10602", q10602),
    ("12904", q12904),
    ("13740", q13740),
    ("14402", q14402),
    ("12120", q12120),
    ("13055", q13055),
    ("10217", q10217),
    ("10865", q10865),
    ("12029", q12029),
    ("13766", q13766),
    ("10723", q10723),
    ("14851", q14851),
    ("12218", q12218),
    ("12639", q12639),
    ("11314", q11314),
    ("14278", q14278),
    ("10868", q10868),
    ("12060", q12060),
    ("12095", q12095),
    ("12862", q12862),
    ("14609", q14609),
    ("12985", q12985),
];
