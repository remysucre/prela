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

/// Per user over `Users LEFT JOIN Badges LEFT JOIN Posts` (the product): [rows,
/// rows with a badge, rows with a post, score sum, views present, views sum].
fn user_bp<Q: Drive<D = Id<User>, R = Id<User>>>(db: &'static So, users: Q) -> Fold<Id<User>, [i64; 6]> {
    users
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(posts_of(db).select((&db.post.score).and((&db.post.view_count).opt())).opt()))
        .fold([0i64; 6], |a, (b, p)| {
            let (n, s, w) = p.map_or((0, 0, None), |(s, w)| (1, s, w));
            [a[0] + 1, a[1] + b.is_some() as i64, a[2] + n, a[3] + s, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0)]
        })
}

/// Per user over `Users LEFT JOIN Posts LEFT JOIN Badges` (the product): [rows with a badge, SUM(b.Class)].
fn user_class_rows(db: &'static So) -> Fold<Id<User>, [i64; 2]> {
    db.user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 2], |a, (_, c)| [a[0] + c.is_some() as i64, a[1] + c.unwrap_or(0)])
}

/// Per user over `Users LEFT JOIN Posts`: [rows, rows with a post, SUM(u.UpVotes), SUM(u.DownVotes), SUM(u.Reputation)].
fn user_left_posts(db: &'static So) -> Fold<Id<User>, [i64; 5]> {
    db.user
        .group_by(Ident::<User>::new())
        .select((&db.user.up_votes).and(&db.user.down_votes).and(&db.user.reputation).and(posts_of(db).opt()))
        .fold([0i64; 5], |a, (((u, d), r), p)| [a[0] + 1, a[1] + p.is_some() as i64, a[2] + u, a[3] + d, a[4] + r])
}

/// Per user over `Users LEFT JOIN Badges LEFT JOIN Votes ON u.Id = v.UserId` (the
/// product): [rows with a badge, up rows, down rows, bounty sum].
fn user_bv(db: &'static So) -> Fold<Id<User>, [i64; 4]> {
    db.user
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(votes_by(db).select((&db.vote.vote_type_id).and((&db.vote.bounty_amount).opt())).opt()))
        .fold([0i64; 4], |a, (b, v)| {
            let (t, x) = v.map_or((0, None), |(t, x)| (t, x));
            [a[0] + b.is_some() as i64, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64, a[3] + x.unwrap_or(0)]
        })
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

// --- batch 136 --------------------------------------------------------------

// WITH UserStatistics AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN p.PostTypeId = 3 THEN 1 ELSE 0 END) AS WikiCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes,
// ROUND(AVG(COALESCE(p.Score, 0)), 2) AS AverageScore
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN Votes v ON p.Id = v.PostId
// WHERE u.Reputation > 1000
// GROUP BY u.Id, u.DisplayName, u.Reputation
// ),
// UserBadges AS (
// SELECT
// b.UserId,
// COUNT(*) AS BadgeCount,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM Badges b
// GROUP BY b.UserId
// )
// SELECT
// us.UserId,
// us.DisplayName,
// us.Reputation,
// us.PostCount,
// us.QuestionCount,
// us.AnswerCount,
// us.WikiCount,
// ub.BadgeCount,
// ub.GoldBadges,
// ub.SilverBadges,
// ub.BronzeBadges,
// us.Upvotes,
// us.Downvotes,
// us.AverageScore
// FROM UserStatistics us
// LEFT JOIN UserBadges ub ON us.UserId = ub.UserId
// ORDER BY us.Reputation DESC, us.PostCount DESC
// LIMIT 100;
fn q9851(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::RepGt(1000), "v", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&us).and((&dp).opt()).and((&bc).opt()).drive(|u, ((a, d), b)| v.push((u, a, d.unwrap_or(0), b)));
    out(v, |&(u, _, d, _)| (rep_desc(db, u), Reverse(d)), 100, |&(u, a, d, b)| {
        let mut f = ["uid", "name", "rep"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.extend(ints(&[d, a.q, a.a, a.t3]));
        f.extend(b.map_or(nulls(4), |b| ints(&b)));
        f.extend([V::I(a.up), V::I(a.down), V::F(round2(a.score_sum as f64 / a.rows as f64))]);
        f
    })
}

// WITH UserActivity AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes,
// COUNT(DISTINCT CASE WHEN p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' THEN p.Id END) AS RecentPostsCount,
// MAX(u.Reputation) AS MaxReputation
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// u.Id, u.DisplayName
// ),
// EffectiveUserActivity AS (
// SELECT
// UserId,
// DisplayName,
// QuestionCount,
// AnswerCount,
// Upvotes,
// Downvotes,
// RecentPostsCount,
// MaxReputation,
// (QuestionCount + AnswerCount) AS TotalPosts,
// (Upvotes - Downvotes) AS NetVotes
// FROM
// UserActivity
// )
// SELECT
// eua.DisplayName,
// eua.TotalPosts,
// eua.NetVotes,
// eua.MaxReputation,
// CASE
// WHEN eua.MaxReputation > 2000 THEN 'Expert'
// WHEN eua.MaxReputation BETWEEN 1000 AND 2000 THEN 'Experienced'
// ELSE 'Novice'
// END AS UserLevel
// FROM
// EffectiveUserActivity eua
// WHERE
// eua.RecentPostsCount > 0
// ORDER BY
// eua.MaxReputation DESC,
// eua.TotalPosts DESC
// LIMIT 10;
fn q7894(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let rc = owned_since(db, month_ago()).group_by(&db.post.owner_user).select(&db.post.score).fold(0i64, |a, _| a + 1);
    let mut v = Vec::new();
    (&us).and(&rc).drive(|u, (a, _)| v.push((u, a)));
    out(v, |&(u, a)| (rep_desc(db, u), Reverse(a.q + a.a)), 10, |&(u, a)| {
        let r = db.user.reputation.get(u).unwrap();
        let lvl = if r > 2000 { "Expert" } else if (1000..=2000).contains(&r) { "Experienced" } else { "Novice" };
        vec![user_col(db, u, "name"), V::I(a.q + a.a), V::I(a.up - a.down), V::I(r), V::S(lvl)]
    })
}

// WITH PostMetrics AS (
// SELECT
// p.Id AS PostId,
// p.PostTypeId,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// p.AnswerCount,
// p.CommentCount,
// p.FavoriteCount,
// COALESCE(p.ClosedDate, '1900-01-01') AS ClosedDate,
// COALESCE(p.LastActivityDate, '1900-01-01') AS LastActivityDate,
// EXTRACT(EPOCH FROM (cast('2024-10-01 12:34:56' as timestamp) - p.CreationDate)) AS PostAgeInSeconds
// FROM
// Posts p
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// ), VoteMetrics AS (
// SELECT
// v.PostId,
// COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes,
// COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes,
// COUNT(CASE WHEN v.VoteTypeId = 6 THEN 1 END) AS CloseVotes,
// COUNT(CASE WHEN v.VoteTypeId = 7 THEN 1 END) AS ReopenVotes
// FROM
// Votes v
// GROUP BY
// v.PostId
// )
// SELECT
// pm.PostId,
// pm.PostTypeId,
// pm.CreationDate,
// pm.ViewCount,
// pm.Score,
// pm.AnswerCount,
// pm.CommentCount,
// pm.FavoriteCount,
// pm.ClosedDate,
// pm.LastActivityDate,
// pm.PostAgeInSeconds,
// COALESCE(vm.UpVotes, 0) AS UpVotes,
// COALESCE(vm.DownVotes, 0) AS DownVotes,
// COALESCE(vm.CloseVotes, 0) AS CloseVotes,
// COALESCE(vm.ReopenVotes, 0) AS ReopenVotes
// FROM
// PostMetrics pm
// LEFT JOIN
// VoteMetrics vm ON pm.PostId = vm.PostId
// ORDER BY
// pm.Score DESC, pm.ViewCount DESC;
fn q14638(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let vf = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 4], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64, a[2] + (t == 6) as i64, a[3] + (t == 7) as i64]);
    let mut v = Vec::new();
    since(db, year_ago()).select(Ident::<Post>::new().and((&vf).opt())).drive(|_, (p, x)| v.push((p, x.unwrap_or([0; 4]))));
    rows(v.iter().map(|&(p, x)| {
        let mut f = post_fields(db, p, &["id", "type_id", "created", "views", "score", "answers", "comments", "favorites"]);
        f.push(V::T(db.post.closed_date.get(p).unwrap_or(ts(1900, 1, 1, 0, 0, 0))));
        f.extend(post_fields(db, p, &["activity"]));
        f.push(V::F((t0 - db.post.creation_date.get(p).unwrap()) as f64 / 1e6));
        f.extend(ints(&x));
        row(f)
    }))
}

// WITH PostVoteCount AS (
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
// UserPostsStats AS (
// SELECT
// U.Id AS UserId,
// COUNT(DISTINCT P.Id) AS PostCount,
// COALESCE(SUM(PVC.VoteCount), 0) AS TotalVotes,
// COALESCE(SUM(PVC.UpVotes), 0) AS TotalUpVotes,
// COALESCE(SUM(PVC.DownVotes), 0) AS TotalDownVotes
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// PostVoteCount PVC ON P.Id = PVC.PostId
// GROUP BY
// U.Id
// ),
// UserBadgeStats AS (
// SELECT
// U.Id AS UserId,
// COUNT(B.Id) AS BadgeCount
// FROM
// Users U
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// GROUP BY
// U.Id
// )
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COALESCE(UPS.PostCount, 0) AS PostCount,
// COALESCE(UPS.TotalVotes, 0) AS TotalVotes,
// COALESCE(UPS.TotalUpVotes, 0) AS TotalUpVotes,
// COALESCE(UPS.TotalDownVotes, 0) AS TotalDownVotes,
// COALESCE(UBS.BadgeCount, 0) AS BadgeCount,
// U.Reputation,
// U.CreationDate,
// U.LastAccessDate
// FROM
// Users U
// LEFT JOIN
// UserPostsStats UPS ON U.Id = UPS.UserId
// LEFT JOIN
// UserBadgeStats UBS ON U.Id = UBS.UserId
// ORDER BY
// U.Reputation DESC;
fn q13194(db: &'static So) -> String {
    let pv = post_votes(db);
    let uf = owned(db).group_by(&db.post.owner_user).select((&pv).opt()).fold([0i64; 4], |a, x| {
        let x = x.unwrap_or([0; 3]);
        [a[0] + 1, a[1] + x[0], a[2] + x[1], a[3] + x[2]]
    });
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&uf).opt()).and(&bu)).drive(|_, ((u, a), b)| v.push((u, a.unwrap_or([0; 4]), b)));
    rows(v.iter().map(|&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a));
        f.push(V::I(b));
        f.extend(["rep", "ucreated", "last_access"].iter().map(|k| user_col(db, u, k)));
        row(f)
    }))
}

// WITH UserVotes AS (
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
// ActiveUsers AS (
// SELECT
// u.Id,
// u.DisplayName,
// u.Reputation,
// u.LastAccessDate,
// u.CreationDate,
// COALESCE(uv.TotalVotes, 0) AS TotalVotes,
// COALESCE(uv.UpVotes, 0) AS UpVotes,
// COALESCE(uv.DownVotes, 0) AS DownVotes
// FROM
// Users u
// LEFT JOIN
// UserVotes uv ON u.Id = uv.UserId
// WHERE
// u.LastAccessDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// ),
// PostStatistics AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(CASE WHEN p.PostTypeId = 10 THEN 1 ELSE 0 END) AS ClosedPosts
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// )
// SELECT
// au.DisplayName,
// au.Reputation,
// au.TotalVotes,
// au.UpVotes,
// au.DownVotes,
// ps.TotalPosts,
// ps.Questions,
// ps.Answers,
// ps.ClosedPosts
// FROM
// ActiveUsers au
// LEFT JOIN
// PostStatistics ps ON au.Id = ps.OwnerUserId
// ORDER BY
// au.Reputation DESC, au.TotalVotes DESC;
fn q14648(db: &'static So) -> String {
    let vn = vote_named(db);
    let ps = owned(db).group_by(&db.post.owner_user).select(&db.post.post_type_id).fold([0i64; 4], |a, t| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (t == 10) as i64]);
    let mut v = Vec::new();
    db.user.with((&db.user.last_access_date).ge(year_ago())).select(Ident::<User>::new().and((&vn).opt()).and((&ps).opt())).drive(|_, ((u, x), p)| v.push((u, x.unwrap_or([0; 3]), p)));
    rows(v.iter().map(|&(u, x, p)| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(ints(&x));
        f.extend(p.map_or(nulls(4), |p| ints(&p)));
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
// SUM(p.Score) AS TotalScore
// FROM Posts p
// WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY p.OwnerUserId
// ),
// CombinedStats AS (
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
// ps.TotalScore
// FROM UserBadges ub
// LEFT JOIN PostStats ps ON ub.UserId = ps.OwnerUserId
// )
// SELECT
// UserId,
// DisplayName,
// BadgeCount,
// GoldBadges,
// SilverBadges,
// BronzeBadges,
// COALESCE(TotalPosts, 0) AS TotalPosts,
// COALESCE(Questions, 0) AS Questions,
// COALESCE(Answers, 0) AS Answers,
// COALESCE(TotalScore, 0) AS TotalScore
// FROM CombinedStats
// ORDER BY TotalScore DESC, BadgeCount DESC
// LIMIT 10;
fn q8220(db: &'static So) -> String {
    let ub = ubc(db);
    let ps = pstat(db, since(db, year_ago()));
    let mut v = Vec::new();
    (&ub).and((&ps).opt()).drive(|u, (b, p)| v.push((u, b, pz(p))));
    out(v, |&(_, b, p)| (Reverse(p[3]), Reverse(b[0])), 10, |&(u, b, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&b));
        f.extend(ints(&[p[0], p[1], p[2], p[3]]));
        f
    })
}

// WITH UserPostSummary AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(P.Score) AS TotalScore,
// SUM(P.ViewCount) AS TotalViews,
// AVG(EXTRACT(EPOCH FROM P.CreationDate)) AS AveragePostCreationDate
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id, U.DisplayName
// ),
// PostActivitySummary AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.Score,
// COALESCE(C.Count, 0) AS CommentCount,
// COALESCE(V.VoteCount, 0) AS VoteCount,
// P.OwnerUserId
// FROM
// Posts P
// LEFT JOIN (
// SELECT PostId, COUNT(*) AS Count
// FROM Comments
// GROUP BY PostId
// ) C ON P.Id = C.PostId
// LEFT JOIN (
// SELECT PostId, COUNT(*) AS VoteCount
// FROM Votes
// GROUP BY PostId
// ) V ON P.Id = V.PostId
// )
// SELECT
// U.UserId,
// U.DisplayName,
// U.TotalPosts,
// U.TotalQuestions,
// U.TotalAnswers,
// U.TotalScore,
// U.TotalViews,
// U.AveragePostCreationDate,
// P.PostId,
// P.Title,
// P.CreationDate,
// P.Score,
// P.CommentCount,
// P.VoteCount
// FROM
// UserPostSummary U
// JOIN
// PostActivitySummary P ON U.UserId = P.OwnerUserId
// ORDER BY
// U.TotalPosts DESC, U.TotalScore DESC, P.Score DESC;
fn q10006(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, creation_date, .. } = &db.post;
    let pf = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(score).and(view_count.opt()).and(creation_date)).fold((Z, 0i128), |(a, e), (((t, s), w), c)| {
        let mut a = a;
        a[0] += 1;
        a[1] += (t == 1) as i64;
        a[2] += (t == 2) as i64;
        a[3] += s;
        a[4] += w.is_some() as i64;
        a[5] += w.unwrap_or(0);
        (a, e + c as i128)
    });
    let cp = comments_per_post(db);
    let vp = votes_per_post(db);
    let mut v = Vec::new();
    owned(db).select(Ident::<Post>::new().and(&cp).and(&vp).and((&db.post.owner_user).select(Ident::<User>::new().and(&pf)))).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(((p, c), x), (u, (a, e)))| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), pviews(a), epoch_avg(a, e)];
        f.extend(post_fields(db, p, &["id", "title", "created", "score"]));
        f.extend(ints(&[c, x]));
        row(f)
    }))
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.Reputation,
// U.CreationDate,
// COUNT(DISTINCT P.Id) AS PostCount,
// COUNT(DISTINCT B.Id) AS BadgeCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// U.Id, U.Reputation, U.CreationDate
// ),
// PostStats AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS TotalPosts,
// SUM(P.Score) AS TotalScore,
// AVG(P.ViewCount) AS AvgViewCount,
// AVG(P.AnswerCount) AS AvgAnswerCount
// FROM
// Posts P
// GROUP BY
// P.OwnerUserId
// )
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// U.CreationDate,
// COALESCE(US.PostCount, 0) AS UserPostCount,
// COALESCE(US.BadgeCount, 0) AS UserBadgeCount,
// COALESCE(US.UpVotes, 0) AS UserUpVotes,
// COALESCE(US.DownVotes, 0) AS UserDownVotes,
// COALESCE(PS.TotalPosts, 0) AS UserTotalPosts,
// COALESCE(PS.TotalScore, 0) AS UserTotalScore,
// COALESCE(PS.AvgViewCount, 0) AS UserAvgViewCount,
// COALESCE(PS.AvgAnswerCount, 0) AS UserAvgAnswerCount
// FROM
// Users U
// LEFT JOIN
// UserStats US ON U.Id = US.UserId
// LEFT JOIN
// PostStats PS ON U.Id = PS.OwnerUserId
// ORDER BY
// U.Reputation DESC
// LIMIT 100;
fn q10025(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "vb", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let bu = badges_per_user(db);
    let Post { score, view_count, answer_count, .. } = &db.post;
    let ps = owned(db).group_by(&db.post.owner_user).select(score.and(view_count.opt()).and(answer_count.opt())).fold([0i64; 6], |a, ((s, w), an)| {
        [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + an.is_some() as i64, a[5] + an.unwrap_or(0)]
    });
    let mut v = Vec::new();
    (&us).and((&dp).opt()).and(&bu).and((&ps).opt()).drive(|u, (((a, d), b), p)| v.push((u, a, d.unwrap_or(0), b, p.unwrap_or([0; 6]))));
    out(v, |&(u, ..)| rep_desc(db, u), 100, |&(u, a, d, b, p)| {
        let mut f = ["uid", "name", "rep", "ucreated"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.extend(ints(&[d, b, a.up, a.down, p[0], p[1]]));
        f.extend([or0(p[3], p[2]), or0(p[5], p[4])]);
        f
    })
}

// WITH UserStats AS (
// SELECT
// Users.Id AS UserId,
// Users.DisplayName,
// COUNT(DISTINCT Posts.Id) AS PostCount,
// SUM(CASE WHEN Posts.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN Posts.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN Votes.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
// SUM(CASE WHEN Votes.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes,
// AVG(Users.Reputation) AS AvgReputation
// FROM
// Users
// LEFT JOIN
// Posts ON Users.Id = Posts.OwnerUserId
// LEFT JOIN
// Votes ON Posts.Id = Votes.PostId
// GROUP BY
// Users.Id, Users.DisplayName
// ),
// PostStats AS (
// SELECT
// Posts.Id AS PostId,
// Posts.Title,
// Posts.CreationDate,
// Posts.Score,
// Posts.ViewCount,
// Posts.AnswerCount,
// Posts.CommentCount,
// CASE
// WHEN Posts.ClosedDate IS NOT NULL THEN 'Closed'
// ELSE 'Open'
// END AS Status
// FROM
// Posts
// )
// SELECT
// u.UserId,
// u.DisplayName,
// u.PostCount,
// u.QuestionCount,
// u.AnswerCount,
// u.Upvotes,
// u.Downvotes,
// u.AvgReputation,
// p.PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// p.AnswerCount AS PostAnswerCount,
// p.CommentCount AS PostCommentCount,
// p.Status
// FROM
// UserStats u
// JOIN
// PostStats p ON u.UserId = p.PostId
// ORDER BY
// u.PostCount DESC,
// u.AvgReputation DESC
// LIMIT 100;
fn q10564(db: &'static So) -> String {
    let pid = pids(db);
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let mut v = Vec::new();
    (&us).and((&dp).opt()).and((&db.user.origid).select(&pid)).drive(|u, ((a, d), p)| v.push((u, a, d.unwrap_or(0), p)));
    out(v, |&(u, _, d, _)| (Reverse(d), rep_desc(db, u)), 100, |&(u, a, d, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&[d, a.q, a.a, a.up, a.down]));
        f.push(V::F(db.user.reputation.get(u).unwrap() as f64));
        f.extend(post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments"]));
        f.push(V::S(if db.post.closed_date.get(p).is_some() { "Closed" } else { "Open" }));
        f
    })
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(DISTINCT p.Id) AS PostCount,
// COUNT(DISTINCT b.Id) AS BadgeCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
// ),
// PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.Score,
// p.ViewCount,
// p.CreationDate,
// COALESCE(p.AcceptedAnswerId, 0) AS AcceptedAnswerId,
// COUNT(c.Id) AS CommentCount,
// COUNT(DISTINCT ph.Id) AS HistoryCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// GROUP BY
// p.Id, p.Title, p.Score, p.ViewCount, p.CreationDate, p.AcceptedAnswerId
// )
// SELECT
// u.UserId,
// u.DisplayName,
// u.Reputation,
// u.PostCount,
// u.BadgeCount,
// u.UpVotes,
// u.DownVotes,
// p.PostId,
// p.Title,
// p.Score,
// p.ViewCount,
// p.CreationDate,
// p.AcceptedAnswerId,
// p.CommentCount,
// p.HistoryCount
// FROM
// UserStats u
// JOIN
// PostStats p ON u.UserId = p.AcceptedAnswerId
// ORDER BY
// u.Reputation DESC, p.Score DESC
// FETCH FIRST 100 ROWS ONLY;
fn q13030(db: &'static So) -> String {
    let uid = uids(db);
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "vb", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let bu = badges_per_user(db);
    let hp = history_per_post(db);
    let mut v = Vec::new();
    stats_fold(db, db.post.iq(), Ident::<Post>::new(), "ch", &[])
        .and(&hp)
        .and((&db.post.accepted_answer_id).opt().map(|a: Option<i64>| a.unwrap_or(0)).select(&uid).select(Ident::<User>::new().and(&us).and((&dp).opt()).and(&bu)))
        .drive(|p, ((s, h), (((u, a), d), b))| v.push((p, s, h, u, a, d.unwrap_or(0), b)));
    out(v, |&(p, _, _, u, ..)| (rep_desc(db, u), score_desc(db, p)), 100, |&(p, s, h, u, a, d, b)| {
        let mut f = ["uid", "name", "rep"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.extend(ints(&[d, b, a.up, a.down]));
        f.extend(post_fields(db, p, &["id", "title", "score", "views", "created"]));
        f.push(V::I(db.post.accepted_answer_id.get(p).unwrap_or(0)));
        f.extend(ints(&[s.cx, h]));
        f
    })
}

// WITH PostStatistics AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// p.AnswerCount,
// p.CommentCount,
// COUNT(c.Id) AS TotalComments,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// COUNT(DISTINCT b.Id) AS TotalBadges
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Badges b ON p.OwnerUserId = b.UserId
// WHERE
// p.CreationDate >= CURRENT_DATE - INTERVAL '1 year'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, p.CommentCount
// ),
// UserStatistics AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// AVG(u.Reputation) AS AverageReputation,
// COUNT(b.Id) AS TotalBadges
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
// ps.ViewCount,
// ps.Score,
// ps.AnswerCount,
// ps.CommentCount,
// ps.TotalComments,
// ps.UpVotes,
// ps.DownVotes,
// us.UserId,
// us.DisplayName,
// us.AverageReputation,
// us.TotalBadges
// FROM
// PostStatistics ps
// JOIN
// Users u ON ps.PostId = u.Id
// JOIN
// UserStatistics us ON u.Id = us.UserId
// ORDER BY
// ps.ViewCount DESC, ps.Score DESC;
fn q14719(db: &'static So) -> String {
    let uid = uids(db);
    let bu = badges_per_user(db);
    let cut = add_years(current_date(), -1);
    let mut v = Vec::new();
    stats_fold(db, db.post.with((&db.post.creation_date).ge(cut)), Ident::<Post>::new(), "cvb", &[])
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&bu)))
        .drive(|p, (s, (u, b))| v.push((p, s, u, b)));
    rows(v.iter().map(|&(p, s, u, b)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "answers", "comments"]);
        f.extend(ints(&[s.cx, s.up, s.down]));
        f.extend([user_col(db, u, "uid"), user_col(db, u, "name"), V::F(db.user.reputation.get(u).unwrap() as f64), V::I(b)]);
        row(f)
    }))
}

// WITH UserVoteStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes,
// COUNT(DISTINCT P.Id) AS TotalPosts,
// COUNT(DISTINCT C.Id) AS TotalComments,
// COUNT(DISTINCT B.Id) AS TotalBadges
// FROM
// Users U
// LEFT JOIN
// Votes V ON U.Id = V.UserId
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// GROUP BY
// U.Id, U.DisplayName
// ),
// PostStats AS (
// SELECT
// P.OwnerUserId,
// P.PostTypeId,
// COUNT(*) AS TotalPostsOfType,
// AVG(P.Score) AS AverageScore,
// SUM(P.ViewCount) AS TotalViews
// FROM
// Posts P
// GROUP BY
// P.OwnerUserId, P.PostTypeId
// ),
// CombinedStats AS (
// SELECT
// U.DisplayName,
// U.TotalUpVotes,
// U.TotalDownVotes,
// COALESCE(S.TotalPostsOfType, 0) AS TotalPostsOfType,
// COALESCE(S.AverageScore, 0) AS AverageScore,
// COALESCE(S.TotalViews, 0) AS TotalViews
// FROM
// UserVoteStats U
// LEFT JOIN
// PostStats S ON U.UserId = S.OwnerUserId
// )
// SELECT
// C.DisplayName,
// C.TotalUpVotes,
// C.TotalDownVotes,
// C.TotalPostsOfType,
// C.AverageScore,
// C.TotalViews
// FROM
// CombinedStats C
// ORDER BY
// C.TotalUpVotes DESC, C.TotalViews DESC
// LIMIT 10;
fn q7928(db: &'static So) -> String {
    let uv = db
        .user
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt().and(posts_of(db).select(comments_of(db).opt()).opt()).and(badges_of(db).opt()))
        .fold([0i64; 2], |a, ((t, _), _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let Post { score, view_count, .. } = &db.post;
    let tf: MatSet<(Id<User>, i64)> = owned(db).select((&db.post.owner_user).and(&db.post.post_type_id)).collect();
    let st = owned(db)
        .group_by((&db.post.owner_user).and(&db.post.post_type_id))
        .select(score.and(view_count.opt()))
        .fold([0i64; 4], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]);
    let ut = (&tf).map(|(u, _)| u).inv().collect::<HashIdx<Id<User>, (Id<User>, i64)>>();
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and(&uv).and((&ut).select(&st).opt())).drive(|_, ((u, x), s)| v.push((u, x[0], x[1], s)));
    out(v, |&(_, up, _, s)| (Reverse(up), Reverse(s.map_or(0, |s| s[3]))), 10, |&(u, up, down, s)| {
        let s = s.unwrap_or([0; 4]);
        vec![user_col(db, u, "name"), V::I(up), V::I(down), V::I(s[0]), or0(s[1], s[0]), V::I(s[3])]
    })
}

// WITH PostSummary AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// COUNT(c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount
// ),
// UserEngagement AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS PostsCreated,
// SUM(p.Score) AS TotalScore,
// SUM(p.ViewCount) AS TotalViews,
// COUNT(DISTINCT b.Id) AS BadgesCount
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
// ps.Score,
// ps.ViewCount,
// ps.CommentCount,
// ps.VoteCount,
// ps.UpVoteCount,
// ps.DownVoteCount,
// ue.UserId,
// ue.DisplayName AS PostOwner,
// ue.PostsCreated,
// ue.TotalScore AS UserTotalScore,
// ue.TotalViews AS UserTotalViews,
// ue.BadgesCount
// FROM
// PostSummary ps
// JOIN
// Users u ON ps.PostId = u.Id
// JOIN
// UserEngagement ue ON u.Id = ue.UserId
// ORDER BY
// ps.CreationDate DESC
// LIMIT 100;
fn q10728(db: &'static So) -> String {
    let uid = uids(db);
    let bu = badges_per_user(db);
    let ub = user_bp(db, db.user.iq());
    let mut v = Vec::new();
    stats_fold(db, db.post.with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cv", &[])
        .and(votes_per_post(db))
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&bu).and(&ub)))
        .drive(|p, ((s, x), ((u, b), q))| v.push((p, s, x, u, b, q)));
    out(v, |&(p, ..)| Reverse(db.post.creation_date.get(p).unwrap()), 100, |&(p, s, x, u, b, q)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend(ints(&[s.cx, x, s.up, s.down]));
        f.extend([user_col(db, u, "uid"), user_col(db, u, "name"), V::I(q[2]), nullable(q[3], q[2]), nullable(q[5], q[4]), V::I(b)]);
        f
    })
}

// WITH PostStatistics AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// p.OwnerUserId
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.OwnerUserId
// ),
// UserReputation AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(DISTINCT P.Id) AS PostsCount,
// SUM(B.Class) AS BadgesCount
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// GROUP BY
// U.Id, U.DisplayName, U.Reputation
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.ViewCount,
// ps.Score,
// ps.CommentCount,
// ps.VoteCount,
// ps.UpVotes,
// ps.DownVotes,
// ur.DisplayName AS OwnerDisplayName,
// ur.Reputation AS UserReputation,
// ur.PostsCount AS TotalPosts,
// ur.BadgesCount AS TotalBadges
// FROM
// PostStatistics ps
// JOIN
// UserReputation ur ON ps.OwnerUserId = ur.UserId
// ORDER BY
// ps.Score DESC,
// ps.ViewCount DESC;
fn q14018(db: &'static So) -> String {
    let dp = ud(db, UserWhere::All, posts_of(db));
    let cs = user_class_rows(db);
    let mut v = Vec::new();
    stats_fold(db, owned_since(db, year_ago()), Ident::<Post>::new(), "cv", &[])
        .and((&db.post.owner_user).select(Ident::<User>::new().and(&dp).and(&cs)))
        .drive(|p, (s, ((u, d), c))| v.push((p, s, u, d, c)));
    rows(v.iter().map(|&(p, s, u, d, c)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend(ints(&[s.cx, s.vx, s.up, s.down]));
        f.extend([user_col(db, u, "name"), user_col(db, u, "rep"), V::I(d), nullable(c[1], c[0])]);
        row(f)
    }))
}

// WITH PostSummary AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// COALESCE(v.UpVotes, 0) AS UpVotes,
// COALESCE(v.DownVotes, 0) AS DownVotes,
// COALESCE(c.CommentCount, 0) AS CommentCount,
// COALESCE(a.AcceptedAnswerCount, 0) AS AcceptedAnswerCount
// FROM
// Posts p
// LEFT JOIN (
// SELECT
// PostId,
// COUNT(CASE WHEN VoteTypeId = 2 THEN 1 END) AS UpVotes,
// COUNT(CASE WHEN VoteTypeId = 3 THEN 1 END) AS DownVotes
// FROM
// Votes
// GROUP BY
// PostId
// ) v ON p.Id = v.PostId
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
// AcceptedAnswerId,
// COUNT(*) AS AcceptedAnswerCount
// FROM
// Posts
// WHERE
// PostTypeId = 1
// GROUP BY
// AcceptedAnswerId
// ) a ON p.Id = a.AcceptedAnswerId
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.ViewCount,
// ps.Score,
// ps.UpVotes,
// ps.DownVotes,
// ps.CommentCount,
// ps.AcceptedAnswerCount,
// (ps.UpVotes - ps.DownVotes) AS NetVotes
// FROM
// PostSummary ps
// ORDER BY
// ps.Score DESC,
// ps.ViewCount DESC
// LIMIT 100;
fn q10762(db: &'static So) -> String {
    let pv = post_votes(db);
    let cp = comments_per_post(db);
    let ac = questions_only(db).group_by(&db.post.accepted_answer).select(&db.post.score).fold(0i64, |a, _| a + 1);
    let mut v = Vec::new();
    since(db, year_ago()).select(Ident::<Post>::new().and((&pv).opt()).and(&cp).and((&ac).opt())).drive(|_, (((p, x), c), a)| v.push((p, x.unwrap_or([0; 3]), c, a.unwrap_or(0))));
    out(v, |&(p, ..)| (score_views(db, p), db.post.origid.get(p).unwrap()), 100, |&(p, x, c, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend(ints(&[x[1], x[2], c, a, x[1] - x[2]]));
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
// FROM Users U
// LEFT JOIN Badges B ON U.Id = B.UserId
// WHERE U.Reputation > 1000
// GROUP BY U.Id, U.DisplayName
// ),
// PostStats AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// AVG(P.Score) AS AverageScore
// FROM Posts P
// GROUP BY P.OwnerUserId
// ),
// UserActivity AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COALESCE(UB.TotalBadges, 0) AS TotalBadges,
// COALESCE(PS.TotalPosts, 0) AS TotalPosts,
// COALESCE(PS.TotalQuestions, 0) AS TotalQuestions,
// COALESCE(PS.TotalAnswers, 0) AS TotalAnswers,
// COALESCE(PS.AverageScore, 0) AS AverageScore
// FROM Users U
// LEFT JOIN UserBadgeStats UB ON U.Id = UB.UserId
// LEFT JOIN PostStats PS ON U.Id = PS.OwnerUserId
// WHERE U.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'
// )
// SELECT
// U.DisplayName,
// U.TotalBadges,
// U.TotalPosts,
// U.TotalQuestions,
// U.TotalAnswers,
// U.AverageScore
// FROM UserActivity U
// ORDER BY U.TotalBadges DESC, U.TotalPosts DESC
// LIMIT 10;
fn q8088(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    user_base(db, UserWhere::CreatedGe(date(2023, 10, 1))).select(Ident::<User>::new().and(&bu).and((&ps).opt())).drive(|_, ((u, b), p)| {
        let b = if db.user.reputation.get(u).unwrap() > 1000 { b } else { 0 };
        v.push((u, b, pz(p)))
    });
    out(v, |&(_, b, p)| (Reverse(b), Reverse(p[0])), 10, |&(u, b, p)| vec![user_col(db, u, "name"), V::I(b), V::I(p[0]), V::I(p[1]), V::I(p[2]), or0(p[3], p[0])])
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
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
// u.Id, u.Reputation
// ),
// BadgeStats AS (
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
// u.Reputation,
// us.PostCount,
// us.QuestionCount,
// us.AnswerCount,
// us.TotalViews,
// us.TotalUpVotes,
// us.TotalDownVotes,
// COALESCE(bs.BadgeCount, 0) AS BadgeCount,
// COALESCE(bs.GoldBadgeCount, 0) AS GoldBadgeCount,
// COALESCE(bs.SilverBadgeCount, 0) AS SilverBadgeCount,
// COALESCE(bs.BronzeBadgeCount, 0) AS BronzeBadgeCount
// FROM
// Users u
// LEFT JOIN
// UserStats us ON u.Id = us.UserId
// LEFT JOIN
// BadgeStats bs ON u.Id = bs.UserId
// ORDER BY
// u.Reputation DESC;
fn q14017(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&us).and((&dp).opt()).and((&bc).opt()).drive(|u, ((a, d), b)| v.push((u, a, d.unwrap_or(0), bz(b))));
    rows(v.iter().map(|&(u, a, d, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep"), V::I(d), V::I(a.q), V::I(a.a), ustat_field(&a, "views_sum"), V::I(a.up), V::I(a.down)];
        f.extend(ints(&b));
        row(f)
    }))
}

// WITH UserReputation AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(DISTINCT P.Id) AS PostCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id, U.DisplayName, U.Reputation
// ), BadgeCounts AS (
// SELECT
// B.UserId,
// COUNT(B.Id) AS BadgeCount
// FROM
// Badges B
// GROUP BY
// B.UserId
// ), PostEngagement AS (
// SELECT
// P.OwnerUserId,
// COUNT(C.Id) AS CommentCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// P.OwnerUserId
// )
// SELECT
// UR.UserId,
// UR.DisplayName,
// UR.Reputation,
// COALESCE(BC.BadgeCount, 0) AS BadgeCount,
// COALESCE(PE.CommentCount, 0) AS CommentCount,
// COALESCE(PE.UpvoteCount, 0) AS UpvoteCount,
// COALESCE(PE.DownvoteCount, 0) AS DownvoteCount,
// UR.PostCount,
// UR.QuestionCount,
// UR.AnswerCount
// FROM
// UserReputation UR
// LEFT JOIN
// BadgeCounts BC ON UR.UserId = BC.UserId
// LEFT JOIN
// PostEngagement PE ON UR.UserId = PE.OwnerUserId
// ORDER BY
// UR.Reputation DESC,
// UR.PostCount DESC,
// UR.UserId;
fn q5998(db: &'static So) -> String {
    let sf = stats_fold(db, owned(db), &db.post.owner_user, "cv", &[]);
    let pq = owned(db).group_by(&db.post.owner_user).select(&db.post.post_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64]);
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and(&bu).and((&sf).opt()).and((&pq).opt())).drive(|_, (((u, b), s), p)| v.push((u, b, s, p.unwrap_or([0; 3]))));
    rows(v.iter().map(|&(u, b, s, p)| {
        let mut f = ["uid", "name", "rep"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.push(V::I(b));
        f.extend(s.map_or(ints(&[0, 0, 0]), |s| ints(&[s.cx, s.up, s.down])));
        f.extend(ints(&p));
        row(f)
    }))
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.Reputation,
// COUNT(DISTINCT P.Id) AS PostCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN P.PostTypeId IN (10, 11) THEN 1 ELSE 0 END) AS ClosedPostCount
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id, U.Reputation
// ),
// VoteStats AS (
// SELECT
// V.UserId,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Votes V
// GROUP BY
// V.UserId
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
// )
// SELECT
// U.UserId,
// U.Reputation,
// U.PostCount,
// U.QuestionCount,
// U.AnswerCount,
// U.ClosedPostCount,
// COALESCE(V.UpVotes, 0) AS UpVotes,
// COALESCE(V.DownVotes, 0) AS DownVotes,
// COALESCE(B.GoldBadges, 0) AS GoldBadges,
// COALESCE(B.SilverBadges, 0) AS SilverBadges,
// COALESCE(B.BronzeBadges, 0) AS BronzeBadges
// FROM
// UserStats U
// LEFT JOIN
// VoteStats V ON U.UserId = V.UserId
// LEFT JOIN
// BadgeStats B ON U.UserId = B.UserId
// ORDER BY
// U.Reputation DESC;
fn q13168(db: &'static So) -> String {
    let ps = owned(db).group_by(&db.post.owner_user).select(&db.post.post_type_id).fold([0i64; 4], |a, t| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + matches!(t, 10 | 11) as i64]);
    let uv = uvotes(db);
    let bc = badge_classes(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&ps).opt()).and((&uv).opt()).and((&bc).opt())).drive(|_, (((u, p), x), b)| v.push((u, p.unwrap_or([0; 4]), x.unwrap_or([0; 3]), bz(b))));
    rows(v.iter().map(|&(u, p, x, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep")];
        f.extend(ints(&p));
        f.extend(ints(&[x[1], x[2], b[1], b[2], b[3]]));
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
// COUNT(c.Id) AS CommentCount,
// u.DisplayName AS OwnerDisplayName,
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
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 YEAR'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, u.DisplayName
// ),
// UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(u.UpVotes) AS TotalUpVotes,
// SUM(u.DownVotes) AS TotalDownVotes,
// SUM(u.Views) AS TotalViews
// FROM
// Users u
// JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.Score,
// ps.ViewCount,
// ps.AnswerCount,
// ps.CommentCount,
// ps.OwnerDisplayName,
// us.UserId,
// us.DisplayName AS UserDisplayName,
// us.PostCount,
// us.TotalUpVotes,
// us.TotalDownVotes,
// us.TotalViews
// FROM
// PostStats ps
// JOIN
// UserStats us ON ps.OwnerDisplayName = us.DisplayName
// ORDER BY
// ps.CreationDate DESC;
fn q13361(db: &'static So) -> String {
    let User { up_votes, down_votes, views, .. } = &db.user;
    let ps = owned(db).group_by(&db.post.owner_user).select((&db.post.owner_user).select(up_votes.and(down_votes).and(views))).fold([0i64; 4], |a, ((u, d), w)| [a[0] + 1, a[1] + u, a[2] + d, a[3] + w]);
    let named: HashIdx<Str, Id<User>> = db.user.with(&ps).select(&db.user.display_name).inv().collect();
    let mut v = Vec::new();
    stats_fold(db, owned_since(db, year_ago()), Ident::<Post>::new(), "cv", &[])
        .and((&db.post.owner_user).select(&db.user.display_name).select(&named).select(Ident::<User>::new().and(&ps)))
        .drive(|p, (s, (u, x))| v.push((p, s, u, x)));
    rows(v.iter().map(|&(p, s, u, x)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers"]);
        f.push(V::I(s.cx));
        f.extend(post_fields(db, p, &["owner"]));
        f.extend([user_col(db, u, "uid"), user_col(db, u, "name"), V::I(x[0])]);
        f.extend(ints(&[x[1], x[2], x[3]]));
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
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
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
// COUNT(b.Id) AS BadgeCount,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadgeCount,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadgeCount,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadgeCount
// FROM
// Badges b
// GROUP BY
// b.UserId
// )
// SELECT
// us.UserId,
// us.DisplayName,
// us.Reputation,
// us.PostCount,
// us.QuestionCount,
// us.AnswerCount,
// us.UpVoteCount,
// us.DownVoteCount,
// COALESCE(bs.BadgeCount, 0) AS BadgeCount,
// COALESCE(bs.GoldBadgeCount, 0) AS GoldBadgeCount,
// COALESCE(bs.SilverBadgeCount, 0) AS SilverBadgeCount,
// COALESCE(bs.BronzeBadgeCount, 0) AS BronzeBadgeCount
// FROM
// UserStats us
// LEFT JOIN
// BadgeStats bs ON us.UserId = bs.UserId
// ORDER BY
// us.Reputation DESC, us.PostCount DESC
// FETCH FIRST 100 ROWS ONLY;
fn q12712(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&us).and((&dp).opt()).and((&bc).opt()).drive(|u, ((a, d), b)| v.push((u, a, d.unwrap_or(0), bz(b))));
    out(v, |&(u, _, d, _)| (rep_desc(db, u), Reverse(d)), 100, |&(u, a, d, b)| {
        let mut f = ["uid", "name", "rep"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.extend(ints(&[d, a.q, a.a, a.up, a.down]));
        f.extend(ints(&b));
        f
    })
}

// WITH PostsStats AS (
// SELECT
// p.Id AS PostId,
// pt.Name AS PostType,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVoteCount,
// COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVoteCount,
// AVG(COALESCE(EXTRACT(EPOCH FROM (p.LastActivityDate - p.CreationDate)), 0)) AS AvgResponseTime
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// WHERE
// p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'
// GROUP BY
// p.Id, pt.Name
// ),
// UserBadges AS (
// SELECT
// u.Id AS UserId,
// COUNT(b.Id) FILTER (WHERE b.Class = 1) AS GoldBadges,
// COUNT(b.Id) FILTER (WHERE b.Class = 2) AS SilverBadges,
// COUNT(b.Id) FILTER (WHERE b.Class = 3) AS BronzeBadges
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
// ps.UpVotes,
// ps.DownVotes,
// ub.GoldBadges,
// ub.SilverBadges,
// ub.BronzeBadges,
// ps.AvgResponseTime
// FROM
// PostsStats ps
// LEFT JOIN
// Users u ON ps.PostId = u.Id
// LEFT JOIN
// UserBadges ub ON u.Id = ub.UserId
// ORDER BY
// ps.AvgResponseTime DESC, ps.CommentCount DESC;
fn q8764(db: &'static So) -> String {
    let uid = uids(db);
    let bc = badge_classes(db);
    let mut v = Vec::new();
    stats_fold(db, since(db, year_ago()), Ident::<Post>::new(), "cv", &[])
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and((&bc).opt())).opt())
        .drive(|p, (s, u)| v.push((p, s, u.map(|(_, b)| bz(b)))));
    let age = |p: Id<Post>| db.post.last_activity_date.get(p).unwrap() - db.post.creation_date.get(p).unwrap();
    rows(v.iter().map(|&(p, s, b)| {
        let mut f = post_fields(db, p, &["id", "type"]);
        f.extend(ints(&[s.cx, s.up, s.down]));
        f.extend(b.map_or(nulls(3), |b| ints(&[b[1], b[2], b[3]])));
        f.push(V::F(age(p) as f64 / 1e6));
        row(f)
    }))
}

// WITH UserActivity AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(DISTINCT P.Id) AS PostCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(COALESCE(P.Score, 0)) AS TotalScore,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// WHERE
// U.Reputation > 0
// GROUP BY
// U.Id, U.DisplayName, U.Reputation
// ),
// BadgeCount AS (
// SELECT
// UserId,
// COUNT(*) AS BadgeTotal
// FROM
// Badges
// GROUP BY
// UserId
// ),
// EngagementSummary AS (
// SELECT
// UA.UserId,
// UA.DisplayName,
// UA.Reputation,
// UA.PostCount,
// UA.QuestionCount,
// UA.AnswerCount,
// UA.TotalScore,
// UA.TotalUpvotes,
// UA.TotalDownvotes,
// COALESCE(BC.BadgeTotal, 0) AS BadgeTotal
// FROM
// UserActivity UA
// LEFT JOIN
// BadgeCount BC ON UA.UserId = BC.UserId
// )
// SELECT
// UserId,
// DisplayName,
// Reputation,
// PostCount,
// QuestionCount,
// AnswerCount,
// TotalScore,
// TotalUpvotes,
// TotalDownvotes,
// BadgeTotal
// FROM
// EngagementSummary
// ORDER BY
// Reputation DESC,
// TotalScore DESC,
// PostCount DESC
// LIMIT 100;
fn q8050(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::RepGt(0), "v", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    (&us).and((&dp).opt()).and(&bu).drive(|u, ((a, d), b)| v.push((u, a, d.unwrap_or(0), b)));
    out(v, |&(u, a, d, _)| (rep_desc(db, u), Reverse(a.score_sum), Reverse(d)), 100, |&(u, a, d, b)| {
        let mut f = ["uid", "name", "rep"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.extend(ints(&[d, a.q, a.a, a.score_sum, a.up, a.down, b]));
        f
    })
}

// WITH PostInfo AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// COUNT(DISTINCT c.Id) AS CommentCount,
// COUNT(DISTINCT a.Id) AS AnswerCount,
// COALESCE(u.DisplayName, 'Community User') AS OwnerDisplayName,
// p.OwnerUserId  -- Added to GROUP BY clause
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Posts a ON p.Id = a.ParentId
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.PostTypeId = 1 AND p.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year')
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.DisplayName, p.OwnerUserId  -- Added missing columns
// ),
// PostHistoryInfo AS (
// SELECT
// ph.PostId,
// COUNT(*) AS EditCount,
// MAX(ph.CreationDate) AS LastEditDate
// FROM
// PostHistory ph
// GROUP BY
// ph.PostId
// ),
// UserBadges AS (
// SELECT
// b.UserId,
// COUNT(*) AS TotalBadges
// FROM
// Badges b
// GROUP BY
// b.UserId
// )
// SELECT
// pi.PostId,
// pi.Title,
// pi.CreationDate,
// pi.Score,
// pi.ViewCount,
// pi.CommentCount,
// pi.AnswerCount,
// ph.EditCount,
// ph.LastEditDate,
// ub.TotalBadges,
// pi.OwnerDisplayName
// FROM
// PostInfo pi
// LEFT JOIN
// PostHistoryInfo ph ON pi.PostId = ph.PostId
// LEFT JOIN
// UserBadges ub ON pi.OwnerUserId = ub.UserId
// ORDER BY
// pi.Score DESC,
// pi.ViewCount DESC;
fn q9584(db: &'static So) -> String {
    let cp = comments_per_post(db);
    let ap = answers_per_post(db);
    let hf = history_n_max(db);
    let bu = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold(0i64, |a, _| a + 1);
    let mut v = Vec::new();
    questions_only(db)
        .with((&db.post.creation_date).ge(year_ago()))
        .select(Ident::<Post>::new().and(&cp).and(&ap).and((&hf).opt()).and((&db.post.owner_user).select(&bu).opt()))
        .drive(|_, x| v.push(x));
    rows(v.iter().map(|&((((p, c), a), h), b)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(c), V::I(a), oint(h.map(|h| h.0)), ots(h.map(|h| h.1)), oint(b), named_owner(db, p, "Community User")]);
        row(f)
    }))
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes,
// COUNT(c.Id) AS CommentCount,
// COUNT(DISTINCT ph.Id) AS EditCount,
// COUNT(DISTINCT b.Id) AS BadgeCount,
// p.OwnerUserId
// FROM
// Posts p
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// LEFT JOIN
// Badges b ON p.OwnerUserId = b.UserId
// WHERE
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY
// p.Id, p.Title, p.OwnerUserId
// ),
// UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// SUM(p.ViewCount) AS TotalViews,
// AVG(p.Score) AS AvgScore,
// COUNT(b.Id) AS TotalBadges
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// WHERE
// u.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '2 years'
// GROUP BY
// u.Id, u.DisplayName
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.Upvotes,
// ps.Downvotes,
// ps.CommentCount,
// ps.EditCount,
// us.UserId,
// us.DisplayName,
// us.TotalViews,
// us.AvgScore,
// us.TotalBadges
// FROM
// PostStats ps
// JOIN
// UserStats us ON ps.OwnerUserId = us.UserId
// ORDER BY
// ps.Upvotes DESC, us.TotalViews DESC
// LIMIT 100;
fn q6704(db: &'static So) -> String {
    let hp = history_per_post(db);
    let us = user_bp(db, db.user.with((&db.user.creation_date).ge(ts(2022, 10, 1, 12, 34, 56))));
    let mut v = Vec::new();
    stats_fold(db, owned_since(db, year_ago()), Ident::<Post>::new(), "vchb", &[])
        .and(&hp)
        .and((&db.post.owner_user).select(Ident::<User>::new().and(&us)))
        .drive(|p, ((s, h), (u, q))| v.push((p, s, h, u, q)));
    out(v, |&(_, s, _, _, q)| (Reverse(s.up), (q[4] == 0, Reverse(q[5]))), 100, |&(p, s, h, u, q)| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend(ints(&[s.up, s.down, s.cx, h]));
        f.extend([user_col(db, u, "uid"), user_col(db, u, "name"), nullable(q[5], q[4]), avg(q[3], q[2]), V::I(q[1])]);
        f
    })
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN p.PostTypeId = 1 AND p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
// ),
// BadgesSummary AS (
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
// COUNT(*) AS TotalEdits,
// MAX(ph.CreationDate) AS LastEditDate
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
// us.TotalQuestions,
// us.TotalAnswers,
// us.AcceptedAnswers,
// bs.GoldBadges,
// bs.SilverBadges,
// bs.BronzeBadges,
// phs.TotalEdits,
// phs.LastEditDate
// FROM
// UserStats us
// LEFT JOIN
// BadgesSummary bs ON us.UserId = bs.UserId
// LEFT JOIN
// PostHistorySummary phs ON us.UserId = phs.UserId
// WHERE
// us.Reputation > 100
// ORDER BY
// us.Reputation DESC, us.TotalPosts DESC
// LIMIT 100;
fn q5267(db: &'static So) -> String {
    let Post { post_type_id, accepted_answer_id, .. } = &db.post;
    let uf = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(accepted_answer_id.opt())).fold([0i64; 4], |a, (t, acc)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (t == 1 && acc.is_some()) as i64]
    });
    let bc = badge_classes(db);
    let ph = db.post_history.group_by(&db.post_history.user).select(&db.post_history.creation_date).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(100)).select(Ident::<User>::new().and((&uf).opt()).and((&bc).opt()).and((&ph).opt())).drive(|_, (((u, a), b), h)| v.push((u, a.unwrap_or([0; 4]), b, h)));
    out(v, |&(u, a, ..)| (rep_desc(db, u), Reverse(a[0])), 100, |&(u, a, b, h)| {
        let mut f = ["uid", "name", "rep"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.extend(ints(&a));
        f.extend(b.map_or(nulls(3), |b| ints(&[b[1], b[2], b[3]])));
        f.extend([oint(h.map(|h| h.0)), ots(h.map(|h| h.1))]);
        f
    })
}

// WITH UserVotes AS (
// SELECT
// u.Id AS UserId,
// COUNT(v.Id) AS TotalVotes,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Users u
// LEFT JOIN
// Votes v ON u.Id = v.UserId
// GROUP BY
// u.Id
// ),
// PostStats AS (
// SELECT
// p.Id AS PostId,
// p.PostTypeId,
// COUNT(c.Id) AS CommentCount,
// COUNT(DISTINCT v.UserId) AS UniqueVoters,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// SUM(CASE WHEN b.UserId IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Badges b ON p.OwnerUserId = b.UserId
// GROUP BY
// p.Id, p.PostTypeId
// )
// SELECT
// u.DisplayName,
// u.Reputation,
// uv.TotalVotes,
// ps.PostId,
// ps.PostTypeId,
// ps.CommentCount,
// ps.UniqueVoters,
// ps.UpVotes AS PostUpVotes,
// ps.DownVotes AS PostDownVotes,
// COUNT(b.Id) AS UserBadgeCount
// FROM
// Users u
// JOIN
// UserVotes uv ON u.Id = uv.UserId
// JOIN
// PostStats ps ON u.Id = ps.UniqueVoters
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// u.DisplayName, u.Reputation, uv.TotalVotes, ps.PostId, ps.PostTypeId,
// ps.CommentCount, ps.UniqueVoters, ps.UpVotes, ps.DownVotes
// ORDER BY
// u.Reputation DESC, ps.UpVotes DESC;
fn q12167(db: &'static So) -> String {
    let uid = uids(db);
    let uv = uvotes(db);
    let bu = badges_per_user(db);
    let dv = db.vote.group_by(&db.vote.post).select(&db.vote.user).count_distinct();
    let mut v = Vec::new();
    stats_fold(db, db.post.iq(), Ident::<Post>::new(), "cvb", &[])
        .and((&dv).opt().map(|d: Option<i64>| d.unwrap_or(0)))
        .and((&dv).opt().map(|d: Option<i64>| d.unwrap_or(0)).select(&uid).select(Ident::<User>::new().and((&uv).opt()).and(&bu)))
        .drive(|p, ((s, d), ((u, x), b))| v.push((p, s, d, u, x.unwrap_or([0; 3]), b)));
    rows(v.iter().map(|&(p, s, d, u, x, b)| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep"), V::I(x[0])];
        f.extend(post_fields(db, p, &["id", "type_id"]));
        f.extend(ints(&[s.cx, d, s.up, s.down, b]));
        row(f)
    }))
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN COALESCE(p.AcceptedAnswerId, 0) END) AS AcceptedAnswers,
// SUM(p.ViewCount) AS TotalViews,
// SUM(p.Score) AS TotalScore
// FROM
// Users u
// JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
// ),
// BadgeCounts AS (
// SELECT
// b.UserId,
// COUNT(b.Id) AS BadgeCount
// FROM
// Badges b
// GROUP BY
// b.UserId
// ),
// QuestionStats AS (
// SELECT
// p.OwnerUserId,
// MAX(p.CreationDate) AS LastQuestionDate,
// COUNT(DISTINCT p.Id) AS QuestionsAnswered
// FROM
// Posts p
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.OwnerUserId
// )
// SELECT
// us.DisplayName,
// us.Reputation,
// us.PostCount,
// us.QuestionsCount,
// us.AnswersCount,
// us.AcceptedAnswers,
// us.TotalViews,
// us.TotalScore,
// COALESCE(bc.BadgeCount, 0) AS BadgeCount,
// COALESCE(qs.LastQuestionDate, '1970-01-01') AS LastQuestionDate,
// COALESCE(qs.QuestionsAnswered, 0) AS QuestionsAnswered
// FROM
// UserStats us
// LEFT JOIN
// BadgeCounts bc ON us.UserId = bc.UserId
// LEFT JOIN
// QuestionStats qs ON us.UserId = qs.OwnerUserId
// ORDER BY
// us.TotalScore DESC, us.Reputation DESC;
fn q9074(db: &'static So) -> String {
    let Post { post_type_id, accepted_answer_id, view_count, score, .. } = &db.post;
    let uf = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(accepted_answer_id.opt()).and(view_count.opt()).and(score)).fold([0i64; 7], |a, (((t, acc), w), s)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + if t == 1 { acc.unwrap_or(0) } else { 0 }, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0), a[6] + s]
    });
    let bu = badges_per_user(db);
    let qs = questions_only(db).group_by(&db.post.owner_user).select(&db.post.creation_date).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let mut v = Vec::new();
    (&uf).and(&bu).and((&qs).opt()).drive(|u, ((a, b), q)| v.push((u, a, b, q)));
    rows(v.iter().map(|&(u, a, b, q)| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(ints(&[a[0], a[1], a[2]]));
        f.extend([nullable(a[3], a[1]), nullable(a[5], a[4]), V::I(a[6]), V::I(b), V::T(q.map_or(0, |q| q.1)), V::I(q.map_or(0, |q| q.0))]);
        row(f)
    }))
}

// WITH PostStatistics AS (
// SELECT
// Posts.Id AS PostId,
// Posts.Title,
// Posts.CreationDate,
// Posts.ViewCount,
// Posts.Score,
// COUNT(DISTINCT Comments.Id) AS CommentCount,
// COUNT(DISTINCT Votes.Id) AS VoteCount,
// SUM(CASE WHEN Votes.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN Votes.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Posts
// LEFT JOIN
// Comments ON Posts.Id = Comments.PostId
// LEFT JOIN
// Votes ON Posts.Id = Votes.PostId
// WHERE
// Posts.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY
// Posts.Id, Posts.Title, Posts.CreationDate, Posts.ViewCount, Posts.Score
// ),
// UserStatistics AS (
// SELECT
// Users.Id AS UserId,
// Users.DisplayName,
// COUNT(DISTINCT Posts.Id) AS PostCount,
// SUM(Users.UpVotes) AS TotalUpVotes,
// SUM(Users.DownVotes) AS TotalDownVotes,
// SUM(Users.Reputation) AS TotalReputation
// FROM
// Users
// LEFT JOIN
// Posts ON Users.Id = Posts.OwnerUserId
// GROUP BY
// Users.Id, Users.DisplayName
// )
// SELECT
// P.PostId,
// P.Title,
// P.CreationDate,
// P.ViewCount,
// P.Score,
// P.CommentCount,
// P.VoteCount,
// P.UpVotes,
// P.DownVotes,
// U.UserId,
// U.DisplayName,
// U.PostCount,
// U.TotalUpVotes,
// U.TotalDownVotes,
// U.TotalReputation
// FROM
// PostStatistics P
// JOIN
// UserStatistics U ON P.PostId = U.UserId
// ORDER BY
// P.ViewCount DESC
// LIMIT 100;
fn q12518(db: &'static So) -> String {
    let uid = uids(db);
    let lp = user_left_posts(db);
    let mut v = Vec::new();
    stats_fold(db, since(db, year_ago()), Ident::<Post>::new(), "cv", &[])
        .and(comments_per_post(db))
        .and(votes_per_post(db))
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&lp)))
        .drive(|p, (((s, c), x), (u, d))| v.push((p, s, c, x, u, d)));
    out(v, |&(p, ..)| views_desc(db, p), 100, |&(p, s, c, x, u, d)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend(ints(&[c, x, s.up, s.down]));
        f.extend([user_col(db, u, "uid"), user_col(db, u, "name")]);
        f.extend(ints(&[d[1], d[2], d[3], d[4]]));
        f
    })
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS PostCount,
// SUM(p.Score) AS TotalScore,
// AVG(p.ViewCount) AS AvgViewCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName
// ),
// RecentPostActivity AS (
// SELECT
// p.OwnerUserId,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN ph.PostHistoryTypeId IN (10, 11, 12) THEN 1 ELSE 0 END) AS ClosedPostCount,
// MAX(p.CreationDate) AS LastPostDate
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// GROUP BY
// p.OwnerUserId
// ),
// FinalStats AS (
// SELECT
// ups.UserId,
// ups.DisplayName,
// ups.PostCount,
// ups.TotalScore,
// ups.AvgViewCount,
// rpa.CommentCount,
// rpa.ClosedPostCount,
// rpa.LastPostDate
// FROM
// UserPostStats ups
// LEFT JOIN
// RecentPostActivity rpa ON ups.UserId = rpa.OwnerUserId
// )
// SELECT
// fs.DisplayName,
// fs.PostCount,
// COALESCE(fs.TotalScore, 0) AS TotalScore,
// ROUND(COALESCE(fs.AvgViewCount, 0), 2) AS AvgViewCount,
// COALESCE(fs.CommentCount, 0) AS CommentCount,
// COALESCE(fs.ClosedPostCount, 0) AS ClosedPostCount,
// fs.LastPostDate
// FROM
// FinalStats fs
// WHERE
// fs.PostCount > 0
// AND fs.LastPostDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days')
// ORDER BY
// fs.TotalScore DESC
// LIMIT 10;
fn q1921(db: &'static So) -> String {
    let Post { score, view_count, creation_date, .. } = &db.post;
    let ps = owned(db).group_by(&db.post.owner_user).select(score.and(view_count.opt())).fold([0i64; 4], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]);
    let ra = owned(db)
        .group_by(&db.post.owner_user)
        .select(creation_date.and(comments_of(db).opt()).and(history_of(db).select(&db.post_history.post_history_type_id).opt()))
        .fold([0, 0, i64::MIN], |a: [i64; 3], ((d, c), t)| [a[0] + c.is_some() as i64, a[1] + matches!(t, Some(10 | 11 | 12)) as i64, a[2].max(d)]);
    let mut v = Vec::new();
    (&ps).and((&ra).filt(|r: [i64; 3]| r[2] >= month_ago())).drive(|u, (p, r)| v.push((u, p, r)));
    out(v, |&(_, p, _)| Reverse(p[1]), 10, |&(u, p, r)| {
        let av = if p[2] == 0 { 0.0 } else { p[3] as f64 / p[2] as f64 };
        vec![user_col(db, u, "name"), V::I(p[0]), V::I(p[1]), V::F(round2(av)), V::I(r[0]), V::I(r[1]), V::T(r[2])]
    })
}

// WITH UserActivity AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(DISTINCT p.Id) AS PostCount,
// COUNT(DISTINCT c.Id) AS CommentCount,
// COUNT(DISTINCT b.Id) AS BadgeCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// LEFT JOIN
// Votes v ON p.Id = v.PostId AND v.UserId = u.Id
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
// ), UserPosts AS (
// SELECT
// ua.UserId,
// ua.DisplayName,
// ua.Reputation,
// COUNT(p.Id) AS TotalPosts,
// AVG(p.Score) AS AvgScore,
// SUM(p.ViewCount) AS TotalViews,
// MAX(p.CreationDate) AS LastPostDate
// FROM
// UserActivity ua
// JOIN
// Posts p ON ua.UserId = p.OwnerUserId
// GROUP BY
// ua.UserId, ua.DisplayName, ua.Reputation
// )
// SELECT
// up.UserId,
// up.DisplayName,
// up.Reputation,
// up.TotalPosts,
// up.AvgScore,
// up.TotalViews,
// up.LastPostDate,
// COALESCE(ua.PostCount, 0) AS UniquePosts,
// COALESCE(ua.CommentCount, 0) AS CommentsMade,
// COALESCE(ua.BadgeCount, 0) AS Badges,
// ua.UpVotes,
// ua.DownVotes
// FROM
// UserPosts up
// LEFT JOIN
// UserActivity ua ON up.UserId = ua.UserId
// ORDER BY
// up.Reputation DESC,
// up.TotalPosts DESC
// LIMIT 100;
fn q7998(db: &'static So) -> String {
    let sv = self_votes(db);
    let cp = comments_per_post(db);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let bu = badges_per_user(db);
    let dc = udc(db);
    let sf = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(comments_of(db).opt().and((&sv).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 2], |a, (p, _)| {
            let t = p.and_then(|(_, t)| t);
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
        });
    let Post { score, view_count, creation_date, .. } = &db.post;
    let up = owned(db).group_by(&db.post.owner_user).select(score.and(view_count.opt()).and(creation_date)).fold([0, 0, 0, 0, i64::MIN], |a: [i64; 5], ((s, w), c)| {
        [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4].max(c)]
    });
    let mut v = Vec::new();
    (&up).and(&dp).and((&dc).opt()).and(&bu).and((&sf).opt()).drive(|u, ((((p, d), c), b), s)| v.push((u, p, d, c.unwrap_or(0), b, s.unwrap_or([0; 2]))));
    out(v, |&(u, p, ..)| (rep_desc(db, u), Reverse(p[0])), 100, |&(u, p, d, c, b, s)| {
        let mut f = ["uid", "name", "rep"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.extend([V::I(p[0]), avg(p[1], p[0]), nullable(p[3], p[2]), V::T(p[4])]);
        f.extend(ints(&[d, c, b, s[0], s[1]]));
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
// SUM(COALESCE(P.ViewCount, 0)) AS TotalViews,
// SUM(COALESCE(V.VoteCount, 0)) AS TotalVotes
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
// GROUP BY
// U.Id, U.DisplayName
// ),
// PostEngagement AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.Score,
// COALESCE(COUNT(C.Id), 0) AS CommentCount,
// COALESCE(SUM(V.BountyAmount), 0) AS TotalBounties,
// P.OwnerUserId
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId AND V.VoteTypeId = 8
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.Score, P.OwnerUserId
// )
// SELECT
// UA.UserId,
// UA.DisplayName,
// UA.PostCount,
// UA.QuestionCount,
// UA.AnswerCount,
// UA.TotalViews,
// UA.TotalVotes,
// PE.PostId,
// PE.Title,
// PE.CreationDate,
// PE.Score,
// PE.CommentCount,
// PE.TotalBounties
// FROM
// UserActivity UA
// LEFT JOIN
// PostEngagement PE ON UA.UserId = PE.OwnerUserId
// ORDER BY
// UA.TotalVotes DESC, UA.TotalViews DESC;
fn q10043(db: &'static So) -> String {
    let vp = votes_per_post(db);
    let Post { post_type_id, view_count, .. } = &db.post;
    let ua = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(view_count.opt()).and(&vp)).fold([0i64; 5], |a, ((t, w), x)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.unwrap_or(0), a[4] + x]
    });
    let sf = stats_fold(db, owned(db), Ident::<Post>::new(), "cv", &[8]);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&ua).opt()).and(posts_of(db).select(Ident::<Post>::new().and(&sf)).opt())).drive(|_, ((u, a), p)| v.push((u, a.unwrap_or([0; 5]), p)));
    rows(v.iter().map(|&(u, a, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a));
        f.extend(match p {
            Some((p, s)) => {
                let mut g = post_fields(db, p, &["id", "title", "created", "score"]);
                g.extend(ints(&[s.cx, s.bounty_sum]));
                g
            }
            None => nulls(6),
        });
        row(f)
    }))
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
// PostStatistics AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS PostCount,
// SUM(P.Score) AS TotalScore,
// AVG(P.ViewCount) AS AvgViews,
// COUNT(DISTINCT P.Tags) AS UniqueTags
// FROM Posts P
// GROUP BY P.OwnerUserId
// ),
// CombinedStatistics AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COALESCE(UBC.BadgeCount, 0) AS BadgeCount,
// COALESCE(UBC.GoldBadges, 0) AS GoldBadges,
// COALESCE(UBC.SilverBadges, 0) AS SilverBadges,
// COALESCE(UBC.BronzeBadges, 0) AS BronzeBadges,
// COALESCE(PS.PostCount, 0) AS PostCount,
// COALESCE(PS.TotalScore, 0) AS TotalScore,
// COALESCE(PS.AvgViews, 0) AS AvgViews,
// COALESCE(PS.UniqueTags, 0) AS UniqueTags
// FROM Users U
// LEFT JOIN UserBadgeCounts UBC ON U.Id = UBC.UserId
// LEFT JOIN PostStatistics PS ON U.Id = PS.OwnerUserId
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
// AvgViews,
// UniqueTags
// FROM CombinedStatistics
// WHERE PostCount > 10
// ORDER BY TotalScore DESC, BadgeCount DESC
// LIMIT 50;
fn q6364(db: &'static So) -> String {
    let ub = ubc(db);
    let Post { score, view_count, .. } = &db.post;
    let ps = owned(db).group_by(&db.post.owner_user).select(score.and(view_count.opt())).fold([0i64; 4], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]);
    let dt = owned(db).group_by(&db.post.owner_user).select(&db.post.tags_str).count_distinct();
    let mut v = Vec::new();
    (&ub).and((&ps).filt(|p: [i64; 4]| p[0] > 10)).and((&dt).opt()).drive(|u, ((b, p), t)| v.push((u, b, p, t.unwrap_or(0))));
    out(v, |&(_, b, p, _)| (Reverse(p[1]), Reverse(b[0])), 50, |&(u, b, p, t)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&b));
        f.extend([V::I(p[0]), V::I(p[1]), or0(p[3], p[2]), V::I(t)]);
        f
    })
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN p.PostTypeId IN (1, 2) THEN p.Score ELSE 0 END) AS TotalScore,
// SUM(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName
// ),
// CommentStats AS (
// SELECT
// c.UserId,
// COUNT(c.Id) AS CommentCount,
// SUM(c.Score) AS TotalCommentScore
// FROM
// Comments c
// GROUP BY
// c.UserId
// ),
// BadgeCounts AS (
// SELECT
// b.UserId,
// COUNT(b.Id) AS BadgeCount,
// SUM(b.Class) AS TotalBadgeClass
// FROM
// Badges b
// GROUP BY
// b.UserId
// )
// SELECT
// ups.UserId,
// ups.DisplayName,
// ups.PostCount,
// ups.QuestionCount,
// ups.AnswerCount,
// ups.TotalScore,
// ups.AcceptedAnswers,
// COALESCE(cs.CommentCount, 0) AS CommentCount,
// COALESCE(cs.TotalCommentScore, 0) AS TotalCommentScore,
// COALESCE(bc.BadgeCount, 0) AS BadgeCount,
// COALESCE(bc.TotalBadgeClass, 0) AS TotalBadgeClass
// FROM
// UserPostStats ups
// LEFT JOIN
// CommentStats cs ON ups.UserId = cs.UserId
// LEFT JOIN
// BadgeCounts bc ON ups.UserId = bc.UserId
// ORDER BY
// ups.TotalScore DESC,
// COALESCE(bc.BadgeCount, 0) DESC,
// ups.PostCount DESC
// LIMIT 100;
fn q9958(db: &'static So) -> String {
    let Post { post_type_id, score, accepted_answer_id, .. } = &db.post;
    let uf = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(score).and(accepted_answer_id.opt())).fold([0i64; 5], |a, ((t, s), acc)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + if matches!(t, 1 | 2) { s } else { 0 }, a[4] + acc.is_some() as i64]
    });
    let cs = db.comment.group_by(&db.comment.user).select(&db.comment.score).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let bs = class_sum(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&uf).opt()).and((&cs).opt()).and((&bs).opt())).drive(|_, (((u, a), c), b)| v.push((u, a.unwrap_or([0; 5]), c.unwrap_or([0; 2]), b.unwrap_or((0, 0)))));
    out(v, |&(_, a, _, b)| (Reverse(a[3]), Reverse(b.0), Reverse(a[0])), 100, |&(u, a, c, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a));
        f.extend(ints(&[c[0], c[1], b.0, b.1]));
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
// SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END) AS PositiveScores,
// AVG(P.Score) AS AvgScore,
// SUM(COALESCE(B.Id, 0)) AS Badges
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// GROUP BY
// U.Id, U.DisplayName
// ),
// PostDetails AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.ViewCount,
// COALESCE(COUNT(C.Id), 0) AS CommentCount,
// COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
// COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
// P.OwnerUserId
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.ViewCount, P.OwnerUserId
// )
// SELECT
// US.UserId,
// US.DisplayName,
// US.TotalPosts,
// US.Questions,
// US.Answers,
// US.PositiveScores,
// US.AvgScore,
// PD.PostId,
// PD.Title,
// PD.CreationDate,
// PD.ViewCount,
// PD.CommentCount,
// PD.UpVotes,
// PD.DownVotes
// FROM
// UserStatistics US
// JOIN
// PostDetails PD ON US.UserId = PD.OwnerUserId
// ORDER BY
// US.TotalPosts DESC, PD.ViewCount DESC;
fn q14009(db: &'static So) -> String {
    let ps = pstat(db, db.post.iq());
    let pb = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(&db.post.score)).opt().and(badges_of(db).opt()))
        .fold([0i64; 5], |a, (p, _)| match p {
            Some((t, s)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (s > 0) as i64, a[3] + s, a[4] + 1],
            None => a,
        });
    let mut v = Vec::new();
    stats_fold(db, owned(db), Ident::<Post>::new(), "cv", &[])
        .and((&db.post.owner_user).select(Ident::<User>::new().and(&ps).and(&pb)))
        .drive(|p, (s, ((u, q), o))| v.push((p, s, u, q, o)));
    rows(v.iter().map(|&(p, s, u, q, o)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&[q[0], o[0], o[1], o[2]]));
        f.push(avg(o[3], o[4]));
        f.extend(post_fields(db, p, &["id", "title", "created", "views"]));
        f.extend(ints(&[s.cx, s.up, s.down]));
        row(f)
    }))
}

// WITH UserPostStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(P.Id) AS PostCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(V.BountyAmount) AS TotalBounty
// FROM Users U
// LEFT JOIN Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId = 8
// GROUP BY U.Id, U.DisplayName
// ),
// UserBadgeCounts AS (
// SELECT
// B.UserId,
// COUNT(CASE WHEN B.Class = 1 THEN 1 END) AS GoldBadges,
// COUNT(CASE WHEN B.Class = 2 THEN 1 END) AS SilverBadges,
// COUNT(CASE WHEN B.Class = 3 THEN 1 END) AS BronzeBadges
// FROM Badges B
// GROUP BY B.UserId
// ),
// PostClosureStats AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS ClosedPostCount
// FROM Posts P
// WHERE P.ClosedDate IS NOT NULL
// GROUP BY P.OwnerUserId
// )
// SELECT
// U.DisplayName,
// UPS.PostCount,
// UPS.QuestionCount,
// UPS.AnswerCount,
// COALESCE(UBC.GoldBadges, 0) AS GoldBadges,
// COALESCE(UBC.SilverBadges, 0) AS SilverBadges,
// COALESCE(UBC.BronzeBadges, 0) AS BronzeBadges,
// COALESCE(PCS.ClosedPostCount, 0) AS ClosedPostCount,
// UPS.TotalBounty
// FROM Users U
// LEFT JOIN UserPostStats UPS ON U.Id = UPS.UserId
// LEFT JOIN UserBadgeCounts UBC ON U.Id = UBC.UserId
// LEFT JOIN PostClosureStats PCS ON U.Id = PCS.OwnerUserId
// WHERE (UPS.PostCount > 5 OR UPS.TotalBounty > 0)
// AND U.Reputation > 100
// ORDER BY UPS.PostCount DESC, UPS.TotalBounty DESC
// LIMIT 10;
fn q4760(db: &'static So) -> String {
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(8))).select((&db.vote.bounty_amount).opt());
    let uf = user_base(db, UserWhere::RepGt(100))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(bounty.opt())).opt())
        .fold([0i64; 5], |a, p| match p {
            Some((t, b)) => {
                let b = b.flatten();
                [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + b.is_some() as i64, a[4] + b.unwrap_or(0)]
            }
            None => a,
        });
    let bc = badge_classes(db);
    let cl = db.post.with(&db.post.closed_date).group_by(&db.post.owner_user).select(&db.post.score).fold(0i64, |a, _| a + 1);
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(100))
        .select(Ident::<User>::new().and((&uf).filt(|a: [i64; 5]| a[0] > 5 || (a[3] > 0 && a[4] > 0))).and((&bc).opt()).and((&cl).opt()))
        .drive(|_, (((u, a), b), c)| v.push((u, a, bz(b), c.unwrap_or(0))));
    out(v, |&(_, a, ..)| (Reverse(a[0]), (a[3] == 0, Reverse(a[4]))), 10, |&(u, a, b, c)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(ints(&[a[0], a[1], a[2], b[1], b[2], b[3], c]));
        f.push(nullable(a[4], a[3]));
        f
    })
}

// WITH RecentPosts AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// p.AnswerCount,
// p.CommentCount,
// u.DisplayName AS AuthorName
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'
// AND p.PostTypeId = 1
// ),
// PostVotes AS (
// SELECT
// v.PostId,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Votes v
// GROUP BY
// v.PostId
// ),
// PostHistorySummaries AS (
// SELECT
// ph.PostId,
// COUNT(CASE WHEN ph.PostHistoryTypeId IN (10, 11, 12) THEN 1 END) AS CloseActionCount,
// COUNT(CASE WHEN ph.PostHistoryTypeId IN (24, 24) THEN 1 END) AS EditActionCount
// FROM
// PostHistory ph
// GROUP BY
// ph.PostId
// )
// SELECT
// rp.PostId,
// rp.Title,
// rp.CreationDate,
// rp.Score,
// rp.ViewCount,
// rp.AnswerCount,
// rp.CommentCount,
// COALESCE(pv.UpVotes, 0) AS TotalUpVotes,
// COALESCE(pv.DownVotes, 0) AS TotalDownVotes,
// COALESCE(phs.CloseActionCount, 0) AS TotalCloseActions,
// COALESCE(phs.EditActionCount, 0) AS TotalEditActions,
// rp.AuthorName
// FROM
// RecentPosts rp
// LEFT JOIN
// PostVotes pv ON rp.PostId = pv.PostId
// LEFT JOIN
// PostHistorySummaries phs ON rp.PostId = phs.PostId
// ORDER BY
// rp.Score DESC,
// rp.CreationDate DESC
// LIMIT 100;
fn q7725(db: &'static So) -> String {
    let pv = post_votes(db);
    let hf = db.post_history.group_by(&db.post_history.post).select(&db.post_history.post_history_type_id).fold([0i64; 2], |a, t| [a[0] + matches!(t, 10 | 11 | 12) as i64, a[1] + (t == 24) as i64]);
    let mut v = Vec::new();
    owned_since(db, month_ago()).with((&db.post.post_type_id).eq(1)).select(Ident::<Post>::new().and((&pv).opt()).and((&hf).opt())).drive(|_, ((p, x), h)| v.push((p, x.unwrap_or([0; 3]), h.unwrap_or([0; 2]))));
    out(v, |&(p, ..)| (score_desc(db, p), Reverse(db.post.creation_date.get(p).unwrap())), 100, |&(p, x, h)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments"]);
        f.extend(ints(&[x[1], x[2], h[0], h[1]]));
        f.extend(post_fields(db, p, &["owner"]));
        f
    })
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// U.CreationDate,
// COUNT(DISTINCT B.Id) AS BadgeCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes
// FROM
// Users U
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// LEFT JOIN
// Votes V ON U.Id = V.UserId
// GROUP BY
// U.Id, U.DisplayName, U.Reputation, U.CreationDate
// ),
// PostInteraction AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS PostCount,
// SUM(P.Score) AS TotalPostScore,
// SUM(P.ViewCount) AS TotalViews,
// MAX(P.CreationDate) AS LastPostDate
// FROM
// Posts P
// GROUP BY
// P.OwnerUserId
// ),
// UserPerformance AS (
// SELECT
// US.UserId,
// US.DisplayName,
// US.Reputation,
// US.CreationDate,
// US.BadgeCount,
// US.TotalUpvotes,
// US.TotalDownvotes,
// PI.PostCount,
// COALESCE(PI.TotalPostScore, 0) AS TotalPostScore,
// COALESCE(PI.TotalViews, 0) AS TotalViews,
// PI.LastPostDate
// FROM
// UserStats US
// LEFT JOIN
// PostInteraction PI ON US.UserId = PI.OwnerUserId
// )
// SELECT
// UserId,
// DisplayName,
// Reputation,
// CreationDate,
// BadgeCount,
// TotalUpvotes,
// TotalDownvotes,
// PostCount,
// TotalPostScore,
// TotalViews,
// LastPostDate
// FROM
// UserPerformance
// WHERE
// Reputation > 1000
// ORDER BY
// Reputation DESC, TotalPostScore DESC;
fn q6543(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let uv = user_bv(db);
    let Post { score, view_count, creation_date, .. } = &db.post;
    let pi = owned(db).group_by(&db.post.owner_user).select(score.and(view_count.opt()).and(creation_date)).fold([0, 0, 0, i64::MIN], |a: [i64; 4], ((s, w), c)| {
        [a[0] + 1, a[1] + s, a[2] + w.unwrap_or(0), a[3].max(c)]
    });
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and(&bu).and(&uv).and((&pi).opt())).drive(|_, (((u, b), x), p)| v.push((u, b, x, p)));
    rows(v.iter().map(|&(u, b, x, p)| {
        let mut f = ["uid", "name", "rep", "ucreated"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.extend(ints(&[b, x[1], x[2]]));
        f.extend(match p {
            Some(p) => vec![V::I(p[0]), V::I(p[1]), V::I(p[2]), V::T(p[3])],
            None => vec![V::Null, V::I(0), V::I(0), V::Null],
        });
        row(f)
    }))
}

// WITH UserScores AS (
// SELECT
// U.Id AS UserId,
// U.Reputation,
// COALESCE(SUM(V.BountyAmount), 0) AS TotalBounty,
// COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpVotes,
// COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownVotes
// FROM Users U
// LEFT JOIN Votes V ON U.Id = V.UserId
// GROUP BY U.Id, U.Reputation
// ),
// PopularPosts AS (
// SELECT
// P.Id AS PostId,
// P.OwnerUserId,
// P.Title,
// P.Score,
// P.ViewCount,
// COUNT(C.Id) AS CommentCount
// FROM Posts P
// LEFT JOIN Comments C ON P.Id = C.PostId
// WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY P.Id, P.OwnerUserId, P.Title, P.Score, P.ViewCount
// ORDER BY P.Score DESC, P.ViewCount DESC
// LIMIT 10
// ),
// PostContributions AS (
// SELECT
// PH.PostId,
// PH.UserId,
// COUNT(PH.Id) AS EditCount
// FROM PostHistory PH
// WHERE PH.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY PH.PostId, PH.UserId
// )
// SELECT
// U.DisplayName,
// U.Reputation,
// US.TotalBounty,
// US.TotalUpVotes,
// US.TotalDownVotes,
// PP.Title AS PopularPostTitle,
// PP.Score AS PostScore,
// PP.ViewCount AS PostViewCount,
// PC.EditCount AS UserEditCount
// FROM Users U
// JOIN UserScores US ON U.Id = US.UserId
// JOIN PopularPosts PP ON U.Id = PP.OwnerUserId
// LEFT JOIN PostContributions PC ON PP.PostId = PC.PostId AND U.Id = PC.UserId
// ORDER BY US.Reputation DESC, PP.Score DESC;
fn q9402(db: &'static So) -> String {
    let y = year_ago();
    let vf = db.vote.group_by(&db.vote.user).select((&db.vote.vote_type_id).and((&db.vote.bounty_amount).opt())).fold([0i64; 3], |a, (t, b)| [a[0] + b.unwrap_or(0), a[1] + (t == 2) as i64, a[2] + (t == 3) as i64]);
    let PostHistory { user, post, creation_date, .. } = &db.post_history;
    let pc = db.post_history.with(creation_date.ge(y)).with(user.and(post.select(&db.post.owner_user)).filt(|(a, b)| a == b)).group_by(post).select(creation_date).fold(0i64, |a, _| a + 1);
    let base = since(db, y);
    let top: MatSet<Id<Post>> = whole(&base)
        .select(Ident::<Post>::new().and(&db.post.score).and((&db.post.view_count).opt()))
        .window(row_number, |((_, s), w): ((Id<Post>, i64), Option<i64>)| (s, w.is_some(), w), desc)
        .filt(|(_, n)| n <= 10)
        .map(|(((p, _), _), _)| p)
        .collect();
    let mut w = Vec::new();
    (&top).select(Ident::<Post>::new().and((&db.post.owner_user).select(Ident::<User>::new().and((&vf).opt()))).and((&pc).opt())).drive(|_, x| w.push(x));
    rows(w.iter().map(|&((p, (u, x)), c)| {
        let x = x.unwrap_or([0; 3]);
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(ints(&x));
        f.extend(post_fields(db, p, &["title", "score", "views"]));
        f.push(oint(c));
        row(f)
    }))
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Users u
// LEFT JOIN
// Posts p ON p.OwnerUserId = u.Id
// LEFT JOIN
// Votes v ON v.PostId = p.Id
// GROUP BY
// u.Id, u.DisplayName
// ),
// PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.Score,
// p.ViewCount,
// p.CreationDate,
// COALESCE(pc.CommentCount, 0) AS CommentCount,
// COALESCE(pa.AnswerCount, 0) AS AnswerCount,
// p.OwnerUserId
// FROM
// Posts p
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount
// FROM Comments
// GROUP BY PostId) pc ON pc.PostId = p.Id
// LEFT JOIN
// (SELECT ParentId, COUNT(*) AS AnswerCount
// FROM Posts WHERE PostTypeId = 2
// GROUP BY ParentId) pa ON pa.ParentId = p.Id
// )
// SELECT
// us.UserId,
// us.DisplayName,
// us.PostCount,
// us.QuestionCount,
// us.AnswerCount,
// us.UpVotes,
// us.DownVotes,
// ps.PostId,
// ps.Title,
// ps.Score,
// ps.ViewCount,
// ps.CreationDate,
// ps.CommentCount,
// ps.AnswerCount
// FROM
// UserStats us
// LEFT JOIN
// PostStats ps ON us.UserId = ps.OwnerUserId
// ORDER BY
// us.UserId, us.PostCount DESC;
fn q14038(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let cp = comments_per_post(db);
    let ta = typed_answers_per_post(db);
    let mut v = Vec::new();
    (&us).and((&dp).opt()).and(posts_of(db).select(Ident::<Post>::new().and(&cp).and(&ta)).opt()).drive(|u, ((a, d), p)| v.push((u, a, d.unwrap_or(0), p)));
    rows(v.iter().map(|&(u, a, d, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&[d, a.q, a.a, a.up, a.down]));
        f.extend(match p {
            Some(((p, c), n)) => {
                let mut g = post_fields(db, p, &["id", "title", "score", "views", "created"]);
                g.extend(ints(&[c, n]));
                g
            }
            None => nulls(7),
        });
        row(f)
    }))
}

// WITH UserReputation AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(b.Id) AS BadgeCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM Users u
// LEFT JOIN Badges b ON u.Id = b.UserId
// LEFT JOIN Votes v ON u.Id = v.UserId
// GROUP BY u.Id, u.DisplayName, u.Reputation
// ),
// PostStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositiveScoreCount
// FROM Posts p
// GROUP BY p.OwnerUserId
// ),
// UserPerformance AS (
// SELECT
// ur.UserId,
// ur.DisplayName,
// ur.Reputation,
// ur.BadgeCount,
// ur.UpVotes,
// ur.DownVotes,
// COALESCE(ps.PostCount, 0) AS PostCount,
// COALESCE(ps.QuestionCount, 0) AS QuestionCount,
// COALESCE(ps.PositiveScoreCount, 0) AS PositiveScoreCount
// FROM UserReputation ur
// LEFT JOIN PostStats ps ON ur.UserId = ps.OwnerUserId
// )
// SELECT
// UserId,
// DisplayName,
// Reputation,
// BadgeCount,
// UpVotes,
// DownVotes,
// PostCount,
// QuestionCount,
// PositiveScoreCount,
// CASE
// WHEN Reputation IS NULL OR Reputation < 0 THEN 'Newbie'
// WHEN Reputation < 1000 THEN 'Intermediate'
// WHEN Reputation < 5000 THEN 'Expert'
// ELSE 'Pro'
// END AS UserLevel
// FROM UserPerformance
// ORDER BY Reputation DESC, BadgeCount DESC;
fn q5985(db: &'static So) -> String {
    let uv = user_bv(db);
    let ps = owned(db).group_by(&db.post.owner_user).select((&db.post.post_type_id).and(&db.post.score)).fold([0i64; 3], |a, (t, s)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (s > 0) as i64]);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and(&uv).and((&ps).opt())).drive(|_, ((u, x), p)| v.push((u, x, p.unwrap_or([0; 3]))));
    rows(v.iter().map(|&(u, x, p)| {
        let r = db.user.reputation.get(u).unwrap();
        let lvl = if r < 0 { "Newbie" } else if r < 1000 { "Intermediate" } else if r < 5000 { "Expert" } else { "Pro" };
        let mut f = ["uid", "name", "rep"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.extend(ints(&[x[0], x[1], x[2]]));
        f.extend(ints(&p));
        f.push(V::S(lvl));
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
// JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// u.Id, u.DisplayName
// ),
// ActivePosts AS (
// SELECT
// p.Id AS PostId,
// p.OwnerUserId,
// p.Title,
// p.Score,
// p.CreationDate,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'
// GROUP BY
// p.Id, p.OwnerUserId, p.Title, p.Score, p.CreationDate
// ),
// EngagedUsers AS (
// SELECT
// DISTINCT u.Id AS UserId,
// u.DisplayName,
// ub.BadgeCount,
// ap.PostId,
// ap.Title,
// ap.Score,
// ap.CommentCount,
// ap.UpVoteCount
// FROM
// Users u
// JOIN
// UserBadges ub ON u.Id = ub.UserId
// JOIN
// ActivePosts ap ON u.Id = ap.OwnerUserId
// )
// SELECT
// eu.UserId,
// eu.DisplayName,
// eu.BadgeCount,
// COUNT(ap.PostId) AS ActivePostCount,
// AVG(ap.Score) AS AveragePostScore,
// SUM(ap.CommentCount) AS TotalCommentCount,
// SUM(ap.UpVoteCount) AS TotalUpVoteCount
// FROM
// EngagedUsers eu
// JOIN
// ActivePosts ap ON eu.UserId = ap.OwnerUserId
// GROUP BY
// eu.UserId, eu.DisplayName, eu.BadgeCount
// ORDER BY
// TotalUpVoteCount DESC, ActivePostCount DESC
// LIMIT 10;
fn q9632(db: &'static So) -> String {
    let bu = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold(0i64, |a, _| a + 1);
    let sf = stats_fold(db, owned_since(db, month_ago()), Ident::<Post>::new(), "cv", &[]);
    let active: HashIdx<Id<User>, Id<Post>> = owned_since(db, month_ago()).select(&db.post.owner_user).inv().collect();
    let eu: MatSet<(Id<User>, Id<Post>)> = db.user.with(&bu).select(Ident::<User>::new().and(&active)).collect();
    type R = (Id<User>, Id<Post>);
    let af = (&eu)
        .group_by(Same::<R>::new().map(|(u, _): R| u))
        .select(Same::<R>::new().map(|(u, _): R| u).select(&active).select((&sf).and(&db.post.score)))
        .fold([0i64; 4], |a, (s, sc)| [a[0] + 1, a[1] + sc, a[2] + s.cx, a[3] + s.up]);
    let mut v = Vec::new();
    (&af).and(&bu).drive(|u, (a, b)| v.push((u, a, b)));
    out(v, |&(_, a, _)| (Reverse(a[3]), Reverse(a[0])), 10, |&(u, a, b)| {
        vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(b), V::I(a[0]), avg(a[1], a[0]), V::I(a[2]), V::I(a[3])]
    })
}

// WITH PostStatistics AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// COUNT(b.Id) AS BadgeCount
// FROM
// Posts p
// LEFT JOIN Comments c ON p.Id = c.PostId
// LEFT JOIN Votes v ON p.Id = v.PostId
// LEFT JOIN Badges b ON p.OwnerUserId = b.UserId
// WHERE
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY
// p.Id, p.Title, p.CreationDate
// ),
// UserStatistics AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT b.Id) AS BadgeCount,
// SUM(u.UpVotes) AS TotalUpVotes,
// SUM(u.DownVotes) AS TotalDownVotes,
// COUNT(DISTINCT p.Id) AS PostCount
// FROM
// Users u
// LEFT JOIN Badges b ON u.Id = b.UserId
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
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
// us.UserId,
// us.DisplayName,
// us.BadgeCount AS UserBadgeCount,
// us.TotalUpVotes AS UserTotalUpVotes,
// us.TotalDownVotes AS UserTotalDownVotes,
// us.PostCount AS UserPostCount
// FROM
// PostStatistics ps
// JOIN
// Users u ON ps.PostId = u.Id
// JOIN
// UserStatistics us ON u.Id = us.UserId
// ORDER BY
// ps.CreationDate DESC
// LIMIT 100;
fn q13250(db: &'static So) -> String {
    let uid = uids(db);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let bu = badges_per_user(db);
    let ud2 = db
        .user
        .group_by(Ident::<User>::new())
        .select((&db.user.up_votes).and(&db.user.down_votes).and(badges_of(db).opt()).and(posts_of(db).opt()))
        .fold([0i64; 2], |a, (((u, d), _), _)| [a[0] + u, a[1] + d]);
    let mut v = Vec::new();
    stats_fold(db, since(db, year_ago()).with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cvb", &[])
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and((&dp).opt()).and(&bu).and(&ud2)))
        .drive(|p, (s, (((u, d), b), x))| v.push((p, s, u, d.unwrap_or(0), b, x)));
    out(v, |&(p, ..)| Reverse(db.post.creation_date.get(p).unwrap()), 100, |&(p, s, u, d, b, x)| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend(ints(&[s.cx, s.vx, s.up, s.down]));
        f.extend([user_col(db, u, "uid"), user_col(db, u, "name")]);
        f.extend(ints(&[b, x[0], x[1], d]));
        f
    })
}

// WITH UserPostMetrics AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS TotalAcceptedAnswers,
// SUM(p.ViewCount) AS TotalViews,
// SUM(p.Score) AS TotalScore,
// AVG(COALESCE(p.CommentCount, 0)) AS AvgCommentsPerPost
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName
// ),
// UserBadgeMetrics AS (
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
// upm.UserId,
// upm.DisplayName,
// upm.TotalPosts,
// upm.TotalQuestions,
// upm.TotalAnswers,
// upm.TotalAcceptedAnswers,
// upm.TotalViews,
// upm.TotalScore,
// upm.AvgCommentsPerPost,
// COALESCE(ubm.TotalBadges, 0) AS TotalBadges,
// COALESCE(ubm.TotalGoldBadges, 0) AS TotalGoldBadges,
// COALESCE(ubm.TotalSilverBadges, 0) AS TotalSilverBadges,
// COALESCE(ubm.TotalBronzeBadges, 0) AS TotalBronzeBadges
// FROM
// UserPostMetrics upm
// LEFT JOIN
// UserBadgeMetrics ubm ON upm.UserId = ubm.UserId
// ORDER BY
// upm.TotalPosts DESC;
fn q14827(db: &'static So) -> String {
    let Post { post_type_id, accepted_answer_id, view_count, score, comment_count, .. } = &db.post;
    let uf = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(accepted_answer_id.opt()).and(view_count.opt()).and(score).and(comment_count)).fold([0i64; 8], |a, ((((t, acc), w), s), c)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + acc.is_some() as i64, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0), a[6] + s, a[7] + c]
    });
    let bc = badge_classes(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&uf).opt()).and((&bc).opt())).drive(|_, ((u, a), b)| v.push((u, a.unwrap_or([0; 8]), bz(b))));
    rows(v.iter().map(|&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&[a[0], a[1], a[2], a[3]]));
        f.extend([nullable(a[5], a[4]), nullable(a[6], a[0]), or0(a[7], a[0])]);
        f.extend(ints(&b));
        row(f)
    }))
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
// PostStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS PostCount,
// SUM(p.ViewCount) AS TotalViews,
// SUM(p.Score) AS TotalScore,
// AVG(p.Score) AS AverageScore
// FROM
// Posts p
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY
// p.OwnerUserId
// ),
// UserPerformance AS (
// SELECT
// ub.UserId,
// ub.DisplayName,
// COALESCE(ps.PostCount, 0) AS PostCount,
// COALESCE(ps.TotalViews, 0) AS TotalViews,
// COALESCE(ps.TotalScore, 0) AS TotalScore,
// COALESCE(ps.AverageScore, 0) AS AverageScore,
// ub.BadgeCount,
// ub.GoldCount,
// ub.SilverCount,
// ub.BronzeCount
// FROM
// UserBadges ub
// LEFT JOIN
// PostStats ps ON ub.UserId = ps.OwnerUserId
// )
// SELECT
// up.DisplayName,
// up.PostCount,
// up.TotalViews,
// up.TotalScore,
// up.AverageScore,
// up.BadgeCount,
// up.GoldCount,
// up.SilverCount,
// up.BronzeCount
// FROM
// UserPerformance up
// ORDER BY
// up.TotalScore DESC,
// up.TotalViews DESC,
// up.PostCount DESC
// LIMIT 10;
fn q9491(db: &'static So) -> String {
    let ub = ubc(db);
    let ps = pstat(db, since(db, year_ago()));
    let mut v = Vec::new();
    (&ub).and((&ps).opt()).drive(|u, (b, p)| v.push((u, b, pz(p))));
    out(v, |&(_, _, p)| (Reverse(p[3]), Reverse(p[5]), Reverse(p[0])), 10, |&(u, b, p)| {
        let mut f = vec![user_col(db, u, "name"), V::I(p[0]), V::I(p[5]), V::I(p[3]), or0(p[3], p[0])];
        f.extend(ints(&b));
        f
    })
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// u.CreationDate,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(p.ViewCount) AS TotalViews,
// SUM(p.Score) AS TotalScore
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// GROUP BY u.Id, u.Reputation, u.CreationDate
// ),
// BadgeStats AS (
// SELECT
// b.UserId,
// COUNT(*) AS BadgeCount,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM Badges b
// GROUP BY b.UserId
// ),
// CombinedStats AS (
// SELECT
// u.UserId,
// u.Reputation,
// u.CreationDate,
// u.PostCount,
// u.QuestionCount,
// u.AnswerCount,
// u.TotalViews,
// u.TotalScore,
// COALESCE(b.BadgeCount, 0) AS BadgeCount,
// COALESCE(b.GoldBadges, 0) AS GoldBadges,
// COALESCE(b.SilverBadges, 0) AS SilverBadges,
// COALESCE(b.BronzeBadges, 0) AS BronzeBadges
// FROM UserStats u
// LEFT JOIN BadgeStats b ON u.UserId = b.UserId
// )
// SELECT
// UserId,
// Reputation,
// CreationDate,
// PostCount,
// QuestionCount,
// AnswerCount,
// TotalViews,
// TotalScore,
// BadgeCount,
// GoldBadges,
// SilverBadges,
// BronzeBadges
// FROM CombinedStats
// ORDER BY Reputation DESC
// FETCH FIRST 10 ROWS ONLY;
fn q12311(db: &'static So) -> String {
    let ps = pstat(db, db.post.iq());
    let bc = badge_classes(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&ps).opt()).and((&bc).opt())).drive(|_, ((u, p), b)| v.push((u, pz(p), bz(b))));
    out(v, |&(u, ..)| rep_desc(db, u), 10, |&(u, p, b)| {
        let mut f = ["uid", "rep", "ucreated"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.extend([V::I(p[0]), V::I(p[1]), V::I(p[2]), pviews(p), nullable(p[3], p[0])]);
        f.extend(ints(&b));
        f
    })
}

// WITH UserReputation AS (
// SELECT
// U.Id AS UserId,
// U.Reputation,
// COUNT(DISTINCT P.Id) AS PostCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id, U.Reputation
// ),
// BadgeSummary AS (
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
// TopUsers AS (
// SELECT
// UR.UserId,
// UR.Reputation,
// UR.PostCount,
// UR.QuestionCount,
// UR.AnswerCount,
// COALESCE(BS.BadgeCount, 0) AS BadgeCount,
// COALESCE(BS.GoldBadges, 0) AS GoldBadges,
// COALESCE(BS.SilverBadges, 0) AS SilverBadges,
// COALESCE(BS.BronzeBadges, 0) AS BronzeBadges
// FROM
// UserReputation UR
// LEFT JOIN
// BadgeSummary BS ON UR.UserId = BS.UserId
// ORDER BY
// UR.Reputation DESC, UR.PostCount DESC
// LIMIT 10
// )
// SELECT
// U.DisplayName,
// U.Reputation,
// T.PostCount,
// T.QuestionCount,
// T.AnswerCount,
// T.BadgeCount,
// T.GoldBadges,
// T.SilverBadges,
// T.BronzeBadges
// FROM
// TopUsers T
// JOIN
// Users U ON T.UserId = U.Id
// WHERE
// T.QuestionCount > 5
// ORDER BY
// T.Reputation DESC;
fn q6563(db: &'static So) -> String {
    let ps = pstat(db, db.post.iq());
    let bc = badge_classes(db);
    let top: MatSet<Id<User>> = whole(&db.user.id)
        .select(Ident::<User>::new().and(&db.user.reputation).and((&ps).opt()))
        .window(row_number, |((_, r), p): ((Id<User>, i64), Option<[i64; 13]>)| (r, pz(p)[0]), desc)
        .filt(|(_, n)| n <= 10)
        .map(|(((u, _), _), _)| u)
        .collect();
    let mut v = Vec::new();
    (&top).select(Ident::<User>::new().and(&ps).and((&bc).opt())).filt(|((_, p), _): ((Id<User>, [i64; 13]), Option<[i64; 4]>)| p[1] > 5).drive(|_, ((u, p), b)| v.push((u, p, bz(b))));
    rows(v.iter().map(|&(u, p, b)| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(ints(&[p[0], p[1], p[2]]));
        f.extend(ints(&b));
        row(f)
    }))
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// U.CreationDate,
// COUNT(DISTINCT P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(P.ViewCount) AS TotalViews,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// U.Id, U.DisplayName, U.Reputation, U.CreationDate
// ),
// BadgesStats AS (
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
// US.UserId,
// US.DisplayName,
// US.Reputation,
// US.TotalPosts,
// US.Questions,
// US.Answers,
// US.TotalViews,
// US.UpVotes,
// US.DownVotes,
// COALESCE(BS.TotalBadges, 0) AS TotalBadges,
// COALESCE(BS.GoldBadges, 0) AS GoldBadges,
// COALESCE(BS.SilverBadges, 0) AS SilverBadges,
// COALESCE(BS.BronzeBadges, 0) AS BronzeBadges
// FROM
// UserStats US
// LEFT JOIN
// BadgesStats BS ON US.UserId = BS.UserId
// WHERE
// US.Reputation > 1000
// ORDER BY
// US.Reputation DESC, US.TotalPosts DESC;
fn q5178(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::RepGt(1000), "v", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&us).and((&dp).opt()).and((&bc).opt()).drive(|u, ((a, d), b)| v.push((u, a, d.unwrap_or(0), bz(b))));
    rows(v.iter().map(|&(u, a, d, b)| {
        let mut f = ["uid", "name", "rep"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.extend([V::I(d), V::I(a.q), V::I(a.a), ustat_field(&a, "views_sum"), V::I(a.up), V::I(a.down)]);
        f.extend(ints(&b));
        row(f)
    }))
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(DISTINCT P.Id) AS PostCount,
// SUM(COALESCE(P.Score, 0)) AS TotalScore,
// SUM(U.UpVotes) AS TotalUpVotes,
// SUM(U.DownVotes) AS TotalDownVotes,
// MAX(P.CreationDate) AS LastPostDate
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id, U.DisplayName
// ),
// PostStats AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.PostTypeId,
// P.ViewCount,
// P.AnswerCount,
// P.CommentCount,
// P.FavoriteCount,
// P.LastActivityDate,
// CASE
// WHEN P.PostTypeId = 1 THEN 'Question'
// WHEN P.PostTypeId = 2 THEN 'Answer'
// ELSE 'Other'
// END AS PostType,
// P.OwnerUserId
// FROM
// Posts P
// ),
// VoteStats AS (
// SELECT
// V.PostId,
// COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVotes,
// COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVotes
// FROM
// Votes V
// GROUP BY
// V.PostId
// )
// SELECT
// US.UserId,
// US.DisplayName,
// US.PostCount,
// US.TotalScore,
// US.TotalUpVotes,
// US.TotalDownVotes,
// PS.PostId,
// PS.Title,
// PS.PostType,
// PS.ViewCount,
// PS.AnswerCount,
// PS.CommentCount,
// PS.FavoriteCount,
// PS.LastActivityDate,
// VS.UpVotes,
// VS.DownVotes
// FROM
// UserStats US
// JOIN
// PostStats PS ON US.UserId = PS.OwnerUserId
// LEFT JOIN
// VoteStats VS ON PS.PostId = VS.PostId
// ORDER BY
// US.TotalScore DESC, US.PostCount DESC;
fn q11632(db: &'static So) -> String {
    let Post { score, creation_date, .. } = &db.post;
    let uf = owned(db)
        .group_by(&db.post.owner_user)
        .select(score.and(creation_date).and((&db.post.owner_user).select((&db.user.up_votes).and(&db.user.down_votes))))
        .fold([0, 0, i64::MIN, 0, 0], |a: [i64; 5], ((s, c), (u, d))| [a[0] + 1, a[1] + s, a[2].max(c), a[3] + u, a[4] + d]);
    let pv = post_votes(db);
    let mut v = Vec::new();
    owned(db).select(Ident::<Post>::new().and((&pv).opt()).and((&db.post.owner_user).select(Ident::<User>::new().and(&uf)))).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((p, x), (u, a))| {
        let t = db.post.post_type_id.get(p).unwrap();
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&[a[0], a[1], a[3], a[4]]));
        f.extend(post_fields(db, p, &["id", "title"]));
        f.push(V::S(if t == 1 { "Question" } else if t == 2 { "Answer" } else { "Other" }));
        f.extend(post_fields(db, p, &["views", "answers", "comments", "favorites", "activity"]));
        f.extend(x.map_or(nulls(2), |x| ints(&[x[1], x[2]])));
        row(f)
    }))
}

// WITH UserVotes AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(v.Id) AS VoteCount
// FROM Users u
// LEFT JOIN Votes v ON u.Id = v.UserId
// GROUP BY u.Id, u.DisplayName
// ),
// PostDetails AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// COUNT(c.Id) AS CommentCount,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes
// FROM Posts p
// LEFT JOIN Comments c ON p.Id = c.PostId
// LEFT JOIN Votes v ON p.Id = v.PostId
// GROUP BY p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount
// ),
// UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(COALESCE(pd.Score, 0)) AS TotalScore,
// SUM(COALESCE(pd.ViewCount, 0)) AS TotalViews,
// SUM(COALESCE(pd.CommentCount, 0)) AS TotalComments,
// SUM(COALESCE(pd.UpVotes, 0)) AS TotalUpVotes,
// SUM(COALESCE(pd.DownVotes, 0)) AS TotalDownVotes
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN PostDetails pd ON p.Id = pd.PostId
// GROUP BY u.Id, u.DisplayName
// )
// SELECT
// u.UserId,
// u.DisplayName,
// u.PostCount,
// u.TotalScore,
// u.TotalViews,
// u.TotalComments,
// u.TotalUpVotes,
// u.TotalDownVotes,
// COALESCE(v.VoteCount, 0) AS TotalVotes
// FROM UserPostStats u
// LEFT JOIN UserVotes v ON u.UserId = v.UserId
// ORDER BY u.TotalScore DESC, u.PostCount DESC
// LIMIT 100;
fn q14531(db: &'static So) -> String {
    let sf = stats_fold(db, owned(db), &db.post.owner_user, "cv", &[]);
    let ps = pstat(db, db.post.iq());
    let vc = votes_per_user(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&ps).opt()).and((&sf).opt()).and(&vc)).drive(|_, (((u, p), s), c)| v.push((u, pz(p), s, c)));
    out(v, |&(_, p, ..)| (Reverse(p[3]), Reverse(p[0])), 100, |&(u, p, s, c)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&[p[0], p[3], p[5]]));
        f.extend(s.map_or(ints(&[0, 0, 0]), |s| ints(&[s.cx, s.up, s.down])));
        f.push(V::I(c));
        f
    })
}

// WITH RankedPosts AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// COUNT(DISTINCT c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= CURRENT_DATE - INTERVAL '1 year'
// GROUP BY
// p.Id, p.Title, p.CreationDate
// ),
// UserActivity AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS PostsCount,
// COUNT(DISTINCT b.Id) AS BadgesCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// u.CreationDate >= CURRENT_DATE - INTERVAL '1 year'
// GROUP BY
// u.Id, u.DisplayName
// )
// SELECT
// rp.PostId,
// rp.Title,
// rp.CreationDate,
// rp.CommentCount,
// rp.VoteCount,
// rp.UpVoteCount,
// rp.DownVoteCount,
// ua.UserId,
// ua.DisplayName,
// ua.PostsCount,
// ua.BadgesCount,
// ua.TotalUpVotes,
// ua.TotalDownVotes
// FROM
// RankedPosts rp
// JOIN
// UserActivity ua ON rp.PostId = ua.UserId
// ORDER BY
// rp.CreationDate DESC;
fn q10360(db: &'static So) -> String {
    let uid = uids(db);
    let cut = add_years(current_date(), -1);
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::CreatedGe(cut), "vb", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    stats_fold(db, db.post.with((&db.post.creation_date).ge(cut)), Ident::<Post>::new(), "cv", &[])
        .and(comments_per_post(db))
        .and(votes_per_post(db))
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&us).and((&dp).opt()).and(&bu)))
        .drive(|p, (((s, c), x), (((u, a), d), b))| v.push((p, s, c, x, u, a, d.unwrap_or(0), b)));
    rows(v.iter().map(|&(p, s, c, x, u, a, d, b)| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend(ints(&[c, x, s.up, s.down]));
        f.extend([user_col(db, u, "uid"), user_col(db, u, "name")]);
        f.extend(ints(&[d, b, a.up, a.down]));
        row(f)
    }))
}

// WITH UserBadgeStats AS (
// SELECT
// U.Id AS UserId,
// U.Reputation,
// COUNT(B.Id) AS BadgeCount,
// SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM
// Users U
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// GROUP BY
// U.Id, U.Reputation
// ),
// PostStats AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS PostCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(P.Score) AS TotalScore,
// AVG(P.Score) AS AvgScore
// FROM
// Posts P
// WHERE
// P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY
// P.OwnerUserId
// )
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// U.Views,
// U.UpVotes,
// U.DownVotes,
// U.LastAccessDate,
// COALESCE(BadgeCount, 0) AS BadgeCount,
// COALESCE(GoldBadges, 0) AS GoldBadges,
// COALESCE(SilverBadges, 0) AS SilverBadges,
// COALESCE(BronzeBadges, 0) AS BronzeBadges,
// COALESCE(PostCount, 0) AS PostCount,
// COALESCE(Questions, 0) AS Questions,
// COALESCE(Answers, 0) AS Answers,
// COALESCE(TotalScore, 0) AS TotalScore,
// COALESCE(AvgScore, 0.0) AS AvgScore
// FROM
// Users U
// LEFT JOIN
// UserBadgeStats UB ON U.Id = UB.UserId
// LEFT JOIN
// PostStats PS ON U.Id = PS.OwnerUserId
// ORDER BY
// U.Reputation DESC
// LIMIT 100;
fn q13948(db: &'static So) -> String {
    let bc = badge_classes(db);
    let ps = pstat(db, since(db, year_ago()));
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&bc).opt()).and((&ps).opt())).drive(|_, ((u, b), p)| v.push((u, bz(b), pz(p))));
    out(v, |&(u, ..)| rep_desc(db, u), 100, |&(u, b, p)| {
        let mut f = ["uid", "name", "rep", "uviews", "uup", "udown", "last_access"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.extend(ints(&b));
        f.extend(ints(&[p[0], p[1], p[2], p[3]]));
        f.push(or0(p[3], p[0]));
        f
    })
}

// WITH UserBadgeCounts AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(CASE WHEN B.Class = 1 THEN 1 END) AS GoldBadges,
// COUNT(CASE WHEN B.Class = 2 THEN 1 END) AS SilverBadges,
// COUNT(CASE WHEN B.Class = 3 THEN 1 END) AS BronzeBadges
// FROM Users U
// LEFT JOIN Badges B ON U.Id = B.UserId
// GROUP BY U.Id, U.DisplayName
// ),
// PostSummary AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// AVG(P.Score) AS AvgScore
// FROM Posts P
// WHERE P.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year')
// GROUP BY P.OwnerUserId
// ),
// UserStatistics AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// U.Views,
// COALESCE(UBC.TotalPosts, 0) AS TotalPosts,
// COALESCE(UBC.Questions, 0) AS TotalQuestions,
// COALESCE(UBC.Answers, 0) AS TotalAnswers,
// UBC2.GoldBadges,
// UBC2.SilverBadges,
// UBC2.BronzeBadges
// FROM Users U
// LEFT JOIN PostSummary UBC ON U.Id = UBC.OwnerUserId
// LEFT JOIN UserBadgeCounts UBC2 ON U.Id = UBC2.UserId
// )
// SELECT
// US.UserId,
// US.DisplayName,
// US.Reputation,
// US.Views,
// US.TotalPosts,
// US.TotalQuestions,
// US.TotalAnswers,
// US.GoldBadges,
// US.SilverBadges,
// US.BronzeBadges
// FROM UserStatistics US
// WHERE (US.TotalPosts > 0 OR US.Reputation > 1000)
// ORDER BY US.Reputation DESC, US.TotalPosts DESC
// LIMIT 50;
fn q4531(db: &'static So) -> String {
    let bc = badge_classes(db);
    let ps = pstat(db, since(db, year_ago()));
    let mut v = Vec::new();
    db.user
        .select(Ident::<User>::new().and(&db.user.reputation).and((&bc).opt()).and((&ps).opt()))
        .filt(|(((_, r), _), p): (((Id<User>, i64), Option<[i64; 4]>), Option<[i64; 13]>)| pz(p)[0] > 0 || r > 1000)
        .drive(|_, (((u, _), b), p)| v.push((u, bz(b), pz(p))));
    out(v, |&(u, _, p)| (rep_desc(db, u), Reverse(p[0])), 50, |&(u, b, p)| {
        let mut f = ["uid", "name", "rep", "uviews"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.extend(ints(&[p[0], p[1], p[2], b[1], b[2], b[3]]));
        f
    })
}

// WITH UserAggregate AS (
// SELECT
// U.Id AS UserId,
// COUNT(DISTINCT P.Id) AS PostCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(COALESCE(V.BountyAmount, 0)) AS TotalBounties,
// AVG(COALESCE(P.Score, 0)) AS AverageScore
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// U.Id
// ),
// PostAggregate AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.LastActivityDate,
// P.ViewCount,
// P.Score,
// COALESCE(COUNT(C.Id), 0) AS CommentCount
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.LastActivityDate, P.ViewCount, P.Score
// ),
// TopUsers AS (
// SELECT
// UA.UserId,
// UA.PostCount,
// UA.QuestionCount,
// UA.AnswerCount,
// UA.TotalBounties,
// UA.AverageScore
// FROM
// UserAggregate UA
// ORDER BY
// UA.PostCount DESC
// LIMIT 10
// )
// SELECT
// TU.UserId,
// TU.PostCount,
// TU.QuestionCount,
// TU.AnswerCount,
// TU.TotalBounties,
// TU.AverageScore,
// PA.Title,
// PA.CreationDate,
// PA.LastActivityDate,
// PA.ViewCount,
// PA.Score,
// PA.CommentCount
// FROM
// TopUsers TU
// JOIN
// Posts P ON TU.UserId = P.OwnerUserId
// JOIN
// PostAggregate PA ON P.Id = PA.PostId
// ORDER BY
// TU.TotalBounties DESC, TU.AverageScore DESC;
fn q14483(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let dp: &'static Fold<Id<User>, i64> = Box::leak(Box::new(ud(db, UserWhere::All, posts_of(db))));
    let cp = comments_per_post(db);
    let top = top_by(dp, 10);
    let mut w = Vec::new();
    (&top).select(Ident::<User>::new().and(&us).and(dp).and(posts_of(db).select(Ident::<Post>::new().and(&cp)))).drive(|_, x| w.push(x));
    rows(w.iter().map(|&(((u, a), d), (p, c))| {
        let mut f = vec![user_col(db, u, "uid")];
        f.extend(ints(&[d, a.q, a.a, a.bounty_sum]));
        f.push(ustat_field(&a, "score_avg_rows"));
        f.extend(post_fields(db, p, &["title", "created", "activity", "views", "score"]));
        f.push(V::I(c));
        row(f)
    }))
}

// WITH UserStatistics AS (
// SELECT
// u.Id AS UserId,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN Votes v ON p.Id = v.PostId
// GROUP BY u.Id
// ),
// BadgeStatistics AS (
// SELECT
// UserId,
// COUNT(*) AS BadgeCount,
// SUM(CASE WHEN Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM Badges
// GROUP BY UserId
// )
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COALESCE(us.PostCount, 0) AS PostCount,
// COALESCE(us.QuestionCount, 0) AS QuestionCount,
// COALESCE(us.AnswerCount, 0) AS AnswerCount,
// COALESCE(us.TotalBounty, 0) AS TotalBounty,
// COALESCE(us.UpVotes, 0) AS UpVotes,
// COALESCE(us.DownVotes, 0) AS DownVotes,
// COALESCE(bs.BadgeCount, 0) AS BadgeCount,
// COALESCE(bs.GoldBadges, 0) AS GoldBadges,
// COALESCE(bs.SilverBadges, 0) AS SilverBadges,
// COALESCE(bs.BronzeBadges, 0) AS BronzeBadges
// FROM Users u
// LEFT JOIN UserStatistics us ON u.Id = us.UserId
// LEFT JOIN BadgeStatistics bs ON u.Id = bs.UserId
// ORDER BY u.Reputation DESC
// LIMIT 100;
fn q14834(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&us).and((&dp).opt()).and((&bc).opt()).drive(|u, ((a, d), b)| v.push((u, a, d.unwrap_or(0), bz(b))));
    out(v, |&(u, ..)| rep_desc(db, u), 100, |&(u, a, d, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&[d, a.q, a.a, a.bounty_sum, a.up, a.down]));
        f.extend(ints(&b));
        f
    })
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// ROUND(AVG(p.Score), 2) AS AvgPostScore
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// GROUP BY u.Id, u.DisplayName, u.Reputation
// ),
// BadgesGranted AS (
// SELECT
// b.UserId,
// COUNT(b.Id) AS TotalBadges,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM Badges b
// GROUP BY b.UserId
// ),
// CombinedStats AS (
// SELECT
// us.UserId,
// us.DisplayName,
// us.Reputation,
// us.PostCount,
// us.AnswerCount,
// us.QuestionCount,
// us.AvgPostScore,
// COALESCE(bg.TotalBadges, 0) AS TotalBadges,
// COALESCE(bg.GoldBadges, 0) AS GoldBadges,
// COALESCE(bg.SilverBadges, 0) AS SilverBadges,
// COALESCE(bg.BronzeBadges, 0) AS BronzeBadges
// FROM UserStats us
// LEFT JOIN BadgesGranted bg ON us.UserId = bg.UserId
// )
// SELECT
// cs.DisplayName,
// cs.Reputation,
// cs.PostCount,
// cs.AnswerCount,
// cs.QuestionCount,
// cs.AvgPostScore,
// cs.TotalBadges,
// cs.GoldBadges,
// cs.SilverBadges,
// cs.BronzeBadges
// FROM CombinedStats cs
// WHERE cs.Reputation > 1000
// ORDER BY cs.Reputation DESC, cs.PostCount DESC
// LIMIT 10;
fn q1703(db: &'static So) -> String {
    let ps = pstat(db, db.post.iq());
    let bc = badge_classes(db);
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and((&ps).opt()).and((&bc).opt())).drive(|_, ((u, p), b)| v.push((u, pz(p), bz(b))));
    out(v, |&(u, p, _)| (rep_desc(db, u), Reverse(p[0])), 10, |&(u, p, b)| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(ints(&[p[0], p[2], p[1]]));
        f.push(if p[0] == 0 { V::Null } else { V::F(round2(p[3] as f64 / p[0] as f64)) });
        f.extend(ints(&b));
        f
    })
}

// WITH RecentPosts AS (
// SELECT p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.OwnerUserId,
// u.DisplayName AS OwnerDisplayName, COUNT(c.Id) AS CommentCount,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes
// FROM Posts p
// JOIN Users u ON p.OwnerUserId = u.Id
// LEFT JOIN Comments c ON p.Id = c.PostId
// LEFT JOIN Votes v ON p.Id = v.PostId
// WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days'
// GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.OwnerUserId, u.DisplayName
// ),
// TopUsers AS (
// SELECT u.Id AS UserId, u.DisplayName, SUM(b.Class) AS TotalBadges, COUNT(p.Id) AS PostCount
// FROM Users u
// LEFT JOIN Badges b ON u.Id = b.UserId
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// GROUP BY u.Id, u.DisplayName
// ORDER BY TotalBadges DESC, PostCount DESC
// LIMIT 10
// ),
// PostVoteSummary AS (
// SELECT p.Id AS PostId,
// COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes,
// COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes
// FROM Posts p
// LEFT JOIN Votes v ON p.Id = v.PostId
// GROUP BY p.Id
// )
// SELECT rp.Title, rp.CreationDate, rp.ViewCount, rp.Score, rp.CommentCount,
// rp.UpVotes, rp.DownVotes, tu.DisplayName AS TopUserDisplayName, tu.TotalBadges, tu.PostCount
// FROM RecentPosts rp
// JOIN TopUsers tu ON rp.OwnerUserId = tu.UserId
// JOIN PostVoteSummary pvs ON rp.Id = pvs.PostId
// ORDER BY rp.CreationDate DESC, rp.Score DESC;
fn q7747(db: &'static So) -> String {
    let tu = g(db).select(badges_of(db).select(&db.badge.class).opt().and(posts_of(db).opt())).fold((None, 0i64), |(c, n): (Option<i64>, i64), (b, p)| {
        (match b {
            Some(b) => Some(c.unwrap_or(0) + b),
            None => c,
        }, n + p.is_some() as i64)
    });
    let top: MatSet<Id<User>> = whole(&db.user.id)
        .select(Ident::<User>::new().and(&tu))
        .window(row_number, |(_, (c, n)): (Id<User>, (Option<i64>, i64))| (c.is_some(), c, n), desc)
        .filt(|(_, r)| r <= 10)
        .map(|((u, _), _)| u)
        .collect();
    let mut w = Vec::new();
    stats_fold(db, owned_since(db, month_ago()).with((&db.post.owner_user).select(&top)), Ident::<Post>::new(), "cv", &[])
        .and((&db.post.owner_user).select(Ident::<User>::new().and(&tu)))
        .drive(|p, (s, (u, t))| w.push((p, s, u, t)));
    rows(w.iter().map(|&(p, s, u, (c, n))| {
        let mut f = post_fields(db, p, &["title", "created", "views", "score"]);
        f.extend(ints(&[s.cx, s.up, s.down]));
        f.extend([user_col(db, u, "name"), oint(c), V::I(n)]);
        row(f)
    }))
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(DISTINCT P.Id) AS TotalPosts,
// SUM(CASE WHEN PT.Name = 'Answer' THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN PT.Name = 'Question' THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesReceived,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesReceived
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// PostTypes PT ON P.PostTypeId = PT.Id
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// U.Id, U.DisplayName, U.Reputation
// ),
// BadgeStats AS (
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
// S.UserId,
// S.DisplayName,
// S.Reputation,
// S.TotalPosts,
// S.TotalQuestions,
// S.TotalAnswers,
// S.UpVotesReceived,
// S.DownVotesReceived,
// COALESCE(B.TotalBadges, 0) AS TotalBadges,
// COALESCE(B.GoldBadges, 0) AS GoldBadges,
// COALESCE(B.SilverBadges, 0) AS SilverBadges,
// COALESCE(B.BronzeBadges, 0) AS BronzeBadges
// FROM
// UserStats S
// LEFT JOIN
// BadgeStats B ON S.UserId = B.UserId
// WHERE
// S.TotalPosts > 0
// ORDER BY
// S.Reputation DESC, S.TotalPosts DESC
// FETCH FIRST 50 ROWS ONLY;
fn q9744(db: &'static So) -> String {
    let nq = named_qa(db);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&dp).and(&nq).and((&bc).opt()).drive(|u, ((d, a), b)| v.push((u, d, a, bz(b))));
    out(v, |&(u, d, ..)| (rep_desc(db, u), Reverse(d)), 50, |&(u, d, a, b)| {
        let mut f = ["uid", "name", "rep"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.extend(ints(&[d, a[0], a[1], a[2], a[3]]));
        f.extend(ints(&b));
        f
    })
}

// WITH UserBadgeCounts AS (
// SELECT
// u.Id AS UserId,
// COUNT(b.Id) AS BadgeCount,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldCount,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverCount,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeCount
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// u.Id
// ),
// ActiveUserPosts AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS PostCount,
// SUM(p.Score) AS TotalScore,
// AVG(p.ViewCount) AS AvgViewCount
// FROM
// Posts p
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 YEAR'
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
// up.PostCount,
// up.TotalScore,
// up.AvgViewCount
// FROM
// Users u
// LEFT JOIN
// UserBadgeCounts ub ON u.Id = ub.UserId
// LEFT JOIN
// ActiveUserPosts up ON u.Id = up.OwnerUserId
// )
// SELECT
// ua.UserId,
// ua.DisplayName,
// COALESCE(ua.BadgeCount, 0) AS BadgeCount,
// COALESCE(ua.GoldCount, 0) AS GoldCount,
// COALESCE(ua.SilverCount, 0) AS SilverCount,
// COALESCE(ua.BronzeCount, 0) AS BronzeCount,
// COALESCE(ua.PostCount, 0) AS PostCount,
// COALESCE(ua.TotalScore, 0) AS TotalScore,
// COALESCE(ua.AvgViewCount, 0) AS AvgViewCount
// FROM
// UserActivity ua
// ORDER BY
// ua.TotalScore DESC,
// ua.BadgeCount DESC
// LIMIT 100;
fn q9526(db: &'static So) -> String {
    let ub = ubc(db);
    let Post { score, view_count, .. } = &db.post;
    let ps = owned_since(db, year_ago()).group_by(&db.post.owner_user).select(score.and(view_count.opt())).fold([0i64; 4], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]);
    let mut v = Vec::new();
    (&ub).and((&ps).opt()).drive(|u, (b, p)| v.push((u, b, p)));
    out(v, |&(_, b, p)| ((p.is_none(), Reverse(p.map_or(0, |p| p[1]))), Reverse(b[0])), 100, |&(u, b, p)| {
        let p = p.unwrap_or([0; 4]);
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&b));
        f.extend([V::I(p[0]), V::I(p[1]), or0(p[3], p[2])]);
        f
    })
}

// WITH UserMetrics AS (
// SELECT
// U.Id AS UserId,
// U.Reputation,
// COUNT(DISTINCT P.Id) AS PostCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// U.Id, U.Reputation
// ), PostStatistics AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.Score,
// P.ViewCount,
// P.CreationDate,
// COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
// SUM(CASE WHEN PH.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS ClosedCount,
// SUM(CASE WHEN PH.PostHistoryTypeId = 11 THEN 1 ELSE 0 END) AS ReopenedCount
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// PostHistory PH ON P.Id = PH.PostId
// GROUP BY
// P.Id, P.Title, P.Score, P.ViewCount, P.CreationDate
// )
// SELECT
// UM.UserId,
// UM.Reputation,
// UM.PostCount,
// UM.QuestionCount,
// UM.AnswerCount,
// UM.UpVoteCount,
// UM.DownVoteCount,
// PS.PostId,
// PS.Title,
// PS.Score,
// PS.ViewCount,
// PS.CreationDate,
// PS.CommentCount,
// PS.ClosedCount,
// PS.ReopenedCount
// FROM
// UserMetrics UM
// JOIN
// PostStatistics PS ON UM.UserId = PS.PostId
// ORDER BY
// UM.Reputation DESC, PS.Score DESC;
fn q12737(db: &'static So) -> String {
    let pid = pids(db);
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let sf = stats_fold(db, db.post.iq(), Ident::<Post>::new(), "ch", &[]);
    let mut v = Vec::new();
    (&us).and((&dp).opt()).and((&db.user.origid).select(&pid).select(Ident::<Post>::new().and(&sf))).drive(|u, ((a, d), (p, s))| v.push((u, a, d.unwrap_or(0), p, s)));
    rows(v.iter().map(|&(u, a, d, p, s)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep")];
        f.extend(ints(&[d, a.q, a.a, a.up, a.down]));
        f.extend(post_fields(db, p, &["id", "title", "score", "views", "created"]));
        f.extend(ints(&[s.cx, s.h10, s.h11]));
        row(f)
    }))
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(DISTINCT P.Id) AS PostCount,
// SUM(CASE WHEN PT.Name = 'Answer' THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN PT.Name = 'Question' THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(U.UpVotes) AS TotalUpVotes,
// SUM(U.DownVotes) AS TotalDownVotes
// FROM Users U
// LEFT JOIN Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN PostTypes PT ON P.PostTypeId = PT.Id
// GROUP BY U.Id, U.DisplayName, U.Reputation
// ),
// BadgeStats AS (
// SELECT
// B.UserId,
// COUNT(B.Id) AS BadgeCount,
// MAX(B.Class) AS HighestBadgeClass
// FROM Badges B
// GROUP BY B.UserId
// ),
// PostStats AS (
// SELECT
// P.OwnerUserId,
// COUNT(C.Id) AS CommentCount,
// MAX(P.Score) AS MaxPostScore,
// AVG(P.ViewCount) AS AvgPostViews
// FROM Posts P
// LEFT JOIN Comments C ON P.Id = C.PostId
// GROUP BY P.OwnerUserId
// )
// SELECT
// U.DisplayName,
// U.Reputation,
// US.PostCount,
// US.QuestionCount,
// US.AnswerCount,
// US.TotalUpVotes,
// US.TotalDownVotes,
// COALESCE(BS.BadgeCount, 0) AS BadgeCount,
// COALESCE(BS.HighestBadgeClass, 0) AS HighestBadgeClass,
// COALESCE(PS.CommentCount, 0) AS CommentCount,
// COALESCE(PS.MaxPostScore, 0) AS MaxPostScore,
// COALESCE(PS.AvgPostViews, 0) AS AvgPostViews
// FROM UserStats US
// JOIN Users U ON U.Id = US.UserId
// LEFT JOIN BadgeStats BS ON U.Id = BS.UserId
// LEFT JOIN PostStats PS ON U.Id = PS.OwnerUserId
// WHERE U.Reputation > 1000
// ORDER BY U.Reputation DESC, US.PostCount DESC
// LIMIT 50;
fn q7609(db: &'static So) -> String {
    let nq = db
        .user
        .group_by(Ident::<User>::new())
        .select((&db.user.up_votes).and(&db.user.down_votes).and(posts_of(db).select(name(db)).opt()))
        .fold([0i64; 5], |a, ((u, d), n)| [a[0] + n.is_some() as i64, a[1] + (n == Some("Question")) as i64, a[2] + (n == Some("Answer")) as i64, a[3] + u, a[4] + d]);
    let bs = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold((0i64, i64::MIN), |(n, m), c| (n + 1, m.max(c)));
    let Post { score, view_count, .. } = &db.post;
    let ps = owned(db).group_by(&db.post.owner_user).select(score.and(view_count.opt()).and(comments_of(db).opt())).fold([0, i64::MIN, 0, 0], |a: [i64; 4], ((s, w), c)| {
        [a[0] + c.is_some() as i64, a[1].max(s), a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]
    });
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and(&nq).and((&bs).opt()).and((&ps).opt())).drive(|_, (((u, a), b), p)| v.push((u, a, b, p)));
    out(v, |&(u, a, ..)| (rep_desc(db, u), Reverse(a[0])), 50, |&(u, a, b, p)| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(ints(&[a[0], a[1], a[2], a[3], a[4], b.map_or(0, |b| b.0), b.map_or(0, |b| b.1)]));
        f.extend(match p {
            Some(p) => vec![V::I(p[0]), V::I(p[1]), or0(p[3], p[2])],
            None => vec![V::I(0), V::I(0), V::F(0.0)],
        });
        f
    })
}

// WITH UserActivity AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts,
// MAX(p.CreationDate) AS LastPostDate
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// GROUP BY u.Id, u.DisplayName
// ),
// RecentPosts AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// u.DisplayName AS Author,
// pt.Name AS PostType,
// COUNT(c.Id) AS CommentCount
// FROM Posts p
// JOIN Users u ON p.OwnerUserId = u.Id
// JOIN PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN Comments c ON p.Id = c.PostId
// WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'
// GROUP BY p.Id, p.Title, p.CreationDate, p.Score, u.DisplayName, pt.Name
// ORDER BY p.CreationDate DESC
// LIMIT 10
// ),
// TopUsers AS (
// SELECT
// ua.UserId,
// ua.DisplayName,
// ua.PostCount,
// ua.PositivePosts,
// (SELECT COUNT(DISTINCT b.Id) FROM Badges b WHERE b.UserId = ua.UserId) AS BadgeCount
// FROM UserActivity ua
// WHERE ua.PostCount > 0
// ORDER BY ua.PositivePosts DESC, ua.PostCount DESC
// LIMIT 5
// )
// SELECT
// tu.DisplayName AS TopUser,
// tu.PostCount,
// tu.PositivePosts,
// tu.BadgeCount,
// rp.Title AS RecentPostTitle,
// rp.Author AS RecentPostAuthor,
// rp.CreationDate AS RecentPostDate,
// rp.CommentCount AS RecentPostComments
// FROM TopUsers tu
// JOIN RecentPosts rp ON tu.DisplayName = rp.Author
// ORDER BY tu.PositivePosts DESC, rp.CreationDate DESC;
fn q6780(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let pa = owned(db).group_by(&db.post.owner_user).select(&db.post.score).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + (s > 0) as i64]);
    let tu: MatSet<Id<User>> = whole(&pa)
        .select(Ident::<User>::new().and(&pa))
        .window(row_number, |(_, a): (Id<User>, [i64; 2])| (a[1], a[0]), desc)
        .filt(|(_, n)| n <= 5)
        .map(|((u, _), _)| u)
        .collect();
    let base = owned_since(db, month_ago());
    let rp: MatSet<Id<Post>> = whole(&base)
        .select(Ident::<Post>::new().and(&db.post.creation_date))
        .window(row_number, |(_, c): (Id<Post>, i64)| c, desc)
        .filt(|(_, n)| n <= 10)
        .map(|((p, _), _)| p)
        .collect();
    let names: HashIdx<Str, Id<Post>> = (&rp).select((&db.post.owner_user).select(&db.user.display_name)).inv().collect();
    let cp = comments_per_post(db);
    let mut v = Vec::new();
    (&tu).select(Ident::<User>::new().and(&pa).and(&bu).and((&db.user.display_name).select(&names).select(Ident::<Post>::new().and(&cp)))).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(((u, a), b), (p, c))| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(b)];
        f.extend(post_fields(db, p, &["title", "owner", "created"]));
        f.push(V::I(c));
        row(f)
    }))
}

// WITH UserVoteStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS Upvotes,
// COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS Downvotes,
// SUM(CASE WHEN v.VoteTypeId IN (2, 3) THEN 1 ELSE 0 END) AS TotalVotes
// FROM Users u
// LEFT JOIN Votes v ON u.Id = v.UserId
// GROUP BY u.Id, u.DisplayName
// ),
// PostActivity AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS CloseCount,
// SUM(CASE WHEN ph.PostHistoryTypeId = 11 THEN 1 ELSE 0 END) AS ReopenCount,
// MAX(p.LastActivityDate) AS LastActivity
// FROM Posts p
// LEFT JOIN Comments c ON p.Id = c.PostId
// LEFT JOIN PostHistory ph ON p.Id = ph.PostId
// WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY p.Id, p.Title
// ),
// PostStats AS (
// SELECT
// pa.PostId,
// pa.Title,
// pa.CommentCount,
// pa.CloseCount,
// pa.ReopenCount,
// pa.LastActivity,
// COALESCE(uvs.Upvotes, 0) AS TotalUpvotes,
// COALESCE(uvs.Downvotes, 0) AS TotalDownvotes,
// (COALESCE(uvs.Upvotes, 0) - COALESCE(uvs.Downvotes, 0)) AS NetVotes
// FROM PostActivity pa
// LEFT JOIN UserVoteStats uvs ON pa.PostId = uvs.UserId
// )
// SELECT
// ps.Title,
// ps.CommentCount,
// ps.CloseCount,
// ps.ReopenCount,
// ps.LastActivity,
// ps.TotalUpvotes,
// ps.TotalDownvotes,
// ps.NetVotes
// FROM PostStats ps
// ORDER BY ps.NetVotes DESC, ps.LastActivity DESC
// LIMIT 10;
fn q7130(db: &'static So) -> String {
    let uid = uids(db);
    let uv = uvotes(db);
    let mut v = Vec::new();
    stats_fold(db, since(db, year_ago()), Ident::<Post>::new(), "ch", &[])
        .and((&db.post.origid).select(&uid).select(&uv).opt())
        .drive(|p, (s, x)| v.push((p, s, x.unwrap_or([0; 3]))));
    out(v, |&(p, _, x)| (Reverse(x[1] - x[2]), Reverse(db.post.last_activity_date.get(p).unwrap())), 10, |&(p, s, x)| {
        let mut f = post_fields(db, p, &["title"]);
        f.extend(ints(&[s.cx, s.h10, s.h11]));
        f.extend(post_fields(db, p, &["activity"]));
        f.extend(ints(&[x[1], x[2], x[1] - x[2]]));
        f
    })
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(DISTINCT B.Id) AS BadgeCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// COUNT(DISTINCT P.Id) AS PostCount,
// COUNT(DISTINCT C.Id) AS CommentCount,
// MAX(P.CreationDate) AS LastPostDate
// FROM Users U
// LEFT JOIN Badges B ON U.Id = B.UserId
// LEFT JOIN Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN Comments C ON P.Id = C.PostId
// LEFT JOIN Votes V ON P.Id = V.PostId
// WHERE U.Reputation > 1000
// GROUP BY U.Id, U.DisplayName, U.Reputation
// ), PopularTags AS (
// SELECT
// T.TagName,
// COUNT(DISTINCT P.Id) AS PostCount
// FROM Tags T
// JOIN Posts P ON P.Tags LIKE CONCAT('%', T.TagName, '%')
// WHERE P.CreationDate > DATE '2024-10-01' - INTERVAL '1 year'
// GROUP BY T.TagName
// HAVING COUNT(DISTINCT P.Id) > 50
// ), ActiveUsers AS (
// SELECT
// U.DisplayName,
// U.Id,
// US.Reputation,
// US.BadgeCount,
// US.UpVotes,
// US.DownVotes,
// US.PostCount,
// US.CommentCount,
// US.LastPostDate
// FROM UserStats US
// JOIN Users U ON US.UserId = U.Id
// WHERE US.PostCount > 5 AND US.CommentCount > 10
// )
// SELECT
// AU.DisplayName,
// AU.Reputation,
// AU.BadgeCount,
// AU.UpVotes,
// AU.DownVotes,
// PT.TagName,
// PT.PostCount
// FROM ActiveUsers AU
// JOIN PopularTags PT ON AU.PostCount > PT.PostCount
// ORDER BY AU.Reputation DESC, PT.PostCount DESC
// FETCH FIRST 10 ROWS ONLY;
fn q9151(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::RepGt(1000), "bcv", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let bu = badges_per_user(db);
    let dc = udc(db);
    let au = user_base(db, UserWhere::RepGt(1000))
        .with((&dp).filt(|d: i64| d > 5))
        .with((&dc).filt(|c: i64| c > 10))
        .group_by(Ident::<User>::new())
        .select(&dp)
        .fold(0i64, |_, d| d);
    let tm = tag_mentions(db);
    let d0 = date(2023, 10, 1);
    let pt = (&tm)
        .with(Same::<(Id<Post>, Id<Tag>)>::new().map(|(p, _)| p).select(&db.post.creation_date).filt(move |c: i64| c > d0))
        .group_by(Same::<(Id<Post>, Id<Tag>)>::new().map(|(_, t)| t))
        .select(Same::<(Id<Post>, Id<Tag>)>::new().map(|(p, _)| p))
        .count_distinct();
    let tags = (&au).select_where((&pt).filt(|n: i64| n > 50).inv(), |d: i64, n: i64| d > n).select(Ident::<Tag>::new().and(&pt));
    let mut v = Vec::new();
    (&au).and(&us).and(&bu).and(tags).drive(|u, (((_, a), b), tn)| v.push((u, a, b, tn)));
    out(v, |&(u, _, _, (_, n))| (rep_desc(db, u), Reverse(n)), 10, |&(u, a, b, (t, n))| {
        vec![user_col(db, u, "name"), user_col(db, u, "rep"), V::I(b), V::I(a.up), V::I(a.down), V::S(db.tag.tag_name.get(t).unwrap()), V::I(n)]
    })
}

// WITH RECURSIVE UserVotes AS (
// SELECT UserId, COUNT(*) AS TotalVotes
// FROM Votes
// GROUP BY UserId
// ),
// TopUsers AS (
// SELECT U.Id, U.DisplayName, U.Reputation, UV.TotalVotes
// FROM Users U
// JOIN UserVotes UV ON U.Id = UV.UserId
// WHERE U.Reputation > 1000
// ORDER BY UV.TotalVotes DESC
// LIMIT 10
// ),
// PostDetails AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.Score,
// P.ViewCount,
// COALESCE(PH.PostHistoryCount, 0) AS PostHistoryCount,
// CASE
// WHEN P.AcceptedAnswerId IS NOT NULL THEN 'Yes'
// ELSE 'No'
// END AS HasAcceptedAnswer
// FROM Posts P
// LEFT JOIN (
// SELECT PostId, COUNT(*) AS PostHistoryCount
// FROM PostHistory
// GROUP BY PostId
// ) PH ON P.Id = PH.PostId
// ),
// UserPostInteractions AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// D.PostId,
// D.Title,
// D.CreationDate,
// D.Score,
// CASE WHEN V.VoteTypeId = 2 THEN 'Upvote' ELSE 'Downvote' END AS VoteType,
// D.HasAcceptedAnswer
// FROM Users U
// JOIN Posts P ON U.Id = P.OwnerUserId
// JOIN PostDetails D ON P.Id = D.PostId
// LEFT JOIN Votes V ON P.Id = V.PostId AND V.UserId = U.Id
// )
// SELECT
// U.DisplayName AS VotingUser,
// D.Title AS PostTitle,
// D.HasAcceptedAnswer,
// COUNT(*) AS VoteCount,
// AVG(D.Score) AS AvgScore,
// MAX(D.CreationDate) AS LatestPostDate
// FROM UserPostInteractions D
// JOIN TopUsers U ON D.UserId = U.Id
// GROUP BY U.DisplayName, D.Title, D.HasAcceptedAnswer
// HAVING COUNT(*) > 2
// ORDER BY AvgScore DESC, VotingUser;
fn q33531(db: &'static So) -> String {
    let vc = db.vote.group_by(&db.vote.user).select(&db.vote.vote_type_id).fold(0i64, |a, _| a + 1);
    let base = user_base(db, UserWhere::RepGt(1000)).with(&vc);
    let top: MatSet<Id<User>> = whole(&base)
        .select(Ident::<User>::new().and(&vc))
        .window(row_number, |(_, n): (Id<User>, i64)| n, desc)
        .filt(|(_, n)| n <= 10)
        .map(|((u, _), _)| u)
        .collect();
    let sv = self_votes(db);
    let Post { title, accepted_answer_id, score, creation_date, .. } = &db.post;
    let g = owned(db)
        .with((&db.post.owner_user).select(&top))
        .group_by((&db.post.owner_user).and(title.opt()).and(accepted_answer_id.opt().map(|a: Option<i64>| a.is_some())))
        .select(score.and(creation_date).and((&sv).opt()))
        .fold([0, 0, i64::MIN], |a: [i64; 3], ((s, c), _)| [a[0] + 1, a[1] + s, a[2].max(c)]);
    let mut v = Vec::new();
    (&g).filt(|a: [i64; 3]| a[0] > 2).drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&(((u, t), h), a)| row(vec![user_col(db, u, "name"), ostr(t), V::S(if h { "Yes" } else { "No" }), V::I(a[0]), avg(a[1], a[0]), V::T(a[2])])))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("9851", q9851),
    ("7894", q7894),
    ("14638", q14638),
    ("13194", q13194),
    ("14648", q14648),
    ("8220", q8220),
    ("10006", q10006),
    ("10025", q10025),
    ("10564", q10564),
    ("13030", q13030),
    ("14719", q14719),
    ("7928", q7928),
    ("10728", q10728),
    ("14018", q14018),
    ("10762", q10762),
    ("8088", q8088),
    ("14017", q14017),
    ("5998", q5998),
    ("13168", q13168),
    ("13361", q13361),
    ("12712", q12712),
    ("8764", q8764),
    ("8050", q8050),
    ("9584", q9584),
    ("6704", q6704),
    ("5267", q5267),
    ("12167", q12167),
    ("9074", q9074),
    ("12518", q12518),
    ("1921", q1921),
    ("7998", q7998),
    ("10043", q10043),
    ("6364", q6364),
    ("9958", q9958),
    ("14009", q14009),
    ("4760", q4760),
    ("7725", q7725),
    ("6543", q6543),
    ("9402", q9402),
    ("14038", q14038),
    ("5985", q5985),
    ("9632", q9632),
    ("13250", q13250),
    ("14827", q14827),
    ("9491", q9491),
    ("12311", q12311),
    ("6563", q6563),
    ("5178", q5178),
    ("11632", q11632),
    ("14531", q14531),
    ("10360", q10360),
    ("13948", q13948),
    ("4531", q4531),
    ("14483", q14483),
    ("14834", q14834),
    ("1703", q1703),
    ("7747", q7747),
    ("9744", q9744),
    ("9526", q9526),
    ("12737", q12737),
    ("7609", q7609),
    ("6780", q6780),
    ("7130", q7130),
    ("9151", q9151),
    ("33531", q33531),
];
