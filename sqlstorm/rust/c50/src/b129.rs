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

fn split_n(s: Str, sep: &str) -> i64 {
    let n = s.chars().count();
    let inner: String = s.chars().skip(1).take(n.saturating_sub(2)).collect();
    inner.split(sep).count() as i64
}

fn ubc(db: &'static So) -> Fold<Id<User>, [i64; 4]> {
    g(db).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    })
}

fn leak(s: String) -> Str {
    Box::leak(s.into_boxed_str())
}

// --- batch 129 --------------------------------------------------------------

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// U.Views,
// U.UpVotes,
// U.DownVotes,
// (SELECT COUNT(*) FROM Posts P WHERE P.OwnerUserId = U.Id) AS PostCount,
// (SELECT COUNT(*) FROM Comments C WHERE C.UserId = U.Id) AS CommentCount,
// (SELECT COUNT(*) FROM Badges B WHERE B.UserId = U.Id) AS BadgeCount
// FROM
// Users U
// WHERE
// U.Reputation > 1000
// ), PostStats AS (
// SELECT
// P.OwnerUserId,
// P.PostTypeId,
// COUNT(*) AS TotalPosts,
// AVG(P.Score) AS AvgScore,
// SUM(P.ViewCount) AS TotalViews,
// SUM(P.AnswerCount) AS TotalAnswers
// FROM
// Posts P
// GROUP BY
// P.OwnerUserId, P.PostTypeId
// ), CombinedStats AS (
// SELECT
// US.UserId,
// US.DisplayName,
// US.Reputation,
// US.Views,
// US.UpVotes,
// US.DownVotes,
// US.PostCount,
// US.CommentCount,
// US.BadgeCount,
// PS.PostTypeId,
// PS.TotalPosts,
// PS.AvgScore,
// PS.TotalViews,
// PS.TotalAnswers
// FROM
// UserStats US
// LEFT JOIN
// PostStats PS ON US.UserId = PS.OwnerUserId
// )
// SELECT
// UserId,
// DisplayName,
// Reputation,
// Views,
// UpVotes,
// DownVotes,
// PostCount,
// CommentCount,
// BadgeCount,
// PostTypeId,
// TotalPosts,
// AvgScore,
// TotalViews,
// TotalAnswers
// FROM
// CombinedStats
// WHERE
// TotalPosts > 5
// ORDER BY
// Reputation DESC, TotalViews DESC
// LIMIT 100;
fn q5421(db: &'static So) -> String {
    let Post { owner_user, post_type_id, score, view_count, answer_count, .. } = &db.post;
    let pt = owned(db).group_by(owner_user.and(post_type_id)).select(score.and(view_count.opt()).and(answer_count.opt())).fold([0i64; 6], |a, ((s, w), an)| {
        [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + an.is_some() as i64, a[5] + an.unwrap_or(0)]
    });
    let keys: MatSet<(Id<User>, i64)> = owned(db).select(owner_user.and(post_type_id)).collect();
    let by_u: HashIdx<Id<User>, (Id<User>, i64)> = (&keys).map(|(u, _)| u).inv().collect();
    let pc = owned(db).group_by(owner_user).fold(0i64, |a, _| a + 1);
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000))
        .select(Ident::<User>::new().and((&pc).opt()).and(comments_per_user(db)).and(&bu).and((&by_u).select(Same::<(Id<User>, i64)>::new().and((&pt).filt(|a: [i64; 6]| a[0] > 5)))))
        .drive(|_, x| v.push(x));
    out(v, |&((((u, _), _), _), (_, a))| (rep_desc(db, u), (a[2] == 0, Reverse(a[3]))), 100, |&((((u, p), c), b), ((_, t), a))| {
        let mut f: Vec<V> = ["uid", "name", "rep", "uviews", "uup", "udown"].iter().map(|k| user_col(db, u, k)).collect();
        f.extend([V::I(p.unwrap_or(0)), V::I(c), V::I(b), V::I(t), V::I(a[0]), avg(a[1], a[0]), nullable(a[3], a[2]), nullable(a[5], a[4])]);
        f
    })
}

// WITH UserReputation AS (
// SELECT u.Id AS UserId, u.DisplayName, u.Reputation, COUNT(b.Id) AS BadgeCount
// FROM Users u
// LEFT JOIN Badges b ON u.Id = b.UserId
// GROUP BY u.Id, u.DisplayName, u.Reputation
// ),
// PostStats AS (
// SELECT p.OwnerUserId, COUNT(p.Id) AS PostCount, SUM(p.Score) AS TotalScore,
// SUM(p.ViewCount) AS TotalViews, AVG(p.AnswerCount) AS AverageAnswers
// FROM Posts p
// WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY p.OwnerUserId
// ),
// CombinedStats AS (
// SELECT ur.UserId, ur.DisplayName, ur.Reputation, ur.BadgeCount,
// ps.PostCount, ps.TotalScore, ps.TotalViews, ps.AverageAnswers
// FROM UserReputation ur
// JOIN PostStats ps ON ur.UserId = ps.OwnerUserId
// )
// SELECT cs.DisplayName, cs.Reputation, cs.BadgeCount, cs.PostCount, cs.TotalScore,
// cs.TotalViews, cs.AverageAnswers
// FROM CombinedStats cs
// ORDER BY cs.Reputation DESC, cs.TotalScore DESC
// LIMIT 10;
fn q5569(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let Post { score, view_count, answer_count, .. } = &db.post;
    let ps = owned_since(db, year_ago()).group_by(&db.post.owner_user).select(score.and(view_count.opt()).and(answer_count.opt())).fold([0i64; 6], |a, ((s, w), an)| {
        [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + an.is_some() as i64, a[5] + an.unwrap_or(0)]
    });
    let mut v = Vec::new();
    (&ps).and(&bu).drive(|u, (a, b)| v.push((u, a, b)));
    out(v, |&(u, a, _)| (rep_desc(db, u), Reverse(a[1])), 10, |&(u, a, b)| {
        vec![user_col(db, u, "name"), user_col(db, u, "rep"), V::I(b), V::I(a[0]), V::I(a[1]), nullable(a[3], a[2]), avg(a[5], a[4])]
    })
}

// WITH UserBadges AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(b.Id) AS BadgeCount,
// SUM(b.Class) AS TotalClass
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// u.Id, u.DisplayName
// ),
// PostScore AS (
// SELECT
// p.OwnerUserId,
// SUM(p.Score) AS TotalPostScore,
// COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS AnswerCount
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// ),
// UserActivity AS (
// SELECT
// u.Id AS UserId,
// COALESCE(ub.BadgeCount, 0) AS BadgeCount,
// COALESCE(ps.TotalPostScore, 0) AS TotalPostScore,
// COALESCE(ps.AnswerCount, 0) AS AnswerCount,
// u.DisplayName
// FROM
// Users u
// LEFT JOIN
// UserBadges ub ON u.Id = ub.UserId
// LEFT JOIN
// PostScore ps ON u.Id = ps.OwnerUserId
// )
// SELECT
// ua.UserId,
// ua.DisplayName,
// ua.BadgeCount,
// ua.TotalPostScore,
// ua.AnswerCount
// FROM
// UserActivity ua
// WHERE
// ua.BadgeCount > 0
// OR ua.TotalPostScore > 0
// OR ua.AnswerCount > 0
// ORDER BY
// ua.TotalPostScore DESC,
// ua.BadgeCount DESC,
// ua.AnswerCount DESC
// LIMIT 10;
fn q5572(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let ps = owned(db).group_by(&db.post.owner_user).select((&db.post.score).and(&db.post.post_type_id)).fold([0i64; 2], |a, (s, t)| [a[0] + s, a[1] + (t == 2) as i64]);
    let mut v = Vec::new();
    db.user
        .select(Ident::<User>::new().and(&bu).and((&ps).opt()))
        .filt(|((_, b), p): ((Id<User>, i64), Option<[i64; 2]>)| b > 0 || p.map_or(false, |p| p[0] > 0 || p[1] > 0))
        .drive(|_, ((u, b), p)| v.push((u, b, p.unwrap_or([0; 2]))));
    out(v, |&(_, b, p)| (Reverse(p[0]), Reverse(b), Reverse(p[1])), 10, |&(u, b, p)| vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(b), V::I(p[0]), V::I(p[1])])
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN p.PostTypeId IN (10, 11) THEN 1 ELSE 0 END) AS TotalClosedPosts
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName
// ), UserBadgeStats AS (
// SELECT
// b.UserId,
// COUNT(b.Id) AS TotalBadges,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM
// Badges b
// GROUP BY
// b.UserId
// ), UserVoteStats AS (
// SELECT
// v.UserId,
// COUNT(v.Id) AS TotalVotes,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Votes v
// GROUP BY
// v.UserId
// )
// SELECT
// ups.UserId,
// ups.DisplayName,
// ups.TotalPosts,
// ups.TotalQuestions,
// ups.TotalAnswers,
// ups.TotalClosedPosts,
// ubs.TotalBadges,
// ubs.GoldBadges,
// ubs.SilverBadges,
// ubs.BronzeBadges,
// uvs.TotalVotes,
// uvs.UpVotes,
// uvs.DownVotes
// FROM
// UserPostStats ups
// LEFT JOIN
// UserBadgeStats ubs ON ups.UserId = ubs.UserId
// LEFT JOIN
// UserVoteStats uvs ON ups.UserId = uvs.UserId
// WHERE
// ups.TotalPosts > 0
// ORDER BY
// ups.TotalPosts DESC, ubs.TotalBadges DESC
// LIMIT 100;
fn q5620(db: &'static So) -> String {
    let uf = g(db).select(posts_of(db).select(&db.post.post_type_id).opt()).fold([0i64; 4], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + matches!(t, 10 | 11) as i64],
        None => a,
    });
    let bc = badge_classes(db);
    let uv = db.vote.group_by(&db.vote.user).select(&db.vote.vote_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64]);
    let mut v = Vec::new();
    (&uf).filt(|a: [i64; 4]| a[0] > 0).and((&bc).opt()).and((&uv).opt()).drive(|u, ((a, b), x)| v.push((u, a, b, x)));
    out(v, |&(_, a, b, _)| (Reverse(a[0]), (b.is_none(), Reverse(b.map(|b| b[0])))), 100, |&(u, a, b, x)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a));
        f.extend((0..4).map(|i| oint(b.map(|b| b[i]))));
        f.extend((0..3).map(|i| oint(x.map(|x| x[i]))));
        f
    })
}

// WITH UserBadgeCounts AS (
// SELECT
// UserId,
// COUNT(*) AS BadgeCount,
// SUM(CASE WHEN Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM Badges
// GROUP BY UserId
// ),
// UserPostStats AS (
// SELECT
// OwnerUserId,
// COUNT(*) AS TotalPosts,
// SUM(CASE WHEN PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(ViewCount) AS TotalViews,
// SUM(Score) AS TotalScore
// FROM Posts
// GROUP BY OwnerUserId
// )
// SELECT
// U.DisplayName,
// U.Reputation,
// U.Location,
// U.CreationDate,
// COALESCE(UBC.BadgeCount, 0) AS BadgeCount,
// COALESCE(UBC.GoldBadges, 0) AS GoldBadges,
// COALESCE(UBC.SilverBadges, 0) AS SilverBadges,
// COALESCE(UBC.BronzeBadges, 0) AS BronzeBadges,
// COALESCE(UPS.TotalPosts, 0) AS TotalPosts,
// COALESCE(UPS.Questions, 0) AS Questions,
// COALESCE(UPS.Answers, 0) AS Answers,
// COALESCE(UPS.TotalViews, 0) AS TotalViews,
// COALESCE(UPS.TotalScore, 0) AS TotalScore
// FROM Users U
// LEFT JOIN UserBadgeCounts UBC ON U.Id = UBC.UserId
// LEFT JOIN UserPostStats UPS ON U.Id = UPS.OwnerUserId
// WHERE U.Reputation > 100
// ORDER BY U.Reputation DESC, BadgeCount DESC
// LIMIT 50;
fn q5686(db: &'static So) -> String {
    let bc = badge_classes(db);
    let Post { post_type_id, view_count, score, .. } = &db.post;
    let ps = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(view_count.opt()).and(score)).fold([0i64; 5], |a, ((t, w), s)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.unwrap_or(0), a[4] + s]
    });
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(100)).select(Ident::<User>::new().and((&bc).opt()).and((&ps).opt())).drive(|_, ((u, b), p)| v.push((u, b.unwrap_or([0; 4]), p.unwrap_or([0; 5]))));
    out(v, |&(u, b, _)| (rep_desc(db, u), Reverse(b[0])), 50, |&(u, b, p)| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep"), ostr(db.user.location.get(u)), user_col(db, u, "ucreated")];
        f.extend(ints(&b));
        f.extend(ints(&p));
        f
    })
}

// WITH UserBadges AS (
// SELECT u.Id AS UserId,
// u.DisplayName,
// COUNT(b.Id) AS BadgeCount,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM Users u
// LEFT JOIN Badges b ON u.Id = b.UserId
// GROUP BY u.Id, u.DisplayName
// ), UserPosts AS (
// SELECT p.OwnerUserId,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(p.ViewCount) AS TotalViews
// FROM Posts p
// GROUP BY p.OwnerUserId
// ), UserActivity AS (
// SELECT u.Id AS UserId,
// u.DisplayName,
// COALESCE(up.TotalPosts, 0) AS TotalPosts,
// COALESCE(up.Questions, 0) AS Questions,
// COALESCE(up.Answers, 0) AS Answers,
// COALESCE(up.TotalViews, 0) AS TotalViews,
// ub.BadgeCount,
// ub.GoldBadges,
// ub.SilverBadges,
// ub.BronzeBadges
// FROM Users u
// LEFT JOIN UserPosts up ON u.Id = up.OwnerUserId
// LEFT JOIN UserBadges ub ON u.Id = ub.UserId
// )
// SELECT UserId,
// DisplayName,
// TotalPosts,
// Questions,
// Answers,
// TotalViews,
// BadgeCount,
// GoldBadges,
// SilverBadges,
// BronzeBadges
// FROM UserActivity
// WHERE TotalPosts > 0
// ORDER BY TotalViews DESC, BadgeCount DESC
// LIMIT 10;
fn q5698(db: &'static So) -> String {
    let ub = ubc(db);
    let ps = owned(db).group_by(&db.post.owner_user).select((&db.post.post_type_id).and((&db.post.view_count).opt())).fold([0i64; 4], |a, (t, w)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.unwrap_or(0)]
    });
    let mut v = Vec::new();
    (&ps).filt(|a: [i64; 4]| a[0] > 0).and(&ub).drive(|u, (p, b)| v.push((u, p, b)));
    out(v, |&(_, p, b)| (Reverse(p[3]), Reverse(b[0])), 10, |&(u, p, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&p));
        f.extend(ints(&b));
        f
    })
}

// WITH PostAggregates AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.PostTypeId,
// COUNT(c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
// MAX(ph.CreationDate) AS LastHistoryEntryDate
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.PostTypeId
// ),
// UserReputation AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// SUM(b.Class) AS TotalBadges,
// AVG(u.Reputation) AS AvgReputation
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// u.Id, u.DisplayName
// )
// SELECT
// pa.PostId,
// pa.Title,
// pa.CreationDate,
// pt.Name AS PostTypeName,
// pa.CommentCount,
// pa.VoteCount,
// pa.LastHistoryEntryDate,
// ur.DisplayName AS AuthorDisplayName,
// ur.TotalBadges,
// ur.AvgReputation
// FROM
// PostAggregates pa
// JOIN
// PostTypes pt ON pa.PostTypeId = pt.Id
// JOIN
// Users au ON pa.PostId = au.Id
// JOIN
// UserReputation ur ON au.Id = ur.UserId
// ORDER BY
// pa.VoteCount DESC,
// pa.CommentCount DESC
// LIMIT 100;
fn q5719(db: &'static So) -> String {
    let uid = uids(db);
    let ur = g(db).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 2], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + c],
        None => a,
    });
    let mut v = Vec::new();
    stats_fold(db, since(db, year_ago()).with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cvh", &[])
        .and(votes_per_post(db))
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&ur)))
        .drive(|p, ((s, x), (u, a))| v.push((p, s, x, u, a)));
    out(v, |&(_, s, x, _, _)| (Reverse(x), Reverse(s.cx)), 100, |&(p, s, x, u, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "type"]);
        f.extend([V::I(s.cx), V::I(x), stat_field(&s, "hmax").unwrap(), user_col(db, u, "name"), nullable(a[1], a[0]), V::F(db.user.reputation.get(u).unwrap() as f64)]);
        f
    })
}

// WITH UserBadgeCounts AS (
// SELECT
// u.Id AS UserId,
// COUNT(b.Id) AS BadgeCount,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM Users u
// LEFT JOIN Badges b ON u.Id = b.UserId
// GROUP BY u.Id
// ),
// PostVoteSummary AS (
// SELECT
// p.Id AS PostId,
// p.OwnerUserId,
// COUNT(v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM Posts p
// LEFT JOIN Votes v ON p.Id = v.PostId
// WHERE p.CreationDate >= '2021-01-01'
// GROUP BY p.Id, p.OwnerUserId
// ),
// PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.ViewCount,
// ps.VoteCount,
// ps.UpVotes,
// ps.DownVotes,
// ub.BadgeCount AS UserBadgeCount,
// ub.GoldBadges,
// ub.SilverBadges,
// ub.BronzeBadges
// FROM Posts p
// INNER JOIN PostVoteSummary ps ON p.Id = ps.PostId
// LEFT JOIN UserBadgeCounts ub ON ub.UserId = ps.OwnerUserId
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.ViewCount,
// ps.VoteCount,
// ps.UpVotes,
// ps.DownVotes,
// COALESCE(ps.UserBadgeCount, 0) AS UserBadgeCount,
// COALESCE(ps.GoldBadges, 0) AS GoldBadges,
// COALESCE(ps.SilverBadges, 0) AS SilverBadges,
// COALESCE(ps.BronzeBadges, 0) AS BronzeBadges
// FROM PostStats ps
// WHERE ps.ViewCount > 1000
// ORDER BY ps.UpVotes DESC, ps.ViewCount DESC
// LIMIT 50;
fn q5766(db: &'static So) -> String {
    let ub = ubc(db);
    let mut v = Vec::new();
    stats_fold(db, since(db, date(2021, 1, 1)).with((&db.post.view_count).gt(1000)), Ident::<Post>::new(), "v", &[])
        .and((&db.post.owner_user).select(&ub).opt())
        .drive(|p, (s, b)| v.push((p, s, b.unwrap_or([0; 4]))));
    out(v, |&(p, s, _)| (Reverse(s.up), views_desc(db, p)), 50, |&(p, s, b)| {
        let mut f = post_fields(db, p, &["id", "title", "views"]);
        f.extend(ints(&[s.vx, s.up, s.down]));
        f.extend(ints(&b));
        f
    })
}

// WITH UserReputation AS (
// SELECT Id, DisplayName, Reputation, UpVotes, DownVotes, Views
// FROM Users
// WHERE Reputation > 1000
// ),
// TopPosts AS (
// SELECT p.Id, p.Title, p.Score, p.ViewCount, u.DisplayName AS OwnerDisplayName
// FROM Posts p
// JOIN UserReputation u ON p.OwnerUserId = u.Id
// WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// AND p.PostTypeId = 1
// ORDER BY p.Score DESC
// LIMIT 10
// ),
// PostHistoryStats AS (
// SELECT ph.PostId, COUNT(ph.Id) AS EditCount,
// MAX(ph.CreationDate) AS LastEdited
// FROM PostHistory ph
// GROUP BY ph.PostId
// )
// SELECT tp.Title, tp.Score, tp.ViewCount,
// tp.OwnerDisplayName,
// p.ClosedDate,
// phs.EditCount,
// phs.LastEdited
// FROM TopPosts tp
// LEFT JOIN Posts p ON tp.Id = p.Id
// LEFT JOIN PostHistoryStats phs ON tp.Id = phs.PostId
// WHERE p.ClosedDate IS NULL
// ORDER BY tp.Score DESC;
fn q5817(db: &'static So) -> String {
    let base = owned_since(db, year_ago()).with((&db.post.post_type_id).eq(1)).with((&db.post.owner_user).select(&db.user.reputation).gt(1000));
    let top: MatSet<Id<Post>> = whole(&base).select(Ident::<Post>::new().and(&db.post.score)).window(row_number, |(_, s)| s, desc).filt(|(_, n)| n <= 10).map(|((p, _), _)| p).collect();
    let hf = history_n_max(db);
    let mut v = Vec::new();
    (&top).select(Ident::<Post>::new().minus(&db.post.closed_date).and((&hf).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(p, h)| {
        let mut f = post_fields(db, p, &["title", "score", "views", "owner", "closed"]);
        f.extend([oint(h.map(|h| h.0)), ots(h.map(|h| h.1))]);
        row(f)
    }))
}

// SELECT
// u.DisplayName AS User,
// p.Title AS Post_Title,
// p.CreationDate AS Post_Creation_Date,
// p.Score AS Post_Score,
// COALESCE(avg_comments.avg_comment_score, 0) AS Average_Comment_Score,
// COALESCE(vote_counts.UpVotes, 0) AS Total_UpVotes,
// COALESCE(vote_counts.DownVotes, 0) AS Total_DownVotes
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// (
// SELECT
// c.PostId,
// AVG(c.Score) AS avg_comment_score
// FROM
// Comments c
// GROUP BY
// c.PostId
// ) avg_comments ON p.Id = avg_comments.PostId
// LEFT JOIN
// (
// SELECT
// v.PostId,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Votes v
// GROUP BY
// v.PostId
// ) vote_counts ON p.Id = vote_counts.PostId
// WHERE
// p.PostTypeId = 1
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q5840(db: &'static So) -> String {
    let cs = db.comment.group_by(&db.comment.post).select(&db.comment.score).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let pv = post_votes(db);
    let mut v = Vec::new();
    owned(db).with((&db.post.post_type_id).eq(1)).select(Ident::<Post>::new().and((&cs).opt()).and((&pv).opt())).drive(|_, x| v.push(x));
    out(v, |&((p, _), _)| newest(db, p), 100, |&((p, c), x)| {
        let x = x.unwrap_or([0; 3]);
        let mut f = post_fields(db, p, &["owner", "title", "created", "score"]);
        f.extend([c.map_or(V::F(0.0), |c| avg(c[1], c[0])), V::I(x[1]), V::I(x[2])]);
        f
    })
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS QuestionCount,
// COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS AnswerCount,
// SUM(p.ViewCount) AS TotalViews,
// SUM(p.Score) AS TotalScore,
// MAX(p.CreationDate) AS LastPostDate
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
// PostHistoryStats AS (
// SELECT
// ph.UserId,
// COUNT(CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN 1 END) AS CloseReopenCount,
// COUNT(CASE WHEN ph.PostHistoryTypeId IN (24, 25) THEN 1 END) AS EditSuggestionCount
// FROM
// PostHistory ph
// GROUP BY
// ph.UserId
// )
// SELECT
// ups.UserId,
// ups.DisplayName,
// ups.TotalPosts,
// ups.QuestionCount,
// ups.AnswerCount,
// ups.TotalViews,
// ups.TotalScore,
// ub.GoldBadges,
// ub.SilverBadges,
// ub.BronzeBadges,
// phs.CloseReopenCount,
// phs.EditSuggestionCount,
// ups.LastPostDate
// FROM
// UserPostStats ups
// LEFT JOIN
// UserBadges ub ON ups.UserId = ub.UserId
// LEFT JOIN
// PostHistoryStats phs ON ups.UserId = phs.UserId
// WHERE
// ups.TotalPosts > 0
// ORDER BY
// ups.TotalScore DESC, ups.LastPostDate DESC
// LIMIT 100;
fn q5843(db: &'static So) -> String {
    let Post { post_type_id, view_count, score, creation_date, .. } = &db.post;
    let uf = g(db).select(posts_of(db).select(post_type_id.and(view_count.opt()).and(score).and(creation_date)).opt()).fold([0, 0, 0, 0, 0, 0, i64::MIN], |a: [i64; 7], p| match p {
        Some((((t, w), s), c)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0), a[5] + s, a[6].max(c)],
        None => a,
    });
    let bc = badge_classes(db);
    let ph = db.post_history.group_by(&db.post_history.user).select(&db.post_history.post_history_type_id).fold([0i64; 2], |a, t| [a[0] + matches!(t, 10 | 11) as i64, a[1] + matches!(t, 24 | 25) as i64]);
    let mut v = Vec::new();
    (&uf).filt(|a: [i64; 7]| a[0] > 0).and((&bc).opt()).and((&ph).opt()).drive(|u, ((a, b), h)| v.push((u, a, b, h)));
    out(v, |&(_, a, _, _)| (Reverse(a[5]), Reverse(a[6])), 100, |&(u, a, b, h)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[..3]));
        f.extend([nullable(a[4], a[3]), V::I(a[5])]);
        f.extend((1..4).map(|i| oint(b.map(|b| b[i]))));
        f.extend((0..2).map(|i| oint(h.map(|h| h[i]))));
        f.push(V::T(a[6]));
        f
    })
}

// WITH UserReputation AS (
// SELECT
// U.Id AS UserId,
// U.Reputation,
// COUNT(DISTINCT P.Id) AS PostCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN P.ViewCount > 100 THEN 1 ELSE 0 END) AS PopularPostCount,
// SUM(CASE WHEN P.Score > 10 THEN 1 ELSE 0 END) AS HighScorePostCount
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id, U.Reputation
// ),
// BadgeStats AS (
// SELECT
// B.UserId,
// COUNT(CASE WHEN B.Class = 1 THEN 1 END) AS GoldBadges,
// COUNT(CASE WHEN B.Class = 2 THEN 1 END) AS SilverBadges,
// COUNT(CASE WHEN B.Class = 3 THEN 1 END) AS BronzeBadges
// FROM
// Badges B
// GROUP BY
// B.UserId
// ),
// Engagement AS (
// SELECT
// UR.UserId,
// UR.Reputation,
// BB.GoldBadges,
// BB.SilverBadges,
// BB.BronzeBadges,
// UR.PostCount,
// UR.QuestionCount,
// UR.AnswerCount,
// UR.PopularPostCount,
// UR.HighScorePostCount
// FROM
// UserReputation UR
// LEFT JOIN
// BadgeStats BB ON UR.UserId = BB.UserId
// )
// SELECT
// E.UserId,
// E.Reputation,
// E.GoldBadges,
// E.SilverBadges,
// E.BronzeBadges,
// E.PostCount,
// E.QuestionCount,
// E.AnswerCount,
// E.PopularPostCount,
// E.HighScorePostCount,
// (CASE
// WHEN E.Reputation > 5000 THEN 'Expert'
// WHEN E.Reputation BETWEEN 1000 AND 5000 THEN 'Experienced'
// ELSE 'Novice'
// END) AS UserLevel
// FROM
// Engagement E
// ORDER BY
// E.Reputation DESC, E.PostCount DESC;
fn q5876(db: &'static So) -> String {
    let Post { post_type_id, view_count, score, .. } = &db.post;
    let uf = g(db).select(posts_of(db).select(post_type_id.and(view_count.opt()).and(score)).opt()).fold([0i64; 5], |a, p| match p {
        Some(((t, w), s)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.map_or(false, |w| w > 100) as i64, a[4] + (s > 10) as i64],
        None => a,
    });
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&uf).and((&bc).opt()).drive(|u, (a, b)| v.push((u, a, b)));
    rows(v.iter().map(|&(u, a, b)| {
        let r = db.user.reputation.get(u).unwrap();
        let mut f = vec![user_col(db, u, "uid"), V::I(r)];
        f.extend((1..4).map(|i| oint(b.map(|b| b[i]))));
        f.extend(ints(&a));
        f.push(V::S(if r > 5000 {
            "Expert"
        } else if (1000..=5000).contains(&r) {
            "Experienced"
        } else {
            "Novice"
        }));
        row(f)
    }))
}

// WITH UserBadges AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(B.Id) AS TotalBadges,
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
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(P.ViewCount) AS TotalViews,
// AVG(P.Score) AS AverageScore
// FROM
// Posts P
// GROUP BY
// P.OwnerUserId
// ),
// UserPerformance AS (
// SELECT
// U.DisplayName,
// COALESCE(UB.TotalBadges, 0) AS TotalBadges,
// COALESCE(PS.TotalPosts, 0) AS TotalPosts,
// COALESCE(PS.Questions, 0) AS Questions,
// COALESCE(PS.Answers, 0) AS Answers,
// COALESCE(PS.TotalViews, 0) AS TotalViews,
// COALESCE(PS.AverageScore, 0) AS AverageScore
// FROM
// Users U
// LEFT JOIN
// UserBadges UB ON U.Id = UB.UserId
// LEFT JOIN
// PostStatistics PS ON U.Id = PS.OwnerUserId
// )
// SELECT
// U.DisplayName,
// U.TotalBadges,
// U.TotalPosts,
// U.Questions,
// U.Answers,
// U.TotalViews,
// U.AverageScore
// FROM
// UserPerformance U
// ORDER BY
// U.TotalBadges DESC,
// U.TotalPosts DESC,
// U.TotalViews DESC;
fn q5931(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let Post { post_type_id, view_count, score, .. } = &db.post;
    let ps = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(view_count.opt()).and(score)).fold([0i64; 5], |a, ((t, w), s)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.unwrap_or(0), a[4] + s]
    });
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and(&bu).and((&ps).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((u, b), p)| {
        let p0 = p.unwrap_or([0; 5]);
        let mut f = vec![user_col(db, u, "name"), V::I(b)];
        f.extend(ints(&p0[..4]));
        f.push(p.map_or(V::F(0.0), |p| avg(p[4], p[0])));
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
// SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END) AS TotalPositivePosts,
// SUM(CASE WHEN P.Score < 0 THEN 1 ELSE 0 END) AS TotalNegativePosts,
// AVG(P.Score) AS AverageScore,
// MAX(P.CreationDate) AS LastPostDate
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id, U.DisplayName
// ),
// PostHistoryCounts AS (
// SELECT
// PH.UserId,
// COUNT(PH.Id) AS TotalEdits,
// SUM(CASE WHEN PHT.Name = 'Edit Body' THEN 1 ELSE 0 END) AS TotalBodyEdits,
// SUM(CASE WHEN PHT.Name = 'Edit Title' THEN 1 ELSE 0 END) AS TotalTitleEdits
// FROM
// PostHistory PH
// JOIN
// PostHistoryTypes PHT ON PH.PostHistoryTypeId = PHT.Id
// GROUP BY
// PH.UserId
// ),
// BadgesSummary AS (
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
// UPS.TotalPositivePosts,
// UPS.TotalNegativePosts,
// UPS.AverageScore,
// UPS.LastPostDate,
// PHC.TotalEdits,
// PHC.TotalBodyEdits,
// PHC.TotalTitleEdits,
// BS.TotalBadges,
// BS.GoldBadges,
// BS.SilverBadges,
// BS.BronzeBadges
// FROM
// UserPostStats UPS
// LEFT JOIN
// PostHistoryCounts PHC ON UPS.UserId = PHC.UserId
// LEFT JOIN
// BadgesSummary BS ON UPS.UserId = BS.UserId
// ORDER BY
// UPS.TotalPosts DESC, UPS.AverageScore DESC;
fn q6002(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, .. } = &db.post;
    let uf = g(db).select(posts_of(db).select(post_type_id.and(score).and(creation_date)).opt()).fold([0, 0, 0, 0, 0, 0, i64::MIN], |a: [i64; 7], p| match p {
        Some(((t, s), c)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (s > 0) as i64, a[4] + (s < 0) as i64, a[5] + s, a[6].max(c)],
        None => a,
    });
    let ph = db
        .post_history
        .group_by(&db.post_history.user)
        .select((&db.post_history.post_history_type).select(&db.post_history_type.name))
        .fold([0i64; 3], |a, n| [a[0] + 1, a[1] + (n == "Edit Body") as i64, a[2] + (n == "Edit Title") as i64]);
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&uf).and((&ph).opt()).and((&bc).opt()).drive(|u, ((a, h), b)| v.push((u, a, h, b)));
    rows(v.iter().map(|&(u, a, h, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[..5]));
        f.extend([avg(a[5], a[0]), if a[0] == 0 { V::Null } else { V::T(a[6]) }]);
        f.extend((0..3).map(|i| oint(h.map(|h| h[i]))));
        f.extend((0..4).map(|i| oint(b.map(|b| b[i]))));
        row(f)
    }))
}

// WITH UserBadgeCounts AS (
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
// UserPostStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS ScorePositive,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers
// FROM
// Posts p
// WHERE
// p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'
// GROUP BY
// p.OwnerUserId
// ),
// CombinedStats AS (
// SELECT
// ubc.UserId,
// ubc.DisplayName,
// ubc.BadgeCount,
// ubc.GoldBadges,
// ubc.SilverBadges,
// ubc.BronzeBadges,
// ups.PostCount,
// ups.ScorePositive,
// ups.Questions,
// ups.Answers
// FROM
// UserBadgeCounts ubc
// LEFT JOIN
// UserPostStats ups ON ubc.UserId = ups.OwnerUserId
// )
// SELECT
// *
// FROM
// CombinedStats
// ORDER BY
// BadgeCount DESC,
// ScorePositive DESC,
// PostCount DESC
// LIMIT 100;
fn q6055(db: &'static So) -> String {
    let ub = ubc(db);
    let rp = owned_since(db, date(2023, 10, 1)).group_by(&db.post.owner_user).select((&db.post.score).and(&db.post.post_type_id)).fold([0i64; 4], |a, (s, t)| {
        [a[0] + 1, a[1] + (s > 0) as i64, a[2] + (t == 1) as i64, a[3] + (t == 2) as i64]
    });
    let mut v = Vec::new();
    (&ub).and((&rp).opt()).drive(|u, (b, p)| v.push((u, b, p)));
    out(v, |&(_, b, p)| (Reverse(b[0]), (p.is_none(), Reverse(p.map(|p| p[1]))), (p.is_none(), Reverse(p.map(|p| p[0])))), 100, |&(u, b, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&b));
        f.extend((0..4).map(|i| oint(p.map(|p| p[i]))));
        f
    })
}

// WITH UserStatistics AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// (SELECT COUNT(*) FROM Posts p WHERE p.OwnerUserId = u.Id) AS TotalPosts,
// (SELECT COUNT(*) FROM Comments c WHERE c.UserId = u.Id) AS TotalComments,
// (SELECT COUNT(*) FROM Badges b WHERE b.UserId = u.Id) AS TotalBadges,
// (SELECT SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) FROM Votes v WHERE v.UserId = u.Id) AS TotalUpVotes,
// (SELECT SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) FROM Votes v WHERE v.UserId = u.Id) AS TotalDownVotes
// FROM Users u
// WHERE u.Reputation >= 1000
// ),
// PostStatistics AS (
// SELECT
// p.OwnerUserId,
// SUM(p.ViewCount) AS TotalViews,
// SUM(p.Score) AS TotalScore,
// COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS TotalQuestions,
// COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS TotalAnswers
// FROM Posts p
// GROUP BY p.OwnerUserId
// ),
// CombinedStatistics AS (
// SELECT
// us.DisplayName,
// us.Reputation,
// us.TotalPosts,
// us.TotalComments,
// us.TotalBadges,
// us.TotalUpVotes,
// us.TotalDownVotes,
// ps.TotalViews,
// ps.TotalScore,
// ps.TotalQuestions,
// ps.TotalAnswers
// FROM UserStatistics us
// JOIN PostStatistics ps ON us.UserId = ps.OwnerUserId
// )
// SELECT
// DisplayName,
// Reputation,
// TotalPosts,
// TotalComments,
// TotalBadges,
// TotalUpVotes,
// TotalDownVotes,
// TotalViews,
// TotalScore,
// TotalQuestions,
// TotalAnswers,
// (TotalUpVotes - TotalDownVotes) AS NetVotes
// FROM CombinedStatistics
// ORDER BY Reputation DESC, TotalScore DESC
// LIMIT 10;
fn q6123(db: &'static So) -> String {
    let pc = owned(db).group_by(&db.post.owner_user).fold(0i64, |a, _| a + 1);
    let bu = badges_per_user(db);
    let uv = db.vote.group_by(&db.vote.user).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let Post { post_type_id, view_count, score, .. } = &db.post;
    let ps = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(view_count.opt()).and(score)).fold([0i64; 5], |a, ((t, w), s)| {
        [a[0] + w.is_some() as i64, a[1] + w.unwrap_or(0), a[2] + s, a[3] + (t == 1) as i64, a[4] + (t == 2) as i64]
    });
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(999))
        .select(Ident::<User>::new().and((&pc).opt()).and(comments_per_user(db)).and(&bu).and((&uv).opt()).and(&ps))
        .drive(|_, x| v.push(x));
    out(v, |&(((((u, _), _), _), _), p)| (rep_desc(db, u), Reverse(p[2])), 10, |&(((((u, n), c), b), x), p)| {
        vec![
            user_col(db, u, "name"),
            user_col(db, u, "rep"),
            V::I(n.unwrap_or(0)),
            V::I(c),
            V::I(b),
            oint(x.map(|x| x[0])),
            oint(x.map(|x| x[1])),
            nullable(p[1], p[0]),
            V::I(p[2]),
            V::I(p[3]),
            V::I(p[4]),
            oint(x.map(|x| x[0] - x[1])),
        ]
    })
}

// WITH PostVoteAggregation AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// pt.Name AS PostType,
// COUNT(v.Id) FILTER (WHERE vt.Name = 'UpMod') AS UpVotes,
// COUNT(v.Id) FILTER (WHERE vt.Name = 'DownMod') AS DownVotes,
// COUNT(v.Id) FILTER (WHERE vt.Name = 'Favorite') AS Favorites
// FROM
// Posts p
// LEFT JOIN
// Votes v ON v.PostId = p.Id
// JOIN
// VoteTypes vt ON vt.Id = v.VoteTypeId
// JOIN
// PostTypes pt ON pt.Id = p.PostTypeId
// WHERE
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY
// p.Id, p.Title, pt.Name
// ), TopPostTypes AS (
// SELECT
// PostType,
// SUM(UpVotes) AS TotalUpVotes
// FROM
// PostVoteAggregation
// GROUP BY
// PostType
// ORDER BY
// TotalUpVotes DESC
// LIMIT 5
// )
// SELECT
// pa.PostId,
// pa.Title,
// pa.PostType,
// pa.UpVotes,
// pa.DownVotes,
// pa.Favorites
// FROM
// PostVoteAggregation pa
// JOIN
// TopPostTypes tpt ON pa.PostType = tpt.PostType
// ORDER BY
// pa.UpVotes DESC, pa.Favorites DESC;
fn q6125(db: &'static So) -> String {
    let base = || since(db, year_ago());
    let pv = base().group_by(Ident::<Post>::new()).select(votes_of(db).select((&db.vote.vote_type).select(&db.vote_type.name))).fold([0i64; 3], |a, n| {
        [a[0] + (n == "UpMod") as i64, a[1] + (n == "DownMod") as i64, a[2] + (n == "Favorite") as i64]
    });
    let tf = by_key(base().with(&pv), name(db), &pv, 0i64, |a, x| a + x[0]);
    let top: MatSet<Str> = whole(&tf).select(Same::new().and(&tf)).window(row_number, |(_, s): (Str, i64)| s, desc).filt(|(_, n)| n <= 5).map(|((k, _), _)| k).collect();
    let by_type: HashIdx<Str, Id<Post>> = base().select(name(db)).inv().collect();
    let mut v = Vec::new();
    (&top).select((&by_type).select(Ident::<Post>::new().and(&pv))).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(p, a)| {
        let mut f = post_fields(db, p, &["id", "title", "type"]);
        f.extend(ints(&a));
        row(f)
    }))
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(P.Id) AS PostCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesReceived,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesReceived
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// U.Id, U.DisplayName, U.Reputation
// ),
// BadgeStats AS (
// SELECT
// B.UserId,
// COUNT(B.Id) AS BadgeCount,
// SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldCount,
// SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverCount,
// SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeCount
// FROM
// Badges B
// GROUP BY
// B.UserId
// )
// SELECT
// U.UserId,
// U.DisplayName,
// U.Reputation,
// U.PostCount,
// U.QuestionCount,
// U.AnswerCount,
// COALESCE(B.BadgeCount, 0) AS TotalBadges,
// COALESCE(B.GoldCount, 0) AS GoldBadges,
// COALESCE(B.SilverCount, 0) AS SilverBadges,
// COALESCE(B.BronzeCount, 0) AS BronzeBadges,
// U.UpVotesReceived,
// U.DownVotesReceived
// FROM
// UserStats U
// LEFT JOIN
// BadgeStats B ON U.UserId = B.UserId
// ORDER BY
// U.Reputation DESC,
// U.PostCount DESC
// LIMIT 50;
fn q6131(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&us).and((&bc).opt()).drive(|u, (a, b)| v.push((u, a, b.unwrap_or([0; 4]))));
    out(v, |&(u, a, _)| (rep_desc(db, u), Reverse(a.n)), 50, |&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(ints(&[a.n, a.q, a.a]));
        f.extend(ints(&b));
        f.extend(ints(&[a.up, a.down]));
        f
    })
}

// WITH UserReputation AS (
// SELECT Id, Reputation, CreationDate, DisplayName, LastAccessDate,
// (SELECT COUNT(*) FROM Badges WHERE UserId = Users.Id) AS BadgeCount,
// (SELECT COUNT(*) FROM Posts WHERE OwnerUserId = Users.Id) AS PostCount,
// (SELECT COUNT(*) FROM Comments WHERE UserId = Users.Id) AS CommentCount
// FROM Users
// WHERE Reputation > 1000
// ),
// PostStatistics AS (
// SELECT P.OwnerUserId,
// COUNT(P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(P.Score) AS TotalScore,
// SUM(P.ViewCount) AS TotalViews,
// AVG(P.Score) AS AvgScore,
// AVG(P.ViewCount) AS AvgViews
// FROM Posts P
// GROUP BY P.OwnerUserId
// ),
// FinalReport AS (
// SELECT U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// Stats.TotalPosts,
// Stats.QuestionCount,
// Stats.AnswerCount,
// Stats.TotalScore,
// Stats.TotalViews,
// Stats.AvgScore,
// Stats.AvgViews,
// U.BadgeCount
// FROM UserReputation U
// LEFT JOIN PostStatistics Stats ON U.Id = Stats.OwnerUserId
// )
// SELECT *
// FROM FinalReport
// ORDER BY Reputation DESC, TotalPosts DESC
// LIMIT 100;
fn q6154(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let Post { post_type_id, view_count, score, .. } = &db.post;
    let ps = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(view_count.opt()).and(score)).fold([0i64; 6], |a, ((t, w), s)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0)]
    });
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and(&bu).and((&ps).opt())).drive(|_, x| v.push(x));
    out(v, |&((u, _), p)| (rep_desc(db, u), (p.is_none(), Reverse(p.map(|p| p[0])))), 100, |&((u, b), p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend((0..4).map(|i| oint(p.map(|p| p[i]))));
        match p {
            Some(p) => f.extend([nullable(p[5], p[4]), avg(p[3], p[0]), avg(p[5], p[4])]),
            None => f.extend(nulls(3)),
        }
        f.push(V::I(b));
        f
    })
}

// WITH UserReputation AS (
// SELECT Id, Reputation
// FROM Users
// WHERE Reputation > (SELECT AVG(Reputation) FROM Users)
// ),
// HighScoringPosts AS (
// SELECT P.Id, P.Score, P.Title, U.DisplayName, P.CreationDate, P.OwnerUserId
// FROM Posts P
// JOIN Users U ON P.OwnerUserId = U.Id
// WHERE P.Score > 100 AND P.PostTypeId = 1
// ),
// PostComments AS (
// SELECT C.PostId, COUNT(C.Id) AS CommentCount
// FROM Comments C
// GROUP BY C.PostId
// ),
// PostVoteCounts AS (
// SELECT V.PostId,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM Votes V
// GROUP BY V.PostId
// )
// SELECT PS.Title,
// PS.Score AS PostScore,
// UC.Reputation AS UserReputation,
// COALESCE(PC.CommentCount, 0) AS TotalComments,
// COALESCE(PVC.UpVotes, 0) AS UpVotes,
// COALESCE(PVC.DownVotes, 0) AS DownVotes,
// PS.CreationDate
// FROM HighScoringPosts PS
// JOIN UserReputation UC ON PS.OwnerUserId = UC.Id
// LEFT JOIN PostComments PC ON PS.Id = PC.PostId
// LEFT JOIN PostVoteCounts PVC ON PS.Id = PVC.PostId
// ORDER BY UC.Reputation DESC, PS.Score DESC
// LIMIT 50;
fn q6165(db: &'static So) -> String {
    let u = db.user.select(&db.user.reputation).fold_flat([0i64; 2], |a, r| [a[0] + 1, a[1] + r]);
    let avgr = u[1] as f64 / u[0] as f64;
    let pv = post_votes(db);
    let mut v = Vec::new();
    owned(db)
        .with((&db.post.score).gt(100))
        .with((&db.post.post_type_id).eq(1))
        .with((&db.post.owner_user).select(&db.user.reputation).filt(move |r: i64| r as f64 > avgr))
        .select(Ident::<Post>::new().and(comments_per_post(db)).and((&pv).opt()))
        .drive(|_, x| v.push(x));
    out(v, |&((p, _), _)| (rep_desc(db, db.post.owner_user.get(p).unwrap()), score_desc(db, p)), 50, |&((p, c), x)| {
        let x = x.unwrap_or([0; 3]);
        let mut f = post_fields(db, p, &["title", "score", "rep"]);
        f.extend([V::I(c), V::I(x[1]), V::I(x[2])]);
        f.extend(post_fields(db, p, &["created"]));
        f
    })
}

// WITH UserBadgeCounts AS (
// SELECT
// u.Id AS UserId,
// COUNT(b.Id) AS BadgeCount
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// u.Id
// ),
// TopBadgedUsers AS (
// SELECT
// UserId,
// BadgeCount
// FROM
// UserBadgeCounts
// WHERE
// BadgeCount > 0
// ORDER BY
// BadgeCount DESC
// LIMIT 10
// ),
// PostStatistics AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS PostCount,
// SUM(p.Score) AS TotalScore,
// AVG(p.ViewCount) AS AverageViewCount
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// ),
// UserPostMetrics AS (
// SELECT
// u.Id AS UserId,
// COALESCE(ps.PostCount, 0) AS PostCount,
// COALESCE(ps.TotalScore, 0) AS TotalScore,
// COALESCE(ps.AverageViewCount, 0) AS AverageViewCount,
// COALESCE(bc.BadgeCount, 0) AS BadgeCount
// FROM
// Users u
// LEFT JOIN
// PostStatistics ps ON u.Id = ps.OwnerUserId
// LEFT JOIN
// UserBadgeCounts bc ON u.Id = bc.UserId
// )
// SELECT
// u.DisplayName,
// upm.PostCount,
// upm.TotalScore,
// upm.AverageViewCount,
// upm.BadgeCount
// FROM
// UserPostMetrics upm
// JOIN
// TopBadgedUsers tbu ON upm.UserId = tbu.UserId
// JOIN
// Users u ON upm.UserId = u.Id
// ORDER BY
// upm.BadgeCount DESC,
// upm.TotalScore DESC;
fn q6166(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let base = db.user.with((&bu).filt(|b: i64| b > 0));
    let top: MatSet<Id<User>> = whole(&base).select(Ident::<User>::new().and(&bu)).window(row_number, |(_, b)| b, desc).filt(|(_, n)| n <= 10).map(|((u, _), _)| u).collect();
    let ps = owned(db).group_by(&db.post.owner_user).select((&db.post.score).and((&db.post.view_count).opt())).fold([0i64; 4], |a, (s, w)| {
        [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]
    });
    let mut v = Vec::new();
    (&top).select(Ident::<User>::new().and(&bu).and((&ps).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((u, b), p)| {
        let p = p.unwrap_or([0; 4]);
        row(vec![user_col(db, u, "name"), V::I(p[0]), V::I(p[1]), if p[2] > 0 { avg(p[3], p[2]) } else { V::F(0.0) }, V::I(b)])
    }))
}

// WITH AggregateBadges AS (
// SELECT UserId,
// COUNT(*) AS TotalBadges,
// SUM(CASE WHEN Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM Badges
// GROUP BY UserId
// ),
// PostMetrics AS (
// SELECT OwnerUserId,
// COUNT(*) AS TotalPosts,
// SUM(CASE WHEN PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(ViewCount) AS TotalViews,
// SUM(AnswerCount) AS TotalAcceptedAnswers
// FROM Posts
// GROUP BY OwnerUserId
// ),
// UserStatistics AS (
// SELECT U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// U.CreationDate,
// AB.TotalBadges,
// AB.GoldBadges,
// AB.SilverBadges,
// AB.BronzeBadges,
// PM.TotalPosts,
// PM.TotalQuestions,
// PM.TotalAnswers,
// PM.TotalViews,
// PM.TotalAcceptedAnswers
// FROM Users U
// LEFT JOIN AggregateBadges AB ON U.Id = AB.UserId
// LEFT JOIN PostMetrics PM ON U.Id = PM.OwnerUserId
// )
// SELECT UserId,
// DisplayName,
// Reputation,
// CreationDate,
// TotalBadges,
// GoldBadges,
// SilverBadges,
// BronzeBadges,
// TotalPosts,
// TotalQuestions,
// TotalAnswers,
// TotalViews,
// TotalAcceptedAnswers
// FROM UserStatistics
// WHERE Reputation > 100
// ORDER BY Reputation DESC, TotalPosts DESC
// LIMIT 10;
fn q6203(db: &'static So) -> String {
    let bc = badge_classes(db);
    let Post { post_type_id, view_count, answer_count, .. } = &db.post;
    let ps = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(view_count.opt()).and(answer_count.opt())).fold([0i64; 7], |a, ((t, w), an)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0), a[5] + an.is_some() as i64, a[6] + an.unwrap_or(0)]
    });
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(100)).select(Ident::<User>::new().and((&bc).opt()).and((&ps).opt())).drive(|_, x| v.push(x));
    out(v, |&((u, _), p)| (rep_desc(db, u), (p.is_none(), Reverse(p.map(|p| p[0])))), 10, |&((u, b), p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep"), user_col(db, u, "ucreated")];
        f.extend((0..4).map(|i| oint(b.map(|b| b[i]))));
        f.extend((0..3).map(|i| oint(p.map(|p| p[i]))));
        f.extend([p.map_or(V::Null, |p| nullable(p[4], p[3])), p.map_or(V::Null, |p| nullable(p[6], p[5]))]);
        f
    })
}

// WITH UserWithBadges AS (
// SELECT u.Id AS UserId, u.DisplayName, COUNT(b.Id) AS BadgeCount
// FROM Users u
// LEFT JOIN Badges b ON u.Id = b.UserId
// GROUP BY u.Id, u.DisplayName
// ),
// PostStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(p.Score) AS TotalScore,
// SUM(p.ViewCount) AS TotalViews
// FROM Posts p
// GROUP BY p.OwnerUserId
// ),
// ActiveUsers AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// ub.BadgeCount,
// ps.PostCount,
// ps.QuestionCount,
// ps.AnswerCount,
// ps.TotalScore,
// ps.TotalViews
// FROM Users u
// JOIN UserWithBadges ub ON u.Id = ub.UserId
// JOIN PostStats ps ON u.Id = ps.OwnerUserId
// WHERE u.LastAccessDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// )
// SELECT
// au.DisplayName,
// au.Reputation,
// au.BadgeCount,
// au.PostCount,
// au.QuestionCount,
// au.AnswerCount,
// au.TotalScore,
// au.TotalViews
// FROM ActiveUsers au
// ORDER BY au.Reputation DESC, au.BadgeCount DESC, au.TotalScore DESC
// LIMIT 10;
fn q6259(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let Post { post_type_id, view_count, score, .. } = &db.post;
    let ps = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(view_count.opt()).and(score)).fold([0i64; 6], |a, ((t, w), s)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0)]
    });
    let mut v = Vec::new();
    db.user.with((&db.user.last_access_date).gt(year_ago())).select(Ident::<User>::new().and(&bu).and(&ps)).drive(|_, x| v.push(x));
    out(v, |&((u, b), p)| (rep_desc(db, u), Reverse(b), Reverse(p[3])), 10, |&((u, b), p)| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep"), V::I(b)];
        f.extend(ints(&p[..4]));
        f.push(nullable(p[5], p[4]));
        f
    })
}

// WITH UserBadgeCounts AS (
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
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(P.Score) AS TotalScore,
// SUM(P.ViewCount) AS TotalViews
// FROM
// Posts P
// GROUP BY
// P.OwnerUserId
// )
// SELECT
// U.DisplayName,
// U.Reputation,
// UBC.BadgeCount,
// PS.PostCount,
// PS.QuestionCount,
// PS.AnswerCount,
// PS.TotalScore,
// PS.TotalViews,
// U.CreationDate,
// U.LastAccessDate,
// U.Location
// FROM
// UserBadgeCounts UBC
// JOIN
// Users U ON UBC.UserId = U.Id
// JOIN
// PostStats PS ON U.Id = PS.OwnerUserId
// WHERE
// UBC.BadgeCount > 0
// ORDER BY
// U.Reputation DESC,
// PS.TotalScore DESC
// LIMIT 10;
fn q6270(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let Post { post_type_id, view_count, score, .. } = &db.post;
    let ps = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(view_count.opt()).and(score)).fold([0i64; 6], |a, ((t, w), s)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0)]
    });
    let mut v = Vec::new();
    (&ps).and((&bu).filt(|b: i64| b > 0)).drive(|u, (p, b)| v.push((u, p, b)));
    out(v, |&(u, p, _)| (rep_desc(db, u), Reverse(p[3])), 10, |&(u, p, b)| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep"), V::I(b)];
        f.extend(ints(&p[..4]));
        f.extend([nullable(p[5], p[4]), user_col(db, u, "ucreated"), user_col(db, u, "last_access"), ostr(db.user.location.get(u))]);
        f
    })
}

// WITH UserReputation AS (
// SELECT u.Id AS UserId, u.Reputation, COUNT(b.Id) AS BadgeCount
// FROM Users u
// LEFT JOIN Badges b ON u.Id = b.UserId
// GROUP BY u.Id, u.Reputation
// ),
// PostStats AS (
// SELECT p.OwnerUserId,
// COUNT(p.Id) AS PostCount,
// SUM(p.Score) AS TotalScore,
// AVG(p.ViewCount) AS AvgViews
// FROM Posts p
// GROUP BY p.OwnerUserId
// ),
// ClosedPostReasons AS (
// SELECT ph.UserId,
// ph.Comment AS CloseReason,
// COUNT(ph.Id) AS CloseCount
// FROM PostHistory ph
// WHERE ph.PostHistoryTypeId = 10
// GROUP BY ph.UserId, ph.Comment
// ),
// CombinedStats AS (
// SELECT ur.UserId,
// ur.Reputation,
// ur.BadgeCount,
// ps.PostCount,
// ps.TotalScore,
// ps.AvgViews,
// cpr.CloseCount,
// cpr.CloseReason
// FROM UserReputation ur
// LEFT JOIN PostStats ps ON ur.UserId = ps.OwnerUserId
// LEFT JOIN ClosedPostReasons cpr ON ur.UserId = cpr.UserId
// )
// SELECT UserId,
// Reputation,
// BadgeCount,
// PostCount,
// TotalScore,
// AvgViews,
// CloseCount,
// CloseReason
// FROM CombinedStats
// WHERE PostCount > 5 AND Reputation > 1000
// ORDER BY TotalScore DESC, Reputation DESC;
fn q6349(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let ps = owned(db).group_by(&db.post.owner_user).select((&db.post.score).and((&db.post.view_count).opt())).fold([0i64; 4], |a, (s, w)| {
        [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]
    });
    let PostHistory { user, comment, .. } = &db.post_history;
    let base = || db.post_history.with((&db.post_history.post_history_type_id).eq(10));
    let cr = base().group_by(user.and(comment.opt())).fold(0i64, |a, _| a + 1);
    let keys: MatSet<(Id<User>, Option<Str>)> = base().select(user.and(comment.opt())).collect();
    let by_u: HashIdx<Id<User>, (Id<User>, Option<Str>)> = (&keys).map(|(u, _)| u).inv().collect();
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000))
        .select(Ident::<User>::new().and(&bu).and((&ps).filt(|p: [i64; 4]| p[0] > 5)).and((&by_u).select(Same::<(Id<User>, Option<Str>)>::new().and(&cr)).opt()))
        .drive(|_, x| v.push(x));
    rows(v.iter().map(|&(((u, b), p), c)| {
        row(vec![user_col(db, u, "uid"), user_col(db, u, "rep"), V::I(b), V::I(p[0]), V::I(p[1]), avg(p[3], p[2]), oint(c.map(|c| c.1)), ostr(c.and_then(|c| (c.0).1))])
    }))
}

// WITH PostVoteStats AS (
// SELECT
// P.Id AS PostId,
// COUNT(V.Id) AS VoteCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Posts P
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// P.Id
// ),
// UserBadgeStats AS (
// SELECT
// U.Id AS UserId,
// COUNT(B.Id) AS BadgeCount,
// SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM
// Users U
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// GROUP BY
// U.Id
// ),
// PostSummary AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// PS.VoteCount,
// PS.UpVotes,
// PS.DownVotes,
// U.DisplayName AS OwnerDisplayName,
// U.Reputation AS OwnerReputation,
// UBadges.BadgeCount,
// UBadges.GoldBadges,
// UBadges.SilverBadges,
// UBadges.BronzeBadges
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// JOIN
// PostVoteStats PS ON P.Id = PS.PostId
// LEFT JOIN
// UserBadgeStats UBadges ON U.Id = UBadges.UserId
// )
// SELECT
// PS.PostId,
// PS.Title,
// PS.CreationDate,
// PS.VoteCount,
// PS.UpVotes,
// PS.DownVotes,
// PS.OwnerDisplayName,
// PS.OwnerReputation,
// PS.BadgeCount,
// PS.GoldBadges,
// PS.SilverBadges,
// PS.BronzeBadges,
// (SELECT COUNT(*) FROM Comments C WHERE C.PostId = PS.PostId) AS CommentCount,
// (SELECT COUNT(*) FROM Posts P2 WHERE P2.ParentId = PS.PostId AND P2.PostTypeId = 2) AS AnswerCount
// FROM
// PostSummary PS
// ORDER BY
// PS.VoteCount DESC, PS.CreationDate DESC
// LIMIT 100;
fn q6388(db: &'static So) -> String {
    let ub = ubc(db);
    let pv = post_votes(db);
    let mut v = Vec::new();
    owned(db)
        .select(Ident::<Post>::new().and((&pv).opt()).and((&db.post.owner_user).select(&ub)).and(comments_per_post(db)).and(typed_answers_per_post(db)))
        .drive(|_, x| v.push(x));
    out(v, |&((((p, x), _), _), _)| (Reverse(x.map_or(0, |x| x[0])), newest(db, p)), 100, |&((((p, x), b), c), a)| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend(ints(&x.unwrap_or([0; 3])));
        f.extend(post_fields(db, p, &["owner", "rep"]));
        f.extend(ints(&b));
        f.extend([V::I(c), V::I(a)]);
        f
    })
}

// WITH TopPosts AS (
// SELECT
// p.Id,
// p.Title,
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
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, p.Score, p.ViewCount
// ),
// UserBadges AS (
// SELECT
// u.Id AS UserId,
// COUNT(b.Id) AS BadgeCount
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// u.Id
// ),
// TopUsers AS (
// SELECT
// u.Id,
// u.DisplayName,
// u.Reputation,
// ub.BadgeCount
// FROM
// Users u
// LEFT JOIN
// UserBadges ub ON u.Id = ub.UserId
// ORDER BY
// u.Reputation DESC
// LIMIT 10
// )
// SELECT
// tp.Title,
// tp.Score,
// tp.ViewCount,
// tp.CommentCount,
// tp.AnswerCount,
// tu.DisplayName AS TopUser,
// tu.Reputation,
// tu.BadgeCount
// FROM
// TopPosts tp
// JOIN
// TopUsers tu ON tp.Id IN (SELECT p.Id FROM Posts p WHERE p.OwnerUserId = tu.Id)
// ORDER BY
// tp.Score DESC, tp.ViewCount DESC;
fn q6394(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let top: MatSet<Id<User>> = whole(&db.user.id).select(Ident::<User>::new().and(&db.user.reputation)).window(row_number, |(_, r)| r, desc).filt(|(_, n)| n <= 10).map(|((u, _), _)| u).collect();
    let an = per_post_distinct(db, children_of(db));
    let mut v = Vec::new();
    (&top)
        .select(Ident::<User>::new().and(&bu).and(posts_of(db).select(Ident::<Post>::new().with((&db.post.post_type_id).eq(1)).and(comments_per_post(db)).and((&an).opt()))))
        .drive(|_, x| v.push(x));
    rows(v.iter().map(|&((u, b), ((p, c), a))| {
        let mut f = post_fields(db, p, &["title", "score", "views"]);
        f.extend([V::I(c), V::I(a.unwrap_or(0)), user_col(db, u, "name"), user_col(db, u, "rep"), V::I(b)]);
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
// ActivePosts AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// AVG(p.Score) AS AvgScore,
// SUM(p.ViewCount) AS TotalViews
// FROM
// Posts p
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
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
// ap.PostCount,
// ap.Questions,
// ap.Answers,
// ap.AvgScore,
// ap.TotalViews
// FROM
// UserBadges ub
// LEFT JOIN
// ActivePosts ap ON ub.UserId = ap.OwnerUserId
// WHERE
// ub.BadgeCount > 0
// ORDER BY
// ub.BadgeCount DESC, ap.TotalViews DESC
// LIMIT 50;
fn q6397(db: &'static So) -> String {
    let bc = badge_classes(db);
    let Post { post_type_id, view_count, score, .. } = &db.post;
    let rp = owned_since(db, year_ago()).group_by(&db.post.owner_user).select(post_type_id.and(view_count.opt()).and(score)).fold([0i64; 6], |a, ((t, w), s)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0)]
    });
    let tv = |p: Option<[i64; 6]>| p.filter(|p| p[4] > 0).map(|p| p[5]);
    let mut v = Vec::new();
    (&bc).and((&rp).opt()).drive(|u, (b, p)| v.push((u, b, p)));
    out(v, |&(_, b, p)| (Reverse(b[0]), (tv(p).is_none(), Reverse(tv(p)))), 50, |&(u, b, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&b));
        f.extend((0..3).map(|i| oint(p.map(|p| p[i]))));
        f.extend([p.map_or(V::Null, |p| avg(p[3], p[0])), oint(tv(p))]);
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
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// u.Id, u.DisplayName
// ), PostsSummary AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(CASE WHEN p.PostTypeId = 3 THEN 1 ELSE 0 END) AS Wikis,
// SUM(p.Score) AS TotalScore,
// SUM(p.ViewCount) AS TotalViews
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// ), UsersPerformance AS (
// SELECT
// ub.UserId,
// ub.DisplayName,
// ub.BadgeCount,
// ub.GoldBadges,
// ub.SilverBadges,
// ub.BronzeBadges,
// ps.TotalPosts,
// ps.Questions,
// ps.Answers,
// ps.Wikis,
// ps.TotalScore,
// ps.TotalViews
// FROM
// UserBadges ub
// JOIN
// PostsSummary ps ON ub.UserId = ps.OwnerUserId
// )
// SELECT
// up.DisplayName,
// up.BadgeCount,
// up.GoldBadges,
// up.SilverBadges,
// up.BronzeBadges,
// up.TotalPosts,
// up.Questions,
// up.Answers,
// up.Wikis,
// up.TotalScore,
// up.TotalViews
// FROM
// UsersPerformance up
// ORDER BY
// up.TotalScore DESC,
// up.BadgeCount DESC
// FETCH FIRST 10 ROWS ONLY;
fn q6425(db: &'static So) -> String {
    let ub = ubc(db);
    let Post { post_type_id, view_count, score, .. } = &db.post;
    let ps = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(view_count.opt()).and(score)).fold([0i64; 7], |a, ((t, w), s)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (t == 3) as i64, a[4] + s, a[5] + w.is_some() as i64, a[6] + w.unwrap_or(0)]
    });
    let mut v = Vec::new();
    (&ub).and(&ps).drive(|u, (b, p)| v.push((u, b, p)));
    out(v, |&(_, b, p)| (Reverse(p[4]), Reverse(b[0])), 10, |&(u, b, p)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(ints(&b));
        f.extend(ints(&p[..5]));
        f.push(nullable(p[6], p[5]));
        f
    })
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
// WHERE
// U.Reputation > 1000
// GROUP BY
// U.Id, U.DisplayName
// ),
// UserPosts AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS PostCount,
// SUM(P.Score) AS TotalScore,
// AVG(P.ViewCount) AS AverageViews
// FROM
// Posts P
// GROUP BY
// P.OwnerUserId
// ),
// UserActivity AS (
// SELECT
// U.Id,
// U.DisplayName,
// UB.BadgeCount,
// UB.GoldBadges,
// UB.SilverBadges,
// UB.BronzeBadges,
// UP.PostCount,
// UP.TotalScore,
// UP.AverageViews
// FROM
// Users U
// LEFT JOIN
// UserBadges UB ON U.Id = UB.UserId
// LEFT JOIN
// UserPosts UP ON U.Id = UP.OwnerUserId
// )
// SELECT
// UA.DisplayName,
// UA.BadgeCount,
// UA.GoldBadges,
// UA.SilverBadges,
// UA.BronzeBadges,
// UA.PostCount,
// UA.TotalScore,
// UA.AverageViews
// FROM
// UserActivity UA
// WHERE
// UA.BadgeCount > 5
// AND UA.PostCount > 10
// ORDER BY
// UA.TotalScore DESC,
// UA.BadgeCount DESC
// LIMIT 10;
fn q6469(db: &'static So) -> String {
    let ub = user_base(db, UserWhere::RepGt(1000)).group_by(Ident::<User>::new()).select(badges_of(db).select(&db.badge.class).opt()).fold([0i64; 4], |a, c| match c {
        Some(c) => [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64],
        None => a,
    });
    let ps = owned(db).group_by(&db.post.owner_user).select((&db.post.score).and((&db.post.view_count).opt())).fold([0i64; 4], |a, (s, w)| {
        [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]
    });
    let mut v = Vec::new();
    (&ub).filt(|b: [i64; 4]| b[0] > 5).and((&ps).filt(|p: [i64; 4]| p[0] > 10)).drive(|u, (b, p)| v.push((u, b, p)));
    out(v, |&(_, b, p)| (Reverse(p[1]), Reverse(b[0])), 10, |&(u, b, p)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(ints(&b));
        f.extend([V::I(p[0]), V::I(p[1]), avg(p[3], p[2])]);
        f
    })
}

// WITH UserBadgeCounts AS (
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
// p.OwnerUserId,
// COUNT(p.Id) AS PostCount,
// SUM(p.Score) AS TotalScore,
// AVG(p.ViewCount) AS AverageViews
// FROM
// Posts p
// WHERE
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY
// p.OwnerUserId
// ),
// TopUsers AS (
// SELECT
// ub.UserId,
// ub.DisplayName,
// ub.BadgeCount,
// pp.PostCount,
// pp.TotalScore,
// pp.AverageViews
// FROM
// UserBadgeCounts ub
// JOIN
// PopularPosts pp ON ub.UserId = pp.OwnerUserId
// ORDER BY
// pp.TotalScore DESC, ub.BadgeCount DESC
// LIMIT 10
// )
// SELECT
// tu.DisplayName,
// tu.BadgeCount,
// tu.PostCount,
// tu.TotalScore,
// tu.AverageViews
// FROM
// TopUsers tu
// JOIN
// PostTypes pt ON pt.Id = (
// SELECT
// p.PostTypeId
// FROM
// Posts p
// WHERE
// p.OwnerUserId = tu.UserId
// ORDER BY
// p.CreationDate DESC
// LIMIT 1
// )
// WHERE
// pt.Name IN ('Question', 'Answer')
// ORDER BY
// tu.TotalScore DESC;
fn q6470(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let rp = owned_since(db, year_ago()).group_by(&db.post.owner_user).select((&db.post.score).and((&db.post.view_count).opt())).fold([0i64; 4], |a, (s, w)| {
        [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]
    });
    let top: MatSet<Id<User>> = whole(&rp)
        .select(Same::new().and(&rp).and(&bu))
        .window(row_number, |((_, a), b): ((Id<User>, [i64; 4]), i64)| (a[1], b), desc)
        .filt(|(_, n)| n <= 10)
        .map(|(((u, _), _), _)| u)
        .collect();
    let latest = owned(db).group_by(&db.post.owner_user).select((&db.post.creation_date).and(&db.post.post_type)).fold(None, |a: Option<(i64, Id<PostType>)>, x| Some(a.map_or(x, |a| a.max(x))));
    let lt = (&latest).map(|x: Option<(i64, Id<PostType>)>| x.unwrap().1).select(&db.post_type.name).filt(|n: Str| n == "Question" || n == "Answer");
    let mut v = Vec::new();
    (&top).select(Ident::<User>::new().and(&rp).and(&bu).and(lt)).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(((u, a), b), _)| row(vec![user_col(db, u, "name"), V::I(b), V::I(a[0]), V::I(a[1]), avg(a[3], a[2])])))
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
// PostActivity AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(p.ViewCount) AS TotalViews,
// AVG(p.Score) AS AverageScore
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// ),
// UserSummary AS (
// SELECT
// ub.UserId,
// ub.DisplayName,
// ub.BadgeCount,
// ub.GoldBadges,
// ub.SilverBadges,
// ub.BronzeBadges,
// pa.PostCount,
// pa.Questions,
// pa.Answers,
// pa.TotalViews,
// pa.AverageScore
// FROM
// UserBadges ub
// LEFT JOIN
// PostActivity pa ON ub.UserId = pa.OwnerUserId
// )
// SELECT
// us.DisplayName,
// us.BadgeCount,
// us.GoldBadges,
// us.SilverBadges,
// us.BronzeBadges,
// us.PostCount,
// us.Questions,
// us.Answers,
// us.TotalViews,
// us.AverageScore
// FROM
// UserSummary us
// WHERE
// us.BadgeCount > 0
// ORDER BY
// us.BadgeCount DESC, us.TotalViews DESC
// LIMIT 10;
fn q6537(db: &'static So) -> String {
    let bc = badge_classes(db);
    let Post { post_type_id, view_count, score, .. } = &db.post;
    let ps = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(view_count.opt()).and(score)).fold([0i64; 6], |a, ((t, w), s)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0)]
    });
    let tv = |p: Option<[i64; 6]>| p.filter(|p| p[4] > 0).map(|p| p[5]);
    let mut v = Vec::new();
    (&bc).and((&ps).opt()).drive(|u, (b, p)| v.push((u, b, p)));
    out(v, |&(_, b, p)| (Reverse(b[0]), (tv(p).is_none(), Reverse(tv(p)))), 10, |&(u, b, p)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(ints(&b));
        f.extend((0..3).map(|i| oint(p.map(|p| p[i]))));
        f.extend([oint(tv(p)), p.map_or(V::Null, |p| avg(p[3], p[0]))]);
        f
    })
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// AVG(u.Reputation) AS AvgReputation,
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
// ),
// HighReputationUsers AS (
// SELECT
// UserId,
// DisplayName,
// PostCount,
// AnswerCount,
// QuestionCount,
// AvgReputation,
// UpVotes,
// DownVotes
// FROM
// UserPostStats
// WHERE
// AvgReputation > 1000
// ),
// TopAnswerers AS (
// SELECT
// UserId,
// DisplayName,
// AnswerCount
// FROM
// HighReputationUsers
// WHERE
// AnswerCount > 10
// ORDER BY
// AnswerCount DESC
// LIMIT 5
// )
// SELECT
// u.DisplayName,
// u.PostCount,
// u.QuestionCount,
// u.AnswerCount,
// u.UpVotes,
// u.DownVotes,
// ROUND(COALESCE((SELECT SUM(b.Class) FROM Badges b WHERE b.UserId = u.UserId), 0), 2) AS TotalBadgeScore
// FROM
// HighReputationUsers u
// JOIN
// TopAnswerers t ON u.UserId = t.UserId
// ORDER BY
// u.UpVotes - u.DownVotes DESC;
fn q6655(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::RepGt(1000), "v", any_post);
    let base = db.user.with((&us).filt(|a: UStats| a.a > 10));
    let top: MatSet<Id<User>> = whole(&base).select(Ident::<User>::new().and(&us)).window(row_number, |(_, a): (Id<User>, UStats)| a.a, desc).filt(|(_, n)| n <= 5).map(|((u, _), _)| u).collect();
    let bs = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold(0i64, |a, c| a + c);
    let mut v = Vec::new();
    (&top).select(Ident::<User>::new().and(&us).and((&bs).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((u, a), b)| row(vec![user_col(db, u, "name"), V::I(a.n), V::I(a.q), V::I(a.a), V::I(a.up), V::I(a.down), V::I(b.unwrap_or(0))])))
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(p.Score) AS TotalScore,
// SUM(p.ViewCount) AS TotalViews
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// GROUP BY u.Id, u.DisplayName
// ),
// RecentPostEdits AS (
// SELECT
// ph.UserId,
// COUNT(ph.Id) AS EditCount,
// MAX(ph.CreationDate) AS LastEditDate
// FROM PostHistory ph
// WHERE ph.PostHistoryTypeId IN (4, 5, 6, 24)
// GROUP BY ph.UserId
// ),
// UserBadges AS (
// SELECT
// b.UserId,
// COUNT(b.Id) AS BadgeCount,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM Badges b
// GROUP BY b.UserId
// )
// SELECT
// ups.UserId,
// ups.DisplayName,
// ups.TotalPosts,
// ups.TotalQuestions,
// ups.TotalAnswers,
// ups.TotalScore,
// ups.TotalViews,
// rpe.EditCount,
// rpe.LastEditDate,
// ub.BadgeCount,
// ub.GoldBadges,
// ub.SilverBadges,
// ub.BronzeBadges
// FROM UserPostStats ups
// LEFT JOIN RecentPostEdits rpe ON ups.UserId = rpe.UserId
// LEFT JOIN UserBadges ub ON ups.UserId = ub.UserId
// WHERE ups.TotalPosts > 0
// ORDER BY ups.TotalScore DESC, ups.TotalPosts DESC
// LIMIT 100;
fn q6696(db: &'static So) -> String {
    let Post { post_type_id, view_count, score, .. } = &db.post;
    let uf = g(db).select(posts_of(db).select(post_type_id.and(view_count.opt()).and(score)).opt()).fold([0i64; 6], |a, p| match p {
        Some(((t, w), s)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0)],
        None => a,
    });
    let ed = db
        .post_history
        .with((&db.post_history.post_history_type_id).in_v(vec![4, 5, 6, 24]))
        .group_by(&db.post_history.user)
        .select(&db.post_history.creation_date)
        .fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&uf).filt(|a: [i64; 6]| a[0] > 0).and((&ed).opt()).and((&bc).opt()).drive(|u, ((a, e), b)| v.push((u, a, e, b)));
    out(v, |&(_, a, _, _)| (Reverse(a[3]), Reverse(a[0])), 100, |&(u, a, e, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[..4]));
        f.extend([nullable(a[5], a[4]), oint(e.map(|e| e.0)), ots(e.map(|e| e.1))]);
        f.extend((0..4).map(|i| oint(b.map(|b| b[i]))));
        f
    })
}

// WITH UserBadges AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(B.Id) AS BadgeCount,
// MAX(B.Class) AS HighestBadgeClass
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
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(P.Score) AS TotalScore,
// AVG(P.Score) AS AverageScore
// FROM
// Posts P
// GROUP BY
// P.OwnerUserId
// ),
// CombinedStats AS (
// SELECT
// UB.UserId,
// UB.DisplayName,
// UB.BadgeCount,
// UB.HighestBadgeClass,
// PS.PostCount,
// PS.QuestionCount,
// PS.AnswerCount,
// PS.TotalScore,
// PS.AverageScore
// FROM
// UserBadges UB
// LEFT JOIN
// PostStats PS ON UB.UserId = PS.OwnerUserId
// )
// SELECT
// CS.DisplayName,
// CS.BadgeCount,
// CS.HighestBadgeClass,
// COALESCE(CS.PostCount, 0) AS TotalPosts,
// COALESCE(CS.QuestionCount, 0) AS TotalQuestions,
// COALESCE(CS.AnswerCount, 0) AS TotalAnswers,
// COALESCE(CS.TotalScore, 0) AS TotalScore,
// COALESCE(CS.AverageScore, 0) AS AverageScore
// FROM
// CombinedStats CS
// ORDER BY
// CS.BadgeCount DESC,
// CS.TotalScore DESC
// LIMIT 100;
fn q6698(db: &'static So) -> String {
    let ub = g(db).select(badges_of(db).select(&db.badge.class).opt()).fold([0, i64::MIN], |a: [i64; 2], c| match c {
        Some(c) => [a[0] + 1, a[1].max(c)],
        None => a,
    });
    let ps = owned(db).group_by(&db.post.owner_user).select((&db.post.post_type_id).and(&db.post.score)).fold([0i64; 4], |a, (t, s)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s]
    });
    let mut v = Vec::new();
    (&ub).and((&ps).opt()).drive(|u, (b, p)| v.push((u, b, p)));
    out(v, |&(_, b, p)| (Reverse(b[0]), (p.is_none(), Reverse(p.map(|p| p[3])))), 100, |&(u, b, p)| {
        let mut f = vec![user_col(db, u, "name"), V::I(b[0]), if b[0] == 0 { V::Null } else { V::I(b[1]) }];
        f.extend(ints(&p.unwrap_or([0; 4])));
        f.push(p.map_or(V::F(0.0), |p| avg(p[3], p[0])));
        f
    })
}

// WITH UserVoteStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(v.Id) AS TotalVotes,
// SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS Upvotes,
// SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS Downvotes,
// SUM(CASE WHEN vt.Name = 'Favorite' THEN 1 ELSE 0 END) AS Favorites
// FROM Users u
// LEFT JOIN Votes v ON u.Id = v.UserId
// LEFT JOIN VoteTypes vt ON v.VoteTypeId = vt.Id
// WHERE u.Reputation > 1000
// GROUP BY u.Id, u.DisplayName
// ),
// PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.ViewCount,
// p.Score,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount
// FROM Posts p
// LEFT JOIN Comments c ON p.Id = c.PostId
// LEFT JOIN Votes v ON p.Id = v.PostId
// WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY p.Id, p.Title, p.ViewCount, p.Score
// ),
// UserPostEngagement AS (
// SELECT
// ups.UserId,
// ups.DisplayName,
// COUNT(ps.PostId) AS PostCount,
// SUM(ps.ViewCount) AS TotalViews,
// SUM(ps.CommentCount) AS TotalComments,
// SUM(ps.VoteCount) AS TotalVotes
// FROM UserVoteStats ups
// JOIN Posts pos ON ups.UserId = pos.OwnerUserId
// JOIN PostStats ps ON pos.Id = ps.PostId
// GROUP BY ups.UserId, ups.DisplayName
// )
// SELECT
// upe.UserId,
// upe.DisplayName,
// upe.PostCount,
// upe.TotalViews,
// upe.TotalComments,
// upe.TotalVotes,
// u.Reputation
// FROM UserPostEngagement upe
// JOIN Users u ON upe.UserId = u.Id
// WHERE u.Reputation > 1000
// ORDER BY upe.TotalVotes DESC, upe.TotalViews DESC
// LIMIT 10;
fn q6759(db: &'static So) -> String {
    let pf = stats_fold(db, since(db, year_ago()), Ident::<Post>::new(), "cv", &[]);
    let ue = owned(db)
        .with((&db.post.owner_user).select(&db.user.reputation).gt(1000))
        .group_by(&db.post.owner_user)
        .select((&pf).and((&db.post.view_count).opt()))
        .fold([0i64; 5], |a, (s, w)| [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s.cx, a[4] + s.vx]);
    let mut v = Vec::new();
    (&ue).drive(|u, a| v.push((u, a)));
    out(v, |&(_, a)| (Reverse(a[4]), (a[1] == 0, Reverse(a[2]))), 10, |&(u, a)| {
        vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(a[0]), nullable(a[2], a[1]), V::I(a[3]), V::I(a[4]), user_col(db, u, "rep")]
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
// PostStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS PostCount,
// SUM(p.ViewCount) AS TotalViews,
// AVG(p.Score) AS AverageScore,
// SUM(p.CommentCount) AS TotalComments
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// ),
// CombinedStats AS (
// SELECT
// u.UserId,
// u.DisplayName,
// COALESCE(pb.PostCount, 0) AS PostCount,
// COALESCE(pb.TotalViews, 0) AS TotalViews,
// COALESCE(pb.AverageScore, 0) AS AverageScore,
// COALESCE(pb.TotalComments, 0) AS TotalComments,
// u.BadgeCount,
// u.GoldBadges,
// u.SilverBadges,
// u.BronzeBadges
// FROM
// UserBadgeStats u
// LEFT JOIN
// PostStats pb ON u.UserId = pb.OwnerUserId
// )
// SELECT
// cs.DisplayName,
// cs.PostCount,
// cs.TotalViews,
// cs.AverageScore,
// cs.TotalComments,
// cs.BadgeCount,
// cs.GoldBadges,
// cs.SilverBadges,
// cs.BronzeBadges
// FROM
// CombinedStats cs
// ORDER BY
// cs.PostCount DESC, cs.TotalViews DESC;
fn q6787(db: &'static So) -> String {
    let ub = ubc(db);
    let Post { view_count, score, comment_count, .. } = &db.post;
    let ps = owned(db).group_by(&db.post.owner_user).select(view_count.opt().and(score).and(comment_count)).fold([0i64; 4], |a, ((w, s), c)| [a[0] + 1, a[1] + w.unwrap_or(0), a[2] + s, a[3] + c]);
    let mut v = Vec::new();
    (&ub).and((&ps).opt()).drive(|u, (b, p)| v.push((u, b, p)));
    rows(v.iter().map(|&(u, b, p)| {
        let q = p.unwrap_or([0; 4]);
        let mut f = vec![user_col(db, u, "name"), V::I(q[0]), V::I(q[1]), p.map_or(V::F(0.0), |p| avg(p[2], p[0])), V::I(q[3])];
        f.extend(ints(&b));
        row(f)
    }))
}

// WITH UserReputation AS (
// SELECT Id, Reputation, CreationDate, DisplayName, LastAccessDate
// FROM Users
// WHERE Reputation > 1000
// ),
// PopularQuestions AS (
// SELECT P.Id AS PostId, P.Title, P.CreationDate, P.Score, P.ViewCount, COUNT(A.Id) AS AnswerCount, P.OwnerUserId
// FROM Posts P
// LEFT JOIN Posts A ON P.Id = A.ParentId
// WHERE P.PostTypeId = 1
// GROUP BY P.Id, P.Title, P.CreationDate, P.Score, P.ViewCount, P.OwnerUserId
// HAVING COUNT(A.Id) >= 5 AND P.Score >= 10
// ),
// RecentEdits AS (
// SELECT PH.PostId, PH.UserId, PH.CreationDate, PH.Comment, U.DisplayName AS Editor
// FROM PostHistory PH
// JOIN Users U ON PH.UserId = U.Id
// WHERE PH.PostHistoryTypeId IN (4, 5)
// AND PH.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 month'
// )
// SELECT
// U.DisplayName AS UserName,
// P.Title AS QuestionTitle,
// P.Score AS QuestionScore,
// P.ViewCount AS QuestionViews,
// RE.Editor,
// RE.CreationDate AS EditDate,
// RE.Comment AS EditComment
// FROM UserReputation U
// JOIN PopularQuestions P ON U.Id = P.OwnerUserId
// JOIN RecentEdits RE ON P.PostId = RE.PostId
// ORDER BY P.Score DESC, RE.CreationDate DESC
// LIMIT 10;
fn q6788(db: &'static So) -> String {
    let an = answers_per_post(db);
    let ed: HashIdx<Id<Post>, Id<PostHistory>> = db
        .post_history
        .with((&db.post_history.post_history_type_id).in_v(vec![4, 5]))
        .with((&db.post_history.creation_date).gt(month_ago()))
        .with(&db.post_history.user)
        .select(&db.post_history.post)
        .inv()
        .collect();
    let mut v = Vec::new();
    owned(db)
        .with((&db.post.post_type_id).eq(1))
        .with((&db.post.score).ge(10))
        .with((&db.post.owner_user).select(&db.user.reputation).gt(1000))
        .with((&an).filt(|n: i64| n >= 5))
        .select(Ident::<Post>::new().and(&ed))
        .drive(|_, x| v.push(x));
    let PostHistory { creation_date, user, comment, .. } = &db.post_history;
    out(v, |&(p, h)| (score_desc(db, p), Reverse(creation_date.get(h).unwrap())), 10, |&(p, h)| {
        let mut f = post_fields(db, p, &["owner", "title", "score", "views"]);
        f.extend([V::S(db.user.display_name.get(user.get(h).unwrap()).unwrap()), V::T(creation_date.get(h).unwrap()), ostr(comment.get(h))]);
        f
    })
}

// WITH UserBadges AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(B.Id) AS BadgeCount
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
// COALESCE(SUM(P.Score), 0) AS TotalScore,
// COALESCE(SUM(P.ViewCount), 0) AS TotalViews,
// MAX(P.CreationDate) AS LastPostDate
// FROM
// Posts P
// GROUP BY
// P.OwnerUserId
// ),
// CombinedStats AS (
// SELECT
// U.DisplayName,
// U.Reputation,
// UB.BadgeCount,
// PS.PostCount,
// PS.TotalScore,
// PS.TotalViews,
// PS.LastPostDate
// FROM
// Users U
// JOIN
// UserBadges UB ON U.Id = UB.UserId
// LEFT JOIN
// PostStats PS ON U.Id = PS.OwnerUserId
// )
// SELECT
// DisplayName,
// Reputation,
// BadgeCount,
// PostCount,
// TotalScore,
// TotalViews,
// LastPostDate
// FROM
// CombinedStats
// WHERE
// Reputation > 1000
// AND BadgeCount > 0
// ORDER BY
// TotalScore DESC,
// PostCount DESC
// LIMIT 10;
fn q6804(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let Post { view_count, score, creation_date, .. } = &db.post;
    let ps = owned(db).group_by(&db.post.owner_user).select(view_count.opt().and(score).and(creation_date)).fold([0, 0, 0, i64::MIN], |a: [i64; 4], ((w, s), c)| {
        [a[0] + 1, a[1] + s, a[2] + w.unwrap_or(0), a[3].max(c)]
    });
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and((&bu).filt(|b: i64| b > 0)).and((&ps).opt())).drive(|_, x| v.push(x));
    out(v, |&(_, p)| ((p.is_none(), Reverse(p.map(|p| p[1]))), (p.is_none(), Reverse(p.map(|p| p[0])))), 10, |&((u, b), p)| {
        vec![user_col(db, u, "name"), user_col(db, u, "rep"), V::I(b), oint(p.map(|p| p[0])), oint(p.map(|p| p[1])), oint(p.map(|p| p[2])), ots(p.map(|p| p[3]))]
    })
}

// WITH UserVoteSummary AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(V.Id) AS TotalVotes,
// SUM(CASE WHEN VT.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN VT.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Users U
// LEFT JOIN
// Votes V ON U.Id = V.UserId
// LEFT JOIN
// VoteTypes VT ON V.VoteTypeId = VT.Id
// GROUP BY
// U.Id, U.DisplayName
// ),
// PostSummary AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.OwnerUserId,
// COUNT(C.Id) AS CommentCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// WHERE
// P.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days'
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.OwnerUserId
// ),
// TopUsers AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// S.TotalVotes,
// S.UpVotes,
// S.DownVotes
// FROM
// Users U
// JOIN
// UserVoteSummary S ON U.Id = S.UserId
// WHERE
// S.TotalVotes > 10
// ORDER BY
// U.Reputation DESC
// LIMIT 10
// )
// SELECT
// PS.PostId,
// PS.Title,
// PS.CreationDate,
// T.DisplayName AS Owner,
// PS.CommentCount,
// PS.UpVotes,
// PS.DownVotes
// FROM
// PostSummary PS
// JOIN
// TopUsers T ON PS.OwnerUserId = T.UserId
// ORDER BY
// PS.CreationDate DESC;
fn q6878(db: &'static So) -> String {
    let base = db.user.with(votes_per_user(db).filt(|n: i64| n > 10));
    let top: MatSet<Id<User>> = whole(&base).select(Ident::<User>::new().and(&db.user.reputation)).window(row_number, |(_, r)| r, desc).filt(|(_, n)| n <= 10).map(|((u, _), _)| u).collect();
    let t0 = date(2024, 9, 1);
    let pf = stats_fold(db, since(db, t0), Ident::<Post>::new(), "cv", &[]);
    let mut v = Vec::new();
    (&top).select(Ident::<User>::new().and(posts_of(db).select(Ident::<Post>::new().and(&pf)))).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(u, (p, s))| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([user_col(db, u, "name"), V::I(s.cx), V::I(s.up), V::I(s.down)]);
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
// SUM(CASE WHEN p.ViewCount > 100 THEN 1 ELSE 0 END) AS PopularPosts,
// SUM(p.Score) AS TotalScore
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName
// ),
// BadgeStats AS (
// SELECT
// b.UserId,
// COUNT(b.Id) AS TotalBadges,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM
// Badges b
// GROUP BY
// b.UserId
// )
// SELECT
// ups.UserId,
// ups.DisplayName,
// ups.TotalPosts,
// ups.Questions,
// ups.Answers,
// ups.PopularPosts,
// ups.TotalScore,
// COALESCE(bs.TotalBadges, 0) AS TotalBadges,
// COALESCE(bs.GoldBadges, 0) AS GoldBadges,
// COALESCE(bs.SilverBadges, 0) AS SilverBadges,
// COALESCE(bs.BronzeBadges, 0) AS BronzeBadges
// FROM
// UserPostStats ups
// LEFT JOIN
// BadgeStats bs ON ups.UserId = bs.UserId
// WHERE
// ups.TotalPosts > 10 AND ups.TotalScore > 10
// ORDER BY
// ups.TotalScore DESC, ups.TotalPosts DESC
// LIMIT 50;
fn q6889(db: &'static So) -> String {
    let Post { post_type_id, view_count, score, .. } = &db.post;
    let uf = g(db).select(posts_of(db).select(post_type_id.and(view_count.opt()).and(score)).opt()).fold([0i64; 5], |a, p| match p {
        Some(((t, w), s)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.map_or(false, |w| w > 100) as i64, a[4] + s],
        None => a,
    });
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&uf).filt(|a: [i64; 5]| a[0] > 10 && a[4] > 10).and((&bc).opt()).drive(|u, (a, b)| v.push((u, a, b.unwrap_or([0; 4]))));
    out(v, |&(_, a, _)| (Reverse(a[4]), Reverse(a[0])), 50, |&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a));
        f.extend(ints(&b));
        f
    })
}

// SELECT
// p.Id AS PostId,
// p.Title,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
// COUNT(DISTINCT ph.UserId) AS UniqueEditors,
// MAX(ph.CreationDate) AS LastEditDate,
// p.CreationDate,
// p.ViewCount,
// COALESCE(pl.RelatedPostCount, 0) AS RelatedPostCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// LEFT JOIN
// (SELECT
// PostId,
// COUNT(*) AS RelatedPostCount
// FROM
// PostLinks
// GROUP BY
// PostId) pl ON p.Id = pl.PostId
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, pl.RelatedPostCount
// HAVING
// COUNT(c.Id) > 5 AND
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) > 10
// ORDER BY
// p.ViewCount DESC, UpVoteCount DESC
// LIMIT 100;
fn q6892(db: &'static So) -> String {
    let ed = per_post_distinct(db, history_of(db).select(&db.post_history.user_id));
    let lc = db.post_link.group_by(&db.post_link.post).fold(0i64, |a, _| a + 1);
    let mut v = Vec::new();
    stats_fold(db, questions_only(db), Ident::<Post>::new(), "cvh", &[])
        .filt(|s: Stats| s.cx > 5 && s.up > 10)
        .and((&ed).opt())
        .and((&lc).opt())
        .drive(|p, ((s, e), l)| v.push((p, s, e.unwrap_or(0), l.unwrap_or(0))));
    out(v, |&(p, s, _, _)| (views_desc(db, p), Reverse(s.up)), 100, |&(p, s, e, l)| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([V::I(s.cx), V::I(s.up), V::I(s.down), V::I(e), stat_field(&s, "hmax").unwrap()]);
        f.extend(post_fields(db, p, &["created", "views"]));
        f.push(V::I(l));
        f
    })
}

// WITH UserBadges AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// B.Name AS BadgeName,
// B.Class,
// COUNT(B.Id) AS BadgeCount
// FROM Users U
// JOIN Badges B ON U.Id = B.UserId
// GROUP BY U.Id, U.DisplayName, B.Name, B.Class
// ),
// TopUsers AS (
// SELECT
// UserId,
// DisplayName,
// SUM(CASE WHEN Class = 1 THEN BadgeCount * 3 WHEN Class = 2 THEN BadgeCount * 2 WHEN Class = 3 THEN BadgeCount END) AS TotalScore
// FROM UserBadges
// GROUP BY UserId, DisplayName
// ORDER BY TotalScore DESC
// LIMIT 10
// ),
// PostStats AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS PostCount,
// SUM(P.ViewCount) AS TotalViews,
// AVG(P.Score) AS AverageScore,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount
// FROM Posts P
// GROUP BY P.OwnerUserId
// )
// SELECT
// T.DisplayName,
// PS.PostCount,
// PS.TotalViews,
// PS.AverageScore,
// PS.QuestionCount,
// PS.AnswerCount
// FROM TopUsers T
// JOIN PostStats PS ON T.UserId = PS.OwnerUserId
// ORDER BY T.TotalScore DESC, PS.TotalViews DESC;
fn q6930(db: &'static So) -> String {
    let Badge { user, name: bname, class, .. } = &db.badge;
    let ubk = db.badge.group_by(user.and(bname).and(class)).fold(0i64, |a, _| a + 1);
    let keys: MatSet<((Id<User>, Str), i64)> = db.badge.select(user.and(bname).and(class)).collect();
    let by_u: HashIdx<Id<User>, ((Id<User>, Str), i64)> = (&keys).map(|((u, _), _)| u).inv().collect();
    let tu = db
        .user
        .with(&by_u)
        .group_by(Ident::<User>::new())
        .select((&by_u).select(Same::<((Id<User>, Str), i64)>::new().map(|(_, c)| c).and(&ubk)))
        .fold((0i64, 0i64), |(n, s), (c, k)| match c {
            1 => (n + 1, s + 3 * k),
            2 => (n + 1, s + 2 * k),
            3 => (n + 1, s + k),
            _ => (n, s),
        });
    let top: MatSet<Id<User>> = whole(&tu)
        .select(Same::new().and(&tu))
        .window(row_number, |(_, (n, s)): (Id<User>, (i64, i64))| (n > 0, s), desc)
        .filt(|(_, n)| n <= 10)
        .map(|((u, _), _)| u)
        .collect();
    let Post { view_count, score, post_type_id, .. } = &db.post;
    let ps = owned(db).group_by(&db.post.owner_user).select(view_count.opt().and(score).and(post_type_id)).fold([0i64; 6], |a, ((w, s), t)| {
        [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s, a[4] + (t == 1) as i64, a[5] + (t == 2) as i64]
    });
    let mut v = Vec::new();
    (&top).select(Ident::<User>::new().and(&ps)).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(u, p)| row(vec![user_col(db, u, "name"), V::I(p[0]), nullable(p[2], p[1]), avg(p[3], p[0]), V::I(p[4]), V::I(p[5])])))
}

// WITH RecentPosts AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.ViewCount,
// p.CreationDate,
// p.Score,
// u.DisplayName AS OwnerName,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 2) AS UpVoteCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate > CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days'
// GROUP BY
// p.Id, p.Title, p.ViewCount, p.CreationDate, p.Score, u.DisplayName
// ),
// TopUsers AS (
// SELECT
// u.Id,
// u.DisplayName,
// SUM(p.Score) AS TotalScore,
// SUM(p.ViewCount) AS TotalViews
// FROM
// Users u
// JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName
// ORDER BY
// TotalScore DESC
// LIMIT 10
// )
// SELECT
// rp.PostId,
// rp.Title,
// rp.ViewCount,
// rp.CreationDate,
// rp.Score,
// rp.OwnerName,
// rp.CommentCount,
// rp.UpVoteCount,
// tu.DisplayName AS TopUserName,
// tu.TotalScore,
// tu.TotalViews
// FROM
// RecentPosts rp
// JOIN
// TopUsers tu ON rp.OwnerName = tu.DisplayName
// ORDER BY
// rp.Score DESC, rp.ViewCount DESC
// LIMIT 50;
fn q6952(db: &'static So) -> String {
    let tf = owned(db).group_by(&db.post.owner_user).select((&db.post.score).and((&db.post.view_count).opt())).fold([0i64; 3], |a, (s, w)| {
        [a[0] + s, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0)]
    });
    let top: MatSet<Id<User>> = whole(&tf).select(Same::new().and(&tf)).window(row_number, |(_, a): (Id<User>, [i64; 3])| a[0], desc).filt(|(_, n)| n <= 10).map(|((u, _), _)| u).collect();
    let tn: HashIdx<Str, Id<User>> = (&top).select(&db.user.display_name).inv().collect();
    let pf = stats_fold(db, owned(db).with((&db.post.creation_date).gt(month_ago())), Ident::<Post>::new(), "cv", &[]);
    let mut v = Vec::new();
    (&pf).and((&db.post.owner_user).select(&db.user.display_name).select(&tn).select(Ident::<User>::new().and(&tf))).drive(|p, (s, (u, a))| v.push((p, s, u, a)));
    out(v, |&(p, _, _, _)| score_views(db, p), 50, |&(p, s, u, a)| {
        let mut f = post_fields(db, p, &["id", "title", "views", "created", "score", "owner"]);
        f.extend([V::I(s.cx), V::I(s.up), user_col(db, u, "name"), V::I(a[0]), nullable(a[2], a[1])]);
        f
    })
}

// WITH UserBadgeCounts AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(B.Id) AS BadgeCount
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
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(P.Score) AS TotalScore,
// AVG(P.ViewCount) AS AvgViewCount
// FROM
// Posts P
// GROUP BY
// P.OwnerUserId
// ),
// UserPerformance AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COALESCE(UBC.BadgeCount, 0) AS BadgeCount,
// COALESCE(PS.PostCount, 0) AS PostCount,
// COALESCE(PS.QuestionCount, 0) AS QuestionCount,
// COALESCE(PS.AnswerCount, 0) AS AnswerCount,
// COALESCE(PS.TotalScore, 0) AS TotalScore,
// COALESCE(PS.AvgViewCount, 0) AS AvgViewCount
// FROM
// Users U
// LEFT JOIN
// UserBadgeCounts UBC ON U.Id = UBC.UserId
// LEFT JOIN
// PostStats PS ON U.Id = PS.OwnerUserId
// )
// SELECT
// UserId,
// DisplayName,
// Reputation,
// BadgeCount,
// PostCount,
// QuestionCount,
// AnswerCount,
// TotalScore,
// AvgViewCount
// FROM
// UserPerformance
// ORDER BY
// Reputation DESC,
// TotalScore DESC
// LIMIT 10;
fn q7000(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let Post { post_type_id, view_count, score, .. } = &db.post;
    let ps = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(view_count.opt()).and(score)).fold([0i64; 6], |a, ((t, w), s)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0)]
    });
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and(&bu).and((&ps).opt())).drive(|_, x| v.push(x));
    out(v, |&((u, _), p)| (rep_desc(db, u), Reverse(p.map_or(0, |p| p[3]))), 10, |&((u, b), p)| {
        let q = p.unwrap_or([0; 6]);
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep"), V::I(b)];
        f.extend(ints(&q[..4]));
        f.push(if q[4] > 0 { avg(q[5], q[4]) } else { V::F(0.0) });
        f
    })
}

// WITH UserReputation AS (
// SELECT U.Id, U.DisplayName, U.Reputation,
// SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM Users U
// LEFT JOIN Badges B ON U.Id = B.UserId
// GROUP BY U.Id, U.DisplayName, U.Reputation
// ),
// PostStatistics AS (
// SELECT P.OwnerUserId, COUNT(P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// AVG(P.Score) AS AvgScore
// FROM Posts P
// GROUP BY P.OwnerUserId
// ),
// CombinedStats AS (
// SELECT UR.Id AS UserId, UR.DisplayName, UR.Reputation,
// COALESCE(PS.TotalPosts, 0) AS TotalPosts,
// COALESCE(PS.Questions, 0) AS Questions,
// COALESCE(PS.Answers, 0) AS Answers,
// UR.GoldBadges, UR.SilverBadges, UR.BronzeBadges,
// PS.AvgScore
// FROM UserReputation UR
// LEFT JOIN PostStatistics PS ON UR.Id = PS.OwnerUserId
// )
// SELECT UserId, DisplayName, Reputation, TotalPosts, Questions, Answers,
// GoldBadges, SilverBadges, BronzeBadges, AvgScore
// FROM CombinedStats
// WHERE Reputation > 1000
// ORDER BY Reputation DESC, TotalPosts DESC
// LIMIT 50;
fn q7061(db: &'static So) -> String {
    let ub = ubc(db);
    let ps = owned(db).group_by(&db.post.owner_user).select((&db.post.post_type_id).and(&db.post.score)).fold([0i64; 4], |a, (t, s)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s]
    });
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and(&ub).and((&ps).opt())).drive(|_, x| v.push(x));
    out(v, |&((u, _), p)| (rep_desc(db, u), Reverse(p.map_or(0, |p| p[0]))), 50, |&((u, b), p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(ints(&p.unwrap_or([0; 4])[..3]));
        f.extend(ints(&b[1..]));
        f.push(p.map_or(V::Null, |p| avg(p[3], p[0])));
        f
    })
}

// WITH UserBadges AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(B.Id) AS BadgeCount,
// SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM Users U
// LEFT JOIN Badges B ON U.Id = B.UserId
// GROUP BY U.Id, U.DisplayName
// ),
// PostStats AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS PostCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersCount,
// SUM(P.Score) AS TotalScore,
// SUM(P.ViewCount) AS TotalViews
// FROM Posts P
// GROUP BY P.OwnerUserId
// )
// SELECT
// UB.DisplayName,
// UB.BadgeCount,
// UB.GoldBadges,
// UB.SilverBadges,
// UB.BronzeBadges,
// PS.PostCount,
// PS.QuestionsCount,
// PS.AnswersCount,
// PS.TotalScore,
// PS.TotalViews
// FROM UserBadges UB
// LEFT JOIN PostStats PS ON UB.UserId = PS.OwnerUserId
// WHERE UB.BadgeCount > 10
// ORDER BY UB.BadgeCount DESC, PS.TotalScore DESC
// LIMIT 100;
fn q7074(db: &'static So) -> String {
    let ub = ubc(db);
    let Post { post_type_id, view_count, score, .. } = &db.post;
    let ps = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(view_count.opt()).and(score)).fold([0i64; 6], |a, ((t, w), s)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0)]
    });
    let mut v = Vec::new();
    (&ub).filt(|b: [i64; 4]| b[0] > 10).and((&ps).opt()).drive(|u, (b, p)| v.push((u, b, p)));
    out(v, |&(_, b, p)| (Reverse(b[0]), (p.is_none(), Reverse(p.map(|p| p[3])))), 100, |&(u, b, p)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(ints(&b));
        f.extend((0..4).map(|i| oint(p.map(|p| p[i]))));
        f.push(p.map_or(V::Null, |p| nullable(p[5], p[4])));
        f
    })
}

// WITH PostStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(*) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// AVG(p.Score) AS AvgScore,
// SUM(p.ViewCount) AS TotalViews,
// SUM(p.CommentCount) AS TotalComments
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// ),
// UserBadges AS (
// SELECT
// b.UserId,
// COUNT(*) AS BadgeCount,
// MAX(b.Class) AS HighestBadgeClass
// FROM
// Badges b
// GROUP BY
// b.UserId
// ),
// ActiveUsers AS (
// SELECT
// u.Id,
// u.DisplayName,
// us.PostCount,
// us.QuestionCount,
// us.AnswerCount,
// us.AvgScore,
// us.TotalViews,
// us.TotalComments,
// ub.BadgeCount,
// ub.HighestBadgeClass
// FROM
// Users u
// JOIN
// PostStats us ON u.Id = us.OwnerUserId
// LEFT JOIN
// UserBadges ub ON u.Id = ub.UserId
// WHERE
// u.Reputation > 1000
// )
// SELECT
// au.DisplayName,
// au.PostCount,
// au.QuestionCount,
// au.AnswerCount,
// au.AvgScore,
// au.TotalViews,
// au.TotalComments,
// COALESCE(au.BadgeCount, 0) AS BadgeCount,
// COALESCE(au.HighestBadgeClass, 0) AS HighestBadgeClass
// FROM
// ActiveUsers au
// ORDER BY
// au.AvgScore DESC,
// au.TotalViews DESC
// LIMIT 10;
fn q7139(db: &'static So) -> String {
    let Post { post_type_id, view_count, score, comment_count, .. } = &db.post;
    let ps = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(view_count.opt()).and(score).and(comment_count)).fold([0i64; 7], |a, (((t, w), s), c)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0), a[6] + c]
    });
    let bm = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0, i64::MIN], |a: [i64; 2], c| [a[0] + 1, a[1].max(c)]);
    let avgs = |p: [i64; 7]| p[3] as f64 / p[0] as f64;
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and(&ps).and((&bm).opt())).drive(|_, x| v.push(x));
    out(v, |&((_, p), _)| (Reverse(fkey(avgs(p))), (p[4] == 0, Reverse(p[5]))), 10, |&((u, p), b)| {
        let b = b.unwrap_or([0; 2]);
        let mut f = vec![user_col(db, u, "name")];
        f.extend(ints(&p[..3]));
        f.extend([V::F(avgs(p)), nullable(p[5], p[4]), V::I(p[6]), V::I(b[0]), V::I(b[1])]);
        f
    })
}

// WITH UserBadges AS (
// SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount
// FROM Users U
// LEFT JOIN Badges B ON U.Id = B.UserId
// GROUP BY U.Id, U.DisplayName
// ), PostStats AS (
// SELECT P.OwnerUserId, P.PostTypeId, COUNT(*) AS PostCount, SUM(P.Score) AS TotalScore, SUM(P.ViewCount) AS TotalViews
// FROM Posts P
// GROUP BY P.OwnerUserId, P.PostTypeId
// ), DetailedStatistics AS (
// SELECT
// U.DisplayName,
// UB.BadgeCount,
// PS.PostTypeId,
// PS.PostCount,
// PS.TotalScore,
// PS.TotalViews
// FROM UserBadges UB
// JOIN PostStats PS ON UB.UserId = PS.OwnerUserId
// JOIN Users U ON U.Id = PS.OwnerUserId
// )
// SELECT
// DisplayName,
// BadgeCount,
// SUM(CASE WHEN PostTypeId = 1 THEN PostCount ELSE 0 END) AS Questions,
// SUM(CASE WHEN PostTypeId = 2 THEN PostCount ELSE 0 END) AS Answers,
// SUM(TotalScore) AS AggregateScore,
// SUM(TotalViews) AS AggregateViews
// FROM DetailedStatistics
// GROUP BY DisplayName, BadgeCount
// ORDER BY AggregateScore DESC, AggregateViews DESC;
fn q7141(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let Post { post_type_id, view_count, score, .. } = &db.post;
    let ps = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(view_count.opt()).and(score)).fold([0i64; 5], |a, ((t, w), s)| {
        [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + s, a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0)]
    });
    let f = db.user.with(&ps).group_by((&db.user.display_name).and(&bu)).select(&ps).fold([0i64; 5], |a, p| [a[0] + p[0], a[1] + p[1], a[2] + p[2], a[3] + p[3], a[4] + p[4]]);
    let mut v = Vec::new();
    (&f).drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&((n, b), a)| row(vec![V::S(n), V::I(b), V::I(a[0]), V::I(a[1]), V::I(a[2]), nullable(a[4], a[3])])))
}

// WITH UserBadgeStats AS (
// SELECT
// U.Id AS UserId,
// COUNT(B.Id) AS BadgeCount,
// SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM
// Users U
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// GROUP BY
// U.Id
// ),
// PostStats AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(P.ViewCount) AS TotalViews,
// SUM(P.Score) AS TotalScore
// FROM
// Posts P
// WHERE
// P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 YEAR'
// GROUP BY
// P.OwnerUserId
// ),
// CombinedStats AS (
// SELECT
// U.DisplayName,
// U.Reputation,
// U.Views AS UserViews,
// BS.BadgeCount,
// BS.GoldBadges,
// BS.SilverBadges,
// BS.BronzeBadges,
// PS.TotalPosts,
// PS.Questions,
// PS.Answers,
// PS.TotalViews,
// PS.TotalScore
// FROM
// Users U
// JOIN
// UserBadgeStats BS ON U.Id = BS.UserId
// JOIN
// PostStats PS ON U.Id = PS.OwnerUserId
// WHERE
// U.Reputation >= 1000
// )
// SELECT
// DisplayName,
// Reputation,
// UserViews,
// BadgeCount,
// GoldBadges,
// SilverBadges,
// BronzeBadges,
// TotalPosts,
// Questions,
// Answers,
// TotalViews,
// TotalScore
// FROM
// CombinedStats
// ORDER BY
// TotalScore DESC, UserViews DESC
// LIMIT 10;
fn q7145(db: &'static So) -> String {
    let ub = ubc(db);
    let Post { post_type_id, view_count, score, .. } = &db.post;
    let rp = owned_since(db, year_ago()).group_by(&db.post.owner_user).select(post_type_id.and(view_count.opt()).and(score)).fold([0i64; 6], |a, ((t, w), s)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0), a[5] + s]
    });
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(999)).select(Ident::<User>::new().and(&ub).and(&rp)).drive(|_, x| v.push(x));
    out(v, |&((u, _), p)| (Reverse(p[5]), Reverse(db.user.views.get(u).unwrap())), 10, |&((u, b), p)| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep"), user_col(db, u, "uviews")];
        f.extend(ints(&b));
        f.extend(ints(&p[..3]));
        f.extend([nullable(p[4], p[3]), V::I(p[5])]);
        f
    })
}

// WITH UserVoteDetails AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(v.Id) AS TotalVotes,
// SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes
// FROM Users u
// LEFT JOIN Votes v ON u.Id = v.UserId
// LEFT JOIN VoteTypes vt ON v.VoteTypeId = vt.Id
// GROUP BY u.Id, u.DisplayName
// ),
// PopularPosts AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.Score,
// p.ViewCount,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount
// FROM Posts p
// LEFT JOIN Comments c ON p.Id = c.PostId
// LEFT JOIN Votes v ON p.Id = v.PostId
// WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'
// GROUP BY p.Id, p.Title, p.Score, p.ViewCount
// HAVING COUNT(v.Id) > 0
// ORDER BY p.Score DESC, p.ViewCount DESC
// LIMIT 10
// )
// SELECT
// p.Title AS PostTitle,
// p.Score,
// p.ViewCount,
// p.CommentCount,
// uv.DisplayName AS VoterName,
// uv.TotalVotes,
// uv.UpVotes,
// uv.DownVotes
// FROM PopularPosts p
// JOIN UserVoteDetails uv ON p.PostId IN (
// SELECT PostId FROM Votes WHERE UserId = uv.UserId
// )
// ORDER BY p.Score DESC, uv.TotalVotes DESC;
fn q7169(db: &'static So) -> String {
    let pf = stats_fold(db, since(db, month_ago()), Ident::<Post>::new(), "cv", &[]);
    let base = since(db, month_ago()).with((&pf).filt(|s: Stats| s.vx > 0));
    let top: MatSet<Id<Post>> = whole(&base)
        .select(Ident::<Post>::new().and(&db.post.score).and((&db.post.view_count).opt()))
        .window(row_number, |((_, s), w)| (s, w), desc)
        .filt(|(_, n)| n <= 10)
        .map(|(((p, _), _), _)| p)
        .collect();
    let pu: MatSet<(Id<Post>, Id<User>)> = db.vote.select((&db.vote.post).and(&db.vote.user)).collect();
    let voters: HashIdx<Id<Post>, (Id<Post>, Id<User>)> = (&pu).map(|(p, _)| p).inv().collect();
    let uv = vote_named(db);
    let mut v = Vec::new();
    (&top).select(Ident::<Post>::new().and(&pf).and((&voters).select(Same::<(Id<Post>, Id<User>)>::new().map(|(_, u)| u).select(Ident::<User>::new().and(&uv))))).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((p, s), (u, a))| {
        let mut f = post_fields(db, p, &["title", "score", "views"]);
        f.extend([V::I(s.cx), user_col(db, u, "name")]);
        f.extend(ints(&a));
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
// FROM Users U
// LEFT JOIN Badges B ON U.Id = B.UserId
// GROUP BY U.Id, U.DisplayName
// ),
// PostStats AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS PostCount,
// SUM(P.ViewCount) AS TotalViews,
// AVG(P.Score) AS AvgScore,
// SUM(CASE WHEN P.PostTypeId = 1 THEN P.AnswerCount ELSE 0 END) AS TotalAnswers
// FROM Posts P
// GROUP BY P.OwnerUserId
// ),
// CombinedStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COALESCE(UB.BadgeCount, 0) AS TotalBadges,
// COALESCE(PS.PostCount, 0) AS TotalPosts,
// COALESCE(PS.TotalViews, 0) AS TotalViews,
// COALESCE(PS.AvgScore, 0) AS AverageScore,
// COALESCE(PS.TotalAnswers, 0) AS TotalAnswers,
// (COALESCE(UB.BadgeCount, 0) + COALESCE(PS.PostCount, 0)) AS CombinedScore
// FROM Users U
// LEFT JOIN UserBadges UB ON U.Id = UB.UserId
// LEFT JOIN PostStats PS ON U.Id = PS.OwnerUserId
// )
// SELECT
// UserId,
// DisplayName,
// TotalBadges,
// TotalPosts,
// TotalViews,
// AverageScore,
// TotalAnswers,
// CombinedScore
// FROM
// CombinedStats
// ORDER BY
// CombinedScore DESC,
// TotalViews DESC,
// AverageScore DESC
// LIMIT 10;
fn q7174(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let Post { post_type_id, view_count, score, answer_count, .. } = &db.post;
    let ps = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(view_count.opt()).and(score).and(answer_count.opt())).fold([0i64; 4], |a, (((t, w), s), an)| {
        [a[0] + 1, a[1] + w.unwrap_or(0), a[2] + s, a[3] + if t == 1 { an.unwrap_or(0) } else { 0 }]
    });
    let avg0 = |p: Option<[i64; 4]>| p.map_or(0.0, |p| p[2] as f64 / p[0] as f64);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and(&bu).and((&ps).opt())).drive(|_, x| v.push(x));
    out(v, |&((_, b), p)| (Reverse(b + p.map_or(0, |p| p[0])), Reverse(p.map_or(0, |p| p[1])), Reverse(fkey(avg0(p)))), 10, |&((u, b), p)| {
        let q = p.unwrap_or([0; 4]);
        vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(b), V::I(q[0]), V::I(q[1]), V::F(avg0(p)), V::I(q[3]), V::I(b + q[0])]
    })
}

// WITH PostStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS QuestionCount,
// COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS AnswerCount,
// SUM(p.ViewCount) AS TotalViews,
// AVG(p.Score) AS AverageScore
// FROM
// Posts p
// WHERE
// p.CreationDate >= '2022-01-01'
// GROUP BY
// p.OwnerUserId
// ),
// UserReputation AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation
// FROM
// Users u
// JOIN
// PostStats ps ON u.Id = ps.OwnerUserId
// )
// SELECT
// ur.DisplayName,
// ur.Reputation,
// ps.QuestionCount,
// ps.AnswerCount,
// ps.TotalViews,
// ps.AverageScore,
// CASE
// WHEN ur.Reputation < 100 THEN 'Newbie'
// WHEN ur.Reputation BETWEEN 100 AND 1000 THEN 'Intermediate'
// ELSE 'Expert'
// END AS UserTier
// FROM
// UserReputation ur
// JOIN
// PostStats ps ON ur.UserId = ps.OwnerUserId
// ORDER BY
// ps.TotalViews DESC,
// ps.AverageScore DESC;
fn q7184(db: &'static So) -> String {
    let Post { post_type_id, view_count, score, .. } = &db.post;
    let ps = owned_since(db, date(2022, 1, 1)).group_by(&db.post.owner_user).select(post_type_id.and(view_count.opt()).and(score)).fold([0i64; 6], |a, ((t, w), s)| {
        [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + 1, a[5] + s]
    });
    let mut v = Vec::new();
    (&ps).drive(|u, a| v.push((u, a)));
    rows(v.iter().map(|&(u, a)| {
        let r = db.user.reputation.get(u).unwrap();
        row(vec![
            user_col(db, u, "name"),
            V::I(r),
            V::I(a[0]),
            V::I(a[1]),
            nullable(a[3], a[2]),
            avg(a[5], a[4]),
            V::S(if r < 100 {
                "Newbie"
            } else if (100..=1000).contains(&r) {
                "Intermediate"
            } else {
                "Expert"
            }),
        ])
    }))
}

// WITH UserVoteStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// COUNT(DISTINCT P.Id) AS TotalPosts
// FROM
// Users U
// LEFT JOIN
// Votes V ON U.Id = V.UserId
// LEFT JOIN
// Posts P ON V.PostId = P.Id
// GROUP BY
// U.Id, U.DisplayName
// ),
// PostStatistics AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS TotalPosts,
// AVG(P.Score) AS AverageScore,
// SUM(CASE WHEN PH.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS TimesClosed,
// SUM(CASE WHEN PH.PostHistoryTypeId = 11 THEN 1 ELSE 0 END) AS TimesReopened,
// SUM(CASE WHEN PH.PostHistoryTypeId IN (52, 53) THEN 1 ELSE 0 END) AS HotQuestionChanges
// FROM
// Posts P
// LEFT JOIN
// PostHistory PH ON P.Id = PH.PostId
// GROUP BY
// P.OwnerUserId
// ),
// CombinedStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// UVS.UpVotes,
// UVS.DownVotes,
// PS.TotalPosts AS UserTotalPosts,
// PS.AverageScore,
// PS.TimesClosed,
// PS.TimesReopened,
// PS.HotQuestionChanges
// FROM
// UserVoteStats UVS
// JOIN
// PostStatistics PS ON UVS.UserId = PS.OwnerUserId
// JOIN
// Users U ON U.Id = PS.OwnerUserId
// )
// SELECT
// UserId,
// DisplayName,
// Reputation,
// UpVotes,
// DownVotes,
// UserTotalPosts,
// AverageScore,
// TimesClosed,
// TimesReopened,
// HotQuestionChanges
// FROM
// CombinedStats
// WHERE
// Reputation > 1000
// ORDER BY
// UpVotes DESC, AverageScore DESC
// LIMIT 50;
fn q7320(db: &'static So) -> String {
    let uvs = g(db).select(votes_by(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 2], |a, t| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let ps = owned(db).group_by(&db.post.owner_user).select((&db.post.score).and(history_of(db).select(&db.post_history.post_history_type_id).opt())).fold([0i64; 5], |a, (s, h)| {
        [a[0] + 1, a[1] + s, a[2] + (h == Some(10)) as i64, a[3] + (h == Some(11)) as i64, a[4] + matches!(h, Some(52 | 53)) as i64]
    });
    let avgs = |p: [i64; 5]| p[1] as f64 / p[0] as f64;
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and(&uvs).and(&ps)).drive(|_, x| v.push(x));
    out(v, |&((_, a), p)| (Reverse(a[0]), Reverse(fkey(avgs(p)))), 50, |&((u, a), p)| {
        vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep"), V::I(a[0]), V::I(a[1]), V::I(p[0]), V::F(avgs(p)), V::I(p[2]), V::I(p[3]), V::I(p[4])]
    })
}

// WITH UserBadgeCounts AS (
// SELECT
// u.Id AS UserId,
// COUNT(b.Id) AS BadgeCount
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// u.Id
// ),
// PostStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS PostCount,
// SUM(p.Score) AS TotalScore,
// AVG(p.ViewCount) AS AverageViews
// FROM
// Posts p
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY
// p.OwnerUserId
// ),
// ActiveUsers AS (
// SELECT
// Users.Id,
// Users.DisplayName,
// COALESCE(ub.BadgeCount, 0) AS BadgeCount,
// COALESCE(ps.PostCount, 0) AS PostCount,
// COALESCE(ps.TotalScore, 0) AS TotalScore,
// COALESCE(ps.AverageViews, 0) AS AverageViews
// FROM
// Users
// LEFT JOIN
// UserBadgeCounts ub ON Users.Id = ub.UserId
// LEFT JOIN
// PostStats ps ON Users.Id = ps.OwnerUserId
// )
// SELECT
// au.Id,
// au.DisplayName,
// au.BadgeCount,
// au.PostCount,
// au.TotalScore,
// au.AverageViews
// FROM
// ActiveUsers au
// WHERE
// au.BadgeCount > 0
// AND au.PostCount > 3
// ORDER BY
// au.TotalScore DESC,
// au.AverageViews DESC
// LIMIT 10;
fn q7365(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let ps = owned_since(db, year_ago()).group_by(&db.post.owner_user).select((&db.post.score).and((&db.post.view_count).opt())).fold([0i64; 4], |a, (s, w)| {
        [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]
    });
    let av = |p: [i64; 4]| if p[2] > 0 { p[3] as f64 / p[2] as f64 } else { 0.0 };
    let mut v = Vec::new();
    db.user
        .select(Ident::<User>::new().and((&bu).filt(|b: i64| b > 0)).and((&ps).filt(|p: [i64; 4]| p[0] > 3)))
        .drive(|_, x| v.push(x));
    out(v, |&(_, p)| (Reverse(p[1]), Reverse(fkey(av(p)))), 10, |&((u, b), p)| {
        vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(b), V::I(p[0]), V::I(p[1]), V::F(av(p))]
    })
}

pub static ENTRIES: &[harness::Entry] = &[
    ("5421", q5421),
    ("5569", q5569),
    ("5572", q5572),
    ("5620", q5620),
    ("5686", q5686),
    ("5698", q5698),
    ("5719", q5719),
    ("5766", q5766),
    ("5817", q5817),
    ("5840", q5840),
    ("5843", q5843),
    ("5876", q5876),
    ("5931", q5931),
    ("6002", q6002),
    ("6055", q6055),
    ("6123", q6123),
    ("6125", q6125),
    ("6131", q6131),
    ("6154", q6154),
    ("6165", q6165),
    ("6166", q6166),
    ("6203", q6203),
    ("6259", q6259),
    ("6270", q6270),
    ("6349", q6349),
    ("6388", q6388),
    ("6394", q6394),
    ("6397", q6397),
    ("6425", q6425),
    ("6469", q6469),
    ("6470", q6470),
    ("6537", q6537),
    ("6655", q6655),
    ("6696", q6696),
    ("6698", q6698),
    ("6759", q6759),
    ("6787", q6787),
    ("6788", q6788),
    ("6804", q6804),
    ("6878", q6878),
    ("6889", q6889),
    ("6892", q6892),
    ("6930", q6930),
    ("6952", q6952),
    ("7000", q7000),
    ("7061", q7061),
    ("7074", q7074),
    ("7139", q7139),
    ("7141", q7141),
    ("7145", q7145),
    ("7169", q7169),
    ("7174", q7174),
    ("7184", q7184),
    ("7320", q7320),
    ("7365", q7365),
];
