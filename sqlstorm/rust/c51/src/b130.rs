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

const Z: [i64; 13] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, i64::MIN, 0, 0];

fn pstat<Q: Drive<D = Id<Post>, R = Id<Post>>>(db: &'static So, base: Q) -> Fold<Id<User>, [i64; 13]> {
    let Post { post_type_id, score, view_count, answer_count, creation_date, closed_date, .. } = &db.post;
    base.with(&db.post.owner_user)
        .group_by(&db.post.owner_user)
        .select(post_type_id.and(score).and(view_count.opt()).and(answer_count.opt()).and(creation_date).and(closed_date.opt()))
        .fold(Z, |a: [i64; 13], (((((t, s), w), an), c), cl)| {
            [
                a[0] + 1,
                a[1] + (t == 1) as i64,
                a[2] + (t == 2) as i64,
                a[3] + s,
                a[4] + w.is_some() as i64,
                a[5] + w.unwrap_or(0),
                a[6] + an.is_some() as i64,
                a[7] + an.unwrap_or(0),
                a[8] + (t == 3) as i64,
                a[9] + matches!(t, 4 | 5) as i64,
                a[10].max(c),
                a[11] + cl.is_some() as i64,
                a[12] + (t == 4) as i64,
            ]
        })
}

fn pviews(p: [i64; 13]) -> V {
    nullable(p[5], p[4])
}

fn pscore_avg(p: [i64; 13]) -> V {
    avg(p[3], p[0])
}

fn pviews_avg(p: [i64; 13]) -> V {
    avg(p[5], p[4])
}

fn onull(p: Option<[i64; 13]>, f: impl Fn([i64; 13]) -> V) -> V {
    p.map_or(V::Null, f)
}

// --- batch 130 --------------------------------------------------------------

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
// PostMetrics AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS PostCount,
// SUM(p.Score) AS TotalScore,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN p.ViewCount > 100 THEN 1 ELSE 0 END) AS PopularPostCount
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// ),
// TopUsers AS (
// SELECT
// u.Id,
// u.DisplayName,
// COALESCE(pc.PostCount, 0) AS PostCount,
// COALESCE(pc.TotalScore, 0) AS TotalScore,
// COALESCE(uc.BadgeCount, 0) AS BadgeCount
// FROM
// Users u
// LEFT JOIN
// PostMetrics pc ON u.Id = pc.OwnerUserId
// LEFT JOIN
// UserBadgeCounts uc ON u.Id = uc.UserId
// WHERE
// u.Reputation > 1000
// )
// SELECT
// tu.DisplayName,
// tu.PostCount,
// tu.TotalScore,
// tu.BadgeCount,
// CASE
// WHEN tu.PostCount > 50 THEN 'High Contributor'
// WHEN tu.PostCount BETWEEN 20 AND 50 THEN 'Moderate Contributor'
// ELSE 'New Contributor'
// END AS ContributorLevel
// FROM
// TopUsers tu
// ORDER BY
// tu.TotalScore DESC,
// tu.PostCount DESC
// LIMIT 10;
fn q7385(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and((&ps).opt()).and(&bu)).drive(|_, ((u, p), b)| v.push((u, p.unwrap_or(Z), b)));
    out(v, |&(_, p, _)| (Reverse(p[3]), Reverse(p[0])), 10, |&(u, p, b)| {
        vec![
            user_col(db, u, "name"),
            V::I(p[0]),
            V::I(p[3]),
            V::I(b),
            V::S(if p[0] > 50 {
                "High Contributor"
            } else if (20..=50).contains(&p[0]) {
                "Moderate Contributor"
            } else {
                "New Contributor"
            }),
        ]
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
// SUM(p.ViewCount) AS TotalViews,
// SUM(p.Score) AS TotalScore
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
// COALESCE(ps.TotalScore, 0) AS TotalScore,
// ub.BadgeCount,
// ub.GoldBadges,
// ub.SilverBadges,
// ub.BronzeBadges
// FROM UserBadges ub
// LEFT JOIN PostStats ps ON ub.UserId = ps.OwnerUserId
// )
// SELECT
// DisplayName,
// TotalPosts,
// Questions,
// Answers,
// TotalViews,
// TotalScore,
// BadgeCount,
// GoldBadges,
// SilverBadges,
// BronzeBadges
// FROM CombinedStats
// WHERE TotalPosts > 10
// ORDER BY TotalScore DESC, TotalViews DESC
// LIMIT 20;
fn q7389(db: &'static So) -> String {
    let ub = ubc(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    (&ub).and((&ps).filt(|p: [i64; 13]| p[0] > 10)).drive(|u, (b, p)| v.push((u, b, p)));
    out(v, |&(_, _, p)| (Reverse(p[3]), Reverse(p[5])), 20, |&(u, b, p)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(ints(&[p[0], p[1], p[2], p[5], p[3]]));
        f.extend(ints(&b));
        f
    })
}

// WITH UserBadgeCounts AS (
// SELECT UserId, COUNT(*) AS BadgeCount
// FROM Badges
// GROUP BY UserId
// ),
// TopUsers AS (
// SELECT U.Id, U.DisplayName, U.Reputation, U.CreationDate, U.LastAccessDate, COALESCE(UB.BadgeCount, 0) AS BadgeCount
// FROM Users U
// LEFT JOIN UserBadgeCounts UB ON U.Id = UB.UserId
// WHERE U.Reputation > 1000
// ORDER BY U.Reputation DESC
// LIMIT 10
// ),
// PostStatistics AS (
// SELECT P.OwnerUserId, COUNT(*) AS PostCount, SUM(P.Score) AS TotalScore, AVG(P.ViewCount) AS AvgViewCount
// FROM Posts P
// GROUP BY P.OwnerUserId
// ),
// PostUserStats AS (
// SELECT T.Id AS UserId, T.DisplayName, PS.PostCount, PS.TotalScore, PS.AvgViewCount, T.BadgeCount
// FROM TopUsers T
// LEFT JOIN PostStatistics PS ON T.Id = PS.OwnerUserId
// )
// SELECT P.Title, P.CreationDate, PS.DisplayName, PS.BadgeCount, PS.TotalScore, PS.AvgViewCount
// FROM Posts P
// JOIN PostUserStats PS ON P.OwnerUserId = PS.UserId
// WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND P.Score > 0
// ORDER BY PS.TotalScore DESC, P.CreationDate DESC
// LIMIT 20;
fn q7447(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let base = user_base(db, UserWhere::RepGt(1000));
    let top: MatSet<Id<User>> = whole(&base).select(Ident::<User>::new().and(&db.user.reputation)).window(row_number, |(_, r)| r, desc).filt(|(_, n)| n <= 10).map(|((u, _), _)| u).collect();
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    (&top)
        .select(Ident::<User>::new().and(&bu).and((&ps).opt()).and(posts_of(db).select(Ident::<Post>::new().with((&db.post.creation_date).ge(year_ago())).with((&db.post.score).gt(0)))))
        .drive(|_, x| v.push(x));
    out(v, |&(((_, _), p), q)| ((p.is_none(), Reverse(p.map(|p| p[3]))), newest(db, q)), 20, |&(((u, b), p), q)| {
        let mut f = post_fields(db, q, &["title", "created"]);
        f.extend([user_col(db, u, "name"), V::I(b), oint(p.map(|p| p[3])), onull(p, pviews_avg)]);
        f
    })
}

// WITH UserBadges AS (
// SELECT UserId, COUNT(*) AS BadgeCount, SUM(CASE WHEN Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM Badges
// GROUP BY UserId
// ),
// PostStatistics AS (
// SELECT
// OwnerUserId,
// COUNT(*) AS TotalPosts,
// SUM(CASE WHEN PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(ViewCount) AS TotalViews,
// SUM(Score) AS TotalScore
// FROM Posts
// GROUP BY OwnerUserId
// ),
// ActiveUsers AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// UB.BadgeCount,
// UB.GoldBadges,
// UB.SilverBadges,
// UB.BronzeBadges,
// PS.TotalPosts,
// PS.Questions,
// PS.Answers,
// PS.TotalViews,
// PS.TotalScore
// FROM Users U
// LEFT JOIN UserBadges UB ON U.Id = UB.UserId
// LEFT JOIN PostStatistics PS ON U.Id = PS.OwnerUserId
// WHERE U.LastAccessDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// )
// SELECT
// UserId,
// DisplayName,
// Reputation,
// BadgeCount,
// GoldBadges,
// SilverBadges,
// BronzeBadges,
// TotalPosts,
// Questions,
// Answers,
// TotalViews,
// TotalScore
// FROM ActiveUsers
// ORDER BY TotalScore DESC, Reputation DESC
// LIMIT 10;
fn q7524(db: &'static So) -> String {
    let bc = badge_classes(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    db.user.with((&db.user.last_access_date).ge(year_ago())).select(Ident::<User>::new().and((&bc).opt()).and((&ps).opt())).drive(|_, x| v.push(x));
    out(v, |&((u, _), p)| ((p.is_none(), Reverse(p.map(|p| p[3]))), rep_desc(db, u)), 10, |&((u, b), p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend((0..4).map(|i| oint(b.map(|b| b[i]))));
        f.extend((0..3).map(|i| oint(p.map(|p| p[i]))));
        f.extend([onull(p, pviews), oint(p.map(|p| p[3]))]);
        f
    })
}

// WITH UserReputation AS (
// SELECT
// Id AS UserId,
// Reputation,
// CreationDate,
// LastAccessDate,
// CASE
// WHEN Reputation > 1000 THEN 'High'
// WHEN Reputation BETWEEN 500 AND 1000 THEN 'Medium'
// ELSE 'Low'
// END AS ReputationLevel
// FROM Users
// ),
// PostStatistics AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// AVG(p.Score) AS AvgScore
// FROM Posts p
// GROUP BY p.OwnerUserId
// ),
// RecentActivity AS (
// SELECT
// p.OwnerUserId,
// MAX(p.LastActivityDate) AS LastActivity,
// COUNT(c.Id) AS CommentCount
// FROM Posts p
// LEFT JOIN Comments c ON p.Id = c.PostId
// GROUP BY p.OwnerUserId
// )
// SELECT
// u.UserId,
// u.Reputation,
// u.ReputationLevel,
// COALESCE(ps.TotalPosts, 0) AS TotalPosts,
// COALESCE(ps.Questions, 0) AS Questions,
// COALESCE(ps.Answers, 0) AS Answers,
// COALESCE(ps.AvgScore, 0) AS AvgScore,
// ra.LastActivity,
// ra.CommentCount
// FROM UserReputation u
// LEFT JOIN PostStatistics ps ON u.UserId = ps.OwnerUserId
// LEFT JOIN RecentActivity ra ON u.UserId = ra.OwnerUserId
// WHERE u.CreationDate >= '2020-01-01'
// ORDER BY u.Reputation DESC, TotalPosts DESC
// LIMIT 10;
fn q757(db: &'static So) -> String {
    let ps = pstat(db, db.post.iq());
    let ra = owned(db).group_by(&db.post.owner_user).select((&db.post.last_activity_date).and(comments_of(db).opt())).fold([i64::MIN, 0], |a: [i64; 2], (la, c)| [a[0].max(la), a[1] + c.is_some() as i64]);
    let mut v = Vec::new();
    user_base(db, UserWhere::CreatedGe(date(2020, 1, 1))).select(Ident::<User>::new().and((&ps).opt()).and((&ra).opt())).drive(|_, x| v.push(x));
    out(v, |&((u, p), _)| (rep_desc(db, u), Reverse(p.map_or(0, |p| p[0]))), 10, |&((u, p), r)| {
        let rep = db.user.reputation.get(u).unwrap();
        let q = p.unwrap_or(Z);
        let mut f = vec![
            user_col(db, u, "uid"),
            V::I(rep),
            V::S(if rep > 1000 {
                "High"
            } else if (500..=1000).contains(&rep) {
                "Medium"
            } else {
                "Low"
            }),
        ];
        f.extend(ints(&q[..3]));
        f.extend([p.map_or(V::F(0.0), pscore_avg), ots(r.map(|r| r[0])), oint(r.map(|r| r[1]))]);
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
// PostStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(p.ViewCount) AS TotalViews
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// ),
// UserPostBadgeStats AS (
// SELECT
// ubs.UserId,
// ubs.DisplayName,
// ubs.BadgeCount,
// ubs.GoldBadges,
// ubs.SilverBadges,
// ubs.BronzeBadges,
// ps.PostCount,
// ps.Questions,
// ps.Answers,
// ps.TotalViews
// FROM
// UserBadgeStats ubs
// LEFT JOIN
// PostStats ps ON ubs.UserId = ps.OwnerUserId
// )
// SELECT
// UserId,
// DisplayName,
// BadgeCount,
// GoldBadges,
// SilverBadges,
// BronzeBadges,
// COALESCE(PostCount, 0) AS PostCount,
// COALESCE(Questions, 0) AS Questions,
// COALESCE(Answers, 0) AS Answers,
// COALESCE(TotalViews, 0) AS TotalViews
// FROM
// UserPostBadgeStats
// ORDER BY
// BadgeCount DESC, TotalViews DESC
// LIMIT 100;
fn q7593(db: &'static So) -> String {
    let ub = ubc(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    (&ub).and((&ps).opt()).drive(|u, (b, p)| v.push((u, b, p.unwrap_or(Z))));
    out(v, |&(_, b, p)| (Reverse(b[0]), Reverse(p[5])), 100, |&(u, b, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&b));
        f.extend(ints(&[p[0], p[1], p[2], p[5]]));
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
// ),
// PostStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS TotalPosts,
// COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS TotalQuestions,
// COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS TotalAnswers,
// SUM(p.Score) AS TotalScore,
// SUM(p.ViewCount) AS TotalViews
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// ),
// UserPerformance AS (
// SELECT
// ub.UserId,
// ub.DisplayName,
// ub.BadgeCount,
// ub.GoldBadges,
// ub.SilverBadges,
// ub.BronzeBadges,
// ps.TotalPosts,
// ps.TotalQuestions,
// ps.TotalAnswers,
// ps.TotalScore,
// ps.TotalViews
// FROM
// UserBadges ub
// JOIN
// PostStats ps ON ub.UserId = ps.OwnerUserId
// )
// SELECT
// DisplayName,
// BadgeCount,
// GoldBadges,
// SilverBadges,
// BronzeBadges,
// TotalPosts,
// TotalQuestions,
// TotalAnswers,
// TotalScore,
// TotalViews,
// (TotalScore / NULLIF(TotalPosts, 0)) AS AvgScorePerPost,
// (TotalViews / NULLIF(TotalPosts, 0)) AS AvgViewsPerPost
// FROM
// UserPerformance
// ORDER BY
// TotalPosts DESC, TotalScore DESC
// LIMIT 10;
fn q7613(db: &'static So) -> String {
    let ub = ubc(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    (&ub).and(&ps).drive(|u, (b, p)| v.push((u, b, p)));
    out(v, |&(_, _, p)| (Reverse(p[0]), Reverse(p[3])), 10, |&(u, b, p)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(ints(&b));
        f.extend(ints(&[p[0], p[1], p[2], p[3]]));
        f.extend([pviews(p), ratio(p[3], p[0]), if p[4] > 0 { ratio(p[5], p[0]) } else { V::Null }]);
        f
    })
}

// WITH UserBadgeCounts AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(b.Id) AS BadgeCount,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldCount,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverCount,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeCount
// FROM Users u
// LEFT JOIN Badges b ON u.Id = b.UserId
// GROUP BY u.Id, u.DisplayName
// ),
// PostStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS TotalPosts,
// COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS QuestionCount,
// COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS AnswerCount,
// SUM(p.Score) AS TotalScore
// FROM Posts p
// GROUP BY p.OwnerUserId
// ),
// CombinedStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COALESCE(ub.BadgeCount, 0) AS BadgeCount,
// COALESCE(ub.GoldCount, 0) AS GoldCount,
// COALESCE(ub.SilverCount, 0) AS SilverCount,
// COALESCE(ub.BronzeCount, 0) AS BronzeCount,
// COALESCE(ps.TotalPosts, 0) AS TotalPosts,
// COALESCE(ps.QuestionCount, 0) AS QuestionCount,
// COALESCE(ps.AnswerCount, 0) AS AnswerCount,
// COALESCE(ps.TotalScore, 0) AS TotalScore
// FROM Users u
// LEFT JOIN UserBadgeCounts ub ON u.Id = ub.UserId
// LEFT JOIN PostStats ps ON u.Id = ps.OwnerUserId
// )
// SELECT
// UserId,
// DisplayName,
// Reputation,
// BadgeCount,
// GoldCount,
// SilverCount,
// BronzeCount,
// TotalPosts,
// QuestionCount,
// AnswerCount,
// TotalScore
// FROM CombinedStats
// WHERE TotalPosts > 0
// ORDER BY TotalScore DESC, Reputation DESC
// LIMIT 10;
fn q7660(db: &'static So) -> String {
    let ub = ubc(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    (&ps).filt(|p: [i64; 13]| p[0] > 0).and(&ub).drive(|u, (p, b)| v.push((u, p, b)));
    out(v, |&(u, p, _)| (Reverse(p[3]), rep_desc(db, u)), 10, |&(u, p, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(ints(&b));
        f.extend(ints(&[p[0], p[1], p[2], p[3]]));
        f
    })
}

// SELECT
// u.Id AS UserId,
// u.DisplayName AS UserName,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty,
// AVG(u.Reputation) AS AverageReputation,
// COUNT(DISTINCT b.Id) AS TotalBadges,
// MAX(p.CreationDate) AS LastPostDate
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Votes v ON p.Id = v.PostId AND v.VoteTypeId = 8
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// WHERE
// u.CreationDate < cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY
// u.Id, u.DisplayName
// ORDER BY
// TotalPosts DESC, AverageReputation DESC
// LIMIT 10;
fn q7688(db: &'static So) -> String {
    let w = UserWhere::CreatedLt(year_ago());
    let us = user_stats_fold_v(db, Ident::<User>::new(), w, "vb", any_post, &[8]);
    let dp = ud(db, w, posts_of(db));
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    (&us).and((&dp).opt()).and(&bu).drive(|u, ((a, p), b)| v.push((u, a, p.unwrap_or(0), b)));
    out(v, |&(u, _, p, _)| (Reverse(p), rep_desc(db, u)), 10, |&(u, a, p, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(p)];
        f.extend(["#q", "#a", "bounty_sum0"].iter().map(|c| ustat_field(&a, c)));
        f.extend([V::F(db.user.reputation.get(u).unwrap() as f64), V::I(b), ustat_field(&a, "created_max")]);
        f
    })
}

// WITH UserBadges AS (
// SELECT UserId, COUNT(*) AS TotalBadges
// FROM Badges
// GROUP BY UserId
// ),
// PopularPosts AS (
// SELECT p.Id, p.Title, p.Score, COUNT(c.Id) AS CommentCount
// FROM Posts p
// LEFT JOIN Comments c ON p.Id = c.PostId
// WHERE p.PostTypeId = 1 AND p.Score >= 10
// GROUP BY p.Id, p.Title, p.Score
// ),
// PostHistorySummary AS (
// SELECT ph.PostId,
// ph.PostHistoryTypeId,
// COUNT(*) AS HistoryCount
// FROM PostHistory ph
// WHERE ph.CreationDate >= DATE '2024-10-01' - INTERVAL '1 year'
// GROUP BY ph.PostId, ph.PostHistoryTypeId
// ),
// TopUsers AS (
// SELECT u.Id, u.DisplayName, u.Reputation, ub.TotalBadges
// FROM Users u
// JOIN UserBadges ub ON u.Id = ub.UserId
// ORDER BY u.Reputation DESC, ub.TotalBadges DESC
// LIMIT 10
// )
// SELECT pu.DisplayName AS TopUser,
// pp.Title AS PopularPostTitle,
// pp.Score AS PopularPostScore,
// phs.PostHistoryTypeId AS HistoryType,
// phs.HistoryCount AS HistoryTypeCount
// FROM TopUsers pu
// JOIN PopularPosts pp ON pp.CommentCount > 5
// JOIN PostHistorySummary phs ON pp.Id = phs.PostId
// WHERE pu.Id IN (SELECT OwnerUserId FROM Posts WHERE Id = pp.Id)
// ORDER BY pu.Reputation DESC, pp.Score DESC;
fn q7717(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let base = db.user.with((&bu).filt(|b: i64| b > 0));
    let top: MatSet<Id<User>> = whole(&base)
        .select(Ident::<User>::new().and(&db.user.reputation).and(&bu))
        .window(row_number, |((_, r), b)| (r, b), desc)
        .filt(|(_, n)| n <= 10)
        .map(|(((u, _), _), _)| u)
        .collect();
    let PostHistory { post, post_history_type_id, creation_date, .. } = &db.post_history;
    let hb = || db.post_history.with(creation_date.ge(date(2023, 10, 1)));
    let hc = hb().group_by(post.and(post_history_type_id)).fold(0i64, |a, _| a + 1);
    let keys: MatSet<(Id<Post>, i64)> = hb().select(post.and(post_history_type_id)).collect();
    let by_post: HashIdx<Id<Post>, (Id<Post>, i64)> = (&keys).map(|(p, _)| p).inv().collect();
    let pp = Ident::<Post>::new()
        .with((&db.post.post_type_id).eq(1))
        .with((&db.post.score).ge(10))
        .with(comments_per_post(db).filt(|c: i64| c > 5))
        .and((&by_post).select(Same::<(Id<Post>, i64)>::new().and(&hc)));
    let mut v = Vec::new();
    (&top).select(Ident::<User>::new().and(posts_of(db).select(pp))).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(u, (p, ((_, t), n)))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title", "score"]));
        f.extend([V::I(t), V::I(n)]);
        row(f)
    }))
}

// WITH UserBadges AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(b.Id) AS BadgeCount
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// u.Id, u.DisplayName
// ),
// RecentPosts AS (
// SELECT
// p.Id AS PostId,
// p.OwnerUserId,
// p.Title,
// p.CreationDate,
// p.Score,
// COUNT(c.Id) AS CommentCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'
// GROUP BY
// p.Id, p.OwnerUserId, p.Title, p.CreationDate, p.Score
// ),
// TopUsers AS (
// SELECT
// ub.UserId,
// ub.DisplayName,
// ub.BadgeCount,
// COUNT(rp.PostId) AS RecentPostCount
// FROM
// UserBadges ub
// JOIN
// RecentPosts rp ON ub.UserId = rp.OwnerUserId
// GROUP BY
// ub.UserId, ub.DisplayName, ub.BadgeCount
// ORDER BY
// ub.BadgeCount DESC, RecentPostCount DESC
// )
// SELECT
// tu.DisplayName,
// tu.BadgeCount,
// tu.RecentPostCount
// FROM
// TopUsers tu
// WHERE
// tu.BadgeCount > 0
// LIMIT 10;
fn q7827(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let rp = owned_since(db, month_ago()).group_by(&db.post.owner_user).fold(0i64, |a, _| a + 1);
    let mut v = Vec::new();
    (&rp).and((&bu).filt(|b: i64| b > 0)).drive(|u, (n, b)| v.push((u, n, b)));
    out(v, |&(_, n, b)| (Reverse(b), Reverse(n)), 10, |&(u, n, b)| vec![user_col(db, u, "name"), V::I(b), V::I(n)])
}

// SELECT
// U.DisplayName AS UserName,
// U.Reputation,
// COUNT(DISTINCT P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN P.ClosedDate IS NOT NULL THEN 1 ELSE 0 END) AS TotalClosedPosts,
// AVG(COALESCE(P.ViewCount, 0)) AS AverageViews,
// AVG(COALESCE(P.Score, 0)) AS AverageScore
// FROM
// Users U
// JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Votes V ON P.Id = V.PostId AND V.VoteTypeId IN (2, 3)
// WHERE
// U.Reputation > (SELECT AVG(Reputation) FROM Users)
// GROUP BY
// U.Id, U.DisplayName, U.Reputation
// HAVING
// COUNT(DISTINCT P.Id) > 10
// ORDER BY
// TotalPosts DESC, U.Reputation DESC
// LIMIT 50;
fn q7836(db: &'static So) -> String {
    let u = db.user.select(&db.user.reputation).fold_flat([0i64; 2], |a, r| [a[0] + 1, a[1] + r]);
    let avgr = u[1] as f64 / u[0] as f64;
    let v23 = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).in_v(vec![2, 3])));
    let Post { post_type_id, closed_date, view_count, score, .. } = &db.post;
    let uf = db
        .user
        .with((&db.user.reputation).filt(move |r: i64| r as f64 > avgr))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(closed_date.opt()).and(view_count.opt()).and(score).and(v23.opt())))
        .fold([0i64; 6], |a, ((((t, c), w), s), _)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + c.is_some() as i64, a[4] + w.unwrap_or(0), a[5] + s]);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let mut v = Vec::new();
    (&uf).and((&dp).filt(|d: i64| d > 10)).drive(|u, (a, d)| v.push((u, a, d)));
    out(v, |&(u, _, d)| (Reverse(d), rep_desc(db, u)), 50, |&(u, a, d)| {
        vec![user_col(db, u, "name"), user_col(db, u, "rep"), V::I(d), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::F(a[4] as f64 / a[0] as f64), V::F(a[5] as f64 / a[0] as f64)]
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
// ),
// PostStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS TotalPosts,
// COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS Questions,
// COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS Answers,
// SUM(p.Score) AS TotalScore,
// SUM(p.ViewCount) AS TotalViews,
// MAX(p.CreationDate) AS LastPostDate
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// )
// SELECT
// ub.UserId,
// ub.DisplayName,
// ub.BadgeCount AS TotalBadges,
// ub.GoldBadges,
// ub.SilverBadges,
// ub.BronzeBadges,
// ps.TotalPosts,
// ps.Questions,
// ps.Answers,
// ps.TotalScore,
// ps.TotalViews,
// ps.LastPostDate
// FROM
// UserBadges ub
// LEFT JOIN
// PostStats ps ON ub.UserId = ps.OwnerUserId
// WHERE
// ub.BadgeCount > 0
// ORDER BY
// ub.BadgeCount DESC,
// ps.TotalScore DESC
// LIMIT 100;
fn q7877(db: &'static So) -> String {
    let bc = badge_classes(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    (&bc).and((&ps).opt()).drive(|u, (b, p)| v.push((u, b, p)));
    out(v, |&(_, b, p)| (Reverse(b[0]), (p.is_none(), Reverse(p.map(|p| p[3])))), 100, |&(u, b, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&b));
        f.extend([0, 1, 2, 3].iter().map(|&i| oint(p.map(|p| p[i]))));
        f.extend([onull(p, pviews), ots(p.map(|p| p[10]))]);
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
// ),
// PostStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(p.ViewCount) AS TotalViews,
// SUM(p.Score) AS TotalScore
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
// ps.TotalPosts,
// ps.Questions,
// ps.Answers,
// ps.TotalViews,
// ps.TotalScore,
// (CASE
// WHEN ps.TotalViews > 0 THEN (CAST(ps.TotalScore AS double precision) / ps.TotalViews) * 100
// ELSE 0
// END) AS EngagementRate
// FROM
// UserBadges ub
// LEFT JOIN
// PostStats ps ON ub.UserId = ps.OwnerUserId
// ORDER BY
// EngagementRate DESC, ub.BadgeCount DESC;
fn q7902(db: &'static So) -> String {
    let ub = ubc(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    (&ub).and((&ps).opt()).drive(|u, (b, p)| v.push((u, b, p)));
    rows(v.iter().map(|&(u, b, p)| {
        let e = match p {
            Some(p) if p[4] > 0 && p[5] > 0 => p[3] as f64 / p[5] as f64 * 100.0,
            _ => 0.0,
        };
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&b));
        f.extend((0..3).map(|i| oint(p.map(|p| p[i]))));
        f.extend([onull(p, pviews), oint(p.map(|p| p[3])), V::F(e)]);
        row(f)
    }))
}

// WITH UserVoteStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// COUNT(V.Id) AS TotalVotes
// FROM
// Users U
// LEFT JOIN
// Votes V ON U.Id = V.UserId
// GROUP BY
// U.Id, U.DisplayName
// ),
// PostAnswerSummary AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS AnswerCount,
// AVG(P.Score) AS AverageScore,
// SUM(P.ViewCount) AS TotalViews
// FROM
// Posts P
// WHERE
// P.PostTypeId = 2
// GROUP BY
// P.OwnerUserId
// ),
// TopUsers AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COALESCE(PA.AnswerCount, 0) AS AnswerCount,
// COALESCE(PA.AverageScore, 0) AS AverageScore,
// COALESCE(PA.TotalViews, 0) AS TotalViews,
// COALESCE(UV.UpVotes, 0) AS UpVotes,
// COALESCE(UV.DownVotes, 0) AS DownVotes
// FROM
// Users U
// LEFT JOIN
// PostAnswerSummary PA ON U.Id = PA.OwnerUserId
// LEFT JOIN
// UserVoteStats UV ON U.Id = UV.UserId
// ORDER BY
// U.Reputation DESC
// LIMIT 10
// )
// SELECT
// TU.UserId,
// TU.DisplayName,
// TU.Reputation,
// TU.AnswerCount,
// TU.AverageScore,
// TU.TotalViews,
// TU.UpVotes,
// TU.DownVotes
// FROM
// TopUsers TU;
fn q7903(db: &'static So) -> String {
    let top: MatSet<Id<User>> = whole(&db.user.id).select(Ident::<User>::new().and(&db.user.reputation)).window(row_number, |(_, r)| r, desc).filt(|(_, n)| n <= 10).map(|((u, _), _)| u).collect();
    let pa = pstat(db, db.post.with((&db.post.post_type_id).eq(2)));
    let uv = db.vote.group_by(&db.vote.user).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let mut v = Vec::new();
    (&top).select(Ident::<User>::new().and((&pa).opt()).and((&uv).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((u, p), x)| {
        let q = p.unwrap_or(Z);
        let x = x.unwrap_or([0; 2]);
        row(vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep"), V::I(q[0]), p.map_or(V::F(0.0), pscore_avg), V::I(q[5]), V::I(x[0]), V::I(x[1])])
    }))
}

// WITH UserPostStatistics AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
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
// UserBadges AS (
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
// ),
// PostCloseStatistics AS (
// SELECT
// PH.UserId,
// COUNT(PH.Id) AS TotalPostClosures,
// MIN(PH.CreationDate) AS FirstCloseDate,
// MAX(PH.CreationDate) AS LastCloseDate
// FROM
// PostHistory PH
// WHERE
// PH.PostHistoryTypeId = 10
// GROUP BY
// PH.UserId
// )
// SELECT
// UPS.UserId,
// UPS.DisplayName,
// UPS.TotalPosts,
// UPS.TotalQuestions,
// UPS.TotalAnswers,
// UPS.TotalUpvotes,
// UPS.TotalDownvotes,
// UB.TotalBadges,
// UB.GoldBadges,
// UB.SilverBadges,
// UB.BronzeBadges,
// PCS.TotalPostClosures,
// PCS.FirstCloseDate,
// PCS.LastCloseDate
// FROM
// UserPostStatistics UPS
// LEFT JOIN
// UserBadges UB ON UPS.UserId = UB.UserId
// LEFT JOIN
// PostCloseStatistics PCS ON UPS.UserId = PCS.UserId
// ORDER BY
// UPS.TotalPosts DESC, UPS.TotalUpvotes DESC
// FETCH FIRST 100 ROWS ONLY;
fn q7939(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let bc = badge_classes(db);
    let cs = db
        .post_history
        .with((&db.post_history.post_history_type_id).eq(10))
        .group_by(&db.post_history.user)
        .select(&db.post_history.creation_date)
        .fold([0, i64::MAX, i64::MIN], |a: [i64; 3], d| [a[0] + 1, a[1].min(d), a[2].max(d)]);
    let mut v = Vec::new();
    (&us).and((&bc).opt()).and((&cs).opt()).drive(|u, ((a, b), c)| v.push((u, a, b, c)));
    out(v, |&(_, a, _, _)| (Reverse(a.n), Reverse(a.up)), 100, |&(u, a, b, c)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&[a.n, a.q, a.a, a.up, a.down]));
        f.extend((0..4).map(|i| oint(b.map(|b| b[i]))));
        f.extend([oint(c.map(|c| c[0])), ots(c.map(|c| c[1])), ots(c.map(|c| c[2]))]);
        f
    })
}

// WITH UserPosts AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN p.PostTypeId IN (4, 5) THEN 1 ELSE 0 END) AS TagWikiCount,
// MAX(p.CreationDate) AS LastPostDate
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// WHERE
// u.Reputation > 1000
// GROUP BY
// u.Id, u.DisplayName
// ),
// VotesSummary AS (
// SELECT
// p.OwnerUserId,
// COUNT(v.Id) AS TotalVotes,
// SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Posts p
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// VoteTypes vt ON v.VoteTypeId = vt.Id
// GROUP BY
// p.OwnerUserId
// ),
// PostHistorySummary AS (
// SELECT
// ph.UserId,
// COUNT(ph.Id) AS TotalEdits,
// SUM(CASE WHEN pht.Name LIKE 'Edit%' THEN 1 ELSE 0 END) AS EditCount,
// SUM(CASE WHEN pht.Name LIKE 'Rollback%' THEN 1 ELSE 0 END) AS RollbackCount
// FROM
// PostHistory ph
// JOIN
// PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id
// GROUP BY
// ph.UserId
// )
// SELECT
// up.UserId,
// up.DisplayName,
// up.TotalPosts,
// up.QuestionCount,
// up.AnswerCount,
// up.TagWikiCount,
// up.LastPostDate,
// vs.TotalVotes,
// vs.UpVotes,
// vs.DownVotes,
// phs.TotalEdits,
// phs.EditCount,
// phs.RollbackCount
// FROM
// UserPosts up
// LEFT JOIN
// VotesSummary vs ON up.UserId = vs.OwnerUserId
// LEFT JOIN
// PostHistorySummary phs ON up.UserId = phs.UserId
// ORDER BY
// up.TotalPosts DESC, up.LastPostDate DESC
// LIMIT 100;
fn q7940(db: &'static So) -> String {
    let ps = pstat(db, db.post.iq());
    let vs = owned(db).group_by(&db.post.owner_user).select(votes_of(db).select((&db.vote.vote_type).select(&db.vote_type.name)).opt()).fold([0i64; 3], |a, n| {
        [a[0] + n.is_some() as i64, a[1] + (n == Some("UpMod")) as i64, a[2] + (n == Some("DownMod")) as i64]
    });
    let ph = db
        .post_history
        .group_by(&db.post_history.user)
        .select((&db.post_history.post_history_type).select(&db.post_history_type.name))
        .fold([0i64; 3], |a, n| [a[0] + 1, a[1] + n.starts_with("Edit") as i64, a[2] + n.starts_with("Rollback") as i64]);
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and((&ps).opt()).and((&vs).opt()).and((&ph).opt())).drive(|_, x| v.push(x));
    out(v, |&(((_, p), _), _)| (Reverse(p.map_or(0, |p| p[0])), (p.is_none(), Reverse(p.map(|p| p[10])))), 100, |&(((u, p), x), h)| {
        let q = p.unwrap_or(Z);
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&[q[0], q[1], q[2], q[9]]));
        f.push(ots(p.map(|p| p[10])));
        f.extend((0..3).map(|i| oint(x.map(|x| x[i]))));
        f.extend((0..3).map(|i| oint(h.map(|h| h[i]))));
        f
    })
}

// WITH UserBadges AS (
// SELECT UserId, COUNT(*) AS BadgeCount
// FROM Badges
// WHERE Class = 1
// GROUP BY UserId
// ),
// PostScoreStats AS (
// SELECT OwnerUserId, AVG(Score) AS AvgScore, SUM(ViewCount) AS TotalViews
// FROM Posts
// WHERE CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY OwnerUserId
// ),
// ActiveUsers AS (
// SELECT U.Id, U.DisplayName, U.Reputation, U.CreationDate, COALESCE(UB.BadgeCount, 0) AS GoldBadgeCount,
// COALESCE(PSS.AvgScore, 0) AS AvgScore, COALESCE(PSS.TotalViews, 0) AS TotalViews
// FROM Users U
// LEFT JOIN UserBadges UB ON U.Id = UB.UserId
// LEFT JOIN PostScoreStats PSS ON U.Id = PSS.OwnerUserId
// WHERE U.Reputation > 1000 AND U.LastAccessDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '6 months'
// ),
// TopPosts AS (
// SELECT P.Id, P.Title, P.OwnerUserId, P.Score
// FROM Posts P
// WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' AND P.Score > 0
// ORDER BY P.Score DESC
// LIMIT 10
// )
// SELECT AU.DisplayName, AU.Reputation, AU.GoldBadgeCount, AU.AvgScore, AU.TotalViews,
// TP.Title, TP.Score
// FROM ActiveUsers AU
// JOIN TopPosts TP ON AU.Id = TP.OwnerUserId
// ORDER BY AU.Reputation DESC, TP.Score DESC;
fn q7958(db: &'static So) -> String {
    let gold = db.badge.with((&db.badge.class).eq(1)).group_by(&db.badge.user).fold(0i64, |a, _| a + 1);
    let ps = pstat(db, since(db, year_ago()));
    let base = since(db, year_ago()).with((&db.post.score).gt(0));
    let tp: MatSet<Id<Post>> = whole(&base).select(Ident::<Post>::new().and(&db.post.score)).window(row_number, |(_, s)| s, desc).filt(|(_, n)| n <= 10).map(|((p, _), _)| p).collect();
    let active = Ident::<User>::new().with((&db.user.reputation).gt(1000)).with((&db.user.last_access_date).ge(ts(2024, 4, 1, 12, 34, 56))).and((&gold).opt()).and((&ps).opt());
    let mut v = Vec::new();
    (&tp).select(Ident::<Post>::new().and((&db.post.owner_user).select(active))).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(p, ((u, g), s))| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep"), V::I(g.unwrap_or(0)), s.map_or(V::F(0.0), pscore_avg), V::I(s.map_or(0, |s| s[5]))];
        f.extend(post_fields(db, p, &["title", "score"]));
        row(f)
    }))
}

// WITH UserBadges AS (
// SELECT
// u.Id AS UserId,
// COUNT(b.Id) AS TotalBadges,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
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
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(CASE WHEN p.ClosedDate IS NOT NULL THEN 1 ELSE 0 END) AS ClosedPosts
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// ),
// UserPerformance AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// ub.TotalBadges,
// ub.GoldBadges,
// ub.SilverBadges,
// ub.BronzeBadges,
// ps.TotalPosts,
// ps.Questions,
// ps.Answers,
// ps.ClosedPosts
// FROM
// Users u
// JOIN
// UserBadges ub ON u.Id = ub.UserId
// LEFT JOIN
// PostStats ps ON u.Id = ps.OwnerUserId
// )
// SELECT
// UserId,
// DisplayName,
// Reputation,
// TotalBadges,
// GoldBadges,
// SilverBadges,
// BronzeBadges,
// TotalPosts,
// Questions,
// Answers,
// ClosedPosts
// FROM
// UserPerformance
// WHERE
// Reputation > 1000
// ORDER BY
// TotalPosts DESC, Reputation DESC
// LIMIT 10;
fn q8011(db: &'static So) -> String {
    let ub = ubc(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and(&ub).and((&ps).opt())).drive(|_, x| v.push(x));
    out(v, |&((u, _), p)| ((p.is_none(), Reverse(p.map(|p| p[0]))), rep_desc(db, u)), 10, |&((u, b), p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(ints(&b));
        f.extend([0, 1, 2, 11].iter().map(|&i| oint(p.map(|p| p[i]))));
        f
    })
}

// WITH UserReputation AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(B.Id) AS BadgeCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN P.Score ELSE 0 END) AS QuestionScore,
// SUM(CASE WHEN P.PostTypeId = 2 THEN P.Score ELSE 0 END) AS AnswerScore,
// COUNT(CASE WHEN P.PostTypeId = 1 THEN P.Id END) AS QuestionCount,
// COUNT(CASE WHEN P.PostTypeId = 2 THEN P.Id END) AS AnswerCount
// FROM
// Users U
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id, U.DisplayName, U.Reputation
// ),
// UserVotes AS (
// SELECT
// V.UserId,
// COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVotesCount,
// COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVotesCount,
// COUNT(CASE WHEN V.VoteTypeId = 10 THEN 1 END) AS DeleteVotesCount
// FROM
// Votes V
// GROUP BY
// V.UserId
// ),
// PostStatistics AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS PostCount,
// SUM(P.ViewCount) AS TotalViews
// FROM
// Posts P
// GROUP BY
// P.OwnerUserId
// )
// SELECT
// UR.UserId,
// UR.DisplayName,
// UR.Reputation,
// UR.BadgeCount,
// UV.UpVotesCount,
// UV.DownVotesCount,
// UV.DeleteVotesCount,
// PS.PostCount,
// PS.TotalViews,
// (UR.QuestionScore + UR.AnswerScore) AS TotalScore
// FROM
// UserReputation UR
// LEFT JOIN
// UserVotes UV ON UR.UserId = UV.UserId
// LEFT JOIN
// PostStatistics PS ON UR.UserId = PS.OwnerUserId
// ORDER BY
// TotalScore DESC, UR.Reputation DESC
// LIMIT 100;
fn q8026(db: &'static So) -> String {
    let uf = g(db).select(badges_of(db).opt().and(posts_of(db).select((&db.post.post_type_id).and(&db.post.score)).opt())).fold([0i64; 5], |a, (b, p)| {
        let (t, s) = p.unwrap_or((0, 0));
        [a[0] + b.is_some() as i64, a[1] + if t == 1 { s } else { 0 }, a[2] + if t == 2 { s } else { 0 }, a[3] + (t == 1) as i64, a[4] + (t == 2) as i64]
    });
    let uv = db.vote.group_by(&db.vote.user).select(&db.vote.vote_type_id).fold([0i64; 3], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + (t == 10) as i64]);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    (&uf).and((&uv).opt()).and((&ps).opt()).drive(|u, ((a, x), p)| v.push((u, a, x, p)));
    out(v, |&(u, a, _, _)| (Reverse(a[1] + a[2]), rep_desc(db, u)), 100, |&(u, a, x, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep"), V::I(a[0])];
        f.extend((0..3).map(|i| oint(x.map(|x| x[i]))));
        f.extend([oint(p.map(|p| p[0])), onull(p, pviews), V::I(a[1] + a[2])]);
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
// SUM(p.ViewCount) AS TotalViews,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes
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
// ), AverageStats AS (
// SELECT
// AVG(TotalPosts) AS AvgTotalPosts,
// AVG(TotalQuestions) AS AvgTotalQuestions,
// AVG(TotalAnswers) AS AvgTotalAnswers,
// AVG(TotalTagWikis) AS AvgTotalTagWikis,
// AVG(TotalViews) AS AvgTotalViews,
// AVG(TotalUpvotes) AS AvgTotalUpvotes,
// AVG(TotalDownvotes) AS AvgTotalDownvotes
// FROM
// UserPostStats
// )
// SELECT
// ups.DisplayName,
// ups.TotalPosts,
// ups.TotalQuestions,
// ups.TotalAnswers,
// ups.TotalTagWikis,
// ups.TotalViews,
// ups.TotalUpvotes,
// ups.TotalDownvotes,
// (ups.TotalPosts - a.AvgTotalPosts) AS PostsDifference,
// (ups.TotalQuestions - a.AvgTotalQuestions) AS QuestionsDifference,
// (ups.TotalAnswers - a.AvgTotalAnswers) AS AnswersDifference,
// (ups.TotalTagWikis - a.AvgTotalTagWikis) AS TagWikisDifference,
// (ups.TotalViews - a.AvgTotalViews) AS ViewsDifference,
// (ups.TotalUpvotes - a.AvgTotalUpvotes) AS UpvotesDifference,
// (ups.TotalDownvotes - a.AvgTotalDownvotes) AS DownvotesDifference
// FROM
// UserPostStats ups
// CROSS JOIN
// AverageStats a
// ORDER BY
// ups.TotalPosts DESC
// LIMIT 10;
fn q8067(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::RepGt(1000), "v", any_post);
    let t = (&us).fold_flat([0i64; 9], |a, s: UStats| {
        [a[0] + 1, a[1] + s.n, a[2] + s.q, a[3] + s.a, a[4] + s.t45, a[5] + (s.views_n > 0) as i64, a[6] + s.views_sum, a[7] + s.up, a[8] + s.down]
    });
    let m = |i: usize| t[i] as f64 / t[0] as f64;
    let mv = t[6] as f64 / t[5] as f64;
    let mut v = Vec::new();
    (&us).drive(|u, a| v.push((u, a)));
    out(v, |&(_, a)| Reverse(a.n), 10, |&(u, a)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(ints(&[a.n, a.q, a.a, a.t45]));
        f.extend([ustat_field(&a, "views_sum"), V::I(a.up), V::I(a.down)]);
        f.extend([V::F(a.n as f64 - m(1)), V::F(a.q as f64 - m(2)), V::F(a.a as f64 - m(3)), V::F(a.t45 as f64 - m(4))]);
        f.push(if a.views_n > 0 { V::F(a.views_sum as f64 - mv) } else { V::Null });
        f.extend([V::F(a.up as f64 - m(7)), V::F(a.down as f64 - m(8))]);
        f
    })
}

// WITH UserBadgeCounts AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(B.Id) AS BadgeCount,
// SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadgeCount,
// SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadgeCount,
// SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadgeCount
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
// COUNT(P.Id) AS TotalPosts,
// SUM(P.AnswerCount) AS TotalAnswers,
// SUM(P.ViewCount) AS TotalViews,
// AVG(P.Score) AS AverageScore,
// MAX(P.CreationDate) AS LatestPostDate
// FROM
// Posts P
// GROUP BY
// P.OwnerUserId
// ),
// UserPerformance AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// Coalesce(UB.BadgeCount, 0) AS BadgeCount,
// Coalesce(PS.TotalPosts, 0) AS TotalPosts,
// Coalesce(PS.TotalAnswers, 0) AS TotalAnswers,
// Coalesce(PS.TotalViews, 0) AS TotalViews,
// Coalesce(PS.AverageScore, 0) AS AverageScore,
// Coalesce(PS.LatestPostDate, '1900-01-01') AS LatestPostDate
// FROM
// Users U
// LEFT JOIN
// UserBadgeCounts UB ON U.Id = UB.UserId
// LEFT JOIN
// PostStats PS ON U.Id = PS.OwnerUserId
// )
// SELECT
// UserId,
// DisplayName,
// BadgeCount,
// TotalPosts,
// TotalAnswers,
// TotalViews,
// AverageScore,
// LatestPostDate
// FROM
// UserPerformance
// WHERE
// BadgeCount > 0 OR TotalPosts > 5
// ORDER BY
// TotalViews DESC, AverageScore DESC;
fn q8075(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    db.user
        .select(Ident::<User>::new().and(&bu).and((&ps).opt()))
        .filt(|((_, b), p): ((Id<User>, i64), Option<[i64; 13]>)| b > 0 || p.map_or(0, |p| p[0]) > 5)
        .drive(|_, x| v.push(x));
    rows(v.iter().map(|&((u, b), p)| {
        let q = p.unwrap_or(Z);
        row(vec![
            user_col(db, u, "uid"),
            user_col(db, u, "name"),
            V::I(b),
            V::I(q[0]),
            V::I(q[7]),
            V::I(q[5]),
            p.map_or(V::F(0.0), pscore_avg),
            V::T(p.map_or(date(1900, 1, 1), |p| p[10])),
        ])
    }))
}

// WITH UserBadgeCounts AS (
// SELECT
// Users.Id AS UserId,
// Users.DisplayName,
// COUNT(Badges.Id) AS BadgeCount
// FROM
// Users
// LEFT JOIN
// Badges ON Users.Id = Badges.UserId
// GROUP BY
// Users.Id, Users.DisplayName
// ),
// PostStats AS (
// SELECT
// Posts.OwnerUserId,
// COUNT(Posts.Id) AS PostCount,
// SUM(CASE WHEN Posts.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN Posts.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(Posts.ViewCount) AS TotalViews,
// AVG(Posts.Score) AS AverageScore
// FROM
// Posts
// GROUP BY
// Posts.OwnerUserId
// ),
// CombinedStats AS (
// SELECT
// u.DisplayName,
// u.Reputation,
// u.LastAccessDate,
// COALESCE(pc.PostCount, 0) AS PostCount,
// COALESCE(pc.QuestionCount, 0) AS QuestionCount,
// COALESCE(pc.AnswerCount, 0) AS AnswerCount,
// COALESCE(pc.TotalViews, 0) AS TotalViews,
// COALESCE(pc.AverageScore, 0) AS AverageScore,
// COALESCE(bc.BadgeCount, 0) AS BadgeCount
// FROM
// Users u
// LEFT JOIN
// PostStats pc ON u.Id = pc.OwnerUserId
// LEFT JOIN
// UserBadgeCounts bc ON u.Id = bc.UserId
// )
// SELECT
// DisplayName,
// Reputation,
// LastAccessDate,
// PostCount,
// QuestionCount,
// AnswerCount,
// TotalViews,
// AverageScore,
// BadgeCount
// FROM
// CombinedStats
// WHERE
// BadgeCount > 0
// ORDER BY
// TotalViews DESC, Reputation DESC
// LIMIT 10;
fn q8079(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&bu).filt(|b: i64| b > 0)).and((&ps).opt())).drive(|_, x| v.push(x));
    out(v, |&((u, _), p)| (Reverse(p.map_or(0, |p| p[5])), rep_desc(db, u)), 10, |&((u, b), p)| {
        let q = p.unwrap_or(Z);
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep"), user_col(db, u, "last_access")];
        f.extend(ints(&[q[0], q[1], q[2], q[5]]));
        f.extend([p.map_or(V::F(0.0), pscore_avg), V::I(b)]);
        f
    })
}

// WITH UserVoteStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(v.Id) AS TotalVotes,
// SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Users u
// LEFT JOIN
// Votes v ON u.Id = v.UserId
// LEFT JOIN
// VoteTypes vt ON v.VoteTypeId = vt.Id
// GROUP BY
// u.Id, u.DisplayName
// ),
// PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// COUNT(c.Id) AS CommentCount,
// COUNT(DISTINCT b.Id) AS BadgeCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Badges b ON p.OwnerUserId = b.UserId
// GROUP BY
// p.Id, p.Title
// ),
// FeaturedUsers AS (
// SELECT
// u.DisplayName,
// u.Reputation,
// us.TotalVotes,
// us.UpVotes,
// us.DownVotes,
// ps.PostId,
// ps.Title,
// ps.CommentCount,
// ps.BadgeCount,
// ps.AnswerCount
// FROM
// UserVoteStats us
// JOIN
// Users u ON us.UserId = u.Id
// JOIN
// PostStats ps ON u.Id = ps.PostId
// WHERE
// u.Reputation > (SELECT AVG(Reputation) FROM Users)
// AND us.TotalVotes > 5
// )
// SELECT
// DisplayName,
// Reputation,
// TotalVotes,
// UpVotes,
// DownVotes,
// Title,
// CommentCount,
// BadgeCount,
// AnswerCount
// FROM
// FeaturedUsers
// ORDER BY
// Reputation DESC, TotalVotes DESC
// LIMIT 10;
fn q8107(db: &'static So) -> String {
    let uid = uids(db);
    let u = db.user.select(&db.user.reputation).fold_flat([0i64; 2], |a, r| [a[0] + 1, a[1] + r]);
    let avgr = u[1] as f64 / u[0] as f64;
    let uvs = vote_named(db);
    let bu = badges_per_user(db);
    let pf = stats_fold(db, db.post.with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cb", &[]);
    let fu = Ident::<User>::new().with((&db.user.reputation).filt(move |r: i64| r as f64 > avgr)).and((&uvs).filt(|a: [i64; 3]| a[0] > 5));
    let mut v = Vec::new();
    (&pf).and((&db.post.owner_user).select(&bu).opt()).and((&db.post.origid).select(&uid).select(fu)).drive(|p, ((s, b), (u, a))| v.push((p, s, b.unwrap_or(0), u, a)));
    out(v, |&(_, _, _, u, a)| (rep_desc(db, u), Reverse(a[0])), 10, |&(p, s, b, u, a)| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(ints(&a));
        f.extend(post_fields(db, p, &["title"]));
        f.extend([V::I(s.cx), V::I(b), V::I(if db.post.post_type_id.get(p).unwrap() == 2 { s.rows } else { 0 })]);
        f
    })
}

// WITH UserBadges AS (
// SELECT UserId, COUNT(*) AS BadgeCount, SUM(CASE WHEN Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM Badges
// GROUP BY UserId
// ),
// UserPosts AS (
// SELECT OwnerUserId, COUNT(*) AS PostCount, SUM(CASE WHEN PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, AVG(Score) AS AvgPostScore
// FROM Posts
// GROUP BY OwnerUserId
// ),
// CombinedStats AS (
// SELECT U.Id AS UserId, U.DisplayName, COALESCE(UB.BadgeCount, 0) AS BadgeCount,
// COALESCE(UB.GoldBadges, 0) AS GoldBadges, COALESCE(UB.SilverBadges, 0) AS SilverBadges,
// COALESCE(UB.BronzeBadges, 0) AS BronzeBadges, COALESCE(UP.PostCount, 0) AS PostCount,
// COALESCE(UP.QuestionCount, 0) AS QuestionCount, COALESCE(UP.AnswerCount, 0) AS AnswerCount,
// COALESCE(UP.AvgPostScore, 0) AS AvgPostScore
// FROM Users U
// LEFT JOIN UserBadges UB ON U.Id = UB.UserId
// LEFT JOIN UserPosts UP ON U.Id = UP.OwnerUserId
// )
// SELECT UserId, DisplayName, BadgeCount, GoldBadges, SilverBadges, BronzeBadges,
// PostCount, QuestionCount, AnswerCount, AvgPostScore
// FROM CombinedStats
// WHERE PostCount > 10
// ORDER BY AvgPostScore DESC
// LIMIT 50;
fn q8130(db: &'static So) -> String {
    let bc = badge_classes(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    (&ps).filt(|p: [i64; 13]| p[0] > 10).and((&bc).opt()).drive(|u, (p, b)| v.push((u, p, b.unwrap_or([0; 4]))));
    let av = |p: [i64; 13]| p[3] as f64 / p[0] as f64;
    out(v, |&(_, p, _)| Reverse(fkey(av(p))), 50, |&(u, p, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&b));
        f.extend(ints(&p[..3]));
        f.push(V::F(av(p)));
        f
    })
}

// WITH UserBadges AS (
// SELECT
// u.Id AS UserId,
// COUNT(b.Id) AS BadgeCount,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// u.Id
// ),
// UserPosts AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS PostCount,
// COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS QuestionCount,
// COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS AnswerCount,
// SUM(p.Score) AS TotalScore
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// ),
// UserSummary AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// ub.BadgeCount,
// ub.GoldBadges,
// ub.SilverBadges,
// ub.BronzeBadges,
// up.PostCount,
// up.QuestionCount,
// up.AnswerCount,
// up.TotalScore
// FROM
// Users u
// LEFT JOIN
// UserBadges ub ON u.Id = ub.UserId
// LEFT JOIN
// UserPosts up ON u.Id = up.OwnerUserId
// )
// SELECT
// us.DisplayName,
// us.BadgeCount,
// us.GoldBadges,
// us.SilverBadges,
// us.BronzeBadges,
// us.PostCount,
// us.QuestionCount,
// us.AnswerCount,
// us.TotalScore
// FROM
// UserSummary us
// WHERE
// us.PostCount > 10
// ORDER BY
// us.TotalScore DESC, us.BadgeCount DESC
// LIMIT 50;
fn q8209(db: &'static So) -> String {
    let ub = ubc(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    (&ub).and((&ps).filt(|p: [i64; 13]| p[0] > 10)).drive(|u, (b, p)| v.push((u, b, p)));
    out(v, |&(_, b, p)| (Reverse(p[3]), Reverse(b[0])), 50, |&(u, b, p)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(ints(&b));
        f.extend(ints(&p[..4]));
        f
    })
}

// WITH UserReputation AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(b.Id) AS BadgeCount,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
// ),
// PostStatistics AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(p.ViewCount) AS TotalViews,
// AVG(p.Score) AS AverageScore
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// ),
// UserEngagement AS (
// SELECT
// ur.UserId,
// ur.DisplayName,
// ur.Reputation,
// ps.TotalPosts,
// ps.Questions,
// ps.Answers,
// ps.TotalViews,
// ps.AverageScore,
// ur.BadgeCount,
// ur.GoldBadges,
// ur.SilverBadges,
// ur.BronzeBadges
// FROM
// UserReputation ur
// JOIN
// PostStatistics ps ON ur.UserId = ps.OwnerUserId
// )
// SELECT
// ue.DisplayName,
// ue.Reputation,
// ue.BadgeCount,
// ue.GoldBadges,
// ue.SilverBadges,
// ue.BronzeBadges,
// ue.TotalPosts,
// ue.Questions,
// ue.Answers,
// ue.TotalViews,
// ue.AverageScore
// FROM
// UserEngagement ue
// WHERE
// ue.Reputation > 1000
// ORDER BY
// ue.Reputation DESC, ue.TotalPosts DESC
// LIMIT 10;
fn q8261(db: &'static So) -> String {
    let ub = ubc(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and(&ub).and(&ps)).drive(|_, x| v.push(x));
    out(v, |&((u, _), p)| (rep_desc(db, u), Reverse(p[0])), 10, |&((u, b), p)| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(ints(&b));
        f.extend(ints(&p[..3]));
        f.extend([pviews(p), pscore_avg(p)]);
        f
    })
}

// WITH UserBadges AS (
// SELECT
// u.Id AS UserId,
// COUNT(b.Id) AS BadgeCount,
// MAX(b.Date) AS LastBadgeDate
// FROM Users u
// LEFT JOIN Badges b ON u.Id = b.UserId
// GROUP BY u.Id
// ),
// PostStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(p.Score) AS TotalScore,
// AVG(p.ViewCount) AS AvgViewCount
// FROM Posts p
// GROUP BY p.OwnerUserId
// ),
// CommentStats AS (
// SELECT
// c.UserId,
// COUNT(c.Id) AS CommentCount,
// SUM(c.Score) AS TotalCommentScore
// FROM Comments c
// GROUP BY c.UserId
// ),
// VoteStats AS (
// SELECT
// v.UserId,
// COUNT(v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
// FROM Votes v
// GROUP BY v.UserId
// )
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// ub.BadgeCount,
// ps.PostCount,
// ps.QuestionCount,
// ps.AnswerCount,
// ps.TotalScore,
// ps.AvgViewCount,
// cs.CommentCount,
// cs.TotalCommentScore,
// vs.VoteCount,
// vs.UpVoteCount,
// vs.DownVoteCount
// FROM Users u
// LEFT JOIN UserBadges ub ON u.Id = ub.UserId
// LEFT JOIN PostStats ps ON u.Id = ps.OwnerUserId
// LEFT JOIN CommentStats cs ON u.Id = cs.UserId
// LEFT JOIN VoteStats vs ON u.Id = vs.UserId
// WHERE u.Reputation > 1000
// ORDER BY u.Reputation DESC, ub.BadgeCount DESC;
fn q8277(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let ps = pstat(db, db.post.iq());
    let cs = db.comment.group_by(&db.comment.user).select(&db.comment.score).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let uv = db.vote.group_by(&db.vote.user).select(&db.vote.vote_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64]);
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and(&bu).and((&ps).opt()).and((&cs).opt()).and((&uv).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((((u, b), p), c), x)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(b)];
        f.extend([0, 1, 2, 3].iter().map(|&i| oint(p.map(|p| p[i]))));
        f.push(onull(p, pviews_avg));
        f.extend((0..2).map(|i| oint(c.map(|c| c[i]))));
        f.extend((0..3).map(|i| oint(x.map(|x| x[i]))));
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
// PostStatistics AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(CASE WHEN p.PostTypeId = 4 THEN 1 ELSE 0 END) AS TagWikis,
// AVG(p.Score) AS AverageScore,
// SUM(p.ViewCount) AS TotalViews
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// )
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COALESCE(ub.BadgeCount, 0) AS BadgeCount,
// COALESCE(ub.GoldBadges, 0) AS GoldBadges,
// COALESCE(ub.SilverBadges, 0) AS SilverBadges,
// COALESCE(ub.BronzeBadges, 0) AS BronzeBadges,
// COALESCE(ps.TotalPosts, 0) AS TotalPosts,
// COALESCE(ps.Questions, 0) AS TotalQuestions,
// COALESCE(ps.Answers, 0) AS TotalAnswers,
// COALESCE(ps.TagWikis, 0) AS TotalTagWikis,
// COALESCE(ps.AverageScore, 0) AS AverageScore,
// COALESCE(ps.TotalViews, 0) AS TotalViews
// FROM
// Users u
// LEFT JOIN
// UserBadges ub ON u.Id = ub.UserId
// LEFT JOIN
// PostStatistics ps ON u.Id = ps.OwnerUserId
// ORDER BY
// BadgeCount DESC, TotalPosts DESC
// LIMIT 10;
fn q8281(db: &'static So) -> String {
    let ub = ubc(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    (&ub).and((&ps).opt()).drive(|u, (b, p)| v.push((u, b, p)));
    out(v, |&(_, b, p)| (Reverse(b[0]), Reverse(p.map_or(0, |p| p[0]))), 10, |&(u, b, p)| {
        let q = p.unwrap_or(Z);
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&b));
        f.extend(ints(&[q[0], q[1], q[2], q[12]]));
        f.extend([p.map_or(V::F(0.0), pscore_avg), V::I(q[5])]);
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
// FROM Users U
// LEFT JOIN Badges B ON U.Id = B.UserId
// GROUP BY U.Id, U.DisplayName
// ),
// TopUsersByReputation AS (
// SELECT
// Id,
// DisplayName,
// Reputation
// FROM Users
// ORDER BY Reputation DESC
// LIMIT 10
// ),
// PostEngagement AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS PostCount,
// SUM(P.ViewCount) AS TotalViews,
// SUM(P.Score) AS TotalScore
// FROM Posts P
// WHERE P.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year')
// GROUP BY P.OwnerUserId
// )
// SELECT
// U.DisplayName,
// T.Reputation,
// U.BadgeCount,
// U.GoldBadges,
// U.SilverBadges,
// U.BronzeBadges,
// PE.PostCount,
// PE.TotalViews,
// PE.TotalScore
// FROM UserBadgeCounts U
// JOIN PostEngagement PE ON U.UserId = PE.OwnerUserId
// JOIN TopUsersByReputation T ON U.UserId = T.Id
// ORDER BY T.Reputation DESC, U.BadgeCount DESC;
fn q8288(db: &'static So) -> String {
    let ub = ubc(db);
    let top: MatSet<Id<User>> = whole(&db.user.id).select(Ident::<User>::new().and(&db.user.reputation)).window(row_number, |(_, r)| r, desc).filt(|(_, n)| n <= 10).map(|((u, _), _)| u).collect();
    let rp = pstat(db, since(db, year_ago()));
    let mut v = Vec::new();
    (&top).select(Ident::<User>::new().and(&ub).and(&rp)).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((u, b), p)| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(ints(&b));
        f.extend([V::I(p[0]), pviews(p), V::I(p[3])]);
        row(f)
    }))
}

// WITH UserBadges AS (
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
// COUNT(P.Id) AS PostCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// AVG(P.Score) AS AvgScore,
// SUM(P.ViewCount) AS TotalViews
// FROM
// Posts P
// WHERE
// P.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY
// P.OwnerUserId
// ),
// CombinedStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// UB.BadgeCount,
// UB.GoldBadges,
// UB.SilverBadges,
// UB.BronzeBadges,
// PS.PostCount,
// PS.QuestionCount,
// PS.AnswerCount,
// PS.AvgScore,
// PS.TotalViews
// FROM
// Users U
// LEFT JOIN
// UserBadges UB ON U.Id = UB.UserId
// LEFT JOIN
// PostStats PS ON U.Id = PS.OwnerUserId
// )
// SELECT
// UserId,
// DisplayName,
// Reputation,
// BadgeCount,
// GoldBadges,
// SilverBadges,
// BronzeBadges,
// PostCount,
// QuestionCount,
// AnswerCount,
// AvgScore,
// TotalViews
// FROM
// CombinedStats
// WHERE
// Reputation > 1000
// ORDER BY
// Reputation DESC,
// PostCount DESC
// LIMIT 50;
fn q8408(db: &'static So) -> String {
    let ub = ubc(db);
    let ps = pstat(db, db.post.with((&db.post.creation_date).gt(year_ago())));
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and(&ub).and((&ps).opt())).drive(|_, x| v.push(x));
    out(v, |&((u, _), p)| (rep_desc(db, u), (p.is_none(), Reverse(p.map(|p| p[0])))), 50, |&((u, b), p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(ints(&b));
        f.extend((0..3).map(|i| oint(p.map(|p| p[i]))));
        f.extend([onull(p, pscore_avg), onull(p, pviews)]);
        f
    })
}

// WITH UserBadgeStats AS (
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
// PostStats AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS TotalPosts,
// COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS TotalQuestions,
// COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS TotalAnswers,
// SUM(P.Score) AS TotalScore,
// SUM(P.ViewCount) AS TotalViews
// FROM
// Posts P
// GROUP BY
// P.OwnerUserId
// ),
// UserPostBadgeStats AS (
// SELECT
// UB.UserId,
// UB.DisplayName,
// COALESCE(PS.TotalPosts, 0) AS TotalPosts,
// COALESCE(PS.TotalQuestions, 0) AS TotalQuestions,
// COALESCE(PS.TotalAnswers, 0) AS TotalAnswers,
// COALESCE(PS.TotalScore, 0) AS TotalScore,
// COALESCE(PS.TotalViews, 0) AS TotalViews,
// UB.TotalBadges,
// UB.GoldBadges,
// UB.SilverBadges,
// UB.BronzeBadges
// FROM
// UserBadgeStats UB
// LEFT JOIN
// PostStats PS ON UB.UserId = PS.OwnerUserId
// )
// SELECT
// U.UserId,
// U.DisplayName,
// U.TotalPosts,
// U.TotalQuestions,
// U.TotalAnswers,
// U.TotalScore,
// U.TotalViews,
// U.TotalBadges,
// U.GoldBadges,
// U.SilverBadges,
// U.BronzeBadges
// FROM
// UserPostBadgeStats U
// ORDER BY
// U.TotalScore DESC, U.TotalPosts DESC
// LIMIT 50;
fn q8507(db: &'static So) -> String {
    let ub = ubc(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    (&ub).and((&ps).opt()).drive(|u, (b, p)| v.push((u, b, p.unwrap_or(Z))));
    out(v, |&(_, _, p)| (Reverse(p[3]), Reverse(p[0])), 50, |&(u, b, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&[p[0], p[1], p[2], p[3], p[5]]));
        f.extend(ints(&b));
        f
    })
}

// WITH UserReputations AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// U.CreationDate,
// COUNT(DISTINCT P.Id) AS PostCount,
// SUM(V.BountyAmount) AS TotalBounty
// FROM Users U
// LEFT JOIN Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN Votes V ON U.Id = V.UserId
// WHERE U.Reputation > 1000
// GROUP BY U.Id, U.DisplayName, U.Reputation, U.CreationDate
// ),
// PostEngagements AS (
// SELECT
// P.OwnerUserId,
// COUNT(C.Id) AS CommentCount,
// SUM(P.ViewCount) AS TotalViews,
// AVG(P.Score) AS AverageScore
// FROM Posts P
// LEFT JOIN Comments C ON P.Id = C.PostId
// WHERE P.CreationDate > '2020-01-01'
// GROUP BY P.OwnerUserId
// ),
// CombinedData AS (
// SELECT
// UR.UserId,
// UR.DisplayName,
// UR.Reputation,
// UR.PostCount,
// UR.TotalBounty,
// PE.CommentCount,
// PE.TotalViews,
// PE.AverageScore
// FROM UserReputations UR
// LEFT JOIN PostEngagements PE ON UR.UserId = PE.OwnerUserId
// )
// SELECT
// CD.DisplayName,
// CD.Reputation,
// CD.PostCount,
// CD.TotalBounty,
// COALESCE(CD.CommentCount, 0) AS CommentCount,
// COALESCE(CD.TotalViews, 0) AS TotalViews,
// COALESCE(CD.AverageScore, 0) AS AverageScore
// FROM CombinedData CD
// ORDER BY CD.Reputation DESC, CD.TotalViews DESC
// LIMIT 50;
fn q8623(db: &'static So) -> String {
    let w = UserWhere::RepGt(1000);
    let dp = ud(db, w, posts_of(db));
    let ub = user_base(db, w).group_by(Ident::<User>::new()).select(posts_of(db).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt())).fold([0i64; 2], |a, (_, b)| {
        let b = b.flatten();
        [a[0] + b.is_some() as i64, a[1] + b.unwrap_or(0)]
    });
    let pe = owned(db)
        .with((&db.post.creation_date).gt(date(2020, 1, 1)))
        .group_by(&db.post.owner_user)
        .select(comments_of(db).opt().and((&db.post.view_count).opt()).and(&db.post.score))
        .fold([0i64; 5], |a, ((c, w), s)| [a[0] + 1, a[1] + c.is_some() as i64, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + s]);
    let mut v = Vec::new();
    (&ub).and((&dp).opt()).and((&pe).opt()).drive(|u, ((a, d), p)| v.push((u, a, d.unwrap_or(0), p)));
    let tv = |p: Option<[i64; 5]>| p.filter(|p| p[2] > 0).map(|p| p[3]);
    out(v, |&(u, _, _, p)| (rep_desc(db, u), (tv(p).is_none(), Reverse(tv(p)))), 50, |&(u, a, d, p)| {
        vec![
            user_col(db, u, "name"),
            user_col(db, u, "rep"),
            V::I(d),
            nullable(a[1], a[0]),
            V::I(p.map_or(0, |p| p[1])),
            V::I(tv(p).unwrap_or(0)),
            p.map_or(V::F(0.0), |p| avg(p[4], p[0])),
        ]
    })
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.PostTypeId,
// COUNT(c.Id) AS TotalComments,
// COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 2) AS UpVotes,
// COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 3) AS DownVotes,
// AVG(u.Reputation) AS AvgUserReputation
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY
// p.Id, p.Title, p.PostTypeId
// ),
// PostHistorySummary AS (
// SELECT
// ph.PostId,
// COUNT(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 END) AS TotalCloseVotes,
// COUNT(CASE WHEN ph.PostHistoryTypeId IN (11, 13) THEN 1 END) AS TotalOpenVotes
// FROM
// PostHistory ph
// GROUP BY
// ph.PostId
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.PostTypeId,
// ps.TotalComments,
// ps.UpVotes,
// ps.DownVotes,
// ps.AvgUserReputation,
// phs.TotalCloseVotes,
// phs.TotalOpenVotes
// FROM
// PostStats ps
// LEFT JOIN
// PostHistorySummary phs ON ps.PostId = phs.PostId
// ORDER BY
// ps.UpVotes - ps.DownVotes DESC,
// ps.TotalComments DESC;
fn q8632(db: &'static So) -> String {
    let hs = db.post_history.group_by(&db.post_history.post).select(&db.post_history.post_history_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 10) as i64, a[1] + matches!(t, 11 | 13) as i64]);
    let mut v = Vec::new();
    stats_fold(db, since(db, year_ago()), Ident::<Post>::new(), "cv", &[]).and((&hs).opt()).drive(|p, (s, h)| v.push((p, s, h)));
    rows(v.iter().map(|&(p, s, h)| {
        let mut f = post_fields(db, p, &["id", "title", "type_id"]);
        f.extend([V::I(s.cx), V::I(s.up), V::I(s.down)]);
        f.push(ofloat(db.post.owner_user.get(p).map(|u| db.user.reputation.get(u).unwrap() as f64)));
        f.extend((0..2).map(|i| oint(h.map(|h| h[i]))));
        row(f)
    }))
}

// WITH UserReputation AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// u.DisplayName,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN pt.Name = 'Question' THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN pt.Name = 'Answer' THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN b.Id IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN Badges b ON u.Id = b.UserId
// GROUP BY u.Id, u.Reputation, u.DisplayName
// ),
// UserActivity AS (
// SELECT
// u.UserId,
// SUM(CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN 1 ELSE 0 END) AS CloseOpenCount,
// SUM(CASE WHEN ph.PostHistoryTypeId = 12 THEN 1 ELSE 0 END) AS DeletedCount,
// SUM(CASE WHEN ph.PostHistoryTypeId = 13 THEN 1 ELSE 0 END) AS UndeletedCount
// FROM Posts p
// JOIN PostHistory ph ON p.Id = ph.PostId
// JOIN UserReputation u ON u.UserId = p.OwnerUserId
// GROUP BY u.UserId
// )
// SELECT
// ur.DisplayName,
// ur.Reputation,
// ur.PostCount,
// ur.QuestionCount,
// ur.AnswerCount,
// ur.BadgeCount,
// ua.CloseOpenCount,
// ua.DeletedCount,
// ua.UndeletedCount
// FROM UserReputation ur
// JOIN UserActivity ua ON ur.UserId = ua.UserId
// WHERE ur.Reputation > 1000
// ORDER BY ur.Reputation DESC, ur.PostCount DESC;
fn q8700(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::RepGt(1000), "b", any_post);
    let ua = owned(db).group_by(&db.post.owner_user).select(history_of(db).select(&db.post_history.post_history_type_id)).fold([0i64; 3], |a, t| {
        [a[0] + matches!(t, 10 | 11) as i64, a[1] + (t == 12) as i64, a[2] + (t == 13) as i64]
    });
    let mut v = Vec::new();
    (&us).and(&ua).drive(|u, (a, h)| v.push((u, a, h)));
    rows(v.iter().map(|&(u, a, h)| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(ints(&[a.n, a.q, a.a, a.bx]));
        f.extend(ints(&h));
        row(f)
    }))
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
// UserPostStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(p.ViewCount) AS TotalViews
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// ),
// PostHistoryStats AS (
// SELECT
// ph.UserId,
// COUNT(ph.Id) AS EditCount,
// SUM(CASE WHEN ph.PostHistoryTypeId IN (4, 5, 6) THEN 1 ELSE 0 END) AS TitleAndBodyEdits
// FROM
// PostHistory ph
// GROUP BY
// ph.UserId
// )
// SELECT
// u.DisplayName,
// u.Reputation,
// ubc.BadgeCount,
// ups.PostCount,
// ups.QuestionCount,
// ups.AnswerCount,
// ups.TotalViews,
// phs.EditCount,
// phs.TitleAndBodyEdits
// FROM
// Users u
// LEFT JOIN
// UserBadgeCounts ubc ON u.Id = ubc.UserId
// LEFT JOIN
// UserPostStats ups ON u.Id = ups.OwnerUserId
// LEFT JOIN
// PostHistoryStats phs ON u.Id = phs.UserId
// WHERE
// u.Reputation > 1000
// ORDER BY
// ubc.BadgeCount DESC,
// ups.TotalViews DESC
// LIMIT 100;
fn q8731(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let ps = pstat(db, db.post.iq());
    let ph = db.post_history.group_by(&db.post_history.user).select(&db.post_history.post_history_type_id).fold([0i64; 2], |a, t| [a[0] + 1, a[1] + matches!(t, 4 | 5 | 6) as i64]);
    let tv = |p: Option<[i64; 13]>| p.filter(|p| p[4] > 0).map(|p| p[5]);
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and(&bu).and((&ps).opt()).and((&ph).opt())).drive(|_, x| v.push(x));
    out(v, |&(((_, b), p), _)| (Reverse(b), (tv(p).is_none(), Reverse(tv(p)))), 100, |&(((u, b), p), h)| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep"), V::I(b)];
        f.extend((0..3).map(|i| oint(p.map(|p| p[i]))));
        f.push(oint(tv(p)));
        f.extend((0..2).map(|i| oint(h.map(|h| h[i]))));
        f
    })
}

// WITH UserPostStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(P.Id) AS PostCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(V.BountyAmount) AS TotalBounty,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// WHERE
// U.Reputation > 1000
// GROUP BY
// U.Id, U.DisplayName
// ),
// UserBadges AS (
// SELECT
// B.UserId,
// COUNT(B.Id) AS BadgeCount,
// MAX(B.Class) AS HighestBadgeClass
// FROM
// Badges B
// GROUP BY
// B.UserId
// )
// SELECT
// PS.UserId,
// PS.DisplayName,
// PS.PostCount,
// PS.QuestionCount,
// PS.AnswerCount,
// PS.TotalBounty,
// PS.TotalUpVotes,
// PS.TotalDownVotes,
// COALESCE(B.BadgeCount, 0) AS BadgeCount,
// COALESCE(B.HighestBadgeClass, 0) AS HighestBadgeClass
// FROM
// UserPostStats PS
// LEFT JOIN
// UserBadges B ON PS.UserId = B.UserId
// ORDER BY
// PS.TotalUpVotes DESC, PS.PostCount DESC
// LIMIT 50;
fn q8739(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::RepGt(1000), "v", any_post);
    let bm = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 2], |a, c| [a[0] + 1, a[1].max(c)]);
    let mut v = Vec::new();
    (&us).and((&bm).opt()).drive(|u, (a, b)| v.push((u, a, b.unwrap_or([0; 2]))));
    out(v, |&(_, a, _)| (Reverse(a.up), Reverse(a.n)), 50, |&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&[a.n, a.q, a.a]));
        f.extend([ustat_field(&a, "bounty_sum"), V::I(a.up), V::I(a.down), V::I(b[0]), V::I(b[1])]);
        f
    })
}

// WITH UserReputation AS (
// SELECT
// Id AS UserId,
// Reputation,
// CreationDate,
// LastAccessDate,
// AboutMe,
// UpVotes,
// DownVotes,
// (UpVotes - DownVotes) AS NetVotes
// FROM Users
// ),
// RecentPosts AS (
// SELECT
// p.Id AS PostId,
// p.PostTypeId,
// p.OwnerUserId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
// FROM Posts p
// LEFT JOIN Comments c ON p.Id = c.PostId
// LEFT JOIN Votes v ON p.Id = v.PostId
// WHERE p.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'
// GROUP BY p.Id, p.PostTypeId, p.OwnerUserId, p.Title, p.CreationDate, p.Score, p.ViewCount
// ),
// PostStatistics AS (
// SELECT
// rp.PostId,
// rp.Title,
// rp.CreationDate,
// rp.Score,
// rp.ViewCount,
// rp.CommentCount,
// rp.UpVoteCount,
// rp.DownVoteCount,
// ur.Reputation,
// ur.NetVotes
// FROM RecentPosts rp
// JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId
// ORDER BY rp.CreationDate DESC
// LIMIT 50
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.Score,
// ps.ViewCount,
// ps.CommentCount,
// ps.UpVoteCount,
// ps.DownVoteCount,
// ps.Reputation,
// ps.NetVotes
// FROM PostStatistics ps
// WHERE ps.Score > 0
// ORDER BY ps.NetVotes DESC, ps.CreationDate DESC;
fn q8748(db: &'static So) -> String {
    let base = owned(db).with((&db.post.creation_date).gt(month_ago()));
    let pf = stats_fold(db, owned(db).with((&db.post.creation_date).gt(month_ago())), Ident::<Post>::new(), "cv", &[]);
    let top: MatSet<Id<Post>> = whole(&base).select(Ident::<Post>::new().and(&db.post.creation_date)).window(row_number, |(_, c)| c, desc).filt(|(_, n)| n <= 50).map(|((p, _), _)| p).collect();
    let mut v = Vec::new();
    (&top).select(Ident::<Post>::new().with((&db.post.score).gt(0)).and(&pf)).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(p, s)| {
        let u = db.post.owner_user.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(s.cx), V::I(s.up), V::I(s.down), user_col(db, u, "rep"), V::I(db.user.up_votes.get(u).unwrap() - db.user.down_votes.get(u).unwrap())]);
        row(f)
    }))
}

// WITH UserVotes AS (
// SELECT V.UserId,
// COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVotes,
// COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVotes
// FROM Votes V
// GROUP BY V.UserId
// ),
// PostStats AS (
// SELECT P.OwnerUserId,
// COUNT(P.Id) AS TotalPosts,
// SUM(P.Score) AS TotalScore,
// SUM(P.ViewCount) AS TotalViews
// FROM Posts P
// GROUP BY P.OwnerUserId
// ),
// UserPostDetails AS (
// SELECT U.Id AS UserId,
// U.DisplayName,
// COALESCE(UV.UpVotes, 0) AS UpVotes,
// COALESCE(UV.DownVotes, 0) AS DownVotes,
// COALESCE(PS.TotalPosts, 0) AS TotalPosts,
// COALESCE(PS.TotalScore, 0) AS TotalScore,
// COALESCE(PS.TotalViews, 0) AS TotalViews
// FROM Users U
// LEFT JOIN UserVotes UV ON U.Id = UV.UserId
// LEFT JOIN PostStats PS ON U.Id = PS.OwnerUserId
// )
// SELECT U.DisplayName,
// U.UpVotes,
// U.DownVotes,
// U.TotalPosts,
// U.TotalScore,
// U.TotalViews
// FROM UserPostDetails U
// WHERE U.TotalPosts > 0
// ORDER BY U.TotalScore DESC, U.TotalViews DESC
// LIMIT 10;
fn q8878(db: &'static So) -> String {
    let uv = db.vote.group_by(&db.vote.user).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    (&ps).filt(|p: [i64; 13]| p[0] > 0).and((&uv).opt()).drive(|u, (p, x)| v.push((u, p, x.unwrap_or([0; 2]))));
    out(v, |&(_, p, _)| (Reverse(p[3]), Reverse(p[5])), 10, |&(u, p, x)| vec![user_col(db, u, "name"), V::I(x[0]), V::I(x[1]), V::I(p[0]), V::I(p[3]), V::I(p[5])])
}

// WITH UserActivity AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(DISTINCT P.Id) AS TotalPosts,
// COUNT(DISTINCT C.Id) AS TotalComments,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes,
// SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM Users U
// LEFT JOIN Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN Comments C ON P.Id = C.PostId
// LEFT JOIN Votes V ON P.Id = V.PostId AND V.UserId = U.Id
// LEFT JOIN Badges B ON U.Id = B.UserId
// GROUP BY U.Id, U.DisplayName
// ),
// PostStatistics AS (
// SELECT
// P.OwnerUserId,
// COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS Questions,
// COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS Answers,
// AVG(P.Score) AS AverageScore,
// SUM(P.ViewCount) AS TotalViews,
// COUNT(DISTINCT P.Tags) AS UniqueTags
// FROM Posts P
// GROUP BY P.OwnerUserId
// ),
// CombinedStats AS (
// SELECT
// UA.UserId,
// UA.DisplayName,
// UA.TotalPosts,
// UA.TotalComments,
// UA.TotalUpVotes,
// UA.TotalDownVotes,
// UA.GoldBadges,
// UA.SilverBadges,
// UA.BronzeBadges,
// PS.Questions,
// PS.Answers,
// PS.AverageScore,
// PS.TotalViews,
// PS.UniqueTags
// FROM UserActivity UA
// LEFT JOIN PostStatistics PS ON UA.UserId = PS.OwnerUserId
// )
// SELECT *
// FROM CombinedStats
// ORDER BY TotalPosts DESC, TotalUpVotes DESC, AverageScore DESC
// LIMIT 100;
fn q8887(db: &'static So) -> String {
    let own = self_votes(db);
    let uf = g(db)
        .select(posts_of(db).select(comments_of(db).opt().and((&own).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 5], |a, (p, c)| {
            let t = p.and_then(|(_, t)| t);
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + (c == Some(1)) as i64, a[3] + (c == Some(2)) as i64, a[4] + (c == Some(3)) as i64]
        });
    let dp = ud(db, UserWhere::All, posts_of(db));
    let dc = ud(db, UserWhere::All, posts_of(db).select(comments_of(db)));
    let ps = pstat(db, db.post.iq());
    let dt = owned(db).group_by(&db.post.owner_user).select(&db.post.tags_str).count_distinct();
    let mut v = Vec::new();
    (&uf).and((&dp).opt()).and((&dc).opt()).and((&ps).opt()).and((&dt).opt()).drive(|u, ((((a, p), c), s), t)| v.push((u, a, p.unwrap_or(0), c.unwrap_or(0), s, t)));
    let av = |s: Option<[i64; 13]>| s.map(|s| s[3] as f64 / s[0] as f64);
    out(v, |&(_, a, p, _, s, _)| (Reverse(p), Reverse(a[0]), (s.is_none(), Reverse(av(s).map(fkey)))), 100, |&(u, a, p, c, s, t)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(p), V::I(c)];
        f.extend(ints(&a));
        f.extend([oint(s.map(|s| s[1])), oint(s.map(|s| s[2])), ofloat(av(s)), onull(s, pviews), if s.is_some() { V::I(t.unwrap_or(0)) } else { V::Null }]);
        f
    })
}

// WITH UserActivity AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsAsked,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersGiven,
// SUM(CASE WHEN P.ViewCount > 100 THEN 1 ELSE 0 END) AS PopularPosts,
// COUNT(COALESCE(CM.Id, 0)) AS CommentCount,
// AVG(U.Reputation) AS AvgReputation
// FROM Users U
// LEFT JOIN Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN Comments CM ON P.Id = CM.PostId
// GROUP BY U.Id, U.DisplayName
// ),
// UserBadges AS (
// SELECT
// B.UserId,
// COUNT(CASE WHEN B.Class = 1 THEN 1 END) AS GoldBadges,
// COUNT(CASE WHEN B.Class = 2 THEN 1 END) AS SilverBadges,
// COUNT(CASE WHEN B.Class = 3 THEN 1 END) AS BronzeBadges
// FROM Badges B
// GROUP BY B.UserId
// )
// SELECT
// UA.UserId,
// UA.DisplayName,
// UA.QuestionsAsked,
// UA.AnswersGiven,
// UA.PopularPosts,
// UA.CommentCount,
// UB.GoldBadges,
// UB.SilverBadges,
// UB.BronzeBadges,
// UA.AvgReputation
// FROM UserActivity UA
// LEFT JOIN UserBadges UB ON UA.UserId = UB.UserId
// ORDER BY UA.AvgReputation DESC, UA.QuestionsAsked DESC
// LIMIT 10;
fn q8933(db: &'static So) -> String {
    let Post { post_type_id, view_count, .. } = &db.post;
    let uf = g(db).select(posts_of(db).select(post_type_id.and(view_count.opt()).and(comments_of(db).opt())).opt()).fold([0i64; 4], |a, p| match p {
        Some(((t, w), _)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.map_or(false, |w| w > 100) as i64],
        None => [a[0] + 1, a[1], a[2], a[3]],
    });
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&uf).and((&bc).opt()).drive(|u, (a, b)| v.push((u, a, b)));
    out(v, |&(u, a, _)| (rep_desc(db, u), Reverse(a[1])), 10, |&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[0])];
        f.extend((1..4).map(|i| oint(b.map(|b| b[i]))));
        f.push(V::F(db.user.reputation.get(u).unwrap() as f64));
        f
    })
}

// WITH UserBadges AS (
// SELECT U.Id AS UserId, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges, SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM Users U
// LEFT JOIN Badges B ON U.Id = B.UserId
// GROUP BY U.Id
// ), PostsStats AS (
// SELECT P.OwnerUserId, COUNT(P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(P.ViewCount) AS TotalViews, AVG(P.Score) AS AverageScore
// FROM Posts P
// GROUP BY P.OwnerUserId
// ), UserProfile AS (
// SELECT U.Id, U.DisplayName, U.Reputation, UB.BadgeCount, UB.GoldBadges, UB.SilverBadges, UB.BronzeBadges,
// PS.TotalPosts, PS.Questions, PS.Answers, PS.TotalViews, PS.AverageScore, U.CreationDate
// FROM Users U
// JOIN UserBadges UB ON U.Id = UB.UserId
// JOIN PostsStats PS ON U.Id = PS.OwnerUserId
// )
// SELECT UP.DisplayName, UP.Reputation, UP.BadgeCount, UP.GoldBadges, UP.SilverBadges, UP.BronzeBadges,
// UP.TotalPosts, UP.Questions, UP.Answers, UP.TotalViews, UP.AverageScore, UP.CreationDate
// FROM UserProfile UP
// WHERE UP.Reputation > 1000
// ORDER BY UP.Reputation DESC, UP.TotalPosts DESC
// LIMIT 100;
fn q8942(db: &'static So) -> String {
    let ub = ubc(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and(&ub).and(&ps)).drive(|_, x| v.push(x));
    out(v, |&((u, _), p)| (rep_desc(db, u), Reverse(p[0])), 100, |&((u, b), p)| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(ints(&b));
        f.extend(ints(&p[..3]));
        f.extend([pviews(p), pscore_avg(p), user_col(db, u, "ucreated")]);
        f
    })
}

// WITH UserBadges AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(CASE WHEN B.Class = 1 THEN 1 END) AS GoldBadges,
// COUNT(CASE WHEN B.Class = 2 THEN 1 END) AS SilverBadges,
// COUNT(CASE WHEN B.Class = 3 THEN 1 END) AS BronzeBadges
// FROM
// Users U
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// GROUP BY
// U.Id, U.DisplayName
// ),
// PopularPosts AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.Score,
// P.ViewCount,
// P.AnswerCount,
// P.CommentCount,
// P.CreationDate,
// U.DisplayName AS OwnerDisplayName
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// WHERE
// P.PostTypeId = 1
// ORDER BY
// P.ViewCount DESC
// LIMIT 10
// ),
// RecentActivity AS (
// SELECT
// PH.UserId,
// PH.PostId,
// PH.CreationDate,
// P.Title,
// P.LastActivityDate,
// PT.Name AS PostType
// FROM
// PostHistory PH
// JOIN
// Posts P ON PH.PostId = P.Id
// JOIN
// PostHistoryTypes PT ON PH.PostHistoryTypeId = PT.Id
// ORDER BY
// PH.CreationDate DESC
// LIMIT 5
// )
// SELECT
// UB.UserId,
// UB.DisplayName,
// UB.GoldBadges,
// UB.SilverBadges,
// UB.BronzeBadges,
// PP.PostId,
// PP.Title AS PopularPostTitle,
// PP.Score AS PopularPostScore,
// PP.ViewCount AS PopularPostViewCount,
// PP.AnswerCount AS PopularPostAnswerCount,
// PP.CommentCount AS PopularPostCommentCount,
// RA.PostId AS RecentActivityPostId,
// RA.CreationDate AS RecentActivityDate,
// RA.Title AS RecentActivityPostTitle,
// RA.PostType AS RecentActivityPostType
// FROM
// UserBadges UB
// LEFT JOIN
// PopularPosts PP ON UB.DisplayName = PP.OwnerDisplayName
// LEFT JOIN
// RecentActivity RA ON UB.UserId = RA.UserId
// WHERE
// UB.GoldBadges > 0 OR UB.SilverBadges > 0 OR UB.BronzeBadges > 0
// ORDER BY
// UB.UserId, PP.ViewCount DESC, RA.CreationDate DESC;
fn q8955(db: &'static So) -> String {
    let ub = ubc(db);
    let base = owned(db).with((&db.post.post_type_id).eq(1));
    let pp: MatSet<Id<Post>> = whole(&base).select(Ident::<Post>::new().and((&db.post.view_count).opt())).window(row_number, |(_, w)| w, desc).filt(|(_, n)| n <= 10).map(|((p, _), _)| p).collect();
    let ppn: HashIdx<Str, Id<Post>> = (&pp).select((&db.post.owner_user).select(&db.user.display_name)).inv().collect();
    let hbase = db.post_history.iq();
    let ra: MatSet<Id<PostHistory>> = whole(hbase).select(Ident::<PostHistory>::new().and(&db.post_history.creation_date)).window(row_number, |(_, d)| d, desc).filt(|(_, n)| n <= 5).map(|((h, _), _)| h).collect();
    let rai: HashIdx<Id<User>, Id<PostHistory>> = (&ra).select(&db.post_history.user).inv().collect();
    let mut v = Vec::new();
    (&ub)
        .filt(|b: [i64; 4]| b[1] > 0 || b[2] > 0 || b[3] > 0)
        .and((&db.user.display_name).select(&ppn).opt())
        .and((&rai).opt())
        .drive(|u, ((b, p), h)| v.push((u, b, p, h)));
    rows(v.iter().map(|&(u, b, p, h)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&b[1..]));
        match p {
            Some(p) => f.extend(post_fields(db, p, &["id", "title", "score", "views", "answers", "comments"])),
            None => f.extend(nulls(6)),
        }
        match h {
            Some(h) => {
                let q = db.post_history.post.get(h).unwrap();
                f.extend(post_fields(db, q, &["id"]));
                f.push(V::T(db.post_history.creation_date.get(h).unwrap()));
                f.extend(post_fields(db, q, &["title"]));
                f.push(V::S(db.post_history_type.name.get(db.post_history.post_history_type.get(h).unwrap()).unwrap()));
            }
            None => f.extend(nulls(4)),
        }
        row(f)
    }))
}

// WITH UserReputation AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(P.Id) AS PostCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(V.BountyAmount) AS TotalBounty
// FROM Users U
// LEFT JOIN Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId IN (8, 9)
// GROUP BY U.Id, U.DisplayName, U.Reputation
// ),
// ActiveBadges AS (
// SELECT
// B.UserId,
// COUNT(CASE WHEN B.Class = 1 THEN 1 END) AS GoldBadges,
// COUNT(CASE WHEN B.Class = 2 THEN 1 END) AS SilverBadges,
// COUNT(CASE WHEN B.Class = 3 THEN 1 END) AS BronzeBadges
// FROM Badges B
// GROUP BY B.UserId
// ),
// TopUsers AS (
// SELECT
// UR.UserId,
// UR.DisplayName,
// UR.Reputation,
// UR.PostCount,
// UR.QuestionCount,
// UR.AnswerCount,
// AB.GoldBadges,
// AB.SilverBadges,
// AB.BronzeBadges,
// UR.TotalBounty
// FROM UserReputation UR
// LEFT JOIN ActiveBadges AB ON UR.UserId = AB.UserId
// ORDER BY UR.Reputation DESC, UR.PostCount DESC
// LIMIT 10
// )
// SELECT
// TU.DisplayName,
// TU.Reputation,
// TU.PostCount,
// TU.QuestionCount,
// TU.AnswerCount,
// COALESCE(TU.GoldBadges, 0) AS GoldBadges,
// COALESCE(TU.SilverBadges, 0) AS SilverBadges,
// COALESCE(TU.BronzeBadges, 0) AS BronzeBadges,
// TU.TotalBounty
// FROM TopUsers TU;
fn q8966(db: &'static So) -> String {
    let us = user_stats_fold_v(db, Ident::<User>::new(), UserWhere::All, "v", any_post, &[8, 9]);
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&us).and((&bc).opt()).drive(|u, (a, b)| v.push((u, a, b.unwrap_or([0; 4]))));
    out(v, |&(u, a, _)| (rep_desc(db, u), Reverse(a.n)), 10, |&(u, a, b)| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(ints(&[a.n, a.q, a.a]));
        f.extend(ints(&b[1..]));
        f.push(ustat_field(&a, "bounty_sum"));
        f
    })
}

// WITH UserReputation AS (
// SELECT Id, Reputation, UpVotes, DownVotes, (UpVotes - DownVotes) AS NetVotes
// FROM Users
// ),
// PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// COALESCE(a.AcceptedAnswerId, -1) AS AcceptedAnswerId,
// COUNT(DISTINCT c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount
// FROM Posts p
// LEFT JOIN Posts a ON p.Id = a.AcceptedAnswerId
// LEFT JOIN Comments c ON p.Id = c.PostId
// LEFT JOIN Votes v ON p.Id = v.PostId
// WHERE p.PostTypeId = 1
// GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, a.AcceptedAnswerId
// ),
// TopPosts AS (
// SELECT
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.Score,
// ps.ViewCount,
// ps.AcceptedAnswerId,
// ps.CommentCount,
// ps.VoteCount,
// ur.Reputation
// FROM PostStats ps
// JOIN UserReputation ur ON ps.PostId = ur.Id
// ORDER BY ps.Score DESC, ps.ViewCount DESC
// LIMIT 10
// )
// SELECT
// tp.PostId,
// tp.Title,
// tp.CreationDate,
// tp.Score,
// tp.ViewCount,
// tp.CommentCount,
// tp.VoteCount,
// tp.Reputation
// FROM TopPosts tp
// JOIN Badges b ON b.UserId = tp.PostId
// WHERE b.Class = 1
// ORDER BY tp.Reputation DESC;
fn q9025(db: &'static So) -> String {
    let uid = uids(db);
    let base = questions_only(db).with((&db.post.origid).select(&uid));
    let top: MatSet<Id<Post>> = whole(&base)
        .select(Ident::<Post>::new().and(&db.post.score).and((&db.post.view_count).opt()))
        .window(row_number, |((_, s), w)| (s, w), desc)
        .filt(|(_, n)| n <= 10)
        .map(|(((p, _), _), _)| p)
        .collect();
    let gold: HashIdx<Id<User>, Id<Badge>> = db.badge.with((&db.badge.class).eq(1)).select(&db.badge.user).inv().collect();
    let mut v = Vec::new();
    (&top)
        .select(Ident::<Post>::new().and(comments_per_post(db)).and(votes_per_post(db)).and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&gold))))
        .drive(|_, x| v.push(x));
    rows(v.iter().map(|&(((p, c), x), (u, _))| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(c), V::I(x), user_col(db, u, "rep")]);
        row(f)
    }))
}

// SELECT
// u.DisplayName AS UserDisplayName,
// p.Title AS PostTitle,
// p.CreationDate AS PostCreationDate,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
// pt.Name AS PostTypeName,
// STRING_AGG(DISTINCT t.TagName, ', ') AS Tags,
// MAX(b.Date) AS LastBadgeDate,
// p.LastActivityDate AS LastActivityDate
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
// Badges b ON u.Id = b.UserId
// LEFT JOIN
// Tags t ON t.ExcerptPostId = p.Id
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// AND u.Reputation > 50
// GROUP BY
// u.DisplayName, p.Title, p.CreationDate, pt.Name, p.LastActivityDate
// HAVING
// COUNT(c.Id) > 5
// ORDER BY
// LastActivityDate DESC, UpVoteCount DESC;
fn q9066(db: &'static So) -> String {
    let Post { title, creation_date, last_activity_date, owner_user, .. } = &db.post;
    let key = || owner_user.select(&db.user.display_name).and(title.opt()).and(creation_date).and(name(db)).and(last_activity_date);
    let base = || owned_since(db, year_ago()).with(owner_user.select(&db.user.reputation).gt(50));
    let sf = stats_fold(db, base(), key(), "cvb", &[]);
    let ex: HashIdx<Id<Post>, Id<Tag>> = (&db.tag.excerpt_post).inv().collect();
    let tags = base().group_by(key()).select((&ex).select(&db.tag.tag_name)).buf_fold(|ts| &*Box::leak(ts.into_vec().into_boxed_slice()));
    let mut v = Vec::new();
    (&sf).filt(|s: Stats| s.cx > 5).and((&tags).opt()).drive(|k, (s, t)| v.push((k, s, t)));
    rows(v.iter().map(|&(((((n, t), cd), ty), la), s, tg)| {
        let tg = tg.map(|ts: &[Str]| {
            let mut d: Vec<Str> = ts.to_vec();
            d.sort();
            d.dedup();
            leak_join(d, ", ")
        });
        row(vec![V::S(n), ostr(t), V::T(cd), V::I(s.cx), V::I(s.up), V::I(s.down), V::S(ty), ostr(tg), stat_field(&s, "bmax").unwrap(), V::T(la)])
    }))
}

// WITH UserBadges AS (
// SELECT UserId, COUNT(*) AS TotalBadges
// FROM Badges
// GROUP BY UserId
// ),
// TopUsers AS (
// SELECT U.Id, U.DisplayName, U.Reputation, UB.TotalBadges
// FROM Users U
// LEFT JOIN UserBadges UB ON U.Id = UB.UserId
// ORDER BY U.Reputation DESC, UB.TotalBadges DESC
// LIMIT 10
// ),
// PostStatistics AS (
// SELECT P.OwnerUserId, COUNT(P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(P.ViewCount) AS TotalViews
// FROM Posts P
// GROUP BY P.OwnerUserId
// ),
// UserPostStats AS (
// SELECT U.Id, U.DisplayName, PS.TotalPosts, PS.TotalQuestions, PS.TotalAnswers, PS.TotalViews
// FROM Users U
// JOIN PostStatistics PS ON U.Id = PS.OwnerUserId
// WHERE U.Id IN (SELECT Id FROM TopUsers)
// )
// SELECT U.DisplayName, U.Reputation, U.TotalBadges,
// UPS.TotalPosts, UPS.TotalQuestions, UPS.TotalAnswers, UPS.TotalViews
// FROM UserBadges UB
// JOIN UserPostStats UPS ON UB.UserId = UPS.Id
// JOIN TopUsers U ON U.Id = UPS.Id
// ORDER BY U.Reputation DESC;
fn q9100(db: &'static So) -> String {
    let bf = db.badge.group_by(&db.badge.user).fold(0i64, |a, _| a + 1);
    let top: MatSet<Id<User>> = whole(&db.user.id)
        .select(Ident::<User>::new().and(&db.user.reputation).and((&bf).opt()))
        .window(row_number, |((_, r), b)| (r, b), desc)
        .filt(|(_, n)| n <= 10)
        .map(|(((u, _), _), _)| u)
        .collect();
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    (&top).select(Ident::<User>::new().and(&bf).and(&ps)).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((u, b), p)| row(vec![user_col(db, u, "name"), user_col(db, u, "rep"), V::I(b), V::I(p[0]), V::I(p[1]), V::I(p[2]), pviews(p)])))
}

// WITH UserBadges AS (
// SELECT
// u.Id AS UserId,
// COUNT(b.Id) AS TotalBadges,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
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
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(p.ViewCount) AS TotalViews,
// SUM(p.Score) AS TotalScore
// FROM
// Posts p
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY
// p.OwnerUserId
// ),
// CombinedStats AS (
// SELECT
// ub.UserId,
// ub.TotalBadges,
// ub.GoldBadges,
// ub.SilverBadges,
// ub.BronzeBadges,
// ps.TotalPosts,
// ps.Questions,
// ps.Answers,
// ps.TotalViews,
// ps.TotalScore
// FROM
// UserBadges ub
// LEFT JOIN
// PostStats ps ON ub.UserId = ps.OwnerUserId
// )
// SELECT
// u.DisplayName,
// cs.TotalBadges,
// cs.GoldBadges,
// cs.SilverBadges,
// cs.BronzeBadges,
// cs.TotalPosts,
// cs.Questions,
// cs.Answers,
// cs.TotalViews,
// cs.TotalScore
// FROM
// CombinedStats cs
// JOIN
// Users u ON cs.UserId = u.Id
// WHERE
// cs.TotalPosts > 5
// ORDER BY
// cs.TotalScore DESC,
// cs.TotalBadges DESC
// LIMIT 10;
fn q9142(db: &'static So) -> String {
    let ub = ubc(db);
    let ps = pstat(db, since(db, year_ago()));
    let mut v = Vec::new();
    (&ub).and((&ps).filt(|p: [i64; 13]| p[0] > 5)).drive(|u, (b, p)| v.push((u, b, p)));
    out(v, |&(_, b, p)| (Reverse(p[3]), Reverse(b[0])), 10, |&(u, b, p)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(ints(&b));
        f.extend(ints(&p[..3]));
        f.extend([pviews(p), V::I(p[3])]);
        f
    })
}

// WITH UserBadges AS (
// SELECT U.Id AS UserId, COUNT(B.Id) AS BadgeCount, SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM Users U
// LEFT JOIN Badges B ON U.Id = B.UserId
// GROUP BY U.Id
// ),
// PostStatistics AS (
// SELECT P.OwnerUserId, COUNT(P.Id) AS PostCount, SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount, SUM(P.ViewCount) AS TotalViews
// FROM Posts P
// GROUP BY P.OwnerUserId
// ),
// UserActivity AS (
// SELECT U.Id AS UserId, U.DisplayName, U.Reputation, U.LastAccessDate,
// COALESCE(UB.BadgeCount, 0) AS BadgeCount, COALESCE(PS.PostCount, 0) AS PostCount,
// COALESCE(PS.QuestionCount, 0) AS QuestionCount, COALESCE(PS.AnswerCount, 0) AS AnswerCount,
// COALESCE(PS.TotalViews, 0) AS TotalViews, UB.GoldBadges, UB.SilverBadges, UB.BronzeBadges
// FROM Users U
// LEFT JOIN UserBadges UB ON U.Id = UB.UserId
// LEFT JOIN PostStatistics PS ON U.Id = PS.OwnerUserId
// )
// SELECT U.UserId, U.DisplayName, U.Reputation, U.LastAccessDate, U.BadgeCount,
// U.PostCount, U.QuestionCount, U.AnswerCount, U.TotalViews,
// U.GoldBadges, U.SilverBadges, U.BronzeBadges
// FROM UserActivity U
// WHERE (U.Reputation > 100 OR U.BadgeCount > 0)
// ORDER BY U.Reputation DESC, U.PostCount DESC
// LIMIT 50;
fn q9149(db: &'static So) -> String {
    let ub = ubc(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    db.user
        .select(Ident::<User>::new().and(&ub).and((&ps).opt()))
        .filt(|((u, b), _): ((Id<User>, [i64; 4]), Option<[i64; 13]>)| db.user.reputation.get(u).unwrap() > 100 || b[0] > 0)
        .drive(|_, ((u, b), p)| v.push((u, b, p.unwrap_or(Z))));
    out(v, |&(u, _, p)| (rep_desc(db, u), Reverse(p[0])), 50, |&(u, b, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep"), user_col(db, u, "last_access"), V::I(b[0])];
        f.extend(ints(&[p[0], p[1], p[2], p[5]]));
        f.extend(ints(&b[1..]));
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
// PopularPosts AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.OwnerUserId,
// P.Score,
// P.ViewCount,
// COUNT(C.Id) AS CommentCount
// FROM Posts P
// LEFT JOIN Comments C ON P.Id = C.PostId
// WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY P.Id, P.Title, P.OwnerUserId, P.Score, P.ViewCount
// HAVING COUNT(C.Id) > 10
// ORDER BY P.Score DESC
// LIMIT 5
// )
// SELECT
// U.DisplayName,
// PB.Title,
// PB.Score,
// PB.ViewCount,
// UBad.BadgeCount,
// UBad.GoldBadges,
// UBad.SilverBadges,
// UBad.BronzeBadges
// FROM PopularPosts PB
// JOIN Users U ON PB.OwnerUserId = U.Id
// JOIN UserBadges UBad ON U.Id = UBad.UserId
// ORDER BY PB.Score DESC, UBad.BadgeCount DESC;
fn q9196(db: &'static So) -> String {
    let ub = ubc(db);
    let base = since(db, year_ago()).with(comments_per_post(db).filt(|c: i64| c > 10));
    let pp: MatSet<Id<Post>> = whole(&base).select(Ident::<Post>::new().and(&db.post.score)).window(row_number, |(_, s)| s, desc).filt(|(_, n)| n <= 5).map(|((p, _), _)| p).collect();
    let mut v = Vec::new();
    (&pp).select(Ident::<Post>::new().and((&db.post.owner_user).select(Ident::<User>::new().and(&ub)))).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(p, (u, b))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title", "score", "views"]));
        f.extend(ints(&b));
        row(f)
    }))
}

// WITH PostStatistics AS (
// SELECT
// P.Id AS PostId,
// P.PostTypeId,
// P.Title,
// COUNT(DISTINCT C.Id) AS CommentCount,
// COUNT(DISTINCT PH.Id) AS EditHistoryCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
// (SELECT COUNT(*) FROM Votes WHERE PostId = P.Id AND VoteTypeId IN (2, 3)) AS TotalVotes,
// MAX(PH.CreationDate) AS LastEditDate
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// PostHistory PH ON P.Id = PH.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// P.Id, P.PostTypeId, P.Title
// ),
// FilteredPosts AS (
// SELECT
// PS.PostId,
// PS.Title,
// PS.PostTypeId,
// PS.CommentCount,
// PS.EditHistoryCount,
// PS.UpVoteCount,
// PS.DownVoteCount,
// PS.TotalVotes,
// PS.LastEditDate
// FROM
// PostStatistics PS
// WHERE
// PS.CommentCount > 5 AND PS.UpVoteCount > PS.DownVoteCount
// )
// SELECT
// F.PostId,
// F.Title,
// F.CommentCount,
// F.EditHistoryCount,
// F.UpVoteCount,
// F.DownVoteCount,
// F.LastEditDate
// FROM
// FilteredPosts F
// JOIN
// Users U ON U.Id = (SELECT OwnerUserId FROM Posts WHERE Id = F.PostId)
// WHERE
// U.Reputation > 1000
// ORDER BY
// F.UpVoteCount DESC, F.CommentCount DESC
// LIMIT 10;
fn q9241(db: &'static So) -> String {
    let base = owned(db).with((&db.post.owner_user).select(&db.user.reputation).gt(1000)).with(comments_per_post(db).filt(|c: i64| c > 5));
    let mut v = Vec::new();
    stats_fold(db, base, Ident::<Post>::new(), "cvh", &[])
        .filt(|s: Stats| s.up > s.down)
        .and(comments_per_post(db))
        .and(history_per_post(db))
        .drive(|p, ((s, c), h)| v.push((p, s, c, h)));
    out(v, |&(_, s, c, _)| (Reverse(s.up), Reverse(c)), 10, |&(p, s, c, h)| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([V::I(c), V::I(h), V::I(s.up), V::I(s.down), stat_field(&s, "hmax").unwrap()]);
        f
    })
}

// WITH UserBadges AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(B.Id) AS BadgeCount
// FROM Users U
// LEFT JOIN Badges B ON U.Id = B.UserId
// GROUP BY U.Id, U.DisplayName
// ),
// PostStatistics AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(P.ViewCount) AS TotalViews,
// AVG(P.Score) AS AverageScore
// FROM Posts P
// GROUP BY P.OwnerUserId
// ),
// BadgeStatistics AS (
// SELECT
// AUD.UserId,
// AUD.BadgeCount,
// PS.TotalPosts,
// PS.TotalQuestions,
// PS.TotalAnswers,
// PS.TotalViews,
// PS.AverageScore,
// CASE
// WHEN AUD.BadgeCount >= 5 THEN 'High Achiever'
// WHEN AUD.BadgeCount BETWEEN 3 AND 4 THEN 'Moderate Achiever'
// ELSE 'Novice'
// END AS AchievementLevel
// FROM UserBadges AUD
// JOIN PostStatistics PS ON AUD.UserId = PS.OwnerUserId
// )
// SELECT
// U.DisplayName,
// BS.BadgeCount,
// BS.TotalPosts,
// BS.TotalQuestions,
// BS.TotalAnswers,
// BS.TotalViews,
// BS.AverageScore,
// BS.AchievementLevel
// FROM BadgeStatistics BS
// JOIN Users U ON BS.UserId = U.Id
// WHERE BS.TotalPosts > 10
// ORDER BY BS.TotalViews DESC, BS.BadgeCount DESC;
fn q9289(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    (&ps).filt(|p: [i64; 13]| p[0] > 10).and(&bu).drive(|u, (p, b)| v.push((u, p, b)));
    rows(v.iter().map(|&(u, p, b)| {
        let mut f = vec![user_col(db, u, "name"), V::I(b)];
        f.extend(ints(&p[..3]));
        f.extend([pviews(p), pscore_avg(p)]);
        f.push(V::S(if b >= 5 {
            "High Achiever"
        } else if (3..=4).contains(&b) {
            "Moderate Achiever"
        } else {
            "Novice"
        }));
        row(f)
    }))
}

// WITH UserPostStats AS (
// SELECT
// Users.Id AS UserId,
// Users.DisplayName,
// COUNT(Posts.Id) AS TotalPosts,
// SUM(CASE WHEN Posts.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN Posts.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN Posts.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers,
// SUM(Posts.ViewCount) AS TotalViews,
// SUM(Posts.Score) AS TotalScore
// FROM Users
// LEFT JOIN Posts ON Users.Id = Posts.OwnerUserId
// GROUP BY Users.Id, Users.DisplayName
// ),
// UserBadges AS (
// SELECT
// UserId,
// COUNT(CASE WHEN Class = 1 THEN 1 END) AS GoldBadges,
// COUNT(CASE WHEN Class = 2 THEN 1 END) AS SilverBadges,
// COUNT(CASE WHEN Class = 3 THEN 1 END) AS BronzeBadges
// FROM Badges
// GROUP BY UserId
// ),
// CombinedStats AS (
// SELECT
// u.UserId,
// u.DisplayName,
// u.TotalPosts,
// u.TotalQuestions,
// u.TotalAnswers,
// u.AcceptedAnswers,
// u.TotalViews,
// u.TotalScore,
// b.GoldBadges,
// b.SilverBadges,
// b.BronzeBadges
// FROM UserPostStats u
// LEFT JOIN UserBadges b ON u.UserId = b.UserId
// )
// SELECT
// UserId,
// DisplayName,
// TotalPosts,
// TotalQuestions,
// TotalAnswers,
// AcceptedAnswers,
// TotalViews,
// TotalScore,
// GoldBadges,
// SilverBadges,
// BronzeBadges
// FROM CombinedStats
// WHERE TotalPosts > 5
// ORDER BY TotalScore DESC, TotalViews DESC
// LIMIT 10;
fn q9318(db: &'static So) -> String {
    let Post { post_type_id, view_count, score, accepted_answer_id, .. } = &db.post;
    let uf = g(db).select(posts_of(db).select(post_type_id.and(view_count.opt()).and(score).and(accepted_answer_id.opt())).opt()).fold([0i64; 7], |a, p| match p {
        Some((((t, w), s), ac)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + ac.is_some() as i64, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0), a[6] + s],
        None => a,
    });
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&uf).filt(|a: [i64; 7]| a[0] > 5).and((&bc).opt()).drive(|u, (a, b)| v.push((u, a, b)));
    out(v, |&(_, a, _)| (Reverse(a[6]), (a[4] == 0, Reverse(a[5]))), 10, |&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[..4]));
        f.extend([nullable(a[5], a[4]), V::I(a[6])]);
        f.extend((1..4).map(|i| oint(b.map(|b| b[i]))));
        f
    })
}

// WITH RecentPosts AS (
// SELECT p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, u.DisplayName AS OwnerDisplayName
// FROM Posts p
// JOIN Users u ON p.OwnerUserId = u.Id
// WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'
// AND p.PostTypeId = 1
// ),
// TagStatistics AS (
// SELECT
// t.TagName,
// COUNT(DISTINCT p.Id) AS PostCount,
// AVG(p.ViewCount) AS AvgViews,
// AVG(p.Score) AS AvgScore
// FROM Tags t
// JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%'
// GROUP BY t.TagName
// ),
// PopularTags AS (
// SELECT TagName, PostCount, AvgViews, AvgScore
// FROM TagStatistics
// WHERE PostCount > 5
// ORDER BY AvgScore DESC
// LIMIT 10
// )
// SELECT
// rp.Title,
// rp.OwnerDisplayName,
// rp.CreationDate,
// rp.Score,
// rp.ViewCount,
// pt.TagName,
// pt.PostCount,
// pt.AvgViews,
// pt.AvgScore
// FROM RecentPosts rp
// JOIN PopularTags pt ON rp.Title LIKE '%' || pt.TagName || '%'
// ORDER BY rp.CreationDate DESC, pt.AvgScore DESC;
fn q9319(db: &'static So) -> String {
    let tm = tag_mentions(db);
    let tf = (&tm)
        .group_by(Same::<(Id<Post>, Id<Tag>)>::new().map(|(_, t)| t))
        .select(Same::<(Id<Post>, Id<Tag>)>::new().map(|(p, _)| p).select((&db.post.view_count).opt().and(&db.post.score)))
        .fold([0i64; 4], |a, (w, s)| [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s]);
    let base = db.tag.with((&tf).filt(|a: [i64; 4]| a[0] > 5));
    let top: MatSet<Id<Tag>> = whole(&base)
        .select(Ident::<Tag>::new().and(&tf))
        .window(row_number, |(_, a): (Id<Tag>, [i64; 4])| fkey(a[3] as f64 / a[0] as f64), desc)
        .filt(|(_, n)| n <= 10)
        .map(|((t, _), _)| t)
        .collect();
    let tn: HashIdx<Str, Id<Tag>> = (&top).select(&db.tag.tag_name).inv().collect();
    let rp = || owned_since(db, month_ago()).with((&db.post.post_type_id).eq(1));
    let titles: MatSet<Str> = rp().select(&db.post.title).collect();
    let m: HashIdx<Str, Id<Tag>> = (&titles).select_where(&tn, |t: Str, n: Str| t.contains(n)).collect();
    let mut v = Vec::new();
    rp().select(Ident::<Post>::new().and((&db.post.title).select(&m).select(Ident::<Tag>::new().and(&tf)))).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(p, (t, a))| {
        let mut f = post_fields(db, p, &["title", "owner", "created", "score", "views"]);
        f.extend([V::S(db.tag.tag_name.get(t).unwrap()), V::I(a[0]), avg(a[2], a[1]), avg(a[3], a[0])]);
        row(f)
    }))
}

// WITH UserBadges AS (
// SELECT UserId, COUNT(*) AS BadgeCount
// FROM Badges
// WHERE Class = 1
// GROUP BY UserId
// ),
// TopUsers AS (
// SELECT u.Id, u.DisplayName, u.Reputation, ub.BadgeCount
// FROM Users u
// LEFT JOIN UserBadges ub ON u.Id = ub.UserId
// WHERE u.Reputation > 1000 AND ub.BadgeCount IS NOT NULL
// ORDER BY u.Reputation DESC
// LIMIT 10
// ),
// PopularPosts AS (
// SELECT p.Id, p.Title, p.Score, p.ViewCount, p.OwnerUserId, p.CreationDate
// FROM Posts p
// JOIN TopUsers tu ON p.OwnerUserId = tu.Id
// WHERE p.PostTypeId = 1
// ORDER BY p.Score DESC, p.ViewCount DESC
// LIMIT 5
// ),
// RecentComments AS (
// SELECT c.PostId, COUNT(c.Id) AS CommentCount
// FROM Comments c
// WHERE c.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'
// GROUP BY c.PostId
// )
// SELECT pp.Title, pp.Score, pp.ViewCount, tu.DisplayName, rc.CommentCount
// FROM PopularPosts pp
// JOIN TopUsers tu ON pp.OwnerUserId = tu.Id
// LEFT JOIN RecentComments rc ON pp.Id = rc.PostId
// ORDER BY pp.Score DESC, rc.CommentCount DESC;
fn q9329(db: &'static So) -> String {
    let gold = db.badge.with((&db.badge.class).eq(1)).group_by(&db.badge.user).fold(0i64, |a, _| a + 1);
    let base = user_base(db, UserWhere::RepGt(1000)).with(&gold);
    let top: MatSet<Id<User>> = whole(&base).select(Ident::<User>::new().and(&db.user.reputation)).window(row_number, |(_, r)| r, desc).filt(|(_, n)| n <= 10).map(|((u, _), _)| u).collect();
    let cand: MatSet<Id<Post>> = (&top).select(posts_of(db).select(Ident::<Post>::new().with((&db.post.post_type_id).eq(1)))).collect();
    let pp: MatSet<Id<Post>> = whole(&cand)
        .select(Ident::<Post>::new().and(&db.post.score).and((&db.post.view_count).opt()))
        .window(row_number, |((_, s), w)| (s, w), desc)
        .filt(|(_, n)| n <= 5)
        .map(|(((p, _), _), _)| p)
        .collect();
    let rc = db.comment.with((&db.comment.creation_date).gt(month_ago())).group_by(&db.comment.post).fold(0i64, |a, _| a + 1);
    let mut v = Vec::new();
    (&pp).select(Ident::<Post>::new().and((&rc).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(p, c)| {
        let mut f = post_fields(db, p, &["title", "score", "views", "owner"]);
        f.push(oint(c));
        row(f)
    }))
}

// WITH AvgUserReputation AS (
// SELECT AVG(Reputation) AS AvgReputation
// FROM Users
// ),
// TopPosts AS (
// SELECT p.Id, p.Title, p.Score, p.ViewCount, p.CreationDate, u.DisplayName AS OwnerDisplayName
// FROM Posts p
// JOIN Users u ON p.OwnerUserId = u.Id
// WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// ORDER BY p.Score DESC
// LIMIT 5
// ),
// RecentPostHistory AS (
// SELECT ph.PostId, p.Title, p.CreationDate, p.OwnerDisplayName, p.ViewCount, p.Score,
// COUNT(*) AS EditCount, MIN(ph.CreationDate) AS FirstEditDate
// FROM PostHistory ph
// JOIN Posts p ON ph.PostId = p.Id
// WHERE ph.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 month'
// GROUP BY ph.PostId, p.Title, p.CreationDate, p.OwnerDisplayName, p.ViewCount, p.Score
// ),
// BadgeCounts AS (
// SELECT UserId, COUNT(*) AS BadgeCount
// FROM Badges
// GROUP BY UserId
// )
// SELECT
// t.Title AS TopPostTitle,
// t.OwnerDisplayName AS TopPostOwner,
// t.Score AS TopPostScore,
// t.ViewCount AS TopPostViews,
// r.EditCount AS RecentEditCount,
// r.FirstEditDate AS FirstRecentEditDate,
// b.BadgeCount AS OwnerBadgeCount,
// a.AvgReputation AS AverageUserReputation
// FROM TopPosts t
// LEFT JOIN RecentPostHistory r ON t.Id = r.PostId
// LEFT JOIN BadgeCounts b ON b.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = t.Id)
// CROSS JOIN AvgUserReputation a;
fn q9337(db: &'static So) -> String {
    let u = db.user.select(&db.user.reputation).fold_flat([0i64; 2], |a, r| [a[0] + 1, a[1] + r]);
    let base = owned_since(db, year_ago());
    let top: MatSet<Id<Post>> = whole(&base).select(Ident::<Post>::new().and(&db.post.score)).window(row_number, |(_, s)| s, desc).filt(|(_, n)| n <= 5).map(|((p, _), _)| p).collect();
    let rh = db
        .post_history
        .with((&db.post_history.creation_date).ge(month_ago()))
        .group_by(&db.post_history.post)
        .select(&db.post_history.creation_date)
        .fold((0i64, i64::MAX), |(n, m), d| (n + 1, m.min(d)));
    let bf = db.badge.group_by(&db.badge.user).fold(0i64, |a, _| a + 1);
    let mut v = Vec::new();
    (&top).select(Ident::<Post>::new().and((&rh).opt()).and((&db.post.owner_user).select(&bf).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((p, r), b)| {
        let mut f = post_fields(db, p, &["title", "owner", "score", "views"]);
        f.extend([oint(r.map(|r| r.0)), ots(r.map(|r| r.1)), oint(b), avg(u[1], u[0])]);
        row(f)
    }))
}

// WITH PostStatistics AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// U.DisplayName AS OwnerDisplayName,
// COUNT(C.Id) AS CommentCount,
// COUNT(V.Id) FILTER (WHERE V.VoteTypeId = 2) AS UpVotes,
// COUNT(V.Id) FILTER (WHERE V.VoteTypeId = 3) AS DownVotes,
// COUNT(B.Id) AS BadgeCount
// FROM Posts P
// LEFT JOIN Users U ON P.OwnerUserId = U.Id
// LEFT JOIN Comments C ON P.Id = C.PostId
// LEFT JOIN Votes V ON P.Id = V.PostId
// LEFT JOIN Badges B ON U.Id = B.UserId
// WHERE P.PostTypeId = 1
// GROUP BY P.Id, P.Title, P.CreationDate, U.DisplayName
// ),
// PostHistoryAnalysis AS (
// SELECT
// PH.PostId,
// MAX(PH.CreationDate) AS LastEditDate,
// COUNT(CASE WHEN PH.PostHistoryTypeId IN (10, 11) THEN 1 END) AS CloseReopenCount,
// COUNT(CASE WHEN PH.PostHistoryTypeId = 24 THEN 1 END) AS SuggestedEditCount
// FROM PostHistory PH
// GROUP BY PH.PostId
// )
// SELECT
// PS.PostId,
// PS.Title,
// PS.OwnerDisplayName,
// PS.CommentCount,
// PS.UpVotes,
// PS.DownVotes,
// COALESCE(PH.LastEditDate, '1970-01-01') AS LastEditDate,
// COALESCE(PH.CloseReopenCount, 0) AS CloseReopenCount,
// COALESCE(PH.SuggestedEditCount, 0) AS SuggestedEditCount,
// (SELECT COUNT(*) FROM Votes V WHERE V.PostId = PS.PostId AND V.VoteTypeId = 1) AS AcceptedAnswerCount,
// (SELECT COUNT(*) FROM PostLinks PL WHERE PL.PostId = PS.PostId) AS RelatedPostCount
// FROM PostStatistics PS
// LEFT JOIN PostHistoryAnalysis PH ON PS.PostId = PH.PostId
// ORDER BY PS.UpVotes DESC, PS.CommentCount DESC
// FETCH FIRST 100 ROWS ONLY;
fn q9353(db: &'static So) -> String {
    let ph = db
        .post_history
        .group_by(&db.post_history.post)
        .select((&db.post_history.creation_date).and(&db.post_history.post_history_type_id))
        .fold([i64::MIN, 0, 0], |a: [i64; 3], (d, t)| [a[0].max(d), a[1] + matches!(t, 10 | 11) as i64, a[2] + (t == 24) as i64]);
    let lc = db.post_link.group_by(&db.post_link.post).fold(0i64, |a, _| a + 1);
    let mut v = Vec::new();
    stats_fold(db, questions_only(db), Ident::<Post>::new(), "cvb", &[]).and((&ph).opt()).and(votes_of_type(db, 1)).and((&lc).opt()).drive(|p, (((s, h), a), l)| v.push((p, s, h, a, l)));
    out(v, |&(_, s, _, _, _)| (Reverse(s.up), Reverse(s.cx)), 100, |&(p, s, h, a, l)| {
        let h = h.unwrap_or([0, 0, 0]);
        let mut f = post_fields(db, p, &["id", "title", "owner"]);
        f.extend([V::I(s.cx), V::I(s.up), V::I(s.down), V::T(h[0]), V::I(h[1]), V::I(h[2]), V::I(a), V::I(l.unwrap_or(0))]);
        f
    })
}

// WITH UserBadgeCounts AS (
// SELECT
// U.Id AS UserId,
// COUNT(B.Id) AS BadgeCount,
// SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM Users U
// LEFT JOIN Badges B ON U.Id = B.UserId
// GROUP BY U.Id
// ),
// PostStats AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS PostCount,
// SUM(P.Score) AS TotalScore,
// SUM(P.ViewCount) AS TotalViews,
// COUNT(C.Id) AS CommentCount
// FROM Posts P
// LEFT JOIN Comments C ON P.Id = C.PostId
// GROUP BY P.OwnerUserId
// ),
// CombinedStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COALESCE(UBC.BadgeCount, 0) AS BadgeCount,
// COALESCE(UBC.GoldBadges, 0) AS GoldBadges,
// COALESCE(UBC.SilverBadges, 0) AS SilverBadges,
// COALESCE(UBC.BronzeBadges, 0) AS BronzeBadges,
// COALESCE(PS.PostCount, 0) AS PostCount,
// COALESCE(PS.TotalScore, 0) AS TotalScore,
// COALESCE(PS.TotalViews, 0) AS TotalViews,
// COALESCE(PS.CommentCount, 0) AS CommentCount
// FROM Users U
// LEFT JOIN UserBadgeCounts UBC ON U.Id = UBC.UserId
// LEFT JOIN PostStats PS ON U.Id = PS.OwnerUserId
// )
// SELECT
// UserId,
// DisplayName,
// BadgeCount,
// GoldBadges,
// SilverBadges,
// BronzeBadges,
// PostCount,
// TotalScore,
// TotalViews,
// CommentCount
// FROM CombinedStats
// ORDER BY TotalScore DESC, PostCount DESC
// LIMIT 10;
fn q9354(db: &'static So) -> String {
    let ub = ubc(db);
    let ps = owned(db).group_by(&db.post.owner_user).select((&db.post.score).and((&db.post.view_count).opt()).and(comments_of(db).opt())).fold([0i64; 4], |a, ((s, w), c)| {
        [a[0] + 1, a[1] + s, a[2] + w.unwrap_or(0), a[3] + c.is_some() as i64]
    });
    let mut v = Vec::new();
    (&ub).and((&ps).opt()).drive(|u, (b, p)| v.push((u, b, p.unwrap_or([0; 4]))));
    out(v, |&(_, _, p)| (Reverse(p[1]), Reverse(p[0])), 10, |&(u, b, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&b));
        f.extend(ints(&p));
        f
    })
}

pub static ENTRIES: &[harness::Entry] = &[
    ("7385", q7385),
    ("7389", q7389),
    ("7447", q7447),
    ("7524", q7524),
    ("757", q757),
    ("7593", q7593),
    ("7613", q7613),
    ("7660", q7660),
    ("7688", q7688),
    ("7717", q7717),
    ("7827", q7827),
    ("7836", q7836),
    ("7877", q7877),
    ("7902", q7902),
    ("7903", q7903),
    ("7939", q7939),
    ("7940", q7940),
    ("7958", q7958),
    ("8011", q8011),
    ("8026", q8026),
    ("8067", q8067),
    ("8075", q8075),
    ("8079", q8079),
    ("8107", q8107),
    ("8130", q8130),
    ("8209", q8209),
    ("8261", q8261),
    ("8277", q8277),
    ("8281", q8281),
    ("8288", q8288),
    ("8408", q8408),
    ("8507", q8507),
    ("8623", q8623),
    ("8632", q8632),
    ("8700", q8700),
    ("8731", q8731),
    ("8739", q8739),
    ("8748", q8748),
    ("8878", q8878),
    ("8887", q8887),
    ("8933", q8933),
    ("8942", q8942),
    ("8955", q8955),
    ("8966", q8966),
    ("9025", q9025),
    ("9066", q9066),
    ("9100", q9100),
    ("9142", q9142),
    ("9149", q9149),
    ("9196", q9196),
    ("9241", q9241),
    ("9289", q9289),
    ("9318", q9318),
    ("9319", q9319),
    ("9329", q9329),
    ("9337", q9337),
    ("9353", q9353),
    ("9354", q9354),
];
