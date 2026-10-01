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

// --- batch 118 --------------------------------------------------------------

// SELECT
// pt.Id AS PostTypeId,
// pt.Name AS PostTypeName,
// COUNT(p.Id) AS PostCount,
// COUNT(c.Id) AS CommentCount,
// COALESCE(AVG(v.TotalVotes), 0) AS AverageVotes
// FROM
// PostTypes pt
// LEFT JOIN
// Posts p ON p.PostTypeId = pt.Id
// LEFT JOIN
// Comments c ON c.PostId = p.Id
// LEFT JOIN
// (SELECT
// PostId, COUNT(Id) AS TotalVotes
// FROM
// Votes
// GROUP BY
// PostId) v ON v.PostId = p.Id
// GROUP BY
// pt.Id, pt.Name
// ORDER BY
// PostTypeId;
fn q11744(db: &'static So) -> String {
    let of_type: HashIdx<Id<PostType>, Id<Post>> = (&db.post.post_type).inv().collect();
    let vf = db.vote.group_by(&db.vote.post).fold(0i64, |a, _| a + 1);
    let f = db
        .post_type
        .group_by(Ident::<PostType>::new())
        .select((&of_type).select(comments_of(db).opt().and((&vf).opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some((c, x)) => [a[0] + 1, a[1] + c.is_some() as i64, a[2] + x.is_some() as i64, a[3] + x.unwrap_or(0)],
            None => a,
        });
    let mut v = Vec::new();
    (&f).drive(|t, a| v.push((t, a)));
    rows(v.iter().map(|&(t, a)| row(vec![V::I(db.post_type.origid.get(t).unwrap()), V::S(db.post_type.name.get(t).unwrap()), V::I(a[0]), V::I(a[1]), or0(a[3], a[2])])))
}

// WITH UserPostCount AS (
// SELECT
// u.Id AS UserId,
// COUNT(p.Id) AS PostCount,
// SUM(p.Score) AS TotalScore
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id
// ),
// BadgeCount AS (
// SELECT
// UserId,
// COUNT(Id) AS BadgeCount
// FROM
// Badges
// GROUP BY
// UserId
// ),
// FinalUserMetrics AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COALESCE(upc.PostCount, 0) AS PostCount,
// COALESCE(upc.TotalScore, 0) AS TotalScore,
// COALESCE(bc.BadgeCount, 0) AS BadgeCount,
// u.Reputation
// FROM
// Users u
// LEFT JOIN
// UserPostCount upc ON u.Id = upc.UserId
// LEFT JOIN
// BadgeCount bc ON u.Id = bc.UserId
// )
// SELECT
// UserId,
// DisplayName,
// PostCount,
// TotalScore,
// BadgeCount,
// Reputation,
// (PostCount + TotalScore + BadgeCount) AS PerformanceMetric
// FROM
// FinalUserMetrics
// ORDER BY
// PerformanceMetric DESC
// LIMIT 10;
fn q11760(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    upqa(db).and(&bu).drive(|u, (a, b)| v.push((u, a, b)));
    out(v, |&(_, a, b)| Reverse(a[0] + a[5] + b), 10, |&(u, a, b)| {
        vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(a[0]), V::I(a[5]), V::I(b), user_col(db, u, "rep"), V::I(a[0] + a[5] + b)]
    })
}

// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(P.Id) AS TotalPosts,
// AVG(COALESCE(P.Score, 0)) AS AverageScore,
// SUM(COALESCE(V.VoteCount, 0)) AS TotalVotes
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// (SELECT
// PostId,
// COUNT(Id) AS VoteCount
// FROM
// Votes
// GROUP BY
// PostId) V ON P.Id = V.PostId
// GROUP BY
// U.Id, U.DisplayName, U.Reputation
// ORDER BY
// TotalPosts DESC;
fn q11771(db: &'static So) -> String {
    let vf = db.vote.group_by(&db.vote.post).fold(0i64, |a, _| a + 1);
    let uf = g(db).select(posts_of(db).select((&db.post.score).and((&vf).opt())).opt()).fold([0i64; 4], |a, p| match p {
        Some((s, x)) => [a[0] + 1, a[1] + 1, a[2] + s, a[3] + x.unwrap_or(0)],
        None => [a[0], a[1] + 1, a[2], a[3]],
    });
    let mut v = Vec::new();
    (&uf).drive(|u, a| v.push((u, a)));
    rows(v.iter().map(|&(u, a)| row(vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep"), V::I(a[0]), avg(a[2], a[1]), V::I(a[3])])))
}

// WITH UserActivity AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN Votes v ON p.Id = v.PostId
// GROUP BY u.Id, u.DisplayName
// ),
// PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// COUNT(c.Id) AS CommentCount,
// COUNT(DISTINCT p2.Id) AS RelatedPostCount
// FROM Posts p
// LEFT JOIN Comments c ON p.Id = c.PostId
// LEFT JOIN PostLinks pl ON p.Id = pl.PostId
// LEFT JOIN Posts p2 ON pl.RelatedPostId = p2.Id
// GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount
// )
// SELECT
// ua.UserId,
// ua.DisplayName,
// ua.PostCount,
// ua.QuestionCount,
// ua.AnswerCount,
// ua.UpVotes,
// ua.DownVotes,
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.Score,
// ps.ViewCount,
// ps.CommentCount,
// ps.RelatedPostCount
// FROM UserActivity ua
// JOIN PostStats ps ON ua.UserId = ps.PostId
// ORDER BY ua.UserId DESC, ps.Score DESC;
fn q11787(db: &'static So) -> String {
    let uid = uids(db);
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let pc = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(links_of(db).opt())).fold(0i64, |a, (c, _)| a + c.is_some() as i64);
    let rel = per_post_distinct(db, links_of(db).select(&db.post_link.related_post));
    let mut v = Vec::new();
    db.post
        .with((&db.post.origid).select(&uid))
        .select(Ident::<Post>::new().and(&pc).and((&rel).opt()).and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&us).and((&dp).opt()))))
        .drive(|_, x| v.push(x));
    rows(v.iter().map(|&(((p, c), r), ((u, a), d))| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(d.unwrap_or(0)), V::I(a.q), V::I(a.a), V::I(a.up), V::I(a.down)];
        f.extend(post_fields(db, p, &["id", "title", "created", "score", "views"]));
        f.extend([V::I(c), V::I(r.unwrap_or(0))]);
        row(f)
    }))
}

// WITH Benchmark AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// U.DisplayName AS OwnerDisplayName,
// P.CreationDate,
// P.Score,
// P.ViewCount,
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
// P.Id, P.Title, U.DisplayName, P.CreationDate, P.Score, P.ViewCount
// )
// SELECT
// B.*,
// (SELECT COUNT(*) FROM Posts WHERE ParentId = B.PostId) AS AnswerCount
// FROM
// Benchmark B
// ORDER BY
// B.Score DESC, B.ViewCount DESC
// LIMIT 100;
fn q11802(db: &'static So) -> String {
    let mut v = Vec::new();
    stats_fold(db, db.post.iq(), Ident::<Post>::new(), "cv", &[]).and(answers_per_post(db)).drive(|p, (s, a)| v.push((p, s, a)));
    out(v, |&(p, _, _)| score_views(db, p), 100, |&(p, s, a)| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "score", "views"]);
        f.extend([V::I(s.cx), V::I(s.vx), V::I(a)]);
        f
    })
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(DISTINCT P.Id) AS PostCount,
// SUM(COALESCE(V.VoteCount, 0)) AS TotalVotes,
// SUM(COALESCE(B.Count, 0)) AS TotalBadges
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN (
// SELECT
// PostId,
// COUNT(*) AS VoteCount
// FROM
// Votes
// GROUP BY
// PostId
// ) V ON P.Id = V.PostId
// LEFT JOIN (
// SELECT
// UserId,
// COUNT(*) AS Count
// FROM
// Badges
// GROUP BY
// UserId
// ) B ON U.Id = B.UserId
// GROUP BY
// U.Id, U.DisplayName
// )
// SELECT
// UserId,
// DisplayName,
// PostCount,
// TotalVotes,
// TotalBadges
// FROM
// UserStats
// ORDER BY
// PostCount DESC, TotalVotes DESC
// LIMIT 10;
fn q11815(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let uf = g(db).select(posts_of(db).select(votes_per_post(db)).opt().and(&bu)).fold([0i64; 3], |a, (x, b)| match x {
        Some(x) => [a[0] + 1, a[1] + x, a[2] + b],
        None => [a[0], a[1], a[2] + b],
    });
    let mut v = Vec::new();
    (&uf).drive(|u, a| v.push((u, a)));
    out(v, |&(_, a)| (Reverse(a[0]), Reverse(a[1])), 10, |&(u, a)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a));
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
// u.Id AS UserId,
// u.DisplayName AS UserDisplayName,
// u.Reputation,
// u.Views AS UserViews,
// COALESCE(badgeCount.BadgeCount, 0) AS TotalBadges,
// COALESCE(voteCount.UpVotes, 0) AS TotalUpVotes,
// COALESCE(voteCount.DownVotes, 0) AS TotalDownVotes
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// (SELECT
// UserId,
// COUNT(*) AS BadgeCount
// FROM
// Badges
// GROUP BY
// UserId) badgeCount ON u.Id = badgeCount.UserId
// LEFT JOIN
// (SELECT
// PostId,
// SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Votes
// GROUP BY
// PostId) voteCount ON p.Id = voteCount.PostId
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// ORDER BY
// p.CreationDate DESC;
fn q11819(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let pv = post_votes(db);
    let mut v = Vec::new();
    owned_since(db, year_ago()).select(Ident::<Post>::new().and((&db.post.owner_user).select(&bu)).and((&pv).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((p, b), x)| {
        let x = x.unwrap_or([0; 3]);
        let u = db.post.owner_user.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "answers", "comments"]);
        f.extend(["uid", "name", "rep", "uviews"].iter().map(|c| user_col(db, u, c)));
        f.extend([V::I(b), V::I(x[1]), V::I(x[2])]);
        row(f)
    }))
}

// WITH UserPostStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(P.Id) AS TotalPosts,
// COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS TotalQuestions,
// COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS TotalAnswers,
// AVG(P.Score) AS AvgScore,
// SUM(P.ViewCount) AS TotalViews
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id, U.DisplayName
// ),
// UserBadgeStats AS (
// SELECT
// B.UserId,
// COUNT(B.Id) AS TotalBadges,
// SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM
// Badges B
// GROUP BY
// B.UserId
// )
// SELECT
// UPS.UserId,
// UPS.DisplayName,
// UPS.TotalPosts,
// UPS.TotalQuestions,
// UPS.TotalAnswers,
// UPS.AvgScore,
// UPS.TotalViews,
// COALESCE(UBS.TotalBadges, 0) AS TotalBadges,
// COALESCE(UBS.GoldBadges, 0) AS GoldBadges,
// COALESCE(UBS.SilverBadges, 0) AS SilverBadges,
// COALESCE(UBS.BronzeBadges, 0) AS BronzeBadges
// FROM
// UserPostStats UPS
// LEFT JOIN
// UserBadgeStats UBS ON UPS.UserId = UBS.UserId
// ORDER BY
// UPS.TotalPosts DESC;
fn q11826(db: &'static So) -> String {
    let bc = badge_classes(db);
    let mut v = Vec::new();
    upqa(db).and((&bc).opt()).drive(|u, (a, b)| v.push((u, a, b.unwrap_or([0; 4]))));
    rows(v.iter().map(|&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[..3]));
        f.extend([avg(a[5], a[0]), nullable(a[4], a[3])]);
        f.extend(ints(&b));
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
// COUNT(DISTINCT a.Id) AS AnswerCount,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 6 THEN 1 ELSE 0 END), 0) AS CloseVotes,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 7 THEN 1 ELSE 0 END), 0) AS ReopenVotes
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Posts a ON p.Id = a.ParentId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.Score,
// ps.ViewCount,
// ps.CommentCount,
// ps.AnswerCount,
// ps.UpVotes,
// ps.DownVotes,
// ps.CloseVotes,
// ps.ReopenVotes
// FROM
// PostStatistics ps
// ORDER BY
// ps.ViewCount DESC, ps.Score DESC;
fn q11837(db: &'static So) -> String {
    let c = per_post_distinct(db, comments_of(db));
    let a = per_post_distinct(db, children_of(db));
    rows(stats_with(db, since(db, year_ago()), "cva", &[], &[&c, &a]).iter().map(|&(p, s, d)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(d[0]), V::I(d[1]), V::I(s.up), V::I(s.down), V::I(s.by_vt[6]), V::I(s.by_vt[7])]);
        row(f)
    }))
}

// WITH PostStats AS (
// SELECT
// COUNT(*) AS TotalPosts,
// SUM(CASE WHEN PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN ParentId IS NOT NULL THEN 1 ELSE 0 END) AS TotalComments
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
// CommentStats AS (
// SELECT
// COUNT(*) AS TotalComments
// FROM
// Comments
// )
// SELECT
// (SELECT TotalPosts FROM PostStats) AS TotalPosts,
// (SELECT TotalQuestions FROM PostStats) AS TotalQuestions,
// (SELECT TotalAnswers FROM PostStats) AS TotalAnswers,
// (SELECT TotalComments FROM PostStats) AS TotalComments,
// (SELECT TotalUsers FROM UserStats) AS TotalUsers,
// (SELECT AverageReputation FROM UserStats) AS AverageReputation,
// (SELECT TotalComments FROM CommentStats) AS TotalComments
fn q11845(db: &'static So) -> String {
    let p = db.post.select((&db.post.post_type_id).and((&db.post.parent_id).opt())).fold_flat([0i64; 4], |a, (t, pa)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + pa.is_some() as i64]
    });
    let (un, rs) = db.user.select(&db.user.reputation).fold_flat((0i64, 0i64), |(n, s), r| (n + 1, s + r));
    row(vec![V::I(p[0]), V::I(p[1]), V::I(p[2]), V::I(p[3]), V::I(un), avg(rs, un), V::I(count(db.comment.iq()))])
}

// WITH PostStats AS (
// SELECT
// p.PostTypeId,
// COUNT(p.Id) AS TotalPosts,
// COALESCE(SUM(c.Score), 0) AS TotalComments,
// COALESCE(AVG(p.Score), 0) AS AvgScore,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpVotes,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownVotes
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.PostTypeId
// )
// SELECT
// pt.Name AS PostTypeName,
// ps.TotalPosts,
// ps.TotalComments,
// ps.AvgScore,
// ps.TotalUpVotes,
// ps.TotalDownVotes
// FROM
// PostTypes pt
// LEFT JOIN
// PostStats ps ON pt.Id = ps.PostTypeId
// ORDER BY
// ps.TotalPosts DESC;
fn q11851(db: &'static So) -> String {
    let ps = by_key(
        db.post.iq(),
        &db.post.post_type_id,
        (&db.post.score).and(comments_of(db).select(&db.comment.score).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()),
        [0i64; 5],
        |a, ((s, c), t)| [a[0] + 1, a[1] + c.unwrap_or(0), a[2] + s, a[3] + (t == Some(2)) as i64, a[4] + (t == Some(3)) as i64],
    );
    let mut v = Vec::new();
    db.post_type.select(Ident::<PostType>::new().and((&db.post_type.origid).select(&ps).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(t, a)| {
        let mut f = vec![V::S(db.post_type.name.get(t).unwrap())];
        match a {
            Some(a) => f.extend([V::I(a[0]), V::I(a[1]), avg(a[2], a[0]), V::I(a[3]), V::I(a[4])]),
            None => f.extend(nulls(5)),
        }
        row(f)
    }))
}

// WITH Benchmark AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// COUNT(DISTINCT b.Id) AS BadgeCount,
// EXTRACT(EPOCH FROM (cast('2024-10-01 12:34:56' as timestamp) - p.CreationDate)) AS TimeSinceCreation
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Badges b ON p.OwnerUserId = b.UserId
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score
// )
// SELECT
// AVG(TimeSinceCreation) AS AvgTimeSinceCreation,
// AVG(ViewCount) AS AvgViewCount,
// AVG(Score) AS AvgScore,
// AVG(CommentCount) AS AvgCommentCount,
// AVG(VoteCount) AS AvgVoteCount,
// AVG(BadgeCount) AS AvgBadgeCount
// FROM
// Benchmark;
fn q11875(db: &'static So) -> String {
    let Post { view_count, score, creation_date, .. } = &db.post;
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let b = per_post_distinct(db, (&db.post.owner_user).select(badges_of(db)));
    let pf = stats_fold(db, questions_only(db), Ident::<Post>::new(), "cvb", &[]);
    let (a, e) = questions_only(db)
        .select(view_count.opt().and(score).and(creation_date).and(&pf).and((&b).opt()))
        .fold_flat(([0i64; 7], 0i128), |(a, e), ((((w, s), cd), st), b)| {
            ([a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s, a[4] + st.cx, a[5] + st.vx, a[6] + b.unwrap_or(0)], e + (t0 - cd) as i128)
        });
    row(vec![V::F(e as f64 / a[0] as f64 / 1e6), avg(a[2], a[1]), avg(a[3], a[0]), avg(a[4], a[0]), avg(a[5], a[0]), avg(a[6], a[0])])
}

// WITH UserActivity AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// COUNT(DISTINCT c.Id) AS TotalComments,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
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
// ua.UserId,
// ua.DisplayName,
// ua.TotalPosts,
// ua.TotalComments,
// ua.TotalUpVotes,
// ua.TotalDownVotes,
// COALESCE(ROUND(CAST(ua.TotalUpVotes AS NUMERIC) / NULLIF(ua.TotalPosts, 0), 2), 0) AS UpvotePercentage,
// COALESCE(ROUND(CAST(ua.TotalDownVotes AS NUMERIC) / NULLIF(ua.TotalPosts, 0), 2), 0) AS DownvotePercentage
// FROM
// UserActivity ua
// ORDER BY
// ua.TotalPosts DESC,
// ua.TotalUpVotes DESC;
fn q11883(db: &'static So) -> String {
    rows(users_with_counts(db, "cv", false).iter().map(|r| {
        let a = r.agg;
        let pct = |x: i64| if a.n == 0 { V::F(0.0) } else { V::F(round2(x as f64 / a.n as f64)) };
        let mut f = user_fields(r, "cv", &["uid", "name", "#n", "#c", "#up", "#down"]);
        f.extend([pct(a.up), pct(a.down)]);
        row(f)
    }))
}

// SELECT
// pt.Name AS PostType,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// COUNT(v.Id) AS TotalVotes,
// COUNT(c.Id) AS TotalComments,
// AVG(p.Score) AS AverageScore,
// AVG(p.ViewCount) AS AverageViewCount
// FROM
// Posts p
// LEFT JOIN
// VoteTypes vt ON vt.Id = (SELECT VoteTypeId FROM Votes WHERE PostId = p.Id LIMIT 1)
// LEFT JOIN
// Votes v ON v.PostId = p.Id
// LEFT JOIN
// Comments c ON c.PostId = p.Id
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// GROUP BY
// pt.Name
// ORDER BY
// TotalPosts DESC;
fn q11887(db: &'static So) -> String {
    let f = by_key(db.post.iq(), name(db), (&db.post.score).and((&db.post.view_count).opt()).and(votes_of(db).opt()).and(comments_of(db).opt()), [0i64; 6], |a, (((s, w), x), c)| {
        [a[0] + x.is_some() as i64, a[1] + c.is_some() as i64, a[2] + 1, a[3] + s, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0)]
    });
    let np = db.post.group_by(name(db)).fold(0i64, |a, _| a + 1);
    let mut v = Vec::new();
    (&f).and(&np).drive(|k, (a, n)| v.push((k, a, n)));
    rows(v.iter().map(|&(k, a, n)| row(vec![V::S(k), V::I(n), V::I(a[0]), V::I(a[1]), avg(a[3], a[2]), avg(a[5], a[4])])))
}

// WITH PostStats AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.Score,
// P.ViewCount,
// COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
// COUNT(DISTINCT A.Id) AS AnswerCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Posts A ON P.Id = A.ParentId AND A.PostTypeId = 2
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// WHERE
// P.PostTypeId = 1
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
// PS.AnswerCount,
// PS.UpVotes,
// PS.DownVotes,
// (PS.UpVotes - PS.DownVotes) AS NetScore
// FROM
// PostStats PS
// ORDER BY
// PS.CreationDate DESC
// LIMIT 100;
fn q11894(db: &'static So) -> String {
    let a = per_post_distinct(db, answers_of(db));
    let mut v = Vec::new();
    stats_fold(db, questions_only(db), Ident::<Post>::new(), "cvA", &[]).and((&a).opt()).drive(|p, (s, a)| v.push((p, s, a.unwrap_or(0))));
    out(v, |&(p, _, _)| newest(db, p), 100, |&(p, s, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(s.cx), V::I(a), V::I(s.up), V::I(s.down), V::I(s.up - s.down)]);
        f
    })
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(c.CommentCount) AS TotalComments,
// COALESCE(SUM(v.UpVotes), 0) AS TotalUpVotes,
// COALESCE(SUM(v.DownVotes), 0) AS TotalDownVotes,
// MAX(p.CreationDate) AS LastPostDate,
// MAX(p.Score) AS HighestPostScore,
// AVG(P.ViewCount) AS AverageViewCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
// LEFT JOIN
// (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM Votes GROUP BY PostId) v ON p.Id = v.PostId
// GROUP BY
// u.Id, u.DisplayName
// ORDER BY
// PostCount DESC;
fn q11905(db: &'static So) -> String {
    let Post { post_type_id, creation_date, score, view_count, .. } = &db.post;
    let cf = db.comment.group_by(&db.comment.post).fold(0i64, |a, _| a + 1);
    let vf = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let uf = g(db)
        .select(posts_of(db).select(post_type_id.and(creation_date).and(score).and(view_count.opt()).and((&cf).opt()).and((&vf).opt())).opt())
        .fold([0, 0, 0, 0, 0, 0, 0, i64::MIN, i64::MIN, 0, 0], |a: [i64; 11], p| match p {
            Some((((((t, cd), s), w), c), x)) => {
                let x = x.unwrap_or([0; 2]);
                [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + c.is_some() as i64, a[4] + c.unwrap_or(0), a[5] + x[0], a[6] + x[1], a[7].max(cd), a[8].max(s), a[9] + w.is_some() as i64, a[10] + w.unwrap_or(0)]
            }
            None => a,
        });
    let mut v = Vec::new();
    (&uf).drive(|u, a| v.push((u, a)));
    rows(v.iter().map(|&(u, a)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[..3]));
        f.extend([nullable(a[4], a[3]), V::I(a[5]), V::I(a[6]), if a[0] == 0 { V::Null } else { V::T(a[7]) }, omax(a[8], a[0]), avg(a[10], a[9])]);
        row(f)
    }))
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// COUNT(b.Id) AS BadgeCount,
// SUM(p.ViewCount) AS TotalViews,
// SUM(p.Score) AS TotalScore
// FROM Users u
// LEFT JOIN Badges b ON u.Id = b.UserId
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// GROUP BY u.Id, u.Reputation
// ),
// PostStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS PostCount,
// SUM(COALESCE(p.AnswerCount, 0)) AS TotalAnswers,
// SUM(COALESCE(p.CommentCount, 0)) AS TotalComments
// FROM Posts p
// GROUP BY p.OwnerUserId
// ),
// FinalStats AS (
// SELECT
// us.UserId,
// us.Reputation,
// us.BadgeCount,
// us.TotalViews,
// us.TotalScore,
// COALESCE(ps.PostCount, 0) AS PostCount,
// COALESCE(ps.TotalAnswers, 0) AS TotalAnswers,
// COALESCE(ps.TotalComments, 0) AS TotalComments
// FROM UserStats us
// LEFT JOIN PostStats ps ON us.UserId = ps.OwnerUserId
// )
// SELECT
// UserId,
// Reputation,
// BadgeCount,
// TotalViews,
// TotalScore,
// PostCount,
// TotalAnswers,
// TotalComments
// FROM FinalStats
// ORDER BY Reputation DESC;
fn q11909(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "b", any_post);
    let pf = owned(db).group_by(&db.post.owner_user).select((&db.post.answer_count).opt().and(&db.post.comment_count)).fold([0i64; 3], |a, (an, cc)| [a[0] + 1, a[1] + an.unwrap_or(0), a[2] + cc]);
    let mut v = Vec::new();
    (&us).and((&pf).opt()).drive(|u, (a, p)| v.push((u, a, p.unwrap_or([0; 3]))));
    rows(v.iter().map(|&(u, a, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep"), V::I(a.bx), ustat_field(&a, "views_sum"), ustat_field(&a, "score_sum")];
        f.extend(ints(&p));
        row(f)
    }))
}

// WITH RecentUserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// u.CreationDate,
// u.LastAccessDate,
// COUNT(DISTINCT p.Id) AS RecentPostCount,
// COUNT(DISTINCT c.Id) AS RecentCommentCount,
// SUM(v.BountyAmount) AS TotalBounties
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId AND p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'
// LEFT JOIN
// Comments c ON u.Id = c.UserId AND c.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'
// LEFT JOIN
// Votes v ON u.Id = v.UserId
// GROUP BY
// u.Id, u.DisplayName, u.Reputation, u.CreationDate, u.LastAccessDate
// )
// SELECT
// *
// FROM
// RecentUserStats
// ORDER BY
// Reputation DESC
// LIMIT 100;
fn q11913(db: &'static So) -> String {
    let d = ts(2024, 9, 1, 12, 34, 56);
    let rp = posts_of(db).select(Ident::<Post>::new().with((&db.post.creation_date).gt(d)));
    let rc = comments_by(db).select(Ident::<Comment>::new().with((&db.comment.creation_date).gt(d)));
    let uf = g(db).select((&rp).opt().and((&rc).opt()).and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt())).fold([0i64; 2], |a, ((_, _), b)| {
        let b = b.flatten();
        [a[0] + b.is_some() as i64, a[1] + b.unwrap_or(0)]
    });
    let dp = ud(db, UserWhere::All, &rp);
    let dc = ud(db, UserWhere::All, &rc);
    let mut v = Vec::new();
    (&uf).and((&dp).opt()).and((&dc).opt()).drive(|u, ((a, p), c)| v.push((u, a, p.unwrap_or(0), c.unwrap_or(0))));
    out(v, |&(u, _, _, _)| rep_desc(db, u), 100, |&(u, a, p, c)| {
        let mut f: Vec<V> = ["uid", "name", "rep", "ucreated", "last_access"].iter().map(|k| user_col(db, u, k)).collect();
        f.extend([V::I(p), V::I(c), nullable(a[1], a[0])]);
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
// p.AnswerCount,
// p.CommentCount,
// COALESCE(u.DisplayName, 'Community User') AS OwnerDisplayName,
// p.LastActivityDate,
// p.LastEditDate
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// ),
// VoteStats AS (
// SELECT
// PostId,
// COUNT(CASE WHEN VoteTypeId = 2 THEN 1 END) AS UpVotes,
// COUNT(CASE WHEN VoteTypeId = 3 THEN 1 END) AS DownVotes,
// COUNT(*) AS TotalVotes
// FROM
// Votes
// GROUP BY
// PostId
// ),
// CombinedStats AS (
// SELECT
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.Score,
// ps.ViewCount,
// ps.AnswerCount,
// ps.CommentCount,
// ps.OwnerDisplayName,
// ps.LastActivityDate,
// ps.LastEditDate,
// COALESCE(vs.UpVotes, 0) AS UpVotes,
// COALESCE(vs.DownVotes, 0) AS DownVotes,
// COALESCE(vs.TotalVotes, 0) AS TotalVotes
// FROM
// PostStats ps
// LEFT JOIN
// VoteStats vs ON ps.PostId = vs.PostId
// )
// SELECT
// *,
// (ViewCount / NULLIF(TotalVotes, 0)) AS ViewPerVoteRatio,
// (UpVotes / NULLIF(ViewCount, 0)) AS UpVoteRatio,
// (DownVotes / NULLIF(ViewCount, 0)) AS DownVoteRatio
// FROM
// CombinedStats
// ORDER BY
// Score DESC
// LIMIT 10;
fn q11922(db: &'static So) -> String {
    let pv = post_votes(db);
    let mut v = Vec::new();
    db.post.select(Ident::<Post>::new().and((&pv).opt())).drive(|_, x| v.push(x));
    out(v, |&(p, _)| score_desc(db, p), 10, |&(p, x)| {
        let x = x.unwrap_or([0; 3]);
        let w = db.post.view_count.get(p);
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments"]);
        f.push(named_owner(db, p, "Community User"));
        f.extend(post_fields(db, p, &["activity", "edited"]));
        f.extend([V::I(x[1]), V::I(x[2]), V::I(x[0])]);
        f.push(match w {
            Some(w) if x[0] != 0 => V::F(w as f64 / x[0] as f64),
            _ => V::Null,
        });
        for n in [x[1], x[2]] {
            f.push(match w {
                Some(w) if w != 0 => V::F(n as f64 / w as f64),
                _ => V::Null,
            });
        }
        f
    })
}

// WITH PostStatistics AS (
// SELECT
// COUNT(*) AS TotalPosts,
// SUM(CASE WHEN PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(ViewCount) AS TotalViews,
// AVG(Score) AS AverageScore
// FROM
// Posts
// ),
// UserStatistics AS (
// SELECT
// COUNT(*) AS TotalUsers,
// AVG(Reputation) AS AverageReputation,
// MAX(Reputation) AS MaxReputation,
// MIN(Reputation) AS MinReputation
// FROM
// Users
// ),
// BadgeStatistics AS (
// SELECT
// COUNT(*) AS TotalBadges,
// SUM(CASE WHEN Class = 1 THEN 1 ELSE 0 END) AS TotalGoldBadges,
// SUM(CASE WHEN Class = 2 THEN 1 ELSE 0 END) AS TotalSilverBadges,
// SUM(CASE WHEN Class = 3 THEN 1 ELSE 0 END) AS TotalBronzeBadges
// FROM
// Badges
// ),
// VoteStatistics AS (
// SELECT
// COUNT(*) AS TotalVotes,
// SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
// FROM
// Votes
// )
// SELECT
// (SELECT TotalPosts FROM PostStatistics) AS TotalPosts,
// (SELECT TotalQuestions FROM PostStatistics) AS TotalQuestions,
// (SELECT TotalAnswers FROM PostStatistics) AS TotalAnswers,
// (SELECT TotalViews FROM PostStatistics) AS TotalViews,
// (SELECT AverageScore FROM PostStatistics) AS AverageScore,
// (SELECT TotalUsers FROM UserStatistics) AS TotalUsers,
// (SELECT AverageReputation FROM UserStatistics) AS AverageUserReputation,
// (SELECT MaxReputation FROM UserStatistics) AS MaxUserReputation,
// (SELECT MinReputation FROM UserStatistics) AS MinUserReputation,
// (SELECT TotalBadges FROM BadgeStatistics) AS TotalBadges,
// (SELECT TotalGoldBadges FROM BadgeStatistics) AS TotalGoldBadges,
// (SELECT TotalSilverBadges FROM BadgeStatistics) AS TotalSilverBadges,
// (SELECT TotalBronzeBadges FROM BadgeStatistics) AS TotalBronzeBadges,
// (SELECT TotalVotes FROM VoteStatistics) AS TotalVotes,
// (SELECT TotalUpVotes FROM VoteStatistics) AS TotalUpVotes,
// (SELECT TotalDownVotes FROM VoteStatistics) AS TotalDownVotes;
fn q11926(db: &'static So) -> String {
    let p = db.post.select((&db.post.post_type_id).and((&db.post.view_count).opt()).and(&db.post.score)).fold_flat([0i64; 6], |a, ((t, w), s)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0), a[5] + s]
    });
    let u = db.user.select(&db.user.reputation).fold_flat([0, 0, i64::MIN, i64::MAX], |a: [i64; 4], r| [a[0] + 1, a[1] + r, a[2].max(r), a[3].min(r)]);
    let b = db.badge.select(&db.badge.class).fold_flat([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    let x = db.vote.select(&db.vote.vote_type_id).fold_flat([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64]);
    let mut f = vec![V::I(p[0]), V::I(p[1]), V::I(p[2]), nullable(p[4], p[3]), avg(p[5], p[0]), V::I(u[0]), avg(u[1], u[0]), V::I(u[2]), V::I(u[3])];
    f.extend(ints(&b));
    f.extend(ints(&x));
    row(f)
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// U.Id, U.DisplayName
// ),
// PostActivity AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.Score,
// P.ViewCount,
// COUNT(C.Id) AS CommentCount,
// COUNT(Ph.Id) AS EditCount
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// PostHistory Ph ON P.Id = Ph.PostId
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.Score, P.ViewCount
// )
// SELECT
// US.UserId,
// US.DisplayName,
// US.TotalPosts,
// US.TotalQuestions,
// US.TotalAnswers,
// US.TotalUpvotes,
// US.TotalDownvotes,
// PA.PostId,
// PA.Title AS PostTitle,
// PA.CreationDate AS PostCreationDate,
// PA.Score AS PostScore,
// PA.ViewCount AS PostViewCount,
// PA.CommentCount,
// PA.EditCount
// FROM
// UserStats US
// JOIN
// PostActivity PA ON US.UserId = PA.PostId
// ORDER BY
// US.TotalUpvotes DESC, US.TotalPosts DESC;
fn q11937(db: &'static So) -> String {
    let uid = uids(db);
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let pa = stats_fold(db, db.post.with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "ch", &[]);
    let mut v = Vec::new();
    (&pa).and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&us))).drive(|p, (s, (u, a))| v.push((p, s, u, a)));
    rows(v.iter().map(|&(p, s, u, a)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(a.n), V::I(a.q), V::I(a.a), V::I(a.up), V::I(a.down)];
        f.extend(post_fields(db, p, &["id", "title", "created", "score", "views"]));
        f.extend([V::I(s.cx), V::I(s.hx)]);
        row(f)
    }))
}

// WITH UserPostCounts AS (
// SELECT
// OwnerUserId,
// COUNT(*) AS PostCount,
// SUM(CASE WHEN PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount
// FROM
// Posts
// GROUP BY
// OwnerUserId
// ),
// UserBadges AS (
// SELECT
// UserId,
// COUNT(*) AS BadgeCount,
// SUM(CASE WHEN Class = 1 THEN 1 ELSE 0 END) AS GoldBadgeCount,
// SUM(CASE WHEN Class = 2 THEN 1 ELSE 0 END) AS SilverBadgeCount,
// SUM(CASE WHEN Class = 3 THEN 1 ELSE 0 END) AS BronzeBadgeCount
// FROM
// Badges
// GROUP BY
// UserId
// )
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COALESCE(up.PostCount, 0) AS TotalPosts,
// COALESCE(up.QuestionCount, 0) AS TotalQuestions,
// COALESCE(up.AnswerCount, 0) AS TotalAnswers,
// COALESCE(ub.BadgeCount, 0) AS TotalBadges,
// COALESCE(ub.GoldBadgeCount, 0) AS TotalGoldBadges,
// COALESCE(ub.SilverBadgeCount, 0) AS TotalSilverBadges,
// COALESCE(ub.BronzeBadgeCount, 0) AS TotalBronzeBadges
// FROM
// Users u
// LEFT JOIN
// UserPostCounts up ON u.Id = up.OwnerUserId
// LEFT JOIN
// UserBadges ub ON u.Id = ub.UserId
// ORDER BY
// u.Reputation DESC
// LIMIT 100;
fn q11940(db: &'static So) -> String {
    let bc = badge_classes(db);
    let pc = user_posts_q(db);
    let mut v = Vec::new();
    (&pc).and((&bc).opt()).drive(|u, (a, b)| v.push((u, a, b.unwrap_or([0; 4]))));
    out(v, |&(u, _, _)| rep_desc(db, u), 100, |&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(ints(&a[..3]));
        f.extend(ints(&b));
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
// SUM(CASE WHEN p.PostTypeId IN (4, 5) THEN 1 ELSE 0 END) AS TotalTagWikis,
// COALESCE(SUM(CASE WHEN v.UserId IS NOT NULL THEN 1 ELSE 0 END), 0) AS TotalVotes,
// COALESCE(SUM(CASE WHEN c.Id IS NOT NULL THEN 1 ELSE 0 END), 0) AS TotalComments
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
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
// TotalTagWikis,
// TotalVotes,
// TotalComments
// FROM
// UserPostStats
// ORDER BY
// TotalPosts DESC;
fn q11946(db: &'static So) -> String {
    let uf = g(db)
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select((&db.vote.user_id).opt()).opt()).and(comments_of(db).opt())).opt())
        .fold([0i64; 6], |a, p| match p {
            Some(((t, x), c)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + matches!(t, 4 | 5) as i64, a[4] + x.flatten().is_some() as i64, a[5] + c.is_some() as i64],
            None => a,
        });
    let mut v = Vec::new();
    (&uf).drive(|u, a| v.push((u, a)));
    rows(v.iter().map(|&(u, a)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a));
        row(f)
    }))
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// COUNT(c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
// COUNT(DISTINCT b.Id) AS BadgeCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Badges b ON p.OwnerUserId = b.UserId
// GROUP BY
// p.Id, p.Title, p.CreationDate
// ),
// UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(p.Id) AS PostCount,
// SUM(p.Score) AS TotalScore
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.CommentCount,
// ps.VoteCount,
// us.UserId,
// us.DisplayName,
// us.Reputation,
// us.PostCount,
// us.TotalScore
// FROM
// PostStats ps
// JOIN
// UserStats us ON ps.PostId = us.UserId
// WHERE
// ps.VoteCount > 10
// ORDER BY
// ps.VoteCount DESC,
// us.Reputation DESC
// LIMIT 100;
fn q11948(db: &'static So) -> String {
    let uid = uids(db);
    let x = per_post_distinct(db, votes_of(db));
    let us = g(db).select(posts_of(db).select(&db.post.score).opt()).fold([0i64; 2], |a, s| match s {
        Some(s) => [a[0] + 1, a[1] + s],
        None => a,
    });
    let mut v = Vec::new();
    stats_fold(db, db.post.with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cvb", &[])
        .and((&x).filt(|n: i64| n > 10))
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&us)))
        .drive(|p, ((s, x), (u, a))| v.push((p, s, x, u, a)));
    out(v, |&(_, _, x, u, _)| (Reverse(x), rep_desc(db, u)), 100, |&(p, s, x, u, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(s.cx), V::I(x), user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep"), V::I(a[0]), nullable(a[1], a[0])]);
        f
    })
}

// WITH UserActivity AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(DISTINCT P.Id) AS PostCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(COALESCE(P.Score, 0)) AS TotalScore,
// SUM(COALESCE(P.ViewCount, 0)) AS TotalViews,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Likes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Dislikes
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// WHERE
// U.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY
// U.Id, U.DisplayName
// )
// SELECT
// UserId,
// DisplayName,
// PostCount,
// QuestionCount,
// AnswerCount,
// TotalScore,
// TotalViews,
// Likes,
// Dislikes
// FROM
// UserActivity
// ORDER BY
// TotalScore DESC
// LIMIT 10;
fn q11959(db: &'static So) -> String {
    let w = UserWhere::CreatedGe(year_ago());
    let us = user_stats_fold(db, Ident::<User>::new(), w, "v", any_post);
    let dp = ud(db, w, posts_of(db));
    let mut v = Vec::new();
    (&us).and((&dp).opt()).drive(|u, (a, d)| v.push((u, a, d.unwrap_or(0))));
    out(v, |&(_, a, _)| Reverse(a.score_sum), 10, |&(u, a, d)| {
        vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(d), V::I(a.q), V::I(a.a), V::I(a.score_sum), V::I(a.views_sum), V::I(a.up), V::I(a.down)]
    })
}

// WITH UserVotes AS (
// SELECT UserId,
// COUNT(*) AS TotalVotes,
// SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM Votes
// GROUP BY UserId
// ),
// PostStats AS (
// SELECT p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// COALESCE(u.DisplayName, 'Deleted User') AS OwnerDisplayName,
// COALESCE(uv.TotalVotes, 0) AS UserTotalVotes,
// COALESCE(uv.UpVotes, 0) AS UserUpVotes,
// COALESCE(uv.DownVotes, 0) AS UserDownVotes
// FROM Posts p
// LEFT JOIN Users u ON p.OwnerUserId = u.Id
// LEFT JOIN UserVotes uv ON u.Id = uv.UserId
// )
// SELECT ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.Score,
// ps.ViewCount,
// ps.OwnerDisplayName,
// ps.UserTotalVotes,
// ps.UserUpVotes,
// ps.UserDownVotes
// FROM PostStats ps
// ORDER BY ps.Score DESC, ps.ViewCount DESC
// LIMIT 100;
fn q11961(db: &'static So) -> String {
    let uv = db.vote.group_by(&db.vote.user).select(&db.vote.vote_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64]);
    let mut v = Vec::new();
    db.post.select(Ident::<Post>::new().and((&db.post.owner_user).select(&uv).opt())).drive(|_, x| v.push(x));
    out(v, |&(p, _)| score_views(db, p), 100, |&(p, x)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.push(named_owner(db, p, "Deleted User"));
        f.extend(ints(&x.unwrap_or([0; 3])));
        f
    })
}

// WITH UserPostCounts AS (
// SELECT
// OwnerUserId,
// COUNT(*) AS PostCount
// FROM
// Posts
// GROUP BY
// OwnerUserId
// ),
// UserVoteCounts AS (
// SELECT
// UserId,
// COUNT(*) AS VoteCount
// FROM
// Votes
// GROUP BY
// UserId
// ),
// PostCommentCounts AS (
// SELECT
// PostId,
// COUNT(*) AS CommentCount
// FROM
// Comments
// GROUP BY
// PostId
// )
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COALESCE(UPC.PostCount, 0) AS TotalPosts,
// COALESCE(UVC.VoteCount, 0) AS TotalVotes,
// COALESCE(PCC.CommentCount, 0) AS TotalComments
// FROM
// Users U
// LEFT JOIN
// UserPostCounts UPC ON U.Id = UPC.OwnerUserId
// LEFT JOIN
// UserVoteCounts UVC ON U.Id = UVC.UserId
// LEFT JOIN
// PostCommentCounts PCC ON U.Id = PCC.PostId
// ORDER BY
// TotalPosts DESC, TotalVotes DESC;
fn q11964(db: &'static So) -> String {
    let pid = pids(db);
    let pc = (&db.post.owner_user).inv().dense_fold_outer(db.user.id.n, 0i64, |a, _| a + 1);
    let vu = votes_per_user(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and(&pc).and(&vu).and((&db.user.origid).select(&pid).select(comments_per_post(db)).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(((u, p), x), c)| row(vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(p), V::I(x), V::I(c.unwrap_or(0))])))
}

// WITH PostStats AS (
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
// p.LastActivityDate
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// ),
// PostHistoryStats AS (
// SELECT
// ph.PostId,
// COUNT(ph.Id) AS EditCount,
// MAX(ph.CreationDate) AS LastEditDate
// FROM
// PostHistory ph
// GROUP BY
// ph.PostId
// ),
// BadgeStats AS (
// SELECT
// b.UserId,
// COUNT(b.Id) AS BadgeCount
// FROM
// Badges b
// GROUP BY
// b.UserId
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.PostCreationDate,
// ps.ViewCount,
// ps.Score,
// ps.AnswerCount,
// ps.CommentCount,
// ps.OwnerReputation,
// ps.OwnerDisplayName,
// ps.LastActivityDate,
// COALESCE(ph.EditCount, 0) AS EditCount,
// ph.LastEditDate,
// COALESCE(bs.BadgeCount, 0) AS OwnerBadgeCount
// FROM
// PostStats ps
// LEFT JOIN
// PostHistoryStats ph ON ps.PostId = ph.PostId
// LEFT JOIN
// BadgeStats bs ON ps.OwnerReputation = bs.UserId
// ORDER BY
// ps.LastActivityDate DESC;
fn q11977(db: &'static So) -> String {
    let uid = uids(db);
    let hc = history_per_post(db);
    let hm = history_max_date(db);
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    owned_since(db, year_ago())
        .select(Ident::<Post>::new().and(&hc).and(&hm).and((&db.post.owner_user).select(&db.user.reputation).select(&uid).select(&bu).opt()))
        .drive(|_, x| v.push(x));
    rows(v.iter().map(|&(((p, h), m), b)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "answers", "comments", "rep", "owner", "activity"]);
        f.extend([V::I(h), if h == 0 { V::Null } else { V::T(m) }, V::I(b.unwrap_or(0))]);
        row(f)
    }))
}

// WITH PostMetrics AS (
// SELECT
// p.Id AS PostId,
// p.PostTypeId,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// p.AnswerCount,
// p.CommentCount,
// COALESCE(u.Reputation, 0) AS OwnerReputation,
// p.Title,
// (SELECT COUNT(*) FROM Votes v WHERE v.PostId = p.Id) AS TotalVotes,
// (SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id) AS TotalComments
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// )
// SELECT
// PostId,
// PostTypeId,
// COUNT(*) AS PostCount,
// AVG(ViewCount) AS AvgViewCount,
// AVG(Score) AS AvgScore,
// AVG(AnswerCount) AS AvgAnswerCount,
// AVG(CommentCount) AS AvgCommentCount,
// AVG(OwnerReputation) AS AvgOwnerReputation,
// SUM(TotalVotes) AS TotalVotes,
// SUM(TotalComments) AS TotalComments
// FROM
// PostMetrics
// GROUP BY
// PostId, PostTypeId
// ORDER BY
// PostCount DESC;
fn q11985(db: &'static So) -> String {
    let mut v = Vec::new();
    since(db, year_ago()).select(Ident::<Post>::new().and(votes_per_post(db)).and(comments_per_post(db))).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((p, x), c)| {
        let fl = |o: Option<i64>| ofloat(o.map(|x| x as f64));
        let mut f = post_fields(db, p, &["id", "type_id"]);
        f.extend([
            V::I(1),
            fl(db.post.view_count.get(p)),
            fl(db.post.score.get(p)),
            fl(db.post.answer_count.get(p)),
            fl(db.post.comment_count.get(p)),
            V::F(db.post.owner_user.get(p).map_or(0, |u| db.user.reputation.get(u).unwrap()) as f64),
            V::I(x),
            V::I(c),
        ]);
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
// COUNT(c.Id) AS TotalComments,
// SUM(CASE WHEN v.CreationDate IS NOT NULL THEN 1 ELSE 0 END) AS TotalVotes
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
// TotalComments,
// TotalVotes
// FROM
// UserPostStats
// ORDER BY
// TotalPosts DESC
// LIMIT 10;
fn q11995(db: &'static So) -> String {
    out(users_with_counts(db, "cv", false), |r| Reverse(r.agg.prows), 10, |r| user_fields(r, "cv", &["uid", "name", "#rows", "#q", "#a", "#cx", "#vx"]))
}

// WITH UserPostStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(P.Id) AS PostCount,
// SUM(COALESCE(P.Score, 0)) AS TotalScore,
// SUM(COALESCE(P.ViewCount, 0)) AS TotalViews,
// SUM(COALESCE(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END, 0)) AS QuestionCount,
// SUM(COALESCE(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END, 0)) AS AnswerCount
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id, U.DisplayName
// ),
// PostHistoryStats AS (
// SELECT
// PH.PostId,
// COUNT(PH.Id) AS EditCount,
// SUM(CASE WHEN PHT.Name = 'Edit Body' THEN 1 ELSE 0 END) AS BodyEdits,
// SUM(CASE WHEN PHT.Name = 'Edit Title' THEN 1 ELSE 0 END) AS TitleEdits
// FROM
// PostHistory PH
// JOIN
// PostHistoryTypes PHT ON PH.PostHistoryTypeId = PHT.Id
// GROUP BY
// PH.PostId
// )
// SELECT
// UPS.UserId,
// UPS.DisplayName,
// UPS.PostCount,
// UPS.TotalScore,
// UPS.TotalViews,
// UPS.QuestionCount,
// UPS.AnswerCount,
// PHS.EditCount,
// PHS.BodyEdits,
// PHS.TitleEdits
// FROM
// UserPostStats UPS
// LEFT JOIN
// PostHistoryStats PHS ON UPS.UserId = PHS.PostId
// ORDER BY
// UPS.TotalScore DESC
// LIMIT 100;
fn q12000(db: &'static So) -> String {
    let pid = pids(db);
    let hs = db
        .post_history
        .group_by(&db.post_history.post)
        .select((&db.post_history.post_history_type).select(&db.post_history_type.name))
        .fold([0i64; 3], |a, n| [a[0] + 1, a[1] + (n == "Edit Body") as i64, a[2] + (n == "Edit Title") as i64]);
    let mut v = Vec::new();
    upqa(db).and((&db.user.origid).select(&pid).select(&hs).opt()).drive(|u, (a, h)| v.push((u, a, h)));
    out(v, |&(_, a, _)| Reverse(a[5]), 100, |&(u, a, h)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(a[0]), V::I(a[5]), V::I(a[4]), V::I(a[1]), V::I(a[2])];
        f.extend((0..3).map(|i| oint(h.map(|h| h[i]))));
        f
    })
}

// WITH Benchmark AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// u.Reputation AS OwnerReputation,
// COUNT(DISTINCT c.Id) AS CommentCount,
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
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY
// p.Id, p.Title, p.CreationDate, u.Reputation
// )
// SELECT
// AVG(OwnerReputation) AS AvgOwnerReputation,
// AVG(CommentCount) AS AvgCommentCount,
// AVG(VoteCount) AS AvgVoteCount,
// AVG(BadgeCount) AS AvgBadgeCount
// FROM
// Benchmark;
fn q12001(db: &'static So) -> String {
    let c = per_post_distinct(db, comments_of(db));
    let x = per_post_distinct(db, votes_of(db));
    let b = per_post_distinct(db, (&db.post.owner_user).select(badges_of(db)));
    let a = since(db, year_ago())
        .select((&db.post.owner_user).select(&db.user.reputation).opt().and((&c).opt()).and((&x).opt()).and((&b).opt()))
        .fold_flat([0i64; 6], |a, (((r, c), x), b)| [a[0] + 1, a[1] + r.is_some() as i64, a[2] + r.unwrap_or(0), a[3] + c.unwrap_or(0), a[4] + x.unwrap_or(0), a[5] + b.unwrap_or(0)]);
    row(vec![avg(a[2], a[1]), avg(a[3], a[0]), avg(a[4], a[0]), avg(a[5], a[0])])
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COALESCE(SUM(V.BountyAmount), 0) AS TotalBounties,
// COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpVotes,
// COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownVotes
// FROM
// Users U
// LEFT JOIN
// Votes V ON U.Id = V.UserId
// GROUP BY
// U.Id, U.DisplayName, U.Reputation
// ),
// PostStats AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(P.Score) AS TotalScore,
// SUM(P.ViewCount) AS TotalViewCount,
// SUM(P.CommentCount) AS TotalComments
// FROM
// Posts P
// GROUP BY
// P.OwnerUserId
// )
// SELECT
// U.Id,
// U.DisplayName,
// U.Reputation,
// COALESCE(PS.TotalPosts, 0) AS PostsCreated,
// COALESCE(PS.TotalQuestions, 0) AS QuestionsAsked,
// COALESCE(PS.TotalAnswers, 0) AS AnswersGiven,
// COALESCE(PS.TotalScore, 0) AS Score,
// COALESCE(PS.TotalViewCount, 0) AS ViewCount,
// COALESCE(PS.TotalComments, 0) AS CommentCount,
// COALESCE(US.TotalBounties, 0) AS TotalBounties,
// COALESCE(US.TotalUpVotes, 0) AS TotalUpVotes,
// COALESCE(US.TotalDownVotes, 0) AS TotalDownVotes
// FROM
// Users U
// LEFT JOIN
// UserStats US ON U.Id = US.UserId
// LEFT JOIN
// PostStats PS ON U.Id = PS.OwnerUserId
// ORDER BY
// U.Reputation DESC;
fn q12006(db: &'static So) -> String {
    let uv = g(db).select(votes_by(db).select((&db.vote.bounty_amount).opt().and(&db.vote.vote_type_id)).opt()).fold([0i64; 3], |a, x| match x {
        Some((b, t)) => [a[0] + b.unwrap_or(0), a[1] + (t == 2) as i64, a[2] + (t == 3) as i64],
        None => a,
    });
    let pf = owned(db)
        .group_by(&db.post.owner_user)
        .select((&db.post.post_type_id).and(&db.post.score).and((&db.post.view_count).opt()).and(&db.post.comment_count))
        .fold([0i64; 6], |a, (((t, s), w), cc)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + w.unwrap_or(0), a[5] + cc]);
    let mut v = Vec::new();
    (&uv).and((&pf).opt()).drive(|u, (a, p)| v.push((u, a, p.unwrap_or([0; 6]))));
    rows(v.iter().map(|&(u, a, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(ints(&p));
        f.extend(ints(&a));
        row(f)
    }))
}

// WITH PostMetrics AS (
// SELECT
// Posts.PostTypeId,
// COUNT(Posts.Id) AS PostCount,
// AVG(Posts.Score) AS AverageScore,
// COUNT(DISTINCT Posts.OwnerUserId) AS UniqueUsers
// FROM
// Posts
// GROUP BY
// Posts.PostTypeId
// ),
// UserMetrics AS (
// SELECT
// AVG(Users.Reputation) AS AverageReputation,
// COUNT(Users.Id) AS TotalUsers
// FROM
// Users
// )
// SELECT
// P.PostTypeId,
// P.PostCount,
// P.AverageScore,
// U.AverageReputation,
// U.TotalUsers
// FROM
// PostMetrics P,
// UserMetrics U
// ORDER BY
// P.PostTypeId;
fn q12012(db: &'static So) -> String {
    let f = by_key(db.post.iq(), &db.post.post_type_id, &db.post.score, (0i64, 0i64), |(n, s), x| (n + 1, s + x));
    let (un, rs) = db.user.select(&db.user.reputation).fold_flat((0i64, 0i64), |(n, s), r| (n + 1, s + r));
    let mut v = Vec::new();
    (&f).drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&(k, (n, s))| row(vec![V::I(k), V::I(n), avg(s, n), avg(rs, un), V::I(un)])))
}

// WITH PostStatistics AS (
// SELECT
// p.Id AS PostId,
// pt.Name AS PostType,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// MAX(p.CreationDate) AS LastActivity
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// GROUP BY
// p.Id, pt.Name
// ),
// UserStatistics AS (
// SELECT
// u.Id AS UserId,
// COUNT(b.Id) AS BadgeCount,
// SUM(CASE WHEN b.Date IS NOT NULL THEN 1 ELSE 0 END) AS TotalBadges
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// u.Id
// )
// SELECT
// ps.PostId,
// ps.PostType,
// ps.CommentCount,
// ps.VoteCount,
// ps.LastActivity,
// u.Id AS UserId,
// us.BadgeCount,
// us.TotalBadges
// FROM
// PostStatistics ps
// JOIN
// Users u ON ps.PostId = u.AccountId
// JOIN
// UserStatistics us ON u.Id = us.UserId
// ORDER BY
// ps.VoteCount DESC, ps.LastActivity DESC;
fn q12014(db: &'static So) -> String {
    let acc: HashIdx<i64, Id<User>> = (&db.user.account_id).inv().collect();
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    stats_fold(db, db.post.with((&db.post.origid).select(&acc)), Ident::<Post>::new(), "cv", &[])
        .and((&db.post.origid).select(&acc).select(Ident::<User>::new().and(&bu)))
        .drive(|p, (s, (u, b))| v.push((p, s, u, b)));
    rows(v.iter().map(|&(p, s, u, b)| {
        let mut f = post_fields(db, p, &["id", "type"]);
        f.extend([V::I(s.cx), V::I(s.vx)]);
        f.extend(post_fields(db, p, &["created"]));
        f.extend([user_col(db, u, "uid"), V::I(b), V::I(b)]);
        row(f)
    }))
}

// WITH Post_Vote_Summary AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.PostTypeId,
// COUNT(v.Id) AS TotalVotes,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
// FROM
// Posts p
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.Title, p.PostTypeId
// ),
// Post_Comment_Summary AS (
// SELECT
// c.PostId,
// COUNT(c.Id) AS TotalComments
// FROM
// Comments c
// GROUP BY
// c.PostId
// )
// SELECT
// pvs.PostId,
// pvs.Title,
// pvs.PostTypeId,
// pvs.TotalVotes,
// pvs.Upvotes,
// pvs.Downvotes,
// COALESCE(pcs.TotalComments, 0) AS TotalComments
// FROM
// Post_Vote_Summary pvs
// LEFT JOIN
// Post_Comment_Summary pcs ON pvs.PostId = pcs.PostId
// ORDER BY
// pvs.TotalVotes DESC,
// pvs.Upvotes DESC,
// pvs.Downvotes ASC;
fn q12021(db: &'static So) -> String {
    let mut v = Vec::new();
    stats_fold(db, db.post.iq(), Ident::<Post>::new(), "v", &[]).and(comments_per_post(db)).drive(|p, (s, c)| v.push((p, s, c)));
    rows(v.iter().map(|&(p, s, c)| {
        let mut f = post_fields(db, p, &["id", "title", "type_id"]);
        f.extend([V::I(s.vx), V::I(s.up), V::I(s.down), V::I(c)]);
        row(f)
    }))
}

// WITH PostStats AS (
// SELECT
// P.Id AS PostId,
// P.PostTypeId,
// P.OwnerUserId,
// COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
// COUNT(V.Id) AS VoteCount,
// AVG(P.Score) AS AverageScore,
// SUM(P.ViewCount) AS TotalViews,
// SUM(P.FavoriteCount) AS TotalFavorites
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// P.Id, P.PostTypeId, P.OwnerUserId
// ),
// UserStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(B.Id) AS TotalBadges,
// AVG(Ps.TotalViews) AS AveragePostViews,
// SUM(Ps.TotalFavorites) AS TotalPostFavorites
// FROM
// Users U
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// LEFT JOIN
// PostStats Ps ON U.Id = Ps.OwnerUserId
// GROUP BY
// U.Id, U.DisplayName
// )
// SELECT
// P.Id AS PostId,
// P.Title,
// U.DisplayName AS OwnerDisplayName,
// Ps.CommentCount,
// Ps.VoteCount,
// Ps.AverageScore,
// Ps.TotalViews,
// Ps.TotalFavorites,
// U.TotalBadges,
// U.AveragePostViews,
// U.TotalPostFavorites
// FROM
// PostStats Ps
// JOIN
// Posts P ON Ps.PostId = P.Id
// JOIN
// UserStats U ON Ps.OwnerUserId = U.UserId
// ORDER BY
// Ps.AverageScore DESC, Ps.TotalViews DESC;
fn q12023(db: &'static So) -> String {
    let Post { view_count, favorite_count, .. } = &db.post;
    let pf = owned(db)
        .group_by(Ident::<Post>::new())
        .select(view_count.opt().and(favorite_count.opt()).and(comments_of(db).opt()).and(votes_of(db).opt()))
        .fold([0i64; 7], |a, (((w, fc), c), x)| {
            [a[0] + c.is_some() as i64, a[1] + x.is_some() as i64, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + fc.is_some() as i64, a[5] + fc.unwrap_or(0), 0]
        });
    let us = g(db).select(badges_of(db).opt().and(posts_of(db).select(&pf).opt())).fold([0i64; 5], |a, (b, p)| {
        let mut a = a;
        a[0] += b.is_some() as i64;
        if let Some(p) = p {
            if p[2] > 0 {
                a[1] += 1;
                a[2] += p[3];
            }
            if p[4] > 0 {
                a[3] += 1;
                a[4] += p[5];
            }
        }
        a
    });
    let mut v = Vec::new();
    (&pf).and((&db.post.owner_user).select(&us)).drive(|p, (a, u)| v.push((p, a, u)));
    rows(v.iter().map(|&(p, a, u)| {
        let mut f = post_fields(db, p, &["id", "title", "owner"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::F(db.post.score.get(p).unwrap() as f64), nullable(a[3], a[2]), nullable(a[5], a[4]), V::I(u[0]), avg(u[2], u[1]), nullable(u[4], u[3])]);
        row(f)
    }))
}

// WITH PostTypeCount AS (
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
// UserCountByYear AS (
// SELECT
// DATE_TRUNC('year', CreationDate) AS RegistrationYear,
// COUNT(Id) AS TotalUsers
// FROM
// Users
// GROUP BY
// DATE_TRUNC('year', CreationDate)
// ),
// VoteTypeCount AS (
// SELECT
// vt.Name AS VoteType,
// COUNT(v.Id) AS TotalVotes
// FROM
// Votes v
// JOIN
// VoteTypes vt ON v.VoteTypeId = vt.Id
// GROUP BY
// vt.Name
// )
// SELECT
// 'PostTypeCount' AS BenchmarkType,
// PostType,
// TotalPosts
// FROM
// PostTypeCount
// UNION ALL
// SELECT
// 'UserCountByYear' AS BenchmarkType,
// CAST(RegistrationYear AS VARCHAR) AS RegistrationYear,
// TotalUsers
// FROM
// UserCountByYear
// UNION ALL
// SELECT
// 'VoteTypeCount' AS BenchmarkType,
// VoteType,
// TotalVotes
// FROM
// VoteTypeCount;
fn q12031(db: &'static So) -> String {
    let pt = by_key(db.post.iq(), name(db), Ident::<Post>::new(), 0i64, |a, _| a + 1);
    let uy = db.user.group_by((&db.user.creation_date).map(trunc_year)).fold(0i64, |a, _| a + 1);
    let vt = db.vote.group_by((&db.vote.vote_type).select(&db.vote_type.name)).fold(0i64, |a, _| a + 1);
    let mut v = Vec::new();
    (&pt).drive(|k, n| v.push(row(vec![V::S("PostTypeCount"), V::S(k), V::I(n)])));
    (&uy).drive(|k, n| v.push(row(vec![V::S("UserCountByYear"), V::Owned(fmt_ts(k).trim_end_matches(".000000").to_string()), V::I(n)])));
    (&vt).drive(|k, n| v.push(row(vec![V::S("VoteTypeCount"), V::S(k), V::I(n)])));
    rows(v)
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(p.Score) AS TotalScore,
// AVG(p.ViewCount) AS AvgViewCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id
// ),
// BadgeStats AS (
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
// ups.TotalScore,
// ups.AvgViewCount,
// bs.TotalBadges,
// bs.TotalGoldBadges,
// bs.TotalSilverBadges,
// bs.TotalBronzeBadges
// FROM
// Users u
// LEFT JOIN
// UserPostStats ups ON u.Id = ups.UserId
// LEFT JOIN
// BadgeStats bs ON u.Id = bs.UserId
// ORDER BY
// ups.TotalScore DESC,
// ups.TotalPosts DESC;
fn q12033(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let uf = g(db).select(posts_of(db).select(post_type_id.and(score).and(view_count.opt())).opt()).fold([0i64; 6], |a, p| match p {
        Some(((t, s), w)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0)],
        None => a,
    });
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&uf).and((&bc).opt()).drive(|u, (a, b)| v.push((u, a, b)));
    rows(v.iter().map(|&(u, a, b)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(ints(&a[..3]));
        f.extend([nullable(a[3], a[0]), avg(a[5], a[4])]);
        f.extend((0..4).map(|i| oint(b.map(|b| b[i]))));
        row(f)
    }))
}

// WITH PostSummary AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// MAX(ph.CreationDate) AS LastEditDate,
// SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounty
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// GROUP BY
// p.Id, p.Title
// ),
// UserActivity AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS PostsCreated,
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
// ps.CommentCount,
// ps.VoteCount,
// ps.LastEditDate,
// ps.TotalBounty,
// ua.UserId,
// ua.DisplayName AS PostOwner,
// ua.PostsCreated,
// ua.GoldBadges,
// ua.SilverBadges,
// ua.BronzeBadges
// FROM
// PostSummary ps
// JOIN
// UserActivity ua ON ps.PostId = ua.UserId
// ORDER BY
// ps.TotalBounty DESC, ps.CommentCount DESC;
fn q12056(db: &'static So) -> String {
    let uid = uids(db);
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "b", any_post);
    let mut v = Vec::new();
    stats_fold(db, db.post.with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cvh", &[])
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&us)))
        .drive(|p, (s, (u, a))| v.push((p, s, u, a)));
    rows(v.iter().map(|&(p, s, u, a)| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([V::I(s.cx), V::I(s.vx), stat_field(&s, "hmax").unwrap(), V::I(s.bounty_sum), user_col(db, u, "uid"), user_col(db, u, "name"), V::I(a.n)]);
        f.extend(["#gold", "#silver", "#bronze"].iter().map(|c| ustat_field(&a, c)));
        row(f)
    }))
}

// WITH PostStats AS (
// SELECT
// p.PostTypeId,
// COUNT(*) AS TotalPosts,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes,
// COALESCE(COUNT(c.Id), 0) AS TotalComments
// FROM
// Posts p
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// WHERE
// p.CreationDate >= '2023-01-01' AND p.CreationDate < '2024-01-01'
// GROUP BY
// p.PostTypeId
// )
// SELECT
// pt.Name AS PostType,
// ps.TotalPosts,
// ps.TotalUpVotes,
// ps.TotalDownVotes,
// ps.TotalComments
// FROM
// PostTypes pt
// JOIN
// PostStats ps ON pt.Id = ps.PostTypeId
// ORDER BY
// ps.TotalPosts DESC;
fn q12059(db: &'static So) -> String {
    let base = db.post.with((&db.post.creation_date).ge(date(2023, 1, 1))).with((&db.post.creation_date).lt(date(2024, 1, 1)));
    let f = stats_fold(db, base, &db.post.post_type, "cv", &[]);
    let mut v = Vec::new();
    (&f).drive(|t, s| v.push((t, s)));
    rows(v.iter().map(|&(t, s)| row(vec![V::S(db.post_type.name.get(t).unwrap()), V::I(s.rows), V::I(s.up), V::I(s.down), V::I(s.cx)])))
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// p.AnswerCount,
// COUNT(c.Id) AS CommentCount,
// COALESCE(u.DisplayName, 'Community User') AS OwnerDisplayName,
// COUNT(DISTINCT v.Id) AS VoteCount,
// AVG(CASE WHEN v.VoteTypeId = 2 THEN 1.0 ELSE 0 END) AS AvgUpVotes,
// AVG(CASE WHEN v.VoteTypeId = 3 THEN 1.0 ELSE 0 END) AS AvgDownVotes
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, u.DisplayName
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
// ps.VoteCount,
// ps.AvgUpVotes,
// ps.AvgDownVotes
// FROM
// PostStats ps
// ORDER BY
// ps.Score DESC, ps.ViewCount DESC;
fn q12064(db: &'static So) -> String {
    let x = per_post_distinct(db, votes_of(db));
    rows(stats_with(db, db.post.iq(), "cv", &[], &[&x]).iter().map(|&(p, s, d)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "answers"]);
        f.extend([V::I(s.cx), named_owner(db, p, "Community User"), V::I(d[0])]);
        f.extend(["up_frac", "down_frac"].iter().map(|c| stat_field(&s, c).unwrap()));
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
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount
// ),
// UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(b.Id) AS BadgeCount,
// SUM(u.UpVotes) AS TotalUpVotes,
// SUM(u.DownVotes) AS TotalDownVotes
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
// ps.UpVotes,
// ps.DownVotes,
// us.UserId,
// us.DisplayName AS OwnerDisplayName,
// us.BadgeCount,
// us.TotalUpVotes,
// us.TotalDownVotes
// FROM
// PostStats ps
// JOIN
// UserStats us ON ps.PostId = us.UserId
// ORDER BY
// ps.ViewCount DESC, ps.Score DESC;
fn q12070(db: &'static So) -> String {
    let uid = uids(db);
    let User { up_votes, down_votes, .. } = &db.user;
    let us = g(db).select(up_votes.and(down_votes).and(badges_of(db).opt())).fold([0i64; 3], |a, ((u, d), b)| [a[0] + b.is_some() as i64, a[1] + u, a[2] + d]);
    let mut v = Vec::new();
    stats_fold(db, db.post.with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cv", &[])
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&us)))
        .drive(|p, (s, (u, a))| v.push((p, s, u, a)));
    rows(v.iter().map(|&(p, s, u, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(s.cx), V::I(s.vx), V::I(s.up), V::I(s.down), user_col(db, u, "uid"), user_col(db, u, "name")]);
        f.extend(ints(&a));
        row(f)
    }))
}

// WITH UserActivity AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS PostCount,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 6 THEN 1 ELSE 0 END), 0) AS CloseVotes,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 9 THEN 1 ELSE 0 END), 0) AS BountyCloseVotes
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
// ua.UserId,
// ua.DisplayName,
// ua.PostCount,
// ua.UpVotes,
// ua.DownVotes,
// ua.CloseVotes,
// ua.BountyCloseVotes
// FROM
// UserActivity ua
// WHERE
// ua.PostCount > 0
// ORDER BY
// ua.PostCount DESC,
// ua.UpVotes DESC
// LIMIT 10;
fn q12073(db: &'static So) -> String {
    let uf = g(db).select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt()).fold([0i64; 5], |a, p| match p {
        Some(t) => [a[0] + 1, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64, a[3] + (t == Some(6)) as i64, a[4] + (t == Some(9)) as i64],
        None => a,
    });
    let mut v = Vec::new();
    (&uf).filt(|a: [i64; 5]| a[0] > 0).drive(|u, a| v.push((u, a)));
    out(v, |&(_, a)| (Reverse(a[0]), Reverse(a[1])), 10, |&(u, a)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a));
        f
    })
}

// WITH UserPostCounts AS (
// SELECT
// OwnerUserId,
// COUNT(*) AS PostCount
// FROM
// Posts
// GROUP BY
// OwnerUserId
// ),
// UserVoteCounts AS (
// SELECT
// UserId,
// COUNT(*) AS VoteCount
// FROM
// Votes
// GROUP BY
// UserId
// ),
// UserBadgeCounts AS (
// SELECT
// UserId,
// COUNT(*) AS BadgeCount
// FROM
// Badges
// GROUP BY
// UserId
// )
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COALESCE(UPC.PostCount, 0) AS TotalPosts,
// COALESCE(UVC.VoteCount, 0) AS TotalVotes,
// COALESCE(UBC.BadgeCount, 0) AS TotalBadges
// FROM
// Users U
// LEFT JOIN
// UserPostCounts UPC ON U.Id = UPC.OwnerUserId
// LEFT JOIN
// UserVoteCounts UVC ON U.Id = UVC.UserId
// LEFT JOIN
// UserBadgeCounts UBC ON U.Id = UBC.UserId
// ORDER BY
// U.Reputation DESC
// LIMIT
// 100;
fn q12076(db: &'static So) -> String {
    let pc = (&db.post.owner_user).inv().dense_fold_outer(db.user.id.n, 0i64, |a, _| a + 1);
    let vu = votes_per_user(db);
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and(&pc).and(&vu).and(&bu)).drive(|_, x| v.push(x));
    out(v, |&(((u, _), _), _)| rep_desc(db, u), 100, |&(((u, p), x), b)| vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep"), V::I(p), V::I(x), V::I(b)])
}

// WITH PostStats AS (
// SELECT
// Posts.Id AS PostId,
// Posts.Title,
// Posts.CreationDate,
// COUNT(Comments.Id) AS CommentCount,
// COUNT(Votes.Id) AS VoteCount,
// SUM(CASE WHEN Votes.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN Votes.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Posts
// LEFT JOIN
// Comments ON Posts.Id = Comments.PostId
// LEFT JOIN
// Votes ON Posts.Id = Votes.PostId
// GROUP BY
// Posts.Id, Posts.Title, Posts.CreationDate
// ),
// UserStats AS (
// SELECT
// Users.Id AS UserId,
// Users.DisplayName,
// COUNT(Badges.Id) AS BadgeCount,
// SUM(Posts.ViewCount) AS TotalViews
// FROM
// Users
// LEFT JOIN
// Badges ON Users.Id = Badges.UserId
// LEFT JOIN
// Posts ON Users.Id = Posts.OwnerUserId
// GROUP BY
// Users.Id, Users.DisplayName
// )
// SELECT
// PostStats.PostId,
// PostStats.Title,
// PostStats.CreationDate,
// PostStats.CommentCount,
// PostStats.VoteCount,
// PostStats.UpVotes,
// PostStats.DownVotes,
// UserStats.UserId,
// UserStats.DisplayName,
// UserStats.BadgeCount,
// UserStats.TotalViews
// FROM
// PostStats
// JOIN
// Posts ON PostStats.PostId = Posts.Id
// JOIN
// Users ON Posts.OwnerUserId = Users.Id
// JOIN
// UserStats ON Users.Id = UserStats.UserId
// ORDER BY
// PostStats.CreationDate DESC
// LIMIT 100;
fn q12081(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "b", any_post);
    let mut v = Vec::new();
    stats_fold(db, owned(db), Ident::<Post>::new(), "cv", &[]).and((&db.post.owner_user).select(Ident::<User>::new().and(&us))).drive(|p, (s, (u, a))| v.push((p, s, u, a)));
    out(v, |&(p, _, _, _)| newest(db, p), 100, |&(p, s, u, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(s.cx), V::I(s.vx), V::I(s.up), V::I(s.down), user_col(db, u, "uid"), user_col(db, u, "name"), V::I(a.bx), ustat_field(&a, "views_sum")]);
        f
    })
}

// SELECT
// p.PostTypeId,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.ANSWERCOUNT > 0 THEN 1 ELSE 0 END) AS TotalQuestionsWithAnswers,
// AVG(p.Score) AS AvgScorePerPost,
// SUM(c.CommentCount) AS TotalComments,
// SUM(v.VoteCount) AS TotalVotes,
// u.Reputation AS UserReputation,
// u.DisplayName AS UserDisplayName
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
// WHERE
// p.CreationDate >= '2023-01-01' AND
// p.CreationDate < '2024-01-01'
// GROUP BY
// p.PostTypeId, u.Reputation, u.DisplayName
// ORDER BY
// TotalPosts DESC;
fn q12082(db: &'static So) -> String {
    let cf = db.comment.group_by(&db.comment.post).fold(0i64, |a, _| a + 1);
    let vf = db.vote.group_by(&db.vote.post).fold(0i64, |a, _| a + 1);
    let base = owned(db).with((&db.post.creation_date).ge(date(2023, 1, 1))).with((&db.post.creation_date).lt(date(2024, 1, 1)));
    let key = (&db.post.post_type_id).and((&db.post.owner_user).select((&db.user.reputation).and(&db.user.display_name)));
    let f = base.group_by(key).select((&db.post.answer_count).opt().and(&db.post.score).and((&cf).opt()).and((&vf).opt())).fold([0i64; 7], |a, (((an, s), c), x)| {
        [a[0] + 1, a[1] + an.is_some_and(|n| n > 0) as i64, a[2] + s, a[3] + c.is_some() as i64, a[4] + c.unwrap_or(0), a[5] + x.is_some() as i64, a[6] + x.unwrap_or(0)]
    });
    let mut v = Vec::new();
    (&f).drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&((t, (r, n)), a)| row(vec![V::I(t), V::I(a[0]), V::I(a[1]), avg(a[2], a[0]), nullable(a[4], a[3]), nullable(a[6], a[5]), V::I(r), V::S(n)])))
}

// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(COALESCE(v.VoteCount, 0)) AS TotalVotes,
// SUM(p.ViewCount) AS TotalViews,
// MAX(p.CreationDate) AS LastPostDate
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// (SELECT
// PostId,
// COUNT(*) AS VoteCount
// FROM
// Votes
// GROUP BY
// PostId) v ON p.Id = v.PostId
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
// ORDER BY
// TotalPosts DESC
// LIMIT
// 100;
fn q12084(db: &'static So) -> String {
    let Post { post_type_id, view_count, creation_date, .. } = &db.post;
    let uf = g(db).select(posts_of(db).select(post_type_id.and(view_count.opt()).and(creation_date).and(votes_per_post(db))).opt()).fold([0, 0, 0, 0, 0, 0, i64::MIN], |a: [i64; 7], p| match p {
        Some((((t, w), c), x)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + x, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0), a[6].max(c)],
        None => a,
    });
    let mut v = Vec::new();
    (&uf).drive(|u, a| v.push((u, a)));
    out(v, |&(_, a)| Reverse(a[0]), 100, |&(u, a)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(ints(&a[..4]));
        f.extend([nullable(a[5], a[4]), if a[0] == 0 { V::Null } else { V::T(a[6]) }]);
        f
    })
}

// WITH PostStats AS (
// SELECT
// P.Id AS PostId,
// P.Score,
// P.OwnerUserId,
// COUNT(C.Id) AS CommentCount
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// GROUP BY
// P.Id, P.Score, P.OwnerUserId
// ),
// UserStats AS (
// SELECT
// U.Id AS UserId,
// U.Reputation,
// COUNT(P.Id) AS PostCount,
// AVG(PS.Score) AS AvgScore,
// AVG(PS.CommentCount) AS AvgCommentsPerPost
// FROM
// Users U
// LEFT JOIN
// PostStats PS ON U.Id = PS.OwnerUserId
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id, U.Reputation
// )
// SELECT
// COUNT(P.Id) AS TotalPosts,
// AVG(P.Score) AS AverageScore,
// U.UserId,
// U.Reputation,
// U.PostCount,
// U.AvgScore,
// U.AvgCommentsPerPost
// FROM
// Posts P
// JOIN
// UserStats U ON U.UserId = P.OwnerUserId
// GROUP BY
// U.UserId, U.Reputation, U.PostCount, U.AvgScore, U.AvgCommentsPerPost
// ORDER BY
// U.Reputation DESC, U.PostCount DESC;
fn q12089(db: &'static So) -> String {
    let uf = g(db).select(posts_of(db).select((&db.post.score).and(comments_per_post(db))).opt().and(posts_of(db).opt())).fold([0i64; 5], |a, (ps, p)| {
        let mut a = a;
        if let Some((s, c)) = ps {
            a[1] += 1;
            a[2] += s;
            a[3] += c;
        }
        a[0] += p.is_some() as i64;
        a
    });
    let pf = owned(db).group_by(&db.post.owner_user).select(&db.post.score).fold((0i64, 0i64), |(n, s), x| (n + 1, s + x));
    let mut v = Vec::new();
    (&pf).and(&uf).drive(|u, (p, a)| v.push((u, p, a)));
    rows(v.iter().map(|&(u, (n, s), a)| row(vec![V::I(n), avg(s, n), user_col(db, u, "uid"), user_col(db, u, "rep"), V::I(a[0]), avg(a[2], a[1]), avg(a[3], a[1])])))
}

// WITH UserActivity AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// COUNT(DISTINCT p.Id) AS PostCount,
// COUNT(DISTINCT c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
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
// ua.UserId,
// ua.Reputation,
// ua.PostCount,
// ua.CommentCount,
// ua.UpVotes,
// ua.DownVotes,
// (ua.UpVotes - ua.DownVotes) AS NetVotes
// FROM
// UserActivity ua
// ORDER BY
// ua.Reputation DESC, NetVotes DESC
// LIMIT 100;
fn q12110(db: &'static So) -> String {
    out(users_with_counts(db, "cv", false), |r| (Reverse(r.rep), Reverse(r.agg.up - r.agg.down)), 100, |r| {
        let mut f = user_fields(r, "cv", &["uid", "rep", "#n", "#c", "#up", "#down"]);
        f.push(V::I(r.agg.up - r.agg.down));
        f
    })
}

// WITH PostStats AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.ViewCount,
// P.Score,
// COALESCE(COUNT(CASE WHEN C.PostId IS NOT NULL THEN 1 END), 0) AS CommentCount,
// COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount,
// COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount,
// COALESCE(SUM(CASE WHEN B.UserId IS NOT NULL THEN 1 END), 0) AS BadgeCount
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// LEFT JOIN
// Badges B ON P.OwnerUserId = B.UserId
// WHERE
// P.CreationDate >= '2023-01-01'
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.ViewCount, P.Score
// )
// SELECT
// PS.PostId,
// PS.Title,
// PS.CreationDate,
// PS.ViewCount,
// PS.Score,
// PS.CommentCount,
// PS.UpVoteCount,
// PS.DownVoteCount,
// PS.BadgeCount
// FROM
// PostStats PS
// ORDER BY
// PS.ViewCount DESC, PS.Score DESC;
fn q12115(db: &'static So) -> String {
    rows(stats_with(db, since(db, date(2023, 1, 1)), "cvb", &[], &[]).iter().map(|&(p, s, _)| {
        row(stat_fields(db, p, &s, &["id", "title", "created", "views", "score", "#cx", "#up", "#down", "#bx"]))
    }))
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
// U.Reputation AS OwnerReputation,
// PH.CreationDate AS LastEditDate,
// COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVotes,
// COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVotes,
// (SELECT COUNT(*) FROM Comments C WHERE C.PostId = P.Id) AS TotalComments
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// PostHistory PH ON PH.PostId = P.Id AND PH.PostHistoryTypeId IN (4, 5, 6)
// LEFT JOIN
// Votes V ON V.PostId = P.Id
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.ViewCount, P.Score, P.AnswerCount, P.CommentCount, U.DisplayName, U.Reputation, PH.CreationDate
// ORDER BY
// P.CreationDate DESC
// LIMIT 100;
fn q12117(db: &'static So) -> String {
    let edits = history_of(db).select(Ident::<PostHistory>::new().with((&db.post_history.post_history_type_id).in_v(vec![4, 5, 6])));
    let key = Ident::<Post>::new().and(edits.select(&db.post_history.creation_date).opt());
    let f = owned(db).group_by(key).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cc = comments_per_post(db);
    let mut v = Vec::new();
    (&f).drive(|k, a| v.push((k, a)));
    out(v, |&((p, _), _)| newest(db, p), 100, |&((p, d), a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "answers", "comments", "owner", "rep"]);
        f.extend([ots(d), V::I(a[0]), V::I(a[1]), V::I(cc.get(p).unwrap())]);
        f
    })
}

// WITH PostCounts AS (
// SELECT
// p.OwnerUserId AS UserId,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(p.Score) AS TotalScore
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// ),
// UserReputation AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// u.DisplayName
// FROM
// Users u
// )
// SELECT
// ur.DisplayName,
// ur.Reputation,
// pc.TotalPosts,
// pc.Questions,
// pc.Answers,
// pc.TotalScore
// FROM
// UserReputation ur
// LEFT JOIN
// PostCounts pc ON ur.UserId = pc.UserId
// ORDER BY
// ur.Reputation DESC;
fn q12123(db: &'static So) -> String {
    let pf = owned(db).group_by(&db.post.owner_user).select((&db.post.post_type_id).and(&db.post.score)).fold([0i64; 4], |a, (t, s)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s]);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&pf).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(u, p)| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend((0..4).map(|i| oint(p.map(|p| p[i]))));
        row(f)
    }))
}

// WITH UserActivity AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN c.Id IS NOT NULL THEN 1 ELSE 0 END) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
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
// ua.UserId,
// ua.DisplayName,
// ua.PostCount,
// ua.QuestionCount,
// ua.AnswerCount,
// ua.CommentCount,
// ua.UpVoteCount,
// ua.DownVoteCount,
// u.Reputation,
// u.CreationDate AS UserCreatedDate,
// u.LastAccessDate
// FROM
// UserActivity ua
// JOIN
// Users u ON ua.UserId = u.Id
// ORDER BY
// ua.PostCount DESC,
// ua.UpVoteCount DESC;
fn q12128(db: &'static So) -> String {
    rows(users_with_counts(db, "cv", false).iter().map(|r| row(user_fields(r, "cv", &["uid", "name", "#n", "#q", "#a", "#cx", "#up", "#down", "rep", "ucreated", "last_access"]))))
}

// WITH UserPostStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(P.Id) AS PostCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END) AS PositiveScoreCount,
// AVG(P.Score) AS AverageScore,
// MAX(P.CreationDate) AS LastPostDate
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
// UPS.PostCount,
// UPS.QuestionCount,
// UPS.AnswerCount,
// UPS.PositiveScoreCount,
// UPS.AverageScore,
// UPS.LastPostDate,
// B.UserBadgeCount
// FROM
// UserPostStats UPS
// LEFT JOIN (
// SELECT
// UserId,
// COUNT(Id) AS UserBadgeCount
// FROM
// Badges
// GROUP BY
// UserId
// ) B ON UPS.UserId = B.UserId
// ORDER BY
// UPS.PostCount DESC
// LIMIT 100;
fn q12130(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, .. } = &db.post;
    let uf = g(db).select(posts_of(db).select(post_type_id.and(score).and(creation_date)).opt()).fold([0, 0, 0, 0, 0, i64::MIN], |a: [i64; 6], p| match p {
        Some(((t, s), c)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (s > 0) as i64, a[4] + s, a[5].max(c)],
        None => a,
    });
    let bf = db.badge.group_by(&db.badge.user).fold(0i64, |a, _| a + 1);
    let mut v = Vec::new();
    (&uf).and((&bf).opt()).drive(|u, (a, b)| v.push((u, a, b)));
    out(v, |&(_, a, _)| Reverse(a[0]), 100, |&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[..4]));
        f.extend([avg(a[4], a[0]), if a[0] == 0 { V::Null } else { V::T(a[5]) }, oint(b)]);
        f
    })
}

// WITH PostStats AS (
// SELECT
// p.PostTypeId,
// COUNT(*) AS TotalPosts,
// SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS TotalPositiveScores,
// SUM(CASE WHEN p.Score < 0 THEN 1 ELSE 0 END) AS TotalNegativeScores,
// AVG(p.ViewCount) AS AvgViewCount,
// AVG(COALESCE(a.AnswerCount, 0)) AS AvgAnswerCount,
// AVG(COALESCE(c.CommentCount, 0)) AS AvgCommentCount
// FROM
// Posts p
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
// LEFT JOIN (
// SELECT
// PostId,
// COUNT(*) AS CommentCount
// FROM
// Comments
// GROUP BY
// PostId
// ) c ON p.Id = c.PostId
// GROUP BY
// p.PostTypeId
// ),
// UserStats AS (
// SELECT
// u.Id AS UserId,
// COUNT(b.Id) AS TotalBadges,
// SUM(u.Views) AS TotalViews,
// AVG(u.Reputation) AS AvgReputation
// FROM
// Users u
// LEFT JOIN Badges b ON u.Id = b.UserId
// GROUP BY
// u.Id
// )
// SELECT
// pt.Name AS PostType,
// ps.TotalPosts,
// ps.TotalPositiveScores,
// ps.TotalNegativeScores,
// ps.AvgViewCount,
// ps.AvgAnswerCount,
// ps.AvgCommentCount,
// us.TotalBadges,
// us.TotalViews,
// us.AvgReputation
// FROM
// PostStats ps
// JOIN PostTypes pt ON ps.PostTypeId = pt.Id
// JOIN UserStats us ON us.UserId = (SELECT MIN(Id) FROM Users)
// ORDER BY
// ps.TotalPosts DESC;
fn q12132(db: &'static So) -> String {
    let Post { score, view_count, .. } = &db.post;
    let f = by_key(db.post.iq(), &db.post.post_type, score.and(view_count.opt()).and(typed_answers_per_post(db)).and(comments_per_post(db)), [0i64; 7], |a, (((s, w), an), c)| {
        [a[0] + 1, a[1] + (s > 0) as i64, a[2] + (s < 0) as i64, a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0), a[5] + an, a[6] + c]
    });
    let umin = db.user.select(&db.user.origid).fold_flat(i64::MAX, |m, x| m.min(x));
    let us = g(db).select((&db.user.views).and(&db.user.reputation).and(badges_of(db).opt())).fold([0i64; 4], |a, ((w, r), b)| [a[0] + b.is_some() as i64, a[1] + w, a[2] + r, a[3] + 1]);
    let one_u = rel(drain(db.user.with((&db.user.origid).eq(umin)).select(&us)));
    let mut v = Vec::new();
    (&f).cross(&one_u).drive(|(t, _), (a, (_, ua))| v.push((t, a, ua)));
    rows(v.iter().map(|&(t, a, ua)| {
        row(vec![
            V::S(db.post_type.name.get(t).unwrap()),
            V::I(a[0]),
            V::I(a[1]),
            V::I(a[2]),
            avg(a[4], a[3]),
            avg(a[5], a[0]),
            avg(a[6], a[0]),
            V::I(ua[0]),
            V::I(ua[1]),
            avg(ua[2], ua[3]),
        ])
    }))
}

// WITH PostActivity AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// COUNT(c.Id) AS CommentCount,
// COUNT(a.Id) AS AnswerCount,
// p.OwnerUserId
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Posts a ON p.Id = a.ParentId
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.OwnerUserId
// ),
// UserContribution AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS QuestionCount,
// SUM(p.ViewCount) AS TotalViews,
// SUM(p.Score) AS TotalScore,
// SUM(b.Class) AS TotalBadges
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId AND p.PostTypeId = 1
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// u.Id, u.DisplayName
// )
// SELECT
// ua.UserId,
// ua.DisplayName,
// ua.QuestionCount,
// ua.TotalViews,
// ua.TotalScore,
// ua.TotalBadges,
// pa.PostId,
// pa.Title,
// pa.ViewCount,
// pa.Score,
// pa.CommentCount,
// pa.AnswerCount
// FROM
// UserContribution ua
// LEFT JOIN
// PostActivity pa ON ua.UserId = pa.OwnerUserId
// ORDER BY
// ua.TotalScore DESC, ua.TotalViews DESC;
fn q12136(db: &'static So) -> String {
    let qs = posts_of(db).select(Ident::<Post>::new().with((&db.post.post_type_id).eq(1)));
    let uf = g(db).select((&qs).select((&db.post.view_count).opt().and(&db.post.score)).opt().and(badges_of(db).select(&db.badge.class).opt())).fold([0i64; 6], |a, (p, b)| {
        let mut a = a;
        if let Some((w, s)) = p {
            a[0] += w.is_some() as i64;
            a[1] += w.unwrap_or(0);
            a[2] += 1;
            a[3] += s;
        }
        if let Some(c) = b {
            a[4] += 1;
            a[5] += c;
        }
        a
    });
    let dq = ud(db, UserWhere::All, &qs);
    let pa = questions_only(db).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(children_of(db).opt())).fold([0i64; 2], |a, (c, k)| [a[0] + c.is_some() as i64, a[1] + k.is_some() as i64]);
    let mut v = Vec::new();
    (&uf).and((&dq).opt()).and(posts_of(db).select(Ident::<Post>::new().and(&pa)).opt()).drive(|u, ((a, d), p)| v.push((u, a, d.unwrap_or(0), p)));
    rows(v.iter().map(|&(u, a, d, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(d), nullable(a[1], a[0]), nullable(a[3], a[2]), nullable(a[5], a[4])];
        match p {
            Some((p, x)) => {
                f.extend(post_fields(db, p, &["id", "title", "views", "score"]));
                f.extend([V::I(x[0]), V::I(x[1])]);
            }
            None => f.extend(nulls(6)),
        }
        row(f)
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("11744", q11744),
    ("11760", q11760),
    ("11771", q11771),
    ("11787", q11787),
    ("11802", q11802),
    ("11815", q11815),
    ("11819", q11819),
    ("11826", q11826),
    ("11837", q11837),
    ("11845", q11845),
    ("11851", q11851),
    ("11875", q11875),
    ("11883", q11883),
    ("11887", q11887),
    ("11894", q11894),
    ("11905", q11905),
    ("11909", q11909),
    ("11913", q11913),
    ("11922", q11922),
    ("11926", q11926),
    ("11937", q11937),
    ("11940", q11940),
    ("11946", q11946),
    ("11948", q11948),
    ("11959", q11959),
    ("11961", q11961),
    ("11964", q11964),
    ("11977", q11977),
    ("11985", q11985),
    ("11995", q11995),
    ("12000", q12000),
    ("12001", q12001),
    ("12006", q12006),
    ("12012", q12012),
    ("12014", q12014),
    ("12021", q12021),
    ("12023", q12023),
    ("12031", q12031),
    ("12033", q12033),
    ("12056", q12056),
    ("12059", q12059),
    ("12064", q12064),
    ("12070", q12070),
    ("12073", q12073),
    ("12076", q12076),
    ("12081", q12081),
    ("12082", q12082),
    ("12084", q12084),
    ("12089", q12089),
    ("12110", q12110),
    ("12115", q12115),
    ("12117", q12117),
    ("12123", q12123),
    ("12128", q12128),
    ("12130", q12130),
    ("12132", q12132),
    ("12136", q12136),
];
