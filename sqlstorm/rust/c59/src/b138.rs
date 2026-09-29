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

fn uvotes(db: &'static So) -> Fold<Id<User>, [i64; 3]> {
    db.vote.group_by(&db.vote.user).select(&db.vote.vote_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64])
}

fn phu(db: &'static So) -> Fold<Id<User>, [i64; 8]> {
    db.post_history.group_by(&db.post_history.user).select(&db.post_history.post_history_type_id).fold([0i64; 8], |a, t| {
        [
            a[0] + 1,
            a[1] + matches!(t, 4 | 5 | 6) as i64,
            a[2] + matches!(t, 10 | 11) as i64,
            a[3] + (t == 10) as i64,
            a[4] + (t == 11) as i64,
            a[5] + (t == 12) as i64,
            a[6] + matches!(t, 24 | 25) as i64,
            a[7] + matches!(t, 4 | 6) as i64,
        ]
    })
}

fn top_by<K: Copy + Eq + std::hash::Hash + Ord + 'static>(f: &'static Fold<K, i64>, n: i64) -> MatSet<K> {
    whole(f).select(Same::new().and(f)).window(row_number, |(_, c): (K, i64)| c, desc).filt(move |(_, r)| r <= n).map(|((k, _), _)| k).collect()
}

fn pz(p: Option<[i64; 13]>) -> [i64; 13] {
    p.unwrap_or(Z)
}

fn bz(b: Option<[i64; 4]>) -> [i64; 4] {
    b.unwrap_or([0; 4])
}

fn class_sum(db: &'static So) -> Fold<Id<User>, (i64, i64)> {
    db.badge.group_by(&db.badge.user).select(&db.badge.class).fold((0i64, 0i64), |(n, s), c| (n + 1, s + c))
}

fn cut<X, T: Ord>(v: &mut Vec<X>, key: impl Fn(&X) -> T, n: usize) {
    v.sort_by_key(|x| key(x));
    if n < v.len() && key(&v[n - 1]) == key(&v[n]) {
        eprintln!("tie at the LIMIT cut");
    }
    v.truncate(n);
}

fn chars(s: Str) -> i64 {
    s.chars().count() as i64
}

fn tag_n(s: Str) -> i64 {
    s.matches('>').count() as i64 + 1
}

fn udc(db: &'static So) -> Fold<Id<User>, i64> {
    owned(db).group_by(&db.post.owner_user).select(comments_per_post(db)).fold(0i64, |a, c| a + c)
}

fn pcc(db: &'static So) -> Fold<Id<User>, [i64; 3]> {
    let Post { comment_count, answer_count, .. } = &db.post;
    owned(db).group_by(&db.post.owner_user).select(comment_count.and(answer_count.opt())).fold([0i64; 3], |a, (c, an)| [a[0] + c, a[1] + an.is_some() as i64, a[2] + an.unwrap_or(0)])
}

fn vid_sum(db: &'static So) -> Fold<Id<Post>, (i64, i64)> {
    db.vote.group_by(&db.vote.post).select(&db.vote.origid).fold((0i64, 0i64), |(n, s), i| (n + 1, s + i))
}

fn named_qa(db: &'static So) -> Fold<Id<User>, [i64; 4]> {
    owned(db).group_by(&db.post.owner_user).select(name(db).and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 4], |a, (n, t)| {
        [a[0] + (n == "Question") as i64, a[1] + (n == "Answer") as i64, a[2] + (t == Some(2)) as i64, a[3] + (t == Some(3)) as i64]
    })
}

fn epoch_avg(p: [i64; 13], e: i128) -> V {
    if p[0] == 0 { V::Null } else { V::F(e as f64 / p[0] as f64 / 1e6) }
}

fn pnq(db: &'static So) -> Fold<Id<User>, [i64; 6]> {
    let Post { post_type_id, score, .. } = &db.post;
    owned(db).group_by(&db.post.owner_user).select(post_type_id.and(score).and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 6], |a, ((t, s), v)| {
        [a[0] + (s > 0) as i64, a[1] + (s < 0) as i64, a[2] + (t == 1) as i64, a[3] + (t == 2) as i64, a[4] + (v == Some(2)) as i64, a[5] + (v == Some(3)) as i64]
    })
}

fn tmax(x: i64) -> V {
    if x == i64::MIN { V::Null } else { V::T(x) }
}

// --- batch 138 --------------------------------------------------------------

// WITH UserBadges AS (
// SELECT UserId,
// COUNT(*) AS TotalBadges,
// SUM(CASE WHEN Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM Badges
// GROUP BY UserId
// ),
// PostStatistics AS (
// SELECT OwnerUserId,
// COUNT(DISTINCT Id) AS TotalPosts,
// SUM(COALESCE(Score, 0)) AS TotalScore,
// SUM(COALESCE(ViewCount, 0)) AS TotalViews,
// AVG(COALESCE(AnswerCount, 0)) AS AverageAnswers
// FROM Posts
// GROUP BY OwnerUserId
// ),
// UserActivity AS (
// SELECT U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// U.Views,
// UB.TotalBadges,
// UB.GoldBadges,
// UB.SilverBadges,
// UB.BronzeBadges,
// PS.TotalPosts,
// PS.TotalScore,
// PS.TotalViews,
// PS.AverageAnswers
// FROM Users U
// LEFT JOIN UserBadges UB ON U.Id = UB.UserId
// LEFT JOIN PostStatistics PS ON U.Id = PS.OwnerUserId
// )
// SELECT UA.UserId,
// UA.DisplayName,
// COALESCE(UA.TotalPosts, 0) AS TotalPosts,
// COALESCE(UA.TotalScore, 0) AS TotalScore,
// COALESCE(UA.TotalViews, 0) AS TotalViews,
// UA.Reputation,
// UA.Views,
// COALESCE(UA.TotalBadges, 0) AS TotalBadges,
// COALESCE(UA.GoldBadges, 0) AS GoldBadges,
// COALESCE(UA.SilverBadges, 0) AS SilverBadges,
// COALESCE(UA.BronzeBadges, 0) AS BronzeBadges,
// CASE
// WHEN UA.Reputation >= 1000 THEN 'Active'
// WHEN UA.Reputation BETWEEN 500 AND 999 THEN 'Moderate'
// ELSE 'Inactive'
// END AS ActivityLevel
// FROM UserActivity UA
// ORDER BY UA.Reputation DESC
// LIMIT 20;
fn q3973(db: &'static So) -> String {
    let bc = badge_classes(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&bc).opt()).and((&ps).opt())).drive(|_, ((u, b), p)| v.push((u, bz(b), pz(p))));
    out(v, |&(u, ..)| rep_desc(db, u), 20, |&(u, b, p)| {
        let r = db.user.reputation.get(u).unwrap();
        let lvl = if r >= 1000 { "Active" } else if (500..=999).contains(&r) { "Moderate" } else { "Inactive" };
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&[p[0], p[3], p[5], r]));
        f.push(user_col(db, u, "uviews"));
        f.extend(ints(&b));
        f.push(V::S(lvl));
        f
    })
}

// WITH PostCounts AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN p.PostTypeId = 4 THEN 1 ELSE 0 END) AS TotalTagWikis
// FROM
// Posts p
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY
// p.OwnerUserId
// ), UserReputations AS (
// SELECT
// u.Id,
// u.Reputation,
// COALESCE(pc.TotalPosts, 0) AS TotalPosts,
// COALESCE(pc.TotalQuestions, 0) AS TotalQuestions,
// COALESCE(pc.TotalAnswers, 0) AS TotalAnswers,
// COALESCE(pc.TotalTagWikis, 0) AS TotalTagWikis
// FROM
// Users u
// LEFT JOIN
// PostCounts pc ON u.Id = pc.OwnerUserId
// ), BadgesSummary AS (
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
// ur.Id AS UserId,
// ur.Reputation,
// ur.TotalPosts,
// ur.TotalQuestions,
// ur.TotalAnswers,
// ur.TotalTagWikis,
// COALESCE(bs.TotalBadges, 0) AS TotalBadges,
// COALESCE(bs.GoldBadges, 0) AS GoldBadges,
// COALESCE(bs.SilverBadges, 0) AS SilverBadges,
// COALESCE(bs.BronzeBadges, 0) AS BronzeBadges,
// (ur.Reputation / NULLIF(ur.TotalPosts, 0)) AS ReputationPerPost
// FROM
// UserReputations ur
// LEFT JOIN
// BadgesSummary bs ON ur.Id = bs.UserId
// ORDER BY
// ur.Reputation DESC,
// ur.TotalPosts DESC
// LIMIT 100;
fn q6464(db: &'static So) -> String {
    let ps = pstat(db, since(db, year_ago()));
    let bc = badge_classes(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&ps).opt()).and((&bc).opt())).drive(|_, ((u, p), b)| v.push((u, pz(p), bz(b))));
    out(v, |&(u, p, _)| (rep_desc(db, u), Reverse(p[0])), 100, |&(u, p, b)| {
        let r = db.user.reputation.get(u).unwrap();
        let mut f = vec![user_col(db, u, "uid"), V::I(r)];
        f.extend(ints(&[p[0], p[1], p[2], p[12]]));
        f.extend(ints(&b));
        f.push(ratio(r, p[0]));
        f
    })
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount,
// SUM(COALESCE(p.Score, 0)) AS TotalScore
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// u.Id
// ),
// PostDetail AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// COALESCE(c.CommentCount, 0) AS CommentCount,
// COALESCE(b.BadgeCount, 0) AS BadgeCount,
// u.DisplayName AS OwnerDisplayName,
// p.OwnerUserId
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
// UserId,
// COUNT(*) AS BadgeCount
// FROM
// Badges
// GROUP BY
// UserId
// ) b ON p.OwnerUserId = b.UserId
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// )
// SELECT
// us.UserId,
// us.PostCount,
// us.QuestionCount,
// us.AnswerCount,
// us.UpvoteCount,
// us.DownvoteCount,
// us.TotalScore,
// pd.PostId,
// pd.Title,
// pd.CreationDate,
// pd.Score,
// pd.CommentCount,
// pd.BadgeCount,
// pd.OwnerDisplayName
// FROM
// UserStats us
// LEFT JOIN
// PostDetail pd ON us.UserId = pd.OwnerUserId
// ORDER BY
// us.TotalScore DESC, us.PostCount DESC;
fn q14662(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let cp = comments_per_post(db);
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    (&us).and((&dp).opt()).and(&bu).and(posts_of(db).select(Ident::<Post>::new().and(&cp)).opt()).drive(|u, (((a, d), b), p)| v.push((u, a, d.unwrap_or(0), b, p)));
    rows(v.iter().map(|&(u, a, d, b, p)| {
        let mut f = vec![user_col(db, u, "uid")];
        f.extend(ints(&[d, a.q, a.a, a.up, a.down, a.score_sum]));
        f.extend(match p {
            Some((p, c)) => {
                let mut g = post_fields(db, p, &["id", "title", "created", "score"]);
                g.extend([V::I(c), V::I(b), user_col(db, u, "name")]);
                g
            }
            None => nulls(7),
        });
        row(f)
    }))
}

// WITH User_Scores AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// COUNT(DISTINCT P.Id) AS TotalPosts,
// COUNT(DISTINCT C.Id) AS TotalComments
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
// ),
// Post_Statistics AS (
// SELECT
// P.Id AS PostId,
// P.Score AS PostScore,
// P.Title,
// P.CreationDate,
// P.LastActivityDate,
// P.AcceptedAnswerId,
// U.DisplayName AS OwnerDisplayName,
// P.ViewCount,
// COALESCE((SELECT COUNT(*) FROM Votes WHERE PostId = P.Id AND VoteTypeId = 2), 0) AS UpVotes,
// COALESCE((SELECT COUNT(*) FROM Votes WHERE PostId = P.Id AND VoteTypeId = 3), 0) AS DownVotes
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// WHERE
// P.PostTypeId = 1
// ),
// Top_Users AS (
// SELECT
// UserId,
// DisplayName,
// UpVotes - DownVotes AS NetVotes,
// TotalPosts,
// TotalComments
// FROM
// User_Scores
// WHERE
// TotalPosts > 0
// ORDER BY
// NetVotes DESC
// LIMIT 10
// )
// SELECT
// PU.PostId,
// PU.Title,
// PU.PostScore,
// PU.CreationDate,
// PU.OwnerDisplayName,
// PU.ViewCount,
// TU.DisplayName AS TopUserName,
// TU.NetVotes,
// TU.TotalPosts,
// TU.TotalComments
// FROM
// Post_Statistics PU
// JOIN
// Top_Users TU ON PU.OwnerDisplayName = TU.DisplayName
// ORDER BY
// PU.PostScore DESC,
// PU.ViewCount DESC;
fn q9380(db: &'static So) -> String {
    let pn = owned(db).group_by(&db.post.owner_user).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let cn = owned(db).group_by(&db.post.owner_user).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let uf = owned(db)
        .group_by(&db.post.owner_user)
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let uf = (&uf).and(&pn).and((&cn).opt()).map(|((a, p), c): (([i64; 2], i64), Option<i64>)| [a[0], a[1], p, c.unwrap_or(0)]).collect::<HashIdx<Id<User>, [i64; 4]>>();
    let top: MatSet<Id<User>> = whole(&uf)
        .select(Ident::<User>::new().and(&uf))
        .window(row_number, |(_, a): (Id<User>, [i64; 4])| a[0] - a[1], desc)
        .filt(|(_, n)| n <= 10)
        .map(|((u, _), _)| u)
        .collect();
    let names: HashIdx<Str, Id<User>> = (&top).select(&db.user.display_name).inv().collect();
    let mut v = Vec::new();
    questions_only(db)
        .with(&db.post.owner_user)
        .select(Ident::<Post>::new().and((&db.post.owner_user).select(&db.user.display_name).select(&names).select(Ident::<User>::new().and(&uf))))
        .drive(|_, x| v.push(x));
    rows(v.iter().map(|&(p, (u, a))| {
        let mut f = post_fields(db, p, &["id", "title", "score", "created", "owner", "views"]);
        f.extend([user_col(db, u, "name"), V::I(a[0] - a[1]), V::I(a[2]), V::I(a[3])]);
        row(f)
    }))
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(CASE WHEN p.PostTypeId = 3 THEN 1 ELSE 0 END) AS WikitPosts,
// SUM(CASE WHEN p.PostTypeId IN (4,5) THEN 1 ELSE 0 END) AS TagWikis,
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
// PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// COUNT(c.Id) AS CommentCount,
// COALESCE(ph.Comment, '') AS LastEditComment,
// ph.CreationDate AS LastEditDate
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId AND ph.CreationDate = (SELECT MAX(CreationDate) FROM PostHistory WHERE PostId = p.Id)
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, ph.Comment, ph.CreationDate
// )
// SELECT
// us.UserId,
// us.Reputation,
// us.PostCount,
// us.Questions,
// us.Answers,
// us.WikitPosts,
// us.TagWikis,
// us.UpVotes,
// us.DownVotes,
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.Score,
// ps.ViewCount,
// ps.CommentCount,
// ps.LastEditComment,
// ps.LastEditDate
// FROM
// UserStats us
// JOIN
// PostStats ps ON us.UserId = ps.PostId
// ORDER BY
// us.Reputation DESC, ps.ViewCount DESC
// LIMIT 100;
fn q14666(db: &'static So) -> String {
    let uid = uids(db);
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let hm = history_n_max(db);
    let PostHistory { post, creation_date, comment, .. } = &db.post_history;
    let latest = || db.post_history.with(post.select(&hm).and(creation_date).filt(|((_, m), d): ((i64, i64), i64)| m == d));
    let lk: MatSet<(Id<Post>, Option<Str>)> = latest().select(post.and(comment.opt())).collect();
    let lh: HashIdx<(Id<Post>, Option<Str>), Id<PostHistory>> = latest().select(post.and(comment.opt())).inv().collect();
    type K = (Id<Post>, Option<Str>);
    let lc = (&lk)
        .group_by(Same::<K>::new())
        .select(Same::<K>::new().map(|(p, _): K| p).select(comments_of(db).opt()).and((&lh).select(creation_date)))
        .fold((0i64, 0i64), |(n, _), (c, d)| (n + c.is_some() as i64, d));
    let li: HashIdx<Id<Post>, (Id<Post>, Option<Str>)> = (&lk).map(|(p, _)| p).inv().collect();
    let cp = comments_per_post(db);
    let mut v = Vec::new();
    db.post
        .select(Ident::<Post>::new().and(&cp).and((&li).select(Same::<(Id<Post>, Option<Str>)>::new().and(&lc)).opt()).and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&us).and((&dp).opt()))))
        .drive(|_, x| v.push(x));
    out(v, |&(((p, _), _), ((u, _), _))| (rep_desc(db, u), views_desc(db, p)), 100, |&(((p, c), h), ((u, a), d))| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep")];
        f.extend(ints(&[d.unwrap_or(0), a.q, a.a, a.t3, a.t45, a.up, a.down]));
        f.extend(post_fields(db, p, &["id", "title", "created", "score", "views"]));
        match h {
            Some(((_, cm), (k, dt))) => f.extend([V::I(k), V::S(cm.unwrap_or("")), V::T(dt)]),
            None => f.extend([V::I(c), V::S(""), V::Null]),
        }
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
// FROM Users U
// LEFT JOIN Badges B ON U.Id = B.UserId
// GROUP BY U.Id, U.DisplayName
// ),
// ActiveUserPosts AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS PostCount,
// SUM(P.ViewCount) AS TotalViews,
// SUM(P.Score) AS TotalScore,
// AVG(P.Score) AS AvgScore
// FROM Posts P
// WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY P.OwnerUserId
// ),
// CombinedData AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// U.Views AS UserViews,
// COALESCE(UPC.PostCount, 0) AS PostCount,
// COALESCE(UPC.TotalViews, 0) AS PostViews,
// COALESCE(UPC.TotalScore, 0) AS PostScore,
// COALESCE(UPC.AvgScore, 0) AS AvgPostScore,
// COALESCE(UBC.BadgeCount, 0) AS BadgeCount,
// COALESCE(UBC.GoldBadgeCount, 0) AS GoldBadgeCount,
// COALESCE(UBC.SilverBadgeCount, 0) AS SilverBadgeCount,
// COALESCE(UBC.BronzeBadgeCount, 0) AS BronzeBadgeCount
// FROM Users U
// LEFT JOIN ActiveUserPosts UPC ON U.Id = UPC.OwnerUserId
// LEFT JOIN UserBadgeCounts UBC ON U.Id = UBC.UserId
// )
// SELECT
// UserId,
// DisplayName,
// Reputation,
// UserViews,
// PostCount,
// PostViews,
// PostScore,
// AvgPostScore,
// BadgeCount,
// GoldBadgeCount,
// SilverBadgeCount,
// BronzeBadgeCount
// FROM CombinedData
// ORDER BY Reputation DESC, PostCount DESC, BadgeCount DESC
// LIMIT 100;
fn q6468(db: &'static So) -> String {
    let ps = pstat(db, since(db, year_ago()));
    let bc = badge_classes(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&ps).opt()).and((&bc).opt())).drive(|_, ((u, p), b)| v.push((u, pz(p), bz(b))));
    out(v, |&(u, p, b)| (rep_desc(db, u), Reverse(p[0]), Reverse(b[0])), 100, |&(u, p, b)| {
        let mut f = ["uid", "name", "rep", "uviews"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.extend(ints(&[p[0], p[5], p[3]]));
        f.push(or0(p[3], p[0]));
        f.extend(ints(&b));
        f
    })
}

// WITH UserBadges AS (
// SELECT
// b.UserId,
// COUNT(b.Id) AS BadgeCount,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM
// Badges b
// GROUP BY
// b.UserId
// ),
// UserScores AS (
// SELECT
// u.Id AS UserId,
// COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) - SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS NetScore
// FROM
// Users u
// LEFT JOIN
// Votes v ON u.Id = v.UserId
// GROUP BY
// u.Id
// ),
// PostStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS PostCount,
// SUM(p.ViewCount) AS TotalViews,
// AVG(p.Score) AS AvgScore
// FROM
// Posts p
// WHERE
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY
// p.OwnerUserId
// )
// SELECT
// u.DisplayName,
// COALESCE(ub.BadgeCount, 0) AS BadgeCount,
// COALESCE(ub.GoldBadges, 0) AS GoldBadges,
// COALESCE(ub.SilverBadges, 0) AS SilverBadges,
// COALESCE(ub.BronzeBadges, 0) AS BronzeBadges,
// us.TotalBounty,
// us.NetScore,
// ps.PostCount,
// ps.TotalViews,
// ps.AvgScore,
// CASE
// WHEN ps.AvgScore > 5 THEN 'High Engagement'
// WHEN ps.AvgScore BETWEEN 1 AND 5 THEN 'Moderate Engagement'
// ELSE 'Low Engagement'
// END AS EngagementLevel
// FROM
// Users u
// LEFT JOIN
// UserBadges ub ON u.Id = ub.UserId
// LEFT JOIN
// UserScores us ON u.Id = us.UserId
// LEFT JOIN
// PostStats ps ON u.Id = ps.OwnerUserId
// WHERE
// u.Reputation > 100
// ORDER BY
// us.NetScore DESC, us.TotalBounty DESC;
fn q4716(db: &'static So) -> String {
    let bc = badge_classes(db);
    let vf = db.vote.group_by(&db.vote.user).select((&db.vote.vote_type_id).and((&db.vote.bounty_amount).opt())).fold([0i64; 2], |a, (t, b)| [a[0] + b.unwrap_or(0), a[1] + (t == 2) as i64 - (t == 3) as i64]);
    let ps = pstat(db, since(db, year_ago()));
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(100)).select(Ident::<User>::new().and((&bc).opt()).and((&vf).opt()).and((&ps).opt())).drive(|_, (((u, b), x), p)| v.push((u, bz(b), x.unwrap_or([0; 2]), p)));
    rows(v.iter().map(|&(u, b, x, p)| {
        let av = p.map(|p| p[3] as f64 / p[0] as f64);
        let lvl = match av {
            Some(a) if a > 5.0 => "High Engagement",
            Some(a) if (1.0..=5.0).contains(&a) => "Moderate Engagement",
            _ => "Low Engagement",
        };
        let mut f = vec![user_col(db, u, "name")];
        f.extend(ints(&b));
        f.extend([V::I(x[0]), V::I(x[1])]);
        f.extend(p.map_or(nulls(3), |p| vec![V::I(p[0]), pviews(p), pscore_avg(p)]));
        f.push(V::S(lvl));
        row(f)
    }))
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges,
// SUM(COALESCE(p.ViewCount, 0)) AS TotalViews
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// WHERE
// u.Reputation > 1000
// GROUP BY
// u.Id, u.DisplayName
// ),
// PostHistoryStats AS (
// SELECT
// ph.UserId,
// COUNT(ph.Id) AS TotalEdits,
// COUNT(DISTINCT ph.PostId) AS TotalEditedPosts,
// MAX(ph.CreationDate) AS LastEditDate
// FROM
// PostHistory ph
// WHERE
// ph.PostHistoryTypeId IN (4, 5, 6, 10, 11)
// GROUP BY
// ph.UserId
// )
// SELECT
// us.UserId,
// us.DisplayName,
// us.TotalPosts,
// us.Questions,
// us.Answers,
// us.TotalUpvotes,
// us.TotalDownvotes,
// us.GoldBadges,
// us.SilverBadges,
// us.BronzeBadges,
// us.TotalViews,
// COALESCE(phs.TotalEdits, 0) AS TotalEdits,
// COALESCE(phs.TotalEditedPosts, 0) AS TotalEditedPosts,
// phs.LastEditDate
// FROM
// UserStats us
// LEFT JOIN
// PostHistoryStats phs ON us.UserId = phs.UserId
// ORDER BY
// us.TotalPosts DESC, us.TotalUpvotes DESC
// FETCH FIRST 50 ROWS ONLY;
fn q7510(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::RepGt(1000), "vb", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let hs = || db.post_history.with((&db.post_history.post_history_type_id).filt(|t: i64| matches!(t, 4 | 5 | 6 | 10 | 11)));
    let hn = hs().group_by(&db.post_history.user).select(&db.post_history.creation_date).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let hd = hs().group_by(&db.post_history.user).select(&db.post_history.post).count_distinct();
    let mut v = Vec::new();
    (&us).and((&dp).opt()).and((&hn).opt()).and((&hd).opt()).drive(|u, (((a, d), h), e)| v.push((u, a, d.unwrap_or(0), h, e.unwrap_or(0))));
    out(v, |&(_, a, d, ..)| (Reverse(d), Reverse(a.up)), 50, |&(u, a, d, h, e)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&[d, a.q, a.a, a.up, a.down, a.bcls[1], a.bcls[2], a.bcls[3], a.views_sum, h.map_or(0, |h| h.0), e]));
        f.push(ots(h.map(|h| h.1)));
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
// p.FavoriteCount,
// u.Reputation AS OwnerReputation,
// u.DisplayName AS OwnerDisplayName
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.CreationDate >= '2020-01-01'
// ),
// PostHistoryStats AS (
// SELECT
// ph.PostId,
// COUNT(*) AS EditCount,
// SUM(CASE WHEN ph.PostHistoryTypeId = 4 THEN 1 ELSE 0 END) AS TitleEdits,
// SUM(CASE WHEN ph.PostHistoryTypeId = 6 THEN 1 ELSE 0 END) AS TagEdits,
// SUM(CASE WHEN ph.PostHistoryTypeId = 5 THEN 1 ELSE 0 END) AS BodyEdits
// FROM
// PostHistory ph
// GROUP BY
// ph.PostId
// ),
// UserEngagement AS (
// SELECT
// p.Id AS PostId,
// COUNT(DISTINCT c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.Score,
// ps.ViewCount,
// ps.AnswerCount,
// ps.CommentCount + ue.CommentCount AS TotalComments,
// ps.FavoriteCount,
// ps.OwnerReputation,
// ps.OwnerDisplayName,
// COALESCE(phs.EditCount, 0) AS TotalEdits,
// COALESCE(phs.TitleEdits, 0) AS TitleEdits,
// COALESCE(phs.TagEdits, 0) AS TagEdits,
// COALESCE(phs.BodyEdits, 0) AS BodyEdits,
// COALESCE(ue.VoteCount, 0) AS TotalVotes
// FROM
// PostStats ps
// LEFT JOIN
// PostHistoryStats phs ON ps.PostId = phs.PostId
// LEFT JOIN
// UserEngagement ue ON ps.PostId = ue.PostId
// ORDER BY
// ps.Score DESC, ps.ViewCount DESC
// LIMIT 100;
fn q12561(db: &'static So) -> String {
    let ht = db.post_history.group_by(&db.post_history.post).select(&db.post_history.post_history_type_id).fold([0i64; 4], |a, t| [a[0] + 1, a[1] + (t == 4) as i64, a[2] + (t == 6) as i64, a[3] + (t == 5) as i64]);
    let cp = comments_per_post(db);
    let vp = votes_per_post(db);
    let mut v = Vec::new();
    owned_since(db, date(2020, 1, 1)).select(Ident::<Post>::new().and(&cp).and(&vp).and((&ht).opt())).drive(|_, (((p, c), x), h)| v.push((p, c, x, h.unwrap_or([0; 4]))));
    out(v, |&(p, ..)| score_views(db, p), 100, |&(p, c, x, h)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers"]);
        f.push(V::I(db.post.comment_count.get(p).unwrap() + c));
        f.extend(post_fields(db, p, &["favorites", "rep", "owner"]));
        f.extend(ints(&h));
        f.push(V::I(x));
        f
    })
}

// WITH TopUsers AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(DISTINCT P.Id) AS PostCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount
// FROM Users U
// JOIN Posts P ON U.Id = P.OwnerUserId
// GROUP BY U.Id, U.DisplayName, U.Reputation
// HAVING COUNT(DISTINCT P.Id) > 10
// ),
// RecentPosts AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// U.DisplayName AS OwnerDisplayName,
// P.ViewCount,
// P.Score
// FROM Posts P
// JOIN Users U ON P.OwnerUserId = U.Id
// WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'
// ),
// PostStatistics AS (
// SELECT
// RP.PostId,
// RP.Title,
// RP.CreationDate,
// RP.OwnerDisplayName,
// RP.ViewCount,
// RP.Score,
// SUM(V.BountyAmount) AS TotalBounties,
// COUNT(C.Id) AS CommentCount,
// COUNT(V.Id) FILTER (WHERE V.VoteTypeId = 2) AS UpVoteCount,
// COUNT(V.Id) FILTER (WHERE V.VoteTypeId = 3) AS DownVoteCount
// FROM RecentPosts RP
// LEFT JOIN Votes V ON RP.PostId = V.PostId
// LEFT JOIN Comments C ON RP.PostId = C.PostId
// GROUP BY RP.PostId, RP.Title, RP.CreationDate, RP.OwnerDisplayName, RP.ViewCount, RP.Score
// ),
// FinalReport AS (
// SELECT
// TU.DisplayName AS TopUserName,
// TU.Reputation,
// PS.Title,
// PS.CreationDate,
// PS.ViewCount,
// PS.Score,
// PS.TotalBounties,
// PS.CommentCount,
// PS.UpVoteCount,
// PS.DownVoteCount
// FROM TopUsers TU
// JOIN PostStatistics PS ON TU.UserId = PS.PostId
// )
// SELECT
// *
// FROM FinalReport
// ORDER BY reputation DESC, ViewCount DESC;
fn q8340(db: &'static So) -> String {
    let uid = uids(db);
    let tu = owned(db).group_by(&db.post.owner_user).select(&db.post.post_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 1) as i64]);
    let mut v = Vec::new();
    stats_fold(db, owned_since(db, month_ago()), Ident::<Post>::new(), "vc", &[])
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and((&tu).filt(|a: [i64; 3]| a[0] > 10))))
        .drive(|p, (s, (u, _))| v.push((p, s, u)));
    rows(v.iter().map(|&(p, s, u)| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(post_fields(db, p, &["title", "created", "views", "score"]));
        f.extend([nullable(s.bounty_sum, s.bounty_n), V::I(s.cx), V::I(s.up), V::I(s.down)]);
        row(f)
    }))
}

// WITH UserStatistics AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// U.CreationDate,
// COUNT(DISTINCT P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(P.Score) AS TotalScore,
// SUM(P.ViewCount) AS TotalViews,
// SUM(P.CommentCount) AS TotalComments
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id, U.DisplayName, U.Reputation, U.CreationDate
// ),
// BadgeStatistics AS (
// SELECT
// B.UserId,
// COUNT(*) AS TotalBadges,
// SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM
// Badges B
// GROUP BY
// B.UserId
// ),
// CloseReasons AS (
// SELECT
// PH.UserId,
// COUNT(*) AS TotalCloseVotes
// FROM
// PostHistory PH
// WHERE
// PH.PostHistoryTypeId = 10
// GROUP BY
// PH.UserId
// )
// SELECT
// US.UserId,
// US.DisplayName,
// US.Reputation,
// US.TotalPosts,
// US.TotalQuestions,
// US.TotalAnswers,
// US.TotalScore,
// US.TotalViews,
// US.TotalComments,
// COALESCE(BS.TotalBadges, 0) AS TotalBadges,
// COALESCE(BS.GoldBadges, 0) AS GoldBadges,
// COALESCE(BS.SilverBadges, 0) AS SilverBadges,
// COALESCE(BS.BronzeBadges, 0) AS BronzeBadges,
// COALESCE(CS.TotalCloseVotes, 0) AS TotalCloseVotes
// FROM
// UserStatistics US
// LEFT JOIN
// BadgeStatistics BS ON US.UserId = BS.UserId
// LEFT JOIN
// CloseReasons CS ON US.UserId = CS.UserId
// ORDER BY
// US.Reputation DESC, US.TotalPosts DESC;
fn q7565(db: &'static So) -> String {
    let ps = pstat(db, db.post.iq());
    let pc = pcc(db);
    let bc = badge_classes(db);
    let cv = db.post_history.with((&db.post_history.post_history_type_id).eq(10)).group_by(&db.post_history.user).select(&db.post_history.post).fold(0i64, |a, _| a + 1);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&ps).opt()).and((&pc).opt()).and((&bc).opt()).and((&cv).opt())).drive(|_, ((((u, p), c), b), x)| v.push((u, pz(p), c.unwrap_or([0; 3]), bz(b), x.unwrap_or(0))));
    rows(v.iter().map(|&(u, p, c, b, x)| {
        let mut f = ["uid", "name", "rep"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.extend([V::I(p[0]), V::I(p[1]), V::I(p[2]), nullable(p[3], p[0]), pviews(p), nullable(c[0], p[0])]);
        f.extend(ints(&b));
        f.push(V::I(x));
        row(f)
    }))
}

// WITH UserVoteCounts AS (
// SELECT
// UserId,
// COUNT(CASE WHEN VoteTypeId IN (2, 3) THEN 1 END) AS UpVoteCount,
// COUNT(CASE WHEN VoteTypeId = 4 THEN 1 END) AS OffensiveVoteCount,
// COUNT(CASE WHEN VoteTypeId = 10 THEN 1 END) AS DeleteVoteCount
// FROM Votes
// GROUP BY UserId
// ),
// TopUsers AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COALESCE(UC.UpVoteCount, 0) AS UpVotes,
// COALESCE(UC.OffensiveVoteCount, 0) AS OffensiveVotes,
// COALESCE(UC.DeleteVoteCount, 0) AS DeleteVotes
// FROM Users U
// LEFT JOIN UserVoteCounts UC ON U.Id = UC.UserId
// )
// SELECT
// A.OwnerUserId,
// A.Title AS AnswerTitle,
// A.Id AS AnswerId,
// COALESCE(UP.UpVotes, 0) AS UserUpVotes,
// COALESCE(UP.OffensiveVotes, 0) AS UserOffensiveVotes,
// COALESCE(UP.DeleteVotes, 0) AS UserDeleteVotes,
// T.TagName,
// COUNT(DISTINCT C.Id) AS CommentCount,
// COUNT(DISTINCT PH.Id) AS HistoryChanges
// FROM Posts A
// JOIN Posts Q ON A.ParentId = Q.Id
// LEFT JOIN Comments C ON C.PostId = A.Id
// LEFT JOIN PostHistory PH ON PH.PostId = A.Id
// JOIN (
// SELECT
// T.Id,
// T.TagName,
// P.Id AS PostId
// FROM Tags T
// JOIN Posts P ON P.Tags LIKE '%' || T.TagName || '%'
// ) T ON T.PostId = Q.Id
// LEFT JOIN TopUsers UP ON UP.UserId = A.OwnerUserId
// WHERE A.PostTypeId = 2
// AND Q.PostTypeId = 1
// AND EXISTS (
// SELECT 1
// FROM Votes V
// WHERE V.PostId = Q.Id
// AND V.VoteTypeId IN (2, 3)
// HAVING COUNT(*) > 0
// )
// GROUP BY
// A.OwnerUserId, A.Title, A.Id, T.TagName, UP.UpVotes, UP.OffensiveVotes, UP.DeleteVotes
// HAVING
// COUNT(DISTINCT C.Id) > 5
// OR SUM(CASE WHEN PH.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) > 2
// ORDER BY
// UserUpVotes DESC,
// CommentCount DESC,
// HistoryChanges DESC;
fn q20706(db: &'static So) -> String {
    let tm = tag_mentions(db);
    let ti: HashIdx<Id<Post>, (Id<Post>, Id<Tag>)> = (&tm).map(|(p, _)| p).inv().collect();
    let v23 = db.vote.with((&db.vote.vote_type_id).filt(|t: i64| matches!(t, 2 | 3))).group_by(&db.vote.post).select(&db.vote.vote_type_id).fold(0i64, |a, _| a + 1);
    let uv = db.vote.group_by(&db.vote.user).select(&db.vote.vote_type_id).fold([0i64; 3], |a, t| [a[0] + matches!(t, 2 | 3) as i64, a[1] + (t == 4) as i64, a[2] + (t == 10) as i64]);
    let cp = comments_per_post(db);
    let hp = history_per_post(db);
    let ans = || db.post.with((&db.post.post_type_id).eq(2)).with((&db.post.parent).select(&db.post.post_type_id).filt(|t: i64| t == 1)).with((&db.post.parent).select(&v23));
    let h10 = ans()
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(history_of(db).select(&db.post_history.post_history_type_id).opt()))
        .fold(0i64, |n, (_, t)| n + (t == Some(10)) as i64);
    let mut v = Vec::new();
    ans().select(Ident::<Post>::new().and(&cp).and(&hp).and(&h10).and((&db.post.parent).select(&ti).map(|(_, t): (Id<Post>, Id<Tag>)| t)).and((&db.post.owner_user).select(&uv).opt()))
        .filt(|(((((_, c), _), k), _), _): (((((Id<Post>, i64), i64), i64), Id<Tag>), Option<[i64; 3]>)| c > 5 || k > 2)
        .drive(|_, x| v.push(x));
    rows(v.iter().map(|&(((((p, c), h), _), t), x)| {
        let x = x.unwrap_or([0; 3]);
        let mut f = post_fields(db, p, &["owner_id", "title", "id"]);
        f.extend(ints(&x));
        f.extend([V::S(db.tag.tag_name.get(t).unwrap()), V::I(c), V::I(h)]);
        row(f)
    }))
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// COUNT(p.Id) AS PostCount,
// COALESCE(SUM(p.Score), 0) AS TotalScore,
// COALESCE(SUM(p.ViewCount), 0) AS TotalViews,
// COALESCE(SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END), 0) AS QuestionCount,
// COALESCE(SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS AnswerCount,
// COALESCE(SUM(CASE WHEN p.PostTypeId = 1 AND p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END), 0) AS AcceptedAnswerCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id
// ),
// UserBadgeStats AS (
// SELECT
// b.UserId,
// COUNT(b.Id) AS BadgeCount,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM
// Badges b
// GROUP BY
// b.UserId
// ),
// UserVotesStats AS (
// SELECT
// v.UserId,
// COUNT(v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Votes v
// GROUP BY
// v.UserId
// )
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// us.PostCount,
// us.TotalScore,
// us.TotalViews,
// us.QuestionCount,
// us.AnswerCount,
// us.AcceptedAnswerCount,
// bs.BadgeCount,
// bs.GoldBadges,
// bs.SilverBadges,
// bs.BronzeBadges,
// vs.VoteCount,
// vs.UpVotes,
// vs.DownVotes
// FROM
// Users u
// LEFT JOIN
// UserPostStats us ON u.Id = us.UserId
// LEFT JOIN
// UserBadgeStats bs ON u.Id = bs.UserId
// LEFT JOIN
// UserVotesStats vs ON u.Id = vs.UserId
// ORDER BY
// us.TotalScore DESC,
// us.PostCount DESC;
fn q10890(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, accepted_answer_id, .. } = &db.post;
    let uf = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(score).and(view_count.opt()).and(accepted_answer_id.opt())).fold([0i64; 6], |a, (((t, s), w), acc)| {
        [a[0] + 1, a[1] + s, a[2] + w.unwrap_or(0), a[3] + (t == 1) as i64, a[4] + (t == 2) as i64, a[5] + (t == 1 && acc.is_some()) as i64]
    });
    let bc = badge_classes(db);
    let uv = uvotes(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&uf).opt()).and((&bc).opt()).and((&uv).opt())).drive(|_, (((u, a), b), x)| v.push((u, a.unwrap_or([0; 6]), b, x)));
    rows(v.iter().map(|&(u, a, b, x)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a));
        f.extend(b.map_or(nulls(4), |b| ints(&b)));
        f.extend(x.map_or(nulls(3), |x| ints(&x)));
        row(f)
    }))
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN p.ViewCount > 100 THEN 1 ELSE 0 END) AS PopularPostsCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.Reputation
// ),
// BadgeStats AS (
// SELECT
// b.UserId,
// COUNT(b.Id) AS BadgeCount,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM
// Badges b
// GROUP BY
// b.UserId
// ),
// VotingStats AS (
// SELECT
// v.UserId,
// COUNT(v.Id) AS VoteCount,
// SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Votes v
// JOIN
// VoteTypes vt ON v.VoteTypeId = vt.Id
// GROUP BY
// v.UserId
// )
// SELECT
// us.UserId,
// us.Reputation,
// us.PostCount,
// us.QuestionCount,
// us.AnswerCount,
// us.PopularPostsCount,
// COALESCE(bs.BadgeCount, 0) AS BadgeCount,
// COALESCE(bs.GoldBadges, 0) AS GoldBadges,
// COALESCE(bs.SilverBadges, 0) AS SilverBadges,
// COALESCE(bs.BronzeBadges, 0) AS BronzeBadges,
// COALESCE(vs.VoteCount, 0) AS VoteCount,
// COALESCE(vs.UpVotes, 0) AS UpVotes,
// COALESCE(vs.DownVotes, 0) AS DownVotes
// FROM
// UserStats us
// LEFT JOIN
// BadgeStats bs ON us.UserId = bs.UserId
// LEFT JOIN
// VotingStats vs ON us.UserId = vs.UserId
// ORDER BY
// us.Reputation DESC, us.PostCount DESC;
fn q14945(db: &'static So) -> String {
    let Post { post_type_id, view_count, .. } = &db.post;
    let uf = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(view_count.opt())).fold([0i64; 4], |a, (t, w)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (w.unwrap_or(0) > 100) as i64]);
    let bc = badge_classes(db);
    let vn = vote_named(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&uf).opt()).and((&bc).opt()).and((&vn).opt())).drive(|_, (((u, a), b), x)| v.push((u, a.unwrap_or([0; 4]), bz(b), x.unwrap_or([0; 3]))));
    rows(v.iter().map(|&(u, a, b, x)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep")];
        f.extend(ints(&a));
        f.extend(ints(&b));
        f.extend(ints(&x));
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
// SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts,
// SUM(CASE WHEN p.Score < 0 THEN 1 ELSE 0 END) AS NegativePosts
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// GROUP BY u.Id, u.DisplayName
// ), UserBadgeCounts AS (
// SELECT
// b.UserId,
// COUNT(b.Id) AS TotalBadges,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM Badges b
// GROUP BY b.UserId
// ), PostVoteStats AS (
// SELECT
// p.OwnerUserId,
// AVG(v.BountyAmount) AS AverageBountyAmount,
// COUNT(DISTINCT v.Id) AS TotalVotes
// FROM Posts p
// JOIN Votes v ON p.Id = v.PostId
// WHERE v.VoteTypeId IN (2, 3)
// GROUP BY p.OwnerUserId
// )
// SELECT
// ups.UserId,
// ups.DisplayName,
// ups.TotalPosts,
// ups.TotalQuestions,
// ups.TotalAnswers,
// ups.PositivePosts,
// ups.NegativePosts,
// COALESCE(ubc.TotalBadges, 0) AS TotalBadges,
// COALESCE(ubc.GoldBadges, 0) AS GoldBadges,
// COALESCE(ubc.SilverBadges, 0) AS SilverBadges,
// COALESCE(ubc.BronzeBadges, 0) AS BronzeBadges,
// COALESCE(pvs.AverageBountyAmount, 0) AS AverageBountyAmount,
// COALESCE(pvs.TotalVotes, 0) AS TotalVotes
// FROM UserPostStats ups
// LEFT JOIN UserBadgeCounts ubc ON ups.UserId = ubc.UserId
// LEFT JOIN PostVoteStats pvs ON ups.UserId = pvs.OwnerUserId
// WHERE ups.TotalPosts > 10
// ORDER BY ups.TotalPosts DESC, ups.PositivePosts DESC;
fn q7413(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let uf = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(score)).fold([0i64; 5], |a, (t, s)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (s > 0) as i64, a[4] + (s < 0) as i64]
    });
    let bc = badge_classes(db);
    let vf = owned(db)
        .group_by(&db.post.owner_user)
        .select(votes_of(db).with((&db.vote.vote_type_id).filt(|t: i64| matches!(t, 2 | 3))).select((&db.vote.bounty_amount).opt()))
        .fold([0i64; 3], |a, b| [a[0] + 1, a[1] + b.is_some() as i64, a[2] + b.unwrap_or(0)]);
    let mut v = Vec::new();
    (&uf).filt(|a: [i64; 5]| a[0] > 10).and((&bc).opt()).and((&vf).opt()).drive(|u, ((a, b), x)| v.push((u, a, bz(b), x.unwrap_or([0; 3]))));
    rows(v.iter().map(|&(u, a, b, x)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a));
        f.extend(ints(&b));
        f.extend([or0(x[2], x[1]), V::I(x[0])]);
        row(f)
    }))
}

// WITH UserReputation AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(COALESCE(vt.VoteCount, 0)) AS TotalVotes,
// SUM(CASE WHEN p.PostTypeId = 1 THEN p.Score ELSE 0 END) AS QuestionScore,
// SUM(CASE WHEN p.PostTypeId = 2 THEN p.Score ELSE 0 END) AS AnswerScore
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN (
// SELECT
// PostId,
// COUNT(*) AS VoteCount
// FROM
// Votes
// GROUP BY
// PostId
// ) vt ON p.Id = vt.PostId
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
// ),
// UserBadges AS (
// SELECT
// b.UserId,
// COUNT(b.Id) AS BadgeCount,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM
// Badges b
// GROUP BY
// b.UserId
// ),
// MinimumReputation AS (
// SELECT
// AVG(Reputation) AS AvgReputation,
// COUNT(*) AS UserCount
// FROM
// UserReputation
// WHERE
// Reputation > 1000
// )
// SELECT
// ur.UserId,
// ur.DisplayName,
// ur.Reputation,
// ur.PostCount,
// ur.TotalVotes,
// ur.QuestionScore,
// ur.AnswerScore,
// COALESCE(ub.BadgeCount, 0) AS BadgeCount,
// COALESCE(ub.GoldBadges, 0) AS GoldBadges,
// COALESCE(ub.SilverBadges, 0) AS SilverBadges,
// COALESCE(ub.BronzeBadges, 0) AS BronzeBadges,
// mr.AvgReputation,
// mr.UserCount
// FROM
// UserReputation ur
// LEFT JOIN
// UserBadges ub ON ur.UserId = ub.UserId
// CROSS JOIN
// MinimumReputation mr
// WHERE
// ur.Reputation > mr.AvgReputation
// ORDER BY
// ur.Reputation DESC
// LIMIT 50;
fn q5861(db: &'static So) -> String {
    let vp = votes_per_post(db);
    let Post { post_type_id, score, .. } = &db.post;
    let uf = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(score).and(&vp)).fold([0i64; 4], |a, ((t, s), x)| {
        [a[0] + 1, a[1] + x, a[2] + if t == 1 { s } else { 0 }, a[3] + if t == 2 { s } else { 0 }]
    });
    let bc = badge_classes(db);
    let t = user_base(db, UserWhere::RepGt(1000)).select(&db.user.reputation).fold_flat([0i64; 2], |a, r| [a[0] + 1, a[1] + r]);
    let mut v = Vec::new();
    db.user
        .select(Ident::<User>::new().and(&db.user.reputation).and((&uf).opt()).and((&bc).opt()))
        .filt(move |(((_, r), _), _): (((Id<User>, i64), Option<[i64; 4]>), Option<[i64; 4]>)| (r as i128) * (t[0] as i128) > t[1] as i128)
        .drive(|_, (((u, _), a), b)| v.push((u, a.unwrap_or([0; 4]), bz(b))));
    out(v, |&(u, ..)| rep_desc(db, u), 50, |&(u, a, b)| {
        let mut f = ["uid", "name", "rep"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.extend(ints(&a));
        f.extend(ints(&b));
        f.extend([avg(t[1], t[0]), V::I(t[0])]);
        f
    })
}

// WITH PostDetails AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate AS PostCreationDate,
// p.OwnerUserId,
// u.DisplayName AS OwnerDisplayName,
// COUNT(CASE WHEN c.Id IS NOT NULL THEN 1 END) AS CommentCount,
// COUNT(CASE WHEN a.Id IS NOT NULL THEN 1 END) AS AnswerCount,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Posts a ON p.Id = a.ParentId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// INNER JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.OwnerUserId, u.DisplayName
// ),
// PostHistorySummary AS (
// SELECT
// ph.PostId,
// COUNT(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 END) AS CloseCount,
// COUNT(CASE WHEN ph.PostHistoryTypeId = 11 THEN 1 END) AS ReopenCount,
// COUNT(CASE WHEN ph.PostHistoryTypeId = 12 THEN 1 END) AS DeleteCount,
// COUNT(CASE WHEN ph.PostHistoryTypeId = 13 THEN 1 END) AS UndeleteCount
// FROM
// PostHistory ph
// GROUP BY
// ph.PostId
// )
// SELECT
// pd.PostId,
// pd.Title,
// pd.PostCreationDate,
// pd.OwnerDisplayName,
// pd.CommentCount,
// pd.AnswerCount,
// pd.UpVotes,
// pd.DownVotes,
// COALESCE(phs.CloseCount, 0) AS CloseCount,
// COALESCE(phs.ReopenCount, 0) AS ReopenCount,
// COALESCE(phs.DeleteCount, 0) AS DeleteCount,
// COALESCE(phs.UndeleteCount, 0) AS UndeleteCount
// FROM
// PostDetails pd
// LEFT JOIN
// PostHistorySummary phs ON pd.PostId = phs.PostId
// ORDER BY
// pd.UpVotes DESC, pd.CommentCount DESC;
fn q8735(db: &'static So) -> String {
    let ht = db.post_history.group_by(&db.post_history.post).select(&db.post_history.post_history_type_id).fold([0i64; 4], |a, t| {
        [a[0] + (t == 10) as i64, a[1] + (t == 11) as i64, a[2] + (t == 12) as i64, a[3] + (t == 13) as i64]
    });
    let mut v = Vec::new();
    stats_fold(db, owned_since(db, date(2023, 1, 1)), Ident::<Post>::new(), "cav", &[]).and((&ht).opt()).drive(|p, (s, h)| v.push((p, s, h.unwrap_or([0; 4]))));
    rows(v.iter().map(|&(p, s, h)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend(ints(&[s.cx, s.ax, s.up, s.down]));
        f.extend(ints(&h));
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
// ActiveUsers AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COALESCE(P.PostCount, 0) AS PostCount,
// COALESCE(C.CommentCount, 0) AS CommentCount
// FROM
// Users U
// LEFT JOIN (
// SELECT
// OwnerUserId AS UserId,
// COUNT(*) AS PostCount
// FROM
// Posts
// WHERE
// CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY
// OwnerUserId
// ) P ON U.Id = P.UserId
// LEFT JOIN (
// SELECT
// UserId,
// COUNT(*) AS CommentCount
// FROM
// Comments
// WHERE
// CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY
// UserId
// ) C ON U.Id = C.UserId
// ),
// UserActivity AS (
// SELECT
// B.UserId,
// B.DisplayName,
// B.BadgeCount,
// B.GoldBadges,
// B.SilverBadges,
// B.BronzeBadges,
// A.PostCount,
// A.CommentCount
// FROM
// UserBadges B
// JOIN
// ActiveUsers A ON B.UserId = A.UserId
// )
// SELECT
// UserId,
// DisplayName,
// BadgeCount,
// GoldBadges,
// SilverBadges,
// BronzeBadges,
// PostCount,
// CommentCount
// FROM
// UserActivity
// WHERE
// PostCount > 5 OR BadgeCount > 0
// ORDER BY
// BadgeCount DESC, PostCount DESC, CommentCount DESC
// LIMIT 20;
fn q8084(db: &'static So) -> String {
    let ub = ubc(db);
    let pc = owned_since(db, year_ago()).group_by(&db.post.owner_user).select(&db.post.score).fold(0i64, |a, _| a + 1);
    let cc = db.comment.with((&db.comment.creation_date).ge(year_ago())).group_by(&db.comment.user).select(&db.comment.score).fold(0i64, |a, _| a + 1);
    let mut v = Vec::new();
    (&ub).and((&pc).opt()).and((&cc).opt()).filt(|((b, p), _): (([i64; 4], Option<i64>), Option<i64>)| p.unwrap_or(0) > 5 || b[0] > 0).drive(|u, ((b, p), c)| v.push((u, b, p.unwrap_or(0), c.unwrap_or(0))));
    out(v, |&(_, b, p, c)| (Reverse(b[0]), Reverse(p), Reverse(c)), 20, |&(u, b, p, c)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&b));
        f.extend(ints(&[p, c]));
        f
    })
}

// WITH UserActivity AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(CASE WHEN p.PostTypeId IN (4, 5) THEN 1 ELSE 0 END) AS TagWikis,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
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
// PostStatistics AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS TotalPostCount,
// SUM(p.Score) AS TotalScore,
// AVG(p.ViewCount) AS AverageViews
// FROM
// Posts p
// WHERE
// p.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year')
// GROUP BY
// p.OwnerUserId
// )
// SELECT
// ua.UserId,
// ua.DisplayName,
// ua.TotalPosts,
// ua.Questions,
// ua.Answers,
// ua.TagWikis,
// ub.GoldBadges,
// ub.SilverBadges,
// ub.BronzeBadges,
// ps.TotalPostCount,
// ps.TotalScore,
// ps.AverageViews,
// (ua.UpVotes - ua.DownVotes) AS NetVotes
// FROM
// UserActivity ua
// LEFT JOIN
// UserBadges ub ON ua.UserId = ub.UserId
// LEFT JOIN
// PostStatistics ps ON ua.UserId = ps.OwnerUserId
// ORDER BY
// ua.TotalPosts DESC, NetVotes DESC
// LIMIT 100;
fn q6963(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::RepGt(1000), "v", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let bc = badge_classes(db);
    let ps = pstat(db, since(db, year_ago()));
    let mut v = Vec::new();
    (&us).and((&dp).opt()).and((&bc).opt()).and((&ps).opt()).drive(|u, (((a, d), b), p)| v.push((u, a, d.unwrap_or(0), b, p)));
    out(v, |&(_, a, d, ..)| (Reverse(d), Reverse(a.up - a.down)), 100, |&(u, a, d, b, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&[d, a.q, a.a, a.t45]));
        f.extend(b.map_or(nulls(3), |b| ints(&[b[1], b[2], b[3]])));
        f.extend(p.map_or(nulls(3), |p| vec![V::I(p[0]), V::I(p[3]), pviews_avg(p)]));
        f.push(V::I(a.up - a.down));
        f
    })
}

// WITH RECURSIVE UserBadges AS (
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
// AveragePosts AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS PostCount,
// AVG(P.Score) AS AverageScore
// FROM
// Posts P
// WHERE
// P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY
// P.OwnerUserId
// ),
// ActiveUsers AS (
// SELECT
// U.Id,
// U.DisplayName,
// U.Reputation,
// COALESCE(UB.BadgeCount, 0) AS BadgeCount,
// COALESCE(AP.PostCount, 0) AS PostCount,
// COALESCE(AP.AverageScore, 0) AS AverageScore
// FROM
// Users U
// LEFT JOIN
// UserBadges UB ON U.Id = UB.UserId
// LEFT JOIN
// AveragePosts AP ON U.Id = AP.OwnerUserId
// WHERE
// U.Reputation > 1000
// )
// SELECT
// AU.DisplayName,
// AU.Reputation,
// AU.BadgeCount,
// AU.PostCount,
// AU.AverageScore,
// CASE
// WHEN AU.BadgeCount > 10 THEN 'Highly Decorated'
// WHEN AU.BadgeCount BETWEEN 5 AND 10 THEN 'Moderately Decorated'
// ELSE 'Needs More Contribution'
// END AS BadgeStatus,
// P.Title AS RecentPostTitle,
// P.Score AS RecentPostScore,
// P.LastActivityDate
// FROM
// ActiveUsers AU
// LEFT JOIN
// Posts P ON AU.Id = P.OwnerUserId
// WHERE
// P.CreationDate = (
// SELECT MAX(P2.CreationDate)
// FROM Posts P2
// WHERE P2.OwnerUserId = AU.Id
// )
// ORDER BY
// AU.Reputation DESC,
// AU.BadgeCount DESC,
// AU.AverageScore DESC
// LIMIT 10;
fn q32601(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let ap = owned_since(db, year_ago()).group_by(&db.post.owner_user).select(&db.post.score).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let lp = owned(db).group_by(&db.post.owner_user).select(&db.post.creation_date).fold(i64::MIN, |a, d| a.max(d));
    let latest: MatSet<Id<Post>> = owned(db).with((&db.post.owner_user).select(&lp).and(&db.post.creation_date).filt(|(m, d): (i64, i64)| m == d)).collect();
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and(&bu).and((&ap).opt()).and(posts_of(db).with(&latest))).drive(|_, x| v.push(x));
    let af = |a: Option<[i64; 2]>| a.map_or(0.0, |a| a[1] as f64 / a[0] as f64);
    out(v, |&(((u, b), a), _)| (rep_desc(db, u), Reverse(b), Reverse(fkey(af(a)))), 10, |&(((u, b), a), p)| {
        let st = if b > 10 { "Highly Decorated" } else if (5..=10).contains(&b) { "Moderately Decorated" } else { "Needs More Contribution" };
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep"), V::I(b), V::I(a.map_or(0, |a| a[0])), V::F(af(a)), V::S(st)];
        f.extend(post_fields(db, p, &["title", "score", "activity"]));
        f
    })
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
// RecentPosts AS (
// SELECT
// P.OwnerUserId,
// P.Title,
// P.CreationDate,
// COUNT(C.Id) AS CommentCount,
// SUM(CASE WHEN V.CreationDate IS NOT NULL THEN 1 ELSE 0 END) AS VoteCount
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// WHERE
// P.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days')
// GROUP BY
// P.OwnerUserId, P.Title, P.CreationDate
// ),
// UserActivity AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// UB.TotalBadges,
// UB.GoldBadges,
// UB.SilverBadges,
// UB.BronzeBadges,
// COUNT(RP.Title) AS RecentPostCount,
// SUM(RP.CommentCount) AS TotalComments,
// SUM(RP.VoteCount) AS TotalVotes
// FROM
// UserBadges UB
// LEFT JOIN
// RecentPosts RP ON UB.UserId = RP.OwnerUserId
// LEFT JOIN
// Users U ON UB.UserId = U.Id
// GROUP BY
// U.Id, U.DisplayName, UB.TotalBadges, UB.GoldBadges, UB.SilverBadges, UB.BronzeBadges
// )
// SELECT
// UA.UserId,
// UA.DisplayName,
// UA.TotalBadges,
// UA.GoldBadges,
// UA.SilverBadges,
// UA.BronzeBadges,
// UA.RecentPostCount,
// UA.TotalComments,
// UA.TotalVotes
// FROM
// UserActivity UA
// ORDER BY
// UA.TotalBadges DESC,
// UA.TotalComments DESC,
// UA.RecentPostCount DESC
// LIMIT 10;
fn q26783(db: &'static So) -> String {
    type K = ((Id<User>, Option<Str>), i64);
    let ub = ubc(db);
    let sf = stats_fold(db, owned_since(db, month_ago()), (&db.post.owner_user).and((&db.post.title).opt()).and(&db.post.creation_date), "cv", &[]);
    let ks: MatSet<K> = owned_since(db, month_ago()).select((&db.post.owner_user).and((&db.post.title).opt()).and(&db.post.creation_date)).collect();
    let rf = (&ks)
        .group_by(Same::<K>::new().map(|((u, _), _)| u))
        .select(Same::<K>::new().map(|((_, t), _)| t.is_some()).and(Same::<K>::new().select(&sf)))
        .fold([0i64; 3], |a, (t, s): (bool, Stats)| [a[0] + t as i64, a[1] + s.cx, a[2] + s.vx]);
    let mut v = Vec::new();
    (&ub).and((&rf).opt()).drive(|u, (b, r)| v.push((u, b, r)));
    out(v, |&(_, b, r)| (Reverse(b[0]), (r.is_none(), Reverse(r.map_or(0, |r| r[1]))), Reverse(r.map_or(0, |r| r[0]))), 10, |&(u, b, r)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&b));
        f.extend(match r {
            Some(r) => ints(&r),
            None => vec![V::I(0), V::Null, V::Null],
        });
        f
    })
}

// WITH UserPerformance AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(b.Id) AS BadgeCount,
// COALESCE(SUM(p.ViewCount), 0) AS TotalPostViews,
// COALESCE(SUM(p.AnswerCount), 0) AS TotalAnswers,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpVotes,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownVotes
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// u.Id, u.DisplayName
// ),
// PostActivity AS (
// SELECT
// p.Id AS PostId,
// p.OwnerUserId,
// p.Title,
// COUNT(c.Id) AS CommentCount,
// MAX(p.LastActivityDate) AS LastActivity
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// WHERE
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY
// p.Id, p.OwnerUserId, p.Title
// ),
// CombinedStats AS (
// SELECT
// up.UserId,
// up.DisplayName,
// up.BadgeCount,
// up.TotalPostViews,
// up.TotalAnswers,
// pa.CommentCount,
// pa.LastActivity,
// up.TotalUpVotes,
// up.TotalDownVotes
// FROM
// UserPerformance up
// LEFT JOIN
// PostActivity pa ON up.UserId = pa.OwnerUserId
// )
// SELECT
// UserId,
// DisplayName,
// BadgeCount,
// TotalPostViews,
// TotalAnswers,
// COALESCE(SUM(CommentCount), 0) AS TotalComments,
// MAX(LastActivity) AS MostRecentActivity,
// TotalUpVotes,
// TotalDownVotes
// FROM
// CombinedStats
// GROUP BY
// UserId, DisplayName, BadgeCount, TotalPostViews, TotalAnswers, TotalUpVotes, TotalDownVotes
// ORDER BY
// TotalPostViews DESC, TotalAnswers DESC, TotalUpVotes DESC
// LIMIT 50;
fn q7944(db: &'static So) -> String {
    let Post { view_count, answer_count, .. } = &db.post;
    let uf = g(db)
        .select(badges_of(db).opt().and(posts_of(db).select(view_count.opt().and(answer_count.opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt()))
        .fold([0i64; 5], |a, (b, p)| {
            let ((w, an), t) = p.unwrap_or(((None, None), None));
            [a[0] + b.is_some() as i64, a[1] + w.unwrap_or(0), a[2] + an.unwrap_or(0), a[3] + (t == Some(2)) as i64, a[4] + (t == Some(3)) as i64]
        });
    let cp = comments_per_post(db);
    let pa = owned_since(db, year_ago()).group_by(&db.post.owner_user).select((&cp).and(&db.post.last_activity_date)).fold((0i64, i64::MIN), |(n, m), (c, d)| (n + c, m.max(d)));
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and(&uf).and((&pa).opt())).drive(|_, ((u, a), p)| v.push((u, a, p)));
    out(v, |&(_, a, _)| (Reverse(a[1]), Reverse(a[2]), Reverse(a[3])), 50, |&(u, a, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&[a[0], a[1], a[2], p.map_or(0, |p| p.0)]));
        f.push(p.map_or(V::Null, |p| tmax(p.1)));
        f.extend(ints(&[a[3], a[4]]));
        f
    })
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesReceived,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesReceived
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
// ),
// BadgeStats AS (
// SELECT
// b.UserId,
// COUNT(*) AS BadgeCount,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM
// Badges b
// GROUP BY
// b.UserId
// ),
// CombinedStats AS (
// SELECT
// us.UserId,
// us.DisplayName,
// us.Reputation,
// us.PostCount,
// us.QuestionCount,
// us.AnswerCount,
// us.UpVotesReceived,
// us.DownVotesReceived,
// COALESCE(bs.BadgeCount, 0) AS BadgeCount,
// COALESCE(bs.GoldBadges, 0) AS GoldBadges,
// COALESCE(bs.SilverBadges, 0) AS SilverBadges,
// COALESCE(bs.BronzeBadges, 0) AS BronzeBadges
// FROM
// UserStats us
// LEFT JOIN
// BadgeStats bs ON us.UserId = bs.UserId
// )
// SELECT
// UserId,
// DisplayName,
// Reputation,
// PostCount,
// QuestionCount,
// AnswerCount,
// UpVotesReceived,
// DownVotesReceived,
// BadgeCount,
// GoldBadges,
// SilverBadges,
// BronzeBadges
// FROM
// CombinedStats
// WHERE
// Reputation > 100
// ORDER BY
// Reputation DESC, PostCount DESC
// LIMIT 10;
fn q6931(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::RepGt(100), "v", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&us).and((&dp).opt()).and((&bc).opt()).drive(|u, ((a, d), b)| v.push((u, a, d.unwrap_or(0), bz(b))));
    out(v, |&(u, _, d, _)| (rep_desc(db, u), Reverse(d)), 10, |&(u, a, d, b)| {
        let mut f = ["uid", "name", "rep"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.extend(ints(&[d, a.q, a.a, a.up, a.down]));
        f.extend(ints(&b));
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
// PostStatistics AS (
// SELECT
// P.OwnerUserId,
// COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS QuestionCount,
// COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS AnswerCount,
// SUM(P.Score) AS TotalScore,
// AVG(P.ViewCount) AS AvgViewCount,
// COUNT(DISTINCT P.Tags) AS UniqueTagCount
// FROM
// Posts P
// GROUP BY
// P.OwnerUserId
// ),
// TopUsers AS (
// SELECT
// U.Id,
// U.DisplayName,
// COALESCE(UBC.BadgeCount, 0) AS BadgeCount,
// COALESCE(PS.QuestionCount, 0) AS QuestionCount,
// COALESCE(PS.AnswerCount, 0) AS AnswerCount,
// COALESCE(PS.TotalScore, 0) AS TotalScore,
// COALESCE(PS.AvgViewCount, 0) AS AvgViewCount,
// COALESCE(UBC.GoldBadges, 0) AS GoldBadges,
// COALESCE(UBC.SilverBadges, 0) AS SilverBadges,
// COALESCE(UBC.BronzeBadges, 0) AS BronzeBadges
// FROM
// Users U
// LEFT JOIN
// UserBadgeCounts UBC ON U.Id = UBC.UserId
// LEFT JOIN
// PostStatistics PS ON U.Id = PS.OwnerUserId
// )
// SELECT
// TU.DisplayName,
// TU.BadgeCount,
// TU.QuestionCount,
// TU.AnswerCount,
// TU.TotalScore,
// TU.AvgViewCount,
// CONCAT('Gold: ', TU.GoldBadges, ', Silver: ', TU.SilverBadges, ', Bronze: ', TU.BronzeBadges) AS BadgeSummary
// FROM
// TopUsers TU
// WHERE
// TU.BadgeCount > 0
// ORDER BY
// TU.TotalScore DESC,
// TU.BadgeCount DESC
// LIMIT 10;
fn q2880(db: &'static So) -> String {
    let ub = ubc(db);
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let ps = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(score).and(view_count.opt())).fold([0i64; 5], |a, ((t, s), w)| {
        [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + s, a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0)]
    });
    let mut v = Vec::new();
    (&ub).filt(|b: [i64; 4]| b[0] > 0).and((&ps).opt()).drive(|u, (b, p)| v.push((u, b, p.unwrap_or([0; 5]))));
    out(v, |&(_, b, p)| (Reverse(p[2]), Reverse(b[0])), 10, |&(u, b, p)| {
        vec![user_col(db, u, "name"), V::I(b[0]), V::I(p[0]), V::I(p[1]), V::I(p[2]), or0(p[4], p[3]), V::Owned(format!("Gold: {}, Silver: {}, Bronze: {}", b[1], b[2], b[3]))]
    })
}

// WITH UserMetrics AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(DISTINCT P.Id) AS PostCount,
// SUM(CASE WHEN P.Score > 0 THEN P.Score ELSE 0 END) AS PositiveScore,
// SUM(CASE WHEN P.Score < 0 THEN P.Score ELSE 0 END) AS NegativeScore,
// COUNT(DISTINCT B.Id) AS BadgeCount
// FROM Users U
// LEFT JOIN Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN Badges B ON U.Id = B.UserId
// GROUP BY U.Id, U.DisplayName, U.Reputation
// ),
// PostStatistics AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.ViewCount,
// P.Score,
// COUNT(C.Id) AS CommentCount,
// COUNT(DISTINCT L.RelatedPostId) AS LinkedPostCount
// FROM Posts P
// LEFT JOIN Comments C ON P.Id = C.PostId
// LEFT JOIN PostLinks L ON P.Id = L.PostId
// GROUP BY P.Id, P.Title, P.ViewCount, P.Score
// ),
// AggregatedMetrics AS (
// SELECT
// UM.UserId,
// UM.DisplayName,
// UM.Reputation,
// UM.PostCount,
// UM.PositiveScore,
// UM.NegativeScore,
// UM.BadgeCount,
// COALESCE(SUM(PS.ViewCount), 0) AS TotalViews,
// COALESCE(SUM(PS.Score), 0) AS TotalScore,
// COALESCE(SUM(PS.CommentCount), 0) AS TotalComments,
// COALESCE(SUM(PS.LinkedPostCount), 0) AS TotalLinkedPosts
// FROM UserMetrics UM
// LEFT JOIN PostStatistics PS ON UM.UserId = PS.PostId
// GROUP BY UM.UserId, UM.DisplayName, UM.Reputation, UM.PostCount, UM.PositiveScore, UM.NegativeScore, UM.BadgeCount
// )
// SELECT
// AM.UserId,
// AM.DisplayName,
// AM.Reputation,
// AM.PostCount,
// AM.PositiveScore,
// AM.NegativeScore,
// AM.BadgeCount,
// AM.TotalViews,
// AM.TotalScore,
// AM.TotalComments,
// AM.TotalLinkedPosts
// FROM AggregatedMetrics AM
// WHERE AM.Reputation > 1000
// ORDER BY AM.Reputation DESC, AM.PostCount DESC, AM.TotalViews DESC;
fn q28584(db: &'static So) -> String {
    let pid = pids(db);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let bu = badges_per_user(db);
    let sc = user_base(db, UserWhere::RepGt(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(&db.post.score).opt().and(badges_of(db).opt()))
        .fold([0i64; 2], |a, (s, _)| {
            let s = s.unwrap_or(0);
            [a[0] + if s > 0 { s } else { 0 }, a[1] + if s < 0 { s } else { 0 }]
        });
    let lp = db.post_link.group_by(&db.post_link.post).select(&db.post_link.related_post_id).count_distinct();
    let cl = db.post.group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(links_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000))
        .select(Ident::<User>::new().and((&dp).opt()).and(&bu).and(&sc).and((&db.user.origid).select(&pid).select(Ident::<Post>::new().and(&cl).and((&lp).opt())).opt()))
        .drive(|_, x| v.push(x));
    rows(v.iter().map(|&((((u, d), b), s), p)| {
        let mut f = ["uid", "name", "rep"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.extend(ints(&[d.unwrap_or(0), s[0], s[1], b]));
        f.extend(match p {
            Some(((p, c), l)) => ints(&[db.post.view_count.get(p).unwrap_or(0), db.post.score.get(p).unwrap(), c, l.unwrap_or(0)]),
            None => ints(&[0, 0, 0, 0]),
        });
        row(f)
    }))
}

// WITH UserActivity AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(DISTINCT P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(CASE WHEN P.PostTypeId = 1 THEN P.AnswerCount ELSE 0 END) AS TotalAnswersAccepted,
// SUM(CASE WHEN P.CommentCount > 0 THEN 1 ELSE 0 END) AS TotalComments
// FROM Users U
// LEFT JOIN Posts P ON U.Id = P.OwnerUserId
// GROUP BY U.Id, U.DisplayName
// ),
// BadgeSummary AS (
// SELECT
// B.UserId,
// COUNT(DISTINCT B.Id) AS TotalBadges,
// SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM Badges B
// GROUP BY B.UserId
// ),
// PostEngagement AS (
// SELECT
// P.OwnerUserId,
// COUNT(DISTINCT C.Id) AS TotalCommentsOnPosts,
// SUM(V.BountyAmount) AS TotalBountyConsumed
// FROM Posts P
// LEFT JOIN Comments C ON P.Id = C.PostId
// LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId IN (2, 3)
// GROUP BY P.OwnerUserId
// )
// SELECT
// UA.UserId,
// UA.DisplayName,
// UA.TotalPosts,
// UA.Questions,
// UA.Answers,
// UA.TotalAnswersAccepted,
// UA.TotalComments,
// COALESCE(BS.TotalBadges, 0) AS TotalBadges,
// COALESCE(BS.GoldBadges, 0) AS GoldBadges,
// COALESCE(BS.SilverBadges, 0) AS SilverBadges,
// COALESCE(BS.BronzeBadges, 0) AS BronzeBadges,
// COALESCE(PE.TotalCommentsOnPosts, 0) AS TotalCommentsOnPosts,
// COALESCE(PE.TotalBountyConsumed, 0) AS TotalBountyConsumed
// FROM UserActivity UA
// LEFT JOIN BadgeSummary BS ON UA.UserId = BS.UserId
// LEFT JOIN PostEngagement PE ON UA.UserId = PE.OwnerUserId
// ORDER BY UA.TotalPosts DESC, UA.DisplayName ASC;
fn q6934(db: &'static So) -> String {
    let Post { post_type_id, answer_count, comment_count, .. } = &db.post;
    let uf = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(answer_count.opt()).and(comment_count)).fold([0i64; 6], |a, ((t, an), cc)| {
        let nn = t != 1 || an.is_some();
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + if t == 1 { an.unwrap_or(0) } else { 0 }, a[4] + nn as i64, a[5] + (cc > 0) as i64]
    });
    let bc = badge_classes(db);
    let dc = udc(db);
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).filt(|t: i64| matches!(t, 2 | 3)))).select((&db.vote.bounty_amount).opt());
    let pe = owned(db).group_by(&db.post.owner_user).select(comments_of(db).opt().and(bounty.opt())).fold(0i64, |a, (_, b)| a + b.flatten().unwrap_or(0));
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&uf).opt()).and((&bc).opt()).and((&dc).opt()).and((&pe).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((((u, a), b), c), e)| {
        let b = bz(b);
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(match a {
            Some(a) => vec![V::I(a[0]), V::I(a[1]), V::I(a[2]), nullable(a[3], a[4]), V::I(a[5])],
            None => ints(&[0, 0, 0, 0, 0]),
        });
        f.extend(ints(&b));
        f.extend(ints(&[c.unwrap_or(0), e.unwrap_or(0)]));
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
// Users AS U
// LEFT JOIN
// Badges AS B ON U.Id = B.UserId
// GROUP BY
// U.Id
// ),
// PostStats AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS PostCount,
// SUM(P.Score) AS TotalScore,
// SUM(P.ViewCount) AS TotalViews,
// AVG(P.Score) AS AvgScore
// FROM
// Posts AS P
// WHERE
// P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY
// P.OwnerUserId
// ),
// VoteStats AS (
// SELECT
// V.UserId,
// COUNT(V.Id) AS VoteCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Votes AS V
// GROUP BY
// V.UserId
// )
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COALESCE(UB.BadgeCount, 0) AS BadgeCount,
// COALESCE(UB.GoldBadges, 0) AS GoldBadges,
// COALESCE(UB.SilverBadges, 0) AS SilverBadges,
// COALESCE(UB.BronzeBadges, 0) AS BronzeBadges,
// COALESCE(PS.PostCount, 0) AS PostCount,
// COALESCE(PS.TotalScore, 0) AS TotalScore,
// COALESCE(PS.TotalViews, 0) AS TotalViews,
// COALESCE(PS.AvgScore, 0) AS AvgScore,
// COALESCE(VS.VoteCount, 0) AS VoteCount,
// COALESCE(VS.UpVotes, 0) AS UpVotes,
// COALESCE(VS.DownVotes, 0) AS DownVotes
// FROM
// Users AS U
// LEFT JOIN
// UserBadges AS UB ON U.Id = UB.UserId
// LEFT JOIN
// PostStats AS PS ON U.Id = PS.OwnerUserId
// LEFT JOIN
// VoteStats AS VS ON U.Id = VS.UserId
// ORDER BY
// TotalScore DESC,
// PostCount DESC,
// BadgeCount DESC
// LIMIT 100;
fn q9245(db: &'static So) -> String {
    let bc = badge_classes(db);
    let ps = pstat(db, since(db, year_ago()));
    let uv = uvotes(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&bc).opt()).and((&ps).opt()).and((&uv).opt())).drive(|_, (((u, b), p), x)| v.push((u, bz(b), pz(p), x.unwrap_or([0; 3]))));
    out(v, |&(_, b, p, _)| (Reverse(p[3]), Reverse(p[0]), Reverse(b[0])), 100, |&(u, b, p, x)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&b));
        f.extend(ints(&[p[0], p[3], p[5]]));
        f.push(or0(p[3], p[0]));
        f.extend(ints(&x));
        f
    })
}

// WITH PostStats AS (
// SELECT
// P.Id AS PostId,
// COUNT(CASE WHEN PH.PostHistoryTypeId = 10 THEN 1 END) AS CloseCount,
// COUNT(CASE WHEN PH.PostHistoryTypeId IN (1, 2, 4, 5) THEN 1 END) AS EditCount,
// COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpvoteCount,
// COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownvoteCount,
// COUNT(CASE WHEN B.UserId IS NOT NULL THEN 1 END) AS BadgeCount
// FROM
// Posts P
// LEFT JOIN
// PostHistory PH ON P.Id = PH.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// LEFT JOIN
// Badges B ON P.OwnerUserId = B.UserId
// WHERE
// P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY
// P.Id
// ),
// MaxStats AS (
// SELECT
// MAX(CloseCount) AS MaxCloseCount,
// MAX(EditCount) AS MaxEditCount,
// MAX(UpvoteCount) AS MaxUpvoteCount,
// MAX(DownvoteCount) AS MaxDownvoteCount,
// MAX(BadgeCount) AS MaxBadgeCount
// FROM
// PostStats
// )
// SELECT
// PS.PostId,
// PS.CloseCount,
// PS.EditCount,
// PS.UpvoteCount,
// PS.DownvoteCount,
// PS.BadgeCount,
// CASE
// WHEN PS.CloseCount = MS.MaxCloseCount THEN 'Highest Close Count'
// ELSE NULL
// END AS CloseCountRank,
// CASE
// WHEN PS.EditCount = MS.MaxEditCount THEN 'Highest Edit Count'
// ELSE NULL
// END AS EditCountRank,
// CASE
// WHEN PS.UpvoteCount = MS.MaxUpvoteCount THEN 'Highest Upvote Count'
// ELSE NULL
// END AS UpvoteCountRank,
// CASE
// WHEN PS.DownvoteCount = MS.MaxDownvoteCount THEN 'Highest Downvote Count'
// ELSE NULL
// END AS DownvoteCountRank,
// CASE
// WHEN PS.BadgeCount = MS.MaxBadgeCount THEN 'Highest Badge Count'
// ELSE NULL
// END AS BadgeCountRank
// FROM
// PostStats PS, MaxStats MS
// ORDER BY
// PS.PostId;
fn q5869(db: &'static So) -> String {
    let pf = since(db, year_ago())
        .group_by(Ident::<Post>::new())
        .select(
            history_of(db)
                .select(&db.post_history.post_history_type_id)
                .opt()
                .and(votes_of(db).select(&db.vote.vote_type_id).opt())
                .and((&db.post.owner_user).select(badges_of(db)).opt()),
        )
        .fold([0i64; 5], |a, ((h, t), b)| {
            [a[0] + (h == Some(10)) as i64, a[1] + matches!(h, Some(1 | 2 | 4 | 5)) as i64, a[2] + (t == Some(2)) as i64, a[3] + (t == Some(3)) as i64, a[4] + b.is_some() as i64]
        });
    let mx = (&pf).fold_flat([i64::MIN; 5], |a, x| [a[0].max(x[0]), a[1].max(x[1]), a[2].max(x[2]), a[3].max(x[3]), a[4].max(x[4])]);
    let names = ["Highest Close Count", "Highest Edit Count", "Highest Upvote Count", "Highest Downvote Count", "Highest Badge Count"];
    let mut v = Vec::new();
    (&pf).drive(|p, a| v.push((p, a)));
    rows(v.iter().map(|&(p, a)| {
        let mut f = post_fields(db, p, &["id"]);
        f.extend(ints(&a));
        f.extend((0..5).map(|i| if a[i] == mx[i] { V::S(names[i]) } else { V::Null }));
        row(f)
    }))
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS TotalAnswers,
// COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS TotalQuestions,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
// FROM
// Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN Votes v ON p.Id = v.PostId
// GROUP BY u.Id, u.DisplayName, u.Reputation
// ),
// BadgeCounts AS (
// SELECT
// b.UserId,
// COUNT(b.Id) AS TotalBadges,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM
// Badges b
// GROUP BY b.UserId
// ),
// PostHistoryStats AS (
// SELECT
// ph.UserId,
// COUNT(ph.Id) AS TotalEdits,
// COUNT(DISTINCT CASE WHEN ph.PostHistoryTypeId = 10 THEN ph.Id END) AS TotalClosedPosts
// FROM
// PostHistory ph
// GROUP BY ph.UserId
// )
// SELECT
// us.UserId,
// us.DisplayName,
// us.Reputation,
// us.TotalPosts,
// us.TotalQuestions,
// us.TotalAnswers,
// us.TotalUpVotes,
// us.TotalDownVotes,
// COALESCE(bc.TotalBadges, 0) AS TotalBadges,
// COALESCE(bc.GoldBadges, 0) AS GoldBadges,
// COALESCE(bc.SilverBadges, 0) AS SilverBadges,
// COALESCE(bc.BronzeBadges, 0) AS BronzeBadges,
// COALESCE(phs.TotalEdits, 0) AS TotalEdits,
// COALESCE(phs.TotalClosedPosts, 0) AS TotalClosedPosts
// FROM
// UserStats us
// LEFT JOIN
// BadgeCounts bc ON us.UserId = bc.UserId
// LEFT JOIN
// PostHistoryStats phs ON us.UserId = phs.UserId
// ORDER BY
// us.Reputation DESC
// LIMIT 50;
fn q6949(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let ps = pstat(db, db.post.iq());
    let bc = badge_classes(db);
    let ph = db.post_history.group_by(&db.post_history.user).select(&db.post_history.post_history_type_id).fold([0i64; 2], |a, t| [a[0] + 1, a[1] + (t == 10) as i64]);
    let mut v = Vec::new();
    (&us).and((&ps).opt()).and((&bc).opt()).and((&ph).opt()).drive(|u, (((a, p), b), h)| v.push((u, a, pz(p), bz(b), h.unwrap_or([0; 2]))));
    out(v, |&(u, ..)| rep_desc(db, u), 50, |&(u, a, p, b, h)| {
        let mut f = ["uid", "name", "rep"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.extend(ints(&[p[0], p[1], p[2], a.up, a.down]));
        f.extend(ints(&b));
        f.extend(ints(&h));
        f
    })
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
// SUM(P.Score) AS TotalScore,
// AVG(P.ViewCount) AS AverageViewCount,
// MAX(P.CreationDate) AS LastPostDate
// FROM
// Posts P
// GROUP BY
// P.OwnerUserId
// ),
// VoteSummary AS (
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
// U.Id AS UserId,
// U.DisplayName,
// COALESCE(UB.BadgeCount, 0) AS BadgeCount,
// COALESCE(UB.GoldBadges, 0) AS GoldBadges,
// COALESCE(UB.SilverBadges, 0) AS SilverBadges,
// COALESCE(UB.BronzeBadges, 0) AS BronzeBadges,
// COALESCE(PS.TotalPosts, 0) AS TotalPosts,
// COALESCE(PS.TotalScore, 0) AS TotalScore,
// COALESCE(PS.AverageViewCount, 0) AS AverageViewCount,
// COALESCE(VS.TotalVotes, 0) AS TotalVotes,
// COALESCE(VS.UpVotes, 0) AS UpVotes,
// COALESCE(VS.DownVotes, 0) AS DownVotes,
// CASE
// WHEN PS.LastPostDate IS NULL THEN 'No Posts'
// ELSE 'Active'
// END AS UserActivity
// FROM
// Users U
// LEFT JOIN
// UserBadgeStats UB ON U.Id = UB.UserId
// LEFT JOIN
// PostStats PS ON U.Id = PS.OwnerUserId
// LEFT JOIN
// VoteSummary VS ON U.Id = VS.UserId
// WHERE
// U.Reputation > (SELECT AVG(Reputation) FROM Users)
// ORDER BY
// U.Reputation DESC;
fn q4768(db: &'static So) -> String {
    let bc = badge_classes(db);
    let Post { score, view_count, creation_date, .. } = &db.post;
    let ps = owned(db).group_by(&db.post.owner_user).select(score.and(view_count.opt()).and(creation_date)).fold([0, 0, 0, 0, i64::MIN], |a: [i64; 5], ((s, w), c)| {
        [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4].max(c)]
    });
    let uv = uvotes(db);
    let t = db.user.select(&db.user.reputation).fold_flat([0i64; 2], |a, r| [a[0] + 1, a[1] + r]);
    let mut v = Vec::new();
    db.user
        .select(Ident::<User>::new().and(&db.user.reputation).and((&bc).opt()).and((&ps).opt()).and((&uv).opt()))
        .filt(move |((((_, r), _), _), _): ((((Id<User>, i64), Option<[i64; 4]>), Option<[i64; 5]>), Option<[i64; 3]>)| (r as i128) * (t[0] as i128) > t[1] as i128)
        .drive(|_, ((((u, _), b), p), x)| v.push((u, bz(b), p, x.unwrap_or([0; 3]))));
    rows(v.iter().map(|&(u, b, p, x)| {
        let pp = p.unwrap_or([0; 5]);
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&b));
        f.extend([V::I(pp[0]), V::I(pp[1]), or0(pp[3], pp[2])]);
        f.extend(ints(&x));
        f.push(V::S(if p.is_none() { "No Posts" } else { "Active" }));
        row(f)
    }))
}

// WITH UserReputation AS (
// SELECT Users.Id, Users.Reputation, COUNT(DISTINCT Posts.Id) AS PostCount,
// SUM(CASE WHEN Votes.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
// SUM(CASE WHEN Votes.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
// FROM Users
// LEFT JOIN Posts ON Users.Id = Posts.OwnerUserId
// LEFT JOIN Votes ON Posts.Id = Votes.PostId
// GROUP BY Users.Id, Users.Reputation
// ),
// PostDetails AS (
// SELECT Posts.Id AS PostId, Posts.Title, Posts.CreationDate, Posts.Score,
// Users.DisplayName AS OwnerName, PostTypes.Name AS PostType,
// COUNT(Comments.Id) AS CommentCount,
// COALESCE(SUM(CASE WHEN Votes.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpvotes,
// COALESCE(SUM(CASE WHEN Votes.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownvotes
// FROM Posts
// LEFT JOIN Users ON Posts.OwnerUserId = Users.Id
// LEFT JOIN PostTypes ON Posts.PostTypeId = PostTypes.Id
// LEFT JOIN Comments ON Posts.Id = Comments.PostId
// LEFT JOIN Votes ON Posts.Id = Votes.PostId
// GROUP BY Posts.Id, Posts.Title, Posts.CreationDate, Posts.Score, Users.DisplayName, PostTypes.Name
// ),
// TopUsers AS (
// SELECT UserReputation.Id, UserReputation.Reputation, UserReputation.PostCount,
// UserReputation.Upvotes, UserReputation.Downvotes
// FROM UserReputation
// WHERE UserReputation.Reputation > (SELECT AVG(Reputation) FROM Users)
// ORDER BY UserReputation.Reputation DESC
// LIMIT 10
// )
// SELECT
// TopUsers.Id AS UserId,
// TopUsers.Reputation,
// PostDetails.Title,
// PostDetails.CreationDate,
// PostDetails.Score,
// PostDetails.CommentCount,
// PostDetails.TotalUpvotes,
// PostDetails.TotalDownvotes
// FROM TopUsers
// JOIN Posts ON TopUsers.Id = Posts.OwnerUserId
// JOIN PostDetails ON Posts.Id = PostDetails.PostId
// ORDER BY TopUsers.Reputation DESC, PostDetails.Score DESC;
fn q6212(db: &'static So) -> String {
    let t = db.user.select(&db.user.reputation).fold_flat([0i64; 2], |a, r| [a[0] + 1, a[1] + r]);
    let base = db.user.with((&db.user.reputation).filt(move |r: i64| (r as i128) * (t[0] as i128) > t[1] as i128));
    let top: MatSet<Id<User>> = whole(&base)
        .select(Ident::<User>::new().and(&db.user.reputation))
        .window(row_number, |(_, r): (Id<User>, i64)| r, desc)
        .filt(|(_, n)| n <= 10)
        .map(|((u, _), _)| u)
        .collect();
    let sf = stats_fold(db, owned(db).with((&db.post.owner_user).select(&top)), Ident::<Post>::new(), "cv", &[]);
    let mut v = Vec::new();
    (&sf).drive(|p, s| v.push((p, s)));
    rows(v.iter().map(|&(p, s)| {
        let u = db.post.owner_user.get(p).unwrap();
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep")];
        f.extend(post_fields(db, p, &["title", "created", "score"]));
        f.extend(ints(&[s.cx, s.up, s.down]));
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
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// u.Id, u.DisplayName
// ),
// BadgeStats AS (
// SELECT
// UserId,
// COUNT(*) AS BadgeCount,
// SUM(CASE WHEN Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM
// Badges
// GROUP BY
// UserId
// ),
// PostStatistics AS (
// SELECT
// p.OwnerUserId,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// AVG(COALESCE((SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id), 0)) AS AvgCommentsPerPost,
// MAX(p.ViewCount) AS MaxViews,
// MIN(p.CreationDate) AS FirstPostDate,
// MAX(p.CreationDate) AS LastPostDate
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// )
// SELECT
// ua.UserId,
// ua.DisplayName,
// ua.PostCount,
// ua.QuestionCount,
// ua.AnswerCount,
// ua.UpvoteCount,
// ua.DownvoteCount,
// bs.BadgeCount,
// bs.GoldBadges,
// bs.SilverBadges,
// bs.BronzeBadges,
// ps.TotalPosts,
// ps.AvgCommentsPerPost,
// ps.MaxViews,
// ps.FirstPostDate,
// ps.LastPostDate
// FROM
// UserActivity ua
// LEFT JOIN
// BadgeStats bs ON ua.UserId = bs.UserId
// LEFT JOIN
// PostStatistics ps ON ua.UserId = ps.OwnerUserId
// ORDER BY
// ua.PostCount DESC,
// ua.UpvoteCount DESC
// LIMIT 100;
fn q8237(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let bc = badge_classes(db);
    let cp = comments_per_post(db);
    let Post { view_count, creation_date, .. } = &db.post;
    let ps = owned(db).group_by(&db.post.owner_user).select((&cp).and(view_count.opt()).and(creation_date)).fold([0, 0, i64::MIN, i64::MAX, i64::MIN], |a: [i64; 5], ((c, w), d)| {
        [a[0] + 1, a[1] + c, w.map_or(a[2], |w| a[2].max(w)), a[3].min(d), a[4].max(d)]
    });
    let mut v = Vec::new();
    (&us).and((&dp).opt()).and((&bc).opt()).and((&ps).opt()).drive(|u, (((a, d), b), p)| v.push((u, a, d.unwrap_or(0), b, p)));
    out(v, |&(_, a, d, ..)| (Reverse(d), Reverse(a.up)), 100, |&(u, a, d, b, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&[d, a.q, a.a, a.up, a.down]));
        f.extend(b.map_or(nulls(4), |b| ints(&b)));
        f.extend(match p {
            Some(p) => vec![V::I(p[0]), avg(p[1], p[0]), if p[2] == i64::MIN { V::Null } else { V::I(p[2]) }, V::T(p[3]), V::T(p[4])],
            None => nulls(5),
        });
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
// PostStats AS (
// SELECT
// P.OwnerUserId,
// COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS Questions,
// COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS Answers,
// SUM(P.Score) AS TotalScore,
// SUM(P.ViewCount) AS TotalViews,
// AVG(EXTRACT(EPOCH FROM (cast('2024-10-01 12:34:56' as timestamp) - P.CreationDate))) AS AverageAgeInSeconds
// FROM
// Posts P
// GROUP BY
// P.OwnerUserId
// ),
// CombinedStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COALESCE(UB.GoldBadges, 0) AS GoldBadges,
// COALESCE(UB.SilverBadges, 0) AS SilverBadges,
// COALESCE(UB.BronzeBadges, 0) AS BronzeBadges,
// COALESCE(PS.Questions, 0) AS TotalQuestions,
// COALESCE(PS.Answers, 0) AS TotalAnswers,
// COALESCE(PS.TotalScore, 0) AS TotalScore,
// COALESCE(PS.TotalViews, 0) AS TotalViews,
// COALESCE(PS.AverageAgeInSeconds, 0) AS AveragePostAgeInSeconds
// FROM
// Users U
// LEFT JOIN
// UserBadges UB ON U.Id = UB.UserId
// LEFT JOIN
// PostStats PS ON U.Id = PS.OwnerUserId
// )
// SELECT
// *,
// CASE
// WHEN TotalQuestions > 0 THEN TotalScore / TotalQuestions
// ELSE 0
// END AS AverageScorePerQuestion,
// CASE
// WHEN AveragePostAgeInSeconds > 0 THEN TotalViews / AveragePostAgeInSeconds
// ELSE 0
// END AS ViewsPerSecond
// FROM
// CombinedStats
// WHERE
// GoldBadges > 0
// ORDER BY
// TotalScore DESC,
// DisplayName ASC
// LIMIT 10;
fn q3381(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let bc = badge_classes(db);
    let Post { post_type_id, score, view_count, creation_date, .. } = &db.post;
    let ps = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(score).and(view_count.opt()).and(creation_date)).fold(([0i64; 5], 0i128), move |(a, e), (((t, s), w), c)| {
        ([a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + w.unwrap_or(0)], e + (t0 - c) as i128)
    });
    let mut v = Vec::new();
    (&bc).filt(|b: [i64; 4]| b[1] > 0).and((&ps).opt()).drive(|u, (b, p)| v.push((u, b, p.unwrap_or(([0; 5], 0)))));
    out(v, |&(u, _, (p, _))| (Reverse(p[3]), db.user.display_name.get(u).unwrap()), 10, |&(u, b, (p, e))| {
        let age = if p[0] == 0 { 0.0 } else { e as f64 / p[0] as f64 / 1e6 };
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&[b[1], b[2], b[3], p[1], p[2], p[3], p[4]]));
        f.push(V::F(age));
        f.push(V::F(if p[1] > 0 { p[3] as f64 / p[1] as f64 } else { 0.0 }));
        f.push(V::F(if age > 0.0 { p[4] as f64 / age } else { 0.0 }));
        f
    })
}

// WITH UserReputation AS (
// SELECT
// Id,
// DisplayName,
// Reputation,
// CASE
// WHEN Reputation >= 1000 THEN 'High'
// WHEN Reputation >= 100 THEN 'Medium'
// ELSE 'Low'
// END AS ReputationLevel
// FROM Users
// ),
// PostStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers
// FROM Posts p
// GROUP BY p.OwnerUserId
// ),
// RecentActivity AS (
// SELECT
// p.OwnerUserId,
// MAX(p.LastActivityDate) AS LastActivityDate
// FROM Posts p
// GROUP BY p.OwnerUserId
// ),
// TopUsers AS (
// SELECT
// ur.DisplayName,
// ur.ReputationLevel,
// ps.TotalPosts,
// ps.TotalQuestions,
// ps.TotalAnswers,
// ra.LastActivityDate,
// ur.Id AS OwnerUserId
// FROM UserReputation ur
// JOIN PostStats ps ON ur.Id = ps.OwnerUserId
// JOIN RecentActivity ra ON ur.Id = ra.OwnerUserId
// WHERE ur.ReputationLevel = 'High'
// AND ps.TotalPosts > 5
// AND ra.LastActivityDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'
// )
// SELECT
// t.DisplayName,
// t.ReputationLevel,
// t.TotalPosts,
// t.TotalQuestions,
// t.TotalAnswers,
// COALESCE(pv.UpVotes, 0) AS RecentUpVotes,
// COALESCE(cv.CloseVotes, 0) AS RecentCloseVotes
// FROM TopUsers t
// LEFT JOIN (
// SELECT
// p.OwnerUserId,
// COUNT(v.Id) AS UpVotes
// FROM Posts p
// JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId = 2
// GROUP BY p.OwnerUserId
// ) pv ON t.OwnerUserId = pv.OwnerUserId
// LEFT JOIN (
// SELECT
// ph.UserId,
// COUNT(ph.Id) AS CloseVotes
// FROM PostHistory ph
// WHERE ph.PostHistoryTypeId = 10
// GROUP BY ph.UserId
// ) cv ON t.OwnerUserId = cv.UserId
// ORDER BY t.TotalPosts DESC, t.ReputationLevel;
fn q228(db: &'static So) -> String {
    let Post { post_type_id, last_activity_date, .. } = &db.post;
    let ps = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(last_activity_date)).fold([0, 0, 0, i64::MIN], |a: [i64; 4], (t, d)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3].max(d)]
    });
    let pv = owned(db).group_by(&db.post.owner_user).select(votes_of(db).with((&db.vote.vote_type_id).eq(2))).fold(0i64, |a, _| a + 1);
    let cv = db.post_history.with((&db.post_history.post_history_type_id).eq(10)).group_by(&db.post_history.user).select(&db.post_history.post).fold(0i64, |a, _| a + 1);
    let m = month_ago();
    let mut v = Vec::new();
    db.user
        .with((&db.user.reputation).ge(1000))
        .select(Ident::<User>::new().and((&ps).filt(move |p: [i64; 4]| p[0] > 5 && p[3] >= m)).and((&pv).opt()).and((&cv).opt()))
        .drive(|_, x| v.push(x));
    rows(v.iter().map(|&(((u, p), x), c)| row(vec![user_col(db, u, "name"), V::S("High"), V::I(p[0]), V::I(p[1]), V::I(p[2]), V::I(x.unwrap_or(0)), V::I(c.unwrap_or(0))])))
}

// WITH RecursiveUserBadges AS (
// SELECT
// b.UserId,
// COUNT(*) AS BadgeCount,
// MIN(b.Date) AS FirstBadgeDate
// FROM
// Badges b
// GROUP BY
// b.UserId
// ),
// ActivePosts AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.ViewCount,
// p.Score,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount
// FROM
// Posts p
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY
// p.Id, p.Title, p.ViewCount, p.Score
// ),
// PostDetails AS (
// SELECT
// p.Id,
// p.OwnerUserId,
// u.DisplayName AS OwnerDisplayName,
// p.Title,
// p.CreationDate,
// ap.ViewCount,
// ap.Score,
// ap.UpVoteCount,
// rb.BadgeCount,
// rb.FirstBadgeDate
// FROM
// Posts p
// JOIN
// ActivePosts ap ON p.Id = ap.PostId
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// RecursiveUserBadges rb ON u.Id = rb.UserId
// WHERE
// p.PostTypeId = 1
// ),
// CloseReasonCounts AS (
// SELECT
// ph.PostId,
// COUNT(*) AS CloseReasonCount,
// MIN(ph.CreationDate) AS FirstClosedDate
// FROM
// PostHistory ph
// WHERE
// ph.PostHistoryTypeId = 10
// GROUP BY
// ph.PostId
// ),
// FinalPostAnalysis AS (
// SELECT
// pd.*,
// crc.CloseReasonCount,
// crc.FirstClosedDate
// FROM
// PostDetails pd
// LEFT JOIN
// CloseReasonCounts crc ON pd.Id = crc.PostId
// )
// SELECT
// fpa.Title,
// fpa.OwnerDisplayName,
// fpa.ViewCount,
// fpa.Score,
// fpa.UpVoteCount,
// fpa.BadgeCount,
// fpa.FirstBadgeDate,
// COALESCE(fpa.CloseReasonCount, 0) AS CloseReasonCount,
// fpa.FirstClosedDate
// FROM
// FinalPostAnalysis fpa
// ORDER BY
// fpa.Score DESC, fpa.ViewCount DESC;
fn q32554(db: &'static So) -> String {
    let bf = db.badge.group_by(&db.badge.user).select(&db.badge.date).fold((0i64, i64::MAX), |(n, m), d| (n + 1, m.min(d)));
    let up = votes_of_type(db, 2);
    let cf = db.post_history.with((&db.post_history.post_history_type_id).eq(10)).group_by(&db.post_history.post).select(&db.post_history.creation_date).fold((0i64, i64::MAX), |(n, m), d| (n + 1, m.min(d)));
    let mut v = Vec::new();
    questions_only(db)
        .with((&db.post.creation_date).ge(year_ago()))
        .select(Ident::<Post>::new().and(&up).and((&db.post.owner_user).select(&bf).opt()).and((&cf).opt()))
        .drive(|_, x| v.push(x));
    rows(v.iter().map(|&(((p, x), b), c)| {
        let mut f = post_fields(db, p, &["title", "owner", "views", "score"]);
        f.push(V::I(x));
        f.extend([oint(b.map(|b| b.0)), ots(b.map(|b| b.1)), V::I(c.map_or(0, |c| c.0)), ots(c.map(|c| c.1))]);
        row(f)
    }))
}

// WITH PostEngagement AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// PT.Name AS PostTypeName,
// U.DisplayName AS OwnerDisplayName,
// P.CreationDate,
// P.Score,
// P.ViewCount,
// COALESCE(C.CommentCount, 0) AS CommentCount,
// COALESCE(V.UpVoteCount, 0) AS UpVoteCount,
// COALESCE(V.DownVoteCount, 0) AS DownVoteCount
// FROM
// Posts P
// JOIN
// PostTypes PT ON P.PostTypeId = PT.Id
// JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN (
// SELECT PostId, COUNT(*) AS CommentCount
// FROM Comments
// GROUP BY PostId
// ) C ON P.Id = C.PostId
// LEFT JOIN (
// SELECT
// PostId,
// SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
// FROM Votes
// GROUP BY PostId
// ) V ON P.Id = V.PostId
// ),
// UserEngagement AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(DISTINCT P.Id) AS TotalPosts,
// SUM(COALESCE(P.ViewCount, 0)) AS TotalViews,
// SUM(COALESCE(P.Score, 0)) AS TotalScore,
// SUM(COALESCE(C.CommentCount, 0)) AS TotalComments
// FROM
// Users U
// LEFT JOIN
// Posts P ON P.OwnerUserId = U.Id
// LEFT JOIN (
// SELECT PostId, COUNT(*) AS CommentCount
// FROM Comments
// GROUP BY PostId
// ) C ON P.Id = C.PostId
// GROUP BY
// U.Id, U.DisplayName
// )
// SELECT
// PE.PostId,
// PE.Title,
// PE.PostTypeName,
// PE.OwnerDisplayName,
// PE.CreationDate,
// PE.Score,
// PE.ViewCount,
// PE.CommentCount,
// PE.UpVoteCount,
// PE.DownVoteCount,
// UE.UserId,
// UE.TotalPosts,
// UE.TotalViews,
// UE.TotalScore,
// UE.TotalComments
// FROM
// PostEngagement PE
// JOIN
// UserEngagement UE ON PE.OwnerDisplayName = UE.DisplayName
// ORDER BY
// PE.Score DESC, PE.ViewCount DESC;
fn q11276(db: &'static So) -> String {
    let cp = comments_per_post(db);
    let pv = post_votes(db);
    let Post { view_count, score, .. } = &db.post;
    let ue = owned(db).group_by(&db.post.owner_user).select(view_count.opt().and(score).and(&cp)).fold([0i64; 4], |a, ((w, s), c)| [a[0] + 1, a[1] + w.unwrap_or(0), a[2] + s, a[3] + c]);
    let names: HashIdx<Str, Id<User>> = db.user.select(&db.user.display_name).inv().collect();
    let mut v = Vec::new();
    owned(db)
        .select(Ident::<Post>::new().and(&cp).and((&pv).opt()).and((&db.post.owner_user).select(&db.user.display_name).select(&names).select(Ident::<User>::new().and((&ue).opt()))))
        .drive(|_, x| v.push(x));
    rows(v.iter().map(|&(((p, c), x), (u, a))| {
        let x = x.unwrap_or([0; 3]);
        let mut f = post_fields(db, p, &["id", "title", "type", "owner", "created", "score", "views"]);
        f.extend(ints(&[c, x[1], x[2]]));
        f.push(user_col(db, u, "uid"));
        f.extend(ints(&a.unwrap_or([0; 4])));
        row(f)
    }))
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
// LEFT JOIN Badges B ON U.Id = B.UserId
// GROUP BY
// U.Id, U.DisplayName
// ),
// PostStats AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// AVG(P.Score) AS AverageScore
// FROM
// Posts P
// GROUP BY
// P.OwnerUserId
// ),
// RecentActivity AS (
// SELECT
// U.Id AS UserId,
// COUNT(CASE WHEN C.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days') THEN 1 END) AS RecentComments,
// COUNT(CASE WHEN V.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days') THEN 1 END) AS RecentVotes
// FROM
// Users U
// LEFT JOIN Comments C ON U.Id = C.UserId
// LEFT JOIN Votes V ON U.Id = V.UserId
// GROUP BY
// U.Id
// )
// SELECT
// U.DisplayName,
// COALESCE(UB.GoldBadges, 0) AS GoldBadges,
// COALESCE(UB.SilverBadges, 0) AS SilverBadges,
// COALESCE(UB.BronzeBadges, 0) AS BronzeBadges,
// COALESCE(PS.TotalPosts, 0) AS TotalPosts,
// COALESCE(PS.TotalQuestions, 0) AS TotalQuestions,
// COALESCE(PS.TotalAnswers, 0) AS TotalAnswers,
// COALESCE(PS.AverageScore, 0) AS AverageScore,
// COALESCE(RA.RecentComments, 0) AS RecentComments,
// COALESCE(RA.RecentVotes, 0) AS RecentVotes
// FROM
// Users U
// LEFT JOIN UserBadges UB ON U.Id = UB.UserId
// LEFT JOIN PostStats PS ON U.Id = PS.OwnerUserId
// LEFT JOIN RecentActivity RA ON U.Id = RA.UserId
// WHERE
// U.Reputation > 1000
// ORDER BY
// U.DisplayName;
fn q293(db: &'static So) -> String {
    let bc = badge_classes(db);
    let ps = pstat(db, db.post.iq());
    let m = month_ago();
    let ra = user_base(db, UserWhere::RepGt(1000))
        .group_by(Ident::<User>::new())
        .select(comments_by(db).select(&db.comment.creation_date).opt().and(votes_by(db).select(&db.vote.creation_date).opt()))
        .fold([0i64; 2], move |a, (c, x)| [a[0] + c.map_or(false, |d| d >= m) as i64, a[1] + x.map_or(false, |d| d >= m) as i64]);
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and((&bc).opt()).and((&ps).opt()).and(&ra)).drive(|_, (((u, b), p), r)| v.push((u, bz(b), pz(p), r[0], r[1])));
    rows(v.iter().map(|&(u, b, p, c, x)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(ints(&[b[1], b[2], b[3], p[0], p[1], p[2]]));
        f.push(or0(p[3], p[0]));
        f.extend(ints(&[c, x]));
        row(f)
    }))
}

// WITH UserBadgeCount AS (
// SELECT
// u.Id AS UserId,
// COUNT(b.Id) AS BadgeCount,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadgeCount,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadgeCount,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadgeCount
// FROM Users u
// LEFT JOIN Badges b ON u.Id = b.UserId
// GROUP BY u.Id
// ),
// ActivePostStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS TotalPosts,
// COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS Questions,
// COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS Answers,
// SUM(p.Score) AS TotalScore,
// SUM(p.ViewCount) AS TotalViews
// FROM Posts p
// WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY p.OwnerUserId
// ),
// UserPerformance AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// u.CreationDate,
// u.LastAccessDate,
// COALESCE(ubc.BadgeCount, 0) AS BadgeCount,
// COALESCE(ubc.GoldBadgeCount, 0) AS GoldBadgeCount,
// COALESCE(ubc.SilverBadgeCount, 0) AS SilverBadgeCount,
// COALESCE(ubc.BronzeBadgeCount, 0) AS BronzeBadgeCount,
// COALESCE(ps.TotalPosts, 0) AS TotalPosts,
// COALESCE(ps.Questions, 0) AS Questions,
// COALESCE(ps.Answers, 0) AS Answers,
// COALESCE(ps.TotalScore, 0) AS TotalScore,
// COALESCE(ps.TotalViews, 0) AS TotalViews
// FROM Users u
// LEFT JOIN UserBadgeCount ubc ON u.Id = ubc.UserId
// LEFT JOIN ActivePostStats ps ON u.Id = ps.OwnerUserId
// )
// SELECT
// UserId,
// DisplayName,
// Reputation,
// CreationDate,
// LastAccessDate,
// BadgeCount,
// GoldBadgeCount,
// SilverBadgeCount,
// BronzeBadgeCount,
// TotalPosts,
// Questions,
// Answers,
// TotalScore,
// TotalViews
// FROM UserPerformance
// ORDER BY TotalScore DESC, Reputation DESC
// LIMIT 10;
fn q5684(db: &'static So) -> String {
    let bc = badge_classes(db);
    let ps = pstat(db, since(db, year_ago()));
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&bc).opt()).and((&ps).opt())).drive(|_, ((u, b), p)| v.push((u, bz(b), pz(p))));
    out(v, |&(u, _, p)| (Reverse(p[3]), rep_desc(db, u)), 10, |&(u, b, p)| {
        let mut f = ["uid", "name", "rep", "ucreated", "last_access"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.extend(ints(&b));
        f.extend(ints(&[p[0], p[1], p[2], p[3], p[5]]));
        f
    })
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// U.UpVotes,
// U.DownVotes,
// COUNT(DISTINCT P.Id) AS PostCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesReceived,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesReceived,
// AVG(P.Score) AS AvgPostScore
// FROM Users U
// LEFT JOIN Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN Votes V ON P.Id = V.PostId
// GROUP BY U.Id, U.DisplayName, U.Reputation, U.UpVotes, U.DownVotes
// ),
// BadgeCounts AS (
// SELECT
// B.UserId,
// COUNT(B.Id) AS BadgeCount,
// SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM Badges B
// GROUP BY B.UserId
// ),
// CombinedStats AS (
// SELECT
// US.UserId,
// US.DisplayName,
// US.Reputation,
// US.PostCount,
// US.QuestionCount,
// US.AnswerCount,
// COALESCE(BC.BadgeCount, 0) AS BadgeCount,
// COALESCE(BC.GoldBadges, 0) AS GoldBadges,
// COALESCE(BC.SilverBadges, 0) AS SilverBadges,
// COALESCE(BC.BronzeBadges, 0) AS BronzeBadges,
// US.UpVotesReceived,
// US.DownVotesReceived,
// US.AvgPostScore
// FROM UserStats US
// LEFT JOIN BadgeCounts BC ON US.UserId = BC.UserId
// )
// SELECT
// C.DisplayName,
// C.Reputation,
// C.PostCount,
// C.QuestionCount,
// C.AnswerCount,
// C.BadgeCount,
// C.GoldBadges,
// C.SilverBadges,
// C.BronzeBadges,
// C.UpVotesReceived,
// C.DownVotesReceived,
// C.AvgPostScore
// FROM CombinedStats C
// WHERE C.Reputation > 1000
// ORDER BY C.Reputation DESC, C.PostCount DESC
// LIMIT 10;
fn q7669(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::RepGt(1000), "v", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&us).and((&dp).opt()).and((&bc).opt()).drive(|u, ((a, d), b)| v.push((u, a, d.unwrap_or(0), bz(b))));
    out(v, |&(u, _, d, _)| (rep_desc(db, u), Reverse(d)), 10, |&(u, a, d, b)| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(ints(&[d, a.q, a.a]));
        f.extend(ints(&b));
        f.extend([V::I(a.up), V::I(a.down), avg(a.score_sum, a.n)]);
        f
    })
}

// WITH UserActivity AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COALESCE(SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS AnswerCount,
// COALESCE(SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END), 0) AS QuestionCount,
// COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes,
// COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// U.Id, U.DisplayName, U.Reputation
// ),
// PostStatistics AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.ViewCount,
// P.Score,
// U.DisplayName AS OwnerDisplayName,
// COALESCE(COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END), 0) AS CommentCount,
// COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpvoteCount,
// COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownvoteCount
// FROM
// Posts P
// LEFT JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.ViewCount, P.Score, U.DisplayName
// ),
// TopUsers AS (
// SELECT
// UserId,
// DisplayName,
// Reputation,
// AnswerCount + QuestionCount AS ActivityScore
// FROM
// UserActivity
// ORDER BY
// ActivityScore DESC
// LIMIT 10
// )
// SELECT
// PS.Title,
// PS.CreationDate,
// PS.ViewCount,
// PS.Score,
// PS.CommentCount,
// PS.UpvoteCount,
// PS.DownvoteCount,
// TU.DisplayName AS TopUser,
// TU.Reputation
// FROM
// PostStatistics PS
// JOIN
// TopUsers TU ON PS.OwnerDisplayName = TU.DisplayName
// WHERE
// PS.ViewCount > 100
// ORDER BY
// PS.Score DESC, PS.ViewCount DESC;
fn q7479(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let top: MatSet<Id<User>> = whole(&db.user.id)
        .select(Ident::<User>::new().and(&us))
        .window(row_number, |(_, a): (Id<User>, UStats)| a.q + a.a, desc)
        .filt(|(_, n)| n <= 10)
        .map(|((u, _), _)| u)
        .collect();
    let names: HashIdx<Str, Id<User>> = (&top).select(&db.user.display_name).inv().collect();
    let mut v = Vec::new();
    stats_fold(db, owned(db).with((&db.post.view_count).gt(100)), Ident::<Post>::new(), "cv", &[])
        .and((&db.post.owner_user).select(&db.user.display_name).select(&names))
        .drive(|p, (s, u)| v.push((p, s, u)));
    rows(v.iter().map(|&(p, s, u)| {
        let mut f = post_fields(db, p, &["title", "created", "views", "score"]);
        f.extend(ints(&[s.cx, s.up, s.down]));
        f.extend([user_col(db, u, "name"), user_col(db, u, "rep")]);
        row(f)
    }))
}

// WITH UserVotes AS (
// SELECT
// v.UserId,
// SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Votes v
// JOIN
// VoteTypes vt ON v.VoteTypeId = vt.Id
// GROUP BY
// v.UserId
// ),
// UserBadges AS (
// SELECT
// b.UserId,
// COUNT(*) FILTER (WHERE b.Class = 1) AS GoldBadges,
// COUNT(*) FILTER (WHERE b.Class = 2) AS SilverBadges,
// COUNT(*) FILTER (WHERE b.Class = 3) AS BronzeBadges
// FROM
// Badges b
// GROUP BY
// b.UserId
// ),
// PostStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AvgScore,
// COUNT(CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS Questions,
// COUNT(CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS Answers
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// ),
// ClosedPosts AS (
// SELECT
// ph.UserId,
// COUNT(DISTINCT ph.PostId) AS ClosedPostCount
// FROM
// PostHistory ph
// WHERE
// ph.PostHistoryTypeId IN (10, 11)
// GROUP BY
// ph.UserId
// )
// SELECT
// u.DisplayName,
// u.Reputation,
// COALESCE(uv.UpVotes, 0) AS UpVotes,
// COALESCE(uv.DownVotes, 0) AS DownVotes,
// COALESCE(ub.GoldBadges, 0) AS GoldBadges,
// COALESCE(ub.SilverBadges, 0) AS SilverBadges,
// COALESCE(ub.BronzeBadges, 0) AS BronzeBadges,
// COALESCE(ps.TotalPosts, 0) AS TotalPosts,
// COALESCE(ps.AvgScore, 0) AS AvgScore,
// COALESCE(ps.Questions, 0) AS Questions,
// COALESCE(ps.Answers, 0) AS Answers,
// COALESCE(cp.ClosedPostCount, 0) AS ClosedPosts
// FROM
// Users u
// LEFT JOIN
// UserVotes uv ON u.Id = uv.UserId
// LEFT JOIN
// UserBadges ub ON u.Id = ub.UserId
// LEFT JOIN
// PostStats ps ON u.Id = ps.OwnerUserId
// LEFT JOIN
// ClosedPosts cp ON u.Id = cp.UserId
// WHERE
// u.Reputation > 1000
// ORDER BY
// u.Reputation DESC
// LIMIT 50;
fn q4072(db: &'static So) -> String {
    let vn = vote_named(db);
    let bc = badge_classes(db);
    let ps = pstat(db, db.post.iq());
    let cp = db.post_history.with((&db.post_history.post_history_type_id).filt(|t: i64| matches!(t, 10 | 11))).group_by(&db.post_history.user).select(&db.post_history.post).count_distinct();
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and((&vn).opt()).and((&bc).opt()).and((&ps).opt()).and((&cp).opt())).drive(|_, ((((u, x), b), p), c)| v.push((u, x.unwrap_or([0; 3]), bz(b), pz(p), c.unwrap_or(0))));
    out(v, |&(u, ..)| rep_desc(db, u), 50, |&(u, x, b, p, c)| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep"), V::I(x[1]), V::I(x[2]), V::I(b[1]), V::I(b[2]), V::I(b[3]), V::I(p[0]), or0(p[3], p[0]), V::I(p[1]), V::I(p[2])];
        f.push(V::I(c));
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
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(CASE WHEN p.PostTypeId = 3 THEN 1 ELSE 0 END) AS Wikis,
// SUM(p.Score) AS TotalScore,
// AVG(p.ViewCount) AS AvgViews
// FROM
// Posts p
// WHERE
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY
// p.OwnerUserId
// ),
// CombinedStats AS (
// SELECT
// ubs.UserId,
// ubs.DisplayName,
// ubs.BadgeCount,
// ubs.GoldBadges,
// ubs.SilverBadges,
// ubs.BronzeBadges,
// ps.TotalPosts,
// ps.Questions,
// ps.Answers,
// ps.Wikis,
// ps.TotalScore,
// ps.AvgViews
// FROM
// UserBadgeStats ubs
// LEFT JOIN
// PostStats ps ON ubs.UserId = ps.OwnerUserId
// )
// SELECT
// c.DisplayName,
// COALESCE(c.BadgeCount, 0) AS BadgeCount,
// COALESCE(c.GoldBadges, 0) AS GoldBadges,
// COALESCE(c.SilverBadges, 0) AS SilverBadges,
// COALESCE(c.BronzeBadges, 0) AS BronzeBadges,
// COALESCE(c.TotalPosts, 0) AS TotalPosts,
// COALESCE(c.Questions, 0) AS Questions,
// COALESCE(c.Answers, 0) AS Answers,
// COALESCE(c.Wikis, 0) AS Wikis,
// COALESCE(c.TotalScore, 0) AS TotalScore,
// COALESCE(c.AvgViews, 0) AS AvgViews
// FROM
// CombinedStats c
// ORDER BY
// c.TotalScore DESC,
// c.BadgeCount DESC
// LIMIT 10;
fn q9627(db: &'static So) -> String {
    let ub = ubc(db);
    let ps = pstat(db, since(db, year_ago()));
    let mut v = Vec::new();
    (&ub).and((&ps).opt()).drive(|u, (b, p)| v.push((u, b, pz(p))));
    out(v, |&(_, b, p)| (Reverse(p[3]), Reverse(b[0])), 10, |&(u, b, p)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(ints(&b));
        f.extend(ints(&[p[0], p[1], p[2], p[8], p[3]]));
        f.push(or0(p[5], p[4]));
        f
    })
}

// WITH UserActivity AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN p.PostTypeId IN (6, 12) THEN 1 ELSE 0 END) AS DeletedPostCount,
// SUM(CASE WHEN v.UserId IS NOT NULL THEN 1 ELSE 0 END) AS VoteCount,
// SUM(CASE WHEN c.Id IS NOT NULL THEN 1 ELSE 0 END) AS CommentCount,
// AVG(u.Reputation) AS AvgReputation
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
// ),
// UserBadges AS (
// SELECT
// b.UserId,
// COUNT(b.Id) AS BadgeCount,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM
// Badges b
// GROUP BY
// b.UserId
// ),
// FinalStats AS (
// SELECT
// ua.UserId,
// ua.DisplayName,
// ua.PostCount,
// ua.QuestionCount,
// ua.AnswerCount,
// ua.DeletedPostCount,
// ua.VoteCount,
// ua.CommentCount,
// ub.BadgeCount,
// ub.GoldBadges,
// ub.SilverBadges,
// ub.BronzeBadges,
// ua.AvgReputation
// FROM
// UserActivity ua
// LEFT JOIN
// UserBadges ub ON ua.UserId = ub.UserId
// )
// SELECT
// fs.DisplayName,
// fs.PostCount,
// fs.QuestionCount,
// fs.AnswerCount,
// fs.DeletedPostCount,
// fs.VoteCount,
// fs.CommentCount,
// fs.BadgeCount,
// fs.GoldBadges,
// fs.SilverBadges,
// fs.BronzeBadges,
// fs.AvgReputation
// FROM
// FinalStats fs
// ORDER BY
// fs.PostCount DESC, fs.QuestionCount DESC, fs.VoteCount DESC
// LIMIT 10;
fn q6080(db: &'static So) -> String {
    let uf = g(db)
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select((&db.vote.user_id).opt()).opt()).and(comments_of(db).opt())).opt())
        .fold([0i64; 5], |a, p| match p {
            Some(((t, v), c)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + matches!(t, 6 | 12) as i64, a[3] + v.flatten().is_some() as i64, a[4] + c.is_some() as i64],
            None => a,
        });
    let dp = ud(db, UserWhere::All, posts_of(db));
    let bc = badge_classes(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&uf).opt()).and((&dp).opt()).and((&bc).opt())).drive(|_, (((u, a), d), b)| v.push((u, a.unwrap_or([0; 5]), d.unwrap_or(0), b)));
    out(v, |&(_, a, d, _)| (Reverse(d), Reverse(a[0]), Reverse(a[3])), 10, |&(u, a, d, b)| {
        let mut f = vec![user_col(db, u, "name"), V::I(d)];
        f.extend(ints(&a));
        f.extend(b.map_or(nulls(4), |b| ints(&b)));
        f.push(V::F(db.user.reputation.get(u).unwrap() as f64));
        f
    })
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS TotalQuestions,
// COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS TotalAnswers,
// SUM(vt.BountyAmount) AS TotalBounty,
// SUM(COALESCE(b.Class, 0)) AS TotalBadges,
// COALESCE(SUM(p.ViewCount), 0) AS TotalViews
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN Votes vt ON p.Id = vt.PostId AND vt.VoteTypeId = 8
// LEFT JOIN Badges b ON u.Id = b.UserId
// GROUP BY u.Id, u.DisplayName, u.Reputation
// ),
// PostHistoryStats AS (
// SELECT
// ph.UserId,
// COUNT(*) AS TotalEdits,
// COUNT(DISTINCT CASE WHEN ph.PostHistoryTypeId = 10 THEN ph.Id END) AS TotalClosures,
// COUNT(DISTINCT CASE WHEN ph.PostHistoryTypeId = 11 THEN ph.Id END) AS TotalReopens,
// COUNT(DISTINCT CASE WHEN ph.PostHistoryTypeId IN (12, 13) THEN ph.Id END) AS TotalDeletions
// FROM PostHistory ph
// GROUP BY ph.UserId
// ),
// FinalStats AS (
// SELECT
// us.UserId,
// us.DisplayName,
// us.Reputation,
// us.TotalPosts,
// us.TotalQuestions,
// us.TotalAnswers,
// us.TotalBounty,
// us.TotalBadges,
// us.TotalViews,
// COALESCE(ps.TotalEdits, 0) AS TotalEdits,
// COALESCE(ps.TotalClosures, 0) AS TotalClosures,
// COALESCE(ps.TotalReopens, 0) AS TotalReopens,
// COALESCE(ps.TotalDeletions, 0) AS TotalDeletions
// FROM UserStats us
// LEFT JOIN PostHistoryStats ps ON us.UserId = ps.UserId
// )
// SELECT
// UserId,
// DisplayName,
// Reputation,
// TotalPosts,
// TotalQuestions,
// TotalAnswers,
// TotalBounty,
// TotalBadges,
// TotalViews,
// TotalEdits,
// TotalClosures,
// TotalReopens,
// TotalDeletions
// FROM FinalStats
// ORDER BY Reputation DESC, TotalPosts DESC
// LIMIT 10;
fn q6881(db: &'static So) -> String {
    let Post { post_type_id, view_count, .. } = &db.post;
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let uf = g(db)
        .select(posts_of(db).select(post_type_id.and(view_count.opt()).and(bounty.opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 5], |a, (p, c)| {
            let (w, b) = p.map_or((None, None), |((_, w), b)| (w, b.flatten()));
            [a[0] + b.is_some() as i64, a[1] + b.unwrap_or(0), a[2] + c.unwrap_or(0), a[3] + w.unwrap_or(0), 0]
        });
    let pq = g(db).select(posts_of(db).select(post_type_id).opt()).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64],
        None => a,
    });
    let ph = db.post_history.group_by(&db.post_history.user).select(&db.post_history.post_history_type_id).fold([0i64; 4], |a, t| {
        [a[0] + 1, a[1] + (t == 10) as i64, a[2] + (t == 11) as i64, a[3] + matches!(t, 12 | 13) as i64]
    });
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and(&pq).and(&uf).and((&ph).opt())).drive(|_, x| v.push(x));
    out(v, |&(((u, q), _), _)| (rep_desc(db, u), Reverse(q[0])), 10, |&(((u, q), a), h)| {
        let mut f = ["uid", "name", "rep"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.extend(ints(&q));
        f.extend([nullable(a[1], a[0]), V::I(a[2]), V::I(a[3])]);
        f.extend(ints(&h.unwrap_or([0; 4])));
        f
    })
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// U.CreationDate,
// COUNT(DISTINCT P.Id) AS TotalPosts,
// COUNT(DISTINCT CASE WHEN P.PostTypeId = 1 THEN P.Id END) AS TotalQuestions,
// COUNT(DISTINCT CASE WHEN P.PostTypeId = 2 THEN P.Id END) AS TotalAnswers,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// U.Id, U.DisplayName, U.Reputation, U.CreationDate
// ), BadgeStats AS (
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
// ), CombinedStats AS (
// SELECT
// US.UserId,
// US.DisplayName,
// US.Reputation,
// US.CreationDate,
// US.TotalPosts,
// US.TotalQuestions,
// US.TotalAnswers,
// US.TotalUpVotes,
// US.TotalDownVotes,
// COALESCE(BS.TotalBadges, 0) AS TotalBadges,
// COALESCE(BS.GoldBadges, 0) AS GoldBadges,
// COALESCE(BS.SilverBadges, 0) AS SilverBadges,
// COALESCE(BS.BronzeBadges, 0) AS BronzeBadges
// FROM
// UserStats US
// LEFT JOIN
// BadgeStats BS ON US.UserId = BS.UserId
// )
// SELECT
// UserId,
// DisplayName,
// Reputation,
// CreationDate,
// TotalPosts,
// TotalQuestions,
// TotalAnswers,
// TotalUpVotes,
// TotalDownVotes,
// TotalBadges,
// GoldBadges,
// SilverBadges,
// BronzeBadges
// FROM
// CombinedStats
// WHERE
// Reputation > 1000
// ORDER BY
// TotalPosts DESC, TotalUpVotes DESC
// FETCH FIRST 10 ROWS ONLY;
fn q6030(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::RepGt(1000), "v", any_post);
    let ps = pstat(db, db.post.iq());
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&us).and((&ps).opt()).and((&bc).opt()).drive(|u, ((a, p), b)| v.push((u, a, pz(p), bz(b))));
    out(v, |&(_, a, p, _)| (Reverse(p[0]), Reverse(a.up)), 10, |&(u, a, p, b)| {
        let mut f = ["uid", "name", "rep", "ucreated"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.extend(ints(&[p[0], p[1], p[2], a.up, a.down]));
        f.extend(ints(&b));
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
// WHERE
// u.Reputation > 1000
// GROUP BY
// u.Id, u.DisplayName
// ),
// PostVoteSummary AS (
// SELECT
// p.OwnerUserId,
// COUNT(v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Posts p
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY
// p.OwnerUserId
// ),
// PostStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS PostCount,
// COUNT(DISTINCT p.Id) FILTER (WHERE p.PostTypeId = 1) AS QuestionCount,
// COUNT(DISTINCT p.Id) FILTER (WHERE p.PostTypeId = 2) AS AnswerCount,
// AVG(p.Score) AS AvgScore,
// MAX(p.CreationDate) AS LastPostDate
// FROM
// Posts p
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY
// p.OwnerUserId
// )
// SELECT
// u.DisplayName,
// u.Reputation,
// ubc.BadgeCount,
// ubc.GoldBadges,
// ubc.SilverBadges,
// ubc.BronzeBadges,
// pvs.VoteCount,
// pvs.UpVotes,
// pvs.DownVotes,
// ps.PostCount,
// ps.QuestionCount,
// ps.AnswerCount,
// ps.AvgScore,
// ps.LastPostDate
// FROM
// Users u
// JOIN
// UserBadgeCounts ubc ON u.Id = ubc.UserId
// JOIN
// PostVoteSummary pvs ON u.Id = pvs.OwnerUserId
// JOIN
// PostStats ps ON u.Id = ps.OwnerUserId
// WHERE
// u.Reputation > 1000
// ORDER BY
// u.Reputation DESC,
// u.DisplayName ASC;
fn q25967(db: &'static So) -> String {
    let ub = ubc(db);
    let vc = owned_since(db, year_ago()).group_by(&db.post.owner_user).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| match t {
        Some(t) => [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64],
        None => a,
    });
    let Post { post_type_id, score, creation_date, .. } = &db.post;
    let ps = owned_since(db, year_ago()).group_by(&db.post.owner_user).select(post_type_id.and(score).and(creation_date)).fold([0, 0, 0, 0, i64::MIN], |a: [i64; 5], ((t, s), c)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4].max(c)]
    });
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and(&ub).and(&vc).and(&ps)).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(((u, b), x), p)| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(ints(&b));
        f.extend(ints(&x));
        f.extend([V::I(p[0]), V::I(p[1]), V::I(p[2]), avg(p[3], p[0]), V::T(p[4])]);
        row(f)
    }))
}

// WITH UserVoteStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(V.Id) AS VoteCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM Users U
// LEFT JOIN Votes V ON U.Id = V.UserId
// GROUP BY U.Id, U.DisplayName
// ),
// RecentPostStats AS (
// SELECT
// P.OwnerUserId,
// COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS QuestionCount,
// COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS AnswerCount,
// AVG(P.Score) AS AverageScore
// FROM Posts P
// WHERE P.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 YEAR'
// GROUP BY P.OwnerUserId
// ),
// ClosedPostStats AS (
// SELECT
// PH.UserId,
// COUNT(PH.Id) AS ClosedPostCount,
// MIN(PH.CreationDate) AS FirstCloseDate
// FROM PostHistory PH
// WHERE PH.PostHistoryTypeId IN (10, 11)
// GROUP BY PH.UserId
// ),
// FinalStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COALESCE(UV.VoteCount, 0) AS TotalVotes,
// COALESCE(RP.QuestionCount, 0) AS TotalQuestions,
// COALESCE(RP.AnswerCount, 0) AS TotalAnswers,
// COALESCE(CP.ClosedPostCount, 0) AS TotalClosedPosts,
// COALESCE(CP.FirstCloseDate, '1970-01-01') AS FirstClosedDate,
// COALESCE(UV.UpVotes, 0) AS UpVotes,
// COALESCE(UV.DownVotes, 0) AS DownVotes,
// COALESCE(RP.AverageScore, 0) AS AverageScore
// FROM Users U
// LEFT JOIN UserVoteStats UV ON U.Id = UV.UserId
// LEFT JOIN RecentPostStats RP ON U.Id = RP.OwnerUserId
// LEFT JOIN ClosedPostStats CP ON U.Id = CP.UserId
// )
// SELECT
// UserId,
// DisplayName,
// TotalVotes,
// TotalQuestions,
// TotalAnswers,
// TotalClosedPosts,
// FirstClosedDate,
// UpVotes,
// DownVotes,
// AverageScore
// FROM FinalStats
// WHERE TotalVotes > 10
// OR (TotalQuestions > 5 AND AverageScore > 0)
// ORDER BY TotalVotes DESC, FirstClosedDate ASC
// LIMIT 100;
fn q20618(db: &'static So) -> String {
    let uv = uvotes(db);
    let Post { post_type_id, score, .. } = &db.post;
    let rp = owned_since(db, year_ago()).group_by(&db.post.owner_user).select(post_type_id.and(score)).fold([0i64; 4], |a, (t, s)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s]);
    let cp = db.post_history.with((&db.post_history.post_history_type_id).filt(|t: i64| matches!(t, 10 | 11))).group_by(&db.post_history.user).select(&db.post_history.creation_date).fold((0i64, i64::MAX), |(n, m), d| (n + 1, m.min(d)));
    let mut v = Vec::new();
    db.user
        .select(Ident::<User>::new().and((&uv).opt()).and((&rp).opt()).and((&cp).opt()))
        .filt(|(((_, x), p), _): (((Id<User>, Option<[i64; 3]>), Option<[i64; 4]>), Option<(i64, i64)>)| {
            let n = x.map_or(0, |x| x[0]);
            let p = p.unwrap_or([0; 4]);
            n > 10 || (p[1] > 5 && p[3] > 0)
        })
        .drive(|_, (((u, x), p), c)| v.push((u, x.unwrap_or([0; 3]), p.unwrap_or([0; 4]), c)));
    out(v, |&(_, x, _, c)| (Reverse(x[0]), c.map_or(0, |c| c.1)), 100, |&(u, x, p, c)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&[x[0], p[1], p[2], c.map_or(0, |c| c.0)]));
        f.push(V::T(c.map_or(0, |c| c.1)));
        f.extend([V::I(x[1]), V::I(x[2]), or0(p[3], p[0])]);
        f
    })
}

// WITH UserBadges AS (
// SELECT
// U.Id AS UserId,
// COUNT(CASE WHEN B.Class = 1 THEN 1 END) AS GoldBadges,
// COUNT(CASE WHEN B.Class = 2 THEN 1 END) AS SilverBadges,
// COUNT(CASE WHEN B.Class = 3 THEN 1 END) AS BronzeBadges
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
// SUM(COALESCE(P.ViewCount, 0)) AS TotalViews,
// SUM(COALESCE(P.Score, 0)) AS TotalScore,
// AVG(COALESCE(P.AnswerCount, 0)) AS AvgAnswers
// FROM
// Posts P
// WHERE
// P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY
// P.OwnerUserId
// ),
// VoteSummary AS (
// SELECT
// V.UserId,
// SUM(CASE WHEN V.VoteTypeId IN (2, 5) THEN 1 ELSE 0 END) AS TotalUpvotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes
// FROM
// Votes V
// GROUP BY
// V.UserId
// )
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COALESCE(UB.GoldBadges, 0) AS GoldBadges,
// COALESCE(UB.SilverBadges, 0) AS SilverBadges,
// COALESCE(UB.BronzeBadges, 0) AS BronzeBadges,
// COALESCE(PS.TotalPosts, 0) AS TotalPosts,
// COALESCE(PS.TotalViews, 0) AS TotalViews,
// COALESCE(PS.TotalScore, 0) AS TotalScore,
// COALESCE(PS.AvgAnswers, 0) AS AvgAnswers,
// COALESCE(VS.TotalUpvotes, 0) AS TotalUpvotes,
// COALESCE(VS.TotalDownvotes, 0) AS TotalDownvotes,
// CASE
// WHEN COALESCE(UB.GoldBadges, 0) > 0 THEN 'Gold Badge Holder'
// WHEN COALESCE(UB.SilverBadges, 0) > 0 THEN 'Silver Badge Holder'
// ELSE 'No Badges'
// END AS BadgeStatus
// FROM
// Users U
// LEFT JOIN
// UserBadges UB ON U.Id = UB.UserId
// LEFT JOIN
// PostStats PS ON U.Id = PS.OwnerUserId
// LEFT JOIN
// VoteSummary VS ON U.Id = VS.UserId
// WHERE
// U.Reputation > 1000
// ORDER BY
// TotalScore DESC, TotalViews DESC
// FETCH FIRST 10 ROWS ONLY;
fn q1877(db: &'static So) -> String {
    let bc = badge_classes(db);
    let ps = pstat(db, since(db, year_ago()));
    let vs = db.vote.group_by(&db.vote.user).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + matches!(t, 2 | 5) as i64, a[1] + (t == 3) as i64]);
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and((&bc).opt()).and((&ps).opt()).and((&vs).opt())).drive(|_, (((u, b), p), x)| v.push((u, bz(b), pz(p), x.unwrap_or([0; 2]))));
    out(v, |&(_, _, p, _)| (Reverse(p[3]), Reverse(p[5])), 10, |&(u, b, p, x)| {
        let st = if b[1] > 0 { "Gold Badge Holder" } else if b[2] > 0 { "Silver Badge Holder" } else { "No Badges" };
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&[b[1], b[2], b[3], p[0], p[5], p[3]]));
        f.push(or0(p[7], p[0]));
        f.extend(ints(&x));
        f.push(V::S(st));
        f
    })
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS TotalQuestions,
// COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS TotalAnswers,
// SUM(p.Score) AS TotalScore,
// SUM(COALESCE(vs.UpVotes, 0)) AS TotalUpVotes,
// SUM(COALESCE(vs.DownVotes, 0)) AS TotalDownVotes
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// (SELECT
// PostId,
// SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Votes
// GROUP BY
// PostId) vs ON p.Id = vs.PostId
// GROUP BY
// u.Id, u.DisplayName
// ),
// PostActivity AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.AnswerCount,
// p.CommentCount,
// MAX(ph.CreationDate) AS LastEditDate,
// COUNT(c.Id) AS TotalComments
// FROM
// Posts p
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.AnswerCount, p.CommentCount
// ),
// BenchmarkResults AS (
// SELECT
// ups.UserId,
// ups.DisplayName,
// ups.TotalPosts,
// ups.TotalQuestions,
// ups.TotalAnswers,
// ups.TotalScore,
// ups.TotalUpVotes,
// ups.TotalDownVotes,
// pa.PostId,
// pa.Title,
// pa.CreationDate,
// pa.ViewCount,
// pa.AnswerCount,
// pa.CommentCount,
// pa.LastEditDate,
// pa.TotalComments
// FROM
// UserPostStats ups
// LEFT JOIN
// PostActivity pa ON ups.UserId = pa.PostId  -- Fixed Join Condition
// )
// SELECT
// *
// FROM
// BenchmarkResults
// ORDER BY
// TotalScore DESC, TotalPosts DESC;
fn q11591(db: &'static So) -> String {
    let pid = pids(db);
    let pv = post_votes(db);
    let Post { post_type_id, score, .. } = &db.post;
    let uf = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(score).and((&pv).opt())).fold([0i64; 6], |a, ((t, s), x)| {
        let x = x.unwrap_or([0; 3]);
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + x[1], a[5] + x[2]]
    });
    let pa = db
        .post
        .group_by(Ident::<Post>::new())
        .select(history_of(db).select(&db.post_history.creation_date).opt().and(comments_of(db).opt()))
        .fold((i64::MIN, 0i64), |(m, n), (h, c)| (h.map_or(m, |d| m.max(d)), n + c.is_some() as i64));
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&uf).opt()).and((&db.user.origid).select(&pid).select(Ident::<Post>::new().and(&pa)).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((u, a), p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(match a {
            Some(a) => vec![V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4]), V::I(a[5])],
            None => vec![V::I(0), V::I(0), V::I(0), V::Null, V::I(0), V::I(0)],
        });
        f.extend(match p {
            Some((p, (h, c))) => {
                let mut g = post_fields(db, p, &["id", "title", "created", "views", "answers", "comments"]);
                g.extend([tmax(h), V::I(c)]);
                g
            }
            None => nulls(8),
        });
        row(f)
    }))
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
// RecentPosts AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(CASE WHEN p.PostTypeId = 10 THEN 1 ELSE 0 END) AS ClosedPosts
// FROM
// Posts p
// WHERE
// p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days'
// GROUP BY
// p.OwnerUserId
// ),
// UserVoteStats AS (
// SELECT
// v.UserId,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Votes v
// GROUP BY
// v.UserId
// )
// SELECT
// u.DisplayName,
// COALESCE(ub.BadgeCount, 0) AS BadgeCount,
// COALESCE(ub.GoldBadges, 0) AS GoldBadges,
// COALESCE(ub.SilverBadges, 0) AS SilverBadges,
// COALESCE(ub.BronzeBadges, 0) AS BronzeBadges,
// COALESCE(rp.TotalPosts, 0) AS TotalPosts,
// COALESCE(rp.Questions, 0) AS Questions,
// COALESCE(rp.Answers, 0) AS Answers,
// COALESCE(rp.ClosedPosts, 0) AS ClosedPosts,
// COALESCE(uvs.UpVotes, 0) AS UpVotes,
// COALESCE(uvs.DownVotes, 0) AS DownVotes,
// CASE
// WHEN COALESCE(rp.ClosedPosts, 0) > 0 THEN 'Has Closed Posts'
// ELSE 'No Closed Posts'
// END AS PostStatus
// FROM
// Users u
// LEFT JOIN
// UserBadges ub ON u.Id = ub.UserId
// LEFT JOIN
// RecentPosts rp ON u.Id = rp.OwnerUserId
// LEFT JOIN
// UserVoteStats uvs ON u.Id = uvs.UserId
// WHERE
// u.Reputation > 1000
// ORDER BY
// BadgeCount DESC, TotalPosts DESC
// LIMIT 50;
fn q4532(db: &'static So) -> String {
    let bc = badge_classes(db);
    let rp = owned_since(db, date(2024, 9, 1)).group_by(&db.post.owner_user).select(&db.post.post_type_id).fold([0i64; 4], |a, t| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (t == 10) as i64]);
    let uv = uvotes(db);
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and((&bc).opt()).and((&rp).opt()).and((&uv).opt())).drive(|_, (((u, b), p), x)| v.push((u, bz(b), p.unwrap_or([0; 4]), x.unwrap_or([0; 3]))));
    out(v, |&(_, b, p, _)| (Reverse(b[0]), Reverse(p[0])), 50, |&(u, b, p, x)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(ints(&b));
        f.extend(ints(&p));
        f.extend([V::I(x[1]), V::I(x[2]), V::S(if p[3] > 0 { "Has Closed Posts" } else { "No Closed Posts" })]);
        f
    })
}

// WITH UserActivity AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COALESCE(SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END), 0) AS QuestionCount,
// COALESCE(SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS AnswerCount,
// COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
// COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// U.Id, U.DisplayName
// ),
// RecentEdits AS (
// SELECT
// PH.UserId,
// COUNT(*) AS EditCount,
// MAX(PH.CreationDate) AS LastEditDate
// FROM
// PostHistory PH
// WHERE
// PH.PostHistoryTypeId IN (4, 5, 6)
// GROUP BY
// PH.UserId
// ),
// UserReputation AS (
// SELECT
// U.Id AS UserId,
// U.Reputation,
// COALESCE(UA.QuestionCount, 0) AS QuestionCount,
// COALESCE(UA.AnswerCount, 0) AS AnswerCount,
// COALESCE(RE.EditCount, 0) AS RecentEdits,
// RE.LastEditDate
// FROM
// Users U
// LEFT JOIN
// UserActivity UA ON U.Id = UA.UserId
// LEFT JOIN
// RecentEdits RE ON U.Id = RE.UserId
// )
// SELECT
// UR.UserId,
// UR.Reputation,
// UR.QuestionCount,
// UR.AnswerCount,
// UR.RecentEdits,
// CASE
// WHEN UR.Reputation > 1000 THEN 'High'
// WHEN UR.Reputation BETWEEN 500 AND 1000 THEN 'Medium'
// ELSE 'Low'
// END AS ReputationCategory,
// (SELECT COUNT(*) FROM Badges B WHERE B.UserId = UR.UserId AND B.Class = 1) AS GoldBadges,
// (SELECT COUNT(*) FROM Badges B WHERE B.UserId = UR.UserId AND B.Class = 2) AS SilverBadges,
// (SELECT COUNT(*) FROM Badges B WHERE B.UserId = UR.UserId AND B.Class = 3) AS BronzeBadges
// FROM
// UserReputation UR
// WHERE
// (UR.QuestionCount > 5 OR UR.AnswerCount > 10)
// AND (UR.Reputation IS NOT NULL)
// ORDER BY
// UR.Reputation DESC, UR.UserId
// LIMIT 10;
fn q3679(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let re = db.post_history.with((&db.post_history.post_history_type_id).filt(|t: i64| matches!(t, 4 | 5 | 6))).group_by(&db.post_history.user).select(&db.post_history.post).fold(0i64, |a, _| a + 1);
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&us).filt(|a: UStats| a.q > 5 || a.a > 10).and((&re).opt()).and((&bc).opt()).drive(|u, ((a, e), b)| v.push((u, a, e.unwrap_or(0), bz(b))));
    out(v, |&(u, ..)| (rep_desc(db, u), db.user.origid.get(u).unwrap()), 10, |&(u, a, e, b)| {
        let r = db.user.reputation.get(u).unwrap();
        let cat = if r > 1000 { "High" } else if (500..=1000).contains(&r) { "Medium" } else { "Low" };
        vec![user_col(db, u, "uid"), V::I(r), V::I(a.q), V::I(a.a), V::I(e), V::S(cat), V::I(b[1]), V::I(b[2]), V::I(b[3])]
    })
}

// WITH UserBadgeStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(B.Id) AS BadgeCount,
// SUM(CASE
// WHEN B.Class = 1 THEN 1
// ELSE 0
// END) AS GoldBadges,
// SUM(CASE
// WHEN B.Class = 2 THEN 1
// ELSE 0
// END) AS SilverBadges,
// SUM(CASE
// WHEN B.Class = 3 THEN 1
// ELSE 0
// END) AS BronzeBadges
// FROM Users U
// LEFT JOIN Badges B ON U.Id = B.UserId
// GROUP BY U.Id, U.DisplayName
// ),
// PostInteractionStats AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS TotalPosts,
// COUNT(C.Id) AS TotalComments,
// COUNT(DISTINCT V.Id) AS TotalVotes,
// SUM(P.ViewCount) AS TotalViews,
// SUM(P.Score) AS TotalScore
// FROM Posts P
// LEFT JOIN Comments C ON P.Id = C.PostId
// LEFT JOIN Votes V ON P.Id = V.PostId
// GROUP BY P.OwnerUserId
// ),
// UserEngagement AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COALESCE(UBS.BadgeCount, 0) AS BadgeCount,
// COALESCE(UBS.GoldBadges, 0) AS GoldBadges,
// COALESCE(UBS.SilverBadges, 0) AS SilverBadges,
// COALESCE(UBS.BronzeBadges, 0) AS BronzeBadges,
// COALESCE(PIS.TotalPosts, 0) AS TotalPosts,
// COALESCE(PIS.TotalComments, 0) AS TotalComments,
// COALESCE(PIS.TotalVotes, 0) AS TotalVotes,
// COALESCE(PIS.TotalViews, 0) AS TotalViews,
// COALESCE(PIS.TotalScore, 0) AS TotalScore
// FROM Users U
// LEFT JOIN UserBadgeStats UBS ON U.Id = UBS.UserId
// LEFT JOIN PostInteractionStats PIS ON U.Id = PIS.OwnerUserId
// )
// SELECT
// UserId,
// DisplayName,
// BadgeCount,
// GoldBadges,
// SilverBadges,
// BronzeBadges,
// TotalPosts,
// TotalComments,
// TotalVotes,
// TotalViews,
// TotalScore,
// (TotalScore * 1.0 / NULLIF(TotalPosts, 0)) AS ScorePerPost,
// (TotalViews * 1.0 / NULLIF(TotalPosts, 0)) AS ViewsPerPost
// FROM UserEngagement
// ORDER BY TotalScore DESC, BadgeCount DESC
// LIMIT 10;
fn q29215(db: &'static So) -> String {
    let ub = ubc(db);
    let Post { score, view_count, .. } = &db.post;
    let pf = owned(db)
        .group_by(&db.post.owner_user)
        .select(score.and(view_count.opt()).and(comments_of(db).opt()).and(votes_of(db).opt()))
        .fold([0i64; 6], |a, (((s, w), c), _)| [a[0] + 1, a[1] + c.is_some() as i64, 0, a[3] + w.unwrap_or(0), a[4] + s, 0]);
    let dv = owned(db).group_by(&db.post.owner_user).select(votes_of(db)).fold(0i64, |n, _| n + 1);
    let mut v = Vec::new();
    (&ub).and((&pf).opt()).and((&dv).opt()).drive(|u, ((b, p), d)| {
        let mut p = p.unwrap_or([0; 6]);
        p[2] = d.unwrap_or(0);
        v.push((u, b, p))
    });
    out(v, |&(_, b, p)| (Reverse(p[4]), Reverse(b[0])), 10, |&(u, b, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&b));
        f.extend(ints(&p[..5]));
        f.extend([ratio(p[4], p[0]), ratio(p[3], p[0])]);
        f
    })
}

// WITH UserBadges AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(b.Id) AS BadgeCount,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldCount,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverCount,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeCount
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
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN p.ViewCount > 1000 THEN 1 ELSE 0 END) AS PopularPostCount
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// ),
// UserActivity AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// ub.BadgeCount,
// ub.GoldCount,
// ub.SilverCount,
// ub.BronzeCount,
// ups.PostCount,
// ups.QuestionCount,
// ups.AnswerCount,
// ups.PopularPostCount
// FROM
// Users u
// LEFT JOIN
// UserBadges ub ON u.Id = ub.UserId
// LEFT JOIN
// UserPostStats ups ON u.Id = ups.OwnerUserId
// )
// SELECT
// ua.UserId,
// ua.DisplayName,
// ua.BadgeCount,
// ua.GoldCount,
// ua.SilverCount,
// ua.BronzeCount,
// COALESCE(ua.PostCount, 0) AS PostCount,
// COALESCE(ua.QuestionCount, 0) AS QuestionCount,
// COALESCE(ua.AnswerCount, 0) AS AnswerCount,
// COALESCE(ua.PopularPostCount, 0) AS PopularPostCount,
// (SELECT COUNT(*) FROM Votes v WHERE v.UserId = ua.UserId AND v.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year') AS RecentVotes,
// (SELECT COUNT(*) FROM Comments c WHERE c.UserId = ua.UserId AND c.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year') AS RecentComments
// FROM
// UserActivity ua
// ORDER BY
// ua.BadgeCount DESC,
// ua.PostCount DESC
// LIMIT 10;
fn q29353(db: &'static So) -> String {
    let ub = ubc(db);
    let Post { post_type_id, view_count, .. } = &db.post;
    let ps = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(view_count.opt())).fold([0i64; 4], |a, (t, w)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (w.unwrap_or(0) > 1000) as i64]);
    let y = year_ago();
    let rv = db.vote.with((&db.vote.creation_date).ge(y)).group_by(&db.vote.user).select(&db.vote.vote_type_id).fold(0i64, |a, _| a + 1);
    let rc = db.comment.with((&db.comment.creation_date).ge(y)).group_by(&db.comment.user).select(&db.comment.score).fold(0i64, |a, _| a + 1);
    let mut v = Vec::new();
    (&ub).and((&ps).opt()).and((&rv).opt()).and((&rc).opt()).drive(|u, (((b, p), x), c)| v.push((u, b, p.unwrap_or([0; 4]), x.unwrap_or(0), c.unwrap_or(0))));
    out(v, |&(_, b, p, ..)| (Reverse(b[0]), Reverse(p[0])), 10, |&(u, b, p, x, c)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&b));
        f.extend(ints(&p));
        f.extend(ints(&[x, c]));
        f
    })
}

// WITH UserBadges AS (
// SELECT
// b.UserId,
// COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldCount,
// COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverCount,
// COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeCount
// FROM
// Badges b
// GROUP BY
// b.UserId
// ),
// PostStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS TotalPosts,
// SUM(COALESCE(p.Score, 0)) AS TotalScore,
// AVG(COALESCE(p.ViewCount, 0)) AS AverageViews
// FROM
// Posts p
// WHERE
// p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY
// p.OwnerUserId
// ),
// ClosedPosts AS (
// SELECT
// ph.UserId,
// COUNT(ph.PostId) AS ClosedCount
// FROM
// PostHistory ph
// WHERE
// ph.PostHistoryTypeId = 10
// GROUP BY
// ph.UserId
// ),
// FinalStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COALESCE(ub.GoldCount, 0) AS GoldBadges,
// COALESCE(ub.SilverCount, 0) AS SilverBadges,
// COALESCE(ub.BronzeCount, 0) AS BronzeBadges,
// COALESCE(ps.TotalPosts, 0) AS TotalPosts,
// COALESCE(ps.TotalScore, 0) AS TotalScore,
// COALESCE(ps.AverageViews, 0) AS AverageViews,
// COALESCE(cp.ClosedCount, 0) AS ClosedPosts
// FROM
// Users u
// LEFT JOIN
// UserBadges ub ON u.Id = ub.UserId
// LEFT JOIN
// PostStats ps ON u.Id = ps.OwnerUserId
// LEFT JOIN
// ClosedPosts cp ON u.Id = cp.UserId
// )
// SELECT
// UserId,
// DisplayName,
// GoldBadges,
// SilverBadges,
// BronzeBadges,
// TotalPosts,
// TotalScore,
// AverageViews,
// ClosedPosts,
// CASE
// WHEN TotalPosts > 10 AND ClosedPosts > 5 THEN 'Active Contributor'
// WHEN ClosedPosts = 0 THEN 'Non-Contributor'
// ELSE 'Moderate Contributor'
// END AS ContributorStatus
// FROM
// FinalStats
// WHERE
// (TotalPosts > 0 OR GoldBadges > 0 OR SilverBadges > 0 OR BronzeBadges > 0)
// ORDER BY
// TotalScore DESC, DisplayName;
fn q3417(db: &'static So) -> String {
    let bc = badge_classes(db);
    let Post { score, view_count, .. } = &db.post;
    let ps = db.post.with((&db.post.creation_date).gt(year_ago())).group_by(&db.post.owner_user).select(score.and(view_count.opt())).fold([0i64; 3], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.unwrap_or(0)]);
    let cp = db.post_history.with((&db.post_history.post_history_type_id).eq(10)).group_by(&db.post_history.user).select(&db.post_history.post).fold(0i64, |a, _| a + 1);
    let mut v = Vec::new();
    db.user
        .select(Ident::<User>::new().and((&bc).opt()).and((&ps).opt()).and((&cp).opt()))
        .filt(|(((_, b), p), _): (((Id<User>, Option<[i64; 4]>), Option<[i64; 3]>), Option<i64>)| {
            let b = bz(b);
            p.is_some() || b[1] > 0 || b[2] > 0 || b[3] > 0
        })
        .drive(|_, (((u, b), p), c)| v.push((u, bz(b), p.unwrap_or([0; 3]), c.unwrap_or(0))));
    rows(v.iter().map(|&(u, b, p, c)| {
        let st = if p[0] > 10 && c > 5 { "Active Contributor" } else if c == 0 { "Non-Contributor" } else { "Moderate Contributor" };
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&[b[1], b[2], b[3], p[0], p[1]]));
        f.extend([or0(p[2], p[0]), V::I(c), V::S(st)]);
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
// SUM(CASE WHEN P.PostTypeId = 10 THEN 1 ELSE 0 END) AS ClosedPosts,
// SUM(CASE WHEN P.LastActivityDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' THEN 1 ELSE 0 END) AS RecentActivity
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// WHERE
// U.Reputation > 100
// GROUP BY
// U.Id, U.DisplayName
// ), RecentVotes AS (
// SELECT
// V.UserId,
// COUNT(V.Id) AS TotalVotes,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Votes V
// WHERE
// V.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'
// GROUP BY
// V.UserId
// ), BadgeSummary AS (
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
// UA.UserId,
// UA.DisplayName,
// UA.TotalPosts,
// UA.Questions,
// UA.Answers,
// UA.ClosedPosts,
// UA.RecentActivity,
// COALESCE(RV.TotalVotes, 0) AS TotalVotes,
// COALESCE(RV.UpVotes, 0) AS UpVotes,
// COALESCE(RV.DownVotes, 0) AS DownVotes,
// COALESCE(BS.TotalBadges, 0) AS TotalBadges,
// COALESCE(BS.GoldBadges, 0) AS GoldBadges,
// COALESCE(BS.SilverBadges, 0) AS SilverBadges,
// COALESCE(BS.BronzeBadges, 0) AS BronzeBadges
// FROM
// UserActivity UA
// LEFT JOIN
// RecentVotes RV ON UA.UserId = RV.UserId
// LEFT JOIN
// BadgeSummary BS ON UA.UserId = BS.UserId
// ORDER BY
// UA.TotalPosts DESC
// LIMIT 100;
fn q6422(db: &'static So) -> String {
    let m = month_ago();
    let Post { post_type_id, last_activity_date, .. } = &db.post;
    let ua = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(last_activity_date)).fold([0i64; 5], move |a, (t, d)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (t == 10) as i64, a[4] + (d >= m) as i64]
    });
    let rv = db.vote.with((&db.vote.creation_date).ge(m)).group_by(&db.vote.user).select(&db.vote.vote_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64]);
    let bc = badge_classes(db);
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(100)).select(Ident::<User>::new().and((&ua).opt()).and((&rv).opt()).and((&bc).opt())).drive(|_, (((u, a), x), b)| v.push((u, a.unwrap_or([0; 5]), x.unwrap_or([0; 3]), bz(b))));
    out(v, |&(_, a, ..)| Reverse(a[0]), 100, |&(u, a, x, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a));
        f.extend(ints(&x));
        f.extend(ints(&b));
        f
    })
}

// WITH PostTagProcessing AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.Body,
// p.CreationDate,
// p.OwnerUserId,
// p.Tags,
// ARRAY_LENGTH(string_to_array(substring(p.Tags, 2, length(p.Tags) - 2), '>'), 1) AS TagCount,
// COALESCE(u.DisplayName, 'Community User') AS OwnerDisplayName,
// COUNT(c.Id) AS CommentCount,
// COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (8, 9)
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, p.Body, p.CreationDate, p.OwnerUserId, p.Tags, u.DisplayName
// ),
// PostHistoryAnalysis AS (
// SELECT
// ph.PostId,
// MAX(ph.CreationDate) AS LastEditDate,
// COUNT(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 END) AS CloseCount,
// COUNT(CASE WHEN ph.PostHistoryTypeId IN (24, 25) THEN 1 END) AS EditCount,
// COUNT(DISTINCT ph.UserId) AS UniqueEditors
// FROM
// PostHistory ph
// GROUP BY
// ph.PostId
// ),
// FinalBenchmarking AS (
// SELECT
// ptp.PostId,
// ptp.Title,
// ptp.TagCount,
// ptp.CommentCount,
// pha.LastEditDate,
// pha.CloseCount,
// pha.EditCount,
// pha.UniqueEditors,
// ptp.TotalBounty,
// (COALESCE(ptp.CommentCount, 0) + COALESCE(pha.EditCount, 0))::float / NULLIF(ptp.TagCount, 0) AS InteractionToTagRatio
// FROM
// PostTagProcessing ptp
// JOIN
// PostHistoryAnalysis pha ON ptp.PostId = pha.PostId
// ORDER BY
// ptp.TagCount DESC,
// ptp.CommentCount DESC
// )
// SELECT
// *,
// CASE
// WHEN InteractionToTagRatio > 2 THEN 'Highly Interactive'
// WHEN InteractionToTagRatio BETWEEN 1 AND 2 THEN 'Moderately Interactive'
// ELSE 'Less Interactive'
// END AS InteractionCategory
// FROM
// FinalBenchmarking
// WHERE
// TagCount >= 3
fn q25760(db: &'static So) -> String {
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).filt(|t: i64| matches!(t, 8 | 9)))).select((&db.vote.bounty_amount).opt());
    let cb = questions_only(db)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(bounty.opt()))
        .fold([0i64; 2], |a, (c, b)| [a[0] + c.is_some() as i64, a[1] + b.flatten().unwrap_or(0)]);
    let ph = db.post_history.group_by(&db.post_history.post).select((&db.post_history.post_history_type_id).and(&db.post_history.creation_date)).fold([i64::MIN, 0, 0], |a: [i64; 3], (t, d)| {
        [a[0].max(d), a[1] + (t == 10) as i64, a[2] + matches!(t, 24 | 25) as i64]
    });
    let pu = db.post_history.group_by(&db.post_history.post).select(&db.post_history.user).count_distinct();
    let mut v = Vec::new();
    questions_only(db)
        .select(Ident::<Post>::new().and((&db.post.tags_str).map(|t: Str| split_n(t, ">")).opt()).and(&cb).and(&ph).and((&pu).opt()))
        .filt(|((((_, t), _), _), _): ((((Id<Post>, Option<i64>), [i64; 2]), [i64; 3]), Option<i64>)| t.map_or(false, |t| t >= 3))
        .drive(|_, x| v.push(x));
    rows(v.iter().map(|&((((p, t), b), h), e)| {
        let t = t.unwrap();
        let cx = b[0];
        let r = ((cx + h[2]) as f32) / t as f32;
        let cat = if r > 2.0 { "Highly Interactive" } else if (1.0..=2.0).contains(&r) { "Moderately Interactive" } else { "Less Interactive" };
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend(ints(&[t, cx]));
        f.push(V::T(h[0]));
        f.extend(ints(&[h[1], h[2], e.unwrap_or(0), b[1]]));
        f.extend([V::F(r as f64), V::S(cat)]);
        row(f)
    }))
}

// WITH UserPostStats AS (
// SELECT u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// AVG(u.Reputation) AS AverageReputation,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN Votes v ON p.Id = v.PostId
// GROUP BY u.Id, u.DisplayName
// ),
// PostInteractions AS (
// SELECT p.Id AS PostId,
// p.Title,
// p.CreationDate,
// COUNT(c.Id) AS CommentCount,
// COUNT(pl.RelatedPostId) AS RelatedPostLinks,
// MAX(CASE WHEN ph.PostHistoryTypeId = 10 THEN ph.CreationDate END) AS ClosedDate,
// MAX(CASE WHEN ph.PostHistoryTypeId = 11 THEN ph.CreationDate END) AS ReopenedDate
// FROM Posts p
// LEFT JOIN Comments c ON p.Id = c.PostId
// LEFT JOIN PostLinks pl ON p.Id = pl.PostId
// LEFT JOIN PostHistory ph ON p.Id = ph.PostId
// GROUP BY p.Id, p.Title, p.CreationDate
// ),
// CombinedStats AS (
// SELECT ups.UserId,
// ups.DisplayName,
// ups.TotalPosts,
// ups.TotalQuestions,
// ups.TotalAnswers,
// ups.AverageReputation,
// ups.TotalUpvotes,
// ups.TotalDownvotes,
// pi.PostId,
// pi.Title,
// pi.CreationDate,
// pi.CommentCount,
// pi.RelatedPostLinks,
// pi.ClosedDate,
// pi.ReopenedDate
// FROM UserPostStats ups
// JOIN PostInteractions pi ON ups.UserId = pi.PostId
// )
// SELECT *,
// CASE
// WHEN ClosedDate IS NOT NULL THEN 'Closed'
// WHEN ReopenedDate IS NOT NULL THEN 'Reopened'
// ELSE 'Active'
// END AS PostStatus
// FROM CombinedStats
// WHERE AverageReputation > 100
// ORDER BY TotalPosts DESC, AverageReputation DESC
// LIMIT 50;
fn q7694(db: &'static So) -> String {
    let uid = uids(db);
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let mut v = Vec::new();
    stats_fold(db, db.post.with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "clh", &[])
        .and((&db.post.origid).select(&uid).with((&db.user.reputation).gt(100)).select(Ident::<User>::new().and(&us).and((&dp).opt())))
        .drive(|p, (s, ((u, a), d))| v.push((p, s, u, a, d.unwrap_or(0))));
    out(v, |&(_, _, u, _, d)| (Reverse(d), rep_desc(db, u)), 50, |&(p, s, u, a, d)| {
        let cl = if s.h10 > 0 { Some(s.h10max) } else { None };
        let ro = if s.h11 > 0 { Some(s.h11max) } else { None };
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(d), V::I(a.q), V::I(a.a), V::F(db.user.reputation.get(u).unwrap() as f64), V::I(a.up), V::I(a.down)];
        f.extend(post_fields(db, p, &["id", "title", "created"]));
        f.extend([V::I(s.cx), V::I(s.lx), ots(cl), ots(ro)]);
        f.push(V::S(if cl.is_some() { "Closed" } else if ro.is_some() { "Reopened" } else { "Active" }));
        f
    })
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// COUNT(DISTINCT b.Id) AS BadgeCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// u.Id, u.DisplayName
// ),
// PostHistoryDetails AS (
// SELECT
// ph.UserId,
// ph.PostId,
// p.Title,
// MAX(ph.CreationDate) AS LastEditDate,
// MAX(CASE WHEN ph.PostHistoryTypeId IN (4, 5, 6) THEN ph.CreationDate END) AS LastContentEditDate,
// MAX(CASE WHEN ph.PostHistoryTypeId = 10 THEN ph.CreationDate END) AS LastClosedDate
// FROM
// PostHistory ph
// JOIN
// Posts p ON ph.PostId = p.Id
// GROUP BY
// ph.UserId, ph.PostId, p.Title
// ),
// FinalStats AS (
// SELECT
// us.UserId,
// us.DisplayName,
// us.PostCount,
// us.QuestionCount,
// us.AnswerCount,
// us.UpVotes,
// us.DownVotes,
// us.BadgeCount,
// COUNT(DISTINCT phd.PostId) AS EditedPostsCount,
// COUNT(DISTINCT phd.PostId) FILTER (WHERE phd.LastClosedDate IS NOT NULL) AS ClosedPostsCount
// FROM
// UserStats us
// LEFT JOIN
// PostHistoryDetails phd ON us.UserId = phd.UserId
// GROUP BY
// us.UserId, us.DisplayName, us.PostCount, us.QuestionCount, us.AnswerCount,
// us.UpVotes, us.DownVotes, us.BadgeCount
// )
// SELECT
// UserId,
// DisplayName,
// PostCount,
// QuestionCount,
// AnswerCount,
// UpVotes,
// DownVotes,
// BadgeCount,
// EditedPostsCount,
// ClosedPostsCount
// FROM
// FinalStats
// ORDER BY
// PostCount DESC, UpVotes DESC
// LIMIT 10;
fn q9133(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "vb", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let bu = badges_per_user(db);
    let ep = db.post_history.group_by(&db.post_history.user).select(&db.post_history.post).count_distinct();
    let cp = db.post_history.with((&db.post_history.post_history_type_id).eq(10)).group_by(&db.post_history.user).select(&db.post_history.post).count_distinct();
    let mut v = Vec::new();
    (&us).and((&dp).opt()).and(&bu).and((&ep).opt()).and((&cp).opt()).drive(|u, ((((a, d), b), e), c)| v.push((u, a, d.unwrap_or(0), b, e.unwrap_or(0), c.unwrap_or(0))));
    out(v, |&(_, a, d, ..)| (Reverse(d), Reverse(a.up)), 10, |&(u, a, d, b, e, c)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&[d, a.q, a.a, a.up, a.down, b, e, c]));
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
// FROM Users u
// LEFT JOIN Badges b ON u.Id = b.UserId
// GROUP BY u.Id, u.DisplayName
// ),
// PostAggregates AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS TotalPosts,
// COUNT(DISTINCT p.ParentId) AS AnsweredQuestions,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN p.ViewCount > 100 THEN 1 ELSE 0 END) AS PopularPosts
// FROM Posts p
// GROUP BY p.OwnerUserId
// ),
// UserActivity AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COALESCE(ps.TotalPosts, 0) AS TotalPosts,
// COALESCE(ps.AnsweredQuestions, 0) AS AnsweredQuestions,
// COALESCE(ps.Questions, 0) AS QuestionsPosted,
// COALESCE(bs.BadgeCount, 0) AS BadgeCount,
// COALESCE(bs.GoldBadges, 0) AS GoldBadges,
// COALESCE(bs.SilverBadges, 0) AS SilverBadges,
// COALESCE(bs.BronzeBadges, 0) AS BronzeBadges
// FROM Users u
// LEFT JOIN PostAggregates ps ON u.Id = ps.OwnerUserId
// LEFT JOIN UserBadgeStats bs ON u.Id = bs.UserId
// WHERE u.Reputation > (SELECT AVG(Reputation) FROM Users)
// AND u.LastAccessDate = (SELECT MAX(LastAccessDate) FROM Users)
// )
// SELECT
// ua.UserId,
// ua.DisplayName,
// ua.TotalPosts,
// ua.AnsweredQuestions,
// ua.QuestionsPosted,
// ua.BadgeCount,
// ua.GoldBadges,
// ua.SilverBadges,
// ua.BronzeBadges,
// CASE WHEN ua.AnsweredQuestions > 0 THEN 'Active Contributor' ELSE 'Lurker' END AS UserStatus,
// CASE
// WHEN ua.BadgeCount = 0 THEN 'No Badges'
// WHEN ua.BadgeCount BETWEEN 1 AND 3 THEN 'Novice'
// WHEN ua.BadgeCount BETWEEN 4 AND 10 THEN 'Intermediate'
// ELSE 'Expert'
// END AS BadgeLevel
// FROM UserActivity ua
// ORDER BY ua.TotalPosts DESC
// LIMIT 10
// OFFSET 0;
fn q24824(db: &'static So) -> String {
    let t = db.user.select(&db.user.reputation).fold_flat([0i64; 2], |a, r| [a[0] + 1, a[1] + r]);
    let ml = db.user.select(&db.user.last_access_date).fold_flat(i64::MIN, |a, d| a.max(d));
    let Post { post_type_id, view_count, .. } = &db.post;
    let ps = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(view_count.opt())).fold([0i64; 3], |a, (t, w)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (w.unwrap_or(0) > 100) as i64]);
    let dpar = owned(db).group_by(&db.post.owner_user).select(&db.post.parent_id).count_distinct();
    let bc = badge_classes(db);
    let mut v = Vec::new();
    db.user
        .with((&db.user.reputation).filt(move |r: i64| (r as i128) * (t[0] as i128) > t[1] as i128))
        .with((&db.user.last_access_date).eq(ml))
        .select(Ident::<User>::new().and((&ps).opt()).and((&dpar).opt()).and((&bc).opt()))
        .drive(|_, (((u, p), d), b)| v.push((u, p.unwrap_or([0; 3]), d.unwrap_or(0), bz(b))));
    out(v, |&(_, p, ..)| Reverse(p[0]), 10, |&(u, p, d, b)| {
        let st = if d > 0 { "Active Contributor" } else { "Lurker" };
        let lvl = if b[0] == 0 { "No Badges" } else if (1..=3).contains(&b[0]) { "Novice" } else if (4..=10).contains(&b[0]) { "Intermediate" } else { "Expert" };
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&[p[0], d, p[1]]));
        f.extend(ints(&b));
        f.extend([V::S(st), V::S(lvl)]);
        f
    })
}

// WITH UserActivity AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(COALESCE(p.ViewCount, 0)) AS TotalViews,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// u.Id, u.DisplayName
// ),
// RecentEdits AS (
// SELECT
// ph.UserId,
// COUNT(ph.Id) AS EditCount,
// MAX(ph.CreationDate) AS LastEditDate
// FROM
// PostHistory ph
// WHERE
// ph.PostHistoryTypeId IN (4, 5, 6)
// GROUP BY
// ph.UserId
// ),
// UserBadges AS (
// SELECT
// b.UserId,
// COUNT(b.Id) AS BadgeCount
// FROM
// Badges b
// GROUP BY
// b.UserId
// ),
// FinalSummary AS (
// SELECT
// ua.UserId,
// ua.DisplayName,
// ua.TotalPosts,
// ua.Questions,
// ua.Answers,
// ua.TotalViews,
// ua.TotalUpvotes,
// ua.TotalDownvotes,
// COALESCE(re.EditCount, 0) AS EditCount,
// COALESCE(re.LastEditDate, '1970-01-01 00:00:00') AS LastEditDate,
// COALESCE(ub.BadgeCount, 0) AS BadgeCount
// FROM
// UserActivity ua
// LEFT JOIN
// RecentEdits re ON ua.UserId = re.UserId
// LEFT JOIN
// UserBadges ub ON ua.UserId = ub.UserId
// )
// SELECT
// UserId,
// DisplayName,
// TotalPosts,
// Questions,
// Answers,
// TotalViews,
// TotalUpvotes,
// TotalDownvotes,
// EditCount,
// LastEditDate,
// BadgeCount,
// CASE
// WHEN TotalPosts > 100 THEN 'Active Contributor'
// WHEN TotalPosts BETWEEN 50 AND 100 THEN 'Moderate Contributor'
// ELSE 'New Contributor'
// END AS ContributionLevel
// FROM
// FinalSummary
// WHERE
// TotalPosts > 0
// ORDER BY
// TotalViews DESC, TotalPosts DESC
// FETCH FIRST 10 ROWS ONLY;
fn q3071(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let re = db.post_history.with((&db.post_history.post_history_type_id).filt(|t: i64| matches!(t, 4 | 5 | 6))).group_by(&db.post_history.user).select(&db.post_history.creation_date).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    (&us).and(&dp).and((&re).opt()).and(&bu).drive(|u, (((a, d), e), b)| v.push((u, a, d, e, b)));
    out(v, |&(_, a, d, ..)| (Reverse(a.views_sum), Reverse(d)), 10, |&(u, a, d, e, b)| {
        let lvl = if d > 100 { "Active Contributor" } else if (50..=100).contains(&d) { "Moderate Contributor" } else { "New Contributor" };
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&[d, a.q, a.a, a.views_sum, a.up, a.down, e.map_or(0, |e| e.0)]));
        f.extend([V::T(e.map_or(0, |e| e.1)), V::I(b), V::S(lvl)]);
        f
    })
}

// WITH UserReputation AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// u.CreationDate,
// COUNT(b.Id) AS BadgeCount,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// u.Id, u.DisplayName, u.Reputation, u.CreationDate
// ),
// RecentPosts AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate AS PostCreationDate,
// p.ViewCount,
// COUNT(c.Id) AS CommentCount,
// AVG(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS AverageUpVotes,
// AVG(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS AverageDownVotes,
// p.OwnerUserId
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.OwnerUserId
// ),
// UserActivity AS (
// SELECT
// u.UserId,
// u.DisplayName,
// COUNT(DISTINCT rp.PostId) AS RecentPostCount,
// SUM(rp.ViewCount) AS TotalViewCount,
// SUM(rp.CommentCount) AS TotalCommentCount,
// SUM(rp.AverageUpVotes * rp.ViewCount) AS WeightedUpVotes,
// SUM(rp.AverageDownVotes * rp.ViewCount) AS WeightedDownVotes
// FROM
// UserReputation u
// JOIN
// RecentPosts rp ON u.UserId = rp.OwnerUserId
// GROUP BY
// u.UserId, u.DisplayName
// )
// SELECT
// ua.UserId,
// ua.DisplayName,
// ua.RecentPostCount,
// ua.TotalViewCount,
// ua.TotalCommentCount,
// ur.Reputation,
// ur.BadgeCount,
// ur.GoldBadges,
// ur.SilverBadges,
// ur.BronzeBadges,
// ua.WeightedUpVotes,
// ua.WeightedDownVotes
// FROM
// UserActivity ua
// JOIN
// UserReputation ur ON ua.UserId = ur.UserId
// ORDER BY
// ua.TotalViewCount DESC,
// ur.Reputation DESC
// LIMIT 50;
fn q9878(db: &'static So) -> String {
    let rp = owned_since(db, month_ago())
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 4], |a, (c, t)| [a[0] + 1, a[1] + c.is_some() as i64, a[2] + (t == Some(2)) as i64, a[3] + (t == Some(3)) as i64]);
    let ua = owned_since(db, month_ago()).group_by(&db.post.owner_user).select((&db.post.view_count).opt().and(&rp)).fold(([0i64; 4], [0f64; 2], [0i64; 2]), |(a, f, n), (w, x)| {
        let up = x[2] as f64 / x[0] as f64;
        let dn = x[3] as f64 / x[0] as f64;
        let (fu, fd) = match w {
            Some(w) => (f[0] + up * w as f64, f[1] + dn * w as f64),
            None => (f[0], f[1]),
        };
        ([a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + x[1]], [fu, fd], [n[0] + w.is_some() as i64, 0])
    });
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&ua).and((&bc).opt()).drive(|u, ((a, f, n), b)| v.push((u, a, f, n, bz(b))));
    out(v, |&(u, a, ..)| ((a[1] == 0, Reverse(a[2])), rep_desc(db, u)), 50, |&(u, a, f, n, b)| {
        let mut r = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(a[0]), nullable(a[2], a[1]), V::I(a[3]), user_col(db, u, "rep")];
        r.extend(ints(&b));
        r.extend(if n[0] == 0 { nulls(2) } else { vec![V::F(f[0]), V::F(f[1])] });
        r
    })
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(CASE WHEN p.PostTypeId = 3 THEN 1 ELSE 0 END) AS Wikis,
// SUM(p.ViewCount) AS TotalViews,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
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
// ),
// BadgeCounts AS (
// SELECT
// UserId,
// COUNT(*) AS TotalBadges,
// SUM(CASE WHEN Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM
// Badges
// GROUP BY
// UserId
// ),
// PostHistoryAnalytics AS (
// SELECT
// ph.UserId,
// COUNT(*) AS EditsCount,
// SUM(CASE WHEN ph.PostHistoryTypeId IN (4, 5) THEN 1 ELSE 0 END) AS TitleAndBodyEdits,
// SUM(CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN 1 ELSE 0 END) AS CloseReopenCounts
// FROM
// PostHistory ph
// GROUP BY
// ph.UserId
// )
// SELECT
// us.UserId,
// us.DisplayName,
// us.TotalPosts,
// us.Questions,
// us.Answers,
// us.Wikis,
// us.TotalViews,
// us.Upvotes,
// us.Downvotes,
// COALESCE(bc.TotalBadges, 0) AS TotalBadges,
// COALESCE(bc.GoldBadges, 0) AS GoldBadges,
// COALESCE(bc.SilverBadges, 0) AS SilverBadges,
// COALESCE(bc.BronzeBadges, 0) AS BronzeBadges,
// COALESCE(ph.EditsCount, 0) AS EditsCount,
// COALESCE(ph.TitleAndBodyEdits, 0) AS TitleAndBodyEdits,
// COALESCE(ph.CloseReopenCounts, 0) AS CloseReopenCounts
// FROM
// UserStats us
// LEFT JOIN
// BadgeCounts bc ON us.UserId = bc.UserId
// LEFT JOIN
// PostHistoryAnalytics ph ON us.UserId = ph.UserId
// ORDER BY
// us.TotalPosts DESC, us.Upvotes DESC;
fn q7876(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::RepGt(1000), "v", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let bc = badge_classes(db);
    let ph = db.post_history.group_by(&db.post_history.user).select(&db.post_history.post_history_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + matches!(t, 4 | 5) as i64, a[2] + matches!(t, 10 | 11) as i64]);
    let mut v = Vec::new();
    (&us).and((&dp).opt()).and((&bc).opt()).and((&ph).opt()).drive(|u, (((a, d), b), h)| v.push((u, a, d.unwrap_or(0), bz(b), h.unwrap_or([0; 3]))));
    rows(v.iter().map(|&(u, a, d, b, h)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&[d, a.q, a.a, a.t3]));
        f.push(ustat_field(&a, "views_sum"));
        f.extend(ints(&[a.up, a.down]));
        f.extend(ints(&b));
        f.extend(ints(&h));
        row(f)
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("3973", q3973),
    ("6464", q6464),
    ("14662", q14662),
    ("9380", q9380),
    ("14666", q14666),
    ("6468", q6468),
    ("4716", q4716),
    ("7510", q7510),
    ("12561", q12561),
    ("8340", q8340),
    ("7565", q7565),
    ("20706", q20706),
    ("10890", q10890),
    ("14945", q14945),
    ("7413", q7413),
    ("5861", q5861),
    ("8735", q8735),
    ("8084", q8084),
    ("6963", q6963),
    ("32601", q32601),
    ("26783", q26783),
    ("7944", q7944),
    ("6931", q6931),
    ("2880", q2880),
    ("28584", q28584),
    ("6934", q6934),
    ("9245", q9245),
    ("5869", q5869),
    ("6949", q6949),
    ("4768", q4768),
    ("6212", q6212),
    ("8237", q8237),
    ("3381", q3381),
    ("228", q228),
    ("32554", q32554),
    ("11276", q11276),
    ("293", q293),
    ("5684", q5684),
    ("7669", q7669),
    ("7479", q7479),
    ("4072", q4072),
    ("9627", q9627),
    ("6080", q6080),
    ("6881", q6881),
    ("6030", q6030),
    ("25967", q25967),
    ("20618", q20618),
    ("1877", q1877),
    ("11591", q11591),
    ("4532", q4532),
    ("3679", q3679),
    ("29215", q29215),
    ("29353", q29353),
    ("3417", q3417),
    ("6422", q6422),
    ("25760", q25760),
    ("7694", q7694),
    ("9133", q9133),
    ("24824", q24824),
    ("3071", q3071),
    ("9878", q9878),
    ("7876", q7876),
];
