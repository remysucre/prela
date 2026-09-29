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
    if n < v.len() && key(&v[n - 1]) == key(&v[n]) {
        eprintln!("tie at the LIMIT cut");
    }
    rows(v.iter().take(n).map(|x| row(f(x))))
}


fn nulls(n: usize) -> Vec<V> {
    (0..n).map(|_| V::Null).collect()
}

fn pids(db: &'static So) -> HashIdx<i64, Id<Post>> {
    (&db.post.origid).inv().collect()
}

fn or0(s: i64, n: i64) -> V {
    if n == 0 { V::F(0.0) } else { avg(s, n) }
}

fn round2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

fn by_voter(db: &'static So) -> HashIdx<Id<User>, Id<Vote>> {
    (&db.vote.user).inv().collect()
}

fn by_name(db: &'static So) -> HashIdx<Str, Id<User>> {
    (&db.user.display_name).inv().collect()
}

fn history_types(db: &'static So) -> Fold<Id<Post>, [i64; 4]> {
    db.post_history
        .group_by(&db.post_history.post)
        .select(&db.post_history.post_history_type_id)
        .fold([0i64; 4], |a, t| [a[0] + 1, a[1] + (t == 10) as i64, a[2] + (t == 11) as i64, a[3] + (t == 12) as i64])
}

fn vote_named(db: &'static So) -> Fold<Id<User>, [i64; 3]> {
    db.vote
        .group_by(&db.vote.user)
        .select((&db.vote.vote_type).select(&db.vote_type.name))
        .fold([0i64; 3], |a, n| [a[0] + 1, a[1] + (n == "UpMod") as i64, a[2] + (n == "DownMod") as i64])
}

fn upqa(db: &'static So) -> Fold<Id<User>, [i64; 6]> {
    let Post { post_type_id, view_count, score, .. } = &db.post;
    g(db).select(posts_of(db).select(post_type_id.and(view_count.opt()).and(score)).opt()).fold([0i64; 6], |a, p| match p {
        Some(((t, w), s)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0), a[5] + s],
        None => a,
    })
}

fn user_votes(db: &'static So) -> Fold<Id<User>, [i64; 3]> {
    g(db).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64],
        None => a,
    })
}

fn fkey(x: f64) -> i64 {
    let b = x.to_bits() as i64;
    b ^ (((b >> 63) as u64) >> 1) as i64
}

fn ints(a: &[i64]) -> Vec<V> {
    a.iter().map(|&x| V::I(x)).collect()
}

fn owner_posts(db: &'static So) -> Fold<Id<User>, [i64; 6]> {
    let Post { post_type_id, view_count, score, .. } = &db.post;
    owned(db)
        .group_by(&db.post.owner_user)
        .select(post_type_id.and(view_count.opt()).and(score))
        .fold([0i64; 6], |a, ((t, w), s)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0), a[5] + s])
}

fn post_totals(db: &'static So) -> [i64; 4] {
    let Post { score, view_count, .. } = &db.post;
    db.post.select(score.and(view_count.opt())).fold_flat([0i64; 4], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)])
}


fn uqacx(db: &'static So) -> Fold<Id<User>, [i64; 5]> {
    g(db).select(posts_of(db).select((&db.post.post_type_id).and(comments_per_post(db)).and(votes_per_post(db))).opt()).fold([0i64; 5], |a, p| match p {
        Some(((t, c), x)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + c, a[4] + x],
        None => a,
    })
}

fn post_votes(db: &'static So) -> Fold<Id<Post>, [i64; 3]> {
    db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64])
}

fn ratio(n: i64, d: i64) -> V {
    if d == 0 { V::Null } else { V::F(n as f64 / d as f64) }
}

fn ratio32(n: i64, d: i64) -> V {
    if d == 0 { V::Null } else { V::F((n as f32 / d as f32) as f64) }
}

fn leak_join(parts: impl IntoIterator<Item = Str>, sep: &str) -> Str {
    Box::leak(parts.into_iter().collect::<Vec<_>>().join(sep).into_boxed_str())
}

fn history_of_types(db: &'static So, types: &'static [i64]) -> DenseFold<Id<Post>, i64> {
    db.post_history
        .with((&db.post_history.post_history_type_id).filt(move |t| types.contains(&t)))
        .select(&db.post_history.post)
        .inv()
        .dense_fold_outer(db.post.id.n, 0i64, |a, _| a + 1)
}

fn named_owner(db: &'static So, p: Id<Post>, dflt: Str) -> V {
    V::S(db.post.owner_user.get(p).map_or(dflt, |u| db.user.display_name.get(u).unwrap()))
}

fn hours_to(t0: i64, t: i64) -> f64 {
    (t0 - t) as f64 / 1e6
}

fn user_posts_q(db: &'static So) -> Fold<Id<User>, [i64; 4]> {
    g(db).select(posts_of(db).select(&db.post.post_type_id).opt()).fold([0i64; 4], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, 0],
        None => a,
    })
}

fn edit_names(db: &'static So) -> Fold<Id<Post>, [i64; 5]> {
    db.post_history
        .group_by(&db.post_history.post)
        .select((&db.post_history.post_history_type).select(&db.post_history_type.name).and(&db.post_history.creation_date))
        .fold([0, 0, 0, i64::MIN, 0], |a: [i64; 5], (n, d)| [a[0] + 1, a[1] + (n == "Edit Body") as i64, a[2] + (n == "Edit Title") as i64, a[3].max(d), 0])
}

// --- batch 121 --------------------------------------------------------------

// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate AS PostCreationDate,
// P.Score AS PostScore,
// P.ViewCount,
// U.DisplayName AS PostOwner,
// U.Reputation AS PostOwnerReputation,
// COALESCE(C.CommentCount, 0) AS TotalComments,
// COALESCE(V.VoteCount, 0) AS TotalVotes
// FROM
// Posts P
// LEFT JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// (SELECT
// PostId, COUNT(*) AS CommentCount
// FROM
// Comments
// GROUP BY
// PostId) C ON P.Id = C.PostId
// LEFT JOIN
// (SELECT
// PostId, COUNT(*) AS VoteCount
// FROM
// Votes
// GROUP BY
// PostId) V ON P.Id = V.PostId
// WHERE
// P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// ORDER BY
// P.CreationDate DESC
// LIMIT 100;
fn q12807(db: &'static So) -> String {
    let mut v = Vec::new();
    since(db, year_ago()).select(Ident::<Post>::new().and(comments_per_post(db)).and(votes_per_post(db))).drive(|_, x| v.push(x));
    out(v, |&((p, _), _)| newest(db, p), 100, |&((p, c), x)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner", "rep"]);
        f.extend([V::I(c), V::I(x)]);
        f
    })
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS PostCount,
// AVG(p.Score) AS AverageScore,
// SUM(CASE WHEN u.Id IS NOT NULL THEN 1 ELSE 0 END) AS UserPostCount,
// AVG(u.Reputation) AS AverageUserReputation,
// SUM(c.CommentCount) AS TotalComments
// FROM
// PostTypes pt
// LEFT JOIN
// Posts p ON p.PostTypeId = pt.Id
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// (SELECT
// PostId,
// COUNT(Id) AS CommentCount
// FROM
// Comments
// GROUP BY
// PostId) c ON c.PostId = p.Id
// GROUP BY
// pt.Name
// ORDER BY
// PostCount DESC;
fn q12815(db: &'static So) -> String {
    let of_type: HashIdx<Id<PostType>, Id<Post>> = (&db.post.post_type).inv().collect();
    let cf = db.comment.group_by(&db.comment.post).fold(0i64, |a, _| a + 1);
    let f = db
        .post_type
        .group_by(&db.post_type.name)
        .select((&of_type).select((&db.post.score).and((&db.post.owner_user).select(&db.user.reputation).opt()).and((&cf).opt())).opt())
        .fold([0i64; 6], |a, p| match p {
            Some(((s, r), c)) => [a[0] + 1, a[1] + s, a[2] + r.is_some() as i64, a[3] + r.unwrap_or(0), a[4] + c.is_some() as i64, a[5] + c.unwrap_or(0)],
            None => a,
        });
    let mut v = Vec::new();
    (&f).drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), V::I(a[2]), avg(a[3], a[2]), nullable(a[5], a[4])])))
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.Reputation,
// COUNT(DISTINCT P.Id) AS TotalPosts,
// COUNT(DISTINCT C.Id) AS TotalComments,
// SUM(V.BountyAmount) AS TotalBounty
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON U.Id = V.UserId
// GROUP BY
// U.Id, U.Reputation
// ),
// PostStats AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// COUNT(DISTINCT C.Id) AS CommentCount,
// COUNT(DISTINCT V.Id) AS VoteCount,
// SUM(CASE
// WHEN V.VoteTypeId = 2 THEN 1
// ELSE 0
// END) AS UpVoteCount,
// SUM(CASE
// WHEN V.VoteTypeId = 3 THEN 1
// ELSE 0
// END) AS DownVoteCount
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// P.Id, P.Title
// )
// SELECT
// U.UserId,
// U.Reputation,
// U.TotalPosts,
// U.TotalComments,
// U.TotalBounty,
// P.PostId,
// P.Title,
// P.CommentCount,
// P.VoteCount,
// P.UpVoteCount,
// P.DownVoteCount
// FROM
// UserStats U
// JOIN
// PostStats P ON U.UserId = P.PostId
// ORDER BY
// U.Reputation DESC, U.TotalPosts DESC;
fn q12818(db: &'static So) -> String {
    let uid = uids(db);
    let ub = g(db).select(posts_of(db).select(comments_of(db).opt()).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt())).fold([0i64; 2], |a, (_, b)| {
        let b = b.flatten();
        [a[0] + b.is_some() as i64, a[1] + b.unwrap_or(0)]
    });
    let dp = ud(db, UserWhere::All, posts_of(db));
    let dc = ud(db, UserWhere::All, posts_of(db).select(comments_of(db)));
    let c = per_post_distinct(db, comments_of(db));
    let x = per_post_distinct(db, votes_of(db));
    let mut v = Vec::new();
    stats_fold(db, db.post.with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cv", &[])
        .and((&c).opt())
        .and((&x).opt())
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&ub).and((&dp).opt()).and((&dc).opt())))
        .drive(|p, (((s, c), x), (((u, b), d), e))| v.push((p, s, c.unwrap_or(0), x.unwrap_or(0), u, b, d.unwrap_or(0), e.unwrap_or(0))));
    rows(v.iter().map(|&(p, s, c, x, u, b, d, e)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep"), V::I(d), V::I(e), nullable(b[1], b[0])];
        f.extend(post_fields(db, p, &["id", "title"]));
        f.extend([V::I(c), V::I(x), V::I(s.up), V::I(s.down)]);
        row(f)
    }))
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.Reputation,
// COUNT(P.Id) AS PostCount,
// SUM(P.Score) AS TotalPostScore,
// COUNT(B.Id) AS BadgeCount
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// GROUP BY
// U.Id, U.Reputation
// ),
// PostStats AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS TotalPosts,
// AVG(P.Score) AS AvgPostScore,
// SUM(P.ViewCount) AS TotalViews
// FROM
// Posts P
// GROUP BY
// P.OwnerUserId
// )
// SELECT
// U.UserId,
// U.Reputation,
// U.PostCount,
// U.TotalPostScore,
// U.BadgeCount,
// COALESCE(P.TotalPosts, 0) AS TotalPosts,
// COALESCE(P.AvgPostScore, 0) AS AvgPostScore,
// COALESCE(P.TotalViews, 0) AS TotalViews
// FROM
// UserStats U
// LEFT JOIN
// PostStats P ON U.UserId = P.OwnerUserId
// ORDER BY
// U.Reputation DESC,
// U.TotalPostScore DESC;
fn q12826(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "b", any_post);
    let pf = owned(db).group_by(&db.post.owner_user).select((&db.post.score).and((&db.post.view_count).opt())).fold([0i64; 3], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.unwrap_or(0)]);
    let mut v = Vec::new();
    (&us).and((&pf).opt()).drive(|u, (a, p)| v.push((u, a, p.unwrap_or([0; 3]))));
    rows(v.iter().map(|&(u, a, p)| row(vec![user_col(db, u, "uid"), user_col(db, u, "rep"), V::I(a.n), ustat_field(&a, "score_sum"), V::I(a.bx), V::I(p[0]), or0(p[1], p[0]), V::I(p[2])])))
}

// WITH UserMetrics AS (
// SELECT
// U.Id AS UserId,
// U.Reputation AS UserReputation
// FROM Users U
// ),
// PostMetrics AS (
// SELECT
// P.PostTypeId,
// COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS QuestionCount,
// COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS AnswerCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes,
// AVG(UM.UserReputation) AS AvgUserReputation
// FROM Posts P
// LEFT JOIN Votes V ON P.Id = V.PostId
// LEFT JOIN UserMetrics UM ON P.OwnerUserId = UM.UserId
// WHERE P.CreationDate >= '2020-01-01'
// GROUP BY P.PostTypeId
// )
// SELECT
// PT.Name AS PostTypeName,
// PM.QuestionCount,
// PM.AnswerCount,
// PM.TotalUpVotes,
// PM.TotalDownVotes,
// PM.AvgUserReputation
// FROM PostMetrics PM
// JOIN PostTypes PT ON PM.PostTypeId = PT.Id
// ORDER BY PM.PostTypeId;
fn q12827(db: &'static So) -> String {
    let f = by_key(
        since(db, date(2020, 1, 1)),
        &db.post.post_type,
        (&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt()).and((&db.post.owner_user).select(&db.user.reputation).opt()),
        [0i64; 6],
        |a, ((t, x), r)| [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (x == Some(2)) as i64, a[3] + (x == Some(3)) as i64, a[4] + r.is_some() as i64, a[5] + r.unwrap_or(0)],
    );
    let mut v = Vec::new();
    (&f).drive(|t, a| v.push((t, a)));
    rows(v.iter().map(|&(t, a)| {
        let mut f = vec![V::S(db.post_type.name.get(t).unwrap())];
        f.extend(ints(&a[..4]));
        f.push(avg(a[5], a[4]));
        row(f)
    }))
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// COUNT(DISTINCT c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// MAX(v.CreationDate) AS LastVoteDate
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate
// ),
// UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(u.Views) AS TotalViews,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// u.Id, u.DisplayName
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.CommentCount,
// ps.VoteCount,
// ps.UpVotes,
// ps.DownVotes,
// ps.LastVoteDate,
// us.UserId,
// us.DisplayName,
// us.PostCount,
// us.TotalViews,
// us.GoldBadges,
// us.SilverBadges,
// us.BronzeBadges
// FROM
// PostStats ps
// JOIN
// UserStats us ON ps.PostId = us.UserId
// ORDER BY
// ps.CreationDate DESC
// FETCH FIRST 100 ROWS ONLY;
fn q12850(db: &'static So) -> String {
    let uid = uids(db);
    let c = per_post_distinct(db, comments_of(db));
    let x = per_post_distinct(db, votes_of(db));
    let us = g(db).select((&db.user.views).and(posts_of(db).opt()).and(badges_of(db).select(&db.badge.class).opt())).fold([0i64; 4], |a, ((w, _), b)| {
        [a[0] + w, a[1] + (b == Some(1)) as i64, a[2] + (b == Some(2)) as i64, a[3] + (b == Some(3)) as i64]
    });
    let dp = ud(db, UserWhere::All, posts_of(db));
    let mut v = Vec::new();
    stats_fold(db, db.post.with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cv", &[])
        .and((&c).opt())
        .and((&x).opt())
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&us).and((&dp).opt())))
        .drive(|p, (((s, c), x), ((u, a), d))| v.push((p, s, c.unwrap_or(0), x.unwrap_or(0), u, a, d.unwrap_or(0))));
    out(v, |&(p, ..)| newest(db, p), 100, |&(p, s, c, x, u, a, d)| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(c), V::I(x), V::I(s.up), V::I(s.down), stat_field(&s, "vmax").unwrap(), user_col(db, u, "uid"), user_col(db, u, "name"), V::I(d)]);
        f.extend(ints(&a));
        f
    })
}

// WITH PostStats AS (
// SELECT
// P.Id AS PostId,
// P.PostTypeId,
// P.Score,
// P.ViewCount,
// P.AnswerCount,
// P.CommentCount,
// P.FavoriteCount,
// U.Reputation AS OwnerReputation,
// U.AccountId AS OwnerAccountId
// FROM
// Posts P
// LEFT JOIN
// Users U ON P.OwnerUserId = U.Id
// ),
// VoteStats AS (
// SELECT
// PostId,
// COUNT(CASE WHEN VoteTypeId = 2 THEN 1 END) AS UpVotes,
// COUNT(CASE WHEN VoteTypeId = 3 THEN 1 END) AS DownVotes
// FROM
// Votes
// GROUP BY
// PostId
// ),
// BadgeStats AS (
// SELECT
// UserId,
// COUNT(*) AS TotalBadges
// FROM
// Badges
// GROUP BY
// UserId
// ),
// FinalStats AS (
// SELECT
// PS.PostId,
// PS.PostTypeId,
// PS.Score,
// PS.ViewCount,
// PS.AnswerCount,
// PS.CommentCount,
// PS.FavoriteCount,
// VS.UpVotes,
// VS.DownVotes,
// PS.OwnerReputation,
// BS.TotalBadges
// FROM
// PostStats PS
// LEFT JOIN
// VoteStats VS ON PS.PostId = VS.PostId
// LEFT JOIN
// BadgeStats BS ON PS.OwnerAccountId = BS.UserId
// )
// SELECT
// PostId,
// PostTypeId,
// Score,
// ViewCount,
// AnswerCount,
// CommentCount,
// FavoriteCount,
// UpVotes,
// DownVotes,
// OwnerReputation,
// TotalBadges
// FROM
// FinalStats
// ORDER BY
// Score DESC, ViewCount DESC
// LIMIT 100;
fn q12875(db: &'static So) -> String {
    let uid = uids(db);
    let pv = post_votes(db);
    let bu = db.badge.group_by(&db.badge.user).fold(0i64, |a, _| a + 1);
    let mut v = Vec::new();
    db.post
        .select(Ident::<Post>::new().and((&pv).opt()).and((&db.post.owner_user).select(&db.user.account_id).select(&uid).select(&bu).opt()))
        .drive(|_, x| v.push(x));
    out(v, |&((p, _), _)| score_views(db, p), 100, |&((p, x), b)| {
        let mut f = post_fields(db, p, &["id", "type_id", "score", "views", "answers", "comments", "favorites"]);
        f.extend([oint(x.map(|x| x[1])), oint(x.map(|x| x[2]))]);
        f.extend(post_fields(db, p, &["rep"]));
        f.push(oint(b));
        f
    })
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN p.Score IS NOT NULL THEN p.Score ELSE 0 END) AS TotalScore
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.Reputation
// ),
// UserBadgeStats AS (
// SELECT
// UserId,
// COUNT(b.Id) AS BadgeCount,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM
// Badges b
// GROUP BY
// UserId
// )
// SELECT
// ups.UserId,
// ups.Reputation,
// ups.PostCount,
// ups.QuestionCount,
// ups.AnswerCount,
// ups.TotalScore,
// ubs.BadgeCount,
// ubs.GoldBadges,
// ubs.SilverBadges,
// ubs.BronzeBadges
// FROM
// UserPostStats ups
// LEFT JOIN
// UserBadgeStats ubs ON ups.UserId = ubs.UserId
// ORDER BY
// ups.Reputation DESC, ups.PostCount DESC;
fn q12880(db: &'static So) -> String {
    let bc = badge_classes(db);
    let mut v = Vec::new();
    upqa(db).and((&bc).opt()).drive(|u, (a, b)| v.push((u, a, b)));
    rows(v.iter().map(|&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep")];
        f.extend(ints(&a[..3]));
        f.push(V::I(a[5]));
        f.extend((0..4).map(|i| oint(b.map(|b| b[i]))));
        row(f)
    }))
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
// p.FavoriteCount,
// u.DisplayName AS OwnerDisplayName,
// COUNT(c.Id) AS TotalComments,
// AVG(v.BountyAmount) AS AverageBounty
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.FavoriteCount, u.DisplayName
// ),
// PostHistoryStats AS (
// SELECT
// ph.PostId,
// COUNT(ph.Id) AS HistoryCount,
// MAX(ph.CreationDate) AS LastEditDate
// FROM
// PostHistory ph
// GROUP BY
// ph.PostId
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.Score,
// ps.ViewCount,
// ps.AnswerCount,
// ps.CommentCount,
// ps.FavoriteCount,
// ps.OwnerDisplayName,
// ps.TotalComments,
// ps.AverageBounty,
// phs.HistoryCount,
// phs.LastEditDate
// FROM
// PostStats ps
// LEFT JOIN
// PostHistoryStats phs ON ps.PostId = phs.PostId
// ORDER BY
// ps.Score DESC, ps.ViewCount DESC
// LIMIT 100;
fn q12881(db: &'static So) -> String {
    let hf = db.post_history.group_by(&db.post_history.post).select(&db.post_history.creation_date).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let mut v = Vec::new();
    stats_fold(db, db.post.iq(), Ident::<Post>::new(), "cv", &[]).and((&hf).opt()).drive(|p, (s, h)| v.push((p, s, h)));
    out(v, |&(p, _, _)| score_views(db, p), 100, |&(p, s, h)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "favorites", "owner"]);
        f.extend([V::I(s.cx), stat_field(&s, "bounty_avg").unwrap(), oint(h.map(|h| h.0)), ots(h.map(|h| h.1))]);
        f
    })
}

// WITH UserPostStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(COALESCE(P.ViewCount, 0)) AS TotalViews,
// SUM(COALESCE(P.Score, 0)) AS TotalScore
// FROM Users U
// LEFT JOIN Posts P ON U.Id = P.OwnerUserId
// GROUP BY U.Id, U.DisplayName
// ),
// UserBadgeStats AS (
// SELECT
// B.UserId,
// COUNT(B.Id) AS TotalBadges,
// SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM Badges B
// GROUP BY B.UserId
// )
// SELECT
// UPS.UserId,
// UPS.DisplayName,
// UPS.TotalPosts,
// UPS.Questions,
// UPS.Answers,
// UPS.TotalViews,
// UPS.TotalScore,
// COALESCE(UBS.TotalBadges, 0) AS TotalBadges,
// COALESCE(UBS.GoldBadges, 0) AS GoldBadges,
// COALESCE(UBS.SilverBadges, 0) AS SilverBadges,
// COALESCE(UBS.BronzeBadges, 0) AS BronzeBadges
// FROM UserPostStats UPS
// LEFT JOIN UserBadgeStats UBS ON UPS.UserId = UBS.UserId
// ORDER BY UPS.TotalScore DESC, UPS.TotalPosts DESC;
fn q12883(db: &'static So) -> String {
    let bc = badge_classes(db);
    let mut v = Vec::new();
    upqa(db).and((&bc).opt()).drive(|u, (a, b)| v.push((u, a, b.unwrap_or([0; 4]))));
    rows(v.iter().map(|&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[..3]));
        f.extend([V::I(a[4]), V::I(a[5])]);
        f.extend(ints(&b));
        row(f)
    }))
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AverageScore,
// MAX(p.LastActivityDate) AS MostRecentActivity
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName
// ),
// PostStats AS (
// SELECT
// Id AS PostId,
// Title,
// CreationDate,
// ViewCount,
// AnswerCount,
// CommentCount,
// OwnerUserId
// FROM
// Posts
// WHERE
// CreationDate >= (TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days')
// )
// SELECT
// us.UserId,
// us.DisplayName,
// us.TotalPosts,
// us.AverageScore,
// us.MostRecentActivity,
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.ViewCount,
// ps.AnswerCount,
// ps.CommentCount
// FROM
// UserStats us
// JOIN
// PostStats ps ON us.UserId = ps.OwnerUserId
// ORDER BY
// us.TotalPosts DESC, us.AverageScore DESC;
fn q12885(db: &'static So) -> String {
    let uf = g(db).select(posts_of(db).select((&db.post.score).and(&db.post.last_activity_date)).opt()).fold([0, 0, i64::MIN], |a: [i64; 3], p| match p {
        Some((s, la)) => [a[0] + 1, a[1] + s, a[2].max(la)],
        None => a,
    });
    let mut v = Vec::new();
    owned_since(db, ts(2024, 9, 1, 12, 34, 56)).select(Ident::<Post>::new().and((&db.post.owner_user).select(Ident::<User>::new().and(&uf)))).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(p, (u, a))| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(a[0]), avg(a[1], a[0]), V::T(a[2])];
        f.extend(post_fields(db, p, &["id", "title", "created", "views", "answers", "comments"]));
        row(f)
    }))
}

// SELECT
// p.Id AS PostID,
// p.Title,
// u.DisplayName AS OwnerDisplayName,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// p.AnswerCount,
// COALESCE(c.CommentCount, 0) AS CommentCount,
// COALESCE(b.BadgeCount, 0) AS BadgeCount,
// COALESCE(v.VoteCount, 0) AS VoteCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON c.PostId = p.Id
// LEFT JOIN
// (SELECT UserId, COUNT(*) AS BadgeCount FROM Badges GROUP BY UserId) b ON b.UserId = u.Id
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS VoteCount FROM Votes GROUP BY PostId) v ON v.PostId = p.Id
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// ORDER BY
// p.CreationDate DESC;
fn q12902(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    owned_since(db, year_ago()).select(Ident::<Post>::new().and(comments_per_post(db)).and((&db.post.owner_user).select(&bu)).and(votes_per_post(db))).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(((p, c), b), x)| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "views", "score", "answers"]);
        f.extend([V::I(c), V::I(b), V::I(x)]);
        row(f)
    }))
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
// p.FavoriteCount,
// COALESCE(u.DisplayName, 'Community User') AS OwnerDisplayName,
// COALESCE(u.Reputation, 0) AS OwnerReputation
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.CreationDate >= '2023-01-01'
// ),
// MostActiveUsers AS (
// SELECT
// u.Id,
// u.DisplayName,
// COUNT(p.Id) AS PostsCount,
// SUM(p.ViewCount) AS TotalViews
// FROM
// Users u
// JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id
// ORDER BY
// PostsCount DESC
// LIMIT 10
// ),
// PopularPosts AS (
// SELECT
// PostId,
// COUNT(*) AS VoteCount
// FROM
// Votes
// GROUP BY
// PostId
// ORDER BY
// VoteCount DESC
// LIMIT 10
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.Score,
// ps.ViewCount,
// ps.OwnerDisplayName,
// ps.OwnerReputation,
// ap.VoteCount AS PopularityScore
// FROM
// PostStats ps
// LEFT JOIN
// PopularPosts ap ON ps.PostId = ap.PostId
// ORDER BY
// ps.ViewCount DESC;
fn q12906(db: &'static So) -> String {
    let vc = db.vote.group_by(&db.vote.post_id).fold(0i64, |a, _| a + 1);
    let pop: MatSet<(i64, i64)> = whole(&vc).select(Same::new().and(&vc)).window(row_number, |(_, n)| n, desc).filt(|(_, n)| n <= 10).map(|(k, _)| k).collect();
    let pop_of: HashIdx<i64, (i64, i64)> = (&pop).map(|(p, _)| p).inv().collect();
    let mut v = Vec::new();
    since(db, date(2023, 1, 1)).select(Ident::<Post>::new().and((&db.post.origid).select(&pop_of).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(p, x)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.push(named_owner(db, p, "Community User"));
        f.push(V::I(db.post.owner_user.get(p).map_or(0, |u| db.user.reputation.get(u).unwrap())));
        f.push(oint(x.map(|x| x.1)));
        row(f)
    }))
}

// WITH PostMetrics AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// COALESCE(v.UpVotes, 0) AS UpVotes,
// COALESCE(v.DownVotes, 0) AS DownVotes,
// COALESCE(u.Reputation, 0) AS UserReputation
// FROM
// Posts p
// LEFT JOIN (
// SELECT
// PostId,
// SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Votes
// GROUP BY
// PostId
// ) v ON p.Id = v.PostId
// LEFT JOIN Users u ON p.OwnerUserId = u.Id
// )
// SELECT
// AVG(Score) AS AverageScore,
// AVG(ViewCount) AS AverageViewCount,
// SUM(UpVotes) AS TotalUpVotes,
// SUM(DownVotes) AS TotalDownVotes,
// COUNT(*) AS TotalPosts,
// MAX(UserReputation) AS HighestUserReputation
// FROM
// PostMetrics;
fn q12912(db: &'static So) -> String {
    let pv = post_votes(db);
    let Post { score, view_count, .. } = &db.post;
    let a = db.post.select(score.and(view_count.opt()).and((&pv).opt()).and((&db.post.owner_user).select(&db.user.reputation).opt())).fold_flat([0i64; 7], |a, (((s, w), x), r)| {
        let x = x.unwrap_or([0; 3]);
        [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + x[1], a[5] + x[2], a[6].max(r.unwrap_or(0))]
    });
    row(vec![avg(a[1], a[0]), avg(a[3], a[2]), V::I(a[4]), V::I(a[5]), V::I(a[0]), V::I(a[6])])
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// COUNT(DISTINCT c.Id) AS CommentCount,
// COUNT(DISTINCT a.Id) AS AnswerCount,
// COALESCE(MAX(v.CreationDate), DATE '1900-01-01') AS LastVoteDate
// FROM
// Posts p
// LEFT JOIN
// Comments c ON c.PostId = p.Id
// LEFT JOIN
// Posts a ON a.ParentId = p.Id AND a.PostTypeId = 2
// LEFT JOIN
// Votes v ON v.PostId = p.Id
// WHERE
// p.CreationDate >= DATE '2024-10-01' - INTERVAL '1 year'
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
// ps.AnswerCount,
// ps.LastVoteDate,
// CASE
// WHEN ps.Score > 0 THEN 'Positive'
// WHEN ps.Score < 0 THEN 'Negative'
// ELSE 'Neutral'
// END AS ScoreType
// FROM
// PostStats ps
// ORDER BY
// ps.Score DESC,
// ps.ViewCount DESC
// LIMIT 100;
fn q12920(db: &'static So) -> String {
    let c = per_post_distinct(db, comments_of(db));
    let a = per_post_distinct(db, answers_of(db));
    out(stats_with(db, since(db, date(2023, 10, 1)), "cvA", &[], &[&c, &a]), |&(p, _, _)| (score_views(db, p), db.post.origid.get(p).unwrap()), 100, |&(p, s, d)| {
        let sc = db.post.score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([V::I(d[0]), V::I(d[1]), V::T(if s.vx == 0 { date(1900, 1, 1) } else { s.vmax })]);
        f.push(V::S(if sc > 0 { "Positive" } else if sc < 0 { "Negative" } else { "Neutral" }));
        f
    })
}

// WITH UserPostCount AS (
// SELECT
// u.Id AS UserId,
// COUNT(p.Id) AS PostCount,
// SUM(COALESCE(p.ViewCount, 0)) AS TotalViews,
// SUM(COALESCE(p.Score, 0)) AS TotalScore
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id
// ),
// UserBadgeCount AS (
// SELECT
// b.UserId,
// COUNT(b.Id) AS BadgeCount
// FROM
// Badges b
// GROUP BY
// b.UserId
// )
// SELECT
// u.DisplayName,
// upc.PostCount,
// upc.TotalViews,
// upc.TotalScore,
// ubc.BadgeCount
// FROM
// Users u
// JOIN
// UserPostCount upc ON u.Id = upc.UserId
// LEFT JOIN
// UserBadgeCount ubc ON u.Id = ubc.UserId
// ORDER BY
// upc.TotalScore DESC,
// upc.PostCount DESC;
fn q12923(db: &'static So) -> String {
    let bf = db.badge.group_by(&db.badge.user).fold(0i64, |a, _| a + 1);
    let mut v = Vec::new();
    upqa(db).and((&bf).opt()).drive(|u, (a, b)| v.push((u, a, b)));
    rows(v.iter().map(|&(u, a, b)| row(vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[4]), V::I(a[5]), oint(b)])))
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.PostTypeId,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// p.AnswerCount,
// p.CommentCount,
// p.FavoriteCount,
// U.Reputation AS OwnerReputation,
// U.Location,
// COUNT(DISTINCT V.Id) AS VoteCount,
// COUNT(DISTINCT C.Id) AS CommentCount
// FROM
// Posts p
// LEFT JOIN
// Users U ON p.OwnerUserId = U.Id
// LEFT JOIN
// Votes V ON p.Id = V.PostId
// LEFT JOIN
// Comments C ON p.Id = C.PostId
// GROUP BY
// p.Id, p.PostTypeId, p.CreationDate, p.Score, p.ViewCount,
// p.AnswerCount, p.CommentCount, p.FavoriteCount,
// U.Reputation, U.Location
// ),
// PostTypeCounts AS (
// SELECT
// pt.Id AS PostTypeId,
// pt.Name AS PostTypeName,
// COUNT(ps.PostId) AS PostCount,
// SUM(ps.ViewCount) AS TotalViews,
// SUM(ps.Score) AS TotalScore,
// AVG(ps.OwnerReputation) AS AvgOwnerReputation
// FROM
// PostTypes pt
// LEFT JOIN
// PostStats ps ON pt.Id = ps.PostTypeId
// GROUP BY
// pt.Id, pt.Name
// )
// SELECT
// ptc.PostTypeId,
// ptc.PostTypeName,
// ptc.PostCount,
// ptc.TotalViews,
// ptc.TotalScore,
// ptc.AvgOwnerReputation,
// (ptc.TotalScore * 1.0 / NULLIF(ptc.PostCount, 0)) AS AvgScorePerPost,
// (ptc.TotalViews * 1.0 / NULLIF(ptc.PostCount, 0)) AS AvgViewsPerPost
// FROM
// PostTypeCounts ptc
// ORDER BY
// ptc.PostTypeId;
fn q12926(db: &'static So) -> String {
    let of_type: HashIdx<Id<PostType>, Id<Post>> = (&db.post.post_type).inv().collect();
    let f = db
        .post_type
        .group_by(Ident::<PostType>::new())
        .select((&of_type).select((&db.post.view_count).opt().and(&db.post.score).and((&db.post.owner_user).select(&db.user.reputation).opt())).opt())
        .fold([0i64; 6], |a, p| match p {
            Some(((w, s), r)) => [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s, a[4] + r.is_some() as i64, a[5] + r.unwrap_or(0)],
            None => a,
        });
    let mut v = Vec::new();
    (&f).drive(|t, a| v.push((t, a)));
    rows(v.iter().map(|&(t, a)| {
        let ts = if a[0] == 0 { None } else { Some(a[3]) };
        let tv = if a[1] == 0 { None } else { Some(a[2]) };
        let per = |x: Option<i64>| match x {
            Some(x) if a[0] != 0 => V::F(x as f64 / a[0] as f64),
            _ => V::Null,
        };
        row(vec![V::I(db.post_type.origid.get(t).unwrap()), V::S(db.post_type.name.get(t).unwrap()), V::I(a[0]), oint(tv), oint(ts), avg(a[5], a[4]), per(ts), per(tv)])
    }))
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.Reputation,
// U.UpVotes,
// U.DownVotes,
// U.Views,
// COUNT(B.Id) AS BadgeCount,
// SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM Users U
// LEFT JOIN Badges B ON U.Id = B.UserId
// GROUP BY U.Id, U.Reputation, U.UpVotes, U.DownVotes, U.Views
// ),
// PostStats AS (
// SELECT
// P.OwnerUserId,
// COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS QuestionCount,
// COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS AnswerCount,
// SUM(P.Score) AS TotalScore,
// SUM(P.ViewCount) AS TotalViews,
// AVG(P.CommentCount) AS AvgComments
// FROM Posts P
// GROUP BY P.OwnerUserId
// )
// SELECT
// U.UserId,
// U.Reputation,
// U.UpVotes,
// U.DownVotes,
// U.Views,
// U.BadgeCount,
// U.GoldBadges,
// U.SilverBadges,
// U.BronzeBadges,
// COALESCE(P.QuestionCount, 0) AS QuestionCount,
// COALESCE(P.AnswerCount, 0) AS AnswerCount,
// COALESCE(P.TotalScore, 0) AS TotalScore,
// COALESCE(P.TotalViews, 0) AS TotalViews,
// COALESCE(P.AvgComments, 0) AS AvgComments
// FROM UserStats U
// LEFT JOIN PostStats P ON U.UserId = P.OwnerUserId
// ORDER BY U.Reputation DESC;
fn q12929(db: &'static So) -> String {
    let bc = g(db).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let Post { post_type_id, score, view_count, comment_count, .. } = &db.post;
    let pf = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(score).and(view_count.opt()).and(comment_count)).fold([0i64; 6], |a, (((t, s), w), cc)| {
        [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + s, a[3] + w.unwrap_or(0), a[4] + cc, a[5] + 1]
    });
    let mut v = Vec::new();
    (&bc).and((&pf).opt()).drive(|u, (b, p)| v.push((u, b, p.unwrap_or([0; 6]))));
    rows(v.iter().map(|&(u, b, p)| {
        let mut f: Vec<V> = ["uid", "rep", "uup", "udown", "uviews"].iter().map(|c| user_col(db, u, c)).collect();
        f.extend(ints(&b));
        f.extend(ints(&p[..4]));
        f.push(or0(p[4], p[5]));
        row(f)
    }))
}

// SELECT
// p.PostTypeId,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// COUNT(DISTINCT c.Id) AS TotalComments,
// AVG(v.CountVotes) AS AverageVotesPerPost,
// AVG(u.Reputation) AS AverageUserReputation
// FROM
// Posts p
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CountVotes
// FROM Votes
// GROUP BY PostId) v ON p.Id = v.PostId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// GROUP BY
// p.PostTypeId
// ORDER BY
// p.PostTypeId;
fn q12957(db: &'static So) -> String {
    let vf = db.vote.group_by(&db.vote.post).fold(0i64, |a, _| a + 1);
    let f = by_key(db.post.iq(), &db.post.post_type_id, (&vf).opt().and((&db.post.owner_user).select(&db.user.reputation).opt()).and(comments_of(db).opt()), [0i64; 4], |a, ((x, r), _)| {
        [a[0] + x.is_some() as i64, a[1] + x.unwrap_or(0), a[2] + r.is_some() as i64, a[3] + r.unwrap_or(0)]
    });
    let np = db.post.group_by(&db.post.post_type_id).fold(0i64, |a, _| a + 1);
    let dc = db.post.group_by(&db.post.post_type_id).select(comments_of(db)).count_distinct();
    let mut v = Vec::new();
    (&f).and(&np).and((&dc).opt()).drive(|k, ((a, n), c)| v.push((k, a, n, c.unwrap_or(0))));
    rows(v.iter().map(|&(k, a, n, c)| row(vec![V::I(k), V::I(n), V::I(c), avg(a[1], a[0]), avg(a[3], a[2])])))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS PostCount,
// AVG(p.Score) AS AverageScore,
// SUM(COALESCE(c.CommentCount, 0)) AS TotalComments
// FROM
// Posts p
// LEFT JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// (SELECT
// PostId,
// COUNT(Id) AS CommentCount
// FROM
// Comments
// GROUP BY
// PostId) c ON p.Id = c.PostId
// GROUP BY
// pt.Name
// ORDER BY
// PostCount DESC;
fn q12961(db: &'static So) -> String {
    let f = by_key(db.post.iq(), name(db), (&db.post.score).and(comments_per_post(db)), [0i64; 3], |a, (s, c)| [a[0] + 1, a[1] + s, a[2] + c]);
    let mut v = Vec::new();
    (&f).drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), V::I(a[2])])))
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(P.Id) AS PostCount,
// SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END) AS PositiveScorePosts,
// SUM(CASE WHEN P.Score < 0 THEN 1 ELSE 0 END) AS NegativeScorePosts,
// SUM(V.BountyAmount) AS TotalBounty
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Votes V ON P.Id = V.PostId AND V.VoteTypeId IN (8, 9)
// GROUP BY
// U.Id, U.DisplayName, U.Reputation
// ),
// PostSummary AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.Score,
// P.ViewCount,
// CASE
// WHEN P.PostTypeId = 1 THEN 'Question'
// WHEN P.PostTypeId = 2 THEN 'Answer'
// ELSE 'Other'
// END AS PostType,
// COUNT(C.Id) AS CommentCount
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.Score, P.ViewCount, P.PostTypeId
// )
// SELECT
// US.UserId,
// US.DisplayName,
// US.Reputation,
// US.PostCount,
// US.PositiveScorePosts,
// US.NegativeScorePosts,
// US.TotalBounty,
// PS.PostId,
// PS.Title,
// PS.CreationDate,
// PS.Score,
// PS.ViewCount,
// PS.PostType,
// PS.CommentCount
// FROM
// UserStats US
// LEFT JOIN
// PostSummary PS ON US.UserId = PS.PostId
// ORDER BY
// US.Reputation DESC, US.PostCount DESC;
fn q12965(db: &'static So) -> String {
    let pid = pids(db);
    let us = g(db)
        .select(posts_of(db).select((&db.post.score).and(votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).in_v(vec![8, 9]))).select((&db.vote.bounty_amount).opt()).opt())).opt())
        .fold([0i64; 5], |a, p| match p {
            Some((s, b)) => {
                let b = b.flatten();
                [a[0] + 1, a[1] + (s > 0) as i64, a[2] + (s < 0) as i64, a[3] + b.is_some() as i64, a[4] + b.unwrap_or(0)]
            }
            None => a,
        });
    let mut v = Vec::new();
    (&us).and((&db.user.origid).select(&pid).select(Ident::<Post>::new().and(comments_per_post(db))).opt()).drive(|u, (a, p)| v.push((u, a, p)));
    rows(v.iter().map(|&(u, a, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep"), V::I(a[0]), V::I(a[1]), V::I(a[2]), nullable(a[4], a[3])];
        match p {
            Some((p, c)) => {
                f.extend(post_fields(db, p, &["id", "title", "created", "score", "views"]));
                f.push(V::S(match db.post.post_type_id.get(p).unwrap() {
                    1 => "Question",
                    2 => "Answer",
                    _ => "Other",
                }));
                f.push(V::I(c));
            }
            None => f.extend(nulls(7)),
        }
        row(f)
    }))
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// COUNT(DISTINCT p.Id) AS PostCount,
// AVG(p.Score) AS AverageScore,
// COUNT(DISTINCT c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// u.Id, u.Reputation
// )
// SELECT
// CASE
// WHEN Reputation >= 10000 THEN 'Top Users'
// WHEN Reputation >= 1000 THEN 'Active Users'
// WHEN Reputation >= 100 THEN 'New Users'
// ELSE 'Inactive Users'
// END AS UserLevel,
// COUNT(UserId) AS UserCount,
// SUM(PostCount) AS TotalPosts,
// AVG(AverageScore) AS AvgPostScore,
// SUM(CommentCount) AS TotalComments,
// SUM(VoteCount) AS TotalVotes
// FROM
// UserStats
// GROUP BY
// UserLevel
// ORDER BY
// UserLevel DESC;
fn q12969(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "cv", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let dc = ud(db, UserWhere::All, posts_of(db).select(comments_of(db)));
    let dv = ud(db, UserWhere::All, posts_of(db).select(votes_of(db)));
    let level = |r: i64| -> Str {
        if r >= 10000 {
            "Top Users"
        } else if r >= 1000 {
            "Active Users"
        } else if r >= 100 {
            "New Users"
        } else {
            "Inactive Users"
        }
    };
    let f = db
        .user
        .group_by((&db.user.reputation).map(level))
        .select((&us).and((&dp).opt()).and((&dc).opt()).and((&dv).opt()))
        .fold(([0i64; 5], 0f64), |(a, s), (((u, p), c), x)| {
            let m = u.n > 0;
            ([a[0] + 1, a[1] + p.unwrap_or(0), a[2] + m as i64, a[3] + c.unwrap_or(0), a[4] + x.unwrap_or(0)], if m { s + u.score_sum as f64 / u.n as f64 } else { s })
        });
    let mut v = Vec::new();
    (&f).drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&(k, (a, s))| row(vec![V::S(k), V::I(a[0]), V::I(a[1]), if a[2] == 0 { V::Null } else { V::F(s / a[2] as f64) }, V::I(a[3]), V::I(a[4])])))
}

// WITH PostStats AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.ViewCount,
// P.Score,
// COALESCE(C.Count, 0) AS CommentCount,
// COALESCE(A.AnswerCount, 0) AS AnswerCount,
// U.Reputation AS OwnerReputation
// FROM
// Posts P
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS Count FROM Comments GROUP BY PostId) C ON P.Id = C.PostId
// LEFT JOIN
// (SELECT ParentId, COUNT(*) AS AnswerCount FROM Posts WHERE PostTypeId = 2 GROUP BY ParentId) A ON P.Id = A.ParentId
// JOIN
// Users U ON P.OwnerUserId = U.Id
// WHERE
// P.PostTypeId = 1
// ),
// PostHistoryStats AS (
// SELECT
// PH.PostId,
// COUNT(*) AS EditCount,
// MAX(PH.CreationDate) AS LastEditDate,
// MAX(CASE WHEN PH.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS IsClosed,
// MAX(CASE WHEN PH.PostHistoryTypeId = 11 THEN 1 ELSE 0 END) AS IsReopened
// FROM
// PostHistory PH
// GROUP BY
// PH.PostId
// )
// SELECT
// PS.PostId,
// PS.Title,
// PS.CreationDate,
// PS.ViewCount,
// PS.Score,
// PS.CommentCount,
// PS.AnswerCount,
// PS.OwnerReputation,
// PHS.EditCount,
// PHS.LastEditDate,
// PHS.IsClosed,
// PHS.IsReopened
// FROM
// PostStats PS
// LEFT JOIN
// PostHistoryStats PHS ON PS.PostId = PHS.PostId
// ORDER BY
// PS.CreationDate DESC
// LIMIT 100;
fn q12972(db: &'static So) -> String {
    let hf = db.post_history.group_by(&db.post_history.post).select((&db.post_history.post_history_type_id).and(&db.post_history.creation_date)).fold([0, i64::MIN, 0, 0], |a: [i64; 4], (t, d)| {
        [a[0] + 1, a[1].max(d), a[2].max((t == 10) as i64), a[3].max((t == 11) as i64)]
    });
    let mut v = Vec::new();
    owned(db).with((&db.post.post_type_id).eq(1)).select(Ident::<Post>::new().and(comments_per_post(db)).and(typed_answers_per_post(db)).and((&hf).opt())).drive(|_, x| v.push(x));
    out(v, |&(((p, _), _), _)| newest(db, p), 100, |&(((p, c), a), h)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend([V::I(c), V::I(a)]);
        f.extend(post_fields(db, p, &["rep"]));
        match h {
            Some(h) => f.extend([V::I(h[0]), V::T(h[1]), V::I(h[2]), V::I(h[3])]),
            None => f.extend(nulls(4)),
        }
        f
    })
}

// WITH UserPosts AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName
// ),
// UserVotes AS (
// SELECT
// v.UserId,
// COUNT(v.Id) AS TotalVotes
// FROM
// Votes v
// GROUP BY
// v.UserId
// )
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// up.TotalPosts,
// uv.TotalVotes
// FROM
// Users u
// LEFT JOIN
// UserPosts up ON u.Id = up.UserId
// LEFT JOIN
// UserVotes uv ON u.Id = uv.UserId
// ORDER BY
// u.Reputation DESC;
fn q12980(db: &'static So) -> String {
    let pc = (&db.post.owner_user).inv().dense_fold_outer(db.user.id.n, 0i64, |a, _| a + 1);
    let vf = db.vote.group_by(&db.vote.user).fold(0i64, |a, _| a + 1);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and(&pc).and((&vf).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((u, p), x)| row(vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep"), V::I(p), oint(x)])))
}

// WITH UserEngagement AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN c.Id IS NOT NULL THEN 1 ELSE 0 END) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 5 THEN 1 ELSE 0 END) AS FavoriteCount,
// SUM(CASE WHEN v.VoteTypeId = 6 THEN 1 ELSE 0 END) AS CloseVoteCount
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN Comments c ON p.Id = c.PostId
// LEFT JOIN Votes v ON p.Id = v.PostId
// GROUP BY u.Id, u.DisplayName
// )
// SELECT
// UserId,
// DisplayName,
// PostCount,
// QuestionCount,
// AnswerCount,
// CommentCount,
// UpVoteCount,
// DownVoteCount,
// FavoriteCount,
// CloseVoteCount
// FROM UserEngagement
// ORDER BY PostCount DESC;
fn q12983(db: &'static So) -> String {
    let uf = g(db)
        .select(posts_of(db).select((&db.post.post_type_id).and(comments_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 7], |a, p| match p {
            Some(((t, c), x)) => [
                a[0] + (t == 1) as i64,
                a[1] + (t == 2) as i64,
                a[2] + c.is_some() as i64,
                a[3] + (x == Some(2)) as i64,
                a[4] + (x == Some(3)) as i64,
                a[5] + (x == Some(5)) as i64,
                a[6] + (x == Some(6)) as i64,
            ],
            None => a,
        });
    let dp = ud(db, UserWhere::All, posts_of(db));
    let mut v = Vec::new();
    (&uf).and((&dp).opt()).drive(|u, (a, d)| v.push((u, a, d.unwrap_or(0))));
    rows(v.iter().map(|&(u, a, d)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(d)];
        f.extend(ints(&a));
        row(f)
    }))
}

// WITH PostCount AS (
// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// GROUP BY
// pt.Name
// ),
// TotalVotes AS (
// SELECT
// p.Id AS PostId,
// SUM(CASE
// WHEN vt.Name = 'UpMod' THEN 1
// WHEN vt.Name = 'DownMod' THEN -1
// ELSE 0
// END) AS VoteScore
// FROM
// Votes v
// JOIN
// VoteTypes vt ON v.VoteTypeId = vt.Id
// JOIN
// Posts p ON v.PostId = p.Id
// GROUP BY
// p.Id
// )
// SELECT
// pc.PostType,
// pc.TotalPosts,
// COALESCE(SUM(tv.VoteScore), 0) AS TotalVoteScore
// FROM
// PostCount pc
// LEFT JOIN
// Posts p ON pc.PostType = (SELECT Name FROM PostTypes WHERE Id = p.PostTypeId)
// LEFT JOIN
// TotalVotes tv ON p.Id = tv.PostId
// GROUP BY
// pc.PostType, pc.TotalPosts
// ORDER BY
// pc.TotalPosts DESC;
fn q12993(db: &'static So) -> String {
    let vs = db.vote.group_by(&db.vote.post).select((&db.vote.vote_type).select(&db.vote_type.name)).fold(0i64, |a, n| a + if n == "UpMod" { 1 } else if n == "DownMod" { -1 } else { 0 });
    let f = by_key(db.post.iq(), name(db), (&vs).opt(), [0i64; 2], |a, x| [a[0] + 1, a[1] + x.unwrap_or(0)]);
    let mut v = Vec::new();
    (&f).drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), V::I(a[1])])))
}

// WITH PostSummary AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.ViewCount,
// P.Score,
// U.DisplayName AS OwnerDisplayName,
// COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
// COUNT(I.Id) AS InteractionCount
// FROM
// Posts P
// LEFT JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// LEFT JOIN
// PostLinks I ON P.Id = I.PostId
// WHERE
// P.CreationDate >= DATE '2023-01-01'
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.ViewCount, P.Score, U.DisplayName
// ),
// PostTypeSummary AS (
// SELECT
// PT.Name AS PostTypeName,
// COUNT(PS.PostId) AS PostCount,
// SUM(PS.ViewCount) AS TotalViews,
// AVG(PS.Score) AS AverageScore
// FROM
// PostSummary PS
// JOIN
// PostTypes PT ON PS.PostId = PT.Id
// GROUP BY
// PT.Name
// )
// SELECT
// PTS.PostTypeName,
// PTS.PostCount,
// PTS.TotalViews,
// PTS.AverageScore
// FROM
// PostTypeSummary PTS
// ORDER BY
// PTS.PostCount DESC;
fn q12994(db: &'static So) -> String {
    let ptid: HashIdx<i64, Id<PostType>> = (&db.post_type.origid).inv().collect();
    let base = since(db, date(2023, 1, 1)).with((&db.post.origid).select(&ptid));
    let f = by_key(base, (&db.post.origid).select(&ptid).select(&db.post_type.name), (&db.post.view_count).opt().and(&db.post.score), [0i64; 4], |a, (w, s)| {
        [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s]
    });
    let mut v = Vec::new();
    (&f).drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), nullable(a[2], a[1]), avg(a[3], a[0])])))
}

// SELECT
// P.Id AS PostID,
// P.Title,
// P.CreationDate AS PostCreationDate,
// U.DisplayName AS OwnerDisplayName,
// U.Reputation AS OwnerReputation,
// P.ViewCount,
// P.Score,
// COALESCE(COUNT(V.id), 0) AS TotalVotes,
// COALESCE((SELECT COUNT(C.id) FROM Comments C WHERE C.PostId = P.Id), 0) AS TotalComments,
// COALESCE((SELECT COUNT(B.id) FROM Badges B WHERE B.UserId = U.Id), 0) AS TotalBadges
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// WHERE
// P.CreationDate >= DATE '2024-10-01' - INTERVAL '1 year'
// GROUP BY
// P.Id, P.Title, P.CreationDate, U.DisplayName, U.Reputation, P.ViewCount, P.Score, U.Id
// ORDER BY
// P.CreationDate DESC;
fn q13015(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    owned_since(db, date(2023, 10, 1)).select(Ident::<Post>::new().and(votes_per_post(db)).and(comments_per_post(db)).and((&db.post.owner_user).select(&bu))).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(((p, x), c), b)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner", "rep", "views", "score"]);
        f.extend([V::I(x), V::I(c), V::I(b)]);
        row(f)
    }))
}

// WITH PostStatistics AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// COUNT(DISTINCT c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
// (SELECT COUNT(a.Id)
// FROM Posts a
// WHERE a.ParentId = p.Id) AS AnswerCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount
// ),
// UserStatistics AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// SUM(u.Views) AS TotalViews,
// SUM(u.UpVotes) AS TotalUpVotes,
// SUM(u.DownVotes) AS TotalDownVotes,
// COUNT(b.Id) AS BadgeCount
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
// ps.AnswerCount,
// us.UserId,
// us.DisplayName,
// us.TotalViews,
// us.TotalUpVotes,
// us.TotalDownVotes,
// us.BadgeCount
// FROM
// PostStatistics ps
// JOIN
// UserStatistics us ON ps.PostId = us.UserId
// ORDER BY
// ps.Score DESC,
// ps.ViewCount DESC
// LIMIT 100;
fn q13016(db: &'static So) -> String {
    let uid = uids(db);
    let c = per_post_distinct(db, comments_of(db));
    let x = per_post_distinct(db, votes_of(db));
    let User { views, up_votes, down_votes, .. } = &db.user;
    let us = g(db).select(views.and(up_votes).and(down_votes).and(badges_of(db).opt())).fold([0i64; 4], |a, (((w, u), d), b)| [a[0] + w, a[1] + u, a[2] + d, a[3] + b.is_some() as i64]);
    let mut v = Vec::new();
    db.post
        .with((&db.post.origid).select(&uid))
        .select(Ident::<Post>::new().and((&c).opt()).and((&x).opt()).and(answers_per_post(db)).and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&us))))
        .drive(|_, y| v.push(y));
    out(v, |&((((p, _), _), _), _)| score_views(db, p), 100, |&((((p, c), x), a), (u, s))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(c.unwrap_or(0)), V::I(x.unwrap_or(0)), V::I(a), user_col(db, u, "uid"), user_col(db, u, "name")]);
        f.extend(ints(&s));
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
// p.FavoriteCount,
// u.DisplayName AS OwnerDisplayName,
// u.Reputation AS OwnerReputation,
// COUNT(v.Id) AS TotalVotes
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount, p.FavoriteCount, u.DisplayName, u.Reputation
// ),
// UserBadges AS (
// SELECT
// b.UserId,
// COUNT(b.Id) AS TotalBadges
// FROM
// Badges b
// GROUP BY
// b.UserId
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.Score,
// ps.ViewCount,
// ps.AnswerCount,
// ps.CommentCount,
// ps.FavoriteCount,
// ps.OwnerDisplayName,
// ps.OwnerReputation,
// ub.TotalBadges,
// ps.TotalVotes
// FROM
// PostStatistics ps
// LEFT JOIN
// UserBadges ub ON ps.OwnerDisplayName = (SELECT DisplayName FROM Users WHERE Id = ub.UserId)
// ORDER BY
// ps.Score DESC, ps.ViewCount DESC;
fn q13022(db: &'static So) -> String {
    let names = by_name(db);
    let bf = db.badge.group_by(&db.badge.user).fold(0i64, |a, _| a + 1);
    let mut v = Vec::new();
    owned(db).select(Ident::<Post>::new().and(votes_per_post(db)).and((&db.post.owner_user).select(&db.user.display_name).select(&names).select(&bf).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((p, x), b)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "favorites", "owner", "rep"]);
        f.extend([oint(b), V::I(x)]);
        row(f)
    }))
}

// SELECT
// COUNT(DISTINCT p.Id) AS TotalPosts,
// COUNT(DISTINCT c.Id) AS TotalComments,
// COUNT(DISTINCT v.Id) AS TotalVotes,
// COUNT(DISTINCT u.Id) AS TotalUsers,
// AVG(post_votes.votes_per_post) AS AvgVotesPerPost,
// AVG(post_comments.comments_per_post) AS AvgCommentsPerPost
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// JOIN
// (SELECT
// PostId,
// COUNT(*) AS votes_per_post
// FROM
// Votes
// GROUP BY
// PostId) AS post_votes ON p.Id = post_votes.PostId
// JOIN
// (SELECT
// PostId,
// COUNT(*) AS comments_per_post
// FROM
// Comments
// GROUP BY
// PostId) AS post_comments ON p.Id = post_comments.PostId
// GROUP BY
// p.Id, post_votes.votes_per_post, post_comments.comments_per_post;
fn q13023(db: &'static So) -> String {
    let mut v = Vec::new();
    db.post.with(votes_of(db)).with(comments_of(db)).select(Ident::<Post>::new().and(votes_per_post(db)).and(comments_per_post(db))).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((p, x), c)| row(vec![V::I(1), V::I(c), V::I(x), V::I(db.post.owner_user.get(p).is_some() as i64), V::F(x as f64), V::F(c as f64)])))
}

// WITH PostSummary AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// p.AnswerCount,
// p.CommentCount,
// p.FavoriteCount,
// COALESCE(u.DisplayName, 'Community') AS OwnerDisplayName,
// pt.Name AS PostTypeName,
// COUNT(c.Id) AS TotalComments,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
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
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, p.CommentCount,
// p.FavoriteCount, u.DisplayName, pt.Name
// ),
// PostHistoryStats AS (
// SELECT
// ph.PostId,
// COUNT(ph.Id) AS TotalEdits,
// MAX(ph.CreationDate) AS LastEditedDate
// FROM
// PostHistory ph
// GROUP BY
// ph.PostId
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.Score,
// ps.ViewCount,
// ps.AnswerCount,
// ps.CommentCount,
// ps.FavoriteCount,
// ps.OwnerDisplayName,
// ps.PostTypeName,
// ps.TotalComments,
// ps.TotalUpVotes,
// ps.TotalDownVotes,
// COALESCE(phs.TotalEdits, 0) AS TotalEdits,
// phs.LastEditedDate
// FROM
// PostSummary ps
// LEFT JOIN
// PostHistoryStats phs ON ps.PostId = phs.PostId
// ORDER BY
// ps.Score DESC, ps.ViewCount DESC;
fn q13035(db: &'static So) -> String {
    let hf = db.post_history.group_by(&db.post_history.post).select(&db.post_history.creation_date).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let mut v = Vec::new();
    stats_fold(db, db.post.iq(), Ident::<Post>::new(), "cv", &[]).and((&hf).opt()).drive(|p, (s, h)| v.push((p, s, h)));
    rows(v.iter().map(|&(p, s, h)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "favorites"]);
        f.push(named_owner(db, p, "Community"));
        f.extend(post_fields(db, p, &["type"]));
        f.extend([V::I(s.cx), V::I(s.up), V::I(s.down), V::I(h.map_or(0, |h| h.0)), ots(h.map(|h| h.1))]);
        row(f)
    }))
}

// WITH PostMetrics AS (
// SELECT
// p.Id AS PostId,
// p.PostTypeId,
// p.CreationDate,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 1 THEN 1 ELSE 0 END), 0) AS AcceptedVoteCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.PostTypeId, p.CreationDate
// )
// SELECT
// pt.Name AS PostType,
// COUNT(pm.PostId) AS TotalPosts,
// AVG(pm.CommentCount) AS AvgCommentsPerPost,
// AVG(pm.VoteCount) AS AvgVotesPerPost,
// AVG(pm.UpVoteCount) AS AvgUpVotesPerPost,
// AVG(pm.DownVoteCount) AS AvgDownVotesPerPost,
// AVG(pm.AcceptedVoteCount) AS AvgAcceptedVotesPerPost
// FROM
// PostTypes pt
// LEFT JOIN
// PostMetrics pm ON pt.Id = pm.PostTypeId
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q13061(db: &'static So) -> String {
    let of_type: HashIdx<Id<PostType>, Id<Post>> = (&db.post.post_type).inv().collect();
    let pf = stats_fold(db, db.post.iq(), Ident::<Post>::new(), "cv", &[]);
    let f = db.post_type.group_by(&db.post_type.name).select((&of_type).select(&pf).opt()).fold([0i64; 6], |a, s| match s {
        Some(s) => [a[0] + 1, a[1] + s.cx, a[2] + s.vx, a[3] + s.up, a[4] + s.down, a[5] + s.by_vt[1]],
        None => a,
    });
    let mut v = Vec::new();
    (&f).drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&(k, a)| {
        let mut f = vec![V::S(k), V::I(a[0])];
        f.extend((1..6).map(|i| avg(a[i], a[0])));
        row(f)
    }))
}

// WITH PostStats AS (
// SELECT
// P.Id AS PostId,
// P.PostTypeId,
// P.CreationDate,
// COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
// COUNT(V.Id) AS VoteCount,
// AVG(U.Reputation) AS AvgUserReputation
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// LEFT JOIN
// Users U ON P.OwnerUserId = U.Id
// GROUP BY
// P.Id, P.PostTypeId, P.CreationDate
// )
// SELECT
// PT.Name AS PostType,
// COUNT(*) AS TotalPosts,
// AVG(CommentCount) AS AvgComments,
// AVG(VoteCount) AS AvgVotes,
// AVG(AvgUserReputation) AS AvgOwnerReputation
// FROM
// PostStats PS
// JOIN
// PostTypes PT ON PS.PostTypeId = PT.Id
// GROUP BY
// PT.Name
// ORDER BY
// TotalPosts DESC;
fn q13067(db: &'static So) -> String {
    let pf = stats_fold(db, db.post.iq(), Ident::<Post>::new(), "cv", &[]);
    let f = by_key(db.post.iq(), name(db), (&pf).and((&db.post.owner_user).select(&db.user.reputation).opt()), [0i64; 5], |a, (s, r)| {
        [a[0] + 1, a[1] + s.cx, a[2] + s.vx, a[3] + r.is_some() as i64, a[4] + r.unwrap_or(0)]
    });
    let mut v = Vec::new();
    (&f).drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), avg(a[2], a[0]), avg(a[4], a[3])])))
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// u.Id, u.Reputation
// ),
// PostHistoryStats AS (
// SELECT
// p.Id AS PostId,
// COUNT(ph.Id) AS EditCount,
// COUNT(DISTINCT ph.UserId) AS EditorCount
// FROM
// Posts p
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// GROUP BY
// p.Id
// )
// SELECT
// u.UserId,
// u.Reputation,
// u.PostCount,
// u.Questions,
// u.Answers,
// u.UpVotes,
// u.DownVotes,
// p.PostId,
// p.EditCount,
// p.EditorCount
// FROM
// UserStats u
// JOIN
// PostHistoryStats p ON u.UserId = p.PostId
// ORDER BY
// u.Reputation DESC, u.PostCount DESC;
fn q13074(db: &'static So) -> String {
    let uid = uids(db);
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let de = per_post_distinct(db, history_of(db).select(&db.post_history.user_id));
    let hc = history_per_post(db);
    let mut v = Vec::new();
    db.post
        .with((&db.post.origid).select(&uid))
        .select(Ident::<Post>::new().and(&hc).and((&de).opt()).and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&us).and((&dp).opt()))))
        .drive(|_, x| v.push(x));
    rows(v.iter().map(|&(((p, h), e), ((u, a), d))| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep"), V::I(d.unwrap_or(0)), V::I(a.q), V::I(a.a), V::I(a.up), V::I(a.down)];
        f.extend(post_fields(db, p, &["id"]));
        f.extend([V::I(h), V::I(e.unwrap_or(0))]);
        row(f)
    }))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AverageScore,
// COUNT(DISTINCT u.Id) AS TotalUsers,
// COUNT(v.Id) AS TotalVotes,
// AVG(COALESCE(c.CommentCount, 0)) AS AverageCommentsPerPost
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q13076(db: &'static So) -> String {
    let f = by_key(db.post.iq(), name(db), (&db.post.score).and(comments_per_post(db)).and(votes_of(db).opt()), [0i64; 4], |a, ((s, c), x)| {
        [a[0] + 1, a[1] + s, a[2] + x.is_some() as i64, a[3] + c]
    });
    let du = db.post.group_by(name(db)).select(&db.post.owner_user).count_distinct();
    let mut v = Vec::new();
    (&f).and((&du).opt()).drive(|k, (a, d)| v.push((k, a, d.unwrap_or(0))));
    rows(v.iter().map(|&(k, a, d)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), V::I(d), V::I(a[2]), avg(a[3], a[0])])))
}

// WITH PostEngagement AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate
// ),
// PostStatistics AS (
// SELECT
// pe.PostId,
// pe.Title,
// pe.CreationDate,
// pe.CommentCount,
// pe.VoteCount,
// pe.UpVotes,
// pe.DownVotes,
// CASE
// WHEN pe.UpVotes > pe.DownVotes THEN 'Positive'
// WHEN pe.DownVotes > pe.UpVotes THEN 'Negative'
// ELSE 'Neutral'
// END AS PostSentiment
// FROM
// PostEngagement pe
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.CommentCount,
// ps.VoteCount,
// ps.UpVotes,
// ps.DownVotes,
// ps.PostSentiment,
// (ps.UpVotes - ps.DownVotes) AS NetVotes
// FROM
// PostStatistics ps
// ORDER BY
// NetVotes DESC, ps.CreationDate DESC;
fn q13079(db: &'static So) -> String {
    rows(stats_with(db, db.post.iq(), "cv", &[], &[]).iter().map(|&(p, s, _)| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(s.cx), V::I(s.vx), V::I(s.up), V::I(s.down)]);
        f.push(V::S(if s.up > s.down { "Positive" } else if s.down > s.up { "Negative" } else { "Neutral" }));
        f.push(V::I(s.up - s.down));
        row(f)
    }))
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
// p.FavoriteCount,
// u.DisplayName AS OwnerDisplayName,
// u.Reputation AS OwnerReputation,
// COALESCE(v.UpVotes, 0) AS UpVotes,
// COALESCE(v.DownVotes, 0) AS DownVotes,
// p.OwnerUserId
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN (
// SELECT
// PostId,
// SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Votes
// GROUP BY
// PostId
// ) v ON p.Id = v.PostId
// ),
// UserBadges AS (
// SELECT
// UserId,
// COUNT(*) AS BadgeCount
// FROM
// Badges
// GROUP BY
// UserId
// ),
// PostHistoryStats AS (
// SELECT
// Ph.PostId,
// COUNT(*) AS HistoryCount,
// MAX(CreationDate) AS LastModified
// FROM
// PostHistory Ph
// GROUP BY
// Ph.PostId
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.Score,
// ps.ViewCount,
// ps.AnswerCount,
// ps.CommentCount,
// ps.FavoriteCount,
// ps.OwnerDisplayName,
// ps.OwnerReputation,
// ub.BadgeCount,
// phs.HistoryCount,
// phs.LastModified,
// ps.UpVotes,
// ps.DownVotes
// FROM
// PostStats ps
// LEFT JOIN
// UserBadges ub ON ps.OwnerUserId = ub.UserId
// LEFT JOIN
// PostHistoryStats phs ON ps.PostId = phs.PostId
// ORDER BY
// ps.ViewCount DESC,
// ps.Score DESC
// LIMIT 100;
fn q13082(db: &'static So) -> String {
    let pv = post_votes(db);
    let bf = db.badge.group_by(&db.badge.user).fold(0i64, |a, _| a + 1);
    let hf = db.post_history.group_by(&db.post_history.post).select(&db.post_history.creation_date).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let mut v = Vec::new();
    owned(db).select(Ident::<Post>::new().and((&pv).opt()).and((&db.post.owner_user).select(&bf).opt()).and((&hf).opt())).drive(|_, x| v.push(x));
    out(v, |&(((p, _), _), _)| views_score(db, p), 100, |&(((p, x), b), h)| {
        let x = x.unwrap_or([0; 3]);
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "favorites", "owner", "rep"]);
        f.extend([oint(b), oint(h.map(|h| h.0)), ots(h.map(|h| h.1)), V::I(x[1]), V::I(x[2])]);
        f
    })
}

// WITH UserPostMetrics AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(P.Id) AS PostCount,
// AVG(P.Score) AS AveragePostScore,
// SUM(CASE WHEN C.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalComments
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// GROUP BY
// U.Id, U.DisplayName, U.Reputation
// )
// SELECT
// UserId,
// DisplayName,
// Reputation,
// PostCount,
// AveragePostScore,
// TotalComments
// FROM
// UserPostMetrics
// ORDER BY
// Reputation DESC, PostCount DESC
// LIMIT 10;
fn q13096(db: &'static So) -> String {
    out(users_with_counts(db, "c", false), |r| (Reverse(r.rep), Reverse(r.agg.prows)), 10, |r| user_fields(r, "c", &["uid", "name", "rep", "#rows", "score_avg", "#cx"]))
}

// WITH UserActivity AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// COUNT(DISTINCT c.Id) AS TotalComments,
// SUM(CASE WHEN v.CreationDate IS NOT NULL THEN 1 ELSE 0 END) AS TotalVotes,
// SUM(CASE WHEN b.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalBadges,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers
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
// ua.TotalComments,
// ua.TotalVotes,
// ua.TotalBadges,
// ua.TotalQuestions,
// ua.TotalAnswers,
// COALESCE(NULLIF(ua.TotalQuestions, 0), 1) AS SafeQuestionCount,
// ROUND((ua.TotalPosts * 1.0 / NULLIF(SafeQuestionCount, 0)), 2) AS PostsPerQuestion
// FROM
// UserActivity ua
// ORDER BY
// ua.TotalPosts DESC
// LIMIT 100;
fn q13099(db: &'static So) -> String {
    out(users_with_counts(db, "cvb", false), |r| Reverse(r.agg.n), 100, |r| {
        let a = r.agg;
        let safe = if a.questions == 0 { 1 } else { a.questions };
        let mut f = user_fields(r, "cvb", &["uid", "name", "#n", "#c", "#vx", "#bx", "#q", "#a"]);
        f.extend([V::I(safe), V::F(round2(a.n as f64 / safe as f64))]);
        f
    })
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.PostTypeId,
// COUNT(c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
// COUNT(DISTINCT bh.Id) AS EditCount,
// MAX(p.CreationDate) AS PostCreationDate,
// MAX(p.LastActivityDate) AS LastActivityDate
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// PostHistory bh ON p.Id = bh.PostId
// WHERE
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY
// p.Id, p.Title, p.PostTypeId
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.PostTypeId,
// ps.CommentCount,
// ps.VoteCount,
// ps.EditCount,
// ps.PostCreationDate,
// ps.LastActivityDate,
// EXTRACT(EPOCH FROM (ps.LastActivityDate - ps.PostCreationDate)) AS ActivityDuration
// FROM
// PostStats ps
// ORDER BY
// ps.LastActivityDate DESC
// LIMIT 100;
fn q13106(db: &'static So) -> String {
    let x = per_post_distinct(db, votes_of(db));
    let h = per_post_distinct(db, history_of(db));
    out(stats_with(db, since(db, year_ago()), "cvh", &[], &[&x, &h]), |&(p, _, _)| Reverse(db.post.last_activity_date.get(p).unwrap()), 100, |&(p, s, d)| {
        let (cd, la) = (db.post.creation_date.get(p).unwrap(), db.post.last_activity_date.get(p).unwrap());
        let mut f = post_fields(db, p, &["id", "title", "type_id"]);
        f.extend([V::I(s.cx), V::I(d[0]), V::I(d[1]), V::T(cd), V::T(la), V::F(hours_to(la, cd))]);
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
// COALESCE(SUM(v.BountyAmount), 0) AS TotalBountyAmount,
// COUNT(DISTINCT b.Id) AS BadgeCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId AND v.VoteTypeId = 8
// LEFT JOIN
// Badges b ON p.OwnerUserId = b.UserId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount
// ),
// UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(COALESCE(v.BountyAmount, 0)) AS TotalBountyAmount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Votes v ON p.Id = v.PostId AND v.VoteTypeId = 8
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.Score,
// ps.ViewCount,
// ps.CommentCount,
// ps.TotalBountyAmount,
// us.UserId,
// us.DisplayName AS OwnerDisplayName,
// us.Reputation AS OwnerReputation,
// us.PostCount AS OwnerPostCount,
// us.TotalBountyAmount AS OwnerTotalBounty
// FROM
// PostStats ps
// JOIN
// Users u ON ps.PostId = u.Id
// JOIN
// UserStats us ON u.Id = us.UserId
// ORDER BY
// ps.Score DESC, ps.ViewCount DESC;
fn q13125(db: &'static So) -> String {
    let uid = uids(db);
    let b = per_post_distinct(db, (&db.post.owner_user).select(badges_of(db)));
    let us = user_stats_fold_v(db, Ident::<User>::new(), UserWhere::All, "v", any_post, &[8]);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let mut v = Vec::new();
    stats_fold(db, db.post.with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cvb", &[8])
        .and((&b).opt())
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&us).and((&dp).opt())))
        .drive(|p, ((s, _), ((u, a), d))| v.push((p, s, u, a, d.unwrap_or(0))));
    rows(v.iter().map(|&(p, s, u, a, d)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(s.cx), V::I(s.bounty_sum), user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep"), V::I(d), V::I(a.bounty_sum)]);
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
// AVG(voteTypeValue) AS AverageVoteType
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// (SELECT Id, VoteTypeId,
// CASE
// WHEN VoteTypeId = 1 THEN 1
// WHEN VoteTypeId = 2 THEN 1
// WHEN VoteTypeId = 3 THEN -1
// ELSE 0
// END AS voteTypeValue
// FROM Votes) vType ON v.Id = vType.Id
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q13136(db: &'static So) -> String {
    let mut v = Vec::new();
    stats_fold(db, questions_only(db), Ident::<Post>::new(), "cv", &[]).drive(|p, s| v.push((p, s)));
    out(v, |&(p, _)| newest(db, p), 100, |&(p, s)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "owner"]);
        f.extend([V::I(s.cx), V::I(s.vx), avg(s.by_vt[1] + s.by_vt[2] - s.by_vt[3], s.vx)]);
        f
    })
}

// WITH PostStatistics AS (
// SELECT
// Posts.Id AS PostId,
// Posts.Title,
// Posts.CreationDate,
// Posts.Score,
// Posts.ViewCount,
// COALESCE(Users.DisplayName, 'Community User') AS Owner,
// COUNT(Comments.Id) AS TotalComments,
// COUNT(CASE WHEN Votes.VoteTypeId = 2 THEN 1 END) AS TotalUpVotes,
// COUNT(CASE WHEN Votes.VoteTypeId = 3 THEN 1 END) AS TotalDownVotes
// FROM
// Posts
// LEFT JOIN
// Users ON Posts.OwnerUserId = Users.Id
// LEFT JOIN
// Comments ON Posts.Id = Comments.PostId
// LEFT JOIN
// Votes ON Posts.Id = Votes.PostId
// GROUP BY
// Posts.Id, Posts.Title, Posts.CreationDate, Posts.Score, Posts.ViewCount, Users.DisplayName
// )
// SELECT
// PostId,
// Title,
// CreationDate,
// Score,
// ViewCount,
// Owner,
// TotalComments,
// TotalUpVotes,
// TotalDownVotes
// FROM
// PostStatistics
// ORDER BY
// Score DESC, ViewCount DESC
// LIMIT 100;
fn q13147(db: &'static So) -> String {
    out(stats_with(db, db.post.iq(), "cv", &[], &[]), |&(p, _, _)| score_views(db, p), 100, |&(p, s, _)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([named_owner(db, p, "Community User"), V::I(s.cx), V::I(s.up), V::I(s.down)]);
        f
    })
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN p.PostTypeId = 3 THEN 1 ELSE 0 END) AS TotalWikis,
// AVG(p.Score) AS AverageScore,
// AVG(p.ViewCount) AS AverageViewCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName
// ),
// UserVoteStats AS (
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
// ups.UserId,
// ups.DisplayName,
// ups.TotalPosts,
// ups.TotalQuestions,
// ups.TotalAnswers,
// ups.TotalWikis,
// ups.AverageScore,
// ups.AverageViewCount,
// uvs.TotalVotes,
// uvs.TotalUpVotes,
// uvs.TotalDownVotes
// FROM
// UserPostStats ups
// LEFT JOIN
// UserVoteStats uvs ON ups.UserId = uvs.UserId
// ORDER BY
// ups.TotalPosts DESC
// LIMIT 100;
fn q13150(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let uf = g(db).select(posts_of(db).select(post_type_id.and(score).and(view_count.opt())).opt()).fold([0i64; 7], |a, p| match p {
        Some(((t, s), w)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (t == 3) as i64, a[4] + s, a[5] + w.is_some() as i64, a[6] + w.unwrap_or(0)],
        None => a,
    });
    let vs = vote_named(db);
    let mut v = Vec::new();
    (&uf).and((&vs).opt()).drive(|u, (a, x)| v.push((u, a, x)));
    out(v, |&(_, a, _)| Reverse(a[0]), 100, |&(u, a, x)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[..4]));
        f.extend([avg(a[4], a[0]), avg(a[6], a[5])]);
        f.extend((0..3).map(|i| oint(x.map(|x| x[i]))));
        f
    })
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate AS PostCreationDate,
// p.ViewCount,
// p.Score,
// p.AnswerCount,
// p.CommentCount,
// u.Reputation AS OwnerReputation,
// u.DisplayName AS OwnerDisplayName,
// COALESCE(v.VoteCount, 0) AS VoteCount,
// COUNT(DISTINCT c.Id) AS CommentCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// (SELECT
// PostId,
// COUNT(*) AS VoteCount
// FROM
// Votes
// GROUP BY
// PostId) v ON p.Id = v.PostId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// WHERE
// p.CreationDate >= '2022-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, p.CommentCount,
// u.Id, u.Reputation, u.DisplayName, v.VoteCount
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q13167(db: &'static So) -> String {
    let vf = db.vote.group_by(&db.vote.post).fold(0i64, |a, _| a + 1);
    let mut v = Vec::new();
    owned_since(db, date(2022, 1, 1)).select(Ident::<Post>::new().and((&vf).opt()).and(comments_per_post(db))).drive(|_, x| v.push(x));
    out(v, |&((p, _), _)| newest(db, p), 100, |&((p, x), c)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "answers", "comments", "rep", "owner"]);
        f.extend([V::I(x.unwrap_or(0)), V::I(c)]);
        f
    })
}

// WITH UserVoteCounts AS (
// SELECT
// UserId,
// COUNT(CASE WHEN VoteTypeId = 2 THEN 1 END) AS UpVotes,
// COUNT(CASE WHEN VoteTypeId = 3 THEN 1 END) AS DownVotes,
// COUNT(*) AS TotalVotes
// FROM Votes
// GROUP BY UserId
// ),
// PostStatistics AS (
// SELECT
// P.Id AS PostId,
// P.OwnerUserId,
// P.Score,
// P.ViewCount,
// COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
// COUNT(CASE WHEN PH.PostId IS NOT NULL THEN 1 END) AS HistoryCount
// FROM Posts P
// LEFT JOIN Comments C ON P.Id = C.PostId
// LEFT JOIN PostHistory PH ON P.Id = PH.PostId
// GROUP BY P.Id, P.OwnerUserId, P.Score, P.ViewCount
// )
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// UV.UpVotes,
// UV.DownVotes,
// UV.TotalVotes,
// PS.PostId,
// PS.Score,
// PS.ViewCount,
// PS.CommentCount,
// PS.HistoryCount
// FROM Users U
// LEFT JOIN UserVoteCounts UV ON U.Id = UV.UserId
// LEFT JOIN PostStatistics PS ON U.Id = PS.OwnerUserId
// ORDER BY U.Reputation DESC, PS.Score DESC;
fn q13176(db: &'static So) -> String {
    let uv = db.vote.group_by(&db.vote.user).select(&db.vote.vote_type_id).fold([0i64; 3], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + 1]);
    let ps = stats_fold(db, owned(db), Ident::<Post>::new(), "ch", &[]);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&uv).opt()).and(posts_of(db).select(Ident::<Post>::new().and(&ps)).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((u, x), p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend((0..3).map(|i| oint(x.map(|x| x[i]))));
        match p {
            Some((p, s)) => {
                f.extend(post_fields(db, p, &["id", "score", "views"]));
                f.extend([V::I(s.cx), V::I(s.hx)]);
            }
            None => f.extend(nulls(5)),
        }
        row(f)
    }))
}

// WITH UserActivity AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(P.ViewCount) AS TotalViews,
// SUM(P.Score) AS TotalScore
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id, U.DisplayName
// ),
// BadgeCounts AS (
// SELECT
// B.UserId,
// COUNT(B.Id) AS TotalBadges
// FROM
// Badges B
// GROUP BY
// B.UserId
// ),
// VotingActivity AS (
// SELECT
// V.UserId,
// COUNT(V.Id) AS TotalVotes,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Votes V
// GROUP BY
// V.UserId
// )
// SELECT
// UA.UserId,
// UA.DisplayName,
// UA.TotalPosts,
// UA.Questions,
// UA.Answers,
// UA.TotalViews,
// UA.TotalScore,
// COALESCE(BC.TotalBadges, 0) AS TotalBadges,
// COALESCE(VA.TotalVotes, 0) AS TotalVotes,
// COALESCE(VA.UpVotes, 0) AS UpVotes,
// COALESCE(VA.DownVotes, 0) AS DownVotes
// FROM
// UserActivity UA
// LEFT JOIN
// BadgeCounts BC ON UA.UserId = BC.UserId
// LEFT JOIN
// VotingActivity VA ON UA.UserId = VA.UserId
// ORDER BY
// UA.TotalScore DESC, UA.TotalPosts DESC;
fn q13179(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let va = db.vote.group_by(&db.vote.user).select(&db.vote.vote_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64]);
    let mut v = Vec::new();
    upqa(db).and(&bu).and((&va).opt()).drive(|u, ((a, b), x)| v.push((u, a, b, x)));
    rows(v.iter().map(|&(u, a, b, x)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[..3]));
        f.extend([nullable(a[4], a[3]), nullable(a[5], a[0]), V::I(b)]);
        f.extend(ints(&x.unwrap_or([0; 3])));
        row(f)
    }))
}

// WITH UserPostCount AS (
// SELECT
// u.Id AS UserId,
// COUNT(p.Id) AS PostCount,
// SUM(u.UpVotes) AS TotalUpVotes,
// SUM(u.DownVotes) AS TotalDownVotes,
// AVG(u.Reputation) AS AverageReputation
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id
// ),
// PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// COALESCE(c.CommentCount, 0) AS CommentCount,
// COALESCE(b.BadgeCount, 0) AS BadgeCount,
// p.OwnerUserId
// FROM
// Posts p
// LEFT JOIN (
// SELECT
// PostId,
// COUNT(Id) AS CommentCount
// FROM
// Comments
// GROUP BY
// PostId
// ) c ON p.Id = c.PostId
// LEFT JOIN (
// SELECT
// UserId,
// COUNT(b.Id) AS BadgeCount
// FROM
// Badges b
// GROUP BY
// UserId
// ) b ON p.OwnerUserId = b.UserId
// )
// SELECT
// upc.UserId,
// upc.PostCount,
// upc.TotalUpVotes,
// upc.TotalDownVotes,
// upc.AverageReputation,
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.Score,
// ps.ViewCount,
// ps.CommentCount,
// ps.BadgeCount
// FROM
// UserPostCount upc
// JOIN
// PostStats ps ON upc.UserId = ps.OwnerUserId
// ORDER BY
// upc.PostCount DESC, upc.AverageReputation DESC;
fn q13180(db: &'static So) -> String {
    let User { up_votes, down_votes, .. } = &db.user;
    let uf = g(db).select(up_votes.and(down_votes).and(posts_of(db).opt())).fold([0i64; 3], |a, ((u, d), p)| [a[0] + p.is_some() as i64, a[1] + u, a[2] + d]);
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    owned(db).select(Ident::<Post>::new().and(comments_per_post(db)).and((&db.post.owner_user).select(Ident::<User>::new().and(&uf).and(&bu)))).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((p, c), ((u, a), b))| {
        let mut f = vec![user_col(db, u, "uid"), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::F(db.user.reputation.get(u).unwrap() as f64)];
        f.extend(post_fields(db, p, &["id", "title", "created", "score", "views"]));
        f.extend([V::I(c), V::I(b)]);
        row(f)
    }))
}

// WITH PostStatistics AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// COUNT(c.Id) AS CommentCount,
// COALESCE(p2.Id, -1) AS AcceptedAnswerId
// FROM
// Posts p
// LEFT JOIN
// Comments c ON c.PostId = p.Id
// LEFT JOIN
// Posts p2 ON p.AcceptedAnswerId = p2.Id
// WHERE
// p.CreationDate >= DATE '2023-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p2.Id
// ),
// UserEngagement AS (
// SELECT
// u.Id AS UserId,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM
// Users u
// LEFT JOIN
// Votes v ON v.UserId = u.Id
// LEFT JOIN
// Badges b ON b.UserId = u.Id
// GROUP BY
// u.Id
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.Score,
// ps.ViewCount,
// ps.CommentCount,
// ue.UserId,
// ue.UpVotes,
// ue.DownVotes,
// ue.GoldBadges,
// ue.SilverBadges,
// ue.BronzeBadges
// FROM
// PostStatistics ps
// LEFT JOIN
// UserEngagement ue ON ue.UserId = ps.AcceptedAnswerId
// ORDER BY
// ps.Score DESC, ps.ViewCount DESC;
fn q13181(db: &'static So) -> String {
    let uid = uids(db);
    let ue = g(db).select(votes_by(db).select(&db.vote.vote_type_id).opt().and(badges_of(db).select(&db.badge.class).opt())).fold([0i64; 5], |a, (t, c)| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + (c == Some(1)) as i64, a[3] + (c == Some(2)) as i64, a[4] + (c == Some(3)) as i64]
    });
    let acc = (&db.post.accepted_answer).select(&db.post.origid).opt().map(|a: Option<i64>| a.unwrap_or(-1));
    let mut v = Vec::new();
    since(db, date(2023, 1, 1)).select(Ident::<Post>::new().and(comments_per_post(db)).and(acc.select(&uid).select(Ident::<User>::new().and(&ue)).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((p, c), e)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.push(V::I(c));
        match e {
            Some((u, a)) => {
                f.push(user_col(db, u, "uid"));
                f.extend(ints(&a));
            }
            None => f.extend(nulls(6)),
        }
        row(f)
    }))
}

// WITH UserPostCounts AS (
// SELECT
// u.Id AS UserId,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id
// ),
// UserBadgeCounts AS (
// SELECT
// b.UserId,
// COUNT(b.Id) AS BadgeCount
// FROM
// Badges b
// GROUP BY
// b.UserId
// ),
// UserVoteCounts AS (
// SELECT
// v.UserId,
// COUNT(v.Id) AS VoteCount
// FROM
// Votes v
// GROUP BY
// v.UserId
// )
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COALESCE(up.PostCount, 0) AS TotalPosts,
// COALESCE(up.QuestionCount, 0) AS TotalQuestions,
// COALESCE(up.AnswerCount, 0) AS TotalAnswers,
// COALESCE(ub.BadgeCount, 0) AS TotalBadges,
// COALESCE(uv.VoteCount, 0) AS TotalVotes,
// u.Reputation,
// u.CreationDate
// FROM
// Users u
// LEFT JOIN
// UserPostCounts up ON u.Id = up.UserId
// LEFT JOIN
// UserBadgeCounts ub ON u.Id = ub.UserId
// LEFT JOIN
// UserVoteCounts uv ON u.Id = uv.UserId
// ORDER BY
// u.Reputation DESC;
fn q13185(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let vu = votes_per_user(db);
    let mut v = Vec::new();
    upqa(db).and(&bu).and(&vu).drive(|u, ((a, b), x)| v.push((u, a, b, x)));
    rows(v.iter().map(|&(u, a, b, x)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[..3]));
        f.extend([V::I(b), V::I(x), user_col(db, u, "rep"), user_col(db, u, "ucreated")]);
        row(f)
    }))
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpVotes,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownVotes,
// AVG(CASE WHEN p.ViewCount IS NOT NULL THEN p.ViewCount ELSE 0 END) AS AvgViewCount,
// AVG(CASE WHEN p.Score IS NOT NULL THEN p.Score ELSE 0 END) AS AvgScore
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// u.Reputation > 0
// GROUP BY
// u.Id, u.DisplayName
// ORDER BY
// PostCount DESC
// LIMIT 100;
fn q13190(db: &'static So) -> String {
    let Post { post_type_id, view_count, score, .. } = &db.post;
    let w = UserWhere::RepGt(0);
    let uf = user_base(db, w)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(view_count.opt()).and(score).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt())
        .fold([0i64; 7], |a, p| {
            let mut a = [a[0] + 1, a[1], a[2], a[3], a[4], a[5], a[6]];
            if let Some((((t, vw), s), x)) = p {
                a[1] += (t == 1) as i64;
                a[2] += (t == 2) as i64;
                a[3] += (x == Some(2)) as i64;
                a[4] += (x == Some(3)) as i64;
                a[5] += vw.unwrap_or(0);
                a[6] += s;
            }
            a
        });
    let dp = ud(db, w, posts_of(db));
    let mut v = Vec::new();
    (&uf).and((&dp).opt()).drive(|u, (a, d)| v.push((u, a, d.unwrap_or(0))));
    out(v, |&(_, _, d)| Reverse(d), 100, |&(u, a, d)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(d)];
        f.extend(ints(&a[1..5]));
        f.extend([avg(a[5], a[0]), avg(a[6], a[0])]);
        f
    })
}

// WITH PostStats AS (
// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(CASE WHEN p.Score IS NOT NULL THEN p.Score ELSE 0 END) AS AverageScore,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers
// FROM
// Posts p
// INNER JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// GROUP BY
// pt.Name
// ),
// UserStats AS (
// SELECT
// COUNT(u.Id) AS TotalUsers,
// AVG(u.Reputation) AS AverageReputation
// FROM
// Users u
// )
// SELECT
// ps.PostType,
// ps.TotalPosts,
// ps.AverageScore,
// ps.TotalQuestions,
// ps.TotalAnswers,
// us.TotalUsers,
// us.AverageReputation
// FROM
// PostStats ps,
// UserStats us
// ORDER BY
// ps.TotalPosts DESC;
fn q13197(db: &'static So) -> String {
    let f = by_key(db.post.iq(), name(db), (&db.post.score).and(&db.post.post_type_id), [0i64; 4], |a, (s, t)| [a[0] + 1, a[1] + s, a[2] + (t == 1) as i64, a[3] + (t == 2) as i64]);
    let (un, rs) = db.user.select(&db.user.reputation).fold_flat((0i64, 0i64), |(n, s), r| (n + 1, s + r));
    let mut v = Vec::new();
    (&f).drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), V::I(a[2]), V::I(a[3]), V::I(un), avg(rs, un)])))
}

// WITH PostStats AS (
// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS PostCount,
// AVG(p.Score) AS AvgScore,
// SUM(p.ViewCount) AS TotalViews,
// AVG(p.AnswerCount) AS AvgAnswerCount,
// AVG(p.CommentCount) AS AvgCommentCount
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// GROUP BY
// pt.Name
// ),
// UserStats AS (
// SELECT
// u.DisplayName,
// COUNT(b.Id) AS BadgeCount,
// MAX(u.Reputation) AS MaxReputation,
// AVG(u.Reputation) AS AvgReputation
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// u.DisplayName
// )
// SELECT
// ps.PostType,
// ps.PostCount,
// ps.AvgScore,
// ps.TotalViews,
// ps.AvgAnswerCount,
// ps.AvgCommentCount,
// us.DisplayName AS UserWithMostBadges,
// us.BadgeCount,
// us.MaxReputation,
// us.AvgReputation
// FROM
// PostStats ps
// CROSS JOIN
// (SELECT
// DisplayName,
// BadgeCount,
// MaxReputation,
// AvgReputation
// FROM
// UserStats
// ORDER BY
// BadgeCount DESC
// LIMIT 1) us
// ORDER BY
// ps.PostCount DESC;
fn q13201(db: &'static So) -> String {
    let Post { score, view_count, answer_count, comment_count, .. } = &db.post;
    let f = by_key(db.post.iq(), name(db), score.and(view_count.opt()).and(answer_count.opt()).and(comment_count), [0i64; 7], |a, (((s, w), an), cc)| {
        [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + an.is_some() as i64, a[5] + an.unwrap_or(0), a[6] + cc]
    });
    let us = db.user.group_by(&db.user.display_name).select((&db.user.reputation).and(badges_of(db).opt())).fold([0, i64::MIN, 0, 0], |a: [i64; 4], (r, b)| [a[0] + b.is_some() as i64, a[1].max(r), a[2] + r, a[3] + 1]);
    let mut top = Vec::new();
    whole(&us).select(Same::new().and(&us)).window(row_number, |(_, a)| a[0], desc).filt(|(_, n)| n <= 1).drive(|_, ((k, a), _)| top.push((k, a)));
    let (tn, ta) = top[0];
    let mut v = Vec::new();
    (&f).drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&(k, a)| {
        row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), nullable(a[3], a[2]), avg(a[5], a[4]), avg(a[6], a[0]), V::S(tn), V::I(ta[0]), V::I(ta[1]), avg(ta[2], ta[3])])
    }))
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COALESCE(p.TotalPosts, 0) AS TotalPosts,
// COALESCE(c.TotalComments, 0) AS TotalComments,
// COALESCE(b.TotalBadges, 0) AS TotalBadges
// FROM
// Users u
// LEFT JOIN (
// SELECT
// OwnerUserId,
// COUNT(*) AS TotalPosts
// FROM
// Posts
// GROUP BY
// OwnerUserId
// ) p ON u.Id = p.OwnerUserId
// LEFT JOIN (
// SELECT
// UserId,
// COUNT(*) AS TotalComments
// FROM
// Comments
// GROUP BY
// UserId
// ) c ON u.Id = c.UserId
// LEFT JOIN (
// SELECT
// UserId,
// COUNT(*) AS TotalBadges
// FROM
// Badges
// GROUP BY
// UserId
// ) b ON u.Id = b.UserId
// ORDER BY
// u.Reputation DESC
// LIMIT 10;
fn q13203(db: &'static So) -> String {
    let pc = (&db.post.owner_user).inv().dense_fold_outer(db.user.id.n, 0i64, |a, _| a + 1);
    let cu = comments_per_user(db);
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and(&pc).and(&cu).and(&bu)).drive(|_, x| v.push(x));
    out(v, |&(((u, _), _), _)| rep_desc(db, u), 10, |&(((u, p), c), b)| vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep"), V::I(p), V::I(c), V::I(b)])
}

pub static ENTRIES: &[harness::Entry] = &[
    ("12807", q12807),
    ("12815", q12815),
    ("12818", q12818),
    ("12826", q12826),
    ("12827", q12827),
    ("12850", q12850),
    ("12875", q12875),
    ("12880", q12880),
    ("12881", q12881),
    ("12883", q12883),
    ("12885", q12885),
    ("12902", q12902),
    ("12906", q12906),
    ("12912", q12912),
    ("12920", q12920),
    ("12923", q12923),
    ("12926", q12926),
    ("12929", q12929),
    ("12957", q12957),
    ("12961", q12961),
    ("12965", q12965),
    ("12969", q12969),
    ("12972", q12972),
    ("12980", q12980),
    ("12983", q12983),
    ("12993", q12993),
    ("12994", q12994),
    ("13015", q13015),
    ("13016", q13016),
    ("13022", q13022),
    ("13023", q13023),
    ("13035", q13035),
    ("13061", q13061),
    ("13067", q13067),
    ("13074", q13074),
    ("13076", q13076),
    ("13079", q13079),
    ("13082", q13082),
    ("13096", q13096),
    ("13099", q13099),
    ("13106", q13106),
    ("13125", q13125),
    ("13136", q13136),
    ("13147", q13147),
    ("13150", q13150),
    ("13167", q13167),
    ("13176", q13176),
    ("13179", q13179),
    ("13180", q13180),
    ("13181", q13181),
    ("13185", q13185),
    ("13190", q13190),
    ("13197", q13197),
    ("13201", q13201),
    ("13203", q13203),
];
