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

fn onull_i(n: i64, x: i64) -> V {
    if n == 0 { V::Null } else { V::I(x) }
}

// --- batch 139 --------------------------------------------------------------

// WITH UserBadgeStats AS (
// SELECT
// U.Id AS UserId,
// COUNT(B.Id) AS TotalBadges,
// SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM Users U
// LEFT JOIN Badges B ON U.Id = B.UserId
// GROUP BY U.Id
// ),
// PostActivityStats AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// MAX(P.CreationDate) AS MostRecentPost
// FROM Posts P
// GROUP BY P.OwnerUserId
// ),
// ClosedPostReasons AS (
// SELECT
// PH.UserId,
// PH.Comment AS CloseReason,
// COUNT(PH.Id) AS CloseReasonCount
// FROM PostHistory PH
// WHERE PH.PostHistoryTypeId = 10
// GROUP BY PH.UserId, PH.Comment
// ),
// FinalStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COALESCE(UB.TotalPosts, 0) AS TotalPosts,
// COALESCE(UB.TotalQuestions, 0) AS TotalQuestions,
// COALESCE(UPB.TotalBadges, 0) AS TotalBadges,
// COALESCE(UPB.GoldBadges, 0) AS GoldBadges,
// COALESCE(UPB.SilverBadges, 0) AS SilverBadges,
// COALESCE(UPB.BronzeBadges, 0) AS BronzeBadges,
// COALESCE(CPR.CloseReasonCount, 0) AS CloseReasonCount
// FROM Users U
// LEFT JOIN PostActivityStats UB ON U.Id = UB.OwnerUserId
// LEFT JOIN UserBadgeStats UPB ON U.Id = UPB.UserId
// LEFT JOIN ClosedPostReasons CPR ON U.Id = CPR.UserId
// )
// SELECT
// F.UserId,
// F.DisplayName,
// F.TotalPosts,
// F.TotalQuestions,
// F.TotalBadges,
// F.GoldBadges,
// F.SilverBadges,
// F.BronzeBadges,
// F.CloseReasonCount,
// CASE
// WHEN F.TotalPosts > 100 THEN 'Elite User'
// WHEN F.TotalPosts BETWEEN 50 AND 100 THEN 'Active User'
// ELSE 'New User'
// END AS UserCategory,
// CASE
// WHEN F.CloseReasonCount > 5 THEN 'Frequent Closures'
// ELSE 'Rarely Closed'
// END AS ClosureFrequency
// FROM FinalStats F
// WHERE F.TotalPosts IS NOT NULL
// ORDER BY F.TotalPosts DESC
// LIMIT 100;
fn q20652(db: &'static So) -> String {
    let ps = owned(db).group_by(&db.post.owner_user).select(&db.post.post_type_id).fold([0i64; 2], |a, t| [a[0] + 1, a[1] + (t == 1) as i64]);
    let bc = badge_classes(db);
    let PostHistory { user, comment, post_history_type_id, .. } = &db.post_history;
    let ck: MatSet<(Id<User>, Option<Str>)> = db.post_history.with(post_history_type_id.eq(10)).with(user).select(user.and(comment.opt())).collect();
    let cf = db.post_history.with(post_history_type_id.eq(10)).group_by(user.and(comment.opt())).select(post_history_type_id).fold(0i64, |a, _| a + 1);
    let ci: HashIdx<Id<User>, (Id<User>, Option<Str>)> = (&ck).map(|(u, _)| u).inv().collect();
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&ps).opt()).and((&bc).opt()).and((&ci).select(&cf).opt())).drive(|_, (((u, p), b), c)| v.push((u, p.unwrap_or([0; 2]), bz(b), c.unwrap_or(0))));
    out(v, |&(u, p, _, c)| (Reverse(p[0]), db.user.origid.get(u).unwrap(), c), 100, |&(u, p, b, c)| {
        let cat = if p[0] > 100 { "Elite User" } else if (50..=100).contains(&p[0]) { "Active User" } else { "New User" };
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&[p[0], p[1]]));
        f.extend(ints(&b));
        f.extend([V::I(c), V::S(cat), V::S(if c > 5 { "Frequent Closures" } else { "Rarely Closed" })]);
        f
    })
}

// WITH UserActivity AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// COUNT(DISTINCT c.Id) AS TotalComments,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes,
// MAX(p.CreationDate) AS LastActivityDate
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// u.Reputation > 1000
// GROUP BY
// u.Id, u.DisplayName
// ),
// UserBadgeActivity AS (
// SELECT
// ub.UserId,
// COUNT(*) AS BadgeCount
// FROM
// Badges ub
// GROUP BY
// ub.UserId
// ),
// PostTypeCounts AS (
// SELECT
// p.OwnerUserId,
// COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS Questions,
// COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS Answers
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// ),
// FinalActivity AS (
// SELECT
// ua.DisplayName,
// ua.TotalPosts,
// ua.TotalComments,
// ua.QuestionsCount,
// ua.AnswersCount,
// ua.TotalUpVotes,
// ua.TotalDownVotes,
// COALESCE(uba.BadgeCount, 0) AS BadgeCount,
// pt.Questions,
// pt.Answers,
// MAX(ua.LastActivityDate) AS LastActive
// FROM
// UserActivity ua
// LEFT JOIN
// UserBadgeActivity uba ON ua.UserId = uba.UserId
// LEFT JOIN
// PostTypeCounts pt ON ua.UserId = pt.OwnerUserId
// GROUP BY
// ua.UserId, ua.DisplayName, ua.TotalPosts,
// ua.TotalComments, ua.QuestionsCount, ua.AnswersCount,
// ua.TotalUpVotes, ua.TotalDownVotes, uba.BadgeCount,
// pt.Questions, pt.Answers
// )
// SELECT
// DisplayName,
// TotalPosts,
// TotalComments,
// QuestionsCount,
// AnswersCount,
// TotalUpVotes,
// TotalDownVotes,
// BadgeCount,
// Questions,
// Answers,
// LastActive
// FROM
// FinalActivity
// ORDER BY
// TotalPosts DESC, TotalUpVotes DESC
// LIMIT 10;
fn q9141(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::RepGt(1000), "cv", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let dc = udc(db);
    let bu = badges_per_user(db);
    let pt = owned(db).group_by(&db.post.owner_user).select(&db.post.post_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64]);
    let mut v = Vec::new();
    (&us).and((&dp).opt()).and((&dc).opt()).and(&bu).and((&pt).opt()).drive(|u, ((((a, d), c), b), t)| v.push((u, a, d.unwrap_or(0), c.unwrap_or(0), b, t)));
    out(v, |&(_, a, d, ..)| (Reverse(d), Reverse(a.up)), 10, |&(u, a, d, c, b, t)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(ints(&[d, c, a.q, a.a, a.up, a.down, b]));
        f.extend(t.map_or(nulls(2), |t| ints(&t)));
        f.push(if a.n == 0 { V::Null } else { V::T(a.pmax) });
        f
    })
}

// WITH UserBadgeStats AS (
// SELECT
// u.Id AS UserId,
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
// u.Id, u.Reputation
// ),
// PostActivity AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts,
// SUM(CASE WHEN p.Score < 0 THEN 1 ELSE 0 END) AS NegativePosts,
// AVG(p.ViewCount) AS AvgViewCount
// FROM
// Posts p
// WHERE
// p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'
// GROUP BY
// p.OwnerUserId
// ),
// VoteDetails AS (
// SELECT
// v.UserId,
// COUNT(v.Id) AS TotalVotes,
// SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN vt.Name = 'DownMod' THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Votes v
// JOIN
// VoteTypes vt ON v.VoteTypeId = vt.Id
// GROUP BY
// v.UserId
// ),
// UserPerformance AS (
// SELECT
// ub.UserId,
// ub.Reputation,
// COALESCE(pa.PostCount, 0) AS PostCount,
// COALESCE(pa.PositivePosts, 0) AS PositivePosts,
// COALESCE(pa.NegativePosts, 0) AS NegativePosts,
// COALESCE(vd.TotalVotes, 0) AS TotalVotes,
// COALESCE(vd.UpVotes, 0) AS UpVotes,
// COALESCE(vd.DownVotes, 0) AS DownVotes,
// ub.BadgeCount,
// ub.GoldBadges,
// ub.SilverBadges,
// ub.BronzeBadges
// FROM
// UserBadgeStats ub
// LEFT JOIN
// PostActivity pa ON ub.UserId = pa.OwnerUserId
// LEFT JOIN
// VoteDetails vd ON ub.UserId = vd.UserId
// )
// SELECT
// u.UserId,
// u.Reputation,
// u.PostCount,
// u.PositivePosts,
// u.NegativePosts,
// u.TotalVotes,
// u.UpVotes,
// u.DownVotes,
// u.BadgeCount,
// u.GoldBadges,
// u.SilverBadges,
// u.BronzeBadges
// FROM
// UserPerformance u
// WHERE
// u.Reputation > (SELECT AVG(Reputation) FROM Users) AND
// u.TotalVotes > 0
// ORDER BY
// u.BadgeCount DESC, u.Reputation DESC
// LIMIT 10;
fn q20370(db: &'static So) -> String {
    let t = db.user.select(&db.user.reputation).fold_flat([0i64; 2], |a, r| [a[0] + 1, a[1] + r]);
    let ub = ubc(db);
    let pa = db.post.with((&db.post.creation_date).ge(date(2023, 10, 1))).group_by(&db.post.owner_user).select(&db.post.score).fold([0i64; 3], |a, s| [a[0] + 1, a[1] + (s > 0) as i64, a[2] + (s < 0) as i64]);
    let vn = vote_named(db);
    let mut v = Vec::new();
    db.user
        .with((&db.user.reputation).filt(move |r: i64| (r as i128) * (t[0] as i128) > t[1] as i128))
        .select(Ident::<User>::new().and(&ub).and((&pa).opt()).and(&vn))
        .drive(|_, (((u, b), p), x)| v.push((u, b, p.unwrap_or([0; 3]), x)));
    out(v, |&(u, b, ..)| (Reverse(b[0]), rep_desc(db, u)), 10, |&(u, b, p, x)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep")];
        f.extend(ints(&p));
        f.extend(ints(&x));
        f.extend(ints(&b));
        f
    })
}

// WITH UserReputation AS (
// SELECT
// Id,
// Reputation,
// CASE
// WHEN Reputation > 1000 THEN 'High'
// WHEN Reputation BETWEEN 500 AND 1000 THEN 'Medium'
// ELSE 'Low'
// END AS ReputationLevel
// FROM Users
// ),
// PostVoteStats AS (
// SELECT
// P.Id AS PostID,
// COUNT(V.Id) AS TotalVotes,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
// FROM Posts P
// LEFT JOIN Votes V ON P.Id = V.PostId
// GROUP BY P.Id
// ),
// PostHistoryAggregates AS (
// SELECT
// PH.PostId,
// COUNT(PH.Id) AS EditCount,
// COUNT(DISTINCT PH.UserId) AS UniqueEditors,
// MAX(PH.CreationDate) FILTER (WHERE PH.PostHistoryTypeId = 24) AS LastEditDate,
// MIN(PH.CreationDate) FILTER (WHERE PH.PostHistoryTypeId = 10) AS ClosedDate
// FROM PostHistory PH
// GROUP BY PH.PostId
// ),
// TopPosts AS (
// SELECT
// P.Id AS PostID,
// P.Score,
// PH.EditCount,
// U.ReputationLevel
// FROM Posts P
// JOIN PostHistoryAggregates PH ON P.Id = PH.PostId
// JOIN UserReputation U ON P.OwnerUserId = U.Id
// WHERE P.PostTypeId = 1
// AND P.Score > 0
// ),
// FinalResults AS (
// SELECT
// T.PostID,
// T.Score,
// PH.TotalVotes,
// PH.Upvotes,
// PH.Downvotes,
// T.EditCount,
// T.ReputationLevel,
// CASE
// WHEN T.EditCount > 2 THEN 'Frequently Updated'
// ELSE 'Rarely Updated'
// END AS UpdateFrequency,
// CASE
// WHEN PH.TotalVotes IS NULL THEN 'No Votes'
// ELSE CASE
// WHEN PH.Upvotes > PH.Downvotes THEN 'Positive Reception'
// WHEN PH.Upvotes < PH.Downvotes THEN 'Negative Reception'
// ELSE 'Neutral Reception'
// END
// END AS VoteReception
// FROM TopPosts T
// LEFT JOIN PostVoteStats PH ON T.PostID = PH.PostID
// )
// SELECT
// F.PostID,
// F.Score,
// F.TotalVotes,
// F.Upvotes,
// F.Downvotes,
// F.EditCount,
// F.ReputationLevel,
// F.UpdateFrequency,
// F.VoteReception,
// CASE WHEN F.Score IS NULL THEN 'Unknown Score' ELSE 'Score Available' END AS ScoreStatus
// FROM FinalResults F
// ORDER BY F.Score DESC NULLS LAST, F.TotalVotes DESC;
fn q24499(db: &'static So) -> String {
    let hp = history_n_max(db);
    let pv = post_votes(db);
    let mut v = Vec::new();
    questions_only(db)
        .with((&db.post.score).gt(0))
        .with(&db.post.owner_user)
        .select(Ident::<Post>::new().and((&hp).map(|h: (i64, i64)| h.0)).and((&pv).opt()))
        .drive(|_, x| v.push(x));
    rows(v.iter().map(|&((p, h), x)| {
        let x = x.unwrap_or([0; 3]);
        let r = db.user.reputation.get(db.post.owner_user.get(p).unwrap()).unwrap();
        let lvl = if r > 1000 { "High" } else if (500..=1000).contains(&r) { "Medium" } else { "Low" };
        let rec = if x[1] > x[2] { "Positive Reception" } else if x[1] < x[2] { "Negative Reception" } else { "Neutral Reception" };
        let mut f = post_fields(db, p, &["id", "score"]);
        f.extend(ints(&[x[0], x[1], x[2], h]));
        f.extend([V::S(lvl), V::S(if h > 2 { "Frequently Updated" } else { "Rarely Updated" }), V::S(rec), V::S("Score Available")]);
        row(f)
    }))
}

// WITH RecursiveUserPosts AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS PostCount
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
// UserActivity AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COALESCE(rb.GoldBadges, 0) AS GoldBadges,
// COALESCE(rb.SilverBadges, 0) AS SilverBadges,
// COALESCE(rb.BronzeBadges, 0) AS BronzeBadges,
// SUM(COALESCE(v.VoteCount, 0)) AS TotalVotes,
// SUM(COALESCE(p.ViewCount, 0)) AS TotalViews,
// SUM(CASE WHEN p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year' AND p.PostTypeId = 1 THEN 1 ELSE 0 END) AS RecentQuestions,
// r.PostCount
// FROM
// Users u
// LEFT JOIN
// UserBadges rb ON u.Id = rb.UserId
// LEFT JOIN (
// SELECT
// PostOwnerId,
// COUNT(*) AS VoteCount
// FROM
// (SELECT
// v.UserId AS PostOwnerId,
// v.Id
// FROM
// Votes v
// JOIN
// Posts p ON v.PostId = p.Id
// WHERE
// v.VoteTypeId IN (2, 3)
// ) AS votes_by_user
// GROUP BY
// PostOwnerId
// ) v ON u.Id = v.PostOwnerId
// LEFT JOIN
// RecursiveUserPosts r ON u.Id = r.UserId
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName, rb.GoldBadges, rb.SilverBadges, rb.BronzeBadges, r.PostCount
// )
// SELECT
// ua.UserId,
// ua.DisplayName,
// ua.GoldBadges,
// ua.SilverBadges,
// ua.BronzeBadges,
// ua.TotalVotes,
// ua.TotalViews,
// ua.RecentQuestions,
// ua.PostCount,
// CASE
// WHEN ua.TotalVotes > 100 THEN 'Active'
// WHEN ua.RecentQuestions > 10 THEN 'Engaged'
// ELSE 'Casual User'
// END AS UserEngagementLevel
// FROM
// UserActivity ua
// WHERE
// ua.PostCount > 0
// ORDER BY
// ua.TotalVotes DESC,
// ua.TotalViews DESC
// LIMIT 100;
fn q30104(db: &'static So) -> String {
    let bc = badge_classes(db);
    let vc = db.vote.with((&db.vote.vote_type_id).filt(|t: i64| matches!(t, 2 | 3))).with(&db.vote.post).group_by(&db.vote.user).select(&db.vote.vote_type_id).fold(0i64, |a, _| a + 1);
    let d0 = date(2023, 10, 1);
    let Post { view_count, creation_date, post_type_id, .. } = &db.post;
    let pf = owned(db)
        .group_by(&db.post.owner_user)
        .select(view_count.opt().and(creation_date).and(post_type_id).and((&db.post.owner_user).select(&vc).opt()))
        .fold([0i64; 4], move |a, (((w, c), t), x)| [a[0] + 1, a[1] + w.unwrap_or(0), a[2] + (c >= d0 && t == 1) as i64, a[3] + x.unwrap_or(0)]);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&bc).opt()).and(&pf)).drive(|_, ((u, b), p)| v.push((u, bz(b), p[3], p)));
    out(v, |&(_, _, x, p)| (Reverse(x), Reverse(p[1])), 100, |&(u, b, x, p)| {
        let lvl = if x > 100 { "Active" } else if p[2] > 10 { "Engaged" } else { "Casual User" };
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&[b[1], b[2], b[3], x, p[1], p[2], p[0]]));
        f.push(V::S(lvl));
        f
    })
}

// WITH UserBadges AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(B.Id) FILTER (WHERE B.Class = 1) AS GoldBadges,
// COUNT(B.Id) FILTER (WHERE B.Class = 2) AS SilverBadges,
// COUNT(B.Id) FILTER (WHERE B.Class = 3) AS BronzeBadges
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
// COUNT(DISTINCT P.Id) AS TotalPosts,
// COUNT(DISTINCT CASE WHEN P.PostTypeId = 1 THEN P.Id END) AS TotalQuestions,
// COUNT(DISTINCT CASE WHEN P.PostTypeId = 2 THEN P.Id END) AS TotalAnswers,
// SUM(P.Score) AS TotalScore,
// SUM(P.ViewCount) AS TotalViews,
// AVG(P.ViewCount) AS AvgPostViews
// FROM
// Posts P
// GROUP BY
// P.OwnerUserId
// ),
// ClosedPostStats AS (
// SELECT
// PH.UserId,
// COUNT(P.Id) AS ClosedPosts
// FROM
// PostHistory PH
// JOIN
// Posts P ON PH.PostId = P.Id
// WHERE
// PH.PostHistoryTypeId = 10
// GROUP BY
// PH.UserId
// ),
// UserActivity AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COALESCE(UB.GoldBadges, 0) AS GoldBadges,
// COALESCE(UB.SilverBadges, 0) AS SilverBadges,
// COALESCE(UB.BronzeBadges, 0) AS BronzeBadges,
// COALESCE(PS.TotalPosts, 0) AS TotalPosts,
// COALESCE(PS.TotalQuestions, 0) AS TotalQuestions,
// COALESCE(PS.TotalAnswers, 0) AS TotalAnswers,
// COALESCE(PS.TotalScore, 0) AS TotalScore,
// COALESCE(PS.TotalViews, 0) AS TotalViews,
// COALESCE(PS.AvgPostViews, 0) AS AvgPostViews,
// COALESCE(CPS.ClosedPosts, 0) AS ClosedPosts
// FROM
// Users U
// LEFT JOIN
// UserBadges UB ON U.Id = UB.UserId
// LEFT JOIN
// PostStatistics PS ON U.Id = PS.OwnerUserId
// LEFT JOIN
// ClosedPostStats CPS ON U.Id = CPS.UserId
// )
// SELECT
// UserId,
// DisplayName,
// GoldBadges,
// SilverBadges,
// BronzeBadges,
// TotalPosts,
// TotalQuestions,
// TotalAnswers,
// TotalScore,
// TotalViews,
// AvgPostViews,
// CASE
// WHEN ClosedPosts > 0 THEN 'Yes'
// ELSE 'No'
// END AS HasClosedPosts
// FROM
// UserActivity
// WHERE
// TotalPosts > 5
// ORDER BY
// TotalScore DESC, TotalPosts DESC;
fn q1673(db: &'static So) -> String {
    let bc = badge_classes(db);
    let ps = pstat(db, db.post.iq());
    let cp = db.post_history.with((&db.post_history.post_history_type_id).eq(10)).group_by(&db.post_history.user).select(&db.post_history.post).fold(0i64, |a, _| a + 1);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&bc).opt()).and((&ps).filt(|p: [i64; 13]| p[0] > 5)).and((&cp).opt())).drive(|_, (((u, b), p), c)| v.push((u, bz(b), p, c.unwrap_or(0))));
    rows(v.iter().map(|&(u, b, p, c)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&[b[1], b[2], b[3], p[0], p[1], p[2], p[3], p[5]]));
        f.extend([or0(p[5], p[4]), V::S(if c > 0 { "Yes" } else { "No" })]);
        row(f)
    }))
}

// WITH UserStatistics AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// u.Views,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// u.Id, u.Reputation, u.Views
// ),
// PostActivity AS (
// SELECT
// p.Id AS PostId,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// COALESCE(c.CommentCount, 0) AS CommentCount,
// COALESCE(ph.EditCount, 0) AS EditCount
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
// COUNT(*) AS EditCount
// FROM
// PostHistory
// WHERE
// PostHistoryTypeId IN (4, 5, 6)
// GROUP BY
// PostId
// ) ph ON p.Id = ph.PostId
// ),
// VoteStatistics AS (
// SELECT
// PostId,
// COUNT(CASE WHEN VoteTypeId = 2 THEN 1 END) AS UpModCount,
// COUNT(CASE WHEN VoteTypeId = 3 THEN 1 END) AS DownModCount
// FROM
// Votes
// GROUP BY
// PostId
// )
// SELECT
// us.UserId,
// us.Reputation,
// us.Views,
// us.PostCount,
// us.QuestionCount,
// us.AnswerCount,
// us.UpVoteCount,
// us.DownVoteCount,
// SUM(pa.Score) AS TotalPostScore,
// SUM(pa.ViewCount) AS TotalPostViews,
// SUM(pa.CommentCount) AS TotalComments,
// SUM(pa.EditCount) AS TotalEdits,
// SUM(vs.UpModCount) AS TotalUpVotes,
// SUM(vs.DownModCount) AS TotalDownVotes
// FROM
// UserStatistics us
// JOIN
// Posts p ON us.UserId = p.OwnerUserId
// JOIN
// PostActivity pa ON p.Id = pa.PostId
// JOIN
// VoteStatistics vs ON p.Id = vs.PostId
// GROUP BY
// us.UserId, us.Reputation, us.Views, us.PostCount, us.QuestionCount, us.AnswerCount, us.UpVoteCount, us.DownVoteCount
// ORDER BY
// TotalPostScore DESC, TotalPostViews DESC;
fn q11874(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let pv = post_votes(db);
    let cp = comments_per_post(db);
    let ec = history_of_types(db, &[4, 5, 6]);
    let Post { score, view_count, .. } = &db.post;
    let pf = owned(db).with(&pv).group_by(&db.post.owner_user).select(score.and(view_count.opt()).and(&cp).and(&ec).and(&pv)).fold([0i64; 7], |a, ((((s, w), c), e), x)| {
        [a[0] + s, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + c, a[4] + e, a[5] + x[1], a[6] + x[2]]
    });
    let mut v = Vec::new();
    (&pf).and(&us).and((&dp).opt()).drive(|u, ((p, a), d)| v.push((u, p, a, d.unwrap_or(0))));
    rows(v.iter().map(|&(u, p, a, d)| {
        let mut f = ["uid", "rep", "uviews"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.extend(ints(&[d, a.q, a.a, a.up, a.down, p[0]]));
        f.push(nullable(p[2], p[1]));
        f.extend(ints(&[p[3], p[4], p[5], p[6]]));
        row(f)
    }))
}

// WITH UserPostStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// U.Views,
// COALESCE(SUM(CASE WHEN P.Score > 0 THEN P.Score ELSE 0 END), 0) AS TotalPositiveScore,
// COALESCE(SUM(CASE WHEN P.Score < 0 THEN P.Score ELSE 0 END), 0) AS TotalNegativeScore,
// COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS QuestionCount,
// COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS AnswerCount,
// COUNT(DISTINCT P.Id) AS TotalPosts
// FROM Users U
// LEFT JOIN Posts P ON U.Id = P.OwnerUserId
// GROUP BY U.Id, U.DisplayName, U.Reputation, U.Views
// ),
// BadgesStats AS (
// SELECT
// B.UserId,
// COUNT(CASE WHEN B.Class = 1 THEN 1 END) AS GoldBadges,
// COUNT(CASE WHEN B.Class = 2 THEN 1 END) AS SilverBadges,
// COUNT(CASE WHEN B.Class = 3 THEN 1 END) AS BronzeBadges
// FROM Badges B
// GROUP BY B.UserId
// ),
// PostHistories AS (
// SELECT
// PH.UserId,
// PH.PostId,
// PH.PostHistoryTypeId,
// COUNT(*) AS EditCount
// FROM PostHistory PH
// WHERE PH.PostHistoryTypeId IN (4, 5, 6, 24)
// GROUP BY PH.UserId, PH.PostId, PH.PostHistoryTypeId
// ),
// UserActivity AS (
// SELECT
// U.UserId,
// COUNT(DISTINCT C.Id) AS CommentCount,
// COUNT(DISTINCT V.Id) AS VoteCount
// FROM (
// SELECT U.Id AS UserId
// FROM Users U
// WHERE U.Reputation > 1000
// ) AS U
// LEFT JOIN Comments C ON C.UserId = U.UserId
// LEFT JOIN Votes V ON V.UserId = U.UserId
// GROUP BY U.UserId
// )
// SELECT
// UPS.UserId,
// UPS.DisplayName,
// UPS.Reputation,
// UPS.Views,
// UPS.TotalPositiveScore,
// UPS.TotalNegativeScore,
// UPS.QuestionCount,
// UPS.AnswerCount,
// COALESCE(BS.GoldBadges, 0) AS GoldBadges,
// COALESCE(BS.SilverBadges, 0) AS SilverBadges,
// COALESCE(BS.BronzeBadges, 0) AS BronzeBadges,
// COALESCE(PH.EditCount, 0) AS TotalEdits,
// COALESCE(A.CommentCount, 0) AS TotalComments,
// COALESCE(A.VoteCount, 0) AS TotalVotes
// FROM UserPostStats UPS
// LEFT JOIN BadgesStats BS ON UPS.UserId = BS.UserId
// LEFT JOIN (
// SELECT UserId, SUM(EditCount) AS EditCount
// FROM PostHistories
// GROUP BY UserId
// ) PH ON UPS.UserId = PH.UserId
// LEFT JOIN UserActivity A ON UPS.UserId = A.UserId
// WHERE UPS.TotalPosts > 10
// ORDER BY UPS.Reputation DESC, UPS.DisplayName ASC;
fn q22390(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let uf = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(score)).fold([0i64; 5], |a, (t, s)| {
        [a[0] + if s > 0 { s } else { 0 }, a[1] + if s < 0 { s } else { 0 }, a[2] + (t == 1) as i64, a[3] + (t == 2) as i64, a[4] + 1]
    });
    let bc = badge_classes(db);
    let ed = db.post_history.with((&db.post_history.post_history_type_id).filt(|t: i64| matches!(t, 4 | 5 | 6 | 24))).group_by(&db.post_history.user).select(&db.post_history.post).fold(0i64, |a, _| a + 1);
    let cu = comments_per_user(db);
    let vu = votes_per_user(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&uf).filt(|a: [i64; 5]| a[4] > 10)).and((&bc).opt()).and((&ed).opt()).and(&cu).and(&vu)).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(((((u, a), b), e), c), x)| {
        let b = bz(b);
        let hi = db.user.reputation.get(u).unwrap() > 1000;
        let mut f = ["uid", "name", "rep", "uviews"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.extend(ints(&[a[0], a[1], a[2], a[3], b[1], b[2], b[3], e.unwrap_or(0), if hi { c } else { 0 }, if hi { x } else { 0 }]));
        row(f)
    }))
}

// WITH UserVotes AS (
// SELECT
// v.UserId,
// COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS Upvotes,
// COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS Downvotes,
// COUNT(CASE WHEN v.VoteTypeId IN (2, 3) THEN 1 END) AS TotalVotes
// FROM
// Votes v
// GROUP BY
// v.UserId
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
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(COALESCE(p.Score, 0)) AS TotalScore,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// ),
// CombinedStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// u.Views,
// COALESCE(uv.Upvotes, 0) AS Upvotes,
// COALESCE(uv.Downvotes, 0) AS Downvotes,
// COALESCE(uv.TotalVotes, 0) AS TotalVotes,
// COALESCE(ub.GoldBadges, 0) AS GoldBadges,
// COALESCE(ub.SilverBadges, 0) AS SilverBadges,
// COALESCE(ub.BronzeBadges, 0) AS BronzeBadges,
// COALESCE(ps.PostCount, 0) AS PostCount,
// COALESCE(ps.TotalScore, 0) AS TotalScore,
// COALESCE(ps.QuestionCount, 0) AS QuestionCount,
// COALESCE(ps.AnswerCount, 0) AS AnswerCount
// FROM
// Users u
// LEFT JOIN
// UserVotes uv ON u.Id = uv.UserId
// LEFT JOIN
// UserBadges ub ON u.Id = ub.UserId
// LEFT JOIN
// PostStatistics ps ON u.Id = ps.OwnerUserId
// )
// SELECT
// c.UserId,
// CONCAT(c.DisplayName, ' (Rep: ', c.Reputation, ')') AS UserDisplay,
// c.PostCount,
// c.TotalScore,
// c.Upvotes,
// c.Downvotes,
// c.QuestionCount,
// c.AnswerCount,
// c.GoldBadges + c.SilverBadges + c.BronzeBadges AS TotalBadges,
// CASE
// WHEN c.PostCount > 50 THEN 'Highly Active'
// WHEN c.PostCount BETWEEN 20 AND 50 THEN 'Moderately Active'
// ELSE 'Less Active'
// END AS ActivityLevel
// FROM
// CombinedStats c
// ORDER BY
// c.TotalScore DESC,
// c.Upvotes DESC
// LIMIT 10;
fn q31268(db: &'static So) -> String {
    let uv = db.vote.group_by(&db.vote.user).select(&db.vote.vote_type_id).fold([0i64; 3], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + matches!(t, 2 | 3) as i64]);
    let bc = badge_classes(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&uv).opt()).and((&bc).opt()).and((&ps).opt())).drive(|_, (((u, x), b), p)| v.push((u, x.unwrap_or([0; 3]), bz(b), pz(p))));
    out(v, |&(_, x, _, p)| (Reverse(p[3]), Reverse(x[0])), 10, |&(u, x, b, p)| {
        let lvl = if p[0] > 50 { "Highly Active" } else if (20..=50).contains(&p[0]) { "Moderately Active" } else { "Less Active" };
        vec![
            user_col(db, u, "uid"),
            V::Owned(format!("{} (Rep: {})", db.user.display_name.get(u).unwrap(), db.user.reputation.get(u).unwrap())),
            V::I(p[0]),
            V::I(p[3]),
            V::I(x[0]),
            V::I(x[1]),
            V::I(p[1]),
            V::I(p[2]),
            V::I(b[1] + b[2] + b[3]),
            V::S(lvl),
        ]
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
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// GROUP BY
// U.Id, U.DisplayName, U.Reputation
// ),
// PostStats AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.ViewCount,
// P.Score,
// COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
// COUNT(CASE WHEN PH.Id IS NOT NULL THEN 1 END) AS HistoryCount
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// PostHistory PH ON P.Id = PH.PostId
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.ViewCount, P.Score
// ),
// TopUsers AS (
// SELECT
// UserId,
// DisplayName,
// Reputation,
// PostCount,
// QuestionCount,
// AnswerCount,
// UpVotes,
// DownVotes,
// GoldBadges,
// SilverBadges,
// BronzeBadges
// FROM
// UserStats
// WHERE
// Reputation > 1000
// ORDER BY
// Reputation DESC
// LIMIT 10
// ),
// TopPosts AS (
// SELECT
// PostId,
// Title,
// CreationDate,
// ViewCount,
// Score,
// CommentCount,
// HistoryCount
// FROM
// PostStats
// ORDER BY
// Score DESC, ViewCount DESC
// LIMIT 5
// )
// SELECT
// U.UserId,
// U.DisplayName,
// U.Reputation,
// P.PostId,
// P.Title AS PostTitle,
// P.CreationDate AS PostCreationDate,
// P.ViewCount AS PostViewCount,
// P.Score AS PostScore,
// P.CommentCount AS PostCommentCount,
// P.HistoryCount AS PostHistoryCount
// FROM
// TopUsers U
// JOIN
// TopPosts P ON U.UserId = P.PostId
// ORDER BY
// U.Reputation DESC, P.Score DESC;
fn q7651(db: &'static So) -> String {
    let pid = pids(db);
    let base = user_base(db, UserWhere::RepGt(1000));
    let tu: MatSet<Id<User>> = whole(&base).select(Ident::<User>::new().and(&db.user.reputation)).window(row_number, |(_, r): (Id<User>, i64)| r, desc).filt(|(_, n)| n <= 10).map(|((u, _), _)| u).collect();
    let tp: MatSet<Id<Post>> = whole(&db.post.id)
        .select(Ident::<Post>::new().and(&db.post.score).and((&db.post.view_count).opt()))
        .window(row_number, |((_, s), w): ((Id<Post>, i64), Option<i64>)| (s, w.is_some(), w), desc)
        .filt(|(_, n)| n <= 5)
        .map(|(((p, _), _), _)| p)
        .collect();
    let sf = stats_fold(db, (&tp).iq(), Ident::<Post>::new(), "ch", &[]);
    let mut v = Vec::new();
    (&tu).select(Ident::<User>::new().and((&db.user.origid).select(&pid).with(&tp).select(Ident::<Post>::new().and(&sf)))).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(u, (p, s))| {
        let mut f = ["uid", "name", "rep"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.extend(post_fields(db, p, &["id", "title", "created", "views", "score"]));
        f.extend(ints(&[s.cx, s.hx]));
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
// COUNT(P.Id) AS TotalPosts,
// COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS TotalQuestions,
// COUNT(CASE WHEN P.PostTypeId = 2 AND P.AcceptedAnswerId IS NOT NULL THEN 1 END) AS AcceptedAnswers,
// SUM(P.Score) AS TotalScore,
// AVG(P.ViewCount) AS AverageViews
// FROM
// Posts P
// GROUP BY
// P.OwnerUserId
// ),
// UserVoting AS (
// SELECT
// V.UserId,
// COUNT(V.Id) AS TotalVotes,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Votes V
// GROUP BY
// V.UserId
// ),
// FinalStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COALESCE(UB.BadgeCount, 0) AS BadgeCount,
// COALESCE(PS.TotalPosts, 0) AS TotalPosts,
// COALESCE(PS.TotalQuestions, 0) AS TotalQuestions,
// COALESCE(PS.AcceptedAnswers, 0) AS AcceptedAnswers,
// COALESCE(PS.TotalScore, 0) AS TotalScore,
// COALESCE(PS.AverageViews, 0) AS AverageViews,
// COALESCE(UV.TotalVotes, 0) AS TotalVotes,
// COALESCE(UV.UpVotes, 0) AS UpVotes,
// COALESCE(UV.DownVotes, 0) AS DownVotes
// FROM
// Users U
// LEFT JOIN UserBadges UB ON U.Id = UB.UserId
// LEFT JOIN PostStats PS ON U.Id = PS.OwnerUserId
// LEFT JOIN UserVoting UV ON U.Id = UV.UserId
// )
// SELECT
// UserId,
// DisplayName,
// BadgeCount,
// TotalPosts,
// TotalQuestions,
// AcceptedAnswers,
// TotalScore,
// AverageViews,
// TotalVotes,
// UpVotes,
// DownVotes,
// CASE
// WHEN TotalPosts = 0 THEN 'No posts'
// WHEN TotalQuestions = 0 THEN 'No questions'
// WHEN AcceptedAnswers > 0 THEN 'Active contributor'
// ELSE 'Lurker'
// END AS UserActivityStatus
// FROM
// FinalStats
// WHERE
// BadgeCount > 0 OR TotalPosts > 5
// ORDER BY
// TotalScore DESC, BadgeCount DESC, TotalPosts DESC;
fn q21533(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let Post { post_type_id, accepted_answer_id, score, view_count, .. } = &db.post;
    let ps = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(accepted_answer_id.opt()).and(score).and(view_count.opt())).fold([0i64; 6], |a, (((t, acc), s), w)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2 && acc.is_some()) as i64, a[3] + s, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0)]
    });
    let uv = uvotes(db);
    let mut v = Vec::new();
    db.user
        .select(Ident::<User>::new().and(&bu).and((&ps).opt()).and((&uv).opt()))
        .filt(|(((_, b), p), _): (((Id<User>, i64), Option<[i64; 6]>), Option<[i64; 3]>)| b > 0 || p.map_or(0, |p| p[0]) > 5)
        .drive(|_, (((u, b), p), x)| v.push((u, b, p.unwrap_or([0; 6]), x.unwrap_or([0; 3]))));
    rows(v.iter().map(|&(u, b, p, x)| {
        let st = if p[0] == 0 { "No posts" } else if p[1] == 0 { "No questions" } else if p[2] > 0 { "Active contributor" } else { "Lurker" };
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&[b, p[0], p[1], p[2], p[3]]));
        f.push(or0(p[5], p[4]));
        f.extend(ints(&x));
        f.push(V::S(st));
        row(f)
    }))
}

// WITH CTE_UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS TotalQuestions,
// COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS TotalAnswers,
// SUM(COALESCE(p.Score, 0)) AS TotalScore,
// SUM(COALESCE(c.Score, 0)) AS TotalCommentScore
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
// ),
// CTE_BadgeStats AS (
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
// CTE_PostHistory AS (
// SELECT
// p.OwnerUserId,
// COUNT(ph.Id) AS TotalEdits,
// COUNT(DISTINCT ph.PostId) AS TotalEditedPosts,
// MAX(ph.CreationDate) AS LastEditDate
// FROM
// PostHistory ph
// JOIN
// Posts p ON ph.PostId = p.Id
// GROUP BY
// p.OwnerUserId
// ),
// CTE_FinalStats AS (
// SELECT
// us.UserId,
// us.DisplayName,
// us.Reputation,
// us.TotalPosts,
// us.TotalQuestions,
// us.TotalAnswers,
// us.TotalScore,
// us.TotalCommentScore,
// COALESCE(bs.TotalBadges, 0) AS TotalBadges,
// COALESCE(bs.GoldBadges, 0) AS GoldBadges,
// COALESCE(bs.SilverBadges, 0) AS SilverBadges,
// COALESCE(bs.BronzeBadges, 0) AS BronzeBadges,
// COALESCE(ph.TotalEdits, 0) AS TotalEdits,
// COALESCE(ph.TotalEditedPosts, 0) AS TotalEditedPosts,
// ph.LastEditDate
// FROM
// CTE_UserStats us
// LEFT JOIN
// CTE_BadgeStats bs ON us.UserId = bs.UserId
// LEFT JOIN
// CTE_PostHistory ph ON us.UserId = ph.OwnerUserId
// )
// SELECT
// UserId,
// DisplayName,
// Reputation,
// TotalPosts,
// TotalQuestions,
// TotalAnswers,
// TotalScore,
// TotalCommentScore,
// TotalBadges,
// GoldBadges,
// SilverBadges,
// BronzeBadges,
// TotalEdits,
// TotalEditedPosts,
// LastEditDate
// FROM
// CTE_FinalStats
// WHERE
// TotalPosts > 10
// ORDER BY
// TotalScore DESC, Reputation DESC;
fn q5786(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "c", any_post);
    let ps = pstat(db, db.post.iq());
    let bc = badge_classes(db);
    let PostHistory { post, creation_date, .. } = &db.post_history;
    let ph = db.post_history.group_by(post.select(&db.post.owner_user)).select(creation_date).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let pd = db.post_history.group_by(post.select(&db.post.owner_user)).select(post).count_distinct();
    let mut v = Vec::new();
    (&us).and((&ps).filt(|p: [i64; 13]| p[0] > 10)).and((&bc).opt()).and((&ph).opt()).and((&pd).opt()).drive(|u, ((((a, p), b), h), d)| v.push((u, a, p, bz(b), h, d.unwrap_or(0))));
    rows(v.iter().map(|&(u, a, p, b, h, d)| {
        let mut f = ["uid", "name", "rep"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.extend(ints(&[p[0], p[1], p[2], a.score_sum, a.cscore]));
        f.extend(ints(&b));
        f.extend([V::I(h.map_or(0, |h| h.0)), V::I(d), ots(h.map(|h| h.1))]);
        row(f)
    }))
}

// WITH UserStatistics AS (
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
// PostMetrics AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS PostCount,
// COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS QuestionCount,
// COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS AnswerCount,
// AVG(p.ViewCount) AS AvgViewCount,
// SUM(p.Score) AS TotalScore
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// ),
// ClosedPosts AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS ClosedCount
// FROM
// Posts p
// WHERE
// p.ClosedDate IS NOT NULL
// GROUP BY
// p.OwnerUserId
// ),
// CombinedStatistics AS (
// SELECT
// u.UserId,
// u.DisplayName,
// u.Reputation,
// COALESCE(pm.PostCount, 0) AS TotalPosts,
// COALESCE(pm.QuestionCount, 0) AS TotalQuestions,
// COALESCE(pm.AnswerCount, 0) AS TotalAnswers,
// COALESCE(pm.AvgViewCount, 0) AS AverageViewCount,
// COALESCE(pm.TotalScore, 0) AS TotalScore,
// COALESCE(cp.ClosedCount, 0) AS TotalClosedPosts,
// u.BadgeCount,
// u.GoldBadges,
// u.SilverBadges,
// u.BronzeBadges
// FROM
// UserStatistics u
// LEFT JOIN
// PostMetrics pm ON u.UserId = pm.OwnerUserId
// LEFT JOIN
// ClosedPosts cp ON u.UserId = cp.OwnerUserId
// )
// SELECT
// *,
// CASE
// WHEN TotalPosts = 0 THEN 'No Posts'
// ELSE CONCAT('Active: ', TotalPosts, ' | Questions: ', TotalQuestions, ' | Answers: ', TotalAnswers)
// END AS ActivitySummary,
// CASE
// WHEN Reputation < 100 THEN 'Newbie'
// WHEN Reputation BETWEEN 100 AND 1000 THEN 'Intermediate'
// ELSE 'Expert'
// END AS UserLevel,
// CASE
// WHEN BadgeCount IS NULL THEN 'No Badges'
// ELSE CONCAT('Badges - Gold: ', GoldBadges, ', Silver: ', SilverBadges, ', Bronze: ', BronzeBadges)
// END AS BadgeSummary
// FROM
// CombinedStatistics
// WHERE
// Reputation > 50
// ORDER BY
// Reputation DESC, TotalScore DESC;
fn q20237(db: &'static So) -> String {
    let bc = badge_classes(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(50)).select(Ident::<User>::new().and((&bc).opt()).and((&ps).opt())).drive(|_, ((u, b), p)| v.push((u, bz(b), pz(p))));
    rows(v.iter().map(|&(u, b, p)| {
        let r = db.user.reputation.get(u).unwrap();
        let mut f = ["uid", "name", "rep"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.extend(ints(&[p[0], p[1], p[2]]));
        f.extend([or0(p[5], p[4]), V::I(p[3]), V::I(p[11])]);
        f.extend(ints(&b));
        f.push(if p[0] == 0 { V::S("No Posts") } else { V::Owned(format!("Active: {} | Questions: {} | Answers: {}", p[0], p[1], p[2])) });
        f.push(V::S(if r < 100 { "Newbie" } else if r <= 1000 { "Intermediate" } else { "Expert" }));
        f.push(V::Owned(format!("Badges - Gold: {}, Silver: {}, Bronze: {}", b[1], b[2], b[3])));
        row(f)
    }))
}

// WITH UserActivity AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount,
// MAX(p.CreationDate) AS LastActiveDate
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
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
// PostSummaries AS (
// SELECT
// DISTINCT p.Id AS PostId,
// p.Title,
// p.CreationDate AS PostCreationDate,
// COALESCE(SUM(c.CommentCount), 0) AS CommentCount,
// COALESCE(SUM(v.UpVoteCount), 0) AS TotalUpVotes,
// COALESCE(SUM(v.DownVoteCount), 0) AS TotalDownVotes
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
// SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
// FROM
// Votes
// GROUP BY
// PostId
// ) v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate
// )
// SELECT
// ua.DisplayName,
// ua.Reputation,
// uBad.GoldBadges,
// uBad.SilverBadges,
// uBad.BronzeBadges,
// ua.PostCount,
// ua.QuestionCount,
// ua.AnswerCount,
// ua.UpvoteCount,
// ua.DownvoteCount,
// ua.LastActiveDate,
// ps.Title AS PostTitle,
// ps.PostCreationDate,
// ps.CommentCount,
// ps.TotalUpVotes,
// ps.TotalDownVotes
// FROM
// UserActivity ua
// LEFT JOIN
// UserBadges uBad ON ua.UserId = uBad.UserId
// LEFT JOIN
// PostSummaries ps ON ua.UserId = ps.PostId
// WHERE
// ua.Reputation > 1000
// ORDER BY
// ua.Reputation DESC,
// ua.LastActiveDate DESC
// LIMIT 100;
fn q6652(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::RepGt(1000), "v", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let bc = badge_classes(db);
    let pid = pids(db);
    let pv = post_votes(db);
    let cp = comments_per_post(db);
    let mut v = Vec::new();
    (&us).and((&dp).opt()).and((&bc).opt()).and((&db.user.origid).select(&pid).select(Ident::<Post>::new().and(&cp).and((&pv).opt())).opt()).drive(|u, (((a, d), b), p)| v.push((u, a, d.unwrap_or(0), b, p)));
    out(v, |&(u, a, ..)| (rep_desc(db, u), (a.n == 0, Reverse(a.pmax))), 100, |&(u, a, d, b, p)| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(b.map_or(nulls(3), |b| ints(&[b[1], b[2], b[3]])));
        f.extend(ints(&[d, a.q, a.a, a.up, a.down]));
        f.push(if a.n == 0 { V::Null } else { V::T(a.pmax) });
        f.extend(match p {
            Some(((p, c), x)) => {
                let x = x.unwrap_or([0; 3]);
                let mut g = post_fields(db, p, &["title", "created"]);
                g.extend(ints(&[c, x[1], x[2]]));
                g
            }
            None => nulls(5),
        });
        f
    })
}

// WITH UserBadgeStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(B.Id) AS TotalBadges,
// COALESCE(SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadges,
// COALESCE(SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadges,
// COALESCE(SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadges
// FROM
// Users U
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// GROUP BY
// U.Id, U.DisplayName
// ),
// RecentPostStats AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS TotalPosts,
// COALESCE(SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END), 0) AS Questions,
// COALESCE(SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS Answers,
// COALESCE(SUM(CASE WHEN P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' THEN 1 ELSE 0 END), 0) AS RecentPosts
// FROM
// Posts P
// GROUP BY
// P.OwnerUserId
// ),
// PostHistoryDetails AS (
// SELECT
// PH.UserId,
// PH.PostId,
// PH.PostHistoryTypeId,
// COUNT(*) AS EditCount
// FROM
// PostHistory PH
// WHERE
// PH.PostHistoryTypeId IN (4, 5, 6)
// GROUP BY
// PH.UserId, PH.PostId, PH.PostHistoryTypeId
// ),
// CombinedStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COALESCE(UB.TotalBadges, 0) AS TotalBadges,
// COALESCE(RP.TotalPosts, 0) AS TotalPosts,
// COALESCE(RP.Questions, 0) AS Questions,
// COALESCE(RP.Answers, 0) AS Answers,
// COALESCE(RP.RecentPosts, 0) AS RecentPosts,
// COALESCE(PH.EditCount, 0) AS EditCount
// FROM
// Users U
// LEFT JOIN
// UserBadgeStats UB ON U.Id = UB.UserId
// LEFT JOIN
// RecentPostStats RP ON U.Id = RP.OwnerUserId
// LEFT JOIN
// PostHistoryDetails PH ON U.Id = PH.UserId
// )
// SELECT
// C.UserId,
// C.DisplayName,
// C.Reputation,
// C.TotalBadges,
// C.TotalPosts,
// C.Questions,
// C.Answers,
// C.RecentPosts,
// CASE
// WHEN C.EditCount > 10 THEN 'Frequent Edits'
// WHEN C.EditCount BETWEEN 5 AND 10 THEN 'Moderate Edits'
// WHEN C.EditCount < 5 THEN 'Infrequent Edits'
// ELSE 'No Edits'
// END AS EditFrequency,
// CASE
// WHEN C.Reputation < 100 THEN 'New User'
// WHEN C.Reputation BETWEEN 100 AND 1000 THEN 'Intermediate User'
// ELSE 'Experienced User'
// END AS UserLevel
// FROM
// CombinedStats C
// WHERE
// C.TotalPosts > 0
// ORDER BY
// C.Reputation DESC, C.TotalBadges DESC;
fn q24954(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let m = month_ago();
    let ps = owned(db).group_by(&db.post.owner_user).select((&db.post.post_type_id).and(&db.post.creation_date)).fold([0i64; 4], move |a, (t, c)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (c >= m) as i64]);
    type K = ((Id<User>, Id<Post>), i64);
    let PostHistory { user, post, post_history_type_id, .. } = &db.post_history;
    let eh = || db.post_history.with(post_history_type_id.filt(|t: i64| matches!(t, 4 | 5 | 6))).with(user);
    let ek: MatSet<K> = eh().select(user.and(post).and(post_history_type_id)).collect();
    let ef = eh().group_by(user.and(post).and(post_history_type_id)).select(post).fold(0i64, |a, _| a + 1);
    let ei: HashIdx<Id<User>, K> = (&ek).map(|((u, _), _)| u).inv().collect();
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and(&bu).and(&ps).and((&ei).select(&ef).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(((u, b), p), e)| {
        let e = e.unwrap_or(0);
        let r = db.user.reputation.get(u).unwrap();
        let ef = if e > 10 { "Frequent Edits" } else if (5..=10).contains(&e) { "Moderate Edits" } else { "Infrequent Edits" };
        let mut f = ["uid", "name", "rep"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.extend(ints(&[b, p[0], p[1], p[2], p[3]]));
        f.extend([V::S(ef), V::S(if r < 100 { "New User" } else if r <= 1000 { "Intermediate User" } else { "Experienced User" })]);
        row(f)
    }))
}

// SELECT
// p.Id AS PostId,
// p.Title,
// u.DisplayName AS OwnerDisplayName,
// COUNT(DISTINCT c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
// p.CreationDate,
// p.LastEditDate,
// p.Score,
// p.ViewCount,
// t.TagName
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// (SELECT UNNEST(string_to_array(p.Tags, ',')) AS TagName) AS t ON TRUE
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, u.DisplayName, p.CreationDate, p.LastEditDate, p.Score, p.ViewCount, t.TagName
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q11718(db: &'static So) -> String {
    let cp = comments_per_post(db);
    let vp = votes_per_post(db);
    let mut v = Vec::new();
    questions_only(db).select(Ident::<Post>::new().and(&cp).and(&vp).and((&db.post.tags_str).flat_map(|t: Str| t.split(',')).opt())).drive(|_, x| v.push(x));
    out(v, |&(((p, _), _), _)| Reverse(db.post.creation_date.get(p).unwrap()), 100, |&(((p, c), x), t)| {
        let mut f = post_fields(db, p, &["id", "title", "owner"]);
        f.extend(ints(&[c, x]));
        f.extend(post_fields(db, p, &["created", "edited", "score", "views"]));
        f.push(ostr(t));
        f
    })
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
// p.ViewCount,
// p.Score,
// t.TagName
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// (SELECT
// p.Id,
// unnest(string_to_array(substring(p.Tags, 2, length(p.Tags)-2), '><')) AS TagName
// FROM
// Posts p) t ON p.Id = t.Id
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, p.CreationDate, u.DisplayName, p.ViewCount, p.Score, t.TagName
// ORDER BY
// p.CreationDate DESC
// LIMIT 100;
fn q14530(db: &'static So) -> String {
    let mut v = Vec::new();
    stats_fold(db, questions_only(db), Ident::<Post>::new(), "cv", &[])
        .and((&db.post.tags_str).flat_map(|t: Str| t[1..t.len() - 1].split("><")).opt())
        .drive(|p, (s, t)| v.push((p, s, t)));
    out(v, |&(p, _, t)| (Reverse(db.post.creation_date.get(p).unwrap()), db.post.origid.get(p).unwrap(), (t.is_none(), t)), 100, |&(p, s, t)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend(ints(&[s.cx, s.vx, s.up, s.down]));
        f.extend(post_fields(db, p, &["views", "score"]));
        f.push(ostr(t));
        f
    })
}

// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.ViewCount,
// P.Score,
// U.Reputation AS OwnerReputation,
// U.DisplayName AS OwnerDisplayName,
// COUNT(DISTINCT C.Id) AS CommentCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
// T.TagName
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// LEFT JOIN
// UNNEST(string_to_array(SUBSTRING(P.Tags FROM 2 FOR LENGTH(P.Tags) - 2), '> <')) AS T(TagName) ON T.TagName IS NOT NULL
// WHERE
// P.CreationDate >= DATE '2024-10-01' - INTERVAL '1 year'
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.ViewCount, P.Score, U.Reputation, U.DisplayName, T.TagName
// ORDER BY
// P.CreationDate DESC
// LIMIT 100;
fn q10495(db: &'static So) -> String {
    let cp = comments_per_post(db);
    let pv = owned_since(db, date(2023, 10, 1))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 3], |a, (_, t)| [0, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let mut v = Vec::new();
    owned_since(db, date(2023, 10, 1))
        .select(Ident::<Post>::new().and(&cp).and((&pv).opt()).and((&db.post.tags_str).flat_map(|t: Str| t[1..t.len() - 1].split("> <")).opt()))
        .drive(|_, x| v.push(x));
    out(v, |&(((p, _), _), _)| Reverse(db.post.creation_date.get(p).unwrap()), 100, |&(((p, c), x), t)| {
        let x = x.unwrap_or([0; 3]);
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "rep", "owner"]);
        f.extend(ints(&[c, x[1], x[2]]));
        f.push(ostr(t));
        f
    })
}

// WITH PostStatistics AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.ViewCount,
// P.Score,
// COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
// COUNT(DISTINCT A.Id) AS AnswerCount,
// COALESCE(B.BadgeCount, 0) AS UserBadgeCount
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Posts A ON P.Id = A.ParentId AND A.PostTypeId = 2
// LEFT JOIN (
// SELECT
// UserId,
// COUNT(Id) AS BadgeCount
// FROM
// Badges
// GROUP BY
// UserId
// ) B ON P.OwnerUserId = B.UserId
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.ViewCount, P.Score, B.BadgeCount
// )
// SELECT
// PS.PostId,
// PS.Title,
// PS.CreationDate,
// PS.ViewCount,
// PS.Score,
// PS.CommentCount,
// PS.AnswerCount,
// PS.UserBadgeCount
// FROM
// PostStatistics PS
// ORDER BY
// PS.Score DESC,
// PS.ViewCount DESC
// LIMIT 100;
fn q11846(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    stats_fold(db, db.post.iq(), Ident::<Post>::new(), "cA", &[]).and(typed_answers_per_post(db)).and((&db.post.owner_user).select(&bu).opt()).drive(|p, ((s, a), b)| v.push((p, s, a, b.unwrap_or(0))));
    out(v, |&(p, ..)| score_views(db, p), 100, |&(p, s, a, b)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend(ints(&[s.cx, a, b]));
        f
    })
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN p.ViewCount IS NOT NULL THEN p.ViewCount ELSE 0 END) AS TotalViews,
// SUM(p.Score) AS TotalScore,
// AVG(p.ViewCount) AS AvgViewsPerPost
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.Reputation
// ), BadgeStats AS (
// SELECT
// b.UserId,
// COUNT(b.Id) AS BadgeCount
// FROM
// Badges b
// GROUP BY
// b.UserId
// )
// SELECT
// u.UserId,
// u.Reputation,
// u.PostCount,
// u.QuestionCount,
// u.AnswerCount,
// u.TotalViews,
// u.TotalScore,
// u.AvgViewsPerPost,
// COALESCE(b.BadgeCount, 0) AS BadgeCount
// FROM
// UserStats u
// LEFT JOIN
// BadgeStats b ON u.UserId = b.UserId
// ORDER BY
// u.Reputation DESC;
fn q10472(db: &'static So) -> String {
    let ps = pstat(db, db.post.iq());
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&ps).opt()).and(&bu)).drive(|_, ((u, p), b)| v.push((u, pz(p), b)));
    rows(v.iter().map(|&(u, p, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep")];
        f.extend(ints(&[p[0], p[1], p[2], p[5]]));
        f.extend([nullable(p[3], p[0]), pviews_avg(p), V::I(b)]);
        row(f)
    }))
}

// WITH PostStats AS (
// SELECT
// pt.Name AS PostType,
// COUNT(p.Id) AS TotalPosts,
// SUM(p.ViewCount) AS TotalViews,
// AVG(COALESCE(p.Score, 0)) AS AvgScore,
// AVG(COALESCE(CASE WHEN p.PostTypeId = 1 THEN p.AnswerCount END, 0)) AS AvgAnswers,
// AVG(COALESCE(p.CommentCount, 0)) AS AvgComments,
// SUM(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// GROUP BY
// pt.Name
// ),
// UserStats AS (
// SELECT
// COUNT(u.Id) AS TotalUsers,
// SUM(u.Reputation) AS TotalReputation,
// AVG(u.Reputation) AS AvgReputation,
// COUNT(DISTINCT b.Id) AS TotalBadges
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// )
// SELECT
// ps.PostType,
// ps.TotalPosts,
// ps.TotalViews,
// ps.AvgScore,
// ps.AvgAnswers,
// ps.AvgComments,
// ps.AcceptedAnswers,
// us.TotalUsers,
// us.TotalReputation,
// us.AvgReputation,
// us.TotalBadges
// FROM
// PostStats ps, UserStats us
// ORDER BY
// ps.TotalPosts DESC;
fn q10885(db: &'static So) -> String {
    let Post { post_type_id, view_count, score, answer_count, comment_count, accepted_answer_id, .. } = &db.post;
    let ps = db.post.group_by(&db.post.post_type).select(post_type_id.and(view_count.opt()).and(score).and(answer_count.opt()).and(comment_count).and(accepted_answer_id.opt())).fold([0i64; 7], |a, (((((t, w), s), an), c), acc)| {
        [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s, a[4] + if t == 1 { an.unwrap_or(0) } else { 0 }, a[5] + c, a[6] + acc.is_some() as i64]
    });
    let us = db.user.select((&db.user.reputation).and(badges_of(db).opt())).fold_flat([0i64; 3], |a, (r, b)| [a[0] + 1, a[1] + r, a[2] + b.is_some() as i64]);
    let mut v = Vec::new();
    (&ps).drive(|t, a| v.push((t, a)));
    rows(v.iter().map(|&(t, a)| {
        row(vec![V::S(db.post_type.name.get(t).unwrap()), V::I(a[0]), nullable(a[2], a[1]), avg(a[3], a[0]), avg(a[4], a[0]), avg(a[5], a[0]), V::I(a[6]), V::I(us[0]), V::I(us[1]), avg(us[1], us[0]), V::I(us[2])])
    }))
}

// WITH UserPostStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(P.Id) AS TotalPosts,
// COUNT(DISTINCT CASE WHEN P.PostTypeId = 1 THEN P.Id END) AS TotalQuestions,
// COUNT(DISTINCT CASE WHEN P.PostTypeId = 2 THEN P.Id END) AS TotalAnswers,
// SUM(P.Score) AS TotalScore,
// SUM(P.ViewCount) AS TotalViews,
// AVG(P.Score) AS AvgScore,
// AVG(P.ViewCount) AS AvgViews,
// SUM(CASE WHEN P.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id, U.DisplayName
// ),
// BadgeStats AS (
// SELECT
// B.UserId,
// COUNT(B.Id) AS TotalBadges
// FROM
// Badges B
// GROUP BY
// B.UserId
// )
// SELECT
// U.UserId,
// U.DisplayName,
// U.TotalPosts,
// U.TotalQuestions,
// U.TotalAnswers,
// U.TotalScore,
// U.TotalViews,
// U.AvgScore,
// U.AvgViews,
// U.AcceptedAnswers,
// COALESCE(B.TotalBadges, 0) AS TotalBadges
// FROM
// UserPostStats U
// LEFT JOIN
// BadgeStats B ON U.UserId = B.UserId
// ORDER BY
// U.TotalScore DESC;
fn q11930(db: &'static So) -> String {
    let ps = pstat(db, db.post.iq());
    let acc = owned(db).with(&db.post.accepted_answer_id).group_by(&db.post.owner_user).select(&db.post.score).fold(0i64, |a, _| a + 1);
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&ps).opt()).and((&acc).opt()).and(&bu)).drive(|_, (((u, p), a), b)| v.push((u, pz(p), a.unwrap_or(0), b)));
    rows(v.iter().map(|&(u, p, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&[p[0], p[1], p[2]]));
        f.extend([nullable(p[3], p[0]), pviews(p), pscore_avg(p), pviews_avg(p), V::I(a), V::I(b)]);
        row(f)
    }))
}

// WITH PostStatistics AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// COUNT(c.Id) AS TotalComments,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpVotes,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownVotes,
// COUNT(DISTINCT b.Id) AS TotalBadges,
// COUNT(DISTINCT pl.RelatedPostId) AS TotalLinks
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Badges b ON p.OwnerUserId = b.UserId
// LEFT JOIN
// PostLinks pl ON p.Id = pl.PostId
// WHERE
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY
// p.Id, p.Title
// ),
// AggregatedStatistics AS (
// SELECT
// COUNT(*) AS TotalPosts,
// SUM(TotalComments) AS PostComments,
// SUM(TotalUpVotes) AS Upvotes,
// SUM(TotalDownVotes) AS Downvotes,
// AVG(TotalLinks) AS AverageLinks
// FROM
// PostStatistics
// )
// SELECT
// TotalPosts,
// PostComments,
// Upvotes,
// Downvotes,
// AverageLinks
// FROM
// AggregatedStatistics;
fn q14854(db: &'static So) -> String {
    let dl = db.post_link.group_by(&db.post_link.post).select(&db.post_link.related_post_id).count_distinct();
    let sf = stats_fold(db, since(db, year_ago()), Ident::<Post>::new(), "cvbl", &[]);
    let t = (&sf).fold_flat([0i64; 4], |a, s: Stats| [a[0] + 1, a[1] + s.cx, a[2] + s.up, a[3] + s.down]);
    let l = since(db, year_ago()).select((&dl).opt()).fold_flat(0i64, |a, x| a + x.unwrap_or(0));
    row(vec![V::I(t[0]), nullable(t[1], t[0]), nullable(t[2], t[0]), nullable(t[3], t[0]), avg(l, t[0])])
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// COALESCE(SUM(c.Score), 0) AS TotalCommentScore
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN Comments c ON p.Id = c.PostId
// GROUP BY u.Id, u.DisplayName, u.Reputation
// ),
// PostStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// AVG(COALESCE(p.Score, 0)) AS AvgScore,
// AVG(COALESCE(p.ViewCount, 0)) AS AvgViews,
// COUNT(DISTINCT p.Id) FILTER (WHERE p.AcceptedAnswerId IS NOT NULL) AS AcceptedAnswers
// FROM Posts p
// GROUP BY p.OwnerUserId
// )
// SELECT
// us.UserId,
// us.DisplayName,
// us.Reputation,
// us.PostCount,
// us.QuestionCount,
// us.AnswerCount,
// us.TotalCommentScore,
// ps.TotalPosts,
// ps.AvgScore,
// ps.AvgViews,
// ps.AcceptedAnswers
// FROM UserStats us
// JOIN PostStats ps ON us.UserId = ps.OwnerUserId
// ORDER BY us.Reputation DESC, us.PostCount DESC;
fn q14571(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "c", any_post);
    let ps = pstat(db, db.post.iq());
    let acc = owned(db).with(&db.post.accepted_answer_id).group_by(&db.post.owner_user).select(&db.post.score).fold(0i64, |a, _| a + 1);
    let mut v = Vec::new();
    (&ps).and(&us).and((&acc).opt()).drive(|u, ((p, a), c)| v.push((u, p, a, c.unwrap_or(0))));
    rows(v.iter().map(|&(u, p, a, c)| {
        let mut f = ["uid", "name", "rep"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.extend(ints(&[p[0], a.q, a.a, a.cscore, p[0]]));
        f.extend([avg(p[3], p[0]), avg(p[5], p[0]), V::I(c)]);
        row(f)
    }))
}

// WITH PostStats AS (
// SELECT
// P.Id AS PostId,
// P.PostTypeId,
// P.CreationDate,
// P.Score,
// P.ViewCount,
// COALESCE(COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END), 0) AS CommentCount,
// COALESCE(COUNT(DISTINCT V.Id), 0) AS VoteCount
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// P.Id, P.PostTypeId, P.CreationDate, P.Score, P.ViewCount
// ),
// UserStats AS (
// SELECT
// U.Id AS UserId,
// U.Reputation,
// COUNT(DISTINCT B.Id) AS BadgeCount,
// COUNT(DISTINCT P.Id) AS TotalPosts
// FROM
// Users U
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id, U.Reputation
// )
// SELECT
// PS.PostId,
// PS.PostTypeId,
// PS.CreationDate,
// PS.Score,
// PS.ViewCount,
// PS.CommentCount,
// PS.VoteCount,
// US.UserId,
// US.Reputation,
// US.BadgeCount,
// US.TotalPosts
// FROM
// PostStats PS
// JOIN
// Users U ON PS.PostTypeId = U.Id
// JOIN
// UserStats US ON U.Id = US.UserId
// ORDER BY
// PS.Score DESC, PS.ViewCount DESC;
fn q13160(db: &'static So) -> String {
    let uid = uids(db);
    let bu = badges_per_user(db);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let mut v = Vec::new();
    stats_fold(db, db.post.iq(), Ident::<Post>::new(), "cv", &[])
        .and(votes_per_post(db))
        .and((&db.post.post_type_id).select(&uid).select(Ident::<User>::new().and(&bu).and((&dp).opt())))
        .drive(|p, ((s, x), ((u, b), d))| v.push((p, s, x, u, b, d.unwrap_or(0))));
    rows(v.iter().map(|&(p, s, x, u, b, d)| {
        let mut f = post_fields(db, p, &["id", "type_id", "created", "score", "views"]);
        f.extend(ints(&[s.cx, x]));
        f.extend([user_col(db, u, "uid"), user_col(db, u, "rep"), V::I(b), V::I(d)]);
        row(f)
    }))
}

fn upb(db: &'static So) -> Fold<Id<User>, [i64; 6]> {
    g(db).select(badges_of(db).opt().and(posts_of(db).select((&db.post.score).and((&db.post.view_count).opt())).opt())).fold([0i64; 6], |a, (b, p)| {
        let (s, w) = p.map_or((None, None), |(s, w)| (Some(s), w));
        [a[0] + b.is_some() as i64, a[1] + s.unwrap_or(0), a[2] + s.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + w.is_some() as i64, a[5] + 1]
    })
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(COALESCE(p.Score, 0)) AS TotalScore,
// SUM(COALESCE(p.ViewCount, 0)) AS TotalViews,
// SUM(CASE WHEN b.Id IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// u.Id, u.Reputation
// ),
// PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// COUNT(c.Id) AS CommentCount,
// COUNT(DISTINCT l.RelatedPostId) AS LinkedPostCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// PostLinks l ON p.Id = l.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate
// )
// SELECT
// us.UserId,
// us.Reputation,
// us.PostCount,
// us.TotalScore,
// us.TotalViews,
// us.BadgeCount,
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.CommentCount,
// ps.LinkedPostCount
// FROM
// UserStats us
// JOIN
// PostStats ps ON us.UserId = ps.PostId
// ORDER BY
// us.Reputation DESC,
// ps.CreationDate DESC;
fn q13809(db: &'static So) -> String {
    let uid = uids(db);
    let ub = upb(db);
    let ps = pstat(db, db.post.iq());
    let dl = db.post_link.group_by(&db.post_link.post).select(&db.post_link.related_post_id).count_distinct();
    let mut v = Vec::new();
    stats_fold(db, db.post.with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cl", &[])
        .and((&dl).opt())
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&ub).and((&ps).opt())))
        .drive(|p, ((s, l), ((u, b), q))| v.push((p, s, l.unwrap_or(0), u, b, pz(q))));
    rows(v.iter().map(|&(p, s, l, u, b, q)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep")];
        f.extend(ints(&[q[0], b[1], b[3], b[0]]));
        f.extend(post_fields(db, p, &["id", "title", "created"]));
        f.extend(ints(&[s.cx, l]));
        row(f)
    }))
}

// WITH PostStats AS (
// SELECT
// P.Id AS PostId,
// P.PostTypeId,
// P.CreationDate,
// P.ViewCount,
// P.Score,
// COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
// COUNT(DISTINCT V.UserId) AS VoteCount
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// P.Id, P.PostTypeId, P.CreationDate, P.ViewCount, P.Score
// ),
// UserStats AS (
// SELECT
// U.Id AS UserId,
// U.Reputation,
// COUNT(B.Id) AS BadgeCount,
// SUM(COALESCE(P.ViewCount, 0)) AS TotalPostViews,
// SUM(COALESCE(P.Score, 0)) AS TotalPostScore
// FROM
// Users U
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id, U.Reputation
// )
// SELECT
// PS.PostId,
// PS.PostTypeId,
// PS.CreationDate,
// PS.ViewCount,
// PS.Score,
// PS.CommentCount,
// PS.VoteCount,
// US.UserId,
// US.Reputation,
// US.BadgeCount,
// US.TotalPostViews,
// US.TotalPostScore
// FROM
// PostStats PS
// JOIN
// UserStats US ON PS.PostId = US.UserId
// ORDER BY
// PS.CreationDate DESC;
fn q11482(db: &'static So) -> String {
    let uid = uids(db);
    let ub = upb(db);
    let ps = pstat(db, db.post.iq());
    let dv = db.vote.group_by(&db.vote.post).select(&db.vote.user).count_distinct();
    let mut v = Vec::new();
    stats_fold(db, db.post.with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cv", &[])
        .and((&dv).opt())
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&ub).and((&ps).opt())))
        .drive(|p, ((s, d), ((u, b), q))| v.push((p, s, d.unwrap_or(0), u, b, pz(q))));
    rows(v.iter().map(|&(p, s, d, u, b, q)| {
        let mut f = post_fields(db, p, &["id", "type_id", "created", "views", "score"]);
        f.extend(ints(&[s.cx, d]));
        f.extend([user_col(db, u, "uid"), user_col(db, u, "rep")]);
        f.extend(ints(&[b[0], b[3], b[1]]));
        row(f)
    }))
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(COALESCE(p.Score, 0)) AS TotalScore,
// SUM(COALESCE(p.ViewCount, 0)) AS TotalViews,
// SUM(CASE WHEN b.Id IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// u.Id, u.Reputation
// ),
// PostStats AS (
// SELECT
// p.Id AS PostId,
// p.PostTypeId,
// COUNT(DISTINCT c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
// MAX(p.CreationDate) AS LastActivityDate
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
// us.UserId,
// us.Reputation,
// us.PostCount,
// us.TotalScore,
// us.TotalViews,
// us.BadgeCount,
// ps.PostId,
// ps.PostTypeId,
// ps.CommentCount,
// ps.VoteCount,
// ps.LastActivityDate
// FROM
// UserStats us
// JOIN
// PostStats ps ON us.UserId = ps.PostId
// ORDER BY
// us.Reputation DESC,
// ps.LastActivityDate DESC
// LIMIT 100;
fn q12343(db: &'static So) -> String {
    let uid = uids(db);
    let ub = upb(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    db.post
        .with((&db.post.origid).select(&uid))
        .select(Ident::<Post>::new().and(comments_per_post(db)).and(votes_per_post(db)).and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&ub).and((&ps).opt()))))
        .drive(|_, x| v.push(x));
    out(v, |&(((p, _), _), ((u, _), _))| (rep_desc(db, u), Reverse(db.post.creation_date.get(p).unwrap())), 100, |&(((p, c), x), ((u, b), q))| {
        let q = pz(q);
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep")];
        f.extend(ints(&[q[0], b[1], b[3], b[0]]));
        f.extend(post_fields(db, p, &["id", "type_id"]));
        f.extend(ints(&[c, x]));
        f.extend(post_fields(db, p, &["created"]));
        f
    })
}

// WITH PostStats AS (
// SELECT
// P.Id AS PostId,
// P.PostTypeId,
// COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
// COUNT(CASE WHEN V.Id IS NOT NULL THEN 1 END) AS VoteCount,
// COUNT(CASE WHEN PH.Id IS NOT NULL THEN 1 END) AS HistoryCount,
// SUM(COALESCE(P.Score, 0)) AS TotalScore,
// SUM(COALESCE(P.ViewCount, 0)) AS TotalViews
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// LEFT JOIN
// PostHistory PH ON P.Id = PH.PostId
// GROUP BY
// P.Id, P.PostTypeId
// ),
// UserStats AS (
// SELECT
// U.Id AS UserId,
// COUNT(DISTINCT P.Id) AS PostCount,
// SUM(U.Reputation) AS TotalReputation
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id
// )
// SELECT
// PS.PostId,
// PS.PostTypeId,
// PS.CommentCount,
// PS.VoteCount,
// PS.HistoryCount,
// PS.TotalScore,
// PS.TotalViews,
// US.UserId,
// US.PostCount,
// US.TotalReputation
// FROM
// PostStats PS
// JOIN
// Users U ON PS.PostTypeId = 1 AND U.Id = PS.PostId
// LEFT JOIN
// UserStats US ON U.Id = US.UserId
// ORDER BY
// PS.TotalViews DESC;
fn q14371(db: &'static So) -> String {
    let uid = uids(db);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let ur = g(db).select((&db.user.reputation).and(posts_of(db).opt())).fold(0i64, |n, (r, _)| n + r);
    let Post { score, view_count, .. } = &db.post;
    let ps = questions_only(db)
        .with((&db.post.origid).select(&uid))
        .group_by(Ident::<Post>::new())
        .select(score.and(view_count.opt()).and(comments_of(db).opt()).and(votes_of(db).opt()).and(history_of(db).opt()))
        .fold([0i64; 5], |a, ((((s, w), c), x), h)| [a[0] + c.is_some() as i64, a[1] + x.is_some() as i64, a[2] + h.is_some() as i64, a[3] + s, a[4] + w.unwrap_or(0)]);
    let mut v = Vec::new();
    (&ps)
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and((&dp).opt()).and(&ur)))
        .drive(|p, (s, ((u, d), r))| v.push((p, s, u, d.unwrap_or(0), r)));
    rows(v.iter().map(|&(p, s, u, d, r)| {
        let mut f = post_fields(db, p, &["id", "type_id"]);
        f.extend(ints(&s));
        f.extend([user_col(db, u, "uid"), V::I(d), V::I(r)]);
        row(f)
    }))
}

// WITH PostStatistics AS (
// SELECT
// P.Id AS PostId,
// P.PostTypeId,
// P.CreationDate,
// P.ViewCount,
// P.Score,
// COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
// COUNT(CASE WHEN V.Id IS NOT NULL THEN 1 END) AS VoteCount
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// P.Id, P.PostTypeId, P.CreationDate, P.ViewCount, P.Score
// ),
// UserStatistics AS (
// SELECT
// U.Id AS UserId,
// U.Reputation,
// COUNT(B.Id) AS BadgeCount,
// COUNT(DISTINCT P.Id) AS PostsCount,
// SUM(COALESCE(P.ViewCount, 0)) AS TotalViews
// FROM
// Users U
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id, U.Reputation
// )
// SELECT
// PS.PostId,
// PS.PostTypeId,
// PS.CreationDate,
// PS.ViewCount,
// PS.Score,
// PS.CommentCount,
// PS.VoteCount,
// US.UserId,
// US.Reputation,
// US.BadgeCount,
// US.PostsCount,
// US.TotalViews
// FROM
// PostStatistics PS
// JOIN
// Users U ON PS.PostTypeId = U.Id
// JOIN
// UserStatistics US ON U.Id = US.UserId
// ORDER BY
// PS.ViewCount DESC, PS.Score DESC;
fn q11435(db: &'static So) -> String {
    let uid = uids(db);
    let ub = upb(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    stats_fold(db, db.post.iq(), Ident::<Post>::new(), "cv", &[])
        .and((&db.post.post_type_id).select(&uid).select(Ident::<User>::new().and(&ub).and((&ps).opt())))
        .drive(|p, (s, ((u, b), q))| v.push((p, s, u, b, pz(q))));
    rows(v.iter().map(|&(p, s, u, b, q)| {
        let mut f = post_fields(db, p, &["id", "type_id", "created", "views", "score"]);
        f.extend(ints(&[s.cx, s.vx]));
        f.extend([user_col(db, u, "uid"), user_col(db, u, "rep")]);
        f.extend(ints(&[b[0], q[0], b[3]]));
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
// PostStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(p.Score) AS TotalScore,
// AVG(p.ViewCount) AS AvgViewCount,
// MAX(p.CreationDate) AS LastPostDate
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// )
// SELECT
// u.DisplayName,
// COALESCE(ub.BadgeCount, 0) AS BadgeCount,
// COALESCE(ub.GoldBadges, 0) AS GoldBadges,
// COALESCE(ps.PostCount, 0) AS PostCount,
// COALESCE(ps.TotalScore, 0) AS TotalScore,
// COALESCE(ps.AvgViewCount, 0) AS AvgViewCount,
// COALESCE(ps.LastPostDate, '1970-01-01') AS LastPostDate
// FROM
// Users u
// LEFT JOIN
// UserBadges ub ON u.Id = ub.UserId
// LEFT JOIN
// PostStats ps ON u.Id = ps.OwnerUserId
// WHERE
// (ub.BadgeCount IS NULL OR ub.BadgeCount >= 1)
// AND (ps.PostCount IS NOT NULL AND ps.PostCount > 0)
// ORDER BY
// TotalScore DESC, BadgeCount DESC
// LIMIT 100;
fn q3160(db: &'static So) -> String {
    let ub = ubc(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    (&ub).filt(|b: [i64; 4]| b[0] >= 1).and(&ps).drive(|u, (b, p)| v.push((u, b, p)));
    out(v, |&(_, b, p)| (Reverse(p[3]), Reverse(b[0])), 100, |&(u, b, p)| vec![user_col(db, u, "name"), V::I(b[0]), V::I(b[1]), V::I(p[0]), V::I(p[3]), or0(p[5], p[4]), V::T(p[10])])
}

// WITH PostStats AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.PostTypeId,
// P.Score,
// P.ViewCount,
// COALESCE(SUM(V.BountyAmount), 0) AS TotalBounty,
// COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
// COUNT(CASE WHEN A.Id IS NOT NULL THEN 1 END) AS AnswerCount,
// P.OwnerUserId
// FROM
// Posts P
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Posts A ON P.Id = A.ParentId
// GROUP BY
// P.Id, P.Title, P.PostTypeId, P.Score, P.ViewCount, P.OwnerUserId
// ),
// UserStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(DISTINCT B.Id) AS BadgeCount,
// COUNT(DISTINCT PostStats.PostId) AS PostsCount,
// SUM(PostStats.Score) AS TotalScore,
// SUM(PostStats.ViewCount) AS TotalViewCount
// FROM
// Users U
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// LEFT JOIN
// PostStats ON U.Id = PostStats.OwnerUserId
// GROUP BY
// U.Id, U.DisplayName, U.Reputation
// )
// SELECT
// U.UserId,
// U.DisplayName,
// U.Reputation,
// U.BadgeCount,
// U.PostsCount,
// U.TotalScore,
// U.TotalViewCount
// FROM
// UserStats U
// ORDER BY
// U.Reputation DESC,
// U.PostsCount DESC;
fn q12871(db: &'static So) -> String {
    let ub = upb(db);
    let bu = badges_per_user(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and(&bu).and(&ub).and((&ps).opt())).drive(|_, (((u, b), x), p)| v.push((u, b, x, pz(p))));
    rows(v.iter().map(|&(u, b, x, p)| {
        let mut f = ["uid", "name", "rep"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.extend([V::I(b), V::I(p[0]), nullable(x[1], x[2]), nullable(x[3], x[4])]);
        row(f)
    }))
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN p.ViewCount IS NOT NULL THEN p.ViewCount ELSE 0 END) AS TotalViews,
// SUM(COALESCE(c.CommentCount, 0)) AS TotalComments,
// SUM(COALESCE(v.VoteCount, 0)) AS TotalVotes
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS VoteCount FROM Votes GROUP BY PostId) v ON p.Id = v.PostId
// GROUP BY
// u.Id, u.DisplayName
// ),
// UserBadgeStats AS (
// SELECT
// UserId,
// COUNT(*) AS TotalBadges
// FROM
// Badges
// GROUP BY
// UserId
// )
// SELECT
// ups.UserId,
// ups.DisplayName,
// ups.TotalPosts,
// ups.TotalQuestions,
// ups.TotalAnswers,
// ups.TotalViews,
// ups.TotalComments,
// ups.TotalVotes,
// COALESCE(ubs.TotalBadges, 0) AS TotalBadges
// FROM
// UserPostStats ups
// LEFT JOIN
// UserBadgeStats ubs ON ups.UserId = ubs.UserId
// ORDER BY
// ups.TotalPosts DESC, ups.TotalVotes DESC;
fn q14642(db: &'static So) -> String {
    let cp = comments_per_post(db);
    let vp = votes_per_post(db);
    let Post { post_type_id, view_count, .. } = &db.post;
    let uf = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(view_count.opt()).and(&cp).and(&vp)).fold([0i64; 6], |a, (((t, w), c), x)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.unwrap_or(0), a[4] + c, a[5] + x]
    });
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&uf).opt()).and(&bu)).drive(|_, ((u, a), b)| v.push((u, a.unwrap_or([0; 6]), b)));
    rows(v.iter().map(|&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a));
        f.push(V::I(b));
        row(f)
    }))
}

// WITH PostDetails AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// u.DisplayName AS OwnerDisplayName,
// COUNT(DISTINCT c.Id) AS CommentCount,
// COUNT(DISTINCT a.Id) AS AnswerCount
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Posts a ON p.Id = a.ParentId AND a.PostTypeId = 2
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, u.DisplayName
// ),
// VoteCounts AS (
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
// pd.PostId,
// pd.Title,
// pd.CreationDate,
// pd.ViewCount,
// pd.Score,
// pd.OwnerDisplayName,
// pd.CommentCount,
// pd.AnswerCount,
// COALESCE(vc.UpVotes, 0) AS UpVotes,
// COALESCE(vc.DownVotes, 0) AS DownVotes,
// (COALESCE(vc.UpVotes, 0) - COALESCE(vc.DownVotes, 0)) AS NetVotes
// FROM
// PostDetails pd
// LEFT JOIN
// VoteCounts vc ON pd.PostId = vc.PostId
// ORDER BY
// NetVotes DESC, pd.Score DESC
// LIMIT 20;
fn q9655(db: &'static So) -> String {
    let cp = comments_per_post(db);
    let ta = typed_answers_per_post(db);
    let pv = post_votes(db);
    let mut v = Vec::new();
    owned_since(db, year_ago()).select(Ident::<Post>::new().and(&cp).and(&ta).and((&pv).opt())).drive(|_, (((p, c), a), x)| v.push((p, c, a, x.unwrap_or([0; 3]))));
    out(v, |&(p, _, _, x)| (Reverse(x[1] - x[2]), score_desc(db, p)), 20, |&(p, c, a, x)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "owner"]);
        f.extend(ints(&[c, a, x[1], x[2], x[1] - x[2]]));
        f
    })
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.Reputation,
// COUNT(DISTINCT P.Id) AS PostCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN P.ViewCount IS NOT NULL THEN P.ViewCount ELSE 0 END) AS TotalViews,
// SUM(CASE WHEN P.Score IS NOT NULL THEN P.Score ELSE 0 END) AS TotalScore
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id, U.Reputation
// ),
// BadgeCounts AS (
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
// )
// SELECT
// U.UserId,
// U.Reputation,
// U.PostCount,
// U.QuestionCount,
// U.AnswerCount,
// U.TotalViews,
// U.TotalScore,
// COALESCE(B.BadgeCount, 0) AS BadgeCount,
// COALESCE(B.GoldBadges, 0) AS GoldBadges,
// COALESCE(B.SilverBadges, 0) AS SilverBadges,
// COALESCE(B.BronzeBadges, 0) AS BronzeBadges
// FROM
// UserStats U
// LEFT JOIN
// BadgeCounts B ON U.UserId = B.UserId
// ORDER BY
// U.Reputation DESC, U.PostCount DESC;
fn q11950(db: &'static So) -> String {
    let ps = pstat(db, db.post.iq());
    let bc = badge_classes(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&ps).opt()).and((&bc).opt())).drive(|_, ((u, p), b)| v.push((u, pz(p), bz(b))));
    rows(v.iter().map(|&(u, p, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep")];
        f.extend(ints(&[p[0], p[1], p[2], p[5], p[3]]));
        f.extend(ints(&b));
        row(f)
    }))
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// SUM(COALESCE(p.ViewCount, 0)) AS TotalViews,
// SUM(COALESCE(p.Score, 0)) AS TotalScore,
// COUNT(DISTINCT p.Id) AS PostCount,
// COUNT(DISTINCT CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN p.Id END) AS AcceptedAnswersCount
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
// p.ViewCount,
// COALESCE(c.CommentCount, 0) AS CommentCount,
// COALESCE(v.VoteCount, 0) AS VoteCount
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
// COUNT(*) AS VoteCount
// FROM
// Votes
// GROUP BY
// PostId
// ) v ON p.Id = v.PostId
// )
// SELECT
// us.UserId,
// us.TotalViews,
// us.TotalScore,
// us.PostCount,
// us.AcceptedAnswersCount,
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.ViewCount,
// ps.CommentCount,
// ps.VoteCount
// FROM
// UserStats us
// JOIN
// PostStats ps ON us.UserId = ps.PostId
// ORDER BY
// us.TotalViews DESC, us.TotalScore DESC;
fn q13765(db: &'static So) -> String {
    let pid = pids(db);
    let ps = pstat(db, db.post.iq());
    let acc = owned(db).with(&db.post.accepted_answer_id).group_by(&db.post.owner_user).select(&db.post.score).fold(0i64, |a, _| a + 1);
    let mut v = Vec::new();
    db.user
        .select(Ident::<User>::new().and((&ps).opt()).and((&acc).opt()).and((&db.user.origid).select(&pid).select(Ident::<Post>::new().and(comments_per_post(db)).and(votes_per_post(db)))))
        .drive(|_, (((u, p), a), ((q, c), x))| v.push((u, pz(p), a.unwrap_or(0), q, c, x)));
    rows(v.iter().map(|&(u, p, a, q, c, x)| {
        let mut f = vec![user_col(db, u, "uid")];
        f.extend(ints(&[p[5], p[3], p[0], a]));
        f.extend(post_fields(db, q, &["id", "title", "created", "views"]));
        f.extend(ints(&[c, x]));
        row(f)
    }))
}

// WITH UserScores AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes,
// COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes,
// COUNT(DISTINCT P.Id) AS PostCount,
// COUNT(DISTINCT B.Id) AS BadgeCount
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// GROUP BY
// U.Id, U.DisplayName, U.Reputation
// ),
// PostMetrics AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.ViewCount,
// P.CommentCount,
// P.AnswerCount,
// U.Id AS UserId,
// U.DisplayName AS OwnerName
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// UserScores PS ON U.Id = PS.UserId
// WHERE
// P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'
// )
// SELECT
// US.UserId,
// US.DisplayName,
// US.Reputation,
// P.PostId,
// P.Title,
// P.ViewCount,
// P.CommentCount,
// P.AnswerCount,
// US.Upvotes,
// US.Downvotes,
// US.BadgeCount
// FROM
// UserScores US
// JOIN
// PostMetrics P ON US.UserId = P.UserId
// ORDER BY
// US.Reputation DESC,
// P.ViewCount DESC
// LIMIT 10;
fn q6314(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "vb", any_post);
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    owned_since(db, month_ago()).select(Ident::<Post>::new().and((&db.post.owner_user).select(Ident::<User>::new().and(&us).and(&bu)))).drive(|_, x| v.push(x));
    out(v, |&(p, ((u, _), _))| (rep_desc(db, u), views_desc(db, p)), 10, |&(p, ((u, a), b))| {
        let mut f = ["uid", "name", "rep"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.extend(post_fields(db, p, &["id", "title", "views", "comments", "answers"]));
        f.extend(ints(&[a.up, a.down, b]));
        f
    })
}

// WITH UserReputation AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers,
// SUM(CASE WHEN p.Score > 0 THEN p.Score ELSE 0 END) AS TotalScore
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
// ),
// BadgeCounts AS (
// SELECT
// b.UserId,
// COUNT(b.Id) AS TotalBadges
// FROM
// Badges b
// GROUP BY
// b.UserId
// ),
// PostStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS TotalPosts,
// SUM(p.ViewCount) AS TotalViews,
// SUM(p.CommentCount) AS TotalComments
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// )
// SELECT
// ur.UserId,
// ur.DisplayName,
// ur.Reputation,
// ur.PostCount,
// COALESCE(bc.TotalBadges, 0) AS TotalBadges,
// ur.AcceptedAnswers,
// ur.TotalScore,
// COALESCE(ps.TotalPosts, 0) AS TotalPosts,
// COALESCE(ps.TotalViews, 0) AS TotalViews,
// COALESCE(ps.TotalComments, 0) AS TotalComments
// FROM
// UserReputation ur
// LEFT JOIN
// BadgeCounts bc ON ur.UserId = bc.UserId
// LEFT JOIN
// PostStats ps ON ur.UserId = ps.OwnerUserId
// ORDER BY
// ur.Reputation DESC,
// ur.PostCount DESC,
// ur.TotalScore DESC
// LIMIT 100;
fn q7906(db: &'static So) -> String {
    let Post { accepted_answer_id, score, view_count, comment_count, .. } = &db.post;
    let uf = owned(db).group_by(&db.post.owner_user).select(accepted_answer_id.opt().and(score).and(view_count.opt()).and(comment_count)).fold([0i64; 6], |a, (((acc, s), w), c)| {
        [a[0] + 1, a[1] + acc.is_some() as i64, a[2] + if s > 0 { s } else { 0 }, a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0), a[5] + c]
    });
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&uf).opt()).and(&bu)).drive(|_, ((u, a), b)| v.push((u, a.unwrap_or([0; 6]), b)));
    out(v, |&(u, a, _)| (rep_desc(db, u), Reverse(a[0]), Reverse(a[2])), 100, |&(u, a, b)| {
        let mut f = ["uid", "name", "rep"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.extend(ints(&[a[0], b, a[1], a[2], a[0], a[4], a[5]]));
        f
    })
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(DISTINCT P.Id) AS PostCount,
// SUM(CASE WHEN P.Score IS NOT NULL THEN P.Score ELSE 0 END) AS TotalScore,
// SUM(CASE WHEN P.ViewCount IS NOT NULL THEN P.ViewCount ELSE 0 END) AS TotalViews,
// SUM(CASE WHEN P.AnswerCount IS NOT NULL THEN P.AnswerCount ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN P.CommentCount IS NOT NULL THEN P.CommentCount ELSE 0 END) AS TotalComments
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id, U.DisplayName
// ),
// BadgeStats AS (
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
// )
// SELECT
// U.UserId,
// U.DisplayName,
// U.PostCount,
// U.TotalScore,
// U.TotalViews,
// U.TotalAnswers,
// U.TotalComments,
// COALESCE(B.BadgeCount, 0) AS BadgeCount,
// COALESCE(B.GoldBadges, 0) AS GoldBadges,
// COALESCE(B.SilverBadges, 0) AS SilverBadges,
// COALESCE(B.BronzeBadges, 0) AS BronzeBadges
// FROM
// UserStats U
// LEFT JOIN
// BadgeStats B ON U.UserId = B.UserId
// ORDER BY
// U.TotalScore DESC
// LIMIT 100;
fn q11871(db: &'static So) -> String {
    let Post { score, view_count, answer_count, comment_count, .. } = &db.post;
    let uf = owned(db).group_by(&db.post.owner_user).select(score.and(view_count.opt()).and(answer_count.opt()).and(comment_count)).fold([0i64; 5], |a, (((s, w), an), c)| {
        [a[0] + 1, a[1] + s, a[2] + w.unwrap_or(0), a[3] + an.unwrap_or(0), a[4] + c]
    });
    let bc = badge_classes(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&uf).opt()).and((&bc).opt())).drive(|_, ((u, a), b)| v.push((u, a.unwrap_or([0; 5]), bz(b))));
    out(v, |&(_, a, _)| Reverse(a[1]), 100, |&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a));
        f.extend(ints(&b));
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
// COALESCE(c.CommentCount, 0) AS CommentCount,
// CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END AS HasAcceptedAnswer,
// p.OwnerUserId
// FROM
// Posts p
// LEFT JOIN (
// SELECT PostId, COUNT(*) AS CommentCount
// FROM Comments
// GROUP BY PostId
// ) c ON p.Id = c.PostId
// ),
// UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(DISTINCT b.Id) AS BadgeCount
// FROM
// Users u
// LEFT JOIN Badges b ON u.Id = b.UserId
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
// ),
// VoteStats AS (
// SELECT
// v.PostId,
// COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes,
// COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes
// FROM
// Votes v
// GROUP BY
// v.PostId
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.Score,
// ps.ViewCount,
// ps.CommentCount,
// ps.HasAcceptedAnswer,
// us.DisplayName AS OwnerDisplayName,
// us.Reputation,
// us.BadgeCount,
// vs.UpVotes,
// vs.DownVotes
// FROM
// PostStats ps
// JOIN
// Users u ON ps.OwnerUserId = u.Id
// JOIN
// UserStats us ON u.Id = us.UserId
// LEFT JOIN
// VoteStats vs ON ps.PostId = vs.PostId
// ORDER BY
// ps.CreationDate DESC
// LIMIT 100;
fn q13094(db: &'static So) -> String {
    let cp = comments_per_post(db);
    let bu = badges_per_user(db);
    let pv = post_votes(db);
    let mut v = Vec::new();
    owned(db).select(Ident::<Post>::new().and(&cp).and((&db.post.owner_user).select(&bu)).and((&pv).opt())).drive(|_, x| v.push(x));
    out(v, |&(((p, _), _), _)| Reverse(db.post.creation_date.get(p).unwrap()), 100, |&(((p, c), b), x)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(c), V::I(db.post.accepted_answer_id.get(p).is_some() as i64)]);
        f.extend(post_fields(db, p, &["owner", "rep"]));
        f.push(V::I(b));
        f.extend(x.map_or(nulls(2), |x| ints(&[x[1], x[2]])));
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
// SUM(CASE WHEN p.ViewCount IS NOT NULL THEN p.ViewCount ELSE 0 END) AS TotalViews,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN Votes v ON p.Id = v.PostId
// GROUP BY u.Id, u.DisplayName
// ),
// BadgeStats AS (
// SELECT
// b.UserId,
// COUNT(DISTINCT b.Id) AS BadgeCount,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM Badges b
// GROUP BY b.UserId
// )
// SELECT
// us.UserId,
// us.DisplayName,
// us.PostCount,
// us.QuestionCount,
// us.AnswerCount,
// us.TotalViews,
// us.UpVotes,
// us.DownVotes,
// COALESCE(bs.BadgeCount, 0) AS BadgeCount,
// COALESCE(bs.GoldBadges, 0) AS GoldBadges,
// COALESCE(bs.SilverBadges, 0) AS SilverBadges,
// COALESCE(bs.BronzeBadges, 0) AS BronzeBadges
// FROM UserStats us
// LEFT JOIN BadgeStats bs ON us.UserId = bs.UserId
// ORDER BY us.PostCount DESC, us.TotalViews DESC;
fn q13626(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&us).and((&dp).opt()).and((&bc).opt()).drive(|u, ((a, d), b)| v.push((u, a, d.unwrap_or(0), bz(b))));
    rows(v.iter().map(|&(u, a, d, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&[d, a.q, a.a, a.views_sum, a.up, a.down]));
        f.extend(ints(&b));
        row(f)
    }))
}

// WITH UserReputation AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
// ),
// BadgeSummary AS (
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
// PostScoreSummary AS (
// SELECT
// p.OwnerUserId,
// SUM(p.Score) AS TotalScore,
// AVG(p.Score) AS AverageScore
// FROM
// Posts p
// WHERE
// p.Score IS NOT NULL
// GROUP BY
// p.OwnerUserId
// )
// SELECT
// ur.DisplayName,
// ur.Reputation,
// ur.PostCount,
// ur.AnswerCount,
// COALESCE(bs.GoldBadges, 0) AS GoldBadges,
// COALESCE(bs.SilverBadges, 0) AS SilverBadges,
// COALESCE(bs.BronzeBadges, 0) AS BronzeBadges,
// COALESCE(pss.TotalScore, 0) AS TotalScore,
// COALESCE(pss.AverageScore, 0) AS AverageScore
// FROM
// UserReputation ur
// LEFT JOIN
// BadgeSummary bs ON ur.UserId = bs.UserId
// LEFT JOIN
// PostScoreSummary pss ON ur.UserId = pss.OwnerUserId
// ORDER BY
// ur.Reputation DESC, ur.PostCount DESC, ur.AnswerCount DESC;
fn q9530(db: &'static So) -> String {
    let ps = pstat(db, db.post.iq());
    let bc = badge_classes(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&ps).opt()).and((&bc).opt())).drive(|_, ((u, p), b)| v.push((u, pz(p), bz(b))));
    rows(v.iter().map(|&(u, p, b)| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(ints(&[p[0], p[2], b[1], b[2], b[3], p[3]]));
        f.push(or0(p[3], p[0]));
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
// SUM(CASE WHEN P.ClosedDate IS NOT NULL THEN 1 ELSE 0 END) AS ClosedPosts,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// U.Id, U.DisplayName
// ),
// BadgesCount AS (
// SELECT
// UserId,
// COUNT(*) AS BadgeCount
// FROM
// Badges
// GROUP BY
// UserId
// ),
// FinalStats AS (
// SELECT
// UA.UserId,
// UA.DisplayName,
// UA.TotalPosts,
// UA.Questions,
// UA.Answers,
// UA.ClosedPosts,
// UA.UpVotes,
// UA.DownVotes,
// COALESCE(BC.BadgeCount, 0) AS BadgeCount
// FROM
// UserActivity UA
// LEFT JOIN
// BadgesCount BC ON UA.UserId = BC.UserId
// )
// SELECT
// UserId,
// DisplayName,
// TotalPosts,
// Questions,
// Answers,
// ClosedPosts,
// UpVotes,
// DownVotes,
// BadgeCount,
// (TotalPosts - ClosedPosts) AS ActivePosts,
// (UpVotes - DownVotes) AS VoteBalance
// FROM
// FinalStats
// ORDER BY
// ActivePosts DESC, VoteBalance DESC
// LIMIT 10;
fn q6903(db: &'static So) -> String {
    let Post { post_type_id, closed_date, .. } = &db.post;
    let uf = owned(db)
        .group_by(&db.post.owner_user)
        .select(post_type_id.and(closed_date.opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 5], |a, ((t, cl), v)| [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + cl.is_some() as i64, a[3] + (v == Some(2)) as i64, a[4] + (v == Some(3)) as i64]);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&uf).opt()).and((&dp).opt()).and(&bu)).drive(|_, (((u, a), d), b)| v.push((u, a.unwrap_or([0; 5]), d.unwrap_or(0), b)));
    out(v, |&(_, a, d, _)| (Reverse(d - a[2]), Reverse(a[3] - a[4])), 10, |&(u, a, d, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&[d, a[0], a[1], a[2], a[3], a[4], b, d - a[2], a[3] - a[4]]));
        f
    })
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// U.Views,
// COUNT(DISTINCT P.Id) AS PostCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN P.PostTypeId = 1 AND P.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers,
// SUM(CASE WHEN B.UserId IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// GROUP BY
// U.Id, U.DisplayName, U.Reputation, U.Views
// ),
// HighReputationUsers AS (
// SELECT
// UserId,
// DisplayName,
// Reputation,
// Views,
// PostCount,
// AnswerCount,
// AcceptedAnswers,
// BadgeCount
// FROM
// UserStats
// WHERE
// Reputation > (SELECT AVG(Reputation) FROM Users)
// ),
// UserPerformance AS (
// SELECT
// H.DisplayName,
// H.Reputation,
// H.PostCount,
// H.AnswerCount,
// H.AcceptedAnswers,
// H.BadgeCount,
// COALESCE((SELECT COUNT(*) FROM Votes V WHERE V.UserId = H.UserId), 0) AS TotalVotes,
// COALESCE((SELECT COUNT(*) FROM Comments C WHERE C.UserId = H.UserId), 0) AS TotalComments
// FROM
// HighReputationUsers H
// )
// SELECT
// U.*,
// (TotalVotes + TotalComments) AS EngagementScore
// FROM
// UserPerformance U
// ORDER BY
// EngagementScore DESC
// LIMIT 10;
fn q6275(db: &'static So) -> String {
    let t = db.user.select(&db.user.reputation).fold_flat([0i64; 2], |a, r| [a[0] + 1, a[1] + r]);
    let Post { post_type_id, accepted_answer_id, .. } = &db.post;
    let uf = owned(db).group_by(&db.post.owner_user).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let ub = g(db).select(posts_of(db).select(post_type_id.and(accepted_answer_id.opt())).opt().and(badges_of(db).opt())).fold([0i64; 3], |a, (p, b)| {
        let (an, acc) = p.map_or((false, false), |(t, acc)| (t == 2, t == 1 && acc.is_some()));
        [a[0] + an as i64, a[1] + acc as i64, a[2] + b.is_some() as i64]
    });
    let vu = votes_per_user(db);
    let cu = comments_per_user(db);
    let mut v = Vec::new();
    db.user
        .with((&db.user.reputation).filt(move |r: i64| (r as i128) * (t[0] as i128) > t[1] as i128))
        .select(Ident::<User>::new().and((&uf).opt()).and(&ub).and(&vu).and(&cu))
        .drive(|_, ((((u, n), b), x), c)| v.push((u, n.unwrap_or(0), b, x, c)));
    out(v, |&(.., x, c)| Reverse(x + c), 10, |&(u, n, b, x, c)| {
        vec![user_col(db, u, "name"), user_col(db, u, "rep"), V::I(n), V::I(b[0]), V::I(b[1]), V::I(b[2]), V::I(x), V::I(c), V::I(x + c)]
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
// SUM(CASE WHEN P.PostTypeId = 2 AND P.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswerCount,
// SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END) AS PositiveScoreCount
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// WHERE
// U.Reputation > 1000
// GROUP BY
// U.Id, U.DisplayName, U.Reputation
// ),
// BadgeCounts AS (
// SELECT
// UserId,
// COUNT(*) AS BadgeCount
// FROM
// Badges
// GROUP BY
// UserId
// ),
// PostHistoryCounts AS (
// SELECT
// U.Id AS UserId,
// COUNT(PH.Id) AS HistoryCount
// FROM
// Users U
// LEFT JOIN
// PostHistory PH ON U.Id = PH.UserId
// GROUP BY
// U.Id
// )
// SELECT
// US.UserId,
// US.DisplayName,
// US.Reputation,
// US.PostCount,
// US.QuestionCount,
// US.AnswerCount,
// US.AcceptedAnswerCount,
// US.PositiveScoreCount,
// COALESCE(BC.BadgeCount, 0) AS BadgeCount,
// COALESCE(PHC.HistoryCount, 0) AS PostHistoryCount
// FROM
// UserStats US
// LEFT JOIN
// BadgeCounts BC ON US.UserId = BC.UserId
// LEFT JOIN
// PostHistoryCounts PHC ON US.UserId = PHC.UserId
// ORDER BY
// US.Reputation DESC, US.PostCount DESC;
fn q5518(db: &'static So) -> String {
    let Post { post_type_id, accepted_answer_id, score, .. } = &db.post;
    let uf = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(accepted_answer_id.opt()).and(score)).fold([0i64; 5], |a, ((t, acc), s)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (t == 2 && acc.is_some()) as i64, a[4] + (s > 0) as i64]
    });
    let bu = badges_per_user(db);
    let hu = db.post_history.group_by(&db.post_history.user).select(&db.post_history.post).fold(0i64, |a, _| a + 1);
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and((&uf).opt()).and(&bu).and((&hu).opt())).drive(|_, (((u, a), b), h)| v.push((u, a.unwrap_or([0; 5]), b, h.unwrap_or(0))));
    rows(v.iter().map(|&(u, a, b, h)| {
        let mut f = ["uid", "name", "rep"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.extend(ints(&a));
        f.extend(ints(&[b, h]));
        row(f)
    }))
}

// WITH UserActivity AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(DISTINCT P.Id) AS PostCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(CASE WHEN P.PostTypeId IN (10, 11) THEN 1 ELSE 0 END) AS ClosedPosts,
// AVG(U.Reputation) AS AvgReputation,
// SUM(COALESCE(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END, 0)) AS UpVotes,
// SUM(COALESCE(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END, 0)) AS DownVotes
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// WHERE
// U.CreationDate < CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'
// GROUP BY
// U.Id, U.DisplayName
// ),
// TopUsers AS (
// SELECT
// UserId,
// DisplayName,
// PostCount,
// Questions,
// Answers,
// ClosedPosts,
// AvgReputation,
// UpVotes - DownVotes AS NetVotes
// FROM
// UserActivity
// WHERE
// PostCount > 5
// ORDER BY
// NetVotes DESC, AvgReputation DESC
// LIMIT 10
// )
// SELECT
// U.UserId,
// U.DisplayName,
// U.PostCount,
// U.Questions,
// U.Answers,
// U.ClosedPosts,
// U.AvgReputation,
// U.NetVotes,
// BH.Name AS BadgeName,
// BH.Class
// FROM
// TopUsers U
// LEFT JOIN
// Badges BH ON U.UserId = BH.UserId
// ORDER BY
// U.NetVotes DESC, U.AvgReputation DESC;
fn q9175(db: &'static So) -> String {
    let Post { post_type_id, .. } = &db.post;
    let pn = owned(db).group_by(&db.post.owner_user).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let uv = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 6], |a, (t, v)| {
        [0, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + matches!(t, 10 | 11) as i64, a[4] + (v == Some(2)) as i64, a[5] + (v == Some(3)) as i64]
    });
    let uf: HashIdx<Id<User>, [i64; 6]> = (&uv).and(&pn).map(|(a, n): ([i64; 6], i64)| [n, a[1], a[2], a[3], a[4], a[5]]).collect();
    let base = user_base(db, UserWhere::CreatedLt(year_ago())).with((&uf).filt(|a: [i64; 6]| a[0] > 5));
    let top: MatSet<Id<User>> = whole(&base)
        .select(Ident::<User>::new().and(&uf).and(&db.user.reputation))
        .window(row_number, |((_, a), r): ((Id<User>, [i64; 6]), i64)| (a[4] - a[5], r), desc)
        .filt(|(_, n)| n <= 10)
        .map(|(((u, _), _), _)| u)
        .collect();
    let mut v = Vec::new();
    (&top).select(Ident::<User>::new().and(&uf).and(badges_of(db).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((u, a), b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&[a[0], a[1], a[2], a[3]]));
        f.extend([V::F(db.user.reputation.get(u).unwrap() as f64), V::I(a[4] - a[5])]);
        f.extend(match b {
            Some(b) => vec![V::S(db.badge.name.get(b).unwrap()), V::I(db.badge.class.get(b).unwrap())],
            None => nulls(2),
        });
        row(f)
    }))
}

// WITH UserPosts AS (
// SELECT
// p.OwnerUserId,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositiveScoreCount,
// SUM(CASE WHEN p.CommentCount IS NOT NULL THEN p.CommentCount ELSE 0 END) AS TotalComments
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
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
// )
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COALESCE(up.PostCount, 0) AS PostCount,
// COALESCE(up.QuestionCount, 0) AS QuestionCount,
// COALESCE(up.AnswerCount, 0) AS AnswerCount,
// COALESCE(up.PositiveScoreCount, 0) AS PositiveScoreCount,
// COALESCE(up.TotalComments, 0) AS TotalComments,
// COALESCE(ub.BadgeCount, 0) AS BadgeCount,
// COALESCE(ub.GoldBadges, 0) AS GoldBadges,
// COALESCE(ub.SilverBadges, 0) AS SilverBadges,
// COALESCE(ub.BronzeBadges, 0) AS BronzeBadges
// FROM
// Users u
// LEFT JOIN
// UserPosts up ON u.Id = up.OwnerUserId
// LEFT JOIN
// UserBadges ub ON u.Id = ub.UserId
// ORDER BY
// u.Reputation DESC;
fn q13535(db: &'static So) -> String {
    let Post { post_type_id, score, comment_count, .. } = &db.post;
    let uf = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(score).and(comment_count)).fold([0i64; 5], |a, ((t, s), c)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (s > 0) as i64, a[4] + c]
    });
    let bc = badge_classes(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&uf).opt()).and((&bc).opt())).drive(|_, ((u, a), b)| v.push((u, a.unwrap_or([0; 5]), bz(b))));
    rows(v.iter().map(|&(u, a, b)| {
        let mut f = ["uid", "name", "rep"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.extend(ints(&a));
        f.extend(ints(&b));
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
// COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
// COUNT(DISTINCT V.Id) AS VoteCount,
// COUNT(A.Id) AS AnswerCount
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// LEFT JOIN
// Posts A ON P.Id = A.ParentId AND A.PostTypeId = 2
// WHERE
// P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.Score, P.ViewCount
// ),
// UserStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(DISTINCT P.Id) AS PostCount,
// SUM(U.UpVotes) AS TotalUpVotes,
// SUM(U.DownVotes) AS TotalDownVotes,
// COUNT(DISTINCT B.Id) AS BadgeCount
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// GROUP BY
// U.Id, U.DisplayName
// )
// SELECT
// PS.PostId,
// PS.Title,
// PS.CreationDate,
// PS.Score,
// PS.ViewCount,
// PS.CommentCount,
// PS.VoteCount,
// PS.AnswerCount,
// US.UserId,
// US.DisplayName,
// US.PostCount,
// US.TotalUpVotes,
// US.TotalDownVotes,
// US.BadgeCount
// FROM
// PostStats PS
// JOIN
// Users U ON PS.PostId = U.AccountId
// JOIN
// UserStats US ON U.Id = US.UserId
// ORDER BY
// PS.Score DESC, PS.ViewCount DESC;
fn q14257(db: &'static So) -> String {
    let acct: HashIdx<i64, Id<User>> = (&db.user.account_id).inv().collect();
    let bu = badges_per_user(db);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let ud2 = g(db)
        .select((&db.user.up_votes).and(&db.user.down_votes).and(posts_of(db).opt().and(badges_of(db).opt())))
        .fold([0i64; 2], |a, ((up, dn), _)| [a[0] + up, a[1] + dn]);
    let mut v = Vec::new();
    stats_fold(db, since(db, year_ago()), Ident::<Post>::new(), "cvA", &[])
        .and(votes_per_post(db))
        .and((&db.post.origid).select(&acct).select(Ident::<User>::new().and(&bu).and((&dp).opt()).and(&ud2)))
        .drive(|p, ((s, x), (((u, b), d), w))| v.push((p, s, x, u, b, d.unwrap_or(0), w)));
    rows(v.iter().map(|&(p, s, x, u, b, d, w)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend(ints(&[s.cx, x, s.ax]));
        f.extend([user_col(db, u, "uid"), user_col(db, u, "name")]);
        f.extend(ints(&[d, w[0], w[1], b]));
        row(f)
    }))
}

// WITH UserVoteStats AS (
// SELECT
// UserId,
// SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// COUNT(*) AS TotalVotes
// FROM Votes
// GROUP BY UserId
// ),
// PostStats AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// U.Id AS OwnerUserId,
// U.DisplayName AS OwnerDisplayName,
// COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
// COUNT(DISTINCT A.Id) AS AnswerCount,
// COALESCE(SUM(VS.UpVotes), 0) AS TotalUpVotes,
// COALESCE(SUM(VS.DownVotes), 0) AS TotalDownVotes,
// P.CreationDate,
// P.Score
// FROM Posts P
// LEFT JOIN Users U ON P.OwnerUserId = U.Id
// LEFT JOIN Comments C ON P.Id = C.PostId
// LEFT JOIN Posts A ON P.Id = A.ParentId AND A.PostTypeId = 2
// LEFT JOIN UserVoteStats VS ON U.Id = VS.UserId
// WHERE P.CreationDate >= '2020-01-01'
// GROUP BY P.Id, P.Title, U.Id, U.DisplayName, P.CreationDate, P.Score
// ),
// FinalStats AS (
// SELECT
// PS.*,
// CASE
// WHEN Score > 0 THEN 'Positive'
// WHEN Score < 0 THEN 'Negative'
// ELSE 'Neutral'
// END AS ScoreTrend
// FROM PostStats PS
// )
// SELECT
// PostId,
// Title,
// OwnerUserId,
// OwnerDisplayName,
// CommentCount,
// AnswerCount,
// TotalUpVotes,
// TotalDownVotes,
// Score,
// ScoreTrend
// FROM FinalStats
// WHERE TotalUpVotes + TotalDownVotes > 10
// ORDER BY Score DESC, CommentCount DESC
// LIMIT 100;
fn q8239(db: &'static So) -> String {
    let uv = uvotes(db);
    let ta = typed_answers_per_post(db);
    let ps = db
        .post
        .with((&db.post.creation_date).ge(date(2020, 1, 1)))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(answers_of(db).opt()).and((&db.post.owner_user).select(&uv).opt()))
        .fold([0i64; 3], |a, ((c, _), x)| {
            let x = x.unwrap_or([0; 3]);
            [a[0] + c.is_some() as i64, a[1] + x[1], a[2] + x[2]]
        });
    let mut v = Vec::new();
    (&ps).filt(|a: [i64; 3]| a[1] + a[2] > 10).and(&ta).drive(|p, (a, n)| v.push((p, a[0], n, a[1], a[2])));
    out(v, |&(p, c, ..)| (score_desc(db, p), Reverse(c)), 100, |&(p, c, a, u, d)| {
        let s = db.post.score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "owner_id", "owner"]);
        f.extend(ints(&[c, a, u, d, s]));
        f.push(V::S(if s > 0 { "Positive" } else if s < 0 { "Negative" } else { "Neutral" }));
        f
    })
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.Reputation,
// COUNT(DISTINCT P.Id) AS PostCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN C.Id IS NOT NULL THEN 1 ELSE 0 END) AS CommentCount
// FROM Users U
// LEFT JOIN Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN Comments C ON P.Id = C.PostId
// GROUP BY U.Id, U.Reputation
// ),
// BadgeStats AS (
// SELECT
// UserId,
// COUNT(*) AS BadgeCount,
// SUM(CASE WHEN Class = 1 THEN 1 ELSE 0 END) AS GoldBadgeCount,
// SUM(CASE WHEN Class = 2 THEN 1 ELSE 0 END) AS SilverBadgeCount,
// SUM(CASE WHEN Class = 3 THEN 1 ELSE 0 END) AS BronzeBadgeCount
// FROM Badges
// GROUP BY UserId
// ),
// PostStats AS (
// SELECT
// P.OwnerUserId,
// COUNT(*) AS TotalPosts,
// SUM(COALESCE(P.Score, 0)) AS TotalScore,
// AVG(P.ViewCount) AS AverageViews
// FROM Posts P
// GROUP BY P.OwnerUserId
// )
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// US.PostCount,
// US.QuestionCount,
// US.AnswerCount,
// US.CommentCount,
// BS.BadgeCount,
// BS.GoldBadgeCount,
// BS.SilverBadgeCount,
// BS.BronzeBadgeCount,
// PS.TotalPosts,
// PS.TotalScore,
// PS.AverageViews
// FROM Users U
// LEFT JOIN UserStats US ON U.Id = US.UserId
// LEFT JOIN BadgeStats BS ON U.Id = BS.UserId
// LEFT JOIN PostStats PS ON U.Id = PS.OwnerUserId
// ORDER BY U.Reputation DESC;
fn q11400(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "c", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let bc = badge_classes(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    (&us).and((&dp).opt()).and((&bc).opt()).and((&ps).opt()).drive(|u, (((a, d), b), p)| v.push((u, a, d.unwrap_or(0), b, p)));
    rows(v.iter().map(|&(u, a, d, b, p)| {
        let mut f = ["uid", "name", "rep"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.extend(ints(&[d, a.q, a.a, a.cx]));
        f.extend(b.map_or(nulls(4), |b| ints(&b)));
        f.extend(p.map_or(nulls(3), |p| vec![V::I(p[0]), V::I(p[3]), pviews_avg(p)]));
        row(f)
    }))
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
// COUNT(DISTINCT p.Id) AS PostCount,
// AVG(EXTRACT(EPOCH FROM (cast('2024-10-01 12:34:56' as timestamp) - u.CreationDate))/3600) AS AvgAgeHours
// FROM
// Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN Votes v ON p.Id = v.PostId
// GROUP BY
// u.Id, u.DisplayName
// ),
// RecentEdits AS (
// SELECT
// ph.UserId,
// COUNT(*) AS EditCount,
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
// COUNT(*) AS BadgeCount,
// MAX(b.Date) AS LastAwarded
// FROM
// Badges b
// WHERE
// b.Class = 1
// GROUP BY
// b.UserId
// )
// SELECT
// us.UserId,
// us.DisplayName,
// us.UpVotes,
// us.DownVotes,
// us.PostCount,
// us.AvgAgeHours,
// COALESCE(re.EditCount, 0) AS EditCount,
// re.LastEditDate,
// COALESCE(ub.BadgeCount, 0) AS GoldBadges,
// COALESCE(ub.LastAwarded, NULL) AS LastGoldBadgeAwarded
// FROM
// UserStats us
// LEFT JOIN RecentEdits re ON us.UserId = re.UserId
// LEFT JOIN UserBadges ub ON us.UserId = ub.UserId
// WHERE
// us.UpVotes > us.DownVotes
// ORDER BY
// us.UpVotes DESC
// LIMIT 10;
fn q1056(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let re = db.post_history.with((&db.post_history.post_history_type_id).filt(|t: i64| matches!(t, 4 | 5 | 6))).group_by(&db.post_history.user).select(&db.post_history.creation_date).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let gb = db.badge.with((&db.badge.class).eq(1)).group_by(&db.badge.user).select(&db.badge.date).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let mut v = Vec::new();
    (&us).filt(|a: UStats| a.up > a.down).and((&dp).opt()).and((&re).opt()).and((&gb).opt()).drive(|u, (((a, d), e), g)| v.push((u, a, d.unwrap_or(0), e, g)));
    out(v, |&(_, a, ..)| Reverse(a.up), 10, |&(u, a, d, e, g)| {
        let age = (t0 - db.user.creation_date.get(u).unwrap()) as f64 / 1e6 / 3600.0;
        vec![
            user_col(db, u, "uid"),
            user_col(db, u, "name"),
            V::I(a.up),
            V::I(a.down),
            V::I(d),
            V::F(age),
            V::I(e.map_or(0, |e| e.0)),
            ots(e.map(|e| e.1)),
            V::I(g.map_or(0, |g| g.0)),
            ots(g.map(|g| g.1)),
        ]
    })
}

// WITH UserReputation AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(DISTINCT P.Id) AS PostCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
// FROM Users U
// LEFT JOIN Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN Votes V ON P.Id = V.PostId
// WHERE U.Reputation > 1000
// GROUP BY U.Id, U.DisplayName, U.Reputation
// ),
// PostAnalytics AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.ViewCount,
// COALESCE(COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END), 0) AS CommentCount,
// COALESCE(SUM(CASE WHEN PH.PostId IS NOT NULL THEN 1 END), 0) AS HistoryCount
// FROM Posts P
// LEFT JOIN Comments C ON P.Id = C.PostId
// LEFT JOIN PostHistory PH ON P.Id = PH.PostId
// GROUP BY P.Id, P.Title, P.CreationDate, P.ViewCount
// )
// SELECT
// UR.UserId,
// UR.DisplayName,
// UR.Reputation,
// UR.PostCount,
// UR.QuestionCount,
// UR.AnswerCount,
// UR.Upvotes,
// UR.Downvotes,
// PA.PostId,
// PA.Title AS PostTitle,
// PA.CreationDate AS PostCreationDate,
// PA.ViewCount AS PostViewCount,
// PA.CommentCount,
// PA.HistoryCount
// FROM UserReputation UR
// JOIN PostAnalytics PA ON UR.UserId = PA.PostId
// ORDER BY UR.Reputation DESC, PA.ViewCount DESC
// LIMIT 50;
fn q5474(db: &'static So) -> String {
    let pid = pids(db);
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::RepGt(1000), "v", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let sf = stats_fold(db, db.post.iq(), Ident::<Post>::new(), "ch", &[]);
    let mut v = Vec::new();
    (&us).and((&dp).opt()).and((&db.user.origid).select(&pid).select(Ident::<Post>::new().and(&sf))).drive(|u, ((a, d), (p, s))| v.push((u, a, d.unwrap_or(0), p, s)));
    out(v, |&(u, _, _, p, _)| (rep_desc(db, u), views_desc(db, p)), 50, |&(u, a, d, p, s)| {
        let mut f = ["uid", "name", "rep"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.extend(ints(&[d, a.q, a.a, a.up, a.down]));
        f.extend(post_fields(db, p, &["id", "title", "created", "views"]));
        f.extend(ints(&[s.cx, s.hx]));
        f
    })
}

// WITH UserBadgeStats AS (
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
// QuestionStatistics AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS TotalQuestions,
// COUNT(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 END) AS AcceptedAnswers,
// AVG(EXTRACT(EPOCH FROM (p.LastActivityDate - p.CreationDate)) / 3600) AS AvgTimeToAnswer
// FROM
// Posts p
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.OwnerUserId
// ),
// EnhancedUserStats AS (
// SELECT
// u.UserId,
// u.DisplayName,
// u.TotalBadges,
// q.TotalQuestions,
// q.AcceptedAnswers,
// q.AvgTimeToAnswer
// FROM
// UserBadgeStats u
// LEFT JOIN
// QuestionStatistics q ON u.UserId = q.OwnerUserId
// )
// SELECT
// eus.DisplayName,
// eus.TotalBadges,
// eus.TotalQuestions,
// eus.AcceptedAnswers,
// eus.AvgTimeToAnswer,
// COALESCE(eus.TotalQuestions * 1.0 / NULLIF(eus.AcceptedAnswers, 0), 0) AS QuestionsToAcceptedRatio,
// COALESCE(eus.AvgTimeToAnswer, 0) AS AverageTimeToFirstAnswerInHours
// FROM
// EnhancedUserStats eus
// ORDER BY
// eus.TotalBadges DESC,
// eus.TotalQuestions DESC;
fn q25142(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let Post { accepted_answer_id, last_activity_date, creation_date, .. } = &db.post;
    let qs = questions_only(db).group_by(&db.post.owner_user).select(accepted_answer_id.opt().and(last_activity_date).and(creation_date)).fold(([0i64; 2], 0i128), |(a, e), ((acc, l), c)| {
        ([a[0] + 1, a[1] + acc.is_some() as i64], e + (l - c) as i128)
    });
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and(&bu).and((&qs).opt())).drive(|_, ((u, b), q)| v.push((u, b, q)));
    rows(v.iter().map(|&(u, b, q)| {
        let mut f = vec![user_col(db, u, "name"), V::I(b)];
        match q {
            Some((a, e)) => {
                let av = e as f64 / a[0] as f64 / 1e6 / 3600.0;
                f.extend([V::I(a[0]), V::I(a[1]), V::F(av), V::F(if a[1] == 0 { 0.0 } else { a[0] as f64 / a[1] as f64 }), V::F(av)]);
            }
            None => f.extend([V::Null, V::Null, V::Null, V::F(0.0), V::F(0.0)]),
        }
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
// u.Id,
// u.DisplayName
// ),
// PostStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersCount,
// COUNT(DISTINCT c.Id) AS CommentsCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY
// p.OwnerUserId
// ),
// UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// ub.BadgeCount,
// ps.PostCount,
// ps.QuestionsCount,
// ps.AnswersCount,
// ps.CommentsCount
// FROM
// Users u
// LEFT JOIN
// UserBadges ub ON u.Id = ub.UserId
// LEFT JOIN
// PostStats ps ON u.Id = ps.OwnerUserId
// )
// SELECT
// ups.UserId,
// ups.DisplayName,
// COALESCE(ups.BadgeCount, 0) AS BadgeCount,
// COALESCE(ups.PostCount, 0) AS PostCount,
// COALESCE(ups.QuestionsCount, 0) AS QuestionsCount,
// COALESCE(ups.AnswersCount, 0) AS AnswersCount,
// COALESCE(ups.CommentsCount, 0) AS CommentsCount
// FROM
// UserPostStats ups
// ORDER BY
// ups.BadgeCount DESC,
// ups.PostCount DESC,
// ups.QuestionsCount DESC
// LIMIT 100;
fn q9963(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let dc = owned_since(db, year_ago()).group_by(&db.post.owner_user).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let pr = owned_since(db, year_ago()).group_by(&db.post.owner_user).select((&db.post.post_type_id).and(comments_of(db).opt())).fold([0i64; 4], |a, (t, _)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, 0]
    });
    let ps: HashIdx<Id<User>, [i64; 4]> = (&pr).and((&dc).opt()).map(|(a, c): ([i64; 4], Option<i64>)| [a[0], a[1], a[2], c.unwrap_or(0)]).collect();
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and(&bu).and((&ps).opt())).drive(|_, ((u, b), p)| v.push((u, b, p.unwrap_or([0; 4]))));
    out(v, |&(_, b, p)| (Reverse(b), Reverse(p[0]), Reverse(p[1])), 100, |&(u, b, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(b)];
        f.extend(ints(&p));
        f
    })
}

// WITH PostStatistics AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.Score,
// p.ViewCount,
// COUNT(DISTINCT c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// p.CreationDate,
// p.LastActivityDate,
// COALESCE(p.AcceptedAnswerId, 0) AS AcceptedAnswerId,
// COALESCE(b.UserId, 0) AS BadgeOwnerId
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Badges b ON b.UserId = p.OwnerUserId
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 YEAR'
// GROUP BY
// p.Id, p.Title, p.Score, p.ViewCount, p.CreationDate, p.LastActivityDate, p.AcceptedAnswerId, b.UserId
// ),
// UserReputation AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// COUNT(b.Id) AS BadgeCount
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// u.Id, u.Reputation
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.Score,
// ps.ViewCount,
// ps.CommentCount,
// ps.UpVotes,
// ps.DownVotes,
// ps.CreationDate,
// ps.LastActivityDate,
// ps.AcceptedAnswerId,
// ur.Reputation AS OwnerReputation,
// ur.BadgeCount
// FROM
// PostStatistics ps
// JOIN
// Users u ON ps.BadgeOwnerId = u.Id
// JOIN
// UserReputation ur ON u.Id = ur.UserId
// ORDER BY
// ps.Score DESC, ps.ViewCount DESC
// LIMIT 100;
fn q7667(db: &'static So) -> String {
    let uid = uids(db);
    let bu = badges_per_user(db);
    let cp = comments_per_post(db);
    let pv = since(db, year_ago())
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and((&db.post.owner_user).select(badges_of(db)).opt()))
        .fold([0i64; 3], |a, ((_, t), _)| [0, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let bo = (&db.post.owner_user).and((&db.post.owner_user).select(&bu)).filt(|(_, b): (Id<User>, i64)| b > 0).map(|(u, _): (Id<User>, i64)| db.user.origid.get(u).unwrap());
    let mut v = Vec::new();
    since(db, year_ago())
        .select(Ident::<Post>::new().and(&cp).and((&pv).opt()).and(bo.opt().map(|o: Option<i64>| o.unwrap_or(0)).select(&uid).select(Ident::<User>::new().and(&bu))))
        .drive(|_, x| v.push(x));
    out(v, |&(((p, _), _), _)| (score_views(db, p), db.post.origid.get(p).unwrap()), 100, |&(((p, c), x), (u, b))| {
        let x = x.unwrap_or([0; 3]);
        let mut f = post_fields(db, p, &["id", "title", "score", "views"]);
        f.extend(ints(&[c, x[1], x[2]]));
        f.extend(post_fields(db, p, &["created", "activity"]));
        f.push(V::I(db.post.accepted_answer_id.get(p).unwrap_or(0)));
        f.extend([user_col(db, u, "rep"), V::I(b)]);
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
// COUNT(c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount,
// COUNT(DISTINCT b.Id) AS BadgeCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Badges b ON p.OwnerUserId = b.UserId
// WHERE
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score
// ),
// UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(u.UpVotes) AS TotalUpVotes,
// SUM(u.DownVotes) AS TotalDownVotes,
// AVG(u.Reputation) AS AverageReputation
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName
// )
// SELECT
// ps.Title,
// ps.ViewCount,
// ps.Score,
// ps.CommentCount,
// ps.VoteCount,
// ps.UpVoteCount,
// ps.DownVoteCount,
// us.DisplayName AS OwnerDisplayName,
// us.PostCount,
// us.TotalUpVotes,
// us.TotalDownVotes,
// us.AverageReputation
// FROM
// PostStats ps
// JOIN
// UserStats us ON ps.PostId = us.UserId
// ORDER BY
// ps.ViewCount DESC
// LIMIT 100;
fn q12552(db: &'static So) -> String {
    let uid = uids(db);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let uvs = g(db).select((&db.user.up_votes).and(&db.user.down_votes).and(posts_of(db).opt())).fold([0i64; 2], |a, ((up, dn), _)| [a[0] + up, a[1] + dn]);
    let mut v = Vec::new();
    stats_fold(db, since(db, year_ago()), Ident::<Post>::new(), "cvb", &[])
        .and(votes_per_post(db))
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and((&dp).opt()).and(&uvs)))
        .drive(|p, ((s, x), ((u, d), w))| v.push((p, s, x, u, d.unwrap_or(0), w)));
    out(v, |&(p, ..)| views_desc(db, p), 100, |&(p, s, x, u, d, w)| {
        let mut f = post_fields(db, p, &["title", "views", "score"]);
        f.extend(ints(&[s.cx, x, s.up, s.down]));
        f.extend([user_col(db, u, "name"), V::I(d), V::I(w[0]), V::I(w[1]), V::F(db.user.reputation.get(u).unwrap() as f64)]);
        f
    })
}

// WITH UserReputation AS (
// SELECT Id, Reputation, CreationDate, DisplayName, LastAccessDate, Views
// FROM Users
// ),
// PostStatistics AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate AS PostCreationDate,
// P.Score,
// P.ViewCount,
// COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS TotalComments,
// COUNT(CASE WHEN A.Id IS NOT NULL THEN 1 END) AS TotalAnswers,
// P.OwnerUserId
// FROM Posts P
// LEFT JOIN Comments C ON C.PostId = P.Id
// LEFT JOIN Posts A ON A.ParentId = P.Id AND A.PostTypeId = 2
// GROUP BY P.Id, P.Title, P.CreationDate, P.Score, P.ViewCount, P.OwnerUserId
// ),
// TopActiveUsers AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(DISTINCT P.Id) AS TotalPosts,
// SUM(COALESCE(V.BountyAmount, 0)) AS TotalBounties
// FROM Users U
// JOIN Posts P ON P.OwnerUserId = U.Id
// LEFT JOIN Votes V ON V.UserId = U.Id AND V.PostId = P.Id
// GROUP BY U.Id, U.DisplayName, U.Reputation
// ORDER BY TotalPosts DESC
// LIMIT 10
// )
// SELECT
// UA.UserId,
// UA.DisplayName,
// UA.Reputation,
// PS.PostId,
// PS.Title,
// PS.PostCreationDate,
// PS.Score,
// PS.ViewCount,
// PS.TotalComments,
// PS.TotalAnswers,
// COALESCE(TBA.TotalBounties, 0) AS TotalBounties
// FROM TopActiveUsers UA
// JOIN PostStatistics PS ON UA.UserId = PS.OwnerUserId
// LEFT JOIN (
// SELECT
// UserId,
// SUM(BountyAmount) AS TotalBounties
// FROM Votes
// GROUP BY UserId
// ) TBA ON TBA.UserId = UA.UserId
// ORDER BY UA.Reputation DESC;
fn q10687(db: &'static So) -> String {
    let dp: &'static Fold<Id<User>, i64> = Box::leak(Box::new(owned(db).group_by(&db.post.owner_user).select(&db.post.score).fold(0i64, |a, _| a + 1)));
    let top = top_by(dp, 10);
    let tb = db.vote.group_by(&db.vote.user).select((&db.vote.bounty_amount).opt()).fold(0i64, |a, b| a + b.unwrap_or(0));
    let ps = owned(db)
        .with((&db.post.owner_user).select(&top))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(answers_of(db).opt()))
        .fold([0i64; 2], |a, (c, x)| [a[0] + c.is_some() as i64, a[1] + x.is_some() as i64]);
    let mut v = Vec::new();
    (&ps).and((&db.post.owner_user).select(Ident::<User>::new().and((&tb).opt()))).drive(|p, x| v.push((p, x)));
    rows(v.iter().map(|&(p, ([c, a], (u, b)))| {
        let mut f = ["uid", "name", "rep"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.extend(post_fields(db, p, &["id", "title", "created", "score", "views"]));
        f.extend(ints(&[c, a, b.unwrap_or(0)]));
        row(f)
    }))
}

// WITH UserVoteSummary AS (
// SELECT
// u.Id AS UserId,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes,
// COUNT(DISTINCT p.Id) AS TotalPosts
// FROM Users u
// LEFT JOIN Votes v ON u.Id = v.UserId
// LEFT JOIN Posts p ON v.PostId = p.Id
// GROUP BY u.Id
// ),
// PostAcceptance AS (
// SELECT
// p.Id AS PostId,
// COUNT(DISTINCT CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN p.AcceptedAnswerId END) AS AcceptedAnswers
// FROM Posts p
// GROUP BY p.Id
// ),
// PostDetails AS (
// SELECT
// p.Id,
// p.Title,
// p.CreationDate,
// COALESCE(ps.TotalUpVotes, 0) AS UserUpVotes,
// COALESCE(ps.TotalDownVotes, 0) AS UserDownVotes,
// COALESCE(pa.AcceptedAnswers, 0) AS AcceptedAnswersCount
// FROM Posts p
// LEFT JOIN UserVoteSummary ps ON p.OwnerUserId = ps.UserId
// LEFT JOIN PostAcceptance pa ON p.Id = pa.PostId
// )
// SELECT
// pd.Id,
// pd.Title,
// pd.CreationDate,
// pd.UserUpVotes,
// pd.UserDownVotes,
// pd.AcceptedAnswersCount,
// CASE
// WHEN pd.UserUpVotes > pd.UserDownVotes THEN 'Positive'
// WHEN pd.UserDownVotes > pd.UserUpVotes THEN 'Negative'
// ELSE 'Neutral'
// END AS VoteSentiment,
// CASE
// WHEN pd.AcceptedAnswersCount > 0 THEN 'Accepted'
// ELSE 'Not Accepted'
// END AS AcceptanceStatus
// FROM PostDetails pd
// WHERE pd.CreationDate >= '2021-01-01'
// AND (pd.UserUpVotes - pd.UserDownVotes) > 0
// ORDER BY pd.UserUpVotes DESC, pd.CreationDate DESC
// LIMIT 100;
fn q1299(db: &'static So) -> String {
    let uv = db.vote.group_by(&db.vote.user).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let mut v = Vec::new();
    db.post
        .with((&db.post.creation_date).ge(date(2021, 1, 1)))
        .select(Ident::<Post>::new().and((&db.post.owner_user).select(&uv).opt()))
        .filt(|(_, x): (Id<Post>, Option<[i64; 2]>)| x.map_or(0, |x| x[0] - x[1]) > 0)
        .drive(|_, (p, x)| v.push((p, x.unwrap())));
    out(v, |&(p, x)| (Reverse(x[0]), Reverse(db.post.creation_date.get(p).unwrap())), 100, |&(p, x)| {
        let acc = db.post.accepted_answer_id.get(p).is_some() as i64;
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend(ints(&[x[0], x[1], acc]));
        f.extend([V::S("Positive"), V::S(if acc > 0 { "Accepted" } else { "Not Accepted" })]);
        f
    })
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(DISTINCT P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(CASE WHEN P.PostTypeId = 1 AND P.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// U.Id, U.DisplayName, U.Reputation
// ), BadgeCounts AS (
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
// )
// SELECT
// US.UserId,
// US.DisplayName,
// US.Reputation,
// US.TotalPosts,
// US.Questions,
// US.Answers,
// US.AcceptedAnswers,
// US.UpVotes,
// US.DownVotes,
// COALESCE(BC.TotalBadges, 0) AS TotalBadges,
// COALESCE(BC.GoldBadges, 0) AS GoldBadges,
// COALESCE(BC.SilverBadges, 0) AS SilverBadges,
// COALESCE(BC.BronzeBadges, 0) AS BronzeBadges
// FROM
// UserStats US
// LEFT JOIN
// BadgeCounts BC ON US.UserId = BC.UserId
// ORDER BY
// US.Reputation DESC,
// US.TotalPosts DESC
// LIMIT 100;
fn q7946(db: &'static So) -> String {
    let Post { post_type_id, accepted_answer_id, .. } = &db.post;
    let uf = owned(db)
        .group_by(&db.post.owner_user)
        .select(post_type_id.and(accepted_answer_id.opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 5], |a, ((t, acc), v)| {
            [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (t == 1 && acc.is_some()) as i64, a[3] + (v == Some(2)) as i64, a[4] + (v == Some(3)) as i64]
        });
    let dp = ud(db, UserWhere::All, posts_of(db));
    let bc = badge_classes(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&uf).opt()).and((&dp).opt()).and((&bc).opt())).drive(|_, (((u, a), d), b)| v.push((u, a.unwrap_or([0; 5]), d.unwrap_or(0), bz(b))));
    out(v, |&(u, _, d, _)| (rep_desc(db, u), Reverse(d)), 100, |&(u, a, d, b)| {
        let mut f = ["uid", "name", "rep"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.push(V::I(d));
        f.extend(ints(&a));
        f.extend(ints(&b));
        f
    })
}

// WITH UserScore AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) - SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS NetVotes,
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
// PostEngagement AS (
// SELECT
// p.Id AS PostId,
// p.OwnerUserId,
// COUNT(c.Id) AS CommentCount,
// COUNT(DISTINCT p2.Id) AS RelatedPosts
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// PostLinks pl ON p.Id = pl.PostId
// LEFT JOIN
// Posts p2 ON pl.RelatedPostId = p2.Id
// WHERE
// p.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days')
// GROUP BY
// p.Id, p.OwnerUserId
// )
// SELECT
// us.UserId,
// us.DisplayName,
// us.NetVotes,
// us.GoldBadges,
// us.SilverBadges,
// us.BronzeBadges,
// COALESCE(pe.CommentCount, 0) AS CommentCount,
// COALESCE(pe.RelatedPosts, 0) AS RelatedPosts
// FROM
// UserScore us
// LEFT JOIN
// PostEngagement pe ON us.UserId = pe.OwnerUserId
// WHERE
// us.NetVotes > 10
// AND (us.GoldBadges > 0 OR us.SilverBadges > 2)
// ORDER BY
// us.NetVotes DESC, us.DisplayName ASC
// LIMIT 50;
fn q3112(db: &'static So) -> String {
    let us = g(db).select(votes_by(db).select(&db.vote.vote_type_id).opt().and(badges_of(db).select(&db.badge.class).opt())).fold([0i64; 5], |a, (t, c)| {
        [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + (c == Some(1)) as i64, a[3] + (c == Some(2)) as i64, a[4] + (c == Some(3)) as i64]
    });
    let dl = db.post_link.with(&db.post_link.related_post).group_by(&db.post_link.post).select(&db.post_link.related_post).count_distinct();
    let pe = db
        .post
        .with((&db.post.creation_date).ge(month_ago()))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(links_of(db).opt()))
        .fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let mut v = Vec::new();
    db.user
        .select(Ident::<User>::new().and((&us).filt(|a: [i64; 5]| a[0] - a[1] > 10 && (a[2] > 0 || a[3] > 2))).and(posts_of(db).with((&db.post.creation_date).ge(month_ago())).select(Ident::<Post>::new().and(&pe).and((&dl).opt())).opt()))
        .drive(|_, ((u, a), p)| v.push((u, a[0] - a[1], [a[2], a[3], a[4]], p)));
    out(v, |&(u, n, ..)| (Reverse(n), db.user.display_name.get(u).unwrap()), 50, |&(u, n, g, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(n)];
        f.extend(ints(&g));
        f.extend(match p {
            Some(((_, c), d)) => ints(&[c, d.unwrap_or(0)]),
            None => ints(&[0, 0]),
        });
        f
    })
}

// WITH UserReputation AS (
// SELECT
// u.Id AS UserId,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// COUNT(DISTINCT p.Id) AS PostCount,
// COALESCE(SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END), 0) AS QuestionCount,
// COALESCE(SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS AnswerCount,
// COALESCE(SUM(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END), 0) AS AcceptedAnswerCount,
// u.Reputation
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// u.Reputation > 1000
// GROUP BY
// u.Id, u.Reputation
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
// )
// SELECT
// ur.UserId,
// u.DisplayName,
// ur.UpVotes,
// ur.DownVotes,
// ur.PostCount,
// ur.QuestionCount,
// ur.AnswerCount,
// ur.AcceptedAnswerCount,
// COALESCE(ub.GoldBadges, 0) AS GoldBadges,
// COALESCE(ub.SilverBadges, 0) AS SilverBadges,
// COALESCE(ub.BronzeBadges, 0) AS BronzeBadges,
// (ur.UpVotes - ur.DownVotes) AS NetVotes
// FROM
// UserReputation ur
// JOIN
// Users u ON ur.UserId = u.Id
// LEFT JOIN
// UserBadges ub ON ur.UserId = ub.UserId
// ORDER BY
// NetVotes DESC, ur.Reputation DESC
// LIMIT 10;
fn q5914(db: &'static So) -> String {
    let Post { post_type_id, accepted_answer_id, .. } = &db.post;
    let uf = owned(db)
        .group_by(&db.post.owner_user)
        .select(post_type_id.and(accepted_answer_id.opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 5], |a, ((t, acc), v)| {
            [a[0] + (v == Some(2)) as i64, a[1] + (v == Some(3)) as i64, a[2] + (t == 1) as i64, a[3] + (t == 2) as i64, a[4] + acc.is_some() as i64]
        });
    let dp = ud(db, UserWhere::All, posts_of(db));
    let bc = badge_classes(db);
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and((&uf).opt()).and((&dp).opt()).and((&bc).opt())).drive(|_, (((u, a), d), b)| v.push((u, a.unwrap_or([0; 5]), d.unwrap_or(0), bz(b))));
    out(v, |&(u, a, ..)| (Reverse(a[0] - a[1]), rep_desc(db, u)), 10, |&(u, a, d, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&[a[0], a[1], d, a[2], a[3], a[4], b[1], b[2], b[3], a[0] - a[1]]));
        f
    })
}

// WITH FilteredPosts AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.Body,
// p.Tags,
// p.CreationDate,
// u.DisplayName AS OwnerName,
// u.Reputation,
// COUNT(DISTINCT c.Id) AS CommentCount,
// COUNT(DISTINCT a.Id) AS AnswerCount,
// pt.Name AS PostTypeName
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON c.PostId = p.Id
// LEFT JOIN
// Posts a ON a.ParentId = p.Id AND a.PostTypeId = 2
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// WHERE
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY
// p.Id, p.Title, p.Body, p.Tags, p.CreationDate, u.DisplayName, u.Reputation, pt.Name
// )
// SELECT
// fp.PostId,
// fp.Title,
// fp.Body,
// fp.Tags,
// fp.CreationDate,
// fp.OwnerName,
// fp.Reputation,
// fp.CommentCount,
// fp.AnswerCount,
// CASE
// WHEN fp.Reputation > 1000 THEN 'High Reputation'
// WHEN fp.Reputation BETWEEN 500 AND 1000 THEN 'Medium Reputation'
// ELSE 'Low Reputation'
// END AS ReputationCategory,
// STRING_AGG(DISTINCT fp.PostTypeName, ', ') AS PostTypeNames
// FROM
// FilteredPosts fp
// JOIN
// PostHistory ph ON ph.PostId = fp.PostId
// WHERE
// ph.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'
// AND ph.PostHistoryTypeId IN (10, 11)
// GROUP BY
// fp.PostId, fp.Title, fp.Body, fp.Tags, fp.CreationDate, fp.OwnerName, fp.Reputation, fp.CommentCount, fp.AnswerCount
// ORDER BY
// fp.CreationDate DESC, fp.Reputation DESC;
fn q25144(db: &'static So) -> String {
    let cp = comments_per_post(db);
    let ta = typed_answers_per_post(db);
    let m = month_ago();
    let ph = db.post_history.with((&db.post_history.creation_date).ge(m)).with((&db.post_history.post_history_type_id).filt(|t: i64| matches!(t, 10 | 11))).group_by(&db.post_history.post).select(&db.post_history.post).fold(0i64, |a, _| a + 1);
    let mut v = Vec::new();
    owned_since(db, year_ago()).select(Ident::<Post>::new().and(&cp).and(&ta).and(&ph)).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(((p, c), a), _)| {
        let r = db.post.owner_user.get(p).map(|u| db.user.reputation.get(u).unwrap()).unwrap();
        let cat = if r > 1000 { "High Reputation" } else if (500..=1000).contains(&r) { "Medium Reputation" } else { "Low Reputation" };
        let mut f = post_fields(db, p, &["id", "title", "body", "tags", "created", "owner", "rep"]);
        f.extend([V::I(c), V::I(a), V::S(cat)]);
        f.extend(post_fields(db, p, &["type"]));
        row(f)
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("20652", q20652),
    ("9141", q9141),
    ("20370", q20370),
    ("24499", q24499),
    ("30104", q30104),
    ("1673", q1673),
    ("11874", q11874),
    ("22390", q22390),
    ("31268", q31268),
    ("7651", q7651),
    ("21533", q21533),
    ("5786", q5786),
    ("20237", q20237),
    ("6652", q6652),
    ("24954", q24954),
    ("11718", q11718),
    ("14530", q14530),
    ("10495", q10495),
    ("11846", q11846),
    ("10472", q10472),
    ("10885", q10885),
    ("11930", q11930),
    ("14854", q14854),
    ("14571", q14571),
    ("13160", q13160),
    ("13809", q13809),
    ("11482", q11482),
    ("12343", q12343),
    ("14371", q14371),
    ("11435", q11435),
    ("3160", q3160),
    ("12871", q12871),
    ("14642", q14642),
    ("9655", q9655),
    ("11950", q11950),
    ("13765", q13765),
    ("6314", q6314),
    ("7906", q7906),
    ("11871", q11871),
    ("13094", q13094),
    ("13626", q13626),
    ("9530", q9530),
    ("6903", q6903),
    ("6275", q6275),
    ("5518", q5518),
    ("9175", q9175),
    ("13535", q13535),
    ("14257", q14257),
    ("8239", q8239),
    ("11400", q11400),
    ("1056", q1056),
    ("5474", q5474),
    ("25142", q25142),
    ("9963", q9963),
    ("7667", q7667),
    ("12552", q12552),
    ("10687", q10687),
    ("1299", q1299),
    ("7946", q7946),
    ("3112", q3112),
    ("5914", q5914),
    ("25144", q25144),
];
