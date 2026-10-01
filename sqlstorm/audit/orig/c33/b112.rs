use harness::prelude::*;
use std::cmp::Reverse;

fn since(db: &'static So, d: i64) -> impl Drive<D = Id<Post>, R = Id<Post>> {
    db.post.with((&db.post.creation_date).ge(d))
}

fn owned_since(db: &'static So, d: i64) -> impl Drive<D = Id<Post>, R = Id<Post>> {
    owned(db).with((&db.post.creation_date).ge(d))
}

fn questions_only(db: &'static So) -> impl Drive<D = Id<Post>, R = Id<Post>> {
    db.post.with((&db.post.post_type_id).eq(1))
}

fn year_ago() -> i64 {
    ts(2023, 10, 1, 12, 34, 56)
}

fn month_ago() -> i64 {
    ts(2024, 9, 1, 12, 34, 56)
}

fn count<Q: Drive>(q: Q) -> i64 {
    q.fold_flat(0i64, |a, _| a + 1)
}

fn one(f: Fold<(), i64>) -> i64 {
    (&f).fold_flat(0i64, |a, x| a + x)
}

fn ud<R>(db: &'static So, w: UserWhere, r: R) -> Fold<Id<User>, i64>
where
    R: IntoQuery,
    R::Q: Probe<D = Id<User>>,
    ROf<R>: Ord,
{
    user_distinct(db, Ident::<User>::new(), w, r)
}

fn g(db: &'static So) -> GroupBy<Ident<User>, impl Drive<D = Id<User>, R = Id<User>>> {
    user_base(db, UserWhere::All).group_by(Ident::<User>::new())
}

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

fn uids(db: &'static So) -> HashIdx<i64, Id<User>> {
    (&db.user.origid).inv().collect()
}

fn badge_classes(db: &'static So) -> Fold<Id<User>, [i64; 4]> {
    db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 4], |mut a, c| {
        a[0] += 1;
        if (1..4).contains(&c) {
            a[c as usize] += 1;
        }
        a
    })
}

fn views_desc(db: &'static So, p: Id<Post>) -> (bool, Reverse<Option<i64>>) {
    let w = db.post.view_count.get(p);
    (w.is_none(), Reverse(w))
}

fn score_desc(db: &'static So, p: Id<Post>) -> Reverse<i64> {
    Reverse(db.post.score.get(p).unwrap())
}

fn score_views(db: &'static So, p: Id<Post>) -> (Reverse<i64>, (bool, Reverse<Option<i64>>)) {
    (score_desc(db, p), views_desc(db, p))
}

fn views_score(db: &'static So, p: Id<Post>) -> ((bool, Reverse<Option<i64>>), Reverse<i64>) {
    (views_desc(db, p), score_desc(db, p))
}

fn rep_desc(db: &'static So, u: Id<User>) -> Reverse<i64> {
    Reverse(db.user.reputation.get(u).unwrap())
}

fn out<X, T: Ord>(mut v: Vec<X>, key: impl Fn(&X) -> T, n: usize, f: impl Fn(&X) -> Vec<V>) -> String {
    v.sort_by_key(|x| key(x));
    let n = if n == 0 { v.len() } else { n };
    rows(v.iter().take(n).map(|x| row(f(x))))
}

fn secs(us: i64) -> V {
    V::F(us as f64 / 1e6)
}

// --- posts ------------------------------------------------------------------

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// u.DisplayName AS OwnerDisplayName,
// u.Reputation AS OwnerReputation,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// AVG(COALESCE(EXTRACT(EPOCH FROM (h.CreationDate - p.CreationDate)), 0)) AS AverageEditTime,
// MAX(h.CreationDate) AS LastEditDate
// FROM
// Posts p
// JOIN
// Users u ON u.Id = p.OwnerUserId
// LEFT JOIN
// Comments c ON c.PostId = p.Id
// LEFT JOIN
// Votes v ON v.PostId = p.Id
// LEFT JOIN
// PostHistory h ON h.PostId = p.Id
// WHERE
// p.CreationDate >= CURRENT_DATE - INTERVAL '3 months'
// GROUP BY
// p.Id, p.Title, p.CreationDate, u.DisplayName, u.Reputation
// ORDER BY
// p.CreationDate DESC;
fn q10414(db: &'static So) -> String {
    let Post { creation_date, .. } = &db.post;
    let mut v = Vec::new();
    owned(db)
        .with(creation_date.ge(add_months(current_date(), -3)))
        .group_by(Ident::<Post>::new())
        .select(
            creation_date
                .and(comments_of(db).opt())
                .and(votes_of(db).select(&db.vote.vote_type_id).opt())
                .and(history_of(db).select(&db.post_history.creation_date).opt()),
        )
        .fold((0i64, 0i64, 0i64, 0i64, 0i128, i64::MIN), |(n, c, u, d, e, m), (((pc, ci), vt), h)| {
            (
                n + 1,
                c + ci.is_some() as i64,
                u + (vt == Some(2)) as i64,
                d + (vt == Some(3)) as i64,
                e + h.map_or(0, |h| (h - pc) as i128),
                h.map_or(m, |h| m.max(h)),
            )
        })
        .drive(|p, a| v.push((p, a)));
    rows(v.iter().map(|&(p, (n, c, u, d, e, m))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner", "rep"]);
        f.extend([V::I(c), V::I(u), V::I(d), V::F(e as f64 / 1e6 / n as f64), if m == i64::MIN { V::Null } else { V::T(m) }]);
        row(f)
    }))
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 2) AS UpVoteCount,
// COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 3) AS DownVoteCount,
// COUNT(b.Id) AS BadgeCount,
// COUNT(CASE WHEN p.PostTypeId = 1 THEN a.Id END) AS AnswerCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Badges b ON p.OwnerUserId = b.UserId
// LEFT JOIN
// Posts a ON p.Id = a.ParentId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.ViewCount,
// ps.Score,
// ps.CommentCount,
// ps.UpVoteCount,
// ps.DownVoteCount,
// ps.BadgeCount,
// ps.AnswerCount
// FROM
// PostStats ps
// ORDER BY
// ps.Score DESC,
// ps.ViewCount DESC
// LIMIT 100;
fn q12587(db: &'static So) -> String {
    out(stats_with(db, db.post.iq(), "cvba", &[], &[]), |&(p, _, _)| score_views(db, p), 100, |&(p, s, _)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        let q = db.post.post_type_id.get(p).unwrap() == 1;
        f.extend([V::I(s.cx), V::I(s.up), V::I(s.down), V::I(s.bx), V::I(if q { s.ax } else { 0 })]);
        f
    })
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.PostTypeId,
// p.CreationDate,
// COUNT(v.Id) AS VoteCount,
// COUNT(c.Id) AS CommentCount,
// COUNT(b.Id) AS BadgeCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
// FROM
// Posts p
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Badges b ON p.OwnerUserId = b.UserId
// WHERE
// p.CreationDate >= DATE '2023-01-01'
// GROUP BY
// p.Id, p.Title, p.PostTypeId, p.CreationDate
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.PostTypeId,
// ps.CreationDate,
// ps.VoteCount,
// ps.CommentCount,
// ps.BadgeCount,
// ps.Upvotes,
// ps.Downvotes,
// u.DisplayName AS OwnerDisplayName,
// u.Reputation AS OwnerReputation
// FROM
// PostStats ps
// JOIN
// Users u ON ps.PostId = u.Id
// ORDER BY
// ps.CreationDate DESC;
fn q14604(db: &'static So) -> String {
    let uid = uids(db);
    let f = stats_fold(db, since(db, date(2023, 1, 1)).with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cvb", &[]);
    let mut v = Vec::new();
    (&f).and((&db.post.origid).select(&uid)).drive(|p, (s, u)| v.push((p, s, u)));
    rows(v.iter().map(|&(p, s, u)| {
        let mut f = post_fields(db, p, &["id", "title", "type_id", "created"]);
        f.extend([V::I(s.vx), V::I(s.cx), V::I(s.bx), V::I(s.up), V::I(s.down), user_col(db, u, "name"), user_col(db, u, "rep")]);
        row(f)
    }))
}

// WITH PostMetrics AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.Score,
// P.ViewCount,
// P.AnswerCount,
// COUNT(C.Id) AS CommentCount,
// U.DisplayName AS OwnerDisplayName,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount
// FROM
// Posts P
// LEFT JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// WHERE
// P.PostTypeId = 1
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.Score, P.ViewCount, P.AnswerCount, U.DisplayName
// )
// SELECT
// PM.PostId,
// PM.Title,
// PM.CreationDate,
// PM.Score,
// PM.ViewCount,
// PM.AnswerCount,
// PM.CommentCount,
// PM.OwnerDisplayName,
// PM.UpvoteCount,
// PM.DownvoteCount,
// (PM.UpvoteCount - PM.DownvoteCount) AS NetVotes
// FROM
// PostMetrics PM
// ORDER BY
// PM.Score DESC
// LIMIT 100;
fn q12581(db: &'static So) -> String {
    out(stats_with(db, questions_only(db), "cv", &[], &[]), |&(p, _, _)| score_desc(db, p), 100, |&(p, s, _)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers"]);
        f.push(V::I(s.cx));
        f.extend(post_fields(db, p, &["owner"]));
        f.extend([V::I(s.up), V::I(s.down), V::I(s.up - s.down)]);
        f
    })
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
// COUNT(c.Id) AS CommentCount,
// SUM(v.BountyAmount) AS TotalBountyAmount,
// PH.UserDisplayName AS LastEditorDisplayName,
// PH.LastEditorUserId,
// PH.LastEditDate
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (8, 9)
// LEFT JOIN
// (SELECT
// Ph.PostId,
// Ph.UserDisplayName,
// Ph.UserId AS LastEditorUserId,
// Ph.CreationDate AS LastEditDate
// FROM
// PostHistory Ph
// WHERE
// Ph.PostHistoryTypeId IN (4, 5)) PH ON PH.PostId = p.Id
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount,
// u.DisplayName, u.Reputation, PH.UserDisplayName, PH.LastEditorUserId, PH.LastEditDate
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q12447(db: &'static So) -> String {
    let PostHistory { post_history_type_id, user_display_name, user_id, creation_date, .. } = &db.post_history;
    let edits = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.in_v(vec![4, 5])));
    let j: MatSet<(Id<Post>, Option<Id<PostHistory>>)> = owned(db).select(Ident::<Post>::new().and(edits.opt())).collect();
    let post_of = (&j).map(|(p, _)| p);
    let hist_of = (&j).flat_map(|(_, h)| h);
    let bounties = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).in_v(vec![8, 9]))).select((&db.vote.bounty_amount).opt());
    let mut v = Vec::new();
    (&j).group_by((&post_of).and((&hist_of).select(user_display_name.opt().and(user_id.opt()).and(creation_date)).opt()))
        .select((&post_of).select(comments_of(db).opt().and(bounties.opt())))
        .fold((0i64, 0i64, 0i64), |(c, bn, bs), (ci, b)| {
            let b = b.flatten();
            (c + ci.is_some() as i64, bn + b.is_some() as i64, bs + b.unwrap_or(0))
        })
        .drive(|k, a| v.push((k, a)));
    out(v, |&((p, _), _)| newest(db, p), 100, |&((p, h), (c, bn, bs))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "owner", "rep"]);
        f.extend([V::I(c), nullable(bs, bn)]);
        match h {
            Some(((n, u), d)) => f.extend([ostr(n), oint(u), V::T(d)]),
            None => f.extend([V::Null, V::Null, V::Null]),
        }
        f
    })
}

// WITH BenchmarkData AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// u.Reputation AS OwnerReputation,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// pt.Name AS PostType,
// pt.Id AS PostTypeId
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
// GROUP BY
// p.Id, u.Reputation, p.Title, p.CreationDate, pt.Name, pt.Id
// )
// SELECT
// PostId,
// Title,
// CreationDate,
// OwnerReputation,
// CommentCount,
// UpVotes,
// DownVotes,
// CASE
// WHEN PostTypeId = 1 THEN 'Question'
// WHEN PostTypeId IN (2, 3) THEN 'Answer'
// ELSE 'Other'
// END AS PostCategory
// FROM
// BenchmarkData
// ORDER BY
// OwnerReputation DESC, CreationDate DESC
// LIMIT 100;
fn q12402(db: &'static So) -> String {
    let rep = |p: Id<Post>| db.post.owner_user.get(p).map(|u| db.user.reputation.get(u).unwrap());
    out(
        stats_with(db, db.post.iq(), "cv", &[], &[]),
        |&(p, _, _)| {
            let r = rep(p);
            (r.is_none(), Reverse(r), newest(db, p))
        },
        100,
        |&(p, s, _)| {
            let mut f = post_fields(db, p, &["id", "title", "created", "rep"]);
            let c = match db.post.post_type_id.get(p).unwrap() {
                1 => "Question",
                2 | 3 => "Answer",
                _ => "Other",
            };
            f.extend([V::I(s.cx), V::I(s.up), V::I(s.down), V::S(c)]);
            f
        },
    )
}

// WITH PostEngagement AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.ViewCount,
// P.Score,
// COALESCE(P.AnswerCount, 0) AS AnswerCount,
// COALESCE(P.CommentCount, 0) AS CommentCount,
// COALESCE(P.FavoriteCount, 0) AS FavoriteCount,
// U.Reputation AS OwnerReputation,
// PT.Name AS PostType,
// COUNT(V.Id) AS VoteCount
// FROM
// Posts P
// LEFT JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// PostTypes PT ON P.PostTypeId = PT.Id
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.ViewCount, P.Score,
// COALESCE(P.AnswerCount, 0), COALESCE(P.CommentCount, 0),
// COALESCE(P.FavoriteCount, 0), U.Reputation, PT.Name
// )
// SELECT
// PostId,
// Title,
// CreationDate,
// ViewCount,
// Score,
// AnswerCount,
// CommentCount,
// FavoriteCount,
// OwnerReputation,
// PostType,
// VoteCount
// FROM
// PostEngagement
// ORDER BY
// Score DESC, ViewCount DESC
// LIMIT
// 100;
fn q12464(db: &'static So) -> String {
    let Post { answer_count, comment_count, favorite_count, .. } = &db.post;
    out(stats_with(db, db.post.iq(), "v", &[], &[]), |&(p, _, _)| score_views(db, p), 100, |&(p, s, _)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([V::I(answer_count.get(p).unwrap_or(0)), V::I(comment_count.get(p).unwrap()), V::I(favorite_count.get(p).unwrap_or(0))]);
        f.extend(post_fields(db, p, &["rep", "type"]));
        f.push(V::I(s.vx));
        f
    })
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.PostTypeId,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// AVG(LENGTH(p.Body)) AS AvgBodyLength,
// MAX(p.Score) AS MaxScore,
// MAX(p.ViewCount) AS MaxViewCount,
// MIN(p.CreationDate) AS FirstPostDate,
// MAX(p.LastActivityDate) AS LastActivityDate
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.PostTypeId
// )
// SELECT
// pt.Name AS PostType,
// SUM(ps.CommentCount) AS TotalComments,
// SUM(ps.VoteCount) AS TotalVotes,
// AVG(ps.AvgBodyLength) AS AvgBodyLength,
// MAX(ps.MaxScore) AS MaxScoreAcrossPosts,
// MAX(ps.MaxViewCount) AS MaxViewCountAcrossPosts,
// COUNT(DISTINCT ps.PostId) AS TotalPosts,
// MIN(ps.FirstPostDate) AS FirstPostDate,
// MAX(ps.LastActivityDate) AS LastActivityDate
// FROM
// PostStats ps
// JOIN
// PostTypes pt ON ps.PostTypeId = pt.Id
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q12184(db: &'static So) -> String {
    let Post { body, score, view_count, creation_date, last_activity_date, .. } = &db.post;
    let s = stats_fold(db, db.post.iq(), name(db), "cv", &[]);
    let o = by_key(
        db.post.iq(),
        name(db),
        body.and(score).and(view_count.opt()).and(creation_date).and(last_activity_date),
        (0i64, 0i64, i64::MIN, 0i64, i64::MIN, i64::MAX, i64::MIN),
        |(n, len, sm, vn, vm, cmin, lmax), ((((b, sc), w), cd), la)| {
            (n + 1, len + b.chars().count() as i64, sm.max(sc), vn + w.is_some() as i64, w.map_or(vm, |w| vm.max(w)), cmin.min(cd), lmax.max(la))
        },
    );
    let mut v = Vec::new();
    (&s).and(&o).drive(|k, (s, o)| v.push((k, s, o)));
    out(v, |x| Reverse(x.2.0), 0, |&(k, s, (n, len, sm, vn, vm, cmin, lmax))| {
        vec![V::S(k), V::I(s.cx), V::I(s.vx), V::F(len as f64 / n as f64), V::I(sm), omax(vm, vn), V::I(n), V::T(cmin), V::T(lmax)]
    })
}

// WITH Benchmark AS (
// SELECT
// P.Id AS PostId,
// P.CreationDate AS PostCreationDate,
// U.Reputation AS UserReputation,
// COUNT(C.Id) AS CommentCount,
// COUNT(V.Id) AS VoteCount,
// MAX(PH.CreationDate) AS LastHistoryActionDate
// FROM
// Posts P
// LEFT JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// LEFT JOIN
// PostHistory PH ON P.Id = PH.PostId
// WHERE
// P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY
// P.Id, P.CreationDate, U.Reputation
// )
// SELECT
// PostId,
// PostCreationDate,
// UserReputation,
// CommentCount,
// VoteCount,
// LastHistoryActionDate,
// EXTRACT(EPOCH FROM (cast('2024-10-01 12:34:56' as timestamp) - PostCreationDate)) AS AgeInSeconds,
// EXTRACT(EPOCH FROM (cast('2024-10-01 12:34:56' as timestamp) - LastHistoryActionDate)) AS LastActionLagInSeconds
// FROM
// Benchmark
// ORDER BY
// VoteCount DESC,
// CommentCount DESC;
fn q14927(db: &'static So) -> String {
    let t = ts(2024, 10, 1, 12, 34, 56);
    rows(stats_with(db, since(db, year_ago()), "cvh", &[], &[]).iter().map(|&(p, s, _)| {
        let mut f = post_fields(db, p, &["id", "created", "rep"]);
        f.extend([
            V::I(s.cx),
            V::I(s.vx),
            stat_field(&s, "hmax").unwrap(),
            secs(t - db.post.creation_date.get(p).unwrap()),
            if s.hx == 0 { V::Null } else { secs(t - s.hmax) },
        ]);
        row(f)
    }))
}

// WITH BenchmarkData AS (
// SELECT
// p.Id AS PostId,
// p.CreationDate AS PostCreationDate,
// p.Score AS PostScore,
// p.ViewCount AS PostViewCount,
// p.AnswerCount AS PostAnswerCount,
// p.CommentCount AS PostCommentCount,
// u.Reputation AS UserReputation,
// u.CreationDate AS UserCreationDate,
// COUNT(c.Id) AS CommentCountByPost,
// COUNT(b.Id) AS BadgeCountByUser
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY
// p.Id, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, u.Reputation, u.CreationDate
// )
// SELECT
// PostId,
// PostCreationDate,
// PostScore,
// PostViewCount,
// PostAnswerCount,
// PostCommentCount,
// UserReputation,
// UserCreationDate,
// CommentCountByPost,
// BadgeCountByUser
// FROM
// BenchmarkData
// ORDER BY
// PostScore DESC, PostViewCount DESC
// LIMIT 100;
fn q11962(db: &'static So) -> String {
    out(stats_with(db, owned_since(db, year_ago()), "cb", &[], &[]), |&(p, _, _)| (score_views(db, p), db.post.origid.get(p).unwrap()), 100, |&(p, s, _)| {
        let mut f = post_fields(db, p, &["id", "created", "score", "views", "answers", "comments", "rep", "ucreated"]);
        f.extend([V::I(s.cx), V::I(s.bx)]);
        f
    })
}

// WITH PostSummary AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// (SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id) AS TotalVotes,
// (SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id) AS TotalComments,
// (SELECT COUNT(*) FROM Posts p2 WHERE p2.ParentId = p.Id) AS TotalAnswers,
// p.OwnerUserId  -- Adding OwnerUserId to the selection
// FROM
// Posts p
// ),
// UserSummary AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// (SELECT COUNT(*) FROM Badges b WHERE b.UserId = u.Id) AS TotalBadges
// FROM
// Users u
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.Score,
// ps.ViewCount,
// ps.TotalVotes,
// ps.TotalComments,
// ps.TotalAnswers,
// us.UserId,
// us.DisplayName,
// us.Reputation,
// us.TotalBadges,
// (SELECT COUNT(*) FROM Users) AS TotalUsers,
// (SELECT COUNT(*) FROM Posts) AS TotalPosts
// FROM
// PostSummary ps
// JOIN
// Users u ON ps.OwnerUserId = u.Id
// JOIN
// UserSummary us ON us.UserId = u.Id
// ORDER BY
// ps.CreationDate DESC
// FETCH FIRST 100 ROWS ONLY;  -- Using standard SQL for limiting rows
fn q14394(db: &'static So) -> String {
    let (tu, tp) = (count(db.user.iq()), count(db.post.iq()));
    let mut v = Vec::new();
    owned(db)
        .select(
            Ident::<Post>::new()
                .and(votes_per_post(db))
                .and(comments_per_post(db))
                .and(answers_per_post(db))
                .and((&db.post.owner_user).select(Ident::<User>::new().and(badges_per_user(db)))),
        )
        .drive(|_, x| v.push(x));
    out(v, |&((((p, _), _), _), _)| newest(db, p), 100, |&((((p, x), c), a), (u, b))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(x), V::I(c), V::I(a), user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep"), V::I(b), V::I(tu), V::I(tp)]);
        f
    })
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// p.AnswerCount,
// p.CommentCount,
// u.DisplayName AS OwnerDisplayName,
// COUNT(c.Id) AS TotalComments,
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
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, p.CommentCount, u.DisplayName
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.ViewCount,
// ps.Score,
// ps.AnswerCount,
// ps.CommentCount,
// ps.OwnerDisplayName,
// ps.TotalComments,
// ps.UpVotes,
// ps.DownVotes,
// (ps.UpVotes - ps.DownVotes) AS VoteBalance,
// CASE
// WHEN ps.Score > 0 THEN 'Positive'
// WHEN ps.Score < 0 THEN 'Negative'
// ELSE 'Neutral'
// END AS ScoreCategory
// FROM
// PostStats ps
// ORDER BY
// ps.Score DESC, ps.ViewCount DESC;
fn q12030(db: &'static So) -> String {
    rows(stats_with(db, questions_only(db), "cv", &[], &[]).iter().map(|&(p, s, _)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "answers", "comments", "owner"]);
        let sc = db.post.score.get(p).unwrap();
        f.extend([
            V::I(s.cx),
            V::I(s.up),
            V::I(s.down),
            V::I(s.up - s.down),
            V::S(if sc > 0 {
                "Positive"
            } else if sc < 0 {
                "Negative"
            } else {
                "Neutral"
            }),
        ]);
        row(f)
    }))
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.PostTypeId,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// COUNT(DISTINCT c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
// AVG(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// AVG(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= '2022-01-01'
// GROUP BY
// p.Id, p.PostTypeId, p.CreationDate, p.ViewCount, p.Score
// ),
// PostTypesStats AS (
// SELECT
// pt.Id AS PostTypeId,
// pt.Name AS PostTypeName,
// COUNT(ps.PostId) AS TotalPosts,
// SUM(ps.ViewCount) AS TotalViews,
// SUM(ps.Score) AS TotalScore,
// SUM(ps.CommentCount) AS TotalComments,
// SUM(ps.VoteCount) AS TotalVotes,
// AVG(ps.UpVotes) AS AvgUpVotes,
// AVG(ps.DownVotes) AS AvgDownVotes
// FROM
// PostStats ps
// JOIN
// PostTypes pt ON ps.PostTypeId = pt.Id
// GROUP BY
// pt.Id, pt.Name
// )
// SELECT
// PostTypeId,
// PostTypeName,
// TotalPosts,
// TotalViews,
// TotalScore,
// TotalComments,
// TotalVotes,
// AvgUpVotes,
// AvgDownVotes
// FROM
// PostTypesStats
// ORDER BY
// TotalPosts DESC;
fn q10017(db: &'static So) -> String {
    let Post { view_count, score, post_type, .. } = &db.post;
    let s = stats_fold(db, since(db, date(2022, 1, 1)), Ident::<Post>::new(), "cv", &[]);
    let c = per_post_distinct(db, comments_of(db));
    let x = per_post_distinct(db, votes_of(db));
    let mut v = Vec::new();
    since(db, date(2022, 1, 1))
        .group_by(post_type)
        .select(view_count.opt().and(score).and(&s).and((&c).opt()).and((&x).opt()))
        .fold((0i64, 0i64, 0i64, 0i64, 0i64, 0i64, 0.0f64, 0.0f64), |(n, vn, vs, ss, cs, xs, uf, df), ((((w, sc), st), ci), xi)| {
            (
                n + 1,
                vn + w.is_some() as i64,
                vs + w.unwrap_or(0),
                ss + sc,
                cs + ci.unwrap_or(0),
                xs + xi.unwrap_or(0),
                uf + st.up as f64 / st.rows as f64,
                df + st.down as f64 / st.rows as f64,
            )
        })
        .drive(|t, a| v.push((t, a)));
    out(v, |x| Reverse(x.1.0), 0, |&(t, (n, vn, vs, ss, cs, xs, uf, df))| {
        vec![
            V::I(db.post_type.origid.get(t).unwrap()),
            V::S(db.post_type.name.get(t).unwrap()),
            V::I(n),
            nullable(vs, vn),
            V::I(ss),
            V::I(cs),
            V::I(xs),
            V::F(uf / n as f64),
            V::F(df / n as f64),
        ]
    })
}

// WITH PostStats AS (
// SELECT
// Posts.Id AS PostId,
// Posts.Title,
// Posts.CreationDate,
// Posts.Score,
// Posts.ViewCount,
// COUNT(Comments.Id) AS CommentCount,
// COUNT(Votes.Id) AS VoteCount,
// SUM(CASE WHEN Votes.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN Votes.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
// Users.DisplayName AS OwnerDisplayName
// FROM
// Posts
// LEFT JOIN
// Comments ON Posts.Id = Comments.PostId
// LEFT JOIN
// Votes ON Posts.Id = Votes.PostId
// LEFT JOIN
// Users ON Posts.OwnerUserId = Users.Id
// GROUP BY
// Posts.Id, Posts.Title, Posts.CreationDate, Posts.Score, Posts.ViewCount, Users.DisplayName
// ),
// UserStats AS (
// SELECT
// Users.Id AS UserId,
// Users.DisplayName,
// COUNT(Badges.Id) AS BadgeCount,
// SUM(Posts.ViewCount) AS TotalViews,
// SUM(Posts.Score) AS TotalScore
// FROM
// Users
// LEFT JOIN
// Posts ON Users.Id = Posts.OwnerUserId
// LEFT JOIN
// Badges ON Users.Id = Badges.UserId
// GROUP BY
// Users.Id, Users.DisplayName
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.Score,
// ps.ViewCount,
// ps.CommentCount,
// ps.VoteCount,
// ps.UpVoteCount,
// ps.DownVoteCount,
// ps.OwnerDisplayName,
// us.UserId,
// us.DisplayName AS UserDisplayName,
// us.BadgeCount,
// us.TotalViews,
// us.TotalScore
// FROM
// PostStats ps
// JOIN
// UserStats us ON ps.OwnerDisplayName = us.DisplayName
// ORDER BY
// ps.CreationDate DESC;
fn q10031(db: &'static So) -> String {
    let uf = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "b", any_post);
    let by_name: HashIdx<Str, Id<User>> = (&db.user.display_name).inv().collect();
    let pf = stats_fold(db, owned(db), Ident::<Post>::new(), "cv", &[]);
    let mut v = Vec::new();
    (&pf)
        .and((&db.post.owner_user).select(&db.user.display_name).select(&by_name).select(Ident::<User>::new().and(&uf)))
        .drive(|p, (s, (u, us))| v.push((p, s, u, us)));
    rows(v.iter().map(|&(p, s, u, us)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(s.cx), V::I(s.vx), V::I(s.up), V::I(s.down)]);
        f.extend(post_fields(db, p, &["owner"]));
        f.extend([user_col(db, u, "uid"), user_col(db, u, "name")]);
        f.extend(["#bx", "views_sum", "score_sum"].iter().map(|c| ustat_field(&us, c)));
        row(f)
    }))
}

// WITH PostMetrics AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.Score,
// P.ViewCount,
// P.AnswerCount,
// P.CommentCount,
// U.DisplayName AS OwnerDisplayName,
// P.LastActivityDate,
// PT.Name AS PostType
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// JOIN
// PostTypes PT ON P.PostTypeId = PT.Id
// ),
// VoteMetrics AS (
// SELECT
// PostId,
// COUNT(CASE WHEN VoteTypeId = 2 THEN 1 END) AS Upvotes,
// COUNT(CASE WHEN VoteTypeId = 3 THEN 1 END) AS Downvotes
// FROM
// Votes
// GROUP BY
// PostId
// ),
// BadgeMetrics AS (
// SELECT
// UserId,
// COUNT(*) AS BadgeCount
// FROM
// Badges
// GROUP BY
// UserId
// )
// SELECT
// PM.PostId,
// PM.Title,
// PM.CreationDate,
// PM.Score,
// PM.ViewCount,
// PM.AnswerCount,
// PM.CommentCount,
// PM.OwnerDisplayName,
// PM.LastActivityDate,
// PM.PostType,
// VM.Upvotes,
// VM.Downvotes,
// BM.BadgeCount
// FROM
// PostMetrics PM
// LEFT JOIN
// VoteMetrics VM ON PM.PostId = VM.PostId
// LEFT JOIN
// BadgeMetrics BM ON PM.OwnerDisplayName = (SELECT DisplayName FROM Users WHERE Id = BM.UserId)
// ORDER BY
// PM.LastActivityDate DESC
// LIMIT 100;
fn q10060(db: &'static So) -> String {
    let vm = db.vote.group_by(&db.vote.post_id).select(&db.vote.vote_type_id).fold((0i64, 0i64), |(u, d), t| (u + (t == 2) as i64, d + (t == 3) as i64));
    let bm = db.badge.group_by(&db.badge.user).fold(0i64, |a, _| a + 1);
    let by_name: HashIdx<Str, Id<User>> = (&db.user.display_name).inv().collect();
    let mut v = Vec::new();
    owned(db)
        .select(
            Ident::<Post>::new()
                .and((&db.post.origid).select(&vm).opt())
                .and((&db.post.owner_user).select(&db.user.display_name).select(&by_name).select(&bm).opt()),
        )
        .drive(|_, x| v.push(x));
    out(v, |&((p, _), _)| Reverse(db.post.last_activity_date.get(p).unwrap()), 100, |&((p, x), b)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "owner", "activity", "type"]);
        f.extend([oint(x.map(|x| x.0)), oint(x.map(|x| x.1)), oint(b)]);
        f
    })
}

// WITH PostMetrics AS (
// SELECT
// pt.Name AS PostType,
// AVG(p.Score) AS AverageScore,
// AVG(p.ViewCount) AS AverageViewCount,
// COUNT(c.Id) AS CommentsCount
// FROM
// Posts p
// LEFT JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// GROUP BY
// pt.Name
// )
// , TotalCounts AS (
// SELECT
// COUNT(DISTINCT p.Id) AS TotalPosts,
// COUNT(DISTINCT u.Id) AS TotalUsers
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// )
// SELECT
// pm.PostType,
// pm.AverageScore,
// pm.AverageViewCount,
// pm.CommentsCount,
// tc.TotalPosts,
// tc.TotalUsers
// FROM
// PostMetrics pm,
// TotalCounts tc
// ORDER BY
// pm.PostType;
fn q10061(db: &'static So) -> String {
    let Post { score, view_count, .. } = &db.post;
    let f = by_key(db.post.iq(), name(db), score.and(view_count.opt()).and(comments_of(db).opt()), [0i64; 5], |a, ((s, w), c)| {
        [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + c.is_some() as i64]
    });
    let tp = one(whole(owned(db)).select(Ident::<Post>::new()).count_distinct());
    let tu = one(whole(owned(db)).select(&db.post.owner_user).count_distinct());
    let mut v = Vec::new();
    (&f).drive(|k, a| v.push((k, a)));
    out(v, |x| x.0, 0, |&(k, a)| vec![V::S(k), avg(a[1], a[0]), avg(a[3], a[2]), V::I(a[4]), V::I(tp), V::I(tu)])
}

// WITH PostStatistics AS (
// SELECT
// p.PostTypeId,
// COUNT(p.Id) AS TotalPosts,
// SUM(p.Score) AS TotalScore,
// SUM(p.ViewCount) AS TotalViews,
// AVG(u.Reputation) AS AverageReputation
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// GROUP BY
// p.PostTypeId
// ),
// PostTypeNames AS (
// SELECT
// pt.Id AS PostTypeId,
// pt.Name AS PostTypeName
// FROM
// PostTypes pt
// )
// SELECT
// ptn.PostTypeName,
// ps.TotalPosts,
// ps.TotalScore,
// ps.TotalViews,
// ps.AverageReputation
// FROM
// PostStatistics ps
// JOIN
// PostTypeNames ptn ON ps.PostTypeId = ptn.PostTypeId
// ORDER BY
// ps.TotalPosts DESC;
fn q10063(db: &'static So) -> String {
    rows(by_count(db).iter().map(|a| row(type_fields(a, &["name", "n", "score_sum", "views_sum", "rep_avg"]))))
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
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
// p.Id, p.Title, p.CreationDate
// ),
// UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(b.Id) AS BadgeCount,
// SUM(up.Reputation) AS TotalReputation
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// LEFT JOIN
// Users up ON u.AccountId = up.Id
// GROUP BY
// u.Id, u.DisplayName
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.CommentCount,
// ps.VoteCount,
// ps.UpVoteCount,
// ps.DownVoteCount,
// us.UserId,
// us.DisplayName,
// us.BadgeCount,
// us.TotalReputation
// FROM
// PostStats ps
// JOIN
// Users u ON ps.PostId = u.Id
// JOIN
// UserStats us ON u.Id = us.UserId
// ORDER BY
// ps.CreationDate DESC;
fn q10064(db: &'static So) -> String {
    let uid = uids(db);
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and((&db.user.account_id).select(&uid).select(&db.user.reputation).opt()))
        .fold((0i64, 0i64, 0i64), |(b, n, s), (bi, r)| (b + bi.is_some() as i64, n + r.is_some() as i64, s + r.unwrap_or(0)));
    let pf = stats_fold(db, since(db, date(2023, 1, 1)).with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cv", &[]);
    let mut v = Vec::new();
    (&pf).and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&us))).drive(|p, (s, (u, a))| v.push((p, s, u, a)));
    rows(v.iter().map(|&(p, s, u, (b, n, r))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(s.cx), V::I(s.vx), V::I(s.up), V::I(s.down), user_col(db, u, "uid"), user_col(db, u, "name"), V::I(b), nullable(r, n)]);
        row(f)
    }))
}

// WITH UserBadgeCounts AS (
// SELECT
// UserId,
// COUNT(CASE WHEN Class = 1 THEN 1 END) AS GoldBadges,
// COUNT(CASE WHEN Class = 2 THEN 1 END) AS SilverBadges,
// COUNT(CASE WHEN Class = 3 THEN 1 END) AS BronzeBadges
// FROM
// Badges
// GROUP BY
// UserId
// ),
// PostSummary AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// u.Reputation,
// COALESCE(ub.GoldBadges, 0) AS GoldBadges,
// COALESCE(ub.SilverBadges, 0) AS SilverBadges,
// COALESCE(ub.BronzeBadges, 0) AS BronzeBadges
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// UserBadgeCounts ub ON u.Id = ub.UserId
// )
// SELECT
// PostId,
// Title,
// CreationDate,
// ViewCount,
// Score,
// Reputation,
// GoldBadges,
// SilverBadges,
// BronzeBadges
// FROM
// PostSummary
// ORDER BY
// ViewCount DESC, Score DESC
// LIMIT 100;
fn q10068(db: &'static So) -> String {
    let ub = badge_classes(db);
    let mut v = Vec::new();
    owned(db).select(Ident::<Post>::new().and((&db.post.owner_user).select(&ub).opt())).drive(|_, x| v.push(x));
    out(v, |&(p, _)| views_score(db, p), 100, |&(p, b)| {
        let b = b.unwrap_or([0; 4]);
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "rep"]);
        f.extend([V::I(b[1]), V::I(b[2]), V::I(b[3])]);
        f
    })
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// COALESCE(a.AnswerCount, 0) AS AnswerCount,
// COALESCE(c.CommentCount, 0) AS CommentCount,
// COALESCE(b.BadgeCount, 0) AS BadgeCount
// FROM
// Posts p
// LEFT JOIN (
// SELECT
// ParentId, COUNT(*) AS AnswerCount
// FROM
// Posts
// WHERE
// PostTypeId = 2
// GROUP BY
// ParentId
// ) a ON p.Id = a.ParentId
// LEFT JOIN (
// SELECT
// PostId, COUNT(*) AS CommentCount
// FROM
// Comments
// GROUP BY
// PostId
// ) c ON p.Id = c.PostId
// LEFT JOIN (
// SELECT
// UserId, COUNT(*) AS BadgeCount
// FROM
// Badges
// GROUP BY
// UserId
// ) b ON p.OwnerUserId = b.UserId
// WHERE
// p.PostTypeId = 1
// )
// SELECT
// Title,
// CreationDate,
// Score,
// ViewCount,
// AnswerCount,
// CommentCount,
// BadgeCount
// FROM
// PostStats
// ORDER BY
// Score DESC, ViewCount DESC
// LIMIT 100;
fn q10111(db: &'static So) -> String {
    let mut v = Vec::new();
    questions_only(db)
        .select(Ident::<Post>::new().and(typed_answers_per_post(db)).and(comments_per_post(db)).and((&db.post.owner_user).select(badges_per_user(db)).opt()))
        .drive(|_, x| v.push(x));
    out(v, |&(((p, _), _), _)| score_views(db, p), 100, |&(((p, a), c), b)| {
        let mut f = post_fields(db, p, &["title", "created", "score", "views"]);
        f.extend([V::I(a), V::I(c), V::I(b.unwrap_or(0))]);
        f
    })
}

// WITH PostStats AS (
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
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount
// ),
// UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// u.Id, u.DisplayName
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.Score,
// ps.ViewCount,
// ps.CommentCount,
// ps.VoteCount,
// us.UserId,
// us.DisplayName,
// us.GoldBadges,
// us.SilverBadges,
// us.BronzeBadges
// FROM
// PostStats ps
// JOIN
// UserStats us ON ps.PostId = us.UserId
// ORDER BY
// ps.Score DESC, ps.ViewCount DESC
// LIMIT 100;
fn q10115(db: &'static So) -> String {
    let uid = uids(db);
    let ub = badge_classes(db);
    let pf = stats_fold(db, db.post.with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cvb", &[]);
    let mut v = Vec::new();
    (&pf).and((&db.post.origid).select(&uid).select(Ident::<User>::new().and((&ub).opt()))).drive(|p, (s, (u, b))| v.push((p, s, u, b)));
    out(v, |&(p, _, _, _)| score_views(db, p), 100, |&(p, s, u, b)| {
        let b = b.unwrap_or([0; 4]);
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(s.cx), V::I(s.vx), user_col(db, u, "uid"), user_col(db, u, "name"), V::I(b[1]), V::I(b[2]), V::I(b[3])]);
        f
    })
}

// WITH PostInteraction AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// COUNT(c.Id) AS CommentCount,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 month'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score
// )
// SELECT
// pi.PostId,
// pi.Title,
// pi.CreationDate,
// pi.ViewCount,
// pi.Score,
// pi.CommentCount,
// pi.UpVoteCount,
// pi.DownVoteCount,
// u.DisplayName AS OwnerDisplayName,
// u.Reputation AS OwnerReputation
// FROM
// PostInteraction pi
// JOIN
// Users u ON pi.PostId = u.AccountId
// ORDER BY
// pi.Score DESC, pi.ViewCount DESC
// LIMIT 100;
fn q10116(db: &'static So) -> String {
    let acc: HashIdx<i64, Id<User>> = (&db.user.account_id).inv().collect();
    let pf = stats_fold(db, since(db, month_ago()).with((&db.post.origid).select(&acc)), Ident::<Post>::new(), "cv", &[]);
    let mut v = Vec::new();
    (&pf).and((&db.post.origid).select(&acc)).drive(|p, (s, u)| v.push((p, s, u)));
    out(v, |&(p, _, _)| score_views(db, p), 100, |&(p, s, u)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([V::I(s.cx), V::I(s.up), V::I(s.down), user_col(db, u, "name"), user_col(db, u, "rep")]);
        f
    })
}

// WITH PostSummary AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.Score,
// P.ViewCount,
// COALESCE(A.AnswerCount, 0) AS AnswerCount,
// COALESCE(C.CommentCount, 0) AS CommentCount,
// U.DisplayName AS OwnerDisplayName,
// U.Reputation AS OwnerReputation
// FROM
// Posts P
// LEFT JOIN (
// SELECT
// ParentId,
// COUNT(*) AS AnswerCount
// FROM
// Posts
// WHERE
// PostTypeId = 2
// GROUP BY
// ParentId
// ) A ON P.Id = A.ParentId
// LEFT JOIN (
// SELECT
// PostId,
// COUNT(*) AS CommentCount
// FROM
// Comments
// GROUP BY
// PostId
// ) C ON P.Id = C.PostId
// LEFT JOIN
// Users U ON P.OwnerUserId = U.Id
// WHERE
// P.PostTypeId = 1
// )
// SELECT
// PS.PostId,
// PS.Title,
// PS.CreationDate,
// PS.Score,
// PS.ViewCount,
// PS.AnswerCount,
// PS.CommentCount,
// PS.OwnerDisplayName,
// PS.OwnerReputation,
// PH.CreationDate AS LastEditDate,
// PH.UserDisplayName AS LastEditor
// FROM
// PostSummary PS
// LEFT JOIN
// PostHistory PH ON PS.PostId = PH.PostId
// WHERE
// PH.PostHistoryTypeId IN (4, 5, 6)
// ORDER BY
// PS.CreationDate DESC
// LIMIT 100;
fn q10107(db: &'static So) -> String {
    let PostHistory { post_history_type_id, creation_date, user_display_name, .. } = &db.post_history;
    let edits = history_of(db).select(Ident::<PostHistory>::new().with(post_history_type_id.in_v(vec![4, 5, 6]))).select(creation_date.and(user_display_name.opt()));
    let mut v = Vec::new();
    questions_only(db)
        .select(Ident::<Post>::new().and(typed_answers_per_post(db)).and(comments_per_post(db)).and(edits))
        .drive(|_, x| v.push(x));
    out(v, |&(((p, _), _), _)| newest(db, p), 100, |&(((p, a), c), (d, n))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(a), V::I(c)]);
        f.extend(post_fields(db, p, &["owner", "rep"]));
        f.extend([V::T(d), ostr(n)]);
        f
    })
}

// WITH PostDetails AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.OwnerUserId,
// u.DisplayName AS OwnerDisplayName,
// COUNT(c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
// SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes
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
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.OwnerUserId, u.DisplayName
// ),
// UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(b.Id) AS BadgeCount,
// SUM(p.ViewCount) AS TotalPostViews,
// SUM(p.AnswerCount) AS TotalAnswers
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
// )
// SELECT
// pd.PostId,
// pd.Title,
// pd.CreationDate,
// pd.OwnerDisplayName,
// us.UserId,
// us.DisplayName AS UserDisplayName,
// us.Reputation,
// pd.CommentCount,
// pd.VoteCount,
// pd.UpVotes,
// pd.DownVotes,
// us.BadgeCount,
// us.TotalPostViews,
// us.TotalAnswers
// FROM
// PostDetails pd
// JOIN
// UserStats us ON pd.OwnerUserId = us.UserId
// ORDER BY
// pd.CreationDate DESC
// LIMIT 100;
fn q10108(db: &'static So) -> String {
    let x = per_post_distinct(db, votes_of(db));
    let pf = stats_fold(db, owned(db), Ident::<Post>::new(), "cv", &[]);
    let uf = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "b", any_post);
    let mut v = Vec::new();
    (&pf)
        .and((&x).opt())
        .and((&db.post.owner_user).select(Ident::<User>::new().and(&uf)))
        .drive(|p, ((s, x), (u, us))| v.push((p, s, x.unwrap_or(0), u, us)));
    out(v, |&(p, _, _, _, _)| newest(db, p), 100, |&(p, s, x, u, us)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend([user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep"), V::I(s.cx), V::I(x), V::I(s.upn), V::I(s.downn)]);
        f.extend(["#bx", "views_sum", "answers_sum"].iter().map(|c| ustat_field(&us, c)));
        f
    })
}

// WITH PostStatistics AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// p.AnswerCount,
// p.CommentCount,
// COALESCE(UPV.UpVoteCount, 0) AS UpVoteCount,
// COALESCE(DNV.DownVoteCount, 0) AS DownVoteCount,
// COALESCE(MV.ModeratorReviewCount, 0) AS ModeratorReviewCount,
// COALESCE(CR.CloseReasonCount, 0) AS CloseReasonCount
// FROM
// Posts p
// LEFT JOIN (
// SELECT
// PostId,
// COUNT(*) AS UpVoteCount
// FROM
// Votes
// WHERE
// VoteTypeId = 2
// GROUP BY
// PostId
// ) UPV ON p.Id = UPV.PostId
// LEFT JOIN (
// SELECT
// PostId,
// COUNT(*) AS DownVoteCount
// FROM
// Votes
// WHERE
// VoteTypeId = 3
// GROUP BY
// PostId
// ) DNV ON p.Id = DNV.PostId
// LEFT JOIN (
// SELECT
// PostId,
// COUNT(*) AS ModeratorReviewCount
// FROM
// Votes
// WHERE
// VoteTypeId = 15
// GROUP BY
// PostId
// ) MV ON p.Id = MV.PostId
// LEFT JOIN (
// SELECT
// PostId,
// COUNT(*) AS CloseReasonCount
// FROM
// PostHistory
// WHERE
// PostHistoryTypeId = 10
// GROUP BY
// PostId
// ) CR ON p.Id = CR.PostId
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.Score,
// ps.ViewCount,
// ps.AnswerCount,
// ps.CommentCount,
// ps.UpVoteCount,
// ps.DownVoteCount,
// ps.ModeratorReviewCount,
// ps.CloseReasonCount
// FROM
// PostStatistics ps
// ORDER BY
// ps.Score DESC,
// ps.ViewCount DESC
// LIMIT 100;
fn q10140(db: &'static So) -> String {
    let closes = db
        .post_history
        .with((&db.post_history.post_history_type_id).eq(10))
        .select(&db.post_history.post)
        .inv()
        .dense_fold_outer(db.post.id.n, 0i64, |a, _| a + 1);
    let mut v = Vec::new();
    db.post
        .select(Ident::<Post>::new().and(votes_of_type(db, 2)).and(votes_of_type(db, 3)).and(votes_of_type(db, 15)).and(closes))
        .drive(|_, x| v.push(x));
    out(v, |&((((p, _), _), _), _)| score_views(db, p), 100, |&((((p, u), d), m), c)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments"]);
        f.extend([V::I(u), V::I(d), V::I(m), V::I(c)]);
        f
    })
}

// WITH PostEngagement AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.ViewCount,
// P.Score,
// COALESCE((SELECT COUNT(*) FROM Comments C WHERE C.PostId = P.Id), 0) AS CommentCount,
// COALESCE((SELECT COUNT(*) FROM Votes V WHERE V.PostId = P.Id AND V.VoteTypeId = 2), 0) AS UpvoteCount,
// COALESCE((SELECT COUNT(*) FROM Votes V WHERE V.PostId = P.Id AND V.VoteTypeId = 3), 0) AS DownvoteCount,
// COALESCE((SELECT COUNT(*) FROM Badges B WHERE B.UserId = P.OwnerUserId), 0) AS BadgeCount,
// U.Reputation AS OwnerReputation,
// U.DisplayName AS OwnerDisplayName
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// WHERE
// P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'
// ),
// PostHistoryEngagement AS (
// SELECT
// PH.PostId,
// PHT.Name AS HistoryType,
// COUNT(*) AS HistoryCount
// FROM
// PostHistory PH
// JOIN
// PostHistoryTypes PHT ON PH.PostHistoryTypeId = PHT.Id
// GROUP BY
// PH.PostId, PHT.Name
// )
// SELECT
// PE.PostId,
// PE.Title,
// PE.CreationDate,
// PE.ViewCount,
// PE.Score,
// PE.CommentCount,
// PE.UpvoteCount,
// PE.DownvoteCount,
// PE.BadgeCount,
// PE.OwnerReputation,
// PE.OwnerDisplayName,
// COALESCE(PHE.HistoryCount, 0) AS EditCount,
// COALESCE(PHE.HistoryCount, 0) AS ClosingCount
// FROM
// PostEngagement PE
// LEFT JOIN
// PostHistoryEngagement PHE ON PE.PostId = PHE.PostId
// ORDER BY
// PE.ViewCount DESC;
fn q10157(db: &'static So) -> String {
    let base = owned(db).with((&db.post.creation_date).ge(ts(2024, 9, 1, 12, 34, 56)));
    let j: MatSet<(Id<Post>, Option<Id<PostHistory>>)> = base.select(Ident::<Post>::new().and(history_of(db).opt())).collect();
    let post_of = (&j).map(|(p, _)| p);
    let hist_of = (&j).flat_map(|(_, h)| h);
    let phe = (&j)
        .group_by((&post_of).and((&hist_of).select((&db.post_history.post_history_type).select(&db.post_history_type.name)).opt()))
        .fold(0i64, |a, _| a + 1);
    let per_post = Same::<(Id<Post>, Option<Str>)>::new()
        .map(|(p, _)| p)
        .select(comments_per_post(db).and(votes_of_type(db, 2)).and(votes_of_type(db, 3)).and((&db.post.owner_user).select(badges_per_user(db))));
    let mut v = Vec::new();
    (&phe).and(per_post).drive(|(p, t), (n, x)| v.push((p, if t.is_none() { 0 } else { n }, x)));
    rows(v.iter().map(|&(p, n, (((c, u), d), b))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([V::I(c), V::I(u), V::I(d), V::I(b)]);
        f.extend(post_fields(db, p, &["rep", "owner"]));
        f.extend([V::I(n), V::I(n)]);
        row(f)
    }))
}

// WITH UserPostDetails AS (
// SELECT
// U.Id AS UserId,
// U.Reputation,
// P.Id AS PostId,
// P.Title,
// P.CreationDate AS PostCreationDate,
// P.ViewCount,
// P.Score,
// COALESCE(COUNT(C.Id), 0) AS CommentCount
// FROM
// Users U
// JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// GROUP BY
// U.Id, U.Reputation, P.Id, P.Title, P.CreationDate, P.ViewCount, P.Score
// ),
// UserBadgeCount AS (
// SELECT
// UserId,
// COUNT(*) AS BadgeCount
// FROM
// Badges
// GROUP BY
// UserId
// )
// SELECT
// UPD.UserId,
// UPD.Reputation,
// UPD.PostId,
// UPD.Title,
// UPD.PostCreationDate,
// UPD.ViewCount,
// UPD.Score,
// UPD.CommentCount,
// COALESCE(UBC.BadgeCount, 0) AS BadgeCount
// FROM
// UserPostDetails UPD
// LEFT JOIN
// UserBadgeCount UBC ON UPD.UserId = UBC.UserId
// ORDER BY
// UPD.Reputation DESC,
// UPD.ViewCount DESC;
fn q10161(db: &'static So) -> String {
    let mut v = Vec::new();
    owned(db)
        .select(Ident::<Post>::new().and(comments_per_post(db)).and((&db.post.owner_user).select(badges_per_user(db))))
        .drive(|_, x| v.push(x));
    rows(v.iter().map(|&((p, c), b)| {
        let mut f = post_fields(db, p, &["uid", "rep", "id", "title", "created", "views", "score"]);
        f.extend([V::I(c), V::I(b)]);
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// COALESCE(c.CommentCount, 0) AS CommentCount,
// COALESCE(v.VoteCount, 0) AS VoteCount,
// AVG(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS AvgUpVotes,
// AVG(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS AvgDownVotes
// FROM
// Posts p
// LEFT JOIN (
// SELECT
// PostId,
// COUNT(*) AS CommentCount
// FROM
// Comments
// GROUP BY
// PostId
// ) c ON p.Id = c.PostId
// LEFT JOIN (
// SELECT
// PostId,
// COUNT(*) AS VoteCount,
// VoteTypeId
// FROM
// Votes
// GROUP BY
// PostId, VoteTypeId
// ) v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, c.CommentCount, v.VoteCount
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q10177(db: &'static So) -> String {
    let cnt = db.vote.group_by((&db.vote.post).and(&db.vote.vote_type_id)).fold(0i64, |a, _| a + 1);
    let j: MatSet<(Id<Post>, Option<i64>)> = db.post.select(Ident::<Post>::new().and(votes_of(db).select(&db.vote.vote_type_id).opt())).collect();
    let post_of = (&j).map(|(p, _)| p);
    let vt_of = (&j).map(|(_, t)| t);
    let pv = (&j).flat_map(|(p, t)| t.map(|t| (p, t)));
    let f = (&j)
        .group_by((&post_of).and((&pv).select(&cnt).opt()))
        .select(&vt_of)
        .fold((0i64, 0i64, 0i64), |(n, u, d), t| (n + 1, u + (t == Some(2)) as i64, d + (t == Some(3)) as i64));
    let cc = Same::<(Id<Post>, Option<i64>)>::new().map(|(p, _)| p).select(comments_per_post(db));
    let mut v = Vec::new();
    (&f).and(cc).drive(|k, (a, c)| v.push((k, a, c)));
    out(v, |&((p, _), _, _)| newest(db, p), 100, |&((p, x), (n, u, d), c)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend([V::I(c), V::I(x.unwrap_or(0)), V::F(u as f64 / n as f64), V::F(d as f64 / n as f64)]);
        f
    })
}

// WITH PostDetails AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.Body,
// U.DisplayName AS OwnerDisplayName,
// P.CreationDate AS PostCreationDate,
// P.Score,
// P.ViewCount,
// COUNT(CASE WHEN C.PostId IS NOT NULL THEN 1 END) AS CommentCount,
// COUNT(PH.Id) AS HistoryCount
// FROM
// Posts P
// LEFT JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// PostHistory PH ON P.Id = PH.PostId
// GROUP BY
// P.Id, P.Title, P.Body, U.DisplayName,
// P.CreationDate, P.Score, P.ViewCount
// ),
// PostStatistics AS (
// SELECT
// PD.PostId,
// PD.Title,
// PD.OwnerDisplayName,
// PD.PostCreationDate,
// PD.Score,
// PD.ViewCount,
// PD.CommentCount,
// PD.HistoryCount,
// COUNT(V.Id) AS VoteCount
// FROM
// PostDetails PD
// LEFT JOIN
// Votes V ON PD.PostId = V.PostId
// GROUP BY
// PD.PostId, PD.Title, PD.OwnerDisplayName,
// PD.PostCreationDate, PD.Score, PD.ViewCount,
// PD.CommentCount, PD.HistoryCount
// )
// SELECT
// PS.PostId,
// PS.Title,
// PS.OwnerDisplayName,
// PS.PostCreationDate,
// PS.Score,
// PS.ViewCount,
// PS.CommentCount,
// PS.HistoryCount,
// PS.VoteCount
// FROM
// PostStatistics PS
// ORDER BY
// PS.Score DESC, PS.ViewCount DESC
// LIMIT 100;
fn q10181(db: &'static So) -> String {
    let mut v = Vec::new();
    stats_fold(db, db.post.iq(), Ident::<Post>::new(), "ch", &[]).and(votes_per_post(db)).drive(|p, (s, n)| v.push((p, s, n)));
    out(v, |&(p, _, _)| score_views(db, p), 100, |&(p, s, n)| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "score", "views"]);
        f.extend([V::I(s.cx), V::I(s.hx), V::I(n)]);
        f
    })
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.PostTypeId,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount
// FROM
// Posts p
// LEFT JOIN Comments c ON p.Id = c.PostId
// LEFT JOIN Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.PostTypeId, p.CreationDate, p.Score, p.ViewCount
// ),
// UserStats AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// COUNT(b.Id) AS BadgeCount,
// SUM(CASE WHEN v.UserId IS NOT NULL THEN 1 ELSE 0 END) AS VotesReceived
// FROM
// Users u
// LEFT JOIN Badges b ON u.Id = b.UserId
// LEFT JOIN Votes v ON u.Id = v.UserId
// GROUP BY
// u.Id, u.Reputation
// ),
// FinalStats AS (
// SELECT
// ps.PostId,
// ps.PostTypeId,
// ps.CreationDate,
// ps.Score,
// ps.ViewCount,
// ps.CommentCount,
// ps.VoteCount,
// us.Reputation AS UserReputation,
// us.BadgeCount
// FROM
// PostStats ps
// LEFT JOIN Users u ON ps.PostTypeId = u.Id
// LEFT JOIN UserStats us ON u.Id = us.UserId
// )
// SELECT
// PostId,
// PostTypeId,
// CreationDate,
// Score,
// ViewCount,
// CommentCount,
// VoteCount,
// UserReputation,
// BadgeCount
// FROM
// FinalStats
// ORDER BY
// Score DESC, ViewCount DESC;
fn q10191(db: &'static So) -> String {
    let uid = uids(db);
    let by_voter: HashIdx<Id<User>, Id<Vote>> = (&db.vote.user).inv().collect();
    let us = g(db).select(badges_of(db).opt().and((&by_voter).opt())).fold(0i64, |a, (b, _)| a + b.is_some() as i64);
    let mut v = Vec::new();
    stats_fold(db, db.post.iq(), Ident::<Post>::new(), "cv", &[])
        .and((&db.post.post_type_id).select(&uid).select(Ident::<User>::new().and(&us)).opt())
        .drive(|p, (s, u)| v.push((p, s, u)));
    rows(v.iter().map(|&(p, s, u)| {
        let mut f = post_fields(db, p, &["id", "type_id", "created", "score", "views"]);
        f.extend([V::I(s.cx), V::I(s.vx)]);
        match u {
            Some((u, b)) => f.extend([user_col(db, u, "rep"), V::I(b)]),
            None => f.extend([V::Null, V::Null]),
        }
        row(f)
    }))
}

// WITH PostMetrics AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// U.DisplayName AS OwnerDisplayName,
// P.CreationDate,
// P.Score,
// P.ViewCount,
// COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
// COUNT(A.Id) AS AnswerCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Posts P
// LEFT JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Posts A ON P.Id = A.ParentId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// WHERE
// P.PostTypeId = 1
// GROUP BY
// P.Id, P.Title, U.DisplayName, P.CreationDate, P.Score, P.ViewCount
// ),
// PostHistoryMetrics AS (
// SELECT
// PH.PostId,
// COUNT(PH.Id) AS EditCount,
// COUNT(CASE WHEN PH.PostHistoryTypeId = 10 THEN 1 END) AS CloseCount,
// COUNT(CASE WHEN PH.PostHistoryTypeId = 11 THEN 1 END) AS ReopenCount,
// COUNT(CASE WHEN PH.PostHistoryTypeId = 12 THEN 1 END) AS DeleteCount
// FROM
// PostHistory PH
// GROUP BY
// PH.PostId
// )
// SELECT
// PM.PostId,
// PM.Title,
// PM.OwnerDisplayName,
// PM.CreationDate,
// PM.Score,
// PM.ViewCount,
// PM.CommentCount,
// PM.AnswerCount,
// PM.UpVotes,
// PM.DownVotes,
// PHM.EditCount,
// PHM.CloseCount,
// PHM.ReopenCount,
// PHM.DeleteCount
// FROM
// PostMetrics PM
// LEFT JOIN
// PostHistoryMetrics PHM ON PM.PostId = PHM.PostId
// ORDER BY
// PM.ViewCount DESC
// LIMIT 100;
fn q10214(db: &'static So) -> String {
    let hf = db
        .post_history
        .group_by(&db.post_history.post)
        .select(&db.post_history.post_history_type_id)
        .fold([0i64; 4], |a, t| [a[0] + 1, a[1] + (t == 10) as i64, a[2] + (t == 11) as i64, a[3] + (t == 12) as i64]);
    let mut v = Vec::new();
    stats_fold(db, questions_only(db), Ident::<Post>::new(), "cva", &[]).and((&hf).opt()).drive(|p, (s, h)| v.push((p, s, h)));
    out(v, |&(p, _, _)| views_desc(db, p), 100, |&(p, s, h)| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "score", "views"]);
        f.extend([V::I(s.cx), V::I(s.ax), V::I(s.up), V::I(s.down)]);
        f.extend((0..4).map(|i| oint(h.map(|h| h[i]))));
        f
    })
}

// --- users ------------------------------------------------------------------

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserID,
// u.DisplayName,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(p.Score) AS TotalScore,
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
// ups.UserID,
// ups.DisplayName,
// ups.PostCount,
// ups.Questions,
// ups.Answers,
// ups.TotalScore,
// ups.TotalViews,
// ups.UpVotes,
// ups.DownVotes,
// u.Reputation,
// u.CreationDate,
// u.LastAccessDate
// FROM
// UserPostStats ups
// JOIN
// Users u ON ups.UserID = u.Id
// ORDER BY
// ups.PostCount DESC, ups.TotalScore DESC;
fn q13253(db: &'static So) -> String {
    users_rows(
        db,
        users_stats_with(db, UserWhere::All, "v", any_post, &[], &[]),
        |_, _, _| 0,
        0,
        &["uid", "name", "#n", "#q", "#a", "score_sum", "views_sum", "#up", "#down", "rep", "ucreated", "last_access"],
    )
}

// WITH UserMetrics AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM
// Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN Votes v ON p.Id = v.PostId AND v.UserId = u.Id
// LEFT JOIN Badges b ON u.Id = b.UserId
// GROUP BY
// u.Id, u.DisplayName
// )
// SELECT
// um.UserId,
// um.DisplayName,
// um.PostCount,
// um.Questions,
// um.Answers,
// um.UpVotes,
// um.DownVotes,
// um.GoldBadges,
// um.SilverBadges,
// um.BronzeBadges,
// (um.UpVotes - um.DownVotes) AS NetVotes
// FROM
// UserMetrics um
// ORDER BY
// um.PostCount DESC,
// NetVotes DESC
// LIMIT 100;
fn q13188(db: &'static So) -> String {
    let own_votes: HashIdx<(Id<User>, Id<Post>), Id<Vote>> = db.vote.select((&db.vote.user).and(&db.vote.post)).inv().collect();
    let to_post = Same::<(Id<User>, Id<Post>)>::new().map(|(_, p)| p);
    let joined = (&to_post).select(&db.post.post_type_id).and((&own_votes).select(&db.vote.vote_type_id).opt());
    let mut v = Vec::new();
    g(db)
        .select(Ident::<User>::new().and(posts_of(db)).select(joined).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 8], |mut a, (p, b)| {
            if let Some((t, vt)) = p {
                a[0] += 1;
                a[1] += (t == 1) as i64;
                a[2] += (t == 2) as i64;
                a[3] += (vt == Some(2)) as i64;
                a[4] += (vt == Some(3)) as i64;
            }
            if let Some(c) = b {
                if (1..4).contains(&c) {
                    a[4 + c as usize] += 1;
                }
            }
            a
        })
        .drive(|u, a| v.push((u, a)));
    out(v, |&(_, a)| (Reverse(a[0]), Reverse(a[3] - a[4])), 100, |&(u, a)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(a.iter().map(|&x| V::I(x)));
        f.push(V::I(a[3] - a[4]));
        f
    })
}

// WITH UserPosts AS (
// SELECT
// u.Id AS UserId,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN p.PostTypeId IN (4, 5) THEN 1 ELSE 0 END) AS TotalTagWikis,
// SUM(COALESCE(p.Score, 0)) AS TotalScore
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id
// ),
// UserVotes AS (
// SELECT
// v.UserId,
// COUNT(v.Id) AS TotalVotes,
// SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS TotalDownVotes
// FROM
// Votes v
// JOIN
// VoteTypes vt ON v.VoteTypeId = vt.Id
// GROUP BY
// v.UserId
// )
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// up.TotalPosts,
// up.TotalQuestions,
// up.TotalAnswers,
// up.TotalTagWikis,
// up.TotalScore,
// uv.TotalVotes,
// uv.TotalUpVotes,
// uv.TotalDownVotes
// FROM
// Users u
// LEFT JOIN
// UserPosts up ON u.Id = up.UserId
// LEFT JOIN
// UserVotes uv ON u.Id = uv.UserId
// ORDER BY
// up.TotalScore DESC, uv.TotalVotes DESC
// LIMIT 100;
fn q10021(db: &'static So) -> String {
    let up = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "", any_post);
    let uv = db
        .vote
        .group_by(&db.vote.user)
        .select((&db.vote.vote_type).select(&db.vote_type.name))
        .fold([0i64; 3], |a, n| [a[0] + 1, a[1] + (n == "UpMod") as i64, a[2] + (n == "DownMod") as i64]);
    let mut v = Vec::new();
    (&up).and((&uv).opt()).drive(|u, (s, x)| v.push((u, s, x)));
    out(v, |(_, s, x)| (Reverse(s.score_sum), x.is_none(), Reverse(x.map(|x| x[0]))), 100, |&(u, s, x)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(["#n", "#q", "#a", "#45", "score_sum0"].iter().map(|c| ustat_field(&s, c)));
        f.extend((0..3).map(|i| oint(x.map(|x| x[i]))));
        f
    })
}

// WITH UserVoteSummary AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// COUNT(DISTINCT P.Id) AS PostCount
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// U.Id, U.DisplayName
// ),
// TopUsers AS (
// SELECT
// *,
// (UpVotes - DownVotes) AS VoteBalance
// FROM
// UserVoteSummary
// ORDER BY
// VoteBalance DESC
// LIMIT 10
// )
// SELECT
// T.DisplayName,
// T.UpVotes,
// T.DownVotes,
// T.VoteBalance,
// P.CreationDate AS PostCreationDate,
// P.Title AS PostTitle,
// P.ViewCount,
// P.Score
// FROM
// TopUsers T
// JOIN
// Posts P ON T.UserId = P.OwnerUserId
// ORDER BY
// T.VoteBalance DESC, P.ViewCount DESC;
fn q10053(db: &'static So) -> String {
    let uf = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let balance = (&uf).map(|s| s.up - s.down);
    let top = whole(&balance)
        .select(Ident::<User>::new().and(&balance))
        .window(row_number, |(_, b)| b, desc)
        .filt(|(_, n)| n <= 10)
        .map(|((u, _), _)| u);
    let mut v = Vec::new();
    top.select(Ident::<User>::new().and(&uf).and(posts_of(db))).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((u, s), p)| {
        let mut f = vec![user_col(db, u, "name"), V::I(s.up), V::I(s.down), V::I(s.up - s.down)];
        f.extend(post_fields(db, p, &["created", "title", "views", "score"]));
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
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpvotes,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownvotes
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// u.Id, u.DisplayName
// ),
// PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpvotes,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownvotes,
// p.OwnerUserId
// FROM
// Posts p
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.OwnerUserId
// )
// SELECT
// ups.UserId,
// ups.DisplayName,
// ups.TotalPosts,
// ups.TotalQuestions,
// ups.TotalAnswers,
// ups.TotalUpvotes,
// ups.TotalDownvotes,
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.ViewCount,
// ps.Score,
// ps.TotalUpvotes AS PostUpvotes,
// ps.TotalDownvotes AS PostDownvotes
// FROM
// UserPostStats ups
// JOIN
// PostStats ps ON ups.UserId = ps.OwnerUserId
// ORDER BY
// ups.TotalPosts DESC, ps.Score DESC;
fn q10092(db: &'static So) -> String {
    let uf = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let pf = stats_fold(db, owned(db), Ident::<Post>::new(), "v", &[]);
    let mut v = Vec::new();
    (&pf).and((&db.post.owner_user).select(Ident::<User>::new().and(&uf))).drive(|p, (s, (u, us))| v.push((p, s, u, us)));
    rows(v.iter().map(|&(p, s, u, us)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(["#n", "#q", "#a", "#up", "#down"].iter().map(|c| ustat_field(&us, c)));
        f.extend(post_fields(db, p, &["id", "title", "created", "views", "score"]));
        f.extend([V::I(s.up), V::I(s.down)]);
        row(f)
    }))
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS PostCount,
// COALESCE(SUM(c.CommentCount), 0) AS TotalComments,
// COALESCE(SUM(v.VoteCount), 0) AS TotalVotes
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN (
// SELECT
// PostId,
// COUNT(*) AS CommentCount
// FROM Comments
// GROUP BY PostId
// ) c ON p.Id = c.PostId
// LEFT JOIN (
// SELECT
// PostId,
// COUNT(*) AS VoteCount
// FROM Votes
// GROUP BY PostId
// ) v ON p.Id = v.PostId
// GROUP BY u.Id, u.DisplayName
// )
// SELECT
// UserId,
// DisplayName,
// PostCount,
// TotalComments,
// TotalVotes
// FROM UserPostStats
// ORDER BY PostCount DESC, TotalVotes DESC
// LIMIT 100;
fn q10096(db: &'static So) -> String {
    let mut v = Vec::new();
    g(db)
        .select(posts_of(db).select(comments_per_post(db).and(votes_per_post(db))).opt())
        .fold([0i64; 3], |a, x| match x {
            Some((c, n)) => [a[0] + 1, a[1] + c, a[2] + n],
            None => a,
        })
        .drive(|u, a| v.push((u, a)));
    out(v, |&(_, a)| (Reverse(a[0]), Reverse(a[2])), 100, |&(u, a)| vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2])])
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN p.PostTypeId = 3 THEN 1 ELSE 0 END) AS TotalWikis,
// AVG(p.Score) AS AvgScore,
// SUM(p.ViewCount) AS TotalViews
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName
// ),
// UserBadgeStats AS (
// SELECT
// b.UserId,
// COUNT(b.Id) AS TotalBadges,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS TotalGoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS TotalSilverBadges,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS TotalBronzeBadges
// FROM
// Badges b
// GROUP BY
// b.UserId
// )
// SELECT
// u.DisplayName,
// ups.TotalPosts,
// ups.TotalQuestions,
// ups.TotalAnswers,
// ups.TotalWikis,
// ups.AvgScore,
// ups.TotalViews,
// ubs.TotalBadges,
// ubs.TotalGoldBadges,
// ubs.TotalSilverBadges,
// ubs.TotalBronzeBadges
// FROM
// UserPostStats ups
// JOIN
// Users u ON ups.UserId = u.Id
// LEFT JOIN
// UserBadgeStats ubs ON u.Id = ubs.UserId
// ORDER BY
// ups.TotalPosts DESC;
fn q10123(db: &'static So) -> String {
    let uf = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "", any_post);
    let bf = badge_classes(db);
    let mut v = Vec::new();
    (&uf).and((&bf).opt()).drive(|u, (s, b)| v.push((u, s, b)));
    rows(v.iter().map(|&(u, s, b)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(["#n", "#q", "#a", "#3", "score_avg", "views_sum"].iter().map(|c| ustat_field(&s, c)));
        f.extend((0..4).map(|i| oint(b.map(|b| b[i]))));
        row(f)
    }))
}

// WITH UserPostStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(DISTINCT P.Id) AS PostsCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersCount,
// COUNT(DISTINCT C.Id) AS CommentsCount,
// COUNT(DISTINCT B.Id) AS BadgesCount,
// SUM(V.BountyAmount) AS TotalBountyAmount
// FROM Users U
// LEFT JOIN Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN Comments C ON P.Id = C.PostId
// LEFT JOIN Badges B ON U.Id = B.UserId
// LEFT JOIN Votes V ON P.Id = V.PostId AND V.UserId = U.Id
// GROUP BY U.Id, U.DisplayName
// )
// SELECT
// UserId,
// DisplayName,
// PostsCount,
// QuestionsCount,
// AnswersCount,
// CommentsCount,
// BadgesCount,
// TotalBountyAmount
// FROM UserPostStats
// ORDER BY PostsCount DESC
// LIMIT 100;
fn q10129(db: &'static So) -> String {
    let own_votes: HashIdx<(Id<User>, Id<Post>), Id<Vote>> = db.vote.select((&db.vote.user).and(&db.vote.post)).inv().collect();
    let to_post = Same::<(Id<User>, Id<Post>)>::new().map(|(_, p)| p);
    let joined = (&to_post)
        .select((&db.post.post_type_id).and(comments_of(db).opt()))
        .and((&own_votes).select((&db.vote.bounty_amount).opt()).opt());
    let f = g(db).select(Ident::<User>::new().and(posts_of(db)).select(joined).opt().and(badges_of(db).opt())).fold([0i64; 4], |a, (p, _)| match p {
        Some(((t, _), b)) => {
            let b = b.flatten();
            [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + b.is_some() as i64, a[3] + b.unwrap_or(0)]
        }
        None => a,
    });
    let p = ud(db, UserWhere::All, posts_of(db));
    let c = ud(db, UserWhere::All, posts_of(db).select(comments_of(db)));
    let b = ud(db, UserWhere::All, badges_of(db));
    let o = |x: Option<i64>| x.unwrap_or(0);
    let mut v = Vec::new();
    (&f).and((&p).opt()).and((&c).opt()).and((&b).opt()).drive(|u, (((a, p), c), b)| v.push((u, a, o(p), o(c), o(b))));
    out(v, |x| Reverse(x.2), 100, |&(u, a, p, c, b)| {
        vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(p), V::I(a[0]), V::I(a[1]), V::I(c), V::I(b), nullable(a[3], a[2])]
    })
}

// WITH UserActivity AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// COUNT(c.Id) AS TotalComments,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes,
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
// u.Id, u.DisplayName
// )
// SELECT
// ua.UserId,
// ua.DisplayName,
// ua.TotalPosts,
// ua.TotalQuestions,
// ua.TotalAnswers,
// ua.TotalComments,
// ua.TotalUpVotes,
// ua.TotalDownVotes,
// ua.TotalBadges,
// u.Reputation,
// u.CreationDate
// FROM
// UserActivity ua
// JOIN
// Users u ON ua.UserId = u.Id
// ORDER BY
// ua.TotalPosts DESC,
// u.Reputation DESC
// LIMIT 100;
fn q10179(db: &'static So) -> String {
    let p = ud(db, UserWhere::All, posts_of(db));
    users_rows(
        db,
        users_stats_with(db, UserWhere::All, "cvb", any_post, &[], &[&p]),
        |u, _, d| (Reverse(d[0]), rep_desc(db, u)),
        100,
        &["uid", "name", "#d0", "#q", "#a", "#cx", "#up", "#down", "#bx", "rep", "ucreated"],
    )
}

// WITH PostCounts AS (
// SELECT
// OwnerUserId,
// COUNT(*) AS PostCount,
// SUM(ViewCount) AS TotalViewCount,
// SUM(Score) AS TotalScore
// FROM Posts
// GROUP BY OwnerUserId
// ),
// UserStats AS (
// SELECT
// U.Id,
// U.DisplayName,
// U.Reputation,
// U.CreationDate,
// U.LastAccessDate,
// COALESCE(PC.PostCount, 0) AS PostCount,
// COALESCE(PC.TotalViewCount, 0) AS TotalViewCount,
// COALESCE(PC.TotalScore, 0) AS TotalScore
// FROM Users U
// LEFT JOIN PostCounts PC ON U.Id = PC.OwnerUserId
// )
// SELECT
// Id,
// DisplayName,
// Reputation,
// CreationDate,
// LastAccessDate,
// PostCount,
// TotalViewCount,
// TotalScore
// FROM UserStats
// ORDER BY Reputation DESC
// LIMIT 100;
fn q10207(db: &'static So) -> String {
    let Post { view_count, score, .. } = &db.post;
    let mut v = Vec::new();
    g(db)
        .select(posts_of(db).select(view_count.opt().and(score)).opt())
        .fold([0i64; 3], |a, p| match p {
            Some((w, s)) => [a[0] + 1, a[1] + w.unwrap_or(0), a[2] + s],
            None => a,
        })
        .drive(|u, a| v.push((u, a)));
    out(v, |&(u, _)| rep_desc(db, u), 100, |&(u, a)| {
        let mut f: Vec<V> = ["uid", "name", "rep", "ucreated", "last_access"].iter().map(|c| user_col(db, u, c)).collect();
        f.extend(a.iter().map(|&x| V::I(x)));
        f
    })
}

// WITH UserPostStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS PostCount,
// AVG(p.Score) AS AvgPostScore,
// SUM(CASE WHEN c.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalComments
// FROM Posts p
// LEFT JOIN Comments c ON p.Id = c.PostId
// GROUP BY p.OwnerUserId
// ),
// UserReputation AS (
// SELECT
// u.Id AS UserId,
// u.Reputation
// FROM Users u
// )
// SELECT
// ups.OwnerUserId,
// ups.PostCount,
// ups.AvgPostScore,
// ups.TotalComments,
// ur.Reputation
// FROM UserPostStats ups
// JOIN UserReputation ur ON ups.OwnerUserId = ur.UserId
// ORDER BY ups.PostCount DESC, ur.Reputation DESC;
fn q10212(db: &'static So) -> String {
    let mut v = Vec::new();
    owned(db)
        .group_by(&db.post.owner_user)
        .select((&db.post.score).and(comments_of(db).opt()))
        .fold((0i64, 0i64, 0i64), |(n, s, c), (sc, ci)| (n + 1, s + sc, c + ci.is_some() as i64))
        .drive(|u, a| v.push((u, a)));
    rows(v.iter().map(|&(u, (n, s, c))| row(vec![user_col(db, u, "uid"), V::I(n), avg(s, n), V::I(c), user_col(db, u, "rep")])))
}

// WITH UserReputation AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// SUM(B.Class) AS TotalBadges,
// SUM(V.BountyAmount) AS TotalBounty
// FROM Users U
// LEFT JOIN Badges B ON U.Id = B.UserId
// LEFT JOIN Votes V ON U.Id = V.UserId
// GROUP BY U.Id, U.DisplayName
// ),
// PostStats AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS TotalPosts,
// COUNT(C.Id) AS TotalComments,
// SUM(P.Score) AS TotalScore,
// SUM(P.ViewCount) AS TotalViews,
// AVG(P.AnswerCount) AS AvgAnswersPerPost
// FROM Posts P
// LEFT JOIN Comments C ON P.Id = C.PostId
// GROUP BY P.OwnerUserId
// )
// SELECT
// U.DisplayName AS User,
// U.Reputation AS Reputation,
// COALESCE(UR.TotalBadges, 0) AS TotalBadges,
// COALESCE(UR.TotalBounty, 0) AS TotalBounty,
// COALESCE(PS.TotalPosts, 0) AS TotalPosts,
// COALESCE(PS.TotalComments, 0) AS TotalComments,
// COALESCE(PS.TotalScore, 0) AS TotalScore,
// COALESCE(PS.TotalViews, 0) AS TotalViews,
// COALESCE(PS.AvgAnswersPerPost, 0) AS AvgAnswersPerPost
// FROM Users U
// LEFT JOIN UserReputation UR ON U.Id = UR.UserId
// LEFT JOIN PostStats PS ON U.Id = PS.OwnerUserId
// ORDER BY U.Reputation DESC;
fn q10213(db: &'static So) -> String {
    let Post { score, view_count, answer_count, owner_user, .. } = &db.post;
    let by_voter: HashIdx<Id<User>, Id<Vote>> = (&db.vote.user).inv().collect();
    let ur = g(db)
        .select(badges_of(db).select(&db.badge.class).opt().and((&by_voter).select((&db.vote.bounty_amount).opt()).opt()))
        .fold((0i64, 0i64), |(bs, vs), (b, x)| (bs + b.unwrap_or(0), vs + x.flatten().unwrap_or(0)));
    let ps = owned(db)
        .group_by(owner_user)
        .select(score.and(view_count.opt()).and(answer_count.opt()).and(comments_of(db).opt()))
        .fold([0i64; 6], |a, (((s, w), an), c)| {
            [a[0] + 1, a[1] + c.is_some() as i64, a[2] + s, a[3] + w.unwrap_or(0), a[4] + an.is_some() as i64, a[5] + an.unwrap_or(0)]
        });
    let mut v = Vec::new();
    (&ur).and((&ps).opt()).drive(|u, (r, p)| v.push((u, r, p.unwrap_or([0; 6]))));
    rows(v.iter().map(|&(u, (bs, vs), a)| {
        row(vec![
            user_col(db, u, "name"),
            user_col(db, u, "rep"),
            V::I(bs),
            V::I(vs),
            V::I(a[0]),
            V::I(a[1]),
            V::I(a[2]),
            V::I(a[3]),
            if a[4] == 0 { V::F(0.0) } else { avg(a[5], a[4]) },
        ])
    }))
}

// WITH UserPostCounts AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(P.Id) AS PostCount
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id, U.DisplayName
// ),
// TopUsers AS (
// SELECT
// UserId,
// DisplayName,
// PostCount
// FROM
// UserPostCounts
// ORDER BY
// PostCount DESC
// LIMIT 10
// ),
// PostStats AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.ViewCount,
// P.Score,
// U.DisplayName AS OwnerDisplayName,
// (SELECT COUNT(*) FROM Comments C WHERE C.PostId = P.Id) AS CommentCount,
// P.OwnerUserId
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// WHERE
// P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 month'
// )
// SELECT
// U.DisplayName AS TopUserDisplayName,
// T.Title,
// T.CreationDate,
// T.ViewCount,
// T.Score,
// T.CommentCount
// FROM
// TopUsers U
// JOIN
// PostStats T ON U.UserId = T.OwnerUserId
// ORDER BY
// U.PostCount DESC, T.Score DESC;
fn q10218(db: &'static So) -> String {
    let pc = g(db).select(posts_of(db).opt()).fold(0i64, |a, p| a + p.is_some() as i64);
    let top = whole(&pc)
        .select(Ident::<User>::new().and(&pc))
        .window(row_number, |(_, n)| n, desc)
        .filt(|(_, r)| r <= 10)
        .map(|((u, _), _)| u);
    let recent = posts_of(db).select(Ident::<Post>::new().with((&db.post.creation_date).ge(month_ago())).and(comments_per_post(db)));
    let mut v = Vec::new();
    top.select(Ident::<User>::new().and(recent)).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(u, (p, c))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title", "created", "views", "score"]));
        f.push(V::I(c));
        row(f)
    }))
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(DISTINCT P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(CASE WHEN P.PostTypeId = 3 THEN 1 ELSE 0 END) AS Wikis,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes,
// SUM(COALESCE(P.Score, 0)) AS TotalScore
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// U.Id, U.DisplayName, U.Reputation
// )
// SELECT
// U.UserId,
// U.DisplayName,
// U.Reputation,
// U.TotalPosts,
// U.Questions,
// U.Answers,
// U.Wikis,
// U.Upvotes,
// U.Downvotes,
// U.TotalScore,
// (U.Upvotes - U.Downvotes) AS NetVotes
// FROM
// UserStats U
// ORDER BY
// U.TotalScore DESC;
fn q10221(db: &'static So) -> String {
    let p = ud(db, UserWhere::All, posts_of(db));
    users_rows(
        db,
        users_stats_with(db, UserWhere::All, "v", any_post, &[], &[&p]),
        |_, _, _| 0,
        0,
        &["uid", "name", "rep", "#d0", "#q", "#a", "#3", "#up", "#down", "score_sum0", "#net"],
    )
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// COALESCE(AVG(v.VoteCount), 0) AS AvgVotes,
// COALESCE(AVG(p.ViewCount), 0) AS AvgViewCount
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
// )
// SELECT
// ups.UserId,
// ups.DisplayName,
// ups.TotalPosts,
// ups.AvgVotes,
// ups.AvgViewCount,
// u.Reputation
// FROM
// UserPostStats ups
// JOIN
// Users u ON ups.UserId = u.Id
// ORDER BY
// u.Reputation DESC
// LIMIT 10;
fn q10227(db: &'static So) -> String {
    let vc = db.vote.group_by(&db.vote.post).fold(0i64, |a, _| a + 1);
    let mut v = Vec::new();
    g(db)
        .select(posts_of(db).select((&db.post.view_count).opt().and((&vc).opt())).opt())
        .fold([0i64; 5], |a, p| match p {
            Some((w, c)) => [a[0] + 1, a[1] + c.is_some() as i64, a[2] + c.unwrap_or(0), a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0)],
            None => a,
        })
        .drive(|u, a| v.push((u, a)));
    let or0 = |s: i64, n: i64| if n == 0 { V::F(0.0) } else { avg(s, n) };
    out(v, |&(u, _)| rep_desc(db, u), 10, |&(u, a)| {
        vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(a[0]), or0(a[2], a[1]), or0(a[4], a[3]), user_col(db, u, "rep")]
    })
}

// --- post types -------------------------------------------------------------

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.PostTypeId,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
// AVG(EXTRACT(EPOCH FROM (p.LastActivityDate - p.CreationDate))) AS AvgActiveDurationInSeconds
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.PostTypeId
// )
// SELECT
// pt.Name AS PostType,
// COUNT(ps.PostId) AS TotalPosts,
// SUM(ps.CommentCount) AS TotalComments,
// SUM(ps.VoteCount) AS TotalVotes,
// SUM(ps.UpVoteCount) AS TotalUpVotes,
// SUM(ps.DownVoteCount) AS TotalDownVotes,
// AVG(ps.AvgActiveDurationInSeconds) AS AvgActiveDuration
// FROM
// PostStats ps
// JOIN
// PostTypes pt ON ps.PostTypeId = pt.Id
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q10185(db: &'static So) -> String {
    let Post { creation_date, last_activity_date, .. } = &db.post;
    let s = stats_fold(db, db.post.iq(), Ident::<Post>::new(), "cv", &[]);
    let f = by_key(db.post.iq(), name(db), (&s).and(creation_date).and(last_activity_date), (0i64, [0i64; 4], 0i128), |(n, a, d), ((s, cd), la)| {
        (n + 1, [a[0] + s.cx, a[1] + s.vx, a[2] + s.up, a[3] + s.down], d + (la - cd) as i128)
    });
    let mut v = Vec::new();
    (&f).drive(|k, a| v.push((k, a)));
    out(v, |x| Reverse(x.1.0), 0, |&(k, (n, a, d))| {
        let mut f = vec![V::S(k), V::I(n)];
        f.extend(a.iter().map(|&x| V::I(x)));
        f.push(V::F(d as f64 / 1e6 / n as f64));
        f
    })
}

// WITH PostStats AS (
// SELECT
// pt.Name AS PostTypeName,
// COUNT(p.Id) AS PostCount,
// AVG(p.Score) AS AvgScore,
// SUM(COALESCE(c.CommentCount, 0)) AS TotalComments,
// SUM(COALESCE(v.VoteCount, 0)) AS TotalVotes
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN (
// SELECT PostId, COUNT(*) AS CommentCount
// FROM Comments
// GROUP BY PostId
// ) c ON p.Id = c.PostId
// LEFT JOIN (
// SELECT
// PostId,
// COUNT(*) AS VoteCount
// FROM Votes
// GROUP BY PostId
// ) v ON p.Id = v.PostId
// WHERE
// p.CreationDate BETWEEN '2023-01-01' AND '2023-12-31'
// GROUP BY
// pt.Name
// )
// SELECT
// PostTypeName,
// PostCount,
// AvgScore,
// TotalComments,
// TotalVotes
// FROM
// PostStats
// ORDER BY
// PostCount DESC;
fn q10187(db: &'static So) -> String {
    let base = db.post.with((&db.post.creation_date).filt(|d| d >= date(2023, 1, 1) && d <= date(2023, 12, 31)));
    let f = by_key(base, name(db), (&db.post.score).and(comments_per_post(db)).and(votes_per_post(db)), [0i64; 4], |a, ((s, c), x)| {
        [a[0] + 1, a[1] + s, a[2] + c, a[3] + x]
    });
    let mut v = Vec::new();
    (&f).drive(|k, a| v.push((k, a)));
    out(v, |x| Reverse(x.1[0]), 0, |&(k, a)| vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), V::I(a[2]), V::I(a[3])])
}

// PostStatistics joins one row per post and only its ViewCount and Score,
// the post's own, are read, so the join is the post itself.
// WITH PostStatistics AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// MAX(b.Class) AS HighestBadgeClass,
// AVG(u.Reputation) AS AverageUserReputation
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
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score
// ),
// PostTypeCounts AS (
// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS PostCount,
// SUM(ps.ViewCount) AS TotalViews,
// AVG(ps.Score) AS AverageScore
// FROM
// PostTypes pt
// LEFT JOIN
// Posts p ON pt.Id = p.PostTypeId
// LEFT JOIN
// PostStatistics ps ON ps.PostId = p.Id
// GROUP BY
// pt.Name
// )
// SELECT
// pt.PostType,
// pt.PostCount,
// pt.TotalViews,
// pt.AverageScore
// FROM
// PostTypeCounts pt
// ORDER BY
// pt.PostCount DESC;
fn q10202(db: &'static So) -> String {
    let Post { view_count, score, .. } = &db.post;
    let of_type: HashIdx<Id<PostType>, Id<Post>> = (&db.post.post_type).inv().collect();
    let mut v = Vec::new();
    db.post_type
        .group_by(&db.post_type.name)
        .select((&of_type).select(view_count.opt().and(score)).opt())
        .fold([0i64; 4], |a, p| match p {
            Some((w, s)) => [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s],
            None => a,
        })
        .drive(|k, a| v.push((k, a)));
    out(v, |x| Reverse(x.1[0]), 0, |&(k, a)| vec![V::S(k), V::I(a[0]), nullable(a[2], a[1]), avg(a[3], a[0])])
}

// --- one-row totals -----------------------------------------------------------

// WITH UserStats AS (
// SELECT
// COUNT(*) AS TotalUsers,
// AVG(Reputation) AS AverageReputation,
// MAX(Reputation) AS MaxReputation,
// MIN(Reputation) AS MinReputation,
// COUNT(CASE WHEN LastAccessDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 YEAR' THEN 1 END) AS ActiveUsers
// FROM
// Users
// ),
// PostStats AS (
// SELECT
// COUNT(*) AS TotalPosts,
// COUNT(CASE WHEN PostTypeId = 1 THEN 1 END) AS TotalQuestions,
// COUNT(CASE WHEN PostTypeId = 2 THEN 1 END) AS TotalAnswers,
// AVG(Score) AS AveragePostScore,
// MAX(ViewCount) AS MaxViews,
// MIN(ViewCount) AS MinViews
// FROM
// Posts
// ),
// BadgeStats AS (
// SELECT
// COUNT(*) AS TotalBadges,
// COUNT(DISTINCT UserId) AS UsersWithBadges,
// COUNT(CASE WHEN Class = 1 THEN 1 END) AS GoldBadges,
// COUNT(CASE WHEN Class = 2 THEN 1 END) AS SilverBadges,
// COUNT(CASE WHEN Class = 3 THEN 1 END) AS BronzeBadges
// FROM
// Badges
// )
// SELECT
// u.TotalUsers,
// u.AverageReputation,
// u.MaxReputation,
// u.MinReputation,
// u.ActiveUsers,
// p.TotalPosts,
// p.TotalQuestions,
// p.TotalAnswers,
// p.AveragePostScore,
// p.MaxViews,
// p.MinViews,
// b.TotalBadges,
// b.UsersWithBadges,
// b.GoldBadges,
// b.SilverBadges,
// b.BronzeBadges
// FROM
// UserStats u, PostStats p, BadgeStats b;
fn q10074(db: &'static So) -> String {
    let User { reputation, last_access_date, .. } = &db.user;
    let (un, rs, rmax, rmin, act) = db
        .user
        .select(reputation.and(last_access_date))
        .fold_flat((0i64, 0i64, i64::MIN, i64::MAX, 0i64), |(n, s, mx, mn, a), (r, la)| (n + 1, s + r, mx.max(r), mn.min(r), a + (la > year_ago()) as i64));
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let (pn, q, a, ss, vn, vmax, vmin) = db.post.select(post_type_id.and(score).and(view_count.opt())).fold_flat(
        (0i64, 0i64, 0i64, 0i64, 0i64, i64::MIN, i64::MAX),
        |(n, q, a, ss, vn, vmax, vmin), ((t, s), w)| {
            (n + 1, q + (t == 1) as i64, a + (t == 2) as i64, ss + s, vn + w.is_some() as i64, w.map_or(vmax, |w| vmax.max(w)), w.map_or(vmin, |w| vmin.min(w)))
        },
    );
    let b = db.badge.select(&db.badge.class).fold_flat([0i64; 4], |mut a, c| {
        a[0] += 1;
        if (1..4).contains(&c) {
            a[c as usize] += 1;
        }
        a
    });
    let holders = one(whole(db.badge.iq()).select(&db.badge.user_id).count_distinct());
    row(vec![
        V::I(un),
        avg(rs, un),
        V::I(rmax),
        V::I(rmin),
        V::I(act),
        V::I(pn),
        V::I(q),
        V::I(a),
        avg(ss, pn),
        omax(vmax, vn),
        omax(vmin, vn),
        V::I(b[0]),
        V::I(holders),
        V::I(b[1]),
        V::I(b[2]),
        V::I(b[3]),
    ])
}

// WITH PostCounts AS (
// SELECT
// COUNT(*) AS TotalPosts,
// COUNT(DISTINCT OwnerUserId) AS TotalUsers,
// SUM(Score) AS TotalPostScore,
// AVG(Score) AS AveragePostScore
// FROM
// Posts
// ),
// VoteCounts AS (
// SELECT
// COUNT(*) AS TotalVotes,
// SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
// FROM
// Votes
// )
// SELECT
// p.TotalPosts,
// p.TotalUsers,
// p.TotalPostScore,
// p.AveragePostScore,
// v.TotalVotes,
// v.TotalUpVotes,
// v.TotalDownVotes
// FROM
// PostCounts p,
// VoteCounts v;
fn q10083(db: &'static So) -> String {
    let (pn, ss) = db.post.select(&db.post.score).fold_flat((0i64, 0i64), |(n, s), x| (n + 1, s + x));
    let owners = one(whole(db.post.iq()).select(&db.post.owner_user_id).count_distinct());
    let (vn, u, d) = db.vote.select(&db.vote.vote_type_id).fold_flat((0i64, 0i64, 0i64), |(n, u, d), t| (n + 1, u + (t == 2) as i64, d + (t == 3) as i64));
    row(vec![V::I(pn), V::I(owners), V::I(ss), avg(ss, pn), V::I(vn), V::I(u), V::I(d)])
}

// WITH PostStats AS (
// SELECT
// COUNT(*) AS TotalPosts,
// COUNT(DISTINCT OwnerUserId) AS TotalUsers,
// AVG(ViewCount) AS AvgViewCount,
// AVG(Score) AS AvgScore
// FROM Posts
// ),
// UserStats AS (
// SELECT
// COUNT(*) AS TotalUsers,
// AVG(Reputation) AS AvgReputation,
// AVG(UpVotes) AS AvgUpVotes,
// AVG(DownVotes) AS AvgDownVotes
// FROM Users
// ),
// VoteStats AS (
// SELECT
// COUNT(*) AS TotalVotes,
// COUNT(DISTINCT PostId) AS TotalPostVotes,
// AVG(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS AvgUpVotes,
// AVG(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS AvgDownVotes
// FROM Votes
// )
// SELECT
// p.TotalPosts,
// p.TotalUsers AS UniquePostOwners,
// p.AvgViewCount,
// p.AvgScore,
// u.TotalUsers AS UserCount,
// u.AvgReputation,
// u.AvgUpVotes AS UserAvgUpVotes,
// u.AvgDownVotes AS UserAvgDownVotes,
// v.TotalVotes,
// v.TotalPostVotes,
// v.AvgUpVotes AS VoteAvgUpVotes,
// v.AvgDownVotes AS VoteAvgDownVotes
// FROM PostStats p
// JOIN UserStats u ON 1=1
// JOIN VoteStats v ON 1=1;
fn q10118(db: &'static So) -> String {
    let Post { score, view_count, .. } = &db.post;
    let (pn, ss, vn, vs) = db.post.select(score.and(view_count.opt())).fold_flat((0i64, 0i64, 0i64, 0i64), |(n, ss, vn, vs), (s, w)| {
        (n + 1, ss + s, vn + w.is_some() as i64, vs + w.unwrap_or(0))
    });
    let owners = one(whole(db.post.iq()).select(&db.post.owner_user_id).count_distinct());
    let User { reputation, up_votes, down_votes, .. } = &db.user;
    let (un, rs, us, ds) = db.user.select(reputation.and(up_votes).and(down_votes)).fold_flat((0i64, 0i64, 0i64, 0i64), |(n, r, u, d), ((x, y), z)| {
        (n + 1, r + x, u + y, d + z)
    });
    let (xn, xu, xd) = db.vote.select(&db.vote.vote_type_id).fold_flat((0i64, 0i64, 0i64), |(n, u, d), t| (n + 1, u + (t == 2) as i64, d + (t == 3) as i64));
    let voted = one(whole(db.vote.iq()).select(&db.vote.post_id).count_distinct());
    row(vec![
        V::I(pn),
        V::I(owners),
        avg(vs, vn),
        avg(ss, pn),
        V::I(un),
        avg(rs, un),
        avg(us, un),
        avg(ds, un),
        V::I(xn),
        V::I(voted),
        avg(xu, xn),
        avg(xd, xn),
    ])
}

// WITH PostStats AS (
// SELECT
// COUNT(*) AS TotalPosts,
// COUNT(DISTINCT OwnerUserId) AS UniquePostOwners,
// AVG(ViewCount) AS AverageViewCount
// FROM Posts
// ),
// UserStats AS (
// SELECT
// COUNT(*) AS TotalUsers,
// AVG(Reputation) AS AverageReputation
// FROM Users
// ),
// CommentStats AS (
// SELECT
// COUNT(*) AS TotalComments,
// AVG(Score) AS AverageCommentScore
// FROM Comments
// ),
// VoteStats AS (
// SELECT
// COUNT(*) AS TotalVotes,
// AVG(BountyAmount) AS AverageBountyAmount
// FROM Votes
// )
// SELECT
// (SELECT TotalPosts FROM PostStats) AS TotalPosts,
// (SELECT UniquePostOwners FROM PostStats) AS UniquePostOwners,
// (SELECT AverageViewCount FROM PostStats) AS AverageViewCount,
// (SELECT TotalUsers FROM UserStats) AS TotalUsers,
// (SELECT AverageReputation FROM UserStats) AS AverageReputation,
// (SELECT TotalComments FROM CommentStats) AS TotalComments,
// (SELECT AverageCommentScore FROM CommentStats) AS AverageCommentScore,
// (SELECT TotalVotes FROM VoteStats) AS TotalVotes,
// (SELECT AverageBountyAmount FROM VoteStats) AS AverageBountyAmount;
fn q10141(db: &'static So) -> String {
    let (pn, vn, vs) = db.post.select((&db.post.view_count).opt()).fold_flat((0i64, 0i64, 0i64), |(n, vn, vs), w| (n + 1, vn + w.is_some() as i64, vs + w.unwrap_or(0)));
    let owners = one(whole(db.post.iq()).select(&db.post.owner_user_id).count_distinct());
    let (un, rs) = db.user.select(&db.user.reputation).fold_flat((0i64, 0i64), |(n, s), r| (n + 1, s + r));
    let (cn, cs) = db.comment.select(&db.comment.score).fold_flat((0i64, 0i64), |(n, s), x| (n + 1, s + x));
    let (xn, bn, bs) = db.vote.select((&db.vote.bounty_amount).opt()).fold_flat((0i64, 0i64, 0i64), |(n, bn, bs), b| (n + 1, bn + b.is_some() as i64, bs + b.unwrap_or(0)));
    row(vec![V::I(pn), V::I(owners), avg(vs, vn), V::I(un), avg(rs, un), V::I(cn), avg(cs, cn), V::I(xn), avg(bs, bn)])
}

pub static ENTRIES: &[harness::Entry] = &[
    ("10414", q10414),
    ("12587", q12587),
    ("14604", q14604),
    ("12581", q12581),
    ("12447", q12447),
    ("12402", q12402),
    ("12464", q12464),
    ("12184", q12184),
    ("14927", q14927),
    ("11962", q11962),
    ("14394", q14394),
    ("12030", q12030),
    ("10017", q10017),
    ("10031", q10031),
    ("10060", q10060),
    ("10061", q10061),
    ("10063", q10063),
    ("10064", q10064),
    ("10068", q10068),
    ("10111", q10111),
    ("10115", q10115),
    ("10116", q10116),
    ("10107", q10107),
    ("10108", q10108),
    ("10140", q10140),
    ("10157", q10157),
    ("10161", q10161),
    ("10177", q10177),
    ("10181", q10181),
    ("10191", q10191),
    ("10214", q10214),
    ("13253", q13253),
    ("13188", q13188),
    ("10021", q10021),
    ("10053", q10053),
    ("10092", q10092),
    ("10096", q10096),
    ("10123", q10123),
    ("10129", q10129),
    ("10179", q10179),
    ("10207", q10207),
    ("10212", q10212),
    ("10213", q10213),
    ("10218", q10218),
    ("10221", q10221),
    ("10227", q10227),
    ("10185", q10185),
    ("10187", q10187),
    ("10202", q10202),
    ("10074", q10074),
    ("10083", q10083),
    ("10118", q10118),
    ("10141", q10141),
];
