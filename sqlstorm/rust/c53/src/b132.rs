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

// --- batch 132 --------------------------------------------------------------

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
// PostAnalysis AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// AVG(P.Score) AS AverageScore,
// SUM(P.ViewCount) AS TotalViews
// FROM
// Posts P
// GROUP BY
// P.OwnerUserId
// ),
// UserPostMetrics AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COALESCE(PA.TotalPosts, 0) AS TotalPosts,
// COALESCE(PA.TotalQuestions, 0) AS TotalQuestions,
// COALESCE(PA.TotalAnswers, 0) AS TotalAnswers,
// COALESCE(PA.AverageScore, 0) AS AverageScore,
// COALESCE(PA.TotalViews, 0) AS TotalViews,
// COALESCE(UB.BadgeCount, 0) AS BadgeCount,
// COALESCE(UB.GoldBadges, 0) AS GoldBadges,
// COALESCE(UB.SilverBadges, 0) AS SilverBadges,
// COALESCE(UB.BronzeBadges, 0) AS BronzeBadges
// FROM
// Users U
// LEFT JOIN
// PostAnalysis PA ON U.Id = PA.OwnerUserId
// LEFT JOIN
// UserBadges UB ON U.Id = UB.UserId
// )
// SELECT
// UserId,
// DisplayName,
// TotalPosts,
// TotalQuestions,
// TotalAnswers,
// AverageScore,
// TotalViews,
// BadgeCount,
// GoldBadges,
// SilverBadges,
// BronzeBadges
// FROM
// UserPostMetrics
// ORDER BY
// TotalPosts DESC,
// BadgeCount DESC
// LIMIT 100;
fn q26257(db: &'static So) -> String {
    let ub = ubc(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    (&ub).and((&ps).opt()).drive(|u, (b, p)| v.push((u, b, p)));
    out(v, |&(_, b, p)| (Reverse(p.map_or(0, |p| p[0])), Reverse(b[0])), 100, |&(u, b, p)| {
        let q = p.unwrap_or(Z);
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&q[..3]));
        f.extend([p.map_or(V::F(0.0), pscore_avg), V::I(q[5])]);
        f.extend(ints(&b));
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
// FROM
// Users U
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// GROUP BY
// U.Id
// ),
// PostStatistics AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS TotalPosts,
// COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS Questions,
// COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS Answers,
// SUM(P.ViewCount) AS TotalViews,
// SUM(P.Score) AS TotalScore
// FROM
// Posts P
// GROUP BY
// P.OwnerUserId
// ),
// UserPostBadgeStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COALESCE(UBC.BadgeCount, 0) AS BadgeCount,
// COALESCE(PS.TotalPosts, 0) AS TotalPosts,
// COALESCE(PS.Questions, 0) AS Questions,
// COALESCE(PS.Answers, 0) AS Answers,
// COALESCE(PS.TotalViews, 0) AS TotalViews,
// COALESCE(PS.TotalScore, 0) AS TotalScore
// FROM
// Users U
// LEFT JOIN
// UserBadgeCounts UBC ON U.Id = UBC.UserId
// LEFT JOIN
// PostStatistics PS ON U.Id = PS.OwnerUserId
// )
// SELECT
// U.UserId,
// U.DisplayName,
// U.Reputation,
// U.BadgeCount,
// U.TotalPosts,
// U.Questions,
// U.Answers,
// U.TotalViews,
// U.TotalScore,
// CASE
// WHEN U.Reputation > 10000 THEN 'Expert'
// WHEN U.Reputation BETWEEN 1000 AND 10000 THEN 'Experienced'
// ELSE 'Novice'
// END AS UserLevel
// FROM
// UserPostBadgeStats U
// ORDER BY
// U.Reputation DESC,
// U.BadgeCount DESC,
// U.TotalPosts DESC
// OFFSET 0 ROWS FETCH NEXT 50 ROWS ONLY;
fn q25996(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and(&bu).and((&ps).opt())).drive(|_, ((u, b), p)| v.push((u, b, p.unwrap_or(Z))));
    out(v, |&(u, b, p)| (rep_desc(db, u), Reverse(b), Reverse(p[0])), 50, |&(u, b, p)| {
        let r = db.user.reputation.get(u).unwrap();
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(r), V::I(b)];
        f.extend(ints(&[p[0], p[1], p[2], p[5], p[3]]));
        f.push(V::S(if r > 10000 {
            "Expert"
        } else if (1000..=10000).contains(&r) {
            "Experienced"
        } else {
            "Novice"
        }));
        f
    })
}

// WITH UserPosts AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS PostsCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersCount,
// AVG(p.Score) AS AverageScore
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// ),
// UserBadges AS (
// SELECT
// b.UserId,
// COUNT(b.Id) AS BadgesCount,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM
// Badges b
// GROUP BY
// b.UserId
// ),
// UserVoteStats AS (
// SELECT
// v.UserId,
// COUNT(v.Id) AS VotesCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesCount
// FROM
// Votes v
// GROUP BY
// v.UserId
// )
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COALESCE(up.PostsCount, 0) AS TotalPosts,
// COALESCE(up.QuestionsCount, 0) AS TotalQuestions,
// COALESCE(up.AnswersCount, 0) AS TotalAnswers,
// COALESCE(up.AverageScore, 0) AS AveragePostScore,
// COALESCE(ub.BadgesCount, 0) AS TotalBadges,
// COALESCE(ub.GoldBadges, 0) AS GoldBadges,
// COALESCE(ub.SilverBadges, 0) AS SilverBadges,
// COALESCE(ub.BronzeBadges, 0) AS BronzeBadges,
// COALESCE(uvs.VotesCount, 0) AS TotalVotes,
// COALESCE(uvs.UpVotesCount, 0) AS TotalUpVotes,
// COALESCE(uvs.DownVotesCount, 0) AS TotalDownVotes
// FROM
// Users u
// LEFT JOIN
// UserPosts up ON u.Id = up.OwnerUserId
// LEFT JOIN
// UserBadges ub ON u.Id = ub.UserId
// LEFT JOIN
// UserVoteStats uvs ON u.Id = uvs.UserId
// WHERE
// u.Reputation > 1000
// ORDER BY
// u.Reputation DESC
// LIMIT 50;
fn q7535(db: &'static So) -> String {
    let bc = badge_classes(db);
    let ps = pstat(db, db.post.iq());
    let uv = uvotes(db);
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and((&ps).opt()).and((&bc).opt()).and((&uv).opt())).drive(|_, x| v.push(x));
    out(v, |&(((u, _), _), _)| rep_desc(db, u), 50, |&(((u, p), b), x)| {
        let q = p.unwrap_or(Z);
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&q[..3]));
        f.push(p.map_or(V::F(0.0), pscore_avg));
        f.extend(ints(&b.unwrap_or([0; 4])));
        f.extend(ints(&x.unwrap_or([0; 3])));
        f
    })
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(P.Id) AS PostCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// AVG(COALESCE(P.Score, 0)) AS AverageScore,
// SUM(COALESCE(P.ViewCount, 0)) AS TotalViews,
// SUM(COALESCE(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END, 0)) AS UpVotes,
// SUM(COALESCE(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END, 0)) AS DownVotes
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// U.Id, U.DisplayName
// ), BadgeStats AS (
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
// U.DisplayName,
// COALESCE(US.PostCount, 0) AS PostCount,
// COALESCE(US.QuestionCount, 0) AS QuestionCount,
// COALESCE(US.AnswerCount, 0) AS AnswerCount,
// COALESCE(US.AverageScore, 0) AS AverageScore,
// COALESCE(US.TotalViews, 0) AS TotalViews,
// COALESCE(US.UpVotes, 0) AS UpVotes,
// COALESCE(US.DownVotes, 0) AS DownVotes,
// COALESCE(BS.BadgeCount, 0) AS BadgeCount,
// COALESCE(BS.GoldBadgeCount, 0) AS GoldBadgeCount,
// COALESCE(BS.SilverBadgeCount, 0) AS SilverBadgeCount,
// COALESCE(BS.BronzeBadgeCount, 0) AS BronzeBadgeCount
// FROM
// Users U
// LEFT JOIN
// UserStats US ON U.Id = US.UserId
// LEFT JOIN
// BadgeStats BS ON U.Id = BS.UserId
// WHERE
// U.Reputation > 1000
// ORDER BY
// U.Reputation DESC, US.PostCount DESC
// LIMIT 10;
fn q8589(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::RepGt(1000), "v", any_post);
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&us).and((&bc).opt()).drive(|u, (a, b)| v.push((u, a, b.unwrap_or([0; 4]))));
    out(v, |&(u, a, _)| (rep_desc(db, u), Reverse(a.n)), 10, |&(u, a, b)| {
        let mut f = vec![user_col(db, u, "name"), V::I(a.n), V::I(a.q), V::I(a.a)];
        f.extend(["score_avg_rows", "views_sum0"].iter().map(|c| ustat_field(&a, c)));
        f.extend([V::I(a.up), V::I(a.down)]);
        f.extend(ints(&b));
        f
    })
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(DISTINCT P.Id) AS PostCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN P.PostTypeId IN (1, 2) THEN P.Score ELSE 0 END) AS TotalScore,
// SUM(V.BountyAmount) AS TotalBounties
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// U.Id, U.DisplayName, U.Reputation
// ),
// PostStats AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.ViewCount,
// P.Score,
// P.AnswerCount,
// P.CommentCount,
// P.FavoriteCount,
// P.CreationDate,
// PT.Name AS PostType,
// P.OwnerUserId
// FROM
// Posts P
// JOIN
// PostTypes PT ON P.PostTypeId = PT.Id
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
// )
// SELECT
// U.UserId,
// U.DisplayName,
// U.Reputation,
// U.PostCount,
// U.QuestionCount,
// U.AnswerCount,
// U.TotalScore,
// U.TotalBounties,
// P.PostId,
// P.Title,
// P.ViewCount,
// P.Score AS PostScore,
// P.AnswerCount AS PostAnswerCount,
// P.CommentCount AS PostCommentCount,
// P.FavoriteCount AS PostFavoriteCount,
// P.CreationDate AS PostCreationDate,
// P.PostType,
// V.UpVotes,
// V.DownVotes,
// V.TotalVotes
// FROM
// UserStats U
// JOIN
// PostStats P ON U.UserId = P.OwnerUserId
// LEFT JOIN
// VoteStats V ON P.PostId = V.PostId
// ORDER BY
// U.Reputation DESC, P.ViewCount DESC;
fn q14102(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let uf = g(db).select(posts_of(db).select(post_type_id.and(score).and(votes_of(db).select((&db.vote.bounty_amount).opt()).opt())).opt()).fold([0i64; 5], |a, p| match p {
        Some(((t, s), b)) => {
            let b = b.flatten();
            [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + if matches!(t, 1 | 2) { s } else { 0 }, a[3] + b.is_some() as i64, a[4] + b.unwrap_or(0)]
        }
        None => a,
    });
    let dp = ud(db, UserWhere::All, posts_of(db));
    let pv = post_votes(db);
    let mut v = Vec::new();
    owned(db).select(Ident::<Post>::new().and((&pv).opt()).and((&db.post.owner_user).select(Ident::<User>::new().and(&uf).and((&dp).opt())))).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((p, x), ((u, a), d))| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep"), V::I(d.unwrap_or(0)), V::I(a[0]), V::I(a[1]), V::I(a[2]), nullable(a[4], a[3])];
        f.extend(post_fields(db, p, &["id", "title", "views", "score", "answers", "comments", "favorites", "created", "type"]));
        f.extend([oint(x.map(|x| x[1])), oint(x.map(|x| x[2])), oint(x.map(|x| x[0]))]);
        row(f)
    }))
}

// WITH UserVoteSummary AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes,
// COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes,
// SUM(CASE WHEN v.VoteTypeId IN (2, 3) THEN 1 ELSE 0 END) AS TotalVotes,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) - SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS NetVotes
// FROM Users u
// LEFT JOIN Votes v ON u.Id = v.UserId
// GROUP BY u.Id, u.DisplayName
// ),
// PostStatistics AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.OwnerUserId,
// COUNT(c.Id) AS CommentCount,
// COUNT(DISTINCT ph.UserId) AS EditCount,
// MAX(ph.CreationDate) AS LastEdited,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
// FROM Posts p
// LEFT JOIN Comments c ON p.Id = c.PostId
// LEFT JOIN PostHistory ph ON p.Id = ph.PostId
// LEFT JOIN Votes v ON p.Id = v.PostId
// WHERE p.CreationDate >= '2023-01-01'
// GROUP BY p.Id, p.Title, p.OwnerUserId
// ),
// UserPostSummary AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// ps.PostId,
// ps.Title,
// ps.CommentCount,
// ps.EditCount,
// ps.LastEdited,
// ps.TotalUpVotes,
// ps.TotalDownVotes,
// uvs.NetVotes AS UserNetVotes
// FROM Users u
// JOIN PostStatistics ps ON u.Id = ps.OwnerUserId
// JOIN UserVoteSummary uvs ON u.Id = uvs.UserId
// )
// SELECT
// ups.UserId,
// ups.DisplayName,
// ups.PostId,
// ups.Title,
// ups.CommentCount,
// ups.EditCount,
// ups.LastEdited,
// ups.TotalUpVotes,
// ups.TotalDownVotes,
// ups.UserNetVotes
// FROM UserPostSummary ups
// WHERE ups.UserNetVotes > 0
// ORDER BY ups.UserNetVotes DESC, ups.TotalUpVotes DESC;
fn q8651(db: &'static So) -> String {
    let uv = user_votes(db);
    let ed = per_post_distinct(db, history_of(db).select(&db.post_history.user_id));
    let mut v = Vec::new();
    stats_fold(db, owned_since(db, date(2023, 1, 1)), Ident::<Post>::new(), "cvh", &[])
        .and((&ed).opt())
        .and((&db.post.owner_user).select(Ident::<User>::new().and((&uv).filt(|a: [i64; 3]| a[1] - a[2] > 0))))
        .drive(|p, ((s, e), (u, a))| v.push((p, s, e.unwrap_or(0), u, a)));
    rows(v.iter().map(|&(p, s, e, u, a)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["id", "title"]));
        f.extend([V::I(s.cx), V::I(e), stat_field(&s, "hmax").unwrap(), V::I(s.up), V::I(s.down), V::I(a[1] - a[2])]);
        row(f)
    }))
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN p.PostTypeId IN (10, 11, 12) THEN 1 ELSE 0 END) AS ClosedPosts,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN Votes v ON p.Id = v.PostId
// GROUP BY u.Id, u.DisplayName, u.Reputation
// ), BadgeStats AS (
// SELECT
// b.UserId,
// COUNT(b.Id) AS BadgeCount,
// COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldBadges,
// COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverBadges,
// COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeBadges
// FROM Badges b
// GROUP BY b.UserId
// ), PostHistoryStats AS (
// SELECT
// ph.UserId,
// COUNT(ph.Id) AS EditCount,
// SUM(CASE WHEN ph.PostHistoryTypeId IN (4, 5, 6) THEN 1 ELSE 0 END) AS TitleEdits,
// SUM(CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN 1 ELSE 0 END) AS ClosedPostChanges
// FROM PostHistory ph
// GROUP BY ph.UserId
// )
// SELECT
// us.UserId,
// us.DisplayName,
// us.Reputation,
// us.PostCount,
// us.QuestionCount,
// us.AnswerCount,
// us.ClosedPosts,
// us.UpVotes,
// us.DownVotes,
// bs.BadgeCount,
// bs.GoldBadges,
// bs.SilverBadges,
// bs.BronzeBadges,
// phs.EditCount,
// phs.TitleEdits,
// phs.ClosedPostChanges
// FROM UserStats us
// LEFT JOIN BadgeStats bs ON us.UserId = bs.UserId
// LEFT JOIN PostHistoryStats phs ON us.UserId = phs.UserId
// WHERE us.Reputation > 1000
// ORDER BY us.Reputation DESC, us.PostCount DESC
// LIMIT 50;
fn q7612(db: &'static So) -> String {
    let Post { post_type_id, .. } = &db.post;
    let uf = user_base(db, UserWhere::RepGt(1000)).group_by(Ident::<User>::new()).select(posts_of(db).select(post_type_id.and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt()).fold([0i64; 5], |a, p| match p {
        Some((t, x)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + matches!(t, 10 | 11 | 12) as i64, a[3] + (x == Some(2)) as i64, a[4] + (x == Some(3)) as i64],
        None => a,
    });
    let dp = ud(db, UserWhere::RepGt(1000), posts_of(db));
    let bc = badge_classes(db);
    let ph = phu(db);
    let mut v = Vec::new();
    (&uf).and((&dp).opt()).and((&bc).opt()).and((&ph).opt()).drive(|u, (((a, d), b), h)| v.push((u, a, d.unwrap_or(0), b, h)));
    out(v, |&(u, _, d, _, _)| (rep_desc(db, u), Reverse(d)), 50, |&(u, a, d, b, h)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep"), V::I(d)];
        f.extend(ints(&a));
        f.extend((0..4).map(|i| oint(b.map(|b| b[i]))));
        f.extend([0, 1, 2].iter().map(|&i| oint(h.map(|h| h[i]))));
        f
    })
}

// WITH UserPosts AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS QuestionCount,
// COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS AnswerCount,
// SUM(p.Score) AS TotalScore,
// SUM(p.ViewCount) AS TotalViews,
// SUM(COALESCE(p.CommentCount, 0)) AS TotalComments
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName
// ),
// PostStatistics AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// pp.DisplayName AS OwnerDisplayName,
// pt.Name AS PostType,
// p.Score,
// p.ViewCount,
// p.CommentCount,
// p.AnswerCount,
// p.FavoriteCount,
// p.ClosedDate,
// p.LastEditDate,
// COALESCE(ph.RevisionCount, 0) AS RevisionCount,
// p.OwnerUserId
// FROM
// Posts p
// LEFT JOIN
// Users pp ON p.OwnerUserId = pp.Id
// LEFT JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN (
// SELECT
// PostId,
// COUNT(*) AS RevisionCount
// FROM
// PostHistory
// GROUP BY
// PostId
// ) ph ON p.Id = ph.PostId
// )
// SELECT
// u.UserId,
// u.DisplayName,
// u.TotalPosts,
// u.QuestionCount,
// u.AnswerCount,
// u.TotalScore,
// AVG(p.Score) AS AvgPostScore,
// AVG(p.ViewCount) AS AvgViewCount,
// AVG(p.AnswerCount) AS AvgAnswerCount,
// AVG(p.CommentCount) AS AvgCommentCount,
// AVG(p.FavoriteCount) AS AvgFavoriteCount,
// SUM(p.RevisionCount) AS TotalRevisions
// FROM
// UserPosts u
// LEFT JOIN
// PostStatistics p ON u.UserId = p.OwnerUserId
// GROUP BY
// u.UserId, u.DisplayName, u.TotalPosts, u.QuestionCount, u.AnswerCount, u.TotalScore
// ORDER BY
// u.TotalPosts DESC
// LIMIT 10;
fn q11679(db: &'static So) -> String {
    let ps = pstat(db, db.post.iq());
    let hp = history_per_post(db);
    let Post { score, view_count, answer_count, comment_count, favorite_count, .. } = &db.post;
    let st = owned(db)
        .group_by(&db.post.owner_user)
        .select(score.and(view_count.opt()).and(answer_count.opt()).and(comment_count).and(favorite_count.opt()).and(&hp))
        .fold([0i64; 10], |a, (((((s, w), an), c), fv), h)| {
            [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + an.is_some() as i64, a[5] + an.unwrap_or(0), a[6] + c, a[7] + fv.is_some() as i64, a[8] + fv.unwrap_or(0), a[9] + h]
        });
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&ps).opt()).and((&st).opt())).drive(|_, x| v.push(x));
    out(v, |&((_, p), _)| Reverse(p.map_or(0, |p| p[0])), 10, |&((u, p), s)| {
        let q = p.unwrap_or(Z);
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(q[0]), V::I(q[1]), V::I(q[2]), oint(p.map(|p| p[3]))];
        match s {
            Some(a) => f.extend([avg(a[1], a[0]), avg(a[3], a[2]), avg(a[5], a[4]), avg(a[6], a[0]), avg(a[8], a[7]), V::I(a[9])]),
            None => f.extend(nulls(6)),
        }
        f
    })
}

// WITH UserBadges AS (
// SELECT
// U.Id AS UserId,
// COUNT(CASE WHEN B.Class = 1 THEN 1 END) AS GoldBadges,
// COUNT(CASE WHEN B.Class = 2 THEN 1 END) AS SilverBadges,
// COUNT(CASE WHEN B.Class = 3 THEN 1 END) AS BronzeBadges,
// SUM(COALESCE(U.UpVotes, 0) - COALESCE(U.DownVotes, 0)) AS NetVotes
// FROM Users U
// LEFT JOIN Badges B ON U.Id = B.UserId
// GROUP BY U.Id
// ),
// PostStatistics AS (
// SELECT
// P.OwnerUserId,
// COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS QuestionCount,
// COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS AnswerCount,
// SUM(P.Score) AS TotalScore,
// SUM(P.ViewCount) AS TotalViews,
// MAX(P.CreationDate) AS LastPostDate
// FROM Posts P
// GROUP BY P.OwnerUserId
// ),
// CombinedStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COALESCE(UB.GoldBadges, 0) AS GoldBadges,
// COALESCE(UB.SilverBadges, 0) AS SilverBadges,
// COALESCE(UB.BronzeBadges, 0) AS BronzeBadges,
// COALESCE(PS.QuestionCount, 0) AS QuestionCount,
// COALESCE(PS.AnswerCount, 0) AS AnswerCount,
// COALESCE(PS.TotalScore, 0) AS TotalScore,
// COALESCE(PS.TotalViews, 0) AS TotalViews,
// COALESCE(UB.NetVotes, 0) AS NetVotes,
// CASE
// WHEN COALESCE(PS.LastPostDate, '1900-01-01') > '2022-01-01' THEN 'Active'
// ELSE 'Inactive'
// END AS ActivityStatus
// FROM Users U
// LEFT JOIN UserBadges UB ON U.Id = UB.UserId
// LEFT JOIN PostStatistics PS ON U.Id = PS.OwnerUserId
// )
// SELECT
// UserId,
// DisplayName,
// GoldBadges,
// SilverBadges,
// BronzeBadges,
// QuestionCount,
// AnswerCount,
// TotalScore,
// TotalViews,
// NetVotes,
// ActivityStatus
// FROM CombinedStats
// WHERE (QuestionCount + AnswerCount) > 0
// ORDER BY TotalScore DESC, GoldBadges DESC, SilverBadges DESC
// LIMIT 10;
fn q2940(db: &'static So) -> String {
    let User { up_votes, down_votes, .. } = &db.user;
    let ub = g(db).select(up_votes.and(down_votes).and(badges_of(db).select(&db.badge.class).opt())).fold([0i64; 4], |a, ((u, d), c)| {
        [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64, a[3] + (u - d)]
    });
    let ps = pstat(db, db.post.iq());
    let t0 = date(2022, 1, 1);
    let mut v = Vec::new();
    (&ub).and((&ps).filt(|p: [i64; 13]| p[1] + p[2] > 0)).drive(|u, (b, p)| v.push((u, b, p)));
    out(v, |&(_, b, p)| (Reverse(p[3]), Reverse(b[0]), Reverse(b[1])), 10, |&(u, b, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&b[..3]));
        f.extend(ints(&[p[1], p[2], p[3], p[5], b[3]]));
        f.push(V::S(if p[10] > t0 { "Active" } else { "Inactive" }));
        f
    })
}

// WITH UserPostStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN P.PostTypeId IN (4, 5) THEN 1 ELSE 0 END) AS TotalTagWikis,
// SUM(P.Score) AS TotalScore,
// AVG(P.ViewCount) AS AvgViewCount
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id, U.DisplayName
// ),
// PostHistoryStats AS (
// SELECT
// PH.UserId,
// COUNT(PH.Id) AS TotalEdits,
// SUM(CASE WHEN PHT.Name = 'Edit Title' THEN 1 ELSE 0 END) AS TotalTitleEdits,
// SUM(CASE WHEN PHT.Name = 'Edit Body' THEN 1 ELSE 0 END) AS TotalBodyEdits,
// SUM(CASE WHEN PHT.Name = 'Edit Tags' THEN 1 ELSE 0 END) AS TotalTagEdits
// FROM
// PostHistory PH
// JOIN
// PostHistoryTypes PHT ON PH.PostHistoryTypeId = PHT.Id
// GROUP BY
// PH.UserId
// ),
// CombinedStats AS (
// SELECT
// UPS.UserId,
// UPS.DisplayName,
// UPS.TotalPosts,
// UPS.TotalQuestions,
// UPS.TotalAnswers,
// UPS.TotalTagWikis,
// UPS.TotalScore,
// UPS.AvgViewCount,
// PHS.TotalEdits,
// PHS.TotalTitleEdits,
// PHS.TotalBodyEdits,
// PHS.TotalTagEdits
// FROM
// UserPostStats UPS
// LEFT JOIN
// PostHistoryStats PHS ON UPS.UserId = PHS.UserId
// )
// SELECT
// UserId,
// DisplayName,
// TotalPosts,
// TotalQuestions,
// TotalAnswers,
// TotalTagWikis,
// TotalScore,
// AvgViewCount,
// COALESCE(TotalEdits, 0) AS TotalEdits,
// COALESCE(TotalTitleEdits, 0) AS TotalTitleEdits,
// COALESCE(TotalBodyEdits, 0) AS TotalBodyEdits,
// COALESCE(TotalTagEdits, 0) AS TotalTagEdits
// FROM
// CombinedStats
// ORDER BY
// TotalScore DESC;
fn q14788(db: &'static So) -> String {
    let ps = pstat(db, db.post.iq());
    let ph = db
        .post_history
        .group_by(&db.post_history.user)
        .select((&db.post_history.post_history_type).select(&db.post_history_type.name))
        .fold([0i64; 4], |a, n| [a[0] + 1, a[1] + (n == "Edit Title") as i64, a[2] + (n == "Edit Body") as i64, a[3] + (n == "Edit Tags") as i64]);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&ps).opt()).and((&ph).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((u, p), h)| {
        let q = p.unwrap_or(Z);
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&[q[0], q[1], q[2], q[9]]));
        f.extend([oint(p.map(|p| p[3])), onull(p, pviews_avg)]);
        f.extend(ints(&h.unwrap_or([0; 4])));
        row(f)
    }))
}

// WITH UserVotes AS (
// SELECT
// v.UserId,
// COUNT(v.Id) AS TotalVotes,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Votes v
// GROUP BY
// v.UserId
// ),
// UserBadges AS (
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
// ),
// PostStatistics AS (
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
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COALESCE(uv.TotalVotes, 0) AS TotalVotes,
// COALESCE(uv.UpVotes, 0) AS UpVotes,
// COALESCE(uv.DownVotes, 0) AS DownVotes,
// COALESCE(ub.TotalBadges, 0) AS TotalBadges,
// COALESCE(ub.GoldBadges, 0) AS GoldBadges,
// COALESCE(ub.SilverBadges, 0) AS SilverBadges,
// COALESCE(ub.BronzeBadges, 0) AS BronzeBadges,
// COALESCE(ps.TotalPosts, 0) AS TotalPosts,
// COALESCE(ps.Questions, 0) AS Questions,
// COALESCE(ps.Answers, 0) AS Answers,
// COALESCE(ps.TotalViews, 0) AS TotalViews,
// COALESCE(ps.TotalScore, 0) AS TotalScore
// FROM
// Users u
// LEFT JOIN
// UserVotes uv ON u.Id = uv.UserId
// LEFT JOIN
// UserBadges ub ON u.Id = ub.UserId
// LEFT JOIN
// PostStatistics ps ON u.Id = ps.OwnerUserId
// WHERE
// u.Reputation > 1000
// ORDER BY
// u.Reputation DESC
// LIMIT 50;
fn q9528(db: &'static So) -> String {
    let uv = uvotes(db);
    let bc = badge_classes(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and((&uv).opt()).and((&bc).opt()).and((&ps).opt())).drive(|_, x| v.push(x));
    out(v, |&(((u, _), _), _)| rep_desc(db, u), 50, |&(((u, x), b), p)| {
        let q = p.unwrap_or(Z);
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(ints(&x.unwrap_or([0; 3])));
        f.extend(ints(&b.unwrap_or([0; 4])));
        f.extend(ints(&[q[0], q[1], q[2], q[5], q[3]]));
        f
    })
}

// WITH UserBadgeCount AS (
// SELECT
// U.Id AS UserId,
// COUNT(B.Id) AS BadgeCount,
// SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldCount,
// SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverCount,
// SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeCount
// FROM
// Users U
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// GROUP BY
// U.Id
// ),
// PostAnalytics AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS TotalPosts,
// COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS Questions,
// COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS Answers,
// SUM(P.ViewCount) AS TotalViews,
// AVG(P.Score) AS AverageScore,
// MAX(P.CreationDate) AS LastPostDate
// FROM
// Posts P
// GROUP BY
// P.OwnerUserId
// ),
// UserPerformance AS (
// SELECT
// U.Id,
// U.DisplayName,
// COALESCE(UB.BadgeCount, 0) AS BadgeCount,
// COALESCE(PA.TotalPosts, 0) AS TotalPosts,
// COALESCE(PA.Questions, 0) AS Questions,
// COALESCE(PA.Answers, 0) AS Answers,
// COALESCE(PA.TotalViews, 0) AS TotalViews,
// COALESCE(PA.AverageScore, 0) AS AverageScore,
// COALESCE(PA.LastPostDate, '1970-01-01') AS LastPostDate
// FROM
// Users U
// LEFT JOIN
// UserBadgeCount UB ON U.Id = UB.UserId
// LEFT JOIN
// PostAnalytics PA ON U.Id = PA.OwnerUserId
// )
// SELECT
// UP.DisplayName,
// UP.BadgeCount,
// UP.TotalPosts,
// UP.Questions,
// UP.Answers,
// UP.TotalViews,
// UP.AverageScore,
// UP.LastPostDate,
// CASE
// WHEN UP.TotalPosts > 10 THEN 'Active'
// WHEN UP.TotalPosts BETWEEN 1 AND 10 THEN 'Moderate'
// ELSE 'Inactive'
// END AS ActivityLevel
// FROM
// UserPerformance UP
// WHERE
// UP.BadgeCount > 0 OR UP.TotalPosts > 0
// ORDER BY
// UP.BadgeCount DESC,
// UP.TotalPosts DESC
// LIMIT 100;
fn q318(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    db.user
        .select(Ident::<User>::new().and(&bu).and((&ps).opt()))
        .filt(|((_, b), p): ((Id<User>, i64), Option<[i64; 13]>)| b > 0 || p.map_or(0, |p| p[0]) > 0)
        .drive(|_, ((u, b), p)| v.push((u, b, p)));
    out(v, |&(_, b, p)| (Reverse(b), Reverse(p.map_or(0, |p| p[0]))), 100, |&(u, b, p)| {
        let q = p.unwrap_or(Z);
        vec![
            user_col(db, u, "name"),
            V::I(b),
            V::I(q[0]),
            V::I(q[1]),
            V::I(q[2]),
            V::I(q[5]),
            p.map_or(V::F(0.0), pscore_avg),
            V::T(p.map_or(0, |p| p[10])),
            V::S(if q[0] > 10 {
                "Active"
            } else if (1..=10).contains(&q[0]) {
                "Moderate"
            } else {
                "Inactive"
            }),
        ]
    })
}

// WITH UserStatistics AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// MAX(u.LastAccessDate) AS LastActiveDate
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN Votes v ON p.Id = v.PostId
// GROUP BY u.Id, u.DisplayName, u.Reputation
// ),
// BadgeSummary AS (
// SELECT
// u.Id AS UserId,
// COUNT(DISTINCT b.Id) AS BadgeCount,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM Users u
// LEFT JOIN Badges b ON u.Id = b.UserId
// GROUP BY u.Id
// ),
// PostHistoryAggregated AS (
// SELECT
// ph.UserId,
// COUNT(*) AS EditCount,
// COUNT(DISTINCT ph.PostId) AS EditedPostCount,
// MAX(ph.CreationDate) AS LastEditDate
// FROM PostHistory ph
// WHERE ph.PostHistoryTypeId IN (4, 5, 6, 10, 11, 12)
// GROUP BY ph.UserId
// )
// SELECT
// us.UserId,
// us.DisplayName,
// us.Reputation,
// us.PostCount,
// us.QuestionCount,
// us.AnswerCount,
// us.UpVotes,
// us.DownVotes,
// bs.BadgeCount,
// bs.GoldBadges,
// bs.SilverBadges,
// bs.BronzeBadges,
// pha.EditCount,
// pha.EditedPostCount,
// pha.LastEditDate,
// us.LastActiveDate
// FROM UserStatistics us
// JOIN BadgeSummary bs ON us.UserId = bs.UserId
// LEFT JOIN PostHistoryAggregated pha ON us.UserId = pha.UserId
// WHERE us.Reputation > 1000
// ORDER BY us.Reputation DESC, us.PostCount DESC
// FETCH FIRST 100 ROWS ONLY;
fn q5277(db: &'static So) -> String {
    let w = UserWhere::RepGt(1000);
    let us = user_stats_fold(db, Ident::<User>::new(), w, "v", any_post);
    let dp = ud(db, w, posts_of(db));
    let ub = ubc(db);
    let PostHistory { post_history_type_id, creation_date, user, post, .. } = &db.post_history;
    let hb = || db.post_history.with(post_history_type_id.in_v(vec![4, 5, 6, 10, 11, 12]));
    let ph = hb().group_by(user).select(creation_date).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let pd = hb().group_by(user).select(post).count_distinct();
    let mut v = Vec::new();
    (&us).and((&dp).opt()).and(&ub).and((&ph).opt()).and((&pd).opt()).drive(|u, ((((a, d), b), h), e)| v.push((u, a, d.unwrap_or(0), b, h, e)));
    out(v, |&(u, _, d, _, _, _)| (rep_desc(db, u), Reverse(d)), 100, |&(u, a, d, b, h, e)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep"), V::I(d), V::I(a.q), V::I(a.a), V::I(a.up), V::I(a.down)];
        f.extend(ints(&b));
        f.extend([oint(h.map(|h| h.0)), if h.is_some() { V::I(e.unwrap_or(0)) } else { V::Null }, ots(h.map(|h| h.1)), user_col(db, u, "last_access")]);
        f
    })
}

// WITH UserVoteStats AS (
// SELECT
// u.Id AS UserId,
// COUNT(v.Id) AS TotalVotes,
// SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS Upvotes,
// SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS Downvotes,
// SUM(CASE WHEN vt.Name = 'Favorite' THEN 1 ELSE 0 END) AS Favorites
// FROM
// Users u
// LEFT JOIN
// Votes v ON u.Id = v.UserId
// LEFT JOIN
// VoteTypes vt ON v.VoteTypeId = vt.Id
// GROUP BY
// u.Id
// ),
// PostStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(CASE WHEN p.PostTypeId = 3 THEN 1 ELSE 0 END) AS Wikis,
// SUM(CASE WHEN p.PostTypeId = 4 THEN 1 ELSE 0 END) AS TagWikis
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// ),
// UserMetrics AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// us.TotalVotes,
// us.Upvotes,
// us.Downvotes,
// us.Favorites,
// ps.TotalPosts,
// ps.Questions,
// ps.Answers,
// ps.Wikis,
// ps.TagWikis
// FROM
// Users u
// LEFT JOIN
// UserVoteStats us ON u.Id = us.UserId
// LEFT JOIN
// PostStats ps ON u.Id = ps.OwnerUserId
// )
// SELECT
// um.UserId,
// um.DisplayName,
// um.Reputation,
// COALESCE(um.TotalVotes, 0) AS TotalVotes,
// COALESCE(um.Upvotes, 0) AS Upvotes,
// COALESCE(um.Downvotes, 0) AS Downvotes,
// COALESCE(um.Favorites, 0) AS Favorites,
// COALESCE(um.TotalPosts, 0) AS TotalPosts,
// COALESCE(um.Questions, 0) AS Questions,
// COALESCE(um.Answers, 0) AS Answers,
// COALESCE(um.Wikis, 0) AS Wikis,
// COALESCE(um.TagWikis, 0) AS TagWikis
// FROM
// UserMetrics um
// ORDER BY
// um.Reputation DESC,
// um.TotalPosts DESC
// LIMIT 100;
fn q14613(db: &'static So) -> String {
    let uv = g(db).select(votes_by(db).select((&db.vote.vote_type).select(&db.vote_type.name)).opt()).fold([0i64; 4], |a, n| match n {
        Some(n) => [a[0] + 1, a[1] + (n == "UpMod") as i64, a[2] + (n == "DownMod") as i64, a[3] + (n == "Favorite") as i64],
        None => a,
    });
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    (&uv).and((&ps).opt()).drive(|u, (x, p)| v.push((u, x, p)));
    out(v, |&(u, _, p)| (rep_desc(db, u), (p.is_none(), Reverse(p.map(|p| p[0])))), 100, |&(u, x, p)| {
        let q = p.unwrap_or(Z);
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(ints(&x));
        f.extend(ints(&[q[0], q[1], q[2], q[8], q[12]]));
        f
    })
}

// WITH RecentPosts AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// u.DisplayName AS OwnerName,
// p.Score,
// p.ViewCount,
// COUNT(c.Id) AS CommentCount
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// WHERE
// p.CreationDate >= DATE('2024-10-01') - INTERVAL '30 days'
// GROUP BY
// p.Id, p.Title, p.CreationDate, u.DisplayName, p.Score, p.ViewCount
// ),
// TopUsers AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// SUM(p.Score) AS TotalScore
// FROM
// Users u
// INNER JOIN
// Posts p ON u.Id = p.OwnerUserId
// WHERE
// p.CreationDate >= DATE('2024-10-01') - INTERVAL '1 year'
// GROUP BY
// u.Id, u.DisplayName
// ORDER BY
// TotalScore DESC
// LIMIT 10
// ),
// PostScores AS (
// SELECT
// rp.PostId,
// rp.Title,
// rp.CreationDate,
// rp.OwnerName,
// rp.Score,
// rp.ViewCount,
// rp.CommentCount,
// COALESCE(v.UpVoteCount, 0) AS UpVoteCount,
// COALESCE(v.DownVoteCount, 0) AS DownVoteCount,
// (rp.Score + COALESCE(v.UpVoteCount, 0) - COALESCE(v.DownVoteCount, 0)) AS AdjustedScore
// FROM
// RecentPosts rp
// LEFT JOIN (
// SELECT
// PostId,
// COUNT(*) FILTER (WHERE VoteTypeId = 2) AS UpVoteCount,
// COUNT(*) FILTER (WHERE VoteTypeId = 3) AS DownVoteCount
// FROM
// Votes
// GROUP BY
// PostId
// ) v ON rp.PostId = v.PostId
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.OwnerName,
// ps.CreationDate,
// ps.Score,
// ps.ViewCount,
// ps.CommentCount,
// ps.AdjustedScore,
// tu.DisplayName AS TopUser
// FROM
// PostScores ps
// LEFT JOIN
// TopUsers tu ON ps.OwnerName = tu.DisplayName
// ORDER BY
// ps.AdjustedScore DESC
// LIMIT 5;
fn q1859(db: &'static So) -> String {
    let base = owned_since(db, date(2023, 10, 1));
    let tf = base.group_by(&db.post.owner_user).select(&db.post.score).fold(0i64, |a, s| a + s);
    let top: MatSet<Id<User>> = whole(&tf).select(Same::new().and(&tf)).window(row_number, |(_, s): (Id<User>, i64)| s, desc).filt(|(_, n)| n <= 10).map(|((u, _), _)| u).collect();
    let tn: HashIdx<Str, Id<User>> = (&top).select(&db.user.display_name).inv().collect();
    let pv = post_votes(db);
    let mut v = Vec::new();
    since(db, date(2024, 9, 1))
        .select(Ident::<Post>::new().and(comments_per_post(db)).and((&pv).opt()).and((&db.post.owner_user).select(&db.user.display_name).select(&tn).opt()))
        .drive(|_, (((p, c), x), t)| {
            let x = x.unwrap_or([0; 3]);
            v.push((p, c, db.post.score.get(p).unwrap() + x[1] - x[2], t));
        });
    out(v, |&(_, _, a, _)| Reverse(a), 5, |&(p, c, a, t)| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "created", "score", "views"]);
        f.extend([V::I(c), V::I(a), t.map_or(V::Null, |t| user_col(db, t, "name"))]);
        f
    })
}

// WITH UserStatistics AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(DISTINCT P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN P.ViewCount > 100 THEN 1 ELSE 0 END) AS PopularPosts,
// AVG(P.Score) AS AverageScore,
// MIN(P.CreationDate) AS AccountStartDate,
// MAX(P.LastActivityDate) AS LastActiveDate
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id, U.DisplayName
// ),
// BadgeStatistics AS (
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
// PostHistoryCounts AS (
// SELECT
// PH.UserId,
// COUNT(PH.Id) AS TotalEdits,
// SUM(CASE WHEN PH.PostHistoryTypeId IN (4, 5, 6) THEN 1 ELSE 0 END) AS TitleBodyTagEdits,
// SUM(CASE WHEN PH.PostHistoryTypeId IN (10, 11) THEN 1 ELSE 0 END) AS CloseReopenActions
// FROM
// PostHistory PH
// GROUP BY
// PH.UserId
// )
// SELECT
// U.UserId,
// U.DisplayName,
// U.TotalPosts,
// U.TotalQuestions,
// U.TotalAnswers,
// U.PopularPosts,
// U.AverageScore,
// B.TotalBadges,
// B.GoldBadges,
// B.SilverBadges,
// B.BronzeBadges,
// PH.TotalEdits,
// PH.TitleBodyTagEdits,
// PH.CloseReopenActions,
// U.AccountStartDate,
// U.LastActiveDate
// FROM
// UserStatistics U
// LEFT JOIN
// BadgeStatistics B ON U.UserId = B.UserId
// LEFT JOIN
// PostHistoryCounts PH ON U.UserId = PH.UserId
// WHERE
// U.TotalPosts > 10
// ORDER BY
// U.PopularPosts DESC, U.AverageScore DESC;
fn q6845(db: &'static So) -> String {
    let Post { post_type_id, view_count, score, creation_date, last_activity_date, .. } = &db.post;
    let uf = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(view_count.opt()).and(score).and(creation_date).and(last_activity_date)).fold(
        [0, 0, 0, 0, 0, i64::MAX, i64::MIN],
        |a: [i64; 7], ((((t, w), s), c), la)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.map_or(false, |w| w > 100) as i64, a[4] + s, a[5].min(c), a[6].max(la)],
    );
    let bc = badge_classes(db);
    let ph = phu(db);
    let mut v = Vec::new();
    (&uf).filt(|a: [i64; 7]| a[0] > 10).and((&bc).opt()).and((&ph).opt()).drive(|u, ((a, b), h)| v.push((u, a, b, h)));
    rows(v.iter().map(|&(u, a, b, h)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[..4]));
        f.push(avg(a[4], a[0]));
        f.extend((0..4).map(|i| oint(b.map(|b| b[i]))));
        f.extend([0, 1, 2].iter().map(|&i| oint(h.map(|h| h[i]))));
        f.extend([V::T(a[5]), V::T(a[6])]);
        row(f)
    }))
}

// WITH PopularQuestions AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// U.DisplayName AS Owner,
// P.ViewCount,
// COUNT(A.Id) AS AnswerCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Posts P
// LEFT JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// Posts A ON P.Id = A.ParentId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// WHERE
// P.PostTypeId = 1
// GROUP BY
// P.Id, P.Title, P.CreationDate, U.DisplayName, P.ViewCount
// HAVING
// COUNT(A.Id) > 0 AND P.ViewCount > 1000
// ),
// VoteSummary AS (
// SELECT
// PostId,
// COUNT(CASE WHEN VoteTypeId = 2 THEN 1 ELSE NULL END) AS UpVotes,
// COUNT(CASE WHEN VoteTypeId = 3 THEN 1 ELSE NULL END) AS DownVotes,
// COUNT(CASE WHEN VoteTypeId IN (10, 11) THEN 1 ELSE NULL END) AS CloseReopenCount
// FROM
// Votes
// GROUP BY
// PostId
// ),
// TopPosts AS (
// SELECT
// PQ.PostId,
// PQ.Title,
// PQ.Owner,
// PQ.ViewCount,
// PQ.AnswerCount,
// PQ.UpVotes,
// PQ.DownVotes,
// COALESCE(VS.UpVotes, 0) AS CumulativeUpVotes,
// COALESCE(VS.DownVotes, 0) AS CumulativeDownVotes,
// COALESCE(VS.CloseReopenCount, 0) AS CloseReopenCount
// FROM
// PopularQuestions PQ
// LEFT JOIN
// VoteSummary VS ON PQ.PostId = VS.PostId
// )
// SELECT
// TP.PostId,
// TP.Title,
// TP.Owner,
// TP.ViewCount,
// TP.AnswerCount,
// (TP.UpVotes + TP.CumulativeUpVotes) AS TotalUpVotes,
// (TP.DownVotes + TP.CumulativeDownVotes) AS TotalDownVotes,
// TP.CloseReopenCount,
// ((TP.UpVotes + TP.CumulativeUpVotes) - (TP.DownVotes + TP.CumulativeDownVotes)) AS VoteBalance
// FROM
// TopPosts TP
// ORDER BY
// VoteBalance DESC, TP.ViewCount DESC
// LIMIT 10;
fn q9540(db: &'static So) -> String {
    let vs = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 3], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + matches!(t, 10 | 11) as i64]);
    let mut v = Vec::new();
    stats_fold(db, questions_only(db).with((&db.post.view_count).gt(1000)), Ident::<Post>::new(), "av", &[])
        .filt(|s: Stats| s.ax > 0)
        .and((&vs).opt())
        .drive(|p, (s, x)| {
            let x = x.unwrap_or([0; 3]);
            v.push((p, s, x, (s.up + x[0]) - (s.down + x[1])));
        });
    out(v, |&(p, _, _, b)| (Reverse(b), views_desc(db, p)), 10, |&(p, s, x, b)| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "views"]);
        f.extend(ints(&[s.ax, s.up + x[0], s.down + x[1], x[2], b]));
        f
    })
}

// WITH UserBadgeCounts AS (
// SELECT
// U.Id AS UserId,
// COUNT(B.Id) AS BadgeCount
// FROM
// Users U
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// GROUP BY
// U.Id
// ),
// PostVoteStats AS (
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
// ClosedPostDetails AS (
// SELECT
// P.Id AS PostId,
// COUNT(PH.Id) AS CloseCount,
// MAX(PH.CreationDate) AS LastClosedDate
// FROM
// Posts P
// JOIN
// PostHistory PH ON P.Id = PH.PostId
// WHERE
// PH.PostHistoryTypeId = 10
// GROUP BY
// P.Id
// ),
// UserPostStats AS (
// SELECT
// U.Id AS UserId,
// COUNT(P.Id) AS TotalPosts,
// COALESCE(CLOSED.CloseCount, 0) AS ClosedPosts,
// COALESCE(PS.VoteCount, 0) AS TotalVotes,
// COALESCE(PS.UpVotes, 0) AS TotalUpVotes,
// COALESCE(PS.DownVotes, 0) AS TotalDownVotes,
// COALESCE(UBC.BadgeCount, 0) AS BadgeCount
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// PostVoteStats PS ON P.Id = PS.PostId
// LEFT JOIN
// ClosedPostDetails CLOSED ON P.Id = CLOSED.PostId
// LEFT JOIN
// UserBadgeCounts UBC ON U.Id = UBC.UserId
// GROUP BY
// U.Id, CLOSED.CloseCount, PS.VoteCount, PS.UpVotes, PS.DownVotes, UBC.BadgeCount
// )
// SELECT
// U.DisplayName,
// U.Reputation,
// U.CreationDate,
// USP.TotalPosts,
// USP.ClosedPosts,
// USP.TotalVotes,
// USP.TotalUpVotes,
// USP.TotalDownVotes,
// USP.BadgeCount
// FROM
// UserPostStats USP
// JOIN
// Users U ON USP.UserId = U.Id
// WHERE
// USP.TotalPosts > 10
// ORDER BY
// U.Reputation DESC, USP.TotalPosts DESC;
fn q7429(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let pv = db.post.group_by(Ident::<Post>::new()).select(votes_of(db).select(&db.vote.vote_type_id).opt()).fold([0i64; 3], |a, t| {
        [a[0] + t.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]
    });
    let cl = db.post_history.with((&db.post_history.post_history_type_id).eq(10)).group_by(&db.post_history.post).fold(0i64, |a, _| a + 1);
    let key = (&db.post.owner_user).and((&cl).opt()).and(&pv);
    let f = owned(db).group_by(key).fold(0i64, |a, _| a + 1);
    let mut v = Vec::new();
    (&f).filt(|n: i64| n > 10).drive(|k, n| v.push((k, n)));
    rows(v.iter().map(|&(((u, c), x), n)| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep"), user_col(db, u, "ucreated"), V::I(n), V::I(c.unwrap_or(0))];
        f.extend(ints(&x));
        f.push(V::I(bu.get(u).unwrap_or(0)));
        row(f)
    }))
}

// WITH UserActivity AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN p.Score ELSE 0 END) AS TotalQuestionScore,
// SUM(CASE WHEN p.PostTypeId = 2 THEN p.Score ELSE 0 END) AS TotalAnswerScore
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
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
// UserVotes AS (
// SELECT
// v.UserId,
// COUNT(v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
// FROM
// Votes v
// GROUP BY
// v.UserId
// )
// SELECT
// ua.UserId,
// ua.DisplayName,
// ua.Reputation,
// ua.PostCount,
// ua.QuestionCount,
// ua.AnswerCount,
// ua.TotalQuestionScore,
// ua.TotalAnswerScore,
// COALESCE(ub.BadgeCount, 0) AS BadgeCount,
// COALESCE(ub.GoldBadges, 0) AS GoldBadges,
// COALESCE(ub.SilverBadges, 0) AS SilverBadges,
// COALESCE(ub.BronzeBadges, 0) AS BronzeBadges,
// COALESCE(uv.VoteCount, 0) AS VoteCount,
// COALESCE(uv.Upvotes, 0) AS Upvotes,
// COALESCE(uv.Downvotes, 0) AS Downvotes
// FROM
// UserActivity ua
// LEFT JOIN
// UserBadges ub ON ua.UserId = ub.UserId
// LEFT JOIN
// UserVotes uv ON ua.UserId = uv.UserId
// ORDER BY
// ua.Reputation DESC, ua.PostCount DESC;
fn q9951(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let uf = g(db).select(posts_of(db).select(post_type_id.and(score)).opt()).fold([0i64; 5], |a, p| match p {
        Some((t, s)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + if t == 1 { s } else { 0 }, a[4] + if t == 2 { s } else { 0 }],
        None => a,
    });
    let bc = badge_classes(db);
    let uv = uvotes(db);
    let mut v = Vec::new();
    (&uf).and((&bc).opt()).and((&uv).opt()).drive(|u, ((a, b), x)| v.push((u, a, b, x)));
    rows(v.iter().map(|&(u, a, b, x)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(ints(&a));
        f.extend(ints(&b.unwrap_or([0; 4])));
        f.extend(ints(&x.unwrap_or([0; 3])));
        row(f)
    }))
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN p.PostTypeId = 3 THEN 1 ELSE 0 END) AS WikiCount,
// SUM(CASE WHEN p.ViewCount > 100 THEN 1 ELSE 0 END) AS HighViewPostCount
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
// COUNT(b.Id) AS BadgeCount,
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
// ups.UserId,
// ups.DisplayName,
// ups.PostCount,
// ups.QuestionCount,
// ups.AnswerCount,
// ups.WikiCount,
// ups.HighViewPostCount,
// COALESCE(ub.BadgeCount, 0) AS TotalBadges,
// COALESCE(ub.GoldBadges, 0) AS GoldBadges,
// COALESCE(ub.SilverBadges, 0) AS SilverBadges,
// COALESCE(ub.BronzeBadges, 0) AS BronzeBadges
// FROM
// UserPostStats ups
// LEFT JOIN
// UserBadges ub ON ups.UserId = ub.UserId
// )
// SELECT
// DisplayName,
// PostCount,
// QuestionCount,
// AnswerCount,
// WikiCount,
// HighViewPostCount,
// TotalBadges,
// GoldBadges,
// SilverBadges,
// BronzeBadges,
// (QuestionCount + AnswerCount + WikiCount) AS TotalPosts,
// (CASE WHEN PostCount = 0 THEN 0 ELSE (HighViewPostCount * 100.0 / PostCount) END) AS HighViewPostPercentage
// FROM
// CombinedStats
// WHERE
// (QuestionCount + AnswerCount + WikiCount) > 5
// ORDER BY
// HighViewPostPercentage DESC, TotalPosts DESC;
fn q26670(db: &'static So) -> String {
    let Post { post_type_id, view_count, .. } = &db.post;
    let uf = g(db).select(posts_of(db).select(post_type_id.and(view_count.opt())).opt()).fold([0i64; 5], |a, p| match p {
        Some((t, w)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (t == 3) as i64, a[4] + w.map_or(false, |w| w > 100) as i64],
        None => a,
    });
    let bc = badge_classes(db);
    let pct = |a: [i64; 5]| if a[0] == 0 { 0.0 } else { a[4] as f64 * 100.0 / a[0] as f64 };
    let mut v = Vec::new();
    (&uf).filt(|a: [i64; 5]| a[1] + a[2] + a[3] > 5).and((&bc).opt()).drive(|u, (a, b)| v.push((u, a, b.unwrap_or([0; 4]))));
    rows(v.iter().map(|&(u, a, b)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(ints(&a));
        f.extend(ints(&b));
        f.extend([V::I(a[1] + a[2] + a[3]), V::F(pct(a))]);
        row(f)
    }))
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
// ),
// PostDetails AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// u.DisplayName AS OwnerDisplayName,
// pt.Name AS PostType,
// COALESCE((
// SELECT
// COUNT(*)
// FROM
// Comments c
// WHERE
// c.PostId = p.Id
// ), 0) AS CommentCount,
// p.OwnerUserId
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// )
// SELECT
// us.DisplayName,
// us.Reputation,
// us.PostCount,
// us.QuestionCount,
// us.AnswerCount,
// us.Upvotes,
// us.Downvotes,
// us.GoldBadges,
// us.SilverBadges,
// us.BronzeBadges,
// pd.PostId,
// pd.Title,
// pd.CreationDate,
// pd.ViewCount,
// pd.Score,
// pd.OwnerDisplayName,
// pd.PostType,
// pd.CommentCount
// FROM
// UserStats us
// JOIN
// PostDetails pd ON us.UserId = pd.OwnerUserId
// ORDER BY
// us.Reputation DESC, pd.CreationDate DESC
// LIMIT 100;
fn q8377(db: &'static So) -> String {
    let uf = g(db)
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 8], |a, (p, c)| {
            let (t, x) = match p {
                Some((t, x)) => (Some(t), x),
                None => (None, None),
            };
            [
                a[0] + t.is_some() as i64,
                a[1] + (t == Some(1)) as i64,
                a[2] + (t == Some(2)) as i64,
                a[3] + (x == Some(2)) as i64,
                a[4] + (x == Some(3)) as i64,
                a[5] + (c == Some(1)) as i64,
                a[6] + (c == Some(2)) as i64,
                a[7] + (c == Some(3)) as i64,
            ]
        });
    let mut v = Vec::new();
    owned(db).select(Ident::<Post>::new().and(comments_per_post(db)).and((&db.post.owner_user).select(Ident::<User>::new().and(&uf)))).drive(|_, x| v.push(x));
    out(v, |&((p, _), (u, _))| (rep_desc(db, u), newest(db, p)), 100, |&((p, c), (u, a))| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(ints(&a));
        f.extend(post_fields(db, p, &["id", "title", "created", "views", "score", "owner", "type"]));
        f.push(V::I(c));
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
// GROUP BY
// U.Id, U.DisplayName
// ),
// PostStats AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS TotalPosts,
// COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS Questions,
// COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS Answers,
// AVG(P.Score) AS AvgScore
// FROM
// Posts P
// GROUP BY
// P.OwnerUserId
// ),
// CommentStats AS (
// SELECT
// C.UserId,
// COUNT(C.Id) AS TotalComments
// FROM
// Comments C
// GROUP BY
// C.UserId
// ),
// FinalStats AS (
// SELECT
// B.UserId,
// B.DisplayName,
// COALESCE(P.TotalPosts, 0) AS TotalPosts,
// COALESCE(P.Questions, 0) AS TotalQuestions,
// COALESCE(P.Answers, 0) AS TotalAnswers,
// COALESCE(C.TotalComments, 0) AS TotalComments,
// B.BadgeCount,
// B.GoldBadges,
// B.SilverBadges,
// B.BronzeBadges,
// P.AvgScore
// FROM
// UserBadges B
// LEFT JOIN
// PostStats P ON B.UserId = P.OwnerUserId
// LEFT JOIN
// CommentStats C ON B.UserId = C.UserId
// )
// SELECT
// F.DisplayName,
// F.TotalPosts,
// F.TotalQuestions,
// F.TotalAnswers,
// F.TotalComments,
// F.BadgeCount,
// F.GoldBadges,
// F.SilverBadges,
// F.BronzeBadges,
// ROUND(F.AvgScore, 2) AS AvgScore,
// CASE
// WHEN F.BadgeCount > 5 THEN 'Experienced User'
// WHEN F.TotalPosts > 100 THEN 'Active Contributor'
// ELSE 'Newbie'
// END AS UserLevel
// FROM
// FinalStats F
// ORDER BY
// F.TotalPosts DESC, F.BadgeCount DESC
// LIMIT 10;
fn q2551(db: &'static So) -> String {
    let ub = ubc(db);
    let ps = pstat(db, db.post.iq());
    let cu = comments_per_user(db);
    let mut v = Vec::new();
    (&ub).and((&ps).opt()).and(&cu).drive(|u, ((b, p), c)| v.push((u, b, p, c)));
    out(v, |&(_, b, p, _)| (Reverse(p.map_or(0, |p| p[0])), Reverse(b[0])), 10, |&(u, b, p, c)| {
        let q = p.unwrap_or(Z);
        let mut f = vec![user_col(db, u, "name"), V::I(q[0]), V::I(q[1]), V::I(q[2]), V::I(c)];
        f.extend(ints(&b));
        f.push(p.map_or(V::Null, |p| V::F(round2(p[3] as f64 / p[0] as f64))));
        f.push(V::S(if b[0] > 5 {
            "Experienced User"
        } else if q[0] > 100 {
            "Active Contributor"
        } else {
            "Newbie"
        }));
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
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN p.PostTypeId = 3 THEN 1 ELSE 0 END) AS WikiCount,
// SUM(p.Score) AS TotalScore,
// AVG(p.ViewCount) AS AvgViewCount
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// ),
// FinalStats AS (
// SELECT
// u.DisplayName,
// u.Reputation,
// u.LastAccessDate,
// u.Views,
// u.UpVotes,
// u.DownVotes,
// ubs.BadgeCount,
// ubs.GoldBadges,
// ubs.SilverBadges,
// ubs.BronzeBadges,
// ps.PostCount,
// ps.QuestionCount,
// ps.AnswerCount,
// ps.WikiCount,
// ps.TotalScore,
// ps.AvgViewCount
// FROM
// Users u
// JOIN
// UserBadgeStats ubs ON u.Id = ubs.UserId
// LEFT JOIN
// PostStats ps ON u.Id = ps.OwnerUserId
// WHERE
// u.Reputation > 100 AND
// u.LastAccessDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// )
// SELECT
// DisplayName,
// Reputation,
// LastAccessDate,
// (Views + UpVotes - DownVotes) AS EngagementScore,
// BadgeCount,
// GoldBadges,
// SilverBadges,
// BronzeBadges,
// PostCount,
// QuestionCount,
// AnswerCount,
// WikiCount,
// TotalScore,
// AvgViewCount
// FROM
// FinalStats
// ORDER BY
// EngagementScore DESC, Reputation DESC
// LIMIT 100;
fn q8954(db: &'static So) -> String {
    let ub = ubc(db);
    let ps = pstat(db, db.post.iq());
    let eng = |u: Id<User>| db.user.views.get(u).unwrap() + db.user.up_votes.get(u).unwrap() - db.user.down_votes.get(u).unwrap();
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(100)).with((&db.user.last_access_date).gt(year_ago())).select(Ident::<User>::new().and(&ub).and((&ps).opt())).drive(|_, x| v.push(x));
    out(v, |&((u, _), _)| (Reverse(eng(u)), rep_desc(db, u)), 100, |&((u, b), p)| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep"), user_col(db, u, "last_access"), V::I(eng(u))];
        f.extend(ints(&b));
        f.extend([0, 1, 2, 8, 3].iter().map(|&i| oint(p.map(|p| p[i]))));
        f.push(onull(p, pviews_avg));
        f
    })
}

// WITH UserStats AS (
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
// PostMetrics AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// AVG(p.Score) AS AverageScore,
// MAX(p.CreationDate) AS LastPostDate
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// ),
// UserActivity AS (
// SELECT
// us.UserId,
// us.DisplayName,
// us.Reputation,
// COALESCE(pm.PostCount, 0) AS PostCount,
// COALESCE(pm.Questions, 0) AS Questions,
// COALESCE(pm.Answers, 0) AS Answers,
// COALESCE(pm.AverageScore, 0) AS AverageScore,
// (TIMESTAMP '2024-10-01 12:34:56' - us.CreationDate) AS AccountAge,
// us.GoldBadges,
// us.SilverBadges,
// us.BronzeBadges
// FROM
// UserStats us
// LEFT JOIN
// PostMetrics pm ON us.UserId = pm.OwnerUserId
// )
// SELECT
// ua.DisplayName,
// ua.Reputation,
// ua.PostCount,
// ua.Questions,
// ua.Answers,
// ua.AverageScore,
// ua.AccountAge,
// ua.GoldBadges,
// ua.SilverBadges,
// ua.BronzeBadges,
// CASE
// WHEN ua.Reputation > 1000 THEN 'Pro User'
// WHEN ua.Reputation BETWEEN 500 AND 1000 THEN 'Intermediate User'
// ELSE 'New User'
// END AS UserLevel
// FROM
// UserActivity ua
// WHERE
// ua.PostCount > 0 OR ua.GoldBadges > 0
// ORDER BY
// ua.Reputation DESC
// FETCH FIRST 10 ROWS ONLY;
fn q3273(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let ub = ubc(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    (&ub).and((&ps).opt()).filt(|(b, p): ([i64; 4], Option<[i64; 13]>)| p.map_or(0, |p| p[0]) > 0 || b[1] > 0).drive(|u, (b, p)| v.push((u, b, p)));
    out(v, |&(u, _, _)| rep_desc(db, u), 10, |&(u, b, p)| {
        let r = db.user.reputation.get(u).unwrap();
        let q = p.unwrap_or(Z);
        let mut f = vec![user_col(db, u, "name"), V::I(r)];
        f.extend(ints(&q[..3]));
        f.extend([p.map_or(V::F(0.0), pscore_avg), V::Iv(t0 - db.user.creation_date.get(u).unwrap())]);
        f.extend(ints(&b[1..]));
        f.push(V::S(if r > 1000 {
            "Pro User"
        } else if (500..=1000).contains(&r) {
            "Intermediate User"
        } else {
            "New User"
        }));
        f
    })
}

// WITH PostStatistics AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.Score,
// P.ViewCount,
// P.AnswerCount,
// P.CommentCount,
// U.Reputation AS OwnerReputation,
// COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVotes,
// COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVotes
// FROM
// Posts P
// LEFT JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.Score, P.ViewCount, P.AnswerCount, P.CommentCount, U.Reputation
// ),
// UserActivity AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(P.Id) AS PostsCreated,
// SUM(COALESCE(P.ViewCount, 0)) AS TotalViews,
// SUM(COALESCE(P.UpVotes, 0)) AS TotalUpVotes,
// SUM(COALESCE(P.DownVotes, 0)) AS TotalDownVotes
// FROM
// Users U
// LEFT JOIN
// (SELECT
// P.Id,
// P.OwnerUserId,
// PS.UpVotes,
// PS.DownVotes,
// P.ViewCount
// FROM
// Posts P
// LEFT JOIN
// (SELECT
// PostId,
// COUNT(CASE WHEN VoteTypeId = 2 THEN 1 END) AS UpVotes,
// COUNT(CASE WHEN VoteTypeId = 3 THEN 1 END) AS DownVotes
// FROM
// Votes
// GROUP BY
// PostId) PS ON P.Id = PS.PostId) P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id, U.DisplayName
// )
// SELECT
// PS.PostId,
// PS.Title,
// PS.CreationDate,
// PS.Score,
// PS.ViewCount,
// PS.AnswerCount,
// PS.CommentCount,
// PS.OwnerReputation,
// PS.UpVotes,
// PS.DownVotes,
// UA.UserId,
// UA.DisplayName AS OwnerDisplayName,
// UA.PostsCreated,
// UA.TotalViews,
// UA.TotalUpVotes,
// UA.TotalDownVotes
// FROM
// PostStatistics PS
// LEFT JOIN
// UserActivity UA ON PS.OwnerReputation = UA.UserId
// ORDER BY
// PS.CreationDate DESC;
fn q13381(db: &'static So) -> String {
    let uid = uids(db);
    let pv = post_votes(db);
    let ua = g(db).select(posts_of(db).select((&db.post.view_count).opt().and((&pv).opt())).opt()).fold([0i64; 4], |a, p| match p {
        Some((w, x)) => {
            let x = x.unwrap_or([0; 3]);
            [a[0] + 1, a[1] + w.unwrap_or(0), a[2] + x[1], a[3] + x[2]]
        }
        None => a,
    });
    let mut v = Vec::new();
    db.post
        .select(Ident::<Post>::new().and((&pv).opt()).and((&db.post.owner_user).select(&db.user.reputation).select(&uid).select(Ident::<User>::new().and(&ua)).opt()))
        .drive(|_, x| v.push(x));
    rows(v.iter().map(|&((p, x), u)| {
        let x = x.unwrap_or([0; 3]);
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "rep"]);
        f.extend([V::I(x[1]), V::I(x[2])]);
        match u {
            Some((u, a)) => {
                f.extend([user_col(db, u, "uid"), user_col(db, u, "name")]);
                f.extend(ints(&a));
            }
            None => f.extend(nulls(6)),
        }
        row(f)
    }))
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(CASE WHEN p.PostTypeId = 3 THEN 1 ELSE 0 END) AS Wikis,
// SUM(CASE WHEN p.ViewCount > 100 THEN 1 ELSE 0 END) AS PopularPosts
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
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
// ),
// PostHistoryStats AS (
// SELECT
// ph.UserId,
// COUNT(ph.Id) AS TotalEdits,
// SUM(CASE WHEN ph.PostHistoryTypeId IN (4, 5, 6) THEN 1 ELSE 0 END) AS TitleAndBodyEdits,
// SUM(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS PostsClosed
// FROM
// PostHistory ph
// GROUP BY
// ph.UserId
// )
// SELECT
// us.UserId,
// us.DisplayName,
// us.Reputation,
// us.TotalPosts,
// us.Questions,
// us.Answers,
// us.Wikis,
// us.PopularPosts,
// COALESCE(bs.TotalBadges, 0) AS TotalBadges,
// COALESCE(bs.GoldBadges, 0) AS GoldBadges,
// COALESCE(bs.SilverBadges, 0) AS SilverBadges,
// COALESCE(bs.BronzeBadges, 0) AS BronzeBadges,
// COALESCE(phs.TotalEdits, 0) AS TotalEdits,
// COALESCE(phs.TitleAndBodyEdits, 0) AS TitleAndBodyEdits,
// COALESCE(phs.PostsClosed, 0) AS PostsClosed
// FROM
// UserStats us
// LEFT JOIN
// BadgeStats bs ON us.UserId = bs.UserId
// LEFT JOIN
// PostHistoryStats phs ON us.UserId = phs.UserId
// ORDER BY
// us.Reputation DESC, us.TotalPosts DESC
// LIMIT 100;
fn q7274(db: &'static So) -> String {
    let Post { post_type_id, view_count, .. } = &db.post;
    let uf = g(db).select(posts_of(db).select(post_type_id.and(view_count.opt())).opt()).fold([0i64; 5], |a, p| match p {
        Some((t, w)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (t == 3) as i64, a[4] + w.map_or(false, |w| w > 100) as i64],
        None => a,
    });
    let bc = badge_classes(db);
    let ph = phu(db);
    let mut v = Vec::new();
    (&uf).and((&bc).opt()).and((&ph).opt()).drive(|u, ((a, b), h)| v.push((u, a, b.unwrap_or([0; 4]), h.unwrap_or([0; 8]))));
    out(v, |&(u, a, _, _)| (rep_desc(db, u), Reverse(a[0])), 100, |&(u, a, b, h)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(ints(&a));
        f.extend(ints(&b));
        f.extend(ints(&[h[0], h[1], h[3]]));
        f
    })
}

// WITH UserBadgeCounts AS (
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
// COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS QuestionCount,
// COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS AnswerCount,
// COALESCE(SUM(P.ViewCount), 0) AS TotalViews,
// COALESCE(AVG(P.Score), 0) AS AvgScore
// FROM
// Posts P
// GROUP BY
// P.OwnerUserId
// ),
// UserPerformance AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COALESCE(UBC.TotalBadges, 0) AS BadgeCount,
// COALESCE(PS.QuestionCount, 0) AS QuestionCount,
// COALESCE(PS.AnswerCount, 0) AS AnswerCount,
// COALESCE(PS.TotalViews, 0) AS TotalViews,
// COALESCE(PS.AvgScore, 0) AS AvgScore
// FROM
// Users U
// LEFT JOIN
// UserBadgeCounts UBC ON U.Id = UBC.UserId
// LEFT JOIN
// PostStatistics PS ON U.Id = PS.OwnerUserId
// ),
// HighestPerformingUsers AS (
// SELECT
// UserId,
// DisplayName,
// AVG(TotalViews) + SUM(BadgeCount) AS PerformanceScore
// FROM
// UserPerformance
// GROUP BY
// UserId, DisplayName
// ORDER BY
// PerformanceScore DESC
// LIMIT 10
// )
// SELECT
// U.UserId,
// U.DisplayName,
// U.BadgeCount,
// U.QuestionCount,
// U.AnswerCount,
// U.TotalViews,
// U.AvgScore,
// HP.PerformanceScore
// FROM
// UserPerformance U
// JOIN
// HighestPerformingUsers HP ON U.UserId = HP.UserId
// WHERE
// U.QuestionCount > 10
// AND (U.AnswerCount > 5 OR U.BadgeCount > 5)
// ORDER BY
// U.AvgScore DESC, U.TotalViews DESC;
fn q919(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let ps = pstat(db, db.post.iq());
    let perf = |b: i64, p: Option<[i64; 13]>| p.map_or(0, |p| p[5]) as f64 + b as f64;
    let top: MatSet<Id<User>> = whole(&db.user.id)
        .select(Ident::<User>::new().and(&bu).and((&ps).opt()))
        .window(row_number, move |((_, b), p): ((Id<User>, i64), Option<[i64; 13]>)| fkey(perf(b, p)), desc)
        .filt(|(_, n)| n <= 10)
        .map(|(((u, _), _), _)| u)
        .collect();
    let mut v = Vec::new();
    (&top)
        .select(Ident::<User>::new().and(&bu).and((&ps).opt()))
        .filt(|((_, b), p): ((Id<User>, i64), Option<[i64; 13]>)| p.map_or(0, |p| p[1]) > 10 && (p.map_or(0, |p| p[2]) > 5 || b > 5))
        .drive(|_, x| v.push(x));
    rows(v.into_iter().map(|((u, b), p)| {
        let q = p.unwrap_or(Z);
        row(vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(b), V::I(q[1]), V::I(q[2]), V::I(q[5]), p.map_or(V::F(0.0), pscore_avg), V::F(perf(b, p))])
    }))
}

// WITH UserBadgeStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(B.Id) AS BadgeCount,
// SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldCount,
// SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverCount,
// SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeCount
// FROM
// Users AS U
// LEFT JOIN
// Badges AS B ON U.Id = B.UserId
// GROUP BY
// U.Id, U.DisplayName
// ),
// PostStats AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(P.Score) AS TotalScore
// FROM
// Posts AS P
// GROUP BY
// P.OwnerUserId
// ),
// UserPerformance AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COALESCE(UB.BadgeCount, 0) AS BadgeCount,
// COALESCE(UB.GoldCount, 0) AS GoldCount,
// COALESCE(UB.SilverCount, 0) AS SilverCount,
// COALESCE(UB.BronzeCount, 0) AS BronzeCount,
// COALESCE(PS.TotalPosts, 0) AS TotalPosts,
// COALESCE(PS.QuestionCount, 0) AS QuestionCount,
// COALESCE(PS.AnswerCount, 0) AS AnswerCount,
// COALESCE(PS.TotalScore, 0) AS TotalScore
// FROM
// Users AS U
// LEFT JOIN
// UserBadgeStats AS UB ON U.Id = UB.UserId
// LEFT JOIN
// PostStats AS PS ON U.Id = PS.OwnerUserId
// )
// SELECT
// UP.UserId,
// UP.DisplayName,
// UP.BadgeCount,
// UP.GoldCount,
// UP.SilverCount,
// UP.BronzeCount,
// UP.TotalPosts,
// UP.QuestionCount,
// UP.AnswerCount,
// UP.TotalScore,
// CASE
// WHEN UP.BadgeCount > 5 AND UP.QuestionCount > 10 THEN 'High Performer'
// WHEN UP.BadgeCount > 2 AND UP.QuestionCount > 5 THEN 'Moderate Performer'
// ELSE 'Beginner'
// END AS PerformanceType
// FROM
// UserPerformance AS UP
// ORDER BY
// UP.TotalScore DESC, UP.BadgeCount DESC
// LIMIT 10;
fn q29147(db: &'static So) -> String {
    let ub = ubc(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    (&ub).and((&ps).opt()).drive(|u, (b, p)| v.push((u, b, p.unwrap_or(Z))));
    out(v, |&(_, b, p)| (Reverse(p[3]), Reverse(b[0])), 10, |&(u, b, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&b));
        f.extend(ints(&p[..4]));
        f.push(V::S(if b[0] > 5 && p[1] > 10 {
            "High Performer"
        } else if b[0] > 2 && p[1] > 5 {
            "Moderate Performer"
        } else {
            "Beginner"
        }));
        f
    })
}

// WITH UserBadges AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(b.Id) AS TotalBadges,
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
// OwnerUserId,
// COUNT(CASE WHEN PostTypeId = 1 THEN 1 END) AS QuestionCount,
// COUNT(CASE WHEN PostTypeId = 2 THEN 1 END) AS AnswerCount,
// SUM(ViewCount) AS TotalViews,
// AVG(ViewCount) AS AverageViews
// FROM
// Posts
// GROUP BY
// OwnerUserId
// ),
// UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COALESCE(ps.QuestionCount, 0) AS Questions,
// COALESCE(ps.AnswerCount, 0) AS Answers,
// COALESCE(ps.TotalViews, 0) AS TotalViews,
// COALESCE(ps.AverageViews, 0) AS AverageViews,
// ub.TotalBadges,
// ub.GoldBadges,
// ub.SilverBadges,
// ub.BronzeBadges
// FROM
// Users u
// LEFT JOIN
// PostStatistics ps ON u.Id = ps.OwnerUserId
// LEFT JOIN
// UserBadges ub ON u.Id = ub.UserId
// )
// SELECT
// ups.UserId,
// ups.DisplayName,
// ups.Questions,
// ups.Answers,
// ups.TotalViews,
// ups.AverageViews,
// ups.TotalBadges,
// ups.GoldBadges,
// ups.SilverBadges,
// ups.BronzeBadges,
// CASE
// WHEN ups.GoldBadges > 0 THEN 'Gold Badge Holder'
// WHEN ups.SilverBadges > 0 THEN 'Silver Badge Holder'
// WHEN ups.BronzeBadges > 0 THEN 'Bronze Badge Holder'
// ELSE 'No Badges'
// END AS BadgeStatus
// FROM
// UserPostStats ups
// WHERE
// (ups.Questions > 0 OR ups.Answers > 0)
// AND ups.TotalViews > (
// SELECT AVG(TotalViews) FROM PostStatistics
// )
// ORDER BY
// ups.TotalViews DESC
// LIMIT 10;
fn q3691(db: &'static So) -> String {
    let ub = ubc(db);
    let ps = pstat(db, db.post.iq());
    let grp = db.post.group_by((&db.post.owner_user_id).opt()).select((&db.post.view_count).opt()).fold([0i64; 2], |a, w| [a[0] + w.is_some() as i64, a[1] + w.unwrap_or(0)]);
    let t = (&grp).fold_flat([0i64; 2], |a, g: [i64; 2]| if g[0] > 0 { [a[0] + 1, a[1] + g[1]] } else { a });
    let avgv = t[1] as f64 / t[0] as f64;
    let mut v = Vec::new();
    (&ub).and((&ps).filt(move |p: [i64; 13]| (p[1] > 0 || p[2] > 0) && p[5] as f64 > avgv)).drive(|u, (b, p)| v.push((u, b, Some(p))));
    out(v, |&(_, _, p)| Reverse(p.map_or(0, |p| p[5])), 10, |&(u, b, p)| {
        let q = p.unwrap_or(Z);
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(q[1]), V::I(q[2]), V::I(q[5]), if q[4] > 0 { pviews_avg(q) } else { V::F(0.0) }];
        f.extend(ints(&b));
        f.push(V::S(if b[1] > 0 {
            "Gold Badge Holder"
        } else if b[2] > 0 {
            "Silver Badge Holder"
        } else if b[3] > 0 {
            "Bronze Badge Holder"
        } else {
            "No Badges"
        }));
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
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN P.PostTypeId = 4 THEN 1 ELSE 0 END) AS TotalTagWikis,
// SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts
// FROM
// Posts P
// GROUP BY
// P.OwnerUserId
// ),
// ActivityStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COALESCE(UB.TotalBadges, 0) AS TotalBadges,
// COALESCE(PS.TotalPosts, 0) AS TotalPosts,
// COALESCE(PS.TotalQuestions, 0) AS TotalQuestions,
// COALESCE(PS.TotalAnswers, 0) AS TotalAnswers,
// COALESCE(PS.TotalTagWikis, 0) AS TotalTagWikis,
// COALESCE(PS.PositivePosts, 0) AS PositivePosts,
// CASE
// WHEN COALESCE(UB.TotalBadges, 0) > 10 THEN 'Active Contributor'
// WHEN COALESCE(PS.TotalPosts, 0) > 100 THEN 'Veteran'
// ELSE 'New Contributor'
// END AS ContributorStatus
// FROM
// Users U
// LEFT JOIN
// UserBadgeStats UB ON U.Id = UB.UserId
// LEFT JOIN
// PostStats PS ON U.Id = PS.OwnerUserId
// )
// SELECT
// UserId,
// DisplayName,
// TotalBadges,
// TotalPosts,
// TotalQuestions,
// TotalAnswers,
// TotalTagWikis,
// PositivePosts,
// ContributorStatus
// FROM
// ActivityStats
// WHERE
// TotalPosts > 0
// ORDER BY
// TotalPosts DESC,
// TotalBadges DESC
// LIMIT 50;
fn q9137(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let ps = pstat(db, db.post.iq());
    let pos = owned(db).group_by(&db.post.owner_user).select(&db.post.score).fold(0i64, |a, s| a + (s > 0) as i64);
    let mut v = Vec::new();
    (&ps).filt(|p: [i64; 13]| p[0] > 0).and(&bu).and(&pos).drive(|u, ((p, b), s)| v.push((u, p, b, s)));
    out(v, |&(_, p, b, _)| (Reverse(p[0]), Reverse(b)), 50, |&(u, p, b, s)| {
        vec![
            user_col(db, u, "uid"),
            user_col(db, u, "name"),
            V::I(b),
            V::I(p[0]),
            V::I(p[1]),
            V::I(p[2]),
            V::I(p[12]),
            V::I(s),
            V::S(if b > 10 {
                "Active Contributor"
            } else if p[0] > 100 {
                "Veteran"
            } else {
                "New Contributor"
            }),
        ]
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
// COUNT(P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(CASE WHEN P.PostTypeId = 3 THEN 1 ELSE 0 END) AS Wikis
// FROM
// Posts P
// GROUP BY
// P.OwnerUserId
// ),
// UserActivity AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COALESCE(UB.BadgeCount, 0) AS BadgeCount,
// COALESCE(PS.TotalPosts, 0) AS TotalPosts,
// COALESCE(PS.Questions, 0) AS TotalQuestions,
// COALESCE(PS.Answers, 0) AS TotalAnswers,
// COALESCE(PS.Wikis, 0) AS TotalWikis,
// SUM(Vs.Score) AS TotalVotes
// FROM
// Users U
// LEFT JOIN UserBadgeCounts UB ON U.Id = UB.UserId
// LEFT JOIN PostStats PS ON U.Id = PS.OwnerUserId
// LEFT JOIN (
// SELECT
// UserId,
// SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS Score
// FROM
// Votes
// GROUP BY
// UserId
// ) Vs ON U.Id = Vs.UserId
// GROUP BY
// U.Id, U.DisplayName, BadgeCount, TotalPosts, TotalQuestions, TotalAnswers, TotalWikis
// )
// SELECT
// UserId,
// DisplayName,
// BadgeCount,
// TotalPosts,
// TotalQuestions,
// TotalAnswers,
// TotalWikis,
// CASE
// WHEN BadgeCount > 10 THEN 'Expert'
// WHEN TotalPosts > 100 THEN 'Active'
// ELSE 'Newbie'
// END AS UserCategory
// FROM
// UserActivity
// ORDER BY
// BadgeCount DESC, TotalPosts DESC
// LIMIT 10;
fn q25579(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and(&bu).and((&ps).opt())).drive(|_, ((u, b), p)| v.push((u, b, p.unwrap_or(Z))));
    out(v, |&(_, b, p)| (Reverse(b), Reverse(p[0])), 10, |&(u, b, p)| {
        vec![
            user_col(db, u, "uid"),
            user_col(db, u, "name"),
            V::I(b),
            V::I(p[0]),
            V::I(p[1]),
            V::I(p[2]),
            V::I(p[8]),
            V::S(if b > 10 {
                "Expert"
            } else if p[0] > 100 {
                "Active"
            } else {
                "Newbie"
            }),
        ]
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
// PostStatistics AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// AVG(p.Score) AS AverageScore,
// AVG(p.ViewCount) AS AverageViews
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// ),
// UserPostAnalytics AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COALESCE(pc.TotalPosts, 0) AS TotalPosts,
// COALESCE(pc.Questions, 0) AS Questions,
// COALESCE(pc.Answers, 0) AS Answers,
// COALESCE(pc.AverageScore, 0) AS AverageScore,
// COALESCE(pc.AverageViews, 0) AS AverageViews,
// COALESCE(bc.BadgeCount, 0) AS BadgeCount,
// COALESCE(bc.GoldBadges, 0) AS GoldBadges,
// COALESCE(bc.SilverBadges, 0) AS SilverBadges,
// COALESCE(bc.BronzeBadges, 0) AS BronzeBadges
// FROM
// Users u
// LEFT JOIN
// PostStatistics pc ON u.Id = pc.OwnerUserId
// LEFT JOIN
// UserBadgeCounts bc ON u.Id = bc.UserId
// )
// SELECT
// UserId,
// DisplayName,
// TotalPosts,
// Questions,
// Answers,
// AverageScore,
// AverageViews,
// BadgeCount,
// GoldBadges,
// SilverBadges,
// BronzeBadges,
// (CASE
// WHEN AverageViews > 100 THEN 'High Engagement'
// WHEN AverageViews > 50 THEN 'Moderate Engagement'
// ELSE 'Low Engagement'
// END) AS EngagementLevel
// FROM
// UserPostAnalytics
// ORDER BY
// TotalPosts DESC, BadgeCount DESC
// LIMIT 10;
fn q26711(db: &'static So) -> String {
    let ub = ubc(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    (&ub).and((&ps).opt()).drive(|u, (b, p)| v.push((u, b, p)));
    out(v, |&(_, b, p)| (Reverse(p.map_or(0, |p| p[0])), Reverse(b[0])), 10, |&(u, b, p)| {
        let q = p.unwrap_or(Z);
        let av = if q[4] > 0 { q[5] as f64 / q[4] as f64 } else { 0.0 };
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&q[..3]));
        f.extend([p.map_or(V::F(0.0), pscore_avg), V::F(av)]);
        f.extend(ints(&b));
        f.push(V::S(if av > 100.0 {
            "High Engagement"
        } else if av > 50.0 {
            "Moderate Engagement"
        } else {
            "Low Engagement"
        }));
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
// COUNT(DISTINCT c.Id) AS CommentCount,
// COUNT(DISTINCT a.Id) AS AnswerCount,
// SUM(v.BountyAmount) AS TotalBounties,
// p.OwnerUserId
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2
// LEFT JOIN
// Votes v ON p.Id = v.PostId AND v.VoteTypeId = 8
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.OwnerUserId
// ),
// UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(DISTINCT p.Id) AS PostsCount,
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
// u.Id, u.DisplayName, u.Reputation
// ),
// VoteStats AS (
// SELECT
// v.UserId,
// COUNT(DISTINCT v.PostId) AS VotesCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Votes v
// GROUP BY
// v.UserId
// )
// SELECT
// us.UserId,
// us.DisplayName,
// us.Reputation,
// us.PostsCount,
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.ViewCount,
// ps.Score,
// ps.CommentCount,
// ps.AnswerCount,
// ps.TotalBounties,
// vs.VotesCount,
// vs.UpVotes,
// vs.DownVotes,
// us.GoldBadges,
// us.SilverBadges,
// us.BronzeBadges
// FROM
// UserStats us
// JOIN
// PostStats ps ON us.UserId = ps.OwnerUserId
// LEFT JOIN
// VoteStats vs ON us.UserId = vs.UserId
// ORDER BY
// us.Reputation DESC,
// ps.Score DESC;
fn q14624(db: &'static So) -> String {
    let pf = stats_fold(db, owned(db), Ident::<Post>::new(), "cAv", &[8]);
    let us = g(db).select(posts_of(db).opt().and(badges_of(db).select(&db.badge.class).opt())).fold([0i64; 3], |a, (_, c)| {
        [a[0] + (c == Some(1)) as i64, a[1] + (c == Some(2)) as i64, a[2] + (c == Some(3)) as i64]
    });
    let dp = ud(db, UserWhere::All, posts_of(db));
    let vp = db.vote.group_by(&db.vote.user).select(&db.vote.post_id).count_distinct();
    let uv = uvotes(db);
    let mut v = Vec::new();
    (&pf)
        .and(comments_per_post(db))
        .and(typed_answers_per_post(db))
        .and((&db.post.owner_user).select(Ident::<User>::new().and(&us).and((&dp).opt()).and((&vp).opt()).and((&uv).opt())))
        .drive(|p, (((s, c), a), ((((u, b), d), x), y))| v.push((p, s, c, a, u, b, d, x, y)));
    rows(v.iter().map(|&(p, s, c, a, u, b, d, x, y)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep"), V::I(d.unwrap_or(0))];
        f.extend(post_fields(db, p, &["id", "title", "created", "views", "score"]));
        f.extend([V::I(c), V::I(a), stat_field(&s, "bounty_sum").unwrap(), oint(x), oint(y.map(|y| y[1])), oint(y.map(|y| y[2]))]);
        f.extend(ints(&b));
        row(f)
    }))
}

// WITH UserBadges AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(b.Id) AS TotalBadges,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM Users u
// LEFT JOIN Badges b ON u.Id = b.UserId
// GROUP BY u.Id, u.DisplayName
// ),
// PostInteractions AS (
// SELECT
// p.Id AS PostId,
// p.OwnerUserId,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// COUNT(DISTINCT pl.RelatedPostId) AS RelatedPostsCount
// FROM Posts p
// LEFT JOIN Comments c ON p.Id = c.PostId
// LEFT JOIN Votes v ON p.Id = v.PostId
// LEFT JOIN PostLinks pl ON p.Id = pl.PostId
// GROUP BY p.Id, p.OwnerUserId
// ),
// UserActivity AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(CASE WHEN p.PostTypeId IN (3, 4, 5) THEN 1 ELSE 0 END) AS WikiPosts,
// SUM(pi.CommentCount) AS TotalComments,
// SUM(pi.UpVotes) AS TotalUpVotes,
// SUM(pi.DownVotes) AS TotalDownVotes,
// SUM(pi.RelatedPostsCount) AS TotalRelatedPosts
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN PostInteractions pi ON p.Id = pi.PostId
// GROUP BY u.Id, u.DisplayName
// )
// SELECT
// ua.UserId,
// ua.DisplayName,
// ua.TotalPosts,
// ua.Questions,
// ua.Answers,
// ua.WikiPosts,
// ua.TotalComments,
// ua.TotalUpVotes,
// ua.TotalDownVotes,
// ub.TotalBadges,
// ub.GoldBadges,
// ub.SilverBadges,
// ub.BronzeBadges
// FROM UserActivity ua
// JOIN UserBadges ub ON ua.UserId = ub.UserId
// ORDER BY ua.TotalPosts DESC, ub.TotalBadges DESC;
fn q27491(db: &'static So) -> String {
    let rel = per_post_distinct(db, links_of(db).select(&db.post_link.related_post_id));
    let pf = stats_fold(db, owned(db), Ident::<Post>::new(), "cvl", &[]);
    let ua = owned(db).group_by(&db.post.owner_user).select((&db.post.post_type_id).and(&pf).and((&rel).opt())).fold([0i64; 8], |a, ((t, s), r)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + matches!(t, 3 | 4 | 5) as i64, a[4] + s.cx, a[5] + s.up, a[6] + s.down, a[7] + r.unwrap_or(0)]
    });
    let ub = ubc(db);
    let mut v = Vec::new();
    (&ub).and((&ua).opt()).drive(|u, (b, a)| v.push((u, b, a)));
    rows(v.iter().map(|&(u, b, a)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        let q = a.unwrap_or([0; 8]);
        f.extend(ints(&q[..4]));
        f.extend((4..7).map(|i| oint(a.map(|a| a[i]))));
        f.extend(ints(&b));
        row(f)
    }))
}

// WITH UserBadgeCounts AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(b.Id) AS BadgeCount,
// SUM(CASE
// WHEN b.Class = 1 THEN 1
// ELSE 0
// END) AS GoldBadgeCount,
// SUM(CASE
// WHEN b.Class = 2 THEN 1
// ELSE 0
// END) AS SilverBadgeCount,
// SUM(CASE
// WHEN b.Class = 3 THEN 1
// ELSE 0
// END) AS BronzeBadgeCount
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
// COUNT(p.Id) AS PostCount,
// COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS QuestionCount,
// COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS AnswerCount,
// SUM(p.Score) AS TotalScore,
// AVG(p.ViewCount) AS AverageViews
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// ),
// UserPerformance AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COALESCE(ps.PostCount, 0) AS PostCount,
// COALESCE(ps.QuestionCount, 0) AS QuestionCount,
// COALESCE(ps.AnswerCount, 0) AS AnswerCount,
// COALESCE(ps.TotalScore, 0) AS TotalScore,
// COALESCE(ps.AverageViews, 0) AS AverageViews,
// COALESCE(bc.BadgeCount, 0) AS BadgeCount,
// COALESCE(bc.GoldBadgeCount, 0) AS GoldBadgeCount,
// COALESCE(bc.SilverBadgeCount, 0) AS SilverBadgeCount,
// COALESCE(bc.BronzeBadgeCount, 0) AS BronzeBadgeCount
// FROM
// Users u
// LEFT JOIN
// PostStatistics ps ON u.Id = ps.OwnerUserId
// LEFT JOIN
// UserBadgeCounts bc ON u.Id = bc.UserId
// )
// SELECT
// UserId,
// DisplayName,
// PostCount,
// QuestionCount,
// AnswerCount,
// TotalScore,
// AverageViews,
// BadgeCount,
// GoldBadgeCount,
// SilverBadgeCount,
// BronzeBadgeCount
// FROM
// UserPerformance
// ORDER BY
// TotalScore DESC,
// PostCount DESC
// LIMIT 10;
fn q28598(db: &'static So) -> String {
    let ub = ubc(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    (&ub).and((&ps).opt()).drive(|u, (b, p)| v.push((u, b, p.unwrap_or(Z))));
    out(v, |&(_, _, p)| (Reverse(p[3]), Reverse(p[0])), 10, |&(u, b, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&p[..4]));
        f.push(if p[4] > 0 { pviews_avg(p) } else { V::F(0.0) });
        f.extend(ints(&b));
        f
    })
}

// WITH RECURSIVE UserBadges AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
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
// AVG(P.Score) AS AvgScore,
// COALESCE(MAX(P.CreationDate), CAST('1900-01-01' AS timestamp)) AS LastPostDate
// FROM
// Posts P
// GROUP BY
// P.OwnerUserId
// ),
// UsersWithPostStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COALESCE(PS.TotalPosts, 0) AS TotalPosts,
// COALESCE(PS.Questions, 0) AS Questions,
// COALESCE(PS.Answers, 0) AS Answers,
// COALESCE(PS.AvgScore, 0) AS AvgScore,
// COALESCE(PS.LastPostDate, CAST('1900-01-01' AS timestamp)) AS LastPostDate,
// UB.GoldBadges,
// UB.SilverBadges,
// UB.BronzeBadges
// FROM
// Users U
// LEFT JOIN
// PostStatistics PS ON U.Id = PS.OwnerUserId
// LEFT JOIN
// UserBadges UB ON U.Id = UB.UserId
// )
// SELECT
// U.UserId,
// U.DisplayName,
// U.TotalPosts,
// U.Questions,
// U.Answers,
// U.AvgScore,
// U.LastPostDate,
// (U.GoldBadges + U.SilverBadges + U.BronzeBadges) AS TotalBadges,
// CASE
// WHEN U.TotalPosts > 100 THEN 'Active Contributor'
// WHEN U.TotalPosts BETWEEN 50 AND 100 THEN 'Moderately Active'
// ELSE 'New Contributor'
// END AS ActivityStatus
// FROM
// UsersWithPostStats U
// WHERE
// (U.TotalPosts > 0 OR U.GoldBadges > 0 OR U.SilverBadges > 0 OR U.BronzeBadges > 0)
// ORDER BY
// U.TotalPosts DESC, U.AvgScore DESC
// LIMIT 50;
fn q34003(db: &'static So) -> String {
    let ub = ubc(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    (&ub).and((&ps).opt()).filt(|(b, p): ([i64; 4], Option<[i64; 13]>)| p.map_or(0, |p| p[0]) > 0 || b[1] > 0 || b[2] > 0 || b[3] > 0).drive(|u, (b, p)| v.push((u, b, p)));
    let av = |p: Option<[i64; 13]>| p.map_or(0.0, |p| p[3] as f64 / p[0] as f64);
    out(v, |&(_, _, p)| (Reverse(p.map_or(0, |p| p[0])), Reverse(fkey(av(p)))), 50, |&(u, b, p)| {
        let q = p.unwrap_or(Z);
        vec![
            user_col(db, u, "uid"),
            user_col(db, u, "name"),
            V::I(q[0]),
            V::I(q[1]),
            V::I(q[2]),
            V::F(av(p)),
            V::T(p.map_or(date(1900, 1, 1), |p| p[10])),
            V::I(b[1] + b[2] + b[3]),
            V::S(if q[0] > 100 {
                "Active Contributor"
            } else if (50..=100).contains(&q[0]) {
                "Moderately Active"
            } else {
                "New Contributor"
            }),
        ]
    })
}

// WITH UserBadgeCounts AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges,
// COUNT(b.Id) AS TotalBadges
// FROM Users u
// LEFT JOIN Badges b ON u.Id = b.UserId
// GROUP BY u.Id, u.DisplayName
// ),
// PostStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS TotalPosts,
// COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS TotalQuestions,
// COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS TotalAnswers,
// SUM(COALESCE(p.Score, 0)) AS TotalScore
// FROM Posts p
// GROUP BY p.OwnerUserId
// ),
// ClosedPostCounts AS (
// SELECT
// ph.UserId,
// COUNT(ph.Id) AS ClosedPosts
// FROM PostHistory ph
// WHERE ph.PostHistoryTypeId = 10
// GROUP BY ph.UserId
// ),
// FinalResults AS (
// SELECT
// ub.UserId,
// ub.DisplayName,
// COALESCE(ps.TotalPosts, 0) AS TotalPosts,
// COALESCE(ps.TotalQuestions, 0) AS TotalQuestions,
// COALESCE(ps.TotalAnswers, 0) AS TotalAnswers,
// COALESCE(ps.TotalScore, 0) AS TotalScore,
// COALESCE(cb.ClosedPosts, 0) AS ClosedPosts,
// ub.GoldBadges,
// ub.SilverBadges,
// ub.BronzeBadges,
// ub.TotalBadges
// FROM UserBadgeCounts ub
// LEFT JOIN PostStats ps ON ub.UserId = ps.OwnerUserId
// LEFT JOIN ClosedPostCounts cb ON ub.UserId = cb.UserId
// )
// SELECT
// UserId,
// DisplayName,
// TotalPosts,
// TotalQuestions,
// TotalAnswers,
// TotalScore,
// ClosedPosts,
// GoldBadges,
// SilverBadges,
// BronzeBadges,
// TotalBadges,
// CASE
// WHEN TotalPosts > 0 AND GoldBadges > 0 THEN 'Active Contributor with Gold'
// WHEN TotalPosts > 0 AND TotalScore > 100 THEN 'Active Contributor with High Score'
// ELSE 'Various Activities'
// END AS UserActivityLabel
// FROM FinalResults
// ORDER BY TotalScore DESC, TotalPosts DESC
// LIMIT 50;
fn q23214(db: &'static So) -> String {
    let ub = ubc(db);
    let ps = pstat(db, db.post.iq());
    let cp = db.post_history.with((&db.post_history.post_history_type_id).eq(10)).group_by(&db.post_history.user).fold(0i64, |a, _| a + 1);
    let mut v = Vec::new();
    (&ub).and((&ps).opt()).and((&cp).opt()).drive(|u, ((b, p), c)| v.push((u, b, p.unwrap_or(Z), c.unwrap_or(0))));
    out(v, |&(_, _, p, _)| (Reverse(p[3]), Reverse(p[0])), 50, |&(u, b, p, c)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&[p[0], p[1], p[2], p[3], c, b[1], b[2], b[3], b[0]]));
        f.push(V::S(if p[0] > 0 && b[1] > 0 {
            "Active Contributor with Gold"
        } else if p[0] > 0 && p[3] > 100 {
            "Active Contributor with High Score"
        } else {
            "Various Activities"
        }));
        f
    })
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE
// WHEN p.PostTypeId = 1 THEN 1
// ELSE 0 END) AS TotalQuestions,
// SUM(CASE
// WHEN p.PostTypeId = 2 THEN 1
// ELSE 0 END) AS TotalAnswers,
// SUM(CASE
// WHEN p.Score > 0 THEN 1
// ELSE 0 END) AS TotalUpvotedPosts,
// AVG(p.Score) AS AverageScore,
// MAX(p.CreationDate) AS LastPostDate
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// GROUP BY u.Id, u.DisplayName
// ),
// TagPostStats AS (
// SELECT
// t.Id AS TagId,
// t.TagName,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE
// WHEN p.PostTypeId = 1 THEN 1
// ELSE 0 END) AS TotalQuestions,
// SUM(CASE
// WHEN p.PostTypeId = 2 THEN 1
// ELSE 0 END) AS TotalAnswers
// FROM Tags t
// LEFT JOIN Posts p ON t.Id = ANY(string_to_array(SUBSTRING(p.Tags, 2, LENGTH(p.Tags)-2), '><')::int[])
// GROUP BY t.Id, t.TagName
// ),
// UserBadgesStats AS (
// SELECT
// b.UserId,
// COUNT(b.Id) AS TotalBadges,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM Badges b
// GROUP BY b.UserId
// )
// SELECT
// u.UserId,
// u.DisplayName,
// COALESCE(u.TotalPosts, 0) AS TotalPosts,
// COALESCE(u.TotalQuestions, 0) AS TotalQuestions,
// COALESCE(u.TotalAnswers, 0) AS TotalAnswers,
// COALESCE(u.TotalUpvotedPosts, 0) AS TotalUpvotedPosts,
// COALESCE(u.AverageScore, 0) AS AverageScore,
// u.LastPostDate,
// COALESCE(b.TotalBadges, 0) AS TotalBadges,
// COALESCE(b.GoldBadges, 0) AS GoldBadges,
// COALESCE(b.SilverBadges, 0) AS SilverBadges,
// COALESCE(b.BronzeBadges, 0) AS BronzeBadges
// FROM UserPostStats u
// LEFT JOIN UserBadgesStats b ON u.UserId = b.UserId
// ORDER BY u.TotalPosts DESC
// LIMIT 10;
fn q10719(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, .. } = &db.post;
    let uf = g(db).select(posts_of(db).select(post_type_id.and(score).and(creation_date)).opt()).fold([0, 0, 0, 0, 0, i64::MIN], |a: [i64; 6], p| match p {
        Some(((t, s), c)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (s > 0) as i64, a[4] + s, a[5].max(c)],
        None => a,
    });
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&uf).and((&bc).opt()).drive(|u, (a, b)| v.push((u, a, b.unwrap_or([0; 4]))));
    out(v, |&(_, a, _)| Reverse(a[0]), 10, |&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[..4]));
        f.extend([if a[0] == 0 { V::F(0.0) } else { avg(a[4], a[0]) }, if a[0] == 0 { V::Null } else { V::T(a[5]) }]);
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
// GROUP BY
// P.OwnerUserId
// ),
// UserActivity AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COALESCE(US.BadgeCount, 0) AS BadgeCount,
// COALESCE(US.GoldBadges, 0) AS GoldBadges,
// COALESCE(US.SilverBadges, 0) AS SilverBadges,
// COALESCE(US.BronzeBadges, 0) AS BronzeBadges,
// COALESCE(PS.TotalPosts, 0) AS TotalPosts,
// COALESCE(PS.Questions, 0) AS Questions,
// COALESCE(PS.Answers, 0) AS Answers,
// COALESCE(PS.TotalViews, 0) AS TotalViews,
// COALESCE(PS.TotalScore, 0) AS TotalScore
// FROM
// Users U
// LEFT JOIN
// UserBadgeCounts US ON U.Id = US.UserId
// LEFT JOIN
// PostStats PS ON U.Id = PS.OwnerUserId
// )
// SELECT
// UA.UserId,
// UA.DisplayName,
// UA.BadgeCount,
// UA.GoldBadges,
// UA.SilverBadges,
// UA.BronzeBadges,
// UA.TotalPosts,
// UA.Questions,
// UA.Answers,
// UA.TotalViews,
// UA.TotalScore,
// ROUND(COALESCE(NULLIF(UA.TotalViews, 0), 1) / NULLIF(UA.TotalPosts, 0), 2) AS AvgViewsPerPost,
// ROUND(COALESCE(NULLIF(UA.TotalScore, 0), 1) / NULLIF(UA.TotalPosts, 0), 2) AS AvgScorePerPost
// FROM
// UserActivity UA
// WHERE
// UA.TotalPosts > 0
// ORDER BY
// UA.TotalScore DESC, UA.BadgeCount DESC
// LIMIT 10;
fn q25661(db: &'static So) -> String {
    let ub = ubc(db);
    let ps = pstat(db, db.post.iq());
    let per = |x: i64, n: i64| V::F(round2(if x == 0 { 1 } else { x } as f64 / n as f64));
    let mut v = Vec::new();
    (&ub).and((&ps).filt(|p: [i64; 13]| p[0] > 0)).drive(|u, (b, p)| v.push((u, b, p)));
    out(v, |&(_, b, p)| (Reverse(p[3]), Reverse(b[0])), 10, |&(u, b, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&b));
        f.extend(ints(&[p[0], p[1], p[2], p[5], p[3]]));
        f.extend([per(p[5], p[0]), per(p[3], p[0])]);
        f
    })
}

// WITH UserActivity AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(CASE WHEN p.PostTypeId = 1 THEN p.Score ELSE 0 END) AS TotalQuestionScore,
// SUM(CASE WHEN p.PostTypeId = 2 THEN p.Score ELSE 0 END) AS TotalAnswerScore,
// AVG(COALESCE(NULLIF(p.ViewCount, 0), 1)) AS AvgViewCount,
// SUM(COALESCE(NULLIF(c.Id, 0), 0)) AS TotalComments
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// GROUP BY
// u.Id, u.DisplayName
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
// GROUP BY
// b.UserId
// ),
// CombinedStats AS (
// SELECT
// ua.UserId,
// ua.DisplayName,
// ua.TotalPosts,
// ua.Questions,
// ua.Answers,
// ua.TotalQuestionScore,
// ua.TotalAnswerScore,
// ua.AvgViewCount,
// ua.TotalComments,
// COALESCE(bc.TotalBadges, 0) AS TotalBadges,
// COALESCE(bc.GoldBadges, 0) AS GoldBadges,
// COALESCE(bc.SilverBadges, 0) AS SilverBadges,
// COALESCE(bc.BronzeBadges, 0) AS BronzeBadges
// FROM
// UserActivity ua
// LEFT JOIN
// BadgeCounts bc ON ua.UserId = bc.UserId
// )
// SELECT
// cs.DisplayName,
// cs.TotalPosts,
// cs.Questions,
// cs.Answers,
// cs.TotalQuestionScore,
// cs.TotalAnswerScore,
// cs.AvgViewCount,
// cs.TotalComments,
// cs.TotalBadges,
// cs.GoldBadges,
// cs.SilverBadges,
// cs.BronzeBadges
// FROM
// CombinedStats cs
// WHERE
// cs.TotalPosts > 10
// ORDER BY
// cs.TotalQuestionScore DESC, cs.TotalPosts DESC
// LIMIT 100;
fn q9769(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let uf = g(db)
        .select(posts_of(db).select(post_type_id.and(score).and(view_count.opt()).and(comments_of(db).select(&db.comment.origid).opt())).opt())
        .fold([0i64; 8], |a, p| match p {
            Some((((t, s), w), c)) => [
                a[0] + 1,
                a[1] + 1,
                a[2] + (t == 1) as i64,
                a[3] + (t == 2) as i64,
                a[4] + if t == 1 { s } else { 0 },
                a[5] + if t == 2 { s } else { 0 },
                a[6] + w.filter(|&w| w != 0).unwrap_or(1),
                a[7] + c.unwrap_or(0),
            ],
            None => [a[0] + 1, a[1], a[2], a[3], a[4], a[5], a[6] + 1, a[7]],
        });
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&uf).filt(|a: [i64; 8]| a[1] > 10).and((&bc).opt()).drive(|u, (a, b)| v.push((u, a, b.unwrap_or([0; 4]))));
    out(v, |&(_, a, _)| (Reverse(a[4]), Reverse(a[1])), 100, |&(u, a, b)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(ints(&a[1..6]));
        f.extend([V::F(a[6] as f64 / a[0] as f64), V::I(a[7])]);
        f.extend(ints(&b));
        f
    })
}

// WITH UserStatistics AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
// COALESCE(SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges,
// COALESCE(SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadges,
// COALESCE(SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadges
// FROM
// Users u
// LEFT JOIN
// Votes v ON u.Id = v.UserId
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// u.Id, u.DisplayName
// ),
// PostAggregates AS (
// SELECT
// p.OwnerUserId,
// COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS QuestionCount,
// COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS AnswerCount,
// SUM(p.Score) AS TotalScore,
// AVG(p.ViewCount) AS AvgViewCount,
// MAX(p.CreationDate) AS MostRecentPost
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// ),
// UserPerformance AS (
// SELECT
// us.UserId,
// us.DisplayName,
// COALESCE(pa.QuestionCount, 0) AS QuestionCount,
// COALESCE(pa.AnswerCount, 0) AS AnswerCount,
// COALESCE(pa.TotalScore, 0) AS TotalScore,
// COALESCE(pa.AvgViewCount, 0) AS AvgViewCount,
// pa.MostRecentPost,
// us.UpVotes,
// us.DownVotes,
// us.GoldBadges,
// us.SilverBadges,
// us.BronzeBadges
// FROM
// UserStatistics us
// LEFT JOIN
// PostAggregates pa ON us.UserId = pa.OwnerUserId
// )
// SELECT
// u.DisplayName,
// u.QuestionCount,
// u.AnswerCount,
// u.TotalScore,
// u.AvgViewCount,
// u.UpVotes,
// u.DownVotes,
// u.GoldBadges,
// u.SilverBadges,
// u.BronzeBadges,
// CASE
// WHEN u.TotalScore > 100 THEN 'High Performer'
// WHEN u.TotalScore BETWEEN 50 AND 100 THEN 'Medium Performer'
// ELSE 'Low Performer'
// END AS PerformanceCategory
// FROM
// UserPerformance u
// ORDER BY
// u.TotalScore DESC;
fn q2661(db: &'static So) -> String {
    let us = g(db).select(votes_by(db).select(&db.vote.vote_type_id).opt().and(badges_of(db).select(&db.badge.class).opt())).fold([0i64; 5], |a, (t, c)| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + (c == Some(1)) as i64, a[3] + (c == Some(2)) as i64, a[4] + (c == Some(3)) as i64]
    });
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    (&us).and((&ps).opt()).drive(|u, (a, p)| v.push((u, a, p)));
    rows(v.iter().map(|&(u, a, p)| {
        let q = p.unwrap_or(Z);
        let mut f = vec![user_col(db, u, "name"), V::I(q[1]), V::I(q[2]), V::I(q[3]), if q[4] > 0 { pviews_avg(q) } else { V::F(0.0) }];
        f.extend(ints(&a));
        f.push(V::S(if q[3] > 100 {
            "High Performer"
        } else if (50..=100).contains(&q[3]) {
            "Medium Performer"
        } else {
            "Low Performer"
        }));
        row(f)
    }))
}

// WITH UserBadgeStats AS (
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
// PostAggStats AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// AVG(P.Score) AS AvgScore,
// SUM(P.ViewCount) AS TotalViews
// FROM Posts P
// GROUP BY P.OwnerUserId
// ),
// PostHistoryStats AS (
// SELECT
// PH.UserId,
// COUNT(PH.Id) AS TotalEdits,
// SUM(CASE WHEN PH.PostHistoryTypeId IN (4, 5, 6) THEN 1 ELSE 0 END) AS TitleEdits,
// SUM(CASE WHEN PH.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS PostsClosed,
// SUM(CASE WHEN PH.PostHistoryTypeId = 12 THEN 1 ELSE 0 END) AS PostsDeleted
// FROM PostHistory PH
// GROUP BY PH.UserId
// )
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COALESCE(UB.BadgeCount, 0) AS BadgeCount,
// COALESCE(UB.GoldBadges, 0) AS GoldBadges,
// COALESCE(UB.SilverBadges, 0) AS SilverBadges,
// COALESCE(UB.BronzeBadges, 0) AS BronzeBadges,
// COALESCE(PA.TotalPosts, 0) AS TotalPosts,
// COALESCE(PA.QuestionCount, 0) AS QuestionCount,
// COALESCE(PA.AnswerCount, 0) AS AnswerCount,
// COALESCE(PA.AvgScore, 0) AS AvgScore,
// COALESCE(PA.TotalViews, 0) AS TotalViews,
// COALESCE(PH.TotalEdits, 0) AS TotalEdits,
// COALESCE(PH.TitleEdits, 0) AS TitleEdits,
// COALESCE(PH.PostsClosed, 0) AS PostsClosed,
// COALESCE(PH.PostsDeleted, 0) AS PostsDeleted
// FROM Users U
// LEFT JOIN UserBadgeStats UB ON U.Id = UB.UserId
// LEFT JOIN PostAggStats PA ON U.Id = PA.OwnerUserId
// LEFT JOIN PostHistoryStats PH ON U.Id = PH.UserId
// ORDER BY BadgeCount DESC, TotalPosts DESC, AvgScore DESC
// LIMIT 100;
fn q28437(db: &'static So) -> String {
    let ub = ubc(db);
    let ps = pstat(db, db.post.iq());
    let ph = phu(db);
    let mut v = Vec::new();
    (&ub).and((&ps).opt()).and((&ph).opt()).drive(|u, ((b, p), h)| v.push((u, b, p, h.unwrap_or([0; 8]))));
    let av = |p: Option<[i64; 13]>| p.map_or(0.0, |p| p[3] as f64 / p[0] as f64);
    out(v, |&(_, b, p, _)| (Reverse(b[0]), Reverse(p.map_or(0, |p| p[0])), Reverse(fkey(av(p)))), 100, |&(u, b, p, h)| {
        let q = p.unwrap_or(Z);
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&b));
        f.extend(ints(&q[..3]));
        f.extend([V::F(av(p)), V::I(q[5])]);
        f.extend(ints(&[h[0], h[1], h[3], h[5]]));
        f
    })
}

// WITH PostMetrics AS (
// SELECT
// COUNT(*) AS TotalPosts,
// COUNT(DISTINCT OwnerUserId) AS UniquePostOwners,
// AVG(ViewCount) AS AvgViewCount,
// SUM(CASE WHEN AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS TotalAcceptedAnswers,
// SUM(CASE WHEN PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers
// FROM
// Posts
// ),
// UserMetrics AS (
// SELECT
// COUNT(*) AS TotalUsers,
// AVG(Reputation) AS AvgReputation,
// COUNT(DISTINCT CASE WHEN LastAccessDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' THEN Id END) AS ActiveUsersLast30Days
// FROM
// Users
// ),
// VoteMetrics AS (
// SELECT
// COUNT(*) AS TotalVotes,
// SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
// FROM
// Votes
// ),
// BadgeMetrics AS (
// SELECT
// COUNT(*) AS TotalBadges,
// COUNT(DISTINCT UserId) AS UsersWithBadges,
// SUM(CASE WHEN Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM
// Badges
// ),
// PostHistoryMetrics AS (
// SELECT
// COUNT(*) AS TotalPostHistories,
// COUNT(DISTINCT PostId) AS UniquePostsEdited,
// COUNT(DISTINCT UserId) AS UniqueEditors
// FROM
// PostHistory
// )
// SELECT
// pm.TotalPosts,
// pm.UniquePostOwners,
// pm.AvgViewCount,
// pm.TotalAcceptedAnswers,
// pm.TotalQuestions,
// pm.TotalAnswers,
// um.TotalUsers,
// um.AvgReputation,
// um.ActiveUsersLast30Days,
// vm.TotalVotes,
// vm.TotalUpVotes,
// vm.TotalDownVotes,
// bm.TotalBadges,
// bm.UsersWithBadges,
// bm.GoldBadges,
// bm.SilverBadges,
// bm.BronzeBadges,
// phm.TotalPostHistories,
// phm.UniquePostsEdited,
// phm.UniqueEditors
// FROM
// PostMetrics pm,
// UserMetrics um,
// VoteMetrics vm,
// BadgeMetrics bm,
// PostHistoryMetrics phm;
fn q14062(db: &'static So) -> String {
    let Post { view_count, accepted_answer_id, post_type_id, .. } = &db.post;
    let p = db.post.select(view_count.opt().and(accepted_answer_id.opt()).and(post_type_id)).fold_flat([0i64; 6], |a, ((w, ac), t)| {
        [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + ac.is_some() as i64, a[4] + (t == 1) as i64, a[5] + (t == 2) as i64]
    });
    let du = one(whole(db.post.iq()).select(&db.post.owner_user_id).count_distinct());
    let u = db.user.select(&db.user.reputation).fold_flat([0i64; 2], |a, r| [a[0] + 1, a[1] + r]);
    let act = count(db.user.with((&db.user.last_access_date).ge(month_ago())));
    let x = db.vote.select(&db.vote.vote_type_id).fold_flat([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64]);
    let b = db.badge.select(&db.badge.class).fold_flat([0i64; 4], |a, c| [a[0] + 1, a[1] + (c == 1) as i64, a[2] + (c == 2) as i64, a[3] + (c == 3) as i64]);
    let bu = one(whole(db.badge.iq()).select(&db.badge.user_id).count_distinct());
    let hp = one(whole(db.post_history.iq()).select(&db.post_history.post_id).count_distinct());
    let hu = one(whole(db.post_history.iq()).select(&db.post_history.user_id).count_distinct());
    row(vec![
        V::I(p[0]),
        V::I(du),
        avg(p[2], p[1]),
        V::I(p[3]),
        V::I(p[4]),
        V::I(p[5]),
        V::I(u[0]),
        avg(u[1], u[0]),
        V::I(act),
        V::I(x[0]),
        V::I(x[1]),
        V::I(x[2]),
        V::I(b[0]),
        V::I(bu),
        V::I(b[1]),
        V::I(b[2]),
        V::I(b[3]),
        V::I(count(db.post_history.iq())),
        V::I(hp),
        V::I(hu),
    ])
}

// WITH UserBadgeCount AS (
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
// PostStatistics AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS PostCount,
// SUM(P.Score) AS TotalScore,
// SUM(P.ViewCount) AS TotalViews,
// AVG(P.AnswerCount) AS AvgAnswersPerQuestion,
// AVG(P.CommentCount) AS AvgCommentsPerPost
// FROM
// Posts P
// WHERE
// P.PostTypeId IN (1, 2)
// GROUP BY
// P.OwnerUserId
// ),
// UserEngagement AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COALESCE(UB.BadgeCount, 0) AS BadgeCount,
// COALESCE(PS.PostCount, 0) AS PostCount,
// COALESCE(PS.TotalScore, 0) AS TotalScore,
// COALESCE(PS.TotalViews, 0) AS TotalViews,
// COALESCE(PS.AvgAnswersPerQuestion, 0) AS AvgAnswersPerQuestion,
// COALESCE(PS.AvgCommentsPerPost, 0) AS AvgCommentsPerPost,
// COALESCE(UB.GoldBadges, 0) AS GoldBadges,
// COALESCE(UB.SilverBadges, 0) AS SilverBadges,
// COALESCE(UB.BronzeBadges, 0) AS BronzeBadges
// FROM
// Users U
// LEFT JOIN
// UserBadgeCount UB ON U.Id = UB.UserId
// LEFT JOIN
// PostStatistics PS ON U.Id = PS.OwnerUserId
// )
// SELECT
// UE.DisplayName,
// UE.Reputation,
// UE.BadgeCount,
// UE.PostCount,
// UE.TotalScore,
// UE.TotalViews,
// UE.AvgAnswersPerQuestion,
// UE.AvgCommentsPerPost,
// CASE
// WHEN UE.Reputation >= 1000 THEN 'High Reputation'
// WHEN UE.Reputation >= 500 THEN 'Medium Reputation'
// ELSE 'Low Reputation'
// END AS ReputationTier,
// CONCAT('Badges: Gold(', UE.GoldBadges, '), Silver(', UE.SilverBadges, '), Bronze(', UE.BronzeBadges, ')') AS BadgeSummary
// FROM
// UserEngagement UE
// ORDER BY
// UE.Reputation DESC, UE.PostCount DESC;
fn q28350(db: &'static So) -> String {
    let ub = ubc(db);
    let ps = pstat(db, db.post.with((&db.post.post_type_id).in_v(vec![1, 2])));
    let cc = owned(db).with((&db.post.post_type_id).in_v(vec![1, 2])).group_by(&db.post.owner_user).select(&db.post.comment_count).fold([0i64; 2], |a, c| [a[0] + 1, a[1] + c]);
    let mut v = Vec::new();
    (&ub).and((&ps).opt()).and((&cc).opt()).drive(|u, ((b, p), c)| v.push((u, b, p, c)));
    rows(v.iter().map(|&(u, b, p, c)| {
        let r = db.user.reputation.get(u).unwrap();
        let q = p.unwrap_or(Z);
        row(vec![
            user_col(db, u, "name"),
            V::I(r),
            V::I(b[0]),
            V::I(q[0]),
            V::I(q[3]),
            V::I(q[5]),
            if q[6] > 0 { avg(q[7], q[6]) } else { V::F(0.0) },
            c.map_or(V::F(0.0), |c| avg(c[1], c[0])),
            V::S(if r >= 1000 {
                "High Reputation"
            } else if r >= 500 {
                "Medium Reputation"
            } else {
                "Low Reputation"
            }),
            V::S(leak(format!("Badges: Gold({}), Silver({}), Bronze({})", b[1], b[2], b[3]))),
        ])
    }))
}

// WITH UserVoteStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVotes,
// COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVotes,
// COUNT(*) AS TotalVotes,
// COALESCE(SUM(V.BountyAmount), 0) AS TotalBountyAmount
// FROM
// Users U
// LEFT JOIN
// Votes V ON U.Id = V.UserId
// GROUP BY
// U.Id, U.DisplayName
// ),
// PostStats AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(COALESCE(P.FavoriteCount, 0)) AS FavoritePosts
// FROM
// Posts P
// GROUP BY
// P.OwnerUserId
// ),
// CombinedStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COALESCE(P.TotalPosts, 0) AS TotalPosts,
// COALESCE(P.Questions, 0) AS Questions,
// COALESCE(P.Answers, 0) AS Answers,
// COALESCE(UV.UpVotes, 0) AS UpVotes,
// COALESCE(UV.DownVotes, 0) AS DownVotes,
// COALESCE(UV.TotalVotes, 0) AS TotalVotes,
// COALESCE(UV.TotalBountyAmount, 0) AS TotalBountyAmount,
// COALESCE(P.FavoritePosts, 0) AS FavoritePosts
// FROM
// Users U
// LEFT JOIN
// PostStats P ON U.Id = P.OwnerUserId
// LEFT JOIN
// UserVoteStats UV ON U.Id = UV.UserId
// )
// SELECT
// C.UserId,
// C.DisplayName,
// C.TotalPosts,
// C.Questions,
// C.Answers,
// C.UpVotes,
// C.DownVotes,
// C.TotalVotes,
// C.TotalBountyAmount,
// C.FavoritePosts,
// CASE
// WHEN C.UpVotes > C.DownVotes THEN 'Predominantly Positive'
// WHEN C.UpVotes < C.DownVotes THEN 'Predominantly Negative'
// ELSE 'Neutral'
// END AS VoteSentiment,
// CASE
// WHEN C.TotalPosts > 100 THEN 'Veteran'
// WHEN C.TotalPosts BETWEEN 50 AND 100 THEN 'Experienced'
// ELSE 'Novice'
// END AS UserLevel
// FROM
// CombinedStats C
// WHERE
// C.TotalVotes > 10 AND
// C.UpVotes / NULLIF(C.TotalVotes, 0) > 0.5
// ORDER BY
// C.TotalBountyAmount DESC,
// C.UpVotes DESC;
fn q3609(db: &'static So) -> String {
    let uv = g(db).select(votes_by(db).select((&db.vote.vote_type_id).and((&db.vote.bounty_amount).opt())).opt()).fold([0i64; 4], |a, x| match x {
        Some((t, b)) => [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + 1, a[3] + b.unwrap_or(0)],
        None => [a[0], a[1], a[2] + 1, a[3]],
    });
    let fav = owned(db).group_by(&db.post.owner_user).select((&db.post.post_type_id).and((&db.post.favorite_count).opt())).fold([0i64; 4], |a, (t, f)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + f.unwrap_or(0)]
    });
    let mut v = Vec::new();
    (&uv).filt(|a: [i64; 4]| a[2] > 10 && a[0] as f64 / a[2] as f64 > 0.5).and((&fav).opt()).drive(|u, (a, p)| v.push((u, a, p.unwrap_or([0; 4]))));
    rows(v.iter().map(|&(u, a, p)| {
        row(vec![
            user_col(db, u, "uid"),
            user_col(db, u, "name"),
            V::I(p[0]),
            V::I(p[1]),
            V::I(p[2]),
            V::I(a[0]),
            V::I(a[1]),
            V::I(a[2]),
            V::I(a[3]),
            V::I(p[3]),
            V::S(if a[0] > a[1] {
                "Predominantly Positive"
            } else if a[0] < a[1] {
                "Predominantly Negative"
            } else {
                "Neutral"
            }),
            V::S(if p[0] > 100 {
                "Veteran"
            } else if (50..=100).contains(&p[0]) {
                "Experienced"
            } else {
                "Novice"
            }),
        ])
    }))
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
// COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS Questions,
// COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS Answers,
// SUM(P.Score) AS TotalScore,
// SUM(P.ViewCount) AS TotalViews,
// AVG(P.ViewCount) AS AvgViewsPerPost
// FROM
// Posts P
// GROUP BY
// P.OwnerUserId
// ),
// UserEngagement AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COALESCE(UB.TotalBadges, 0) AS TotalBadges,
// COALESCE(PS.TotalPosts, 0) AS TotalPosts,
// COALESCE(PS.Questions, 0) AS TotalQuestions,
// COALESCE(PS.Answers, 0) AS TotalAnswers,
// COALESCE(PS.TotalScore, 0) AS TotalScore,
// COALESCE(PS.TotalViews, 0) AS TotalViews
// FROM
// Users U
// LEFT JOIN
// UserBadgeStats UB ON U.Id = UB.UserId
// LEFT JOIN
// PostStats PS ON U.Id = PS.OwnerUserId
// ),
// HighEngagementUsers AS (
// SELECT
// UE.UserId,
// UE.DisplayName,
// UE.TotalBadges,
// UE.TotalPosts,
// UE.TotalQuestions,
// UE.TotalAnswers,
// UE.TotalScore,
// UE.TotalViews,
// CAST(UE.TotalScore AS FLOAT) / NULLIF(UE.TotalPosts, 0) AS AvgScorePerPost,
// CAST(UE.TotalViews AS FLOAT) / NULLIF(UE.TotalPosts, 0) AS AvgViewsPerPost
// FROM
// UserEngagement UE
// WHERE
// UE.TotalPosts > 10
// )
// SELECT
// H.UserId,
// H.DisplayName,
// H.TotalBadges,
// H.TotalPosts,
// H.TotalQuestions,
// H.TotalAnswers,
// H.TotalScore,
// H.TotalViews,
// H.AvgScorePerPost,
// H.AvgViewsPerPost
// FROM
// HighEngagementUsers H
// ORDER BY
// H.AvgScorePerPost DESC, H.TotalViews DESC
// LIMIT 10;
fn q27175(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let ps = pstat(db, db.post.iq());
    let per = |x: i64, n: i64| (x as f32 / n as f32) as f64;
    let mut v = Vec::new();
    (&ps).filt(|p: [i64; 13]| p[0] > 10).and(&bu).drive(|u, (p, b)| v.push((u, p, b)));
    out(v, |&(_, p, _)| (Reverse(fkey(per(p[3], p[0]))), Reverse(p[5])), 10, |&(u, p, b)| {
        vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(b), V::I(p[0]), V::I(p[1]), V::I(p[2]), V::I(p[3]), V::I(p[5]), V::F(per(p[3], p[0])), V::F(per(p[5], p[0]))]
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
// GROUP BY
// U.Id, U.DisplayName
// ),
// PostMetrics AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// AVG(P.Score) AS AvgScore,
// SUM(P.ViewCount) AS TotalViews
// FROM
// Posts P
// GROUP BY
// P.OwnerUserId
// ),
// UserActivity AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// UB.BadgeCount,
// PM.TotalPosts,
// PM.TotalQuestions,
// PM.TotalAnswers,
// PM.AvgScore,
// PM.TotalViews
// FROM
// Users U
// LEFT JOIN
// UserBadges UB ON U.Id = UB.UserId
// LEFT JOIN
// PostMetrics PM ON U.Id = PM.OwnerUserId
// ),
// TopUsers AS (
// SELECT
// UserId,
// DisplayName,
// BadgeCount,
// TotalPosts,
// TotalQuestions,
// TotalAnswers,
// AvgScore,
// TotalViews
// FROM
// UserActivity
// WHERE
// TotalPosts > 0
// ORDER BY
// TotalPosts DESC, AvgScore DESC
// LIMIT 10
// )
// SELECT
// TU.DisplayName,
// COALESCE(TU.BadgeCount, 0) AS BadgeCount,
// COALESCE(TU.TotalPosts, 0) AS TotalPosts,
// COALESCE(TU.TotalQuestions, 0) AS TotalQuestions,
// COALESCE(TU.TotalAnswers, 0) AS TotalAnswers,
// COALESCE(TU.AvgScore, 0) AS AvgScore,
// COALESCE(TU.TotalViews, 0) AS TotalViews,
// PHT.Name AS RecentPostEditType
// FROM
// TopUsers TU
// LEFT JOIN
// PostHistory PH ON PH.UserId = TU.UserId
// LEFT JOIN
// PostHistoryTypes PHT ON PH.PostHistoryTypeId = PHT.Id
// WHERE
// PH.CreationDate = (
// SELECT MAX(CreationDate)
// FROM PostHistory
// WHERE UserId = TU.UserId
// )
// ORDER BY
// TU.BadgeCount DESC, TU.TotalPosts DESC;
fn q4004(db: &'static So) -> String {
    let bf = db.badge.group_by(&db.badge.user).fold(0i64, |a, _| a + 1);
    let ps = pstat(db, db.post.iq());
    let base = db.user.with(&ps);
    let av = |p: [i64; 13]| p[3] as f64 / p[0] as f64;
    let top: MatSet<Id<User>> = whole(&base)
        .select(Ident::<User>::new().and(&ps))
        .window(row_number, move |(_, p): (Id<User>, [i64; 13])| (p[0], fkey(av(p))), desc)
        .filt(|(_, n)| n <= 10)
        .map(|((u, _), _)| u)
        .collect();
    let hmax = db.post_history.group_by(&db.post_history.user).select(&db.post_history.creation_date).fold(i64::MIN, |a, d| a.max(d));
    let hbu: HashIdx<Id<User>, Id<PostHistory>> = (&db.post_history.user).inv().collect();
    let mut v = Vec::new();
    (&top)
        .select(Ident::<User>::new().and((&bf).opt()).and(&ps).and(&hmax).and((&hbu).select(Ident::<PostHistory>::new().and(&db.post_history.creation_date))))
        .filt(|((((_, _), _), m), (_, d)): ((((Id<User>, Option<i64>), [i64; 13]), i64), (Id<PostHistory>, i64))| d == m)
        .drive(|_, x| v.push(x));
    rows(v.iter().map(|&((((u, b), p), _), (h, _))| {
        row(vec![
            user_col(db, u, "name"),
            V::I(b.unwrap_or(0)),
            V::I(p[0]),
            V::I(p[1]),
            V::I(p[2]),
            V::F(av(p)),
            V::I(if p[4] > 0 { p[5] } else { 0 }),
            V::S(db.post_history_type.name.get(db.post_history.post_history_type.get(h).unwrap()).unwrap()),
        ])
    }))
}

// WITH UserActivity AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN P.PostTypeId = 3 THEN 1 ELSE 0 END) AS TotalWikis,
// SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END) AS TotalUpvotedPosts,
// SUM(CASE WHEN P.Score < 0 THEN 1 ELSE 0 END) AS TotalDownvotedPosts,
// AVG(COALESCE(P.ViewCount, 0)) AS AvgViewCount,
// AVG(COALESCE(U.Reputation, 0)) AS AvgReputation
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id, U.DisplayName
// ),
// BadgeStatistics AS (
// SELECT
// B.UserId,
// COUNT(B.Id) AS BadgeCount,
// SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM
// Badges B
// GROUP BY
// B.UserId
// ),
// PostHistoryStats AS (
// SELECT
// PH.UserId,
// COUNT(PH.Id) AS EditsCount,
// SUM(CASE WHEN PH.PostHistoryTypeId IN (4, 5, 6) THEN 1 ELSE 0 END) AS TitleEdits,
// SUM(CASE WHEN PH.PostHistoryTypeId IN (10, 11) THEN 1 ELSE 0 END) AS CloseReopenActions
// FROM
// PostHistory PH
// GROUP BY
// PH.UserId
// )
// SELECT
// UA.UserId,
// UA.DisplayName,
// UA.TotalPosts,
// UA.TotalQuestions,
// UA.TotalAnswers,
// UA.TotalWikis,
// UA.TotalUpvotedPosts,
// UA.TotalDownvotedPosts,
// UA.AvgViewCount,
// UA.AvgReputation,
// COALESCE(BS.BadgeCount, 0) AS BadgeCount,
// COALESCE(BS.GoldBadges, 0) AS GoldBadges,
// COALESCE(BS.SilverBadges, 0) AS SilverBadges,
// COALESCE(BS.BronzeBadges, 0) AS BronzeBadges,
// COALESCE(PHS.EditsCount, 0) AS EditsCount,
// COALESCE(PHS.TitleEdits, 0) AS TitleEdits,
// COALESCE(PHS.CloseReopenActions, 0) AS CloseReopenActions
// FROM
// UserActivity UA
// LEFT JOIN
// BadgeStatistics BS ON UA.UserId = BS.UserId
// LEFT JOIN
// PostHistoryStats PHS ON UA.UserId = PHS.UserId
// ORDER BY
// UA.TotalPosts DESC, UA.AvgReputation DESC
// LIMIT 100;
fn q8411(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let uf = g(db).select(posts_of(db).select(post_type_id.and(score).and(view_count.opt())).opt()).fold([0i64; 8], |a, p| match p {
        Some(((t, s), w)) => [a[0] + 1, a[1] + 1, a[2] + (t == 1) as i64, a[3] + (t == 2) as i64, a[4] + (t == 3) as i64, a[5] + (s > 0) as i64, a[6] + (s < 0) as i64, a[7] + w.unwrap_or(0)],
        None => [a[0] + 1, a[1], a[2], a[3], a[4], a[5], a[6], a[7]],
    });
    let bc = badge_classes(db);
    let ph = phu(db);
    let mut v = Vec::new();
    (&uf).and((&bc).opt()).and((&ph).opt()).drive(|u, ((a, b), h)| v.push((u, a, b.unwrap_or([0; 4]), h.unwrap_or([0; 8]))));
    out(v, |&(u, a, _, _)| (Reverse(a[1]), rep_desc(db, u)), 100, |&(u, a, b, h)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[1..7]));
        f.extend([V::F(a[7] as f64 / a[0] as f64), V::F(db.user.reputation.get(u).unwrap() as f64)]);
        f.extend(ints(&b));
        f.extend(ints(&[h[0], h[1], h[2]]));
        f
    })
}

// WITH UserActivity AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsPosted,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersPosted,
// SUM(CASE WHEN P.ViewCount > 100 THEN 1 ELSE 0 END) AS PopularPostsCount,
// COALESCE(SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges,
// COALESCE(SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadges,
// COALESCE(SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadges
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// GROUP BY
// U.Id, U.DisplayName
// ),
// UserPostHistory AS (
// SELECT
// UP.Id AS UserId,
// COUNT(PH.Id) AS TotalPostEdits,
// SUM(CASE WHEN PH.PostHistoryTypeId IN (4, 5, 6) THEN 1 ELSE 0 END) AS TitleBodyTagEdits,
// SUM(CASE WHEN PH.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS CloseVotes,
// SUM(CASE WHEN PH.PostHistoryTypeId = 11 THEN 1 ELSE 0 END) AS ReopenVotes
// FROM
// Users UP
// LEFT JOIN
// PostHistory PH ON UP.Id = PH.UserId
// GROUP BY
// UP.Id
// ),
// UserStatistics AS (
// SELECT
// UA.UserId,
// UA.DisplayName,
// UA.TotalPosts,
// UA.QuestionsPosted,
// UA.AnswersPosted,
// UA.PopularPostsCount,
// UA.GoldBadges,
// UA.SilverBadges,
// UA.BronzeBadges,
// UPH.TotalPostEdits,
// UPH.TitleBodyTagEdits,
// UPH.CloseVotes,
// UPH.ReopenVotes
// FROM
// UserActivity UA
// LEFT JOIN
// UserPostHistory UPH ON UA.UserId = UPH.UserId
// )
// SELECT
// UserId,
// DisplayName,
// TotalPosts,
// QuestionsPosted,
// AnswersPosted,
// PopularPostsCount,
// GoldBadges,
// SilverBadges,
// BronzeBadges,
// TotalPostEdits,
// TitleBodyTagEdits,
// CloseVotes,
// ReopenVotes,
// ROUND(COALESCE(TotalPosts::DECIMAL / NULLIF(QuestionsPosted, 0), 0), 2) AS PostToQuestionRatio,
// ROUND(COALESCE(AnswersPosted::DECIMAL / NULLIF(QuestionsPosted, 0), 0), 2) AS AnswerToQuestionRatio
// FROM
// UserStatistics
// ORDER BY
// TotalPosts DESC;
fn q27713(db: &'static So) -> String {
    let Post { post_type_id, view_count, .. } = &db.post;
    let uf = g(db).select(posts_of(db).select(post_type_id.and(view_count.opt())).opt().and(badges_of(db).select(&db.badge.class).opt())).fold([0i64; 7], |a, (p, c)| {
        let (n, t, w) = match p {
            Some((t, w)) => (1, Some(t), w),
            None => (0, None, None),
        };
        [a[0] + n, a[1] + (t == Some(1)) as i64, a[2] + (t == Some(2)) as i64, a[3] + w.map_or(false, |w| w > 100) as i64, a[4] + (c == Some(1)) as i64, a[5] + (c == Some(2)) as i64, a[6] + (c == Some(3)) as i64]
    });
    let ph = phu(db);
    let r = |x: i64, d: i64| V::F(if d == 0 { 0.0 } else { round2(x as f64 / d as f64) });
    let mut v = Vec::new();
    (&uf).and((&ph).opt()).drive(|u, (a, h)| v.push((u, a, h.unwrap_or([0; 8]))));
    rows(v.iter().map(|&(u, a, h)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a));
        f.extend(ints(&[h[0], h[1], h[3], h[4]]));
        f.extend([r(a[0], a[1]), r(a[2], a[1])]);
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
// SUM(CASE WHEN p.PostTypeId = 3 THEN 1 ELSE 0 END) AS Wikis,
// SUM(CASE WHEN p.PostTypeId IN (1, 2) THEN p.Score ELSE 0 END) AS TotalScore,
// SUM(p.ViewCount) AS TotalViews
// FROM
// Users u
// JOIN
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
// PostHistorySummary AS (
// SELECT
// ph.UserId,
// COUNT(ph.Id) AS TotalChanges,
// SUM(CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN 1 ELSE 0 END) AS CloseReopenCount,
// SUM(CASE WHEN ph.PostHistoryTypeId IN (6, 4) THEN 1 ELSE 0 END) AS TagTitleEditCount
// FROM
// PostHistory ph
// GROUP BY
// ph.UserId
// ),
// CombinedStats AS (
// SELECT
// ups.UserId,
// ups.DisplayName,
// ups.TotalPosts,
// ups.Questions,
// ups.Answers,
// ups.Wikis,
// ups.TotalScore,
// ups.TotalViews,
// ub.GoldBadges,
// ub.SilverBadges,
// ub.BronzeBadges,
// phs.TotalChanges,
// phs.CloseReopenCount,
// phs.TagTitleEditCount
// FROM
// UserPostStats ups
// LEFT JOIN
// UserBadges ub ON ups.UserId = ub.UserId
// LEFT JOIN
// PostHistorySummary phs ON ups.UserId = phs.UserId
// )
// SELECT
// UserId,
// DisplayName,
// TotalPosts,
// Questions,
// Answers,
// Wikis,
// TotalScore,
// TotalViews,
// COALESCE(GoldBadges, 0) AS GoldBadges,
// COALESCE(SilverBadges, 0) AS SilverBadges,
// COALESCE(BronzeBadges, 0) AS BronzeBadges,
// COALESCE(TotalChanges, 0) AS TotalChanges,
// COALESCE(CloseReopenCount, 0) AS CloseReopenCount,
// COALESCE(TagTitleEditCount, 0) AS TagTitleEditCount
// FROM
// CombinedStats
// ORDER BY
// TotalScore DESC, TotalPosts DESC;
fn q25718(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let ps = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(score).and(view_count.opt())).fold([0i64; 7], |a, ((t, s), w)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (t == 3) as i64, a[4] + if matches!(t, 1 | 2) { s } else { 0 }, a[5] + w.is_some() as i64, a[6] + w.unwrap_or(0)]
    });
    let bc = badge_classes(db);
    let ph = phu(db);
    let mut v = Vec::new();
    (&ps).and((&bc).opt()).and((&ph).opt()).drive(|u, ((p, b), h)| v.push((u, p, b.unwrap_or([0; 4]), h.unwrap_or([0; 8]))));
    rows(v.iter().map(|&(u, p, b, h)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&p[..5]));
        f.push(nullable(p[6], p[5]));
        f.extend(ints(&b[1..]));
        f.extend(ints(&[h[0], h[2], h[7]]));
        row(f)
    }))
}

// WITH RecentPosts AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.OwnerUserId,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.OwnerUserId
// ),
// UsersWithBadges AS (
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
// TopContributors AS (
// SELECT
// u.DisplayName,
// SUM(rp.UpVotes - rp.DownVotes) AS NetVotes,
// COUNT(rp.PostId) AS TotalPosts
// FROM
// UsersWithBadges uwb
// JOIN
// RecentPosts rp ON uwb.UserId = rp.OwnerUserId
// JOIN
// Users u ON uwb.UserId = u.Id
// GROUP BY
// u.DisplayName
// ORDER BY
// NetVotes DESC
// LIMIT 5
// ),
// PostHistoryChanges AS (
// SELECT
// ph.PostId,
// p.Title,
// ph.CreationDate,
// ph.UserDisplayName,
// ph.Comment,
// ph.UserId,
// p.ViewCount,
// pt.Name AS PostHistoryType
// FROM
// PostHistory ph
// INNER JOIN
// Posts p ON ph.PostId = p.Id
// LEFT JOIN
// PostHistoryTypes pt ON ph.PostHistoryTypeId = pt.Id
// WHERE
// ph.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '60 days'
// )
// SELECT
// p.Title AS PostTitle,
// p.CreationDate AS PostCreationDate,
// u.DisplayName AS PostOwner,
// ph.UserDisplayName AS LastEditedBy,
// ph.CreationDate AS LastEditDate,
// ph.Comment AS EditComment,
// ph.PostHistoryType AS ChangeType,
// rp.CommentCount,
// rp.UpVotes,
// rp.DownVotes
// FROM
// RecentPosts rp
// JOIN
// Posts p ON rp.PostId = p.Id
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// PostHistoryChanges ph ON rp.PostId = ph.PostId
// WHERE
// rp.CommentCount > 0
// OR (rp.UpVotes - rp.DownVotes) > 0
// ORDER BY
// rp.CreationDate DESC
// LIMIT
// 10;
fn q30824(db: &'static So) -> String {
    let rp = stats_fold(db, since(db, month_ago()), Ident::<Post>::new(), "cv", &[]);
    let hb = db.post_history.with((&db.post_history.creation_date).ge(ts(2024, 8, 2, 12, 34, 56))).select(&db.post_history.post).inv().collect::<HashIdx<Id<Post>, Id<PostHistory>>>();
    let mut v = Vec::new();
    (&rp).filt(|s: Stats| s.cx > 0 || s.up - s.down > 0).and(&db.post.owner_user).and((&hb).opt()).drive(|p, ((s, _), h)| v.push((p, s, h)));
    let PostHistory { user_display_name, creation_date, comment, post_history_type, .. } = &db.post_history;
    out(v, |&(p, _, _)| newest(db, p), 10, |&(p, s, h)| {
        let mut f = post_fields(db, p, &["title", "created", "owner"]);
        match h {
            Some(h) => f.extend([ostr(user_display_name.get(h)), V::T(creation_date.get(h).unwrap()), ostr(comment.get(h)), V::S(db.post_history_type.name.get(post_history_type.get(h).unwrap()).unwrap())]),
            None => f.extend(nulls(4)),
        }
        f.extend(ints(&[s.cx, s.up, s.down]));
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
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges,
// (SELECT COUNT(*) FROM Comments C WHERE C.UserId = U.Id) AS TotalComments
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// GROUP BY
// U.Id, U.DisplayName
// ),
// PostEngagement AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.ViewCount,
// P.CreationDate,
// COUNT(C.Id) AS CommentCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// P.Id, P.Title, P.ViewCount, P.CreationDate
// ),
// ActiveUsers AS (
// SELECT
// UA.UserId,
// UA.DisplayName,
// UA.PostCount,
// UA.QuestionCount,
// UA.AnswerCount,
// UA.UpVotes,
// UA.DownVotes,
// UA.GoldBadges,
// UA.SilverBadges,
// UA.BronzeBadges,
// UA.TotalComments,
// PE.PostId,
// PE.Title,
// PE.ViewCount,
// PE.CommentCount,
// PE.UpVotes AS PostUpVotes,
// PE.DownVotes AS PostDownVotes
// FROM
// UserActivity UA
// JOIN
// PostEngagement PE ON UA.UserId = PE.PostId
// WHERE
// UA.PostCount > 5
// )
// SELECT
// UserId,
// DisplayName,
// PostCount,
// QuestionCount,
// AnswerCount,
// UpVotes,
// DownVotes,
// GoldBadges,
// SilverBadges,
// BronzeBadges,
// TotalComments,
// Title,
// ViewCount,
// CommentCount,
// PostUpVotes,
// PostDownVotes
// FROM
// ActiveUsers
// ORDER BY
// PostCount DESC, UpVotes DESC
// LIMIT 100;
fn q6460(db: &'static So) -> String {
    let uid = uids(db);
    let uf = g(db)
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 7], |a, (p, c)| {
            let (t, x) = match p {
                Some((t, x)) => (Some(t), x),
                None => (None, None),
            };
            [a[0] + (t == Some(1)) as i64, a[1] + (t == Some(2)) as i64, a[2] + (x == Some(2)) as i64, a[3] + (x == Some(3)) as i64, a[4] + (c == Some(1)) as i64, a[5] + (c == Some(2)) as i64, a[6] + (c == Some(3)) as i64]
        });
    let dp = ud(db, UserWhere::All, posts_of(db));
    let cu = comments_per_user(db);
    let pf = stats_fold(db, db.post.with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cv", &[]);
    let mut v = Vec::new();
    (&pf)
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&uf).and((&dp).filt(|d: i64| d > 5)).and(&cu)))
        .drive(|p, (s, (((u, a), d), c))| v.push((p, s, u, a, d, c)));
    out(v, |&(_, _, _, a, d, _)| (Reverse(d), Reverse(a[2])), 100, |&(p, s, u, a, d, c)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(d)];
        f.extend(ints(&a));
        f.push(V::I(c));
        f.extend(post_fields(db, p, &["title", "views"]));
        f.extend(ints(&[s.cx, s.up, s.down]));
        f
    })
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// COUNT(c.Id) AS CommentCount,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) - SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS VoteScore,
// DATE_TRUNC('month', p.CreationDate) AS MonthYear
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, DATE_TRUNC('month', p.CreationDate)
// ),
// AverageStats AS (
// SELECT
// MonthYear,
// AVG(CommentCount) AS AvgComments,
// AVG(VoteScore) AS AvgVoteScore
// FROM
// PostStats
// GROUP BY
// MonthYear
// )
// SELECT
// MonthYear,
// AvgComments,
// AvgVoteScore
// FROM
// AverageStats
// ORDER BY
// MonthYear DESC;
fn q13336(db: &'static So) -> String {
    let f = stats_fold(db, db.post.iq(), Ident::<Post>::new(), "cv", &[]);
    let m = db.post.group_by((&db.post.creation_date).map(trunc_month)).select(&f).fold([0i64; 3], |a, s| [a[0] + 1, a[1] + s.cx, a[2] + s.up - s.down]);
    let mut v = Vec::new();
    (&m).drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&(k, a)| row(vec![V::T(k), avg(a[1], a[0]), avg(a[2], a[0])])))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("26257", q26257),
    ("25996", q25996),
    ("7535", q7535),
    ("8589", q8589),
    ("14102", q14102),
    ("8651", q8651),
    ("7612", q7612),
    ("11679", q11679),
    ("2940", q2940),
    ("14788", q14788),
    ("9528", q9528),
    ("318", q318),
    ("5277", q5277),
    ("14613", q14613),
    ("1859", q1859),
    ("6845", q6845),
    ("9540", q9540),
    ("7429", q7429),
    ("9951", q9951),
    ("26670", q26670),
    ("8377", q8377),
    ("2551", q2551),
    ("8954", q8954),
    ("3273", q3273),
    ("13381", q13381),
    ("7274", q7274),
    ("919", q919),
    ("29147", q29147),
    ("3691", q3691),
    ("9137", q9137),
    ("25579", q25579),
    ("26711", q26711),
    ("14624", q14624),
    ("27491", q27491),
    ("28598", q28598),
    ("34003", q34003),
    ("23214", q23214),
    ("10719", q10719),
    ("25661", q25661),
    ("9769", q9769),
    ("2661", q2661),
    ("28437", q28437),
    ("14062", q14062),
    ("28350", q28350),
    ("3609", q3609),
    ("27175", q27175),
    ("4004", q4004),
    ("8411", q8411),
    ("27713", q27713),
    ("25718", q25718),
    ("30824", q30824),
    ("6460", q6460),
    ("13336", q13336),
];
