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

fn self_votes(db: &'static So) -> HashIdx<Id<Post>, Id<Vote>> {
    let Vote { user, post, .. } = &db.vote;
    db.vote.with(user.and(post.select(&db.post.owner_user)).filt(|(a, b)| a == b)).select(post).inv().collect()
}

fn history_n_max(db: &'static So) -> Fold<Id<Post>, (i64, i64)> {
    db.post_history.group_by(&db.post_history.post).select(&db.post_history.creation_date).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)))
}

// --- batch 127 --------------------------------------------------------------

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.OwnerUserId,
// p.Score,
// p.ViewCount,
// COUNT(DISTINCT c.Id) AS CommentCount,
// COUNT(DISTINCT a.Id) AS AnswerCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Posts a ON p.Id = a.ParentId
// WHERE
// p.PostTypeId IN (1, 2)
// GROUP BY
// p.Id, p.OwnerUserId, p.Score, p.ViewCount
// ),
// UserStats AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// COUNT(DISTINCT b.Id) AS BadgeCount,
// SUM(ps.Score) AS TotalScore,
// SUM(ps.ViewCount) AS TotalViews,
// SUM(ps.CommentCount) AS TotalComments,
// SUM(ps.AnswerCount) AS TotalAnswers
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// LEFT JOIN
// PostStats ps ON u.Id = ps.OwnerUserId
// GROUP BY
// u.Id, u.Reputation
// )
// SELECT
// u.UserId,
// u.Reputation,
// u.BadgeCount,
// u.TotalScore,
// u.TotalViews,
// u.TotalComments,
// u.TotalAnswers
// FROM
// UserStats u
// ORDER BY
// u.TotalScore DESC, u.Reputation DESC;
fn q14814(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let an = per_post_distinct(db, children_of(db));
    let ps = Ident::<Post>::new().with(post_type_id.in_v(vec![1, 2])).select(score.and(view_count.opt()).and(comments_per_post(db)).and((&an).opt()));
    let uf = g(db).select(badges_of(db).opt().and(posts_of(db).select(ps).opt())).fold([0i64; 6], |a, (_, p)| match p {
        Some((((s, w), c), n)) => [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + c, a[5] + n.unwrap_or(0)],
        None => a,
    });
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    (&uf).and(&bu).drive(|u, (a, b)| v.push((u, a, b)));
    rows(v.iter().map(|&(u, a, b)| row(vec![user_col(db, u, "uid"), user_col(db, u, "rep"), V::I(b), nullable(a[1], a[0]), nullable(a[3], a[2]), nullable(a[4], a[0]), nullable(a[5], a[0])])))
}

// WITH EnhancedPostStatistics AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.Score,
// P.ViewCount,
// COALESCE(COUNT(Cm.Id), 0) AS CommentCount,
// COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount,
// COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount,
// COALESCE(COUNT(H.Id), 0) AS EditCount
// FROM
// Posts P
// LEFT JOIN
// Comments Cm ON P.Id = Cm.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// LEFT JOIN
// PostHistory H ON P.Id = H.PostId
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.Score, P.ViewCount
// )
// SELECT
// EPS.PostId,
// EPS.Title,
// EPS.CreationDate,
// EPS.Score,
// EPS.ViewCount,
// EPS.CommentCount,
// EPS.UpVoteCount,
// EPS.DownVoteCount,
// EPS.EditCount,
// PT.Name AS PostTypeName
// FROM
// EnhancedPostStatistics EPS
// JOIN
// PostTypes PT ON EPS.PostId IN (
// SELECT Id FROM Posts WHERE PostTypeId = PT.Id
// )
// ORDER BY
// EPS.Score DESC, EPS.ViewCount DESC
// LIMIT 100;
fn q14816(db: &'static So) -> String {
    let mut v = Vec::new();
    stats_fold(db, db.post.iq(), Ident::<Post>::new(), "cvh", &[]).drive(|p, s| v.push((p, s)));
    out(v, |&(p, _)| score_views(db, p), 100, |&(p, s)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(s.cx), V::I(s.up), V::I(s.down), V::I(s.hx)]);
        f.extend(post_fields(db, p, &["type"]));
        f
    })
}

// WITH PostStats AS (
// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AvgScore,
// SUM(p.ViewCount) AS TotalViews,
// COUNT(DISTINCT p.OwnerUserId) AS UniquePostOwners,
// COUNT(DISTINCT c.Id) AS TotalComments
// FROM
// Posts p
// LEFT JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// GROUP BY
// pt.Name
// ),
// UserStats AS (
// SELECT
// u.DisplayName,
// u.Reputation,
// COUNT(b.Id) AS TotalBadges,
// SUM(COALESCE(p.ViewCount, 0)) AS TotalViewsByUser
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.DisplayName, u.Reputation
// )
// SELECT
// ps.PostType,
// ps.TotalPosts,
// ps.AvgScore,
// ps.TotalViews,
// ps.UniquePostOwners,
// ps.TotalComments,
// us.DisplayName,
// us.Reputation,
// us.TotalBadges,
// us.TotalViewsByUser
// FROM
// PostStats ps
// JOIN
// UserStats us ON ps.UniquePostOwners = us.TotalBadges
// ORDER BY
// ps.TotalPosts DESC, us.Reputation DESC;
fn q14819(db: &'static So) -> String {
    let User { display_name, reputation, .. } = &db.user;
    let us = db
        .user
        .group_by(display_name.and(reputation))
        .select(badges_of(db).opt().and(posts_of(db).select((&db.post.view_count).opt()).opt()))
        .fold([0i64; 2], |a, (b, w)| [a[0] + b.is_some() as i64, a[1] + w.flatten().unwrap_or(0)]);
    let by_bx: HashIdx<i64, (Str, i64)> = (&us).map(|a| a[0]).inv().collect();
    let pf = by_key(db.post.iq(), name(db), comments_of(db).opt().and(&db.post.score).and((&db.post.view_count).opt()), [0i64; 4], |a, ((_, s), w)| {
        [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]
    });
    let du = db.post.group_by(name(db)).select(&db.post.owner_user_id).count_distinct();
    let dc = db.post.group_by(name(db)).select(comments_of(db)).count_distinct();
    let mut v = Vec::new();
    (&pf)
        .and((&du).opt())
        .and((&dc).opt())
        .and((&du).opt().map(|d: Option<i64>| d.unwrap_or(0)).select(&by_bx).select(Same::<(Str, i64)>::new().and(&us)))
        .drive(|k, (((a, d), c), (g, ua))| v.push((k, a, d.unwrap_or(0), c.unwrap_or(0), g, ua)));
    rows(v.iter().map(|&(k, a, d, c, (n, r), ua)| {
        row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), nullable(a[3], a[2]), V::I(d), V::I(c), V::S(n), V::I(r), V::I(ua[0]), V::I(ua[1])])
    }))
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// u.DisplayName AS OwnerDisplayName,
// COUNT(c.Id) AS CommentCount,
// COUNT(a.Id) AS AnswerCount
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Posts a ON p.Id = a.ParentId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName
// ),
// VoteStats AS (
// SELECT
// PostId,
// SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Votes
// GROUP BY
// PostId
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.ViewCount,
// ps.Score,
// ps.OwnerDisplayName,
// ps.CommentCount,
// ps.AnswerCount,
// COALESCE(vs.UpVotes, 0) AS UpVotes,
// COALESCE(vs.DownVotes, 0) AS DownVotes
// FROM
// PostStats ps
// LEFT JOIN
// VoteStats vs ON ps.PostId = vs.PostId
// ORDER BY
// ps.ViewCount DESC, ps.Score DESC
// LIMIT 100;
fn q14835(db: &'static So) -> String {
    let pv = post_votes(db);
    let mut v = Vec::new();
    stats_fold(db, db.post.iq(), Ident::<Post>::new(), "ca", &[]).and((&pv).opt()).drive(|p, (s, x)| v.push((p, s, x.unwrap_or([0; 3]))));
    out(v, |&(p, _, _)| views_score(db, p), 100, |&(p, s, x)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "owner"]);
        f.extend([V::I(s.cx), V::I(s.ax), V::I(x[1]), V::I(x[2])]);
        f
    })
}

// WITH UserPostCounts AS (
// SELECT
// u.Id AS UserId,
// COUNT(p.Id) AS PostCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id
// ),
// PostVoteCounts AS (
// SELECT
// p.Id AS PostId,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Posts p
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id
// ),
// PostSummary AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// pc.UpVotes,
// pc.DownVotes,
// upc.UserId,
// upc.PostCount
// FROM
// Posts p
// JOIN
// PostVoteCounts pc ON p.Id = pc.PostId
// JOIN
// UserPostCounts upc ON p.OwnerUserId = upc.UserId
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.Score,
// ps.ViewCount,
// ps.UpVotes,
// ps.DownVotes,
// ps.PostCount
// FROM
// PostSummary ps
// ORDER BY
// ps.Score DESC,
// ps.ViewCount DESC
// LIMIT 10;
fn q14850(db: &'static So) -> String {
    let pv = post_votes(db);
    let pc = owned(db).group_by(&db.post.owner_user).fold(0i64, |a, _| a + 1);
    let mut v = Vec::new();
    owned(db).select(Ident::<Post>::new().and((&pv).opt()).and((&db.post.owner_user).select(&pc))).drive(|_, x| v.push(x));
    out(v, |&((p, _), _)| score_views(db, p), 10, |&((p, x), n)| {
        let x = x.unwrap_or([0; 3]);
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(x[1]), V::I(x[2]), V::I(n)]);
        f
    })
}

// WITH UserPostStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(DISTINCT P.Id) AS TotalPosts,
// COUNT(DISTINCT CASE WHEN P.PostTypeId = 1 THEN P.Id END) AS TotalQuestions,
// COUNT(DISTINCT CASE WHEN P.PostTypeId = 2 THEN P.Id END) AS TotalAnswers,
// SUM(COALESCE(P.Score, 0)) AS TotalScore,
// SUM(COALESCE(P.ViewCount, 0)) AS TotalViews,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
// FROM Users U
// LEFT JOIN Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN Votes V ON P.Id = V.PostId
// GROUP BY U.Id, U.DisplayName, U.Reputation
// )
// SELECT
// UserId,
// DisplayName,
// Reputation,
// TotalPosts,
// TotalQuestions,
// TotalAnswers,
// TotalScore,
// TotalViews,
// TotalUpVotes,
// TotalDownVotes
// FROM UserPostStats
// ORDER BY TotalScore DESC, TotalPosts DESC;
fn q14856(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let pq = user_posts_q(db);
    let mut v = Vec::new();
    (&us).and(&pq).drive(|u, (a, q)| v.push((u, a, q)));
    rows(v.iter().map(|&(u, a, q)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(ints(&q[..3]));
        f.extend(["score_sum0", "views_sum0", "#up", "#down"].iter().map(|c| ustat_field(&a, c)));
        row(f)
    }))
}

// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// COALESCE(SUM(p.Score), 0) AS TotalScore,
// COALESCE(SUM(p.ViewCount), 0) AS TotalViews,
// COUNT(DISTINCT c.Id) AS TotalComments,
// COUNT(DISTINCT v.Id) AS TotalVotes,
// COALESCE(SUM(b.Count), 0) AS TotalTags
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// (SELECT
// Id, COUNT(*) AS Count
// FROM
// Tags
// GROUP BY
// Id) b ON p.Tags LIKE '%' || b.Id || '%'
// WHERE
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q14860(db: &'static So) -> String {
    let base = || since(db, month_ago());
    let strs: MatSet<Str> = base().select(&db.post.tags_str).collect();
    let m: HashIdx<Str, Id<Tag>> = (&strs).select_where((&db.tag.origid).inv(), |s: Str, id: i64| s.contains(&id.to_string())).collect();
    let f = by_key(
        base(),
        name(db),
        comments_of(db).opt().and(votes_of(db).opt()).and((&db.post.tags_str).select(&m).opt()).and(&db.post.score).and((&db.post.view_count).opt()),
        [0i64; 4],
        |a, ((((_, _), t), s), w)| [a[0] + 1, a[1] + s, a[2] + w.unwrap_or(0), a[3] + t.is_some() as i64],
    );
    let dc = base().group_by(name(db)).select(comments_of(db)).count_distinct();
    let dv = base().group_by(name(db)).select(votes_of(db)).count_distinct();
    let mut v = Vec::new();
    (&f).and((&dc).opt()).and((&dv).opt()).drive(|k, ((a, c), x)| v.push((k, a, c.unwrap_or(0), x.unwrap_or(0))));
    rows(v.iter().map(|&(k, a, c, x)| row(vec![V::S(k), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(c), V::I(x), V::I(a[3])])))
}

// WITH PostStats AS (
// SELECT
// P.Id AS PostId,
// P.PostTypeId,
// P.CreationDate,
// P.Score,
// P.ViewCount,
// P.AnswerCount,
// P.CommentCount,
// U.Reputation AS OwnerReputation,
// U.Id AS OwnerUserId,
// U.CreationDate AS UserCreationDate
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// ),
// UserStats AS (
// SELECT
// U.Id AS UserId,
// COUNT(B.Id) AS BadgeCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesCount,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesCount
// FROM
// Users U
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// LEFT JOIN
// Votes V ON U.Id = V.UserId
// GROUP BY
// U.Id
// )
// SELECT
// PS.PostId,
// PS.PostTypeId,
// PS.CreationDate,
// PS.Score,
// PS.ViewCount,
// PS.AnswerCount,
// PS.CommentCount,
// PS.OwnerReputation,
// US.BadgeCount,
// US.UpVotesCount,
// US.DownVotesCount
// FROM
// PostStats PS
// JOIN
// UserStats US ON PS.OwnerUserId = US.UserId
// ORDER BY
// PS.CreationDate DESC
// FETCH FIRST 100 ROWS ONLY;
fn q14864(db: &'static So) -> String {
    let us = g(db).select(badges_of(db).opt().and(votes_by(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 3], |a, (b, t)| {
        [a[0] + b.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let mut v = Vec::new();
    owned(db).select(Ident::<Post>::new().and((&db.post.owner_user).select(&us))).drive(|_, x| v.push(x));
    out(v, |&(p, _)| newest(db, p), 100, |&(p, a)| {
        let mut f = post_fields(db, p, &["id", "type_id", "created", "score", "views", "answers", "comments", "rep"]);
        f.extend(ints(&a));
        f
    })
}

// WITH UserPosts AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS PostCount,
// SUM(p.Score) AS TotalScore,
// SUM(p.ViewCount) AS TotalViews
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName
// ),
// PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// pt.Name AS PostType,
// p.CreationDate,
// p.LastActivityDate,
// p.Score,
// p.ViewCount,
// COALESCE(c.CommentCount, 0) AS CommentCount,
// p.OwnerUserId
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN (
// SELECT
// PostId,
// COUNT(*) AS CommentCount
// FROM
// Comments
// GROUP BY
// PostId
// ) c ON p.Id = c.PostId
// )
// SELECT
// up.DisplayName,
// up.PostCount,
// up.TotalScore,
// up.TotalViews,
// ps.PostId,
// ps.Title,
// ps.PostType,
// ps.CreationDate,
// ps.LastActivityDate,
// ps.Score,
// ps.ViewCount,
// ps.CommentCount
// FROM
// UserPosts up
// JOIN
// PostStats ps ON up.UserId = ps.OwnerUserId
// ORDER BY
// up.TotalScore DESC,
// up.PostCount DESC;
fn q14867(db: &'static So) -> String {
    let up = upqa(db);
    let mut v = Vec::new();
    owned(db).select(Ident::<Post>::new().and(comments_per_post(db)).and((&db.post.owner_user).select(Ident::<User>::new().and(&up)))).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((p, c), (u, a))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), nullable(a[5], a[0]), nullable(a[4], a[3])];
        f.extend(post_fields(db, p, &["id", "title", "type", "created", "activity", "score", "views"]));
        f.push(V::I(c));
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
// SUM(CASE WHEN p.PostTypeId = 3 THEN 1 ELSE 0 END) AS TotalWikis,
// AVG(p.Score) AS AvgPostScore,
// SUM(COALESCE(c.CommentCount, 0)) AS TotalComments,
// SUM(COALESCE(v.VoteCount, 0)) AS TotalVotes
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN (
// SELECT PostId, COUNT(*) AS CommentCount
// FROM Comments
// GROUP BY PostId
// ) c ON p.Id = c.PostId
// LEFT JOIN (
// SELECT PostId, COUNT(*) AS VoteCount
// FROM Votes
// GROUP BY PostId
// ) v ON p.Id = v.PostId
// GROUP BY u.Id, u.DisplayName
// )
// SELECT
// UserId,
// DisplayName,
// TotalPosts,
// TotalQuestions,
// TotalAnswers,
// TotalWikis,
// AvgPostScore,
// TotalComments,
// TotalVotes
// FROM UserPostStats
// ORDER BY TotalPosts DESC
// LIMIT 10;
fn q14876(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let uf = g(db).select(posts_of(db).select(post_type_id.and(score).and(comments_per_post(db)).and(votes_per_post(db))).opt()).fold([0i64; 7], |a, p| match p {
        Some((((t, s), c), x)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (t == 3) as i64, a[4] + s, a[5] + c, a[6] + x],
        None => a,
    });
    let mut v = Vec::new();
    (&uf).drive(|u, a| v.push((u, a)));
    out(v, |&(_, a)| Reverse(a[0]), 10, |&(u, a)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[..4]));
        f.extend([avg(a[4], a[0]), V::I(a[5]), V::I(a[6])]);
        f
    })
}

// SELECT
// u.DisplayName AS UserName,
// p.Title AS PostTitle,
// p.CreationDate AS PostCreationDate,
// p.ViewCount AS PostViewCount,
// p.Score AS PostScore,
// COUNT(c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
// pt.Name AS PostType,
// CASE
// WHEN p.AcceptedAnswerId IS NOT NULL THEN 'Accepted Answer'
// ELSE 'Not Accepted'
// END AS AcceptedStatus
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
// p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days'
// GROUP BY
// u.DisplayName, p.Title, p.CreationDate, p.ViewCount, p.Score, pt.Name, p.AcceptedAnswerId
// ORDER BY
// PostScore DESC, PostViewCount DESC;
fn q14885(db: &'static So) -> String {
    let Post { title, creation_date, view_count, score, accepted_answer_id, owner_user, .. } = &db.post;
    let key = || owner_user.select(&db.user.display_name).and(title.opt()).and(creation_date).and(view_count.opt()).and(score).and(name(db)).and(accepted_answer_id.opt());
    let base = || owned_since(db, date(2024, 9, 1));
    let f = base().group_by(key()).select(comments_of(db).opt().and(votes_of(db).opt())).fold(0i64, |a, (c, _)| a + c.is_some() as i64);
    let dv = base().group_by(key()).select(votes_of(db)).count_distinct();
    let mut v = Vec::new();
    (&f).and((&dv).opt()).drive(|k, (c, x)| v.push((k, c, x.unwrap_or(0))));
    rows(v.iter().map(|&(((((((n, t), cd), w), s), ty), acc), c, x)| {
        row(vec![V::S(n), ostr(t), V::T(cd), oint(w), V::I(s), V::I(c), V::I(x), V::S(ty), V::S(if acc.is_some() { "Accepted Answer" } else { "Not Accepted" })])
    }))
}

// WITH PostSummary AS (
// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AverageScore,
// SUM(p.ViewCount) AS TotalViews,
// SUM(CASE WHEN p.OwnerUserId IS NOT NULL THEN 1 ELSE 0 END) AS TotalUserPosts
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// GROUP BY
// pt.Name
// ),
// UserSummary AS (
// SELECT
// COUNT(*) AS TotalUsers,
// AVG(Reputation) AS AverageReputation,
// SUM(CASE WHEN Reputation >= 1000 THEN 1 ELSE 0 END) AS ActiveUsers
// FROM
// Users
// ),
// BadgeSummary AS (
// SELECT
// COUNT(*) AS TotalBadges,
// COUNT(DISTINCT UserId) AS UsersWithBadges,
// MAX(CASE WHEN Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// MAX(CASE WHEN Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// MAX(CASE WHEN Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM
// Badges
// )
// SELECT
// ps.PostType,
// ps.TotalPosts,
// ps.AverageScore,
// ps.TotalViews,
// ps.TotalUserPosts,
// us.TotalUsers,
// us.AverageReputation,
// us.ActiveUsers,
// bs.TotalBadges,
// bs.UsersWithBadges,
// bs.GoldBadges,
// bs.SilverBadges,
// bs.BronzeBadges
// FROM
// PostSummary ps,
// UserSummary us,
// BadgeSummary bs
// ORDER BY
// ps.TotalPosts DESC;
fn q14890(db: &'static So) -> String {
    let f = by_key(db.post.iq(), name(db), (&db.post.score).and((&db.post.view_count).opt()).and((&db.post.owner_user_id).opt()), [0i64; 5], |a, ((s, w), o)| {
        [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + o.is_some() as i64]
    });
    let u = db.user.select(&db.user.reputation).fold_flat([0i64; 3], |a, r| [a[0] + 1, a[1] + r, a[2] + (r >= 1000) as i64]);
    let b = db.badge.select(&db.badge.class).fold_flat([0i64; 4], |a, c| [a[0] + 1, a[1].max((c == 1) as i64), a[2].max((c == 2) as i64), a[3].max((c == 3) as i64)]);
    let du = one(whole(db.badge.iq()).select(&db.badge.user_id).count_distinct());
    let mut v = Vec::new();
    (&f).drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&(k, a)| {
        row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), nullable(a[3], a[2]), V::I(a[4]), V::I(u[0]), avg(u[1], u[0]), V::I(u[2]), V::I(b[0]), V::I(du), V::I(b[1]), V::I(b[2]), V::I(b[3])])
    }))
}

// WITH PostStats AS (
// SELECT
// COUNT(*) AS TotalPosts,
// COUNT(DISTINCT OwnerUserId) AS UniqueUsers,
// SUM(COALESCE(AnswerCount, 0)) AS TotalAnswers,
// SUM(COALESCE(CommentCount, 0)) AS TotalComments
// FROM Posts
// ),
// VoteStats AS (
// SELECT
// COUNT(*) AS TotalVotes,
// COUNT(DISTINCT UserId) AS UniqueVoters
// FROM Votes
// ),
// UserStats AS (
// SELECT
// COUNT(*) AS TotalUsers,
// AVG(Reputation) AS AvgReputation,
// MAX(Reputation) AS MaxReputation
// FROM Users
// )
// SELECT
// p.TotalPosts,
// p.UniqueUsers,
// p.TotalAnswers,
// p.TotalComments,
// v.TotalVotes,
// v.UniqueVoters,
// u.TotalUsers,
// u.AvgReputation,
// u.MaxReputation
// FROM
// PostStats p,
// VoteStats v,
// UserStats u;
fn q14903(db: &'static So) -> String {
    let Post { answer_count, comment_count, .. } = &db.post;
    let p = db.post.select(answer_count.opt().and(comment_count)).fold_flat([0i64; 3], |a, (an, c)| [a[0] + 1, a[1] + an.unwrap_or(0), a[2] + c]);
    let du = one(whole(db.post.iq()).select(&db.post.owner_user_id).count_distinct());
    let dv = one(whole(db.vote.iq()).select(&db.vote.user_id).count_distinct());
    let u = db.user.select(&db.user.reputation).fold_flat([0, 0, i64::MIN], |a: [i64; 3], r| [a[0] + 1, a[1] + r, a[2].max(r)]);
    row(vec![V::I(p[0]), V::I(du), nullable(p[1], p[0]), nullable(p[2], p[0]), V::I(count(db.vote.iq())), V::I(dv), V::I(u[0]), avg(u[1], u[0]), omax(u[2], u[0])])
}

// WITH PostStats AS (
// SELECT
// P.Id AS PostId,
// P.PostTypeId,
// P.CreationDate,
// P.ViewCount,
// P.Score,
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
// P.Id,
// P.PostTypeId,
// P.CreationDate,
// P.ViewCount,
// P.Score
// )
// SELECT
// PT.Name AS PostType,
// COUNT(*) AS TotalPosts,
// AVG(ViewCount) AS AvgViewCount,
// AVG(Score) AS AvgScore,
// SUM(CommentCount) AS TotalComments,
// SUM(VoteCount) AS TotalVotes,
// AVG(AvgUserReputation) AS AvgUserReputation
// FROM
// PostStats PS
// JOIN
// PostTypes PT ON PS.PostTypeId = PT.Id
// GROUP BY
// PT.Name
// ORDER BY
// TotalPosts DESC;
fn q14913(db: &'static So) -> String {
    let pf = stats_fold(db, db.post.iq(), Ident::<Post>::new(), "cv", &[]);
    let f = by_key(
        db.post.iq(),
        name(db),
        (&db.post.view_count).opt().and(&db.post.score).and(&pf).and((&db.post.owner_user).select(&db.user.reputation).opt()),
        [0i64; 8],
        |a, (((w, s), st), r)| [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s, a[4] + st.cx, a[5] + st.vx, a[6] + r.is_some() as i64, a[7] + r.unwrap_or(0)],
    );
    let mut v = Vec::new();
    (&f).drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), avg(a[2], a[1]), avg(a[3], a[0]), V::I(a[4]), V::I(a[5]), avg(a[7], a[6])])))
}

// WITH PostStats AS (
// SELECT
// p.Id as PostId,
// p.PostTypeId,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// COALESCE(c.CommentCount, 0) AS CommentCount,
// COALESCE(a.AnswerCount, 0) AS AnswerCount
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
// ParentId,
// COUNT(*) AS AnswerCount
// FROM
// Posts
// WHERE
// PostTypeId = 2
// GROUP BY
// ParentId
// ) a ON p.Id = a.ParentId
// )
// SELECT
// pt.Name AS PostType,
// COUNT(ps.PostId) AS TotalPosts,
// AVG(ps.Score) AS AverageScore,
// SUM(ps.ViewCount) AS TotalViews,
// SUM(ps.CommentCount) AS TotalComments,
// SUM(ps.AnswerCount) AS TotalAnswers
// FROM
// PostStats ps
// JOIN
// PostTypes pt ON ps.PostTypeId = pt.Id
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q14920(db: &'static So) -> String {
    let f = by_key(
        db.post.iq(),
        name(db),
        (&db.post.score).and((&db.post.view_count).opt()).and(comments_per_post(db)).and(typed_answers_per_post(db)),
        [0i64; 6],
        |a, (((s, w), c), an)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + c, a[5] + an],
    );
    let mut v = Vec::new();
    (&f).drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), avg(a[1], a[0]), nullable(a[3], a[2]), V::I(a[4]), V::I(a[5])])))
}

// WITH PostAnalytics AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// p.AnswerCount,
// COALESCE(COUNT(c.Id), 0) AS CommentCount,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount
// )
// SELECT
// pa.PostId,
// pa.Title,
// pa.CreationDate,
// pa.Score,
// pa.ViewCount,
// pa.AnswerCount,
// pa.CommentCount,
// pa.UpVotes,
// pa.DownVotes,
// (pa.UpVotes - pa.DownVotes) AS EngagementScore
// FROM
// PostAnalytics pa
// ORDER BY
// EngagementScore DESC;
fn q14928(db: &'static So) -> String {
    let mut v = Vec::new();
    stats_fold(db, since(db, date(2023, 1, 1)), Ident::<Post>::new(), "cv", &[]).drive(|p, s| v.push((p, s)));
    rows(v.iter().map(|&(p, s)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers"]);
        f.extend(ints(&[s.cx, s.up, s.down, s.up - s.down]));
        row(f)
    }))
}

// WITH UserEngagement AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(DISTINCT P.Id) AS TotalPosts,
// COUNT(DISTINCT C.Id) AS TotalComments,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// U.Id, U.DisplayName
// )
// SELECT
// UserId,
// DisplayName,
// TotalPosts,
// TotalComments,
// TotalUpvotes,
// TotalDownvotes,
// (TotalUpvotes - TotalDownvotes) AS NetVotes,
// CASE
// WHEN TotalPosts = 0 THEN 0
// ELSE (TotalComments * 1.0 / TotalPosts)
// END AS CommentPerPostRatio
// FROM
// UserEngagement
// ORDER BY
// TotalPosts DESC;
fn q14932(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "cv", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let dc = ud(db, UserWhere::All, posts_of(db).select(comments_of(db)));
    let mut v = Vec::new();
    (&us).and((&dp).opt()).and((&dc).opt()).drive(|u, ((a, p), c)| v.push((u, a, p.unwrap_or(0), c.unwrap_or(0))));
    rows(v.iter().map(|&(u, a, p, c)| {
        row(vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(p), V::I(c), V::I(a.up), V::I(a.down), V::I(a.up - a.down), V::F(if p == 0 { 0.0 } else { c as f64 / p as f64 })])
    }))
}

// WITH UserPostCounts AS (
// SELECT OwnerUserId, COUNT(*) AS PostCount
// FROM Posts
// GROUP BY OwnerUserId
// ),
// UserVoteCounts AS (
// SELECT UserId, COUNT(*) AS VoteCount
// FROM Votes
// GROUP BY UserId
// ),
// UserBadgeCounts AS (
// SELECT UserId, COUNT(*) AS BadgeCount
// FROM Badges
// GROUP BY UserId
// ),
// UserActivity AS (
// SELECT u.Id AS UserId,
// u.DisplayName,
// COALESCE(up.PostCount, 0) AS PostCount,
// COALESCE(uv.VoteCount, 0) AS VoteCount,
// COALESCE(ub.BadgeCount, 0) AS BadgeCount,
// u.Reputation,
// u.CreationDate,
// u.LastAccessDate
// FROM Users u
// LEFT JOIN UserPostCounts up ON u.Id = up.OwnerUserId
// LEFT JOIN UserVoteCounts uv ON u.Id = uv.UserId
// LEFT JOIN UserBadgeCounts ub ON u.Id = ub.UserId
// )
// SELECT UserId,
// DisplayName,
// PostCount,
// VoteCount,
// BadgeCount,
// Reputation,
// CreationDate,
// LastAccessDate
// FROM UserActivity
// ORDER BY Reputation DESC, PostCount DESC, VoteCount DESC
// LIMIT 100;
fn q14940(db: &'static So) -> String {
    let pc = owned(db).group_by(&db.post.owner_user).fold(0i64, |a, _| a + 1);
    let vc = votes_per_user(db);
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&pc).opt()).and(&vc).and(&bu)).drive(|_, (((u, p), x), b)| v.push((u, p.unwrap_or(0), x, b)));
    out(v, |&(u, p, x, _)| (rep_desc(db, u), Reverse(p), Reverse(x)), 100, |&(u, p, x, b)| {
        vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(p), V::I(x), V::I(b), user_col(db, u, "rep"), user_col(db, u, "ucreated"), user_col(db, u, "last_access")]
    })
}

// WITH PostCounts AS (
// SELECT
// p.OwnerUserId,
// COUNT(*) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// ),
// UserReputation AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// COALESCE(pc.PostCount, 0) AS TotalPosts,
// COALESCE(pc.QuestionCount, 0) AS TotalQuestions,
// COALESCE(pc.AnswerCount, 0) AS TotalAnswers
// FROM
// Users u
// LEFT JOIN
// PostCounts pc ON u.Id = pc.OwnerUserId
// )
// SELECT
// u.UserId,
// u.Reputation,
// u.TotalPosts,
// u.TotalQuestions,
// u.TotalAnswers,
// CASE
// WHEN u.TotalPosts > 0 THEN (u.Reputation / u.TotalPosts)
// ELSE 0
// END AS ReputationPerPost,
// CASE
// WHEN u.TotalQuestions > 0 THEN (u.Reputation / u.TotalQuestions)
// ELSE 0
// END AS ReputationPerQuestion,
// CASE
// WHEN u.TotalAnswers > 0 THEN (u.Reputation / u.TotalAnswers)
// ELSE 0
// END AS ReputationPerAnswer
// FROM
// UserReputation u
// ORDER BY
// u.Reputation DESC
// LIMIT 100;
fn q14944(db: &'static So) -> String {
    let pc = owned(db).group_by(&db.post.owner_user).select(&db.post.post_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64]);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&pc).opt())).drive(|_, (u, p)| v.push((u, p.unwrap_or([0; 3]))));
    let div = |r: i64, d: i64| V::F(if d > 0 { r as f64 / d as f64 } else { 0.0 });
    out(v, |&(u, _)| rep_desc(db, u), 100, |&(u, a)| {
        let r = db.user.reputation.get(u).unwrap();
        let mut f = vec![user_col(db, u, "uid"), V::I(r)];
        f.extend(ints(&a));
        f.extend([div(r, a[0]), div(r, a[1]), div(r, a[2])]);
        f
    })
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(p.Score) AS TotalScore,
// AVG(p.Score) AS AverageScore
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName
// ),
// PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// COUNT(DISTINCT c.Id) AS TotalComments
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount
// )
// SELECT
// us.UserId,
// us.DisplayName,
// us.TotalPosts,
// us.TotalQuestions,
// us.TotalAnswers,
// us.TotalScore,
// us.AverageScore,
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.Score,
// ps.ViewCount,
// ps.TotalComments
// FROM
// UserStats us
// JOIN
// PostStats ps ON us.UserId = ps.PostId
// ORDER BY
// us.TotalScore DESC, us.TotalPosts DESC;
fn q14953(db: &'static So) -> String {
    let uid = uids(db);
    let up = upqa(db);
    let mut v = Vec::new();
    db.post
        .with((&db.post.origid).select(&uid))
        .select(Ident::<Post>::new().and(comments_per_post(db)).and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&up))))
        .drive(|_, x| v.push(x));
    rows(v.iter().map(|&((p, c), (u, a))| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), nullable(a[5], a[0]), avg(a[5], a[0])];
        f.extend(post_fields(db, p, &["id", "title", "created", "score", "views"]));
        f.push(V::I(c));
        row(f)
    }))
}

// WITH UserActivity AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(c.Score) AS TotalCommentScore,
// SUM(v.BountyAmount) AS TotalBountyAmount,
// COUNT(DISTINCT b.Id) AS BadgeCount,
// MAX(p.CreationDate) AS LastActive
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId AND v.UserId = u.Id
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
// )
// SELECT
// UserId,
// DisplayName,
// Reputation,
// PostCount,
// QuestionCount,
// AnswerCount,
// TotalCommentScore,
// TotalBountyAmount,
// BadgeCount,
// LastActive
// FROM
// UserActivity
// ORDER BY
// Reputation DESC, PostCount DESC
// LIMIT 100;
fn q14964(db: &'static So) -> String {
    let own = self_votes(db);
    let Post { post_type_id, creation_date, .. } = &db.post;
    let uf = g(db)
        .select(posts_of(db).select(post_type_id.and(creation_date).and(comments_of(db).select(&db.comment.score).opt()).and((&own).select((&db.vote.bounty_amount).opt()).opt())).opt().and(badges_of(db).opt()))
        .fold([0, 0, 0, 0, 0, 0, i64::MIN], |a: [i64; 7], (p, _)| match p {
            Some((((t, c), cs), b)) => {
                let b = b.flatten();
                [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + cs.is_some() as i64, a[3] + cs.unwrap_or(0), a[4] + b.is_some() as i64, a[5] + b.unwrap_or(0), a[6].max(c)]
            }
            None => a,
        });
    let dp = ud(db, UserWhere::All, posts_of(db));
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    (&uf).and((&dp).opt()).and(&bu).drive(|u, ((a, p), b)| v.push((u, a, p.unwrap_or(0), b)));
    out(v, |&(u, _, p, _)| (rep_desc(db, u), Reverse(p)), 100, |&(u, a, p, b)| {
        vec![
            user_col(db, u, "uid"),
            user_col(db, u, "name"),
            user_col(db, u, "rep"),
            V::I(p),
            V::I(a[0]),
            V::I(a[1]),
            nullable(a[3], a[2]),
            nullable(a[5], a[4]),
            V::I(b),
            if p == 0 { V::Null } else { V::T(a[6]) },
        ]
    })
}

// WITH Benchmark AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// u.Reputation AS UserReputation,
// u.CreationDate AS UserCreationDate,
// COUNT(c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
// (
// SELECT COUNT(*)
// FROM PostHistory ph
// WHERE ph.PostId = p.Id
// ) AS EditCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, u.Reputation, p.Title, p.CreationDate, p.Score, p.ViewCount, u.CreationDate
// )
// SELECT
// *,
// (CASE
// WHEN UserReputation > 1000 THEN 'High Reputation'
// WHEN UserReputation BETWEEN 501 AND 1000 THEN 'Medium Reputation'
// ELSE 'Low Reputation'
// END) AS ReputationCategory
// FROM
// Benchmark
// ORDER BY
// CreationDate DESC, Score DESC;
fn q14969(db: &'static So) -> String {
    let mut v = Vec::new();
    stats_fold(db, owned(db), Ident::<Post>::new(), "cv", &[]).and(votes_per_post(db)).and(history_per_post(db)).drive(|p, ((s, x), h)| v.push((p, s, x, h)));
    rows(v.iter().map(|&(p, s, x, h)| {
        let r = db.user.reputation.get(db.post.owner_user.get(p).unwrap()).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "rep", "ucreated"]);
        f.extend([V::I(s.cx), V::I(x), V::I(h)]);
        f.push(V::S(if r > 1000 {
            "High Reputation"
        } else if (501..=1000).contains(&r) {
            "Medium Reputation"
        } else {
            "Low Reputation"
        }));
        row(f)
    }))
}

// WITH UserPostCounts AS (
// SELECT
// U.Id AS UserId,
// U.Reputation,
// COUNT(P.Id) AS PostCount
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id, U.Reputation
// ),
// UserBadgeCounts AS (
// SELECT
// B.UserId,
// COUNT(B.Id) AS BadgeCount
// FROM
// Badges B
// GROUP BY
// B.UserId
// ),
// OverallStats AS (
// SELECT
// UPC.UserId,
// UPC.Reputation,
// UPC.PostCount,
// COALESCE(UBC.BadgeCount, 0) AS BadgeCount
// FROM
// UserPostCounts UPC
// LEFT JOIN
// UserBadgeCounts UBC ON UPC.UserId = UBC.UserId
// )
// SELECT
// UserId,
// Reputation,
// PostCount,
// BadgeCount,
// (PostCount + BadgeCount) AS TotalScore
// FROM
// OverallStats
// ORDER BY
// TotalScore DESC
// LIMIT 10;
fn q14990(db: &'static So) -> String {
    let pc = g(db).select(posts_of(db).opt()).fold(0i64, |a, p| a + p.is_some() as i64);
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    (&pc).and(&bu).drive(|u, (n, b)| v.push((u, n, b)));
    out(v, |&(_, n, b)| Reverse(n + b), 10, |&(u, n, b)| vec![user_col(db, u, "uid"), user_col(db, u, "rep"), V::I(n), V::I(b), V::I(n + b)])
}

// WITH PostMetrics AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.Score,
// P.ViewCount,
// COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
// COUNT(DISTINCT V.Id) AS VoteCount,
// COUNT(DISTINCT B.Id) AS BadgeCount,
// COALESCE(PA.Title, 'No Accepted Answer') AS AcceptedAnswerTitle
// FROM
// Posts P
// LEFT JOIN
// Comments C ON C.PostId = P.Id
// LEFT JOIN
// Votes V ON V.PostId = P.Id
// LEFT JOIN
// Badges B ON B.UserId = P.OwnerUserId
// LEFT JOIN
// Posts PA ON PA.Id = P.AcceptedAnswerId
// WHERE
// P.CreationDate >= '2023-01-01'
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.Score, P.ViewCount, PA.Title
// )
// SELECT
// PostId,
// Title,
// CreationDate,
// Score,
// ViewCount,
// CommentCount,
// VoteCount,
// BadgeCount,
// AcceptedAnswerTitle
// FROM
// PostMetrics
// ORDER BY
// CreationDate DESC
// LIMIT 100;
fn q14992(db: &'static So) -> String {
    let owner_user_id = &db.post.owner_user_id;
    let by_uid: HashIdx<i64, Id<Badge>> = (&db.badge.user_id).inv().collect();
    let base = || since(db, date(2023, 1, 1));
    let cf = base()
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).opt()).and(owner_user_id.select(&by_uid).opt()))
        .fold(0i64, |a, ((c, _), _)| a + c.is_some() as i64);
    let bd = base().group_by(Ident::<Post>::new()).select(owner_user_id.select(&by_uid)).count_distinct();
    let mut v = Vec::new();
    (&cf).and(votes_per_post(db)).and((&bd).opt()).drive(|p, ((c, x), b)| v.push((p, c, x, b.unwrap_or(0))));
    out(v, |&(p, ..)| newest(db, p), 100, |&(p, c, x, b)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(c), V::I(x), V::I(b)]);
        f.push(V::S(db.post.accepted_answer.get(p).and_then(|a| db.post.title.get(a)).unwrap_or("No Accepted Answer")));
        f
    })
}

// SELECT
// P.Id AS PostId,
// P.Title,
// U.DisplayName AS OwnerDisplayName,
// P.CreationDate,
// P.Score,
// P.ViewCount,
// COALESCE(C.CommentCount, 0) AS CommentCount,
// COALESCE(A.AnswerCount, 0) AS AnswerCount
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount
// FROM Comments
// GROUP BY PostId) C ON P.Id = C.PostId
// LEFT JOIN
// (SELECT ParentId, COUNT(*) AS AnswerCount
// FROM Posts
// WHERE PostTypeId = 2
// GROUP BY ParentId) A ON P.Id = A.ParentId
// WHERE
// P.PostTypeId = 1
// ORDER BY
// P.CreationDate DESC
// LIMIT 10;
fn q15276(db: &'static So) -> String {
    let mut v = Vec::new();
    owned(db).with((&db.post.post_type_id).eq(1)).select(Ident::<Post>::new().and(comments_per_post(db)).and(typed_answers_per_post(db))).drive(|_, x| v.push(x));
    out(v, |&((p, _), _)| newest(db, p), 10, |&((p, c), a)| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "score", "views"]);
        f.extend([V::I(c), V::I(a)]);
        f
    })
}

// SELECT
// p.Id AS PostId,
// p.Title,
// u.DisplayName AS OwnerDisplayName,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// COALESCE(c.CommentCount, 0) AS CommentCount
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
fn q15591(db: &'static So) -> String {
    let mut v = Vec::new();
    owned(db).with((&db.post.post_type_id).eq(1)).select(Ident::<Post>::new().and(comments_per_post(db))).drive(|_, x| v.push(x));
    out(v, |&(p, _)| newest(db, p), 10, |&(p, c)| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "score", "views"]);
        f.push(V::I(c));
        f
    })
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// u.DisplayName AS OwnerDisplayName,
// COALESCE(c.CommentCount, 0) AS NumberOfComments,
// COALESCE(an.AnswerCount, 0) AS NumberOfAnswers,
// COALESCE(v.TotalUpVotes, 0) AS TotalUpVotes
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
// LEFT JOIN
// (SELECT ParentId, COUNT(*) AS AnswerCount FROM Posts WHERE PostTypeId = 2 GROUP BY ParentId) an ON p.Id = an.ParentId
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS TotalUpVotes FROM Votes WHERE VoteTypeId = 2 GROUP BY PostId) v ON p.Id = v.PostId
// WHERE
// p.PostTypeId = 1
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q15896(db: &'static So) -> String {
    let mut v = Vec::new();
    owned(db)
        .with((&db.post.post_type_id).eq(1))
        .select(Ident::<Post>::new().and(comments_per_post(db)).and(typed_answers_per_post(db)).and(votes_of_type(db, 2)))
        .drive(|_, x| v.push(x));
    out(v, |&(((p, _), _), _)| newest(db, p), 10, |&(((p, c), a), u)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend([V::I(c), V::I(a), V::I(u)]);
        f
    })
}

// SELECT
// p.Title,
// p.CreationDate,
// u.DisplayName AS OwnerName,
// COUNT(c.Id) AS CommentCount,
// COALESCE(SUM(v.VoteCount), 0) AS TotalVotes
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS VoteCount FROM Votes GROUP BY PostId) v ON p.Id = v.PostId
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Title, p.CreationDate, u.DisplayName, p.Id
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q17737(db: &'static So) -> String {
    let vc = db.vote.group_by(&db.vote.post).fold(0i64, |a, _| a + 1);
    let f = owned(db)
        .with((&db.post.post_type_id).eq(1))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and((&vc).opt()))
        .fold([0i64; 2], |a, (c, x)| [a[0] + c.is_some() as i64, a[1] + x.unwrap_or(0)]);
    let mut v = Vec::new();
    (&f).drive(|p, a| v.push((p, a)));
    out(v, |&(p, _)| newest(db, p), 10, |&(p, a)| {
        let mut f = post_fields(db, p, &["title", "created", "owner"]);
        f.extend(ints(&a));
        f
    })
}

// SELECT
// U.DisplayName,
// COUNT(P.Id) AS PostCount,
// SUM(COALESCE(V.VoteCount, 0)) AS TotalVotes
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS VoteCount
// FROM Votes
// GROUP BY PostId) V ON P.Id = V.PostId
// WHERE
// U.Reputation > 100
// GROUP BY
// U.Id, U.DisplayName
// ORDER BY
// TotalVotes DESC;
fn q17913(db: &'static So) -> String {
    let uf = user_base(db, UserWhere::RepGt(100)).group_by(Ident::<User>::new()).select(posts_of(db).select(votes_per_post(db)).opt()).fold([0i64; 2], |a, x| match x {
        Some(x) => [a[0] + 1, a[1] + x],
        None => a,
    });
    let mut v = Vec::new();
    (&uf).drive(|u, a| v.push((u, a)));
    rows(v.iter().map(|&(u, a)| row(vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1])])))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// u.DisplayName AS OwnerDisplayName,
// p.Score,
// p.ViewCount,
// COALESCE(c.CommentCount, 0) AS CommentCount,
// COALESCE(a.AnswerCount, 0) AS AnswerCount
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
fn q18301(db: &'static So) -> String {
    let mut v = Vec::new();
    owned(db).with((&db.post.post_type_id).eq(1)).select(Ident::<Post>::new().and(comments_per_post(db)).and(typed_answers_per_post(db))).drive(|_, x| v.push(x));
    out(v, |&((p, _), _)| newest(db, p), 10, |&((p, c), a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner", "score", "views"]);
        f.extend([V::I(c), V::I(a)]);
        f
    })
}

// SELECT
// p.Title,
// p.CreationDate,
// u.DisplayName AS Author,
// COUNT(c.Id) AS CommentCount,
// SUM(v.vote_value) AS TotalVotes
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// (SELECT
// PostId,
// CASE
// WHEN VoteTypeId = 2 THEN 1
// WHEN VoteTypeId = 3 THEN -1
// ELSE 0
// END AS vote_value
// FROM
// Votes) v ON p.Id = v.PostId
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Title, p.CreationDate, u.DisplayName
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q18638(db: &'static So) -> String {
    let Post { title, creation_date, owner_user, .. } = &db.post;
    let f = owned(db)
        .with((&db.post.post_type_id).eq(1))
        .group_by(title.opt().and(creation_date).and(owner_user.select(&db.user.display_name)))
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| {
            let val = match t {
                Some(2) => 1,
                Some(3) => -1,
                _ => 0,
            };
            [a[0] + c.is_some() as i64, a[1] + t.is_some() as i64, a[2] + val]
        });
    let mut v = Vec::new();
    (&f).drive(|k, a| v.push((k, a)));
    out(v, |&(((_, cd), _), _)| Reverse(cd), 10, |&(((t, cd), n), a)| vec![ostr(t), V::T(cd), V::S(n), V::I(a[0]), nullable(a[2], a[1])])
}

// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// u.DisplayName AS Author,
// COALESCE(c.CommentCount, 0) AS TotalComments,
// COALESCE(v.UpVotes, 0) AS TotalUpVotes,
// COALESCE(v.DownVotes, 0) AS TotalDownVotes
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
// LEFT JOIN
// (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM Votes
// GROUP BY PostId) v ON p.Id = v.PostId
// WHERE
// p.PostTypeId = 1
// ORDER BY
// p.CreationDate DESC
// LIMIT 10;
fn q18654(db: &'static So) -> String {
    let pv = post_votes(db);
    let mut v = Vec::new();
    owned(db).with((&db.post.post_type_id).eq(1)).select(Ident::<Post>::new().and(comments_per_post(db)).and((&pv).opt())).drive(|_, x| v.push(x));
    out(v, |&((p, _), _)| newest(db, p), 10, |&((p, c), x)| {
        let x = x.unwrap_or([0; 3]);
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend([V::I(c), V::I(x[1]), V::I(x[2])]);
        f
    })
}

// SELECT
// p.Id AS PostId,
// p.Title,
// u.DisplayName AS OwnerName,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// COALESCE(c.CommentCount, 0) AS CommentCount
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
fn q19611(db: &'static So) -> String {
    let mut v = Vec::new();
    owned(db).with((&db.post.post_type_id).eq(1)).select(Ident::<Post>::new().and(comments_per_post(db))).drive(|_, x| v.push(x));
    out(v, |&(p, _)| newest(db, p), 10, |&(p, c)| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "score", "views"]);
        f.push(V::I(c));
        f
    })
}

// WITH UserBadges AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(b.Id) AS BadgeCount,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM Users u
// LEFT JOIN Badges b ON u.Id = b.UserId
// GROUP BY u.Id, u.DisplayName
// ),
// PostStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(p.ViewCount) AS TotalViews
// FROM Posts p
// GROUP BY p.OwnerUserId
// ),
// CombinedStats AS (
// SELECT
// ub.UserId,
// ub.DisplayName,
// COALESCE(ps.TotalPosts, 0) AS TotalPosts,
// COALESCE(ps.Questions, 0) AS Questions,
// COALESCE(ps.Answers, 0) AS Answers,
// COALESCE(ps.TotalViews, 0) AS TotalViews,
// ub.BadgeCount,
// ub.GoldBadges,
// ub.SilverBadges,
// ub.BronzeBadges
// FROM UserBadges ub
// LEFT JOIN PostStats ps ON ub.UserId = ps.OwnerUserId
// )
// SELECT
// cs.DisplayName,
// cs.TotalPosts,
// cs.Questions,
// cs.Answers,
// cs.TotalViews,
// cs.BadgeCount,
// cs.GoldBadges,
// cs.SilverBadges,
// cs.BronzeBadges
// FROM CombinedStats cs
// WHERE cs.TotalPosts > 0
// ORDER BY cs.TotalViews DESC
// LIMIT 10;
fn q2249(db: &'static So) -> String {
    let ub = g(db).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let Post { post_type_id, view_count, .. } = &db.post;
    let pf = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(view_count.opt())).fold([0i64; 4], |a, (t, w)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.unwrap_or(0)]);
    let mut v = Vec::new();
    (&ub).and((&pf).filt(|a: [i64; 4]| a[0] > 0)).drive(|u, (b, p)| v.push((u, b, p)));
    out(v, |&(_, _, p)| Reverse(p[3]), 10, |&(u, b, p)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(ints(&p));
        f.extend(ints(&b));
        f
    })
}

// WITH UserBadgeStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(b.Id) AS BadgeCount,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// u.Id, u.DisplayName
// ),
// PopularPosts AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// COUNT(c.Id) AS CommentCount,
// SUM(p.Score) AS TotalScore
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// WHERE
// p.CreationDate >= '2022-01-01'
// GROUP BY
// p.Id, p.Title
// ORDER BY
// TotalScore DESC
// LIMIT 10
// ),
// BadgeAwardedPosts AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// ub.UserId,
// ub.DisplayName,
// ub.BadgeCount
// FROM
// PopularPosts pp
// JOIN
// Posts p ON p.Id = pp.PostId
// JOIN
// UserBadgeStats ub ON ub.UserId = p.OwnerUserId
// )
// SELECT
// bp.PostId,
// bp.Title,
// bp.DisplayName AS UserDisplayName,
// bp.BadgeCount,
// pp.CommentCount AS PostCommentCount,
// pp.TotalScore AS PostTotalScore
// FROM
// BadgeAwardedPosts bp
// JOIN
// PopularPosts pp ON bp.PostId = pp.PostId
// ORDER BY
// bp.BadgeCount DESC, pp.TotalScore DESC;
fn q25231(db: &'static So) -> String {
    let pp = since(db, date(2022, 1, 1)).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(&db.post.score)).fold([0i64; 2], |a, (c, s)| [a[0] + c.is_some() as i64, a[1] + s]);
    let top: MatSet<Id<Post>> = whole(&pp)
        .select(Same::new().and(&pp))
        .window(row_number, |(p, a): (Id<Post>, [i64; 2])| (a[1], Reverse(db.post.origid.get(p).unwrap())), desc)
        .filt(|(_, n)| n <= 10)
        .map(|((p, _), _)| p)
        .collect();
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    (&top).select(Ident::<Post>::new().and(&pp).and((&db.post.owner_user).select(Ident::<User>::new().and(&bu)))).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((p, a), (u, b))| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([user_col(db, u, "name"), V::I(b), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH UserBadges AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(b.Id) AS BadgeCount,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// u.Id, u.DisplayName
// ),
// PostStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// AVG(p.Score) AS AverageScore
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// )
// SELECT
// ub.UserId,
// ub.DisplayName,
// ub.BadgeCount,
// ub.GoldBadges,
// ub.SilverBadges,
// ub.BronzeBadges,
// ps.PostCount,
// ps.Questions,
// ps.Answers,
// ps.AverageScore
// FROM
// UserBadges ub
// LEFT JOIN
// PostStats ps ON ub.UserId = ps.OwnerUserId
// WHERE
// ub.BadgeCount > 0
// ORDER BY
// ub.BadgeCount DESC,
// ps.PostCount DESC
// LIMIT 50;
fn q25335(db: &'static So) -> String {
    let ub = g(db).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let pf = owned(db).group_by(&db.post.owner_user).select((&db.post.post_type_id).and(&db.post.score)).fold([0i64; 4], |a, (t, s)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s]);
    let mut v = Vec::new();
    (&ub).filt(|a: [i64; 4]| a[0] > 0).and((&pf).opt()).drive(|u, (b, p)| v.push((u, b, p)));
    out(v, |&(_, b, p)| (Reverse(b[0]), (p.is_none(), Reverse(p.map(|p| p[0])))), 50, |&(u, b, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&b));
        f.extend((0..3).map(|i| oint(p.map(|p| p[i]))));
        f.push(p.map_or(V::Null, |p| avg(p[3], p[0])));
        f
    })
}

// WITH UserStatistics AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(DISTINCT P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(P.Score) AS TotalScore,
// AVG(V.BountyAmount) AS AverageBounty
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Votes V ON P.Id = V.PostId AND V.VoteTypeId = 8
// GROUP BY
// U.Id, U.DisplayName
// ),
// ClosedPostStatistics AS (
// SELECT
// PH.UserId,
// COUNT(DISTINCT PH.PostId) AS TotalClosedPosts,
// COUNT(DISTINCT CASE WHEN P.PostTypeId = 1 THEN PH.PostId END) AS TotalClosedQuestions,
// COUNT(DISTINCT CASE WHEN P.PostTypeId = 2 THEN PH.PostId END) AS TotalClosedAnswers,
// COUNT(DISTINCT PH.Comment) AS TotalCloseReasons
// FROM
// PostHistory PH
// JOIN
// Posts P ON PH.PostId = P.Id
// WHERE
// PH.PostHistoryTypeId IN (10, 11)
// GROUP BY
// PH.UserId
// )
// SELECT
// US.UserId,
// US.DisplayName,
// US.TotalPosts,
// US.Questions,
// US.Answers,
// US.TotalScore,
// US.AverageBounty,
// CPS.TotalClosedPosts,
// CPS.TotalClosedQuestions,
// CPS.TotalClosedAnswers,
// CPS.TotalCloseReasons
// FROM
// UserStatistics US
// LEFT JOIN
// ClosedPostStatistics CPS ON US.UserId = CPS.UserId
// ORDER BY
// US.TotalScore DESC,
// US.TotalPosts DESC;
fn q25396(db: &'static So) -> String {
    let us = user_stats_fold_v(db, Ident::<User>::new(), UserWhere::All, "v", any_post, &[8]);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let PostHistory { post, post_history_type_id, comment, user, .. } = &db.post_history;
    let base = || db.post_history.with(post_history_type_id.in_v(vec![10, 11]));
    let typed = |t: i64| post.select(Ident::<Post>::new().with((&db.post.post_type_id).eq(t)));
    let d1 = base().group_by(user).select(post).count_distinct();
    let d2 = base().group_by(user).select(typed(1)).count_distinct();
    let d3 = base().group_by(user).select(typed(2)).count_distinct();
    let d4 = base().group_by(user).select(comment).count_distinct();
    let mut v = Vec::new();
    (&us)
        .and((&dp).opt())
        .and((&d1).opt())
        .and((&d2).opt())
        .and((&d3).opt())
        .and((&d4).opt())
        .drive(|u, (((((a, p), x1), x2), x3), x4)| v.push((u, a, p.unwrap_or(0), x1, [x2, x3, x4])));
    rows(v.iter().map(|&(u, a, p, x1, xs)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(p), V::I(a.q), V::I(a.a), ustat_field(&a, "score_sum"), avg(a.bounty_sum, a.bounty_n)];
        match x1 {
            Some(x1) => {
                f.push(V::I(x1));
                f.extend(xs.iter().map(|x| V::I(x.unwrap_or(0))));
            }
            None => f.extend(nulls(4)),
        }
        row(f)
    }))
}

// WITH TitleStats AS (
// SELECT
// AVG(LENGTH(Title)) AS AvgTitleLength,
// MIN(LENGTH(Title)) AS MinTitleLength,
// MAX(LENGTH(Title)) AS MaxTitleLength,
// COUNT(*) AS TitleCount
// FROM
// Posts
// WHERE
// PostTypeId = 1
// ),
// BodyStats AS (
// SELECT
// AVG(LENGTH(Body)) AS AvgBodyLength,
// MIN(LENGTH(Body)) AS MinBodyLength,
// MAX(LENGTH(Body)) AS MaxBodyLength,
// COUNT(*) AS BodyCount
// FROM
// Posts
// WHERE
// PostTypeId IN (1, 2)
// ),
// TagStats AS (
// SELECT
// COUNT(DISTINCT Tags) AS DistinctTagCount,
// SUM(LENGTH(Tags) - LENGTH(REPLACE(Tags, '<', '')) / LENGTH('<')) AS TagCount
// FROM
// Posts
// WHERE
// PostTypeId = 1
// ),
// CombinedStats AS (
// SELECT
// t.AvgTitleLength,
// t.MinTitleLength,
// t.MaxTitleLength,
// b.AvgBodyLength,
// b.MinBodyLength,
// b.MaxBodyLength,
// tg.DistinctTagCount,
// tg.TagCount
// FROM
// TitleStats t, BodyStats b, TagStats tg
// )
// SELECT
// AvgTitleLength,
// MinTitleLength,
// MaxTitleLength,
// AvgBodyLength,
// MinBodyLength,
// MaxBodyLength,
// DistinctTagCount,
// TagCount
// FROM
// CombinedStats;
fn q25433(db: &'static So) -> String {
    let Post { title, body, tags_str, post_type_id, .. } = &db.post;
    let len = |s: Str| s.chars().count() as i64;
    let lens = move |a: [i64; 4], s: Str| {
        let l = len(s);
        [a[0] + 1, a[1] + l, a[2].min(l), a[3].max(l)]
    };
    let t = questions_only(db).select(title).fold_flat([0, 0, i64::MAX, i64::MIN], lens);
    let b = db.post.with(post_type_id.in_v(vec![1, 2])).select(body).fold_flat([0, 0, i64::MAX, i64::MIN], lens);
    let dt = one(whole(questions_only(db)).select(tags_str).count_distinct());
    let tc = questions_only(db).select(tags_str).fold_flat((0i64, 0.0f64), move |(n, a), s| (n + 1, a + (len(s) as f64 - s.replace('<', "").chars().count() as f64 / 1.0)));
    let tsum = if tc.0 == 0 { V::Null } else { V::F(tc.1) };
    row(vec![avg(t[1], t[0]), omax(t[2], t[0]), omax(t[3], t[0]), avg(b[1], b[0]), omax(b[2], b[0]), omax(b[3], b[0]), V::I(dt), tsum])
}

// WITH UserReputation AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COALESCE(b.badge_count, 0) AS BadgeCount,
// COALESCE(p.post_count, 0) AS PostCount
// FROM
// Users u
// LEFT JOIN (
// SELECT
// UserId,
// COUNT(*) AS badge_count
// FROM
// Badges
// GROUP BY
// UserId
// ) b ON u.Id = b.UserId
// LEFT JOIN (
// SELECT
// OwnerUserId AS UserId,
// COUNT(*) AS post_count
// FROM
// Posts
// WHERE
// PostTypeId = 1
// GROUP BY
// OwnerUserId
// ) p ON u.Id = p.UserId
// ),
// TopUsers AS (
// SELECT
// UserId,
// DisplayName,
// Reputation,
// BadgeCount,
// PostCount
// FROM
// UserReputation
// ORDER BY
// Reputation DESC,
// BadgeCount DESC,
// PostCount DESC
// LIMIT 10
// ),
// PostDetails AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.ViewCount,
// p.Score,
// p.CreationDate,
// u.DisplayName AS OwnerDisplayName,
// p.OwnerUserId
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.PostTypeId = 1
// AND p.ViewCount > 100
// )
// SELECT
// u.DisplayName,
// u.Reputation,
// u.BadgeCount,
// u.PostCount,
// pd.PostId,
// pd.Title,
// pd.ViewCount,
// pd.Score,
// pd.CreationDate
// FROM
// TopUsers u
// JOIN
// PostDetails pd ON u.UserId = pd.OwnerUserId
// ORDER BY
// u.Reputation DESC,
// pd.ViewCount DESC;
fn q25554(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let qc = owned(db).with((&db.post.post_type_id).eq(1)).group_by(&db.post.owner_user).fold(0i64, |a, _| a + 1);
    let top: MatSet<Id<User>> = whole(&db.user.id)
        .select(Ident::<User>::new().and(&db.user.reputation).and(&bu).and((&qc).opt()))
        .window(row_number, |(((_, r), b), q)| (r, b, q.unwrap_or(0)), desc)
        .filt(|(_, n)| n <= 10)
        .map(|((((u, _), _), _), _)| u)
        .collect();
    let pd = Ident::<Post>::new().with((&db.post.post_type_id).eq(1)).with((&db.post.view_count).gt(100));
    let mut v = Vec::new();
    (&top).select(Ident::<User>::new().and(&bu).and((&qc).opt()).and(posts_of(db).select(pd))).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(((u, b), q), p)| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep"), V::I(b), V::I(q.unwrap_or(0))];
        f.extend(post_fields(db, p, &["id", "title", "views", "score", "created"]));
        row(f)
    }))
}

// WITH UserBadges AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(B.Id) AS BadgeCount,
// SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM
// Users U
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// GROUP BY
// U.Id, U.DisplayName
// ),
// PostStats AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS PostCount,
// SUM(P.ViewCount) AS TotalViews,
// SUM(P.Score) AS TotalScore,
// AVG(P.Score) AS AvgScore
// FROM
// Posts P
// WHERE
// P.OwnerUserId IS NOT NULL
// GROUP BY
// P.OwnerUserId
// ),
// UserPostBadgeStats AS (
// SELECT
// U.DisplayName,
// U.BadgeCount,
// U.GoldBadges,
// U.SilverBadges,
// U.BronzeBadges,
// P.PostCount,
// P.TotalViews,
// P.TotalScore,
// P.AvgScore
// FROM
// UserBadges U
// JOIN
// PostStats P ON U.UserId = P.OwnerUserId
// )
// SELECT
// DisplayName,
// BadgeCount,
// GoldBadges,
// SilverBadges,
// BronzeBadges,
// PostCount,
// TotalViews,
// TotalScore,
// AvgScore
// FROM
// UserPostBadgeStats
// WHERE
// BadgeCount > 0 AND PostCount > 10
// ORDER BY
// TotalScore DESC, TotalViews DESC
// LIMIT 10;
fn q25679(db: &'static So) -> String {
    let ub = g(db).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let pf = owned(db).group_by(&db.post.owner_user).select((&db.post.view_count).opt().and(&db.post.score)).fold([0i64; 4], |a, (w, s)| [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s]);
    let mut v = Vec::new();
    (&ub).and(&pf).filt(|(b, p): ([i64; 4], [i64; 4])| b[0] > 0 && p[0] > 10).drive(|u, (b, p)| v.push((u, b, p)));
    out(v, |&(_, _, p)| (Reverse(p[3]), (p[1] == 0, Reverse(p[2]))), 10, |&(u, b, p)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(ints(&b));
        f.extend([V::I(p[0]), nullable(p[2], p[1]), V::I(p[3]), avg(p[3], p[0])]);
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
// SUM(p.ViewCount) AS TotalViews,
// SUM(p.Score) AS TotalScore
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName
// ),
// UserBadges AS (
// SELECT
// UserId,
// COUNT(Id) AS TotalBadges,
// SUM(CASE WHEN Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM
// Badges
// GROUP BY
// UserId
// ),
// UserVoteStats AS (
// SELECT
// v.UserId,
// COUNT(v.Id) AS TotalVotes,
// SUM(CASE WHEN vt.Id = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN vt.Id = 3 THEN 1 ELSE 0 END) AS DownVotes
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
// ups.TotalViews,
// ups.TotalScore,
// ub.TotalBadges,
// ub.GoldBadges,
// ub.SilverBadges,
// ub.BronzeBadges,
// uvs.TotalVotes,
// uvs.UpVotes,
// uvs.DownVotes
// FROM
// UserPostStats ups
// LEFT JOIN
// UserBadges ub ON ups.UserId = ub.UserId
// LEFT JOIN
// UserVoteStats uvs ON ups.UserId = uvs.UserId
// WHERE
// ups.TotalPosts > 0
// ORDER BY
// ups.TotalScore DESC, ups.TotalViews DESC;
fn q25812(db: &'static So) -> String {
    let bc = badge_classes(db);
    let uv = db.vote.group_by(&db.vote.user).select((&db.vote.vote_type).select(&db.vote_type.origid)).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64]);
    let mut v = Vec::new();
    upqa(db).filt(|a: [i64; 6]| a[0] > 0).and((&bc).opt()).and((&uv).opt()).drive(|u, ((a, b), x)| v.push((u, a, b, x)));
    rows(v.iter().map(|&(u, a, b, x)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[..3]));
        f.extend([nullable(a[4], a[3]), V::I(a[5])]);
        f.extend((0..4).map(|i| oint(b.map(|b| b[i]))));
        f.extend((0..3).map(|i| oint(x.map(|x| x[i]))));
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
// SUM(p.ViewCount) AS TotalViews,
// SUM(p.Score) AS TotalScore
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName
// ),
// UserBadges AS (
// SELECT
// b.UserId,
// COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldBadges,
// COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverBadges,
// COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeBadges
// FROM
// Badges b
// GROUP BY
// b.UserId
// ),
// PostVoteStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(v.Id) AS TotalVotes,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Posts p
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.OwnerUserId
// )
// SELECT
// ups.UserId,
// ups.DisplayName,
// ups.TotalPosts,
// ups.TotalQuestions,
// ups.TotalAnswers,
// ups.TotalViews,
// ups.TotalScore,
// ub.GoldBadges,
// ub.SilverBadges,
// ub.BronzeBadges,
// pvs.TotalVotes,
// pvs.UpVotes,
// pvs.DownVotes
// FROM
// UserPostStats ups
// LEFT JOIN
// UserBadges ub ON ups.UserId = ub.UserId
// LEFT JOIN
// PostVoteStats pvs ON ups.UserId = pvs.OwnerUserId
// WHERE
// ups.TotalPosts > 0
// ORDER BY
// ups.TotalScore DESC,
// ups.TotalPosts DESC
// LIMIT 100;
fn q25918(db: &'static So) -> String {
    let bc = badge_classes(db);
    let pv = owned(db).group_by(&db.post.owner_user).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| {
        [a[0] + t.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let mut v = Vec::new();
    upqa(db).filt(|a: [i64; 6]| a[0] > 0).and((&bc).opt()).and((&pv).opt()).drive(|u, ((a, b), x)| v.push((u, a, b, x)));
    out(v, |&(_, a, _, _)| (Reverse(a[5]), Reverse(a[0])), 100, |&(u, a, b, x)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[..3]));
        f.extend([nullable(a[4], a[3]), V::I(a[5])]);
        f.extend((1..4).map(|i| oint(b.map(|b| b[i]))));
        f.extend((0..3).map(|i| oint(x.map(|x| x[i]))));
        f
    })
}

// WITH PostDetails AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.Body,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// p.AnswerCount,
// p.CommentCount,
// p.Tags,
// u.DisplayName AS OwnerDisplayName,
// array_length(string_to_array(substring(p.Tags, 2, length(p.Tags)-2), '> <'), 1) AS TagCount,
// COALESCE((
// SELECT COUNT(*)
// FROM PostHistory ph
// WHERE ph.PostId = p.Id AND ph.PostHistoryTypeId = 10
// ), 0) AS CloseCount,
// COALESCE((
// SELECT COUNT(*)
// FROM PostHistory ph
// WHERE ph.PostId = p.Id AND ph.PostHistoryTypeId IN (10, 11)
// ), 0) AS ClosureReopenCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.ViewCount > 100
// )
// SELECT
// pd.*,
// CASE
// WHEN pd.CloseCount > 0 THEN 'Closed'
// WHEN pd.ClosureReopenCount > 0 THEN 'Reopened'
// ELSE 'Active'
// END AS PostStatus,
// ROUND(EXTRACT(EPOCH FROM (cast('2024-10-01 12:34:56' as timestamp) - pd.CreationDate)) / 3600, 2) AS HoursSinceCreation,
// pd.TagCount * pd.Score AS TagScoreImpact
// FROM
// PostDetails pd
// ORDER BY
// pd.Score DESC,
// pd.ViewCount DESC
// LIMIT 50;
fn q26182(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let h10 = history_of_types(db, &[10]);
    let h1011 = history_of_types(db, &[10, 11]);
    let tag_count = |p: Id<Post>| {
        db.post.tags_str.get(p).map(|s| {
            let n = s.chars().count();
            let inner: String = s.chars().skip(1).take(n.saturating_sub(2)).collect();
            inner.split("> <").count() as i64
        })
    };
    let mut v = Vec::new();
    owned(db).with((&db.post.view_count).gt(100)).select(Ident::<Post>::new().and(&h10).and(&h1011)).drive(|_, x| v.push(x));
    out(v, |&((p, _), _)| score_views(db, p), 50, |&((p, c), r)| {
        let tc = tag_count(p);
        let mut f = post_fields(db, p, &["id", "title", "body", "created", "views", "score", "answers", "comments", "tags", "owner"]);
        f.extend([oint(tc), V::I(c), V::I(r)]);
        f.push(V::S(if c > 0 {
            "Closed"
        } else if r > 0 {
            "Reopened"
        } else {
            "Active"
        }));
        f.push(V::F(round2(hours_to(t0, db.post.creation_date.get(p).unwrap()) / 3600.0)));
        f.push(oint(tc.map(|t| t * db.post.score.get(p).unwrap())));
        f
    })
}

// WITH UserBadgeSummary AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(B.Id) AS BadgeCount,
// SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM
// Users U
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// GROUP BY
// U.Id, U.DisplayName
// ),
// PostStatistics AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS TotalPosts,
// COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS Questions,
// COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS Answers,
// COUNT(CASE WHEN P.PostTypeId IN (4, 5) THEN 1 END) AS TagWikis,
// SUM(P.ViewCount) AS TotalViews,
// SUM(P.Score) AS TotalScore
// FROM
// Posts P
// GROUP BY
// P.OwnerUserId
// ),
// UserPostBadgeSummary AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// UBS.BadgeCount,
// UBS.GoldBadges,
// UBS.SilverBadges,
// UBS.BronzeBadges,
// PS.TotalPosts,
// PS.Questions,
// PS.Answers,
// PS.TagWikis,
// PS.TotalViews,
// PS.TotalScore
// FROM
// UserBadgeSummary UBS
// JOIN
// PostStatistics PS ON UBS.UserId = PS.OwnerUserId
// JOIN
// Users U ON UBS.UserId = U.Id
// )
// SELECT
// UserId,
// DisplayName,
// BadgeCount,
// GoldBadges,
// SilverBadges,
// BronzeBadges,
// TotalPosts,
// Questions,
// Answers,
// TagWikis,
// TotalViews,
// TotalScore,
// CASE
// WHEN TotalPosts > 0 THEN TotalScore / CAST(TotalPosts AS FLOAT)
// ELSE 0
// END AS AvgScorePerPost
// FROM
// UserPostBadgeSummary
// ORDER BY
// AvgScorePerPost DESC,
// BadgeCount DESC
// LIMIT 10;
fn q26327(db: &'static So) -> String {
    let ub = g(db).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let Post { post_type_id, view_count, score, .. } = &db.post;
    let ps = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(view_count.opt()).and(score)).fold([0i64; 7], |a, ((t, w), s)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + matches!(t, 4 | 5) as i64, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0), a[6] + s]
    });
    let per = |p: [i64; 7]| if p[0] > 0 { (p[6] as f32 / p[0] as f32) as f64 } else { 0.0 };
    let mut v = Vec::new();
    (&ub).and(&ps).drive(|u, (b, p)| v.push((u, b, p)));
    out(v, |&(_, b, p)| (Reverse(fkey(per(p))), Reverse(b[0])), 10, |&(u, b, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&b));
        f.extend(ints(&p[..4]));
        f.extend([nullable(p[5], p[4]), V::I(p[6]), V::F(per(p))]);
        f
    })
}

// WITH UserBadgeStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges,
// COUNT(b.Id) AS TotalBadges
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// u.Id, u.DisplayName
// ),
// PostStat AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(p.ViewCount) AS TotalViews,
// AVG(p.Score) AS AverageScore
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// ),
// UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// us.TotalPosts,
// us.TotalQuestions,
// us.TotalAnswers,
// us.TotalViews,
// us.AverageScore,
// u.Reputation,
// u.CreationDate
// FROM
// Users u
// LEFT JOIN
// PostStat us ON u.Id = us.OwnerUserId
// )
// SELECT
// ups.DisplayName,
// ups.Reputation,
// ups.CreationDate,
// ups.TotalPosts,
// ups.TotalQuestions,
// ups.TotalAnswers,
// ups.TotalViews,
// ups.AverageScore,
// ub.GoldBadges,
// ub.SilverBadges,
// ub.BronzeBadges,
// ub.TotalBadges
// FROM
// UserPostStats ups
// LEFT JOIN
// UserBadgeStats ub ON ups.UserId = ub.UserId
// WHERE
// ups.Reputation > 1000
// ORDER BY
// ups.TotalViews DESC, ups.TotalPosts DESC, ups.DisplayName ASC
// LIMIT 10;
fn q26388(db: &'static So) -> String {
    let Post { post_type_id, view_count, score, .. } = &db.post;
    let ps = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(view_count.opt()).and(score)).fold([0i64; 6], |a, ((t, w), s)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0), a[5] + s]
    });
    let ub = g(db).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64, a[3] + 1],
        None => a,
    });
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and((&ps).opt()).and(&ub)).drive(|_, x| v.push(x));
    let tv = |p: Option<[i64; 6]>| p.filter(|p| p[3] > 0).map(|p| p[4]);
    out(
        v,
        |&((u, p), _)| ((tv(p).is_none(), Reverse(tv(p))), (p.is_none(), Reverse(p.map(|p| p[0]))), db.user.display_name.get(u).unwrap()),
        10,
        |&((u, p), b)| {
            let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep"), user_col(db, u, "ucreated")];
            f.extend((0..3).map(|i| oint(p.map(|p| p[i]))));
            f.extend([oint(tv(p)), p.map_or(V::Null, |p| avg(p[5], p[0]))]);
            f.extend(ints(&b));
            f
        },
    )
}

pub static ENTRIES: &[harness::Entry] = &[
    ("14814", q14814),
    ("14816", q14816),
    ("14819", q14819),
    ("14835", q14835),
    ("14850", q14850),
    ("14856", q14856),
    ("14860", q14860),
    ("14864", q14864),
    ("14867", q14867),
    ("14876", q14876),
    ("14885", q14885),
    ("14890", q14890),
    ("14903", q14903),
    ("14913", q14913),
    ("14920", q14920),
    ("14928", q14928),
    ("14932", q14932),
    ("14940", q14940),
    ("14944", q14944),
    ("14953", q14953),
    ("14964", q14964),
    ("14969", q14969),
    ("14990", q14990),
    ("14992", q14992),
    ("15276", q15276),
    ("15591", q15591),
    ("15896", q15896),
    ("17737", q17737),
    ("17913", q17913),
    ("18301", q18301),
    ("18638", q18638),
    ("18654", q18654),
    ("19611", q19611),
    ("2249", q2249),
    ("25231", q25231),
    ("25335", q25335),
    ("25396", q25396),
    ("25433", q25433),
    ("25554", q25554),
    ("25679", q25679),
    ("25812", q25812),
    ("25918", q25918),
    ("26182", q26182),
    ("26327", q26327),
    ("26388", q26388),
];
