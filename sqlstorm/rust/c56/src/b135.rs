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

/// Per user over `Users LEFT JOIN Posts LEFT JOIN Badges` (the product): [rows with a badge, SUM(b.Class)].
fn user_class_rows(db: &'static So) -> Fold<Id<User>, [i64; 2]> {
    db.user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 2], |a, (_, c)| [a[0] + c.is_some() as i64, a[1] + c.unwrap_or(0)])
}

/// Per user over `Users LEFT JOIN Posts LEFT JOIN Badges` (the product): [rows,
/// question rows, answer rows, rows with a badge].
fn user_pb(db: &'static So) -> Fold<Id<User>, [i64; 4]> {
    db.user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(&db.post.post_type_id).opt().and(badges_of(db).opt()))
        .fold([0i64; 4], |a, (t, b)| [a[0] + 1, a[1] + (t == Some(1)) as i64, a[2] + (t == Some(2)) as i64, a[3] + b.is_some() as i64])
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

fn badges_by_uid(db: &'static So) -> HashIdx<i64, Id<Badge>> {
    (&db.badge.user_id).inv().collect()
}

/// Per post over `Posts LEFT JOIN Comments LEFT JOIN Votes LEFT JOIN Badges ON
/// p.OwnerUserId = b.UserId` (the product, badges on the raw id): [comment rows,
/// vote rows, up rows, down rows, gold rows, silver rows, bronze rows].
fn cvb_raw<Q: Drive<D = Id<Post>, R = Id<Post>>>(db: &'static So, base: Q, bu: &HashIdx<i64, Id<Badge>>) -> Fold<Id<Post>, [i64; 7]> {
    base.group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and((&db.post.owner_user_id).select(bu.select(&db.badge.class)).opt()))
        .fold([0i64; 7], |a, ((c, t), b)| {
            [a[0] + c.is_some() as i64, a[1] + t.is_some() as i64, a[2] + (t == Some(2)) as i64, a[3] + (t == Some(3)) as i64, a[4] + (b == Some(1)) as i64, a[5] + (b == Some(2)) as i64, a[6] + (b == Some(3)) as i64]
        })
}

fn vid_sum(db: &'static So) -> Fold<Id<Post>, (i64, i64)> {
    db.vote.group_by(&db.vote.post).select(&db.vote.origid).fold((0i64, 0i64), |(n, s), i| (n + 1, s + i))
}

// --- batch 135 --------------------------------------------------------------

// WITH UserBadges AS (
// SELECT
// u.Id AS UserId,
// COUNT(b.Id) AS BadgeCount
// FROM Users u
// LEFT JOIN Badges b ON u.Id = b.UserId
// GROUP BY u.Id
// ),
// PostStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS PostCount,
// SUM(p.Score) AS TotalScore,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(p.ViewCount) AS TotalViews
// FROM Posts p
// WHERE p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY p.OwnerUserId
// ),
// UserPostBadgeStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COALESCE(ps.PostCount, 0) AS PostCount,
// COALESCE(ps.TotalScore, 0) AS TotalScore,
// COALESCE(ps.QuestionCount, 0) AS QuestionCount,
// COALESCE(ps.AnswerCount, 0) AS AnswerCount,
// COALESCE(ps.TotalViews, 0) AS TotalViews,
// COALESCE(ub.BadgeCount, 0) AS BadgeCount
// FROM Users u
// LEFT JOIN PostStats ps ON u.Id = ps.OwnerUserId
// LEFT JOIN UserBadges ub ON u.Id = ub.UserId
// )
// SELECT
// DisplayName,
// PostCount,
// TotalScore,
// QuestionCount,
// AnswerCount,
// TotalViews,
// BadgeCount
// FROM UserPostBadgeStats
// WHERE PostCount > 10
// ORDER BY TotalScore DESC, BadgeCount DESC, PostCount DESC;
fn q6139(db: &'static So) -> String {
    let ps = pstat(db, db.post.with((&db.post.creation_date).gt(year_ago())));
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    (&ps).filt(|p: [i64; 13]| p[0] > 10).and(&bu).drive(|u, (p, b)| v.push((u, p, b)));
    rows(v.iter().map(|&(u, p, b)| row(vec![user_col(db, u, "name"), V::I(p[0]), V::I(p[3]), V::I(p[1]), V::I(p[2]), V::I(p[5]), V::I(b)])))
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS PostCount,
// COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS QuestionCount,
// COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS AnswerCount,
// SUM(COALESCE(p.Score, 0)) AS TotalScore,
// SUM(COALESCE(p.ViewCount, 0)) AS TotalViews,
// SUM(COALESCE(c.Id, 0)) AS CommentCount
// FROM
// Users AS u
// LEFT JOIN
// Posts AS p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Comments AS c ON p.Id = c.PostId
// GROUP BY
// u.Id, u.DisplayName
// ),
// UserBadges AS (
// SELECT
// ba.UserId,
// COUNT(ba.Id) AS BadgeCount
// FROM
// Badges AS ba
// GROUP BY
// ba.UserId
// ),
// TotalStats AS (
// SELECT
// ups.UserId,
// ups.DisplayName,
// ups.PostCount,
// ups.QuestionCount,
// ups.AnswerCount,
// ups.TotalScore,
// ups.TotalViews,
// ups.CommentCount,
// COALESCE(ub.BadgeCount, 0) AS BadgeCount
// FROM
// UserPostStats AS ups
// LEFT JOIN
// UserBadges AS ub ON ups.UserId = ub.UserId
// )
// SELECT
// UserId,
// DisplayName,
// PostCount,
// QuestionCount,
// AnswerCount,
// TotalScore,
// TotalViews,
// CommentCount,
// BadgeCount
// FROM
// TotalStats
// ORDER BY
// TotalScore DESC
// LIMIT 100;
fn q12540(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let uf = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(score.and(view_count.opt()).and(comments_of(db).select(&db.comment.origid).opt())).opt())
        .fold([0i64; 3], |a, p| match p {
            Some(((s, w), i)) => [a[0] + s, a[1] + w.unwrap_or(0), a[2] + i.unwrap_or(0)],
            None => a,
        });
    let pc = owned(db).group_by(&db.post.owner_user).select(post_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64]);
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    (&uf).and((&pc).opt()).and(&bu).drive(|u, ((x, p), b)| {
        let p = p.unwrap_or([0; 3]);
        v.push((u, [p[0], p[1], p[2], x[0], x[1], x[2]], b))
    });
    out(v, |&(_, p, _)| Reverse(p[3]), 100, |&(u, p, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&p));
        f.push(V::I(b));
        f
    })
}

// WITH PostStats AS (
// SELECT
// p.PostTypeId,
// COUNT(*) AS TotalPosts,
// SUM(CASE WHEN p.ViewCount IS NOT NULL THEN p.ViewCount ELSE 0 END) AS TotalViews,
// SUM(CASE WHEN p.Score IS NOT NULL THEN p.Score ELSE 0 END) AS TotalScore,
// AVG(CASE WHEN p.ViewCount IS NOT NULL THEN p.ViewCount ELSE 0 END) AS AverageViews,
// AVG(CASE WHEN p.Score IS NOT NULL THEN p.Score ELSE 0 END) AS AverageScore
// FROM
// Posts p
// GROUP BY
// p.PostTypeId
// ),
// UserStats AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// COUNT(b.Id) AS TotalBadges,
// SUM(v.BountyAmount) AS TotalBounty
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// LEFT JOIN
// Votes v ON u.Id = v.UserId
// GROUP BY
// u.Id, u.Reputation
// )
// SELECT
// p.PostTypeId,
// ps.TotalPosts,
// ps.TotalViews,
// ps.TotalScore,
// ps.AverageViews,
// ps.AverageScore,
// COUNT(DISTINCT us.UserId) AS TotalUsers,
// SUM(us.Reputation) AS TotalReputation,
// SUM(us.TotalBadges) AS TotalBadges,
// SUM(us.TotalBounty) AS TotalBounty
// FROM
// PostStats ps
// JOIN
// Posts p ON p.PostTypeId = ps.PostTypeId
// JOIN
// UserStats us ON p.OwnerUserId = us.UserId
// GROUP BY
// p.PostTypeId, ps.TotalPosts, ps.TotalViews, ps.TotalScore, ps.AverageViews, ps.AverageScore
// ORDER BY
// p.PostTypeId;
fn q13252(db: &'static So) -> String {
    let Post { view_count, score, .. } = &db.post;
    let ps = db.post.group_by(&db.post.post_type_id).select(view_count.opt().and(score)).fold([0i64; 3], |a, (w, s)| [a[0] + 1, a[1] + w.unwrap_or(0), a[2] + s]);
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold([0i64; 3], |a, (b, x)| {
            let x = x.flatten();
            [a[0] + b.is_some() as i64, a[1] + x.is_some() as i64, a[2] + x.unwrap_or(0)]
        });
    let tf = owned(db)
        .group_by(&db.post.post_type_id)
        .select((&db.post.owner_user).select((&db.user.reputation).and(&us)))
        .fold([0i64; 4], |a, (r, x)| [a[0] + r, a[1] + x[0], a[2] + (x[1] > 0) as i64, a[3] + x[2]]);
    let du = owned(db).group_by(&db.post.post_type_id).select(&db.post.owner_user).count_distinct();
    let mut v = Vec::new();
    (&ps).and(&tf).and(&du).drive(|t, ((p, f), d)| v.push((t, p, f, d)));
    rows(v.iter().map(|&(t, p, f, d)| {
        row(vec![V::I(t), V::I(p[0]), V::I(p[1]), V::I(p[2]), avg(p[1], p[0]), avg(p[2], p[0]), V::I(d), V::I(f[0]), V::I(f[1]), nullable(f[3], f[2])])
    }))
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
// SUM(P.Score) AS TotalScore,
// SUM(P.ViewCount) AS TotalViews
// FROM
// Posts P
// WHERE
// P.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'
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
// COALESCE(PS.TotalViews, 0) AS TotalViews
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
// Reputation,
// BadgeCount,
// PostCount,
// TotalScore,
// TotalViews,
// CASE
// WHEN BadgeCount > 5 THEN 'Expert'
// WHEN BadgeCount BETWEEN 3 AND 5 THEN 'Proficient'
// ELSE 'Novice'
// END AS UserLevel
// FROM
// UserEngagement
// WHERE
// Reputation > 1000
// ORDER BY
// TotalScore DESC, PostCount DESC;
fn q8324(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let ps = pstat(db, since(db, date(2023, 10, 1)));
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and(&bu).and((&ps).opt())).drive(|_, ((u, b), p)| v.push((u, b, pz(p))));
    rows(v.iter().map(|&(u, b, p)| {
        let lvl = if b > 5 { "Expert" } else if (3..=5).contains(&b) { "Proficient" } else { "Novice" };
        row(vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep"), V::I(b), V::I(p[0]), V::I(p[3]), V::I(p[5]), V::S(lvl)])
    }))
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.PostTypeId,
// p.CreationDate,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
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
// p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY
// p.Id, p.PostTypeId, p.CreationDate
// ),
// AverageStats AS (
// SELECT
// PostTypeId,
// AVG(CommentCount) AS AvgCommentCount,
// AVG(VoteCount) AS AvgVoteCount,
// AVG(UpVotes) AS AvgUpVotes,
// AVG(DownVotes) AS AvgDownVotes,
// AVG(BadgeCount) AS AvgBadgeCount
// FROM
// PostStats
// GROUP BY
// PostTypeId
// )
// SELECT
// pt.Name AS PostType,
// AVG(AvgCommentCount) AS AvgCommentCount,
// AVG(AvgVoteCount) AS AvgVoteCount,
// AVG(AvgUpVotes) AS AvgUpVotes,
// AVG(AvgDownVotes) AS AvgDownVotes,
// AVG(AvgBadgeCount) AS AvgBadgeCount
// FROM
// AverageStats as avg
// JOIN
// PostTypes pt ON avg.PostTypeId = pt.Id
// GROUP BY
// pt.Name
// ORDER BY
// pt.Name;
fn q10285(db: &'static So) -> String {
    let base = || db.post.with((&db.post.creation_date).gt(year_ago()));
    let bu = badges_by_uid(db);
    let sf = cvb_raw(db, base(), &bu);
    let nb = base().group_by(Ident::<Post>::new()).select((&db.post.owner_user_id).select(&bu)).fold(0i64, |n, _| n + 1);
    let tf = base().group_by(&db.post.post_type).select((&sf).and((&nb).opt())).fold([0i64; 6], |a, (s, b)| {
        [a[0] + 1, a[1] + s[0], a[2] + s[1], a[3] + s[2], a[4] + s[3], a[5] + b.unwrap_or(0)]
    });
    let tn = db.post_type.with(&tf).group_by(&db.post_type.name).select(&tf).fold(([0f64; 5], 0i64), |(m, n), a: [i64; 6]| {
        let mut m = m;
        for i in 0..5 {
            m[i] += a[i + 1] as f64 / a[0] as f64;
        }
        (m, n + 1)
    });
    let mut v = Vec::new();
    (&tn).drive(|t, a| v.push((t, a)));
    rows(v.iter().map(|&(t, (m, n))| {
        let mut f = vec![V::S(t)];
        f.extend(m.iter().map(|&x| V::F(x / n as f64)));
        row(f)
    }))
}

// WITH PostStats AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.Score,
// COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
// COUNT(DISTINCT PH.Id) AS EditCount,
// SUM(CASE WHEN V.CreationDate IS NOT NULL THEN 1 ELSE 0 END) AS VoteCount
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// PostHistory PH ON P.Id = PH.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.Score
// ),
// UserStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(DISTINCT P.Id) AS QuestionCount,
// SUM(P.Score) AS TotalScore,
// SUM(U.UpVotes) AS TotalUpVotes,
// SUM(U.DownVotes) AS TotalDownVotes
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// WHERE
// P.PostTypeId = 1
// GROUP BY
// U.Id, U.DisplayName
// )
// SELECT
// PS.PostId,
// PS.Title,
// PS.CreationDate,
// PS.Score AS PostScore,
// PS.CommentCount,
// PS.EditCount,
// PS.VoteCount,
// US.UserId,
// US.DisplayName AS UserDisplayName,
// US.QuestionCount,
// US.TotalScore,
// US.TotalUpVotes,
// US.TotalDownVotes
// FROM
// PostStats PS
// JOIN
// UserStats US ON PS.PostId = US.UserId
// ORDER BY
// PS.Score DESC, PS.CommentCount DESC
// LIMIT 100;
fn q11158(db: &'static So) -> String {
    let uid = uids(db);
    let hp = history_per_post(db);
    let qf = questions_only(db)
        .group_by(&db.post.owner_user)
        .select((&db.post.score).and((&db.post.owner_user).select((&db.user.up_votes).and(&db.user.down_votes))))
        .fold([0i64; 4], |a, (s, (u, d))| [a[0] + 1, a[1] + s, a[2] + u, a[3] + d]);
    let mut v = Vec::new();
    stats_fold(db, db.post.iq(), Ident::<Post>::new(), "chv", &[])
        .and(&hp)
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&qf)))
        .drive(|p, ((s, h), (u, q))| v.push((p, s, h, u, q)));
    out(v, |&(p, s, ..)| (score_desc(db, p), Reverse(s.cx)), 100, |&(p, s, h, u, q)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score"]);
        f.extend([V::I(s.cx), V::I(h), V::I(s.vx), user_col(db, u, "uid"), user_col(db, u, "name"), V::I(q[0]), V::I(q[1])]);
        f.extend([V::I(q[2]), V::I(q[3])]);
        f
    })
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(CASE WHEN p.PostTypeId IN (1, 2) THEN p.Score ELSE 0 END) AS TotalScore,
// SUM(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers
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
// ups.TotalScore,
// ups.AcceptedAnswers,
// COALESCE(bs.TotalBadges, 0) AS TotalBadges,
// COALESCE(bs.GoldBadges, 0) AS GoldBadges,
// COALESCE(bs.SilverBadges, 0) AS SilverBadges,
// COALESCE(bs.BronzeBadges, 0) AS BronzeBadges
// FROM
// UserPostStats ups
// LEFT JOIN
// BadgeStats bs ON ups.UserId = bs.UserId
// ORDER BY
// ups.TotalScore DESC
// LIMIT 100;
fn q12386(db: &'static So) -> String {
    let Post { post_type_id, score, accepted_answer_id, .. } = &db.post;
    let uf = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(score).and(accepted_answer_id.opt())).fold([0i64; 5], |a, ((t, s), acc)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + if matches!(t, 1 | 2) { s } else { 0 }, a[4] + acc.is_some() as i64]
    });
    let bc = badge_classes(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&uf).opt()).and((&bc).opt())).drive(|_, ((u, p), b)| v.push((u, p.unwrap_or([0; 5]), bz(b))));
    out(v, |&(_, p, _)| Reverse(p[3]), 100, |&(u, p, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&p));
        f.extend(ints(&b));
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
// BadgeCounts AS (
// SELECT
// UserId,
// COUNT(*) AS BadgeCount
// FROM
// Badges
// GROUP BY
// UserId
// ),
// UserPerformance AS (
// SELECT
// us.UserId,
// us.Reputation,
// us.PostCount,
// us.QuestionCount,
// us.AnswerCount,
// us.UpVotes,
// us.DownVotes,
// COALESCE(bc.BadgeCount, 0) AS BadgeCount
// FROM
// UserStats us
// LEFT JOIN
// BadgeCounts bc ON us.UserId = bc.UserId
// )
// SELECT
// UserId,
// Reputation,
// PostCount,
// QuestionCount,
// AnswerCount,
// UpVotes,
// DownVotes,
// BadgeCount,
// (PostCount * 1.0 / NULLIF(QuestionCount, 0)) AS AnswerQuestionRatio,
// (UpVotes * 1.0 / NULLIF(PostCount, 0)) AS UpvotePostRatio
// FROM
// UserPerformance
// ORDER BY
// Reputation DESC;
fn q10042(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    (&us).and((&dp).opt()).and(&bu).drive(|u, ((a, d), b)| v.push((u, a, d.unwrap_or(0), b)));
    rows(v.iter().map(|&(u, a, d, b)| row(vec![user_col(db, u, "uid"), user_col(db, u, "rep"), V::I(d), V::I(a.q), V::I(a.a), V::I(a.up), V::I(a.down), V::I(b), ratio(d, a.q), ratio(a.up, d)])))
}

// WITH UserMetrics AS (
// SELECT
// U.Id AS UserId,
// U.Reputation,
// U.Views,
// U.UpVotes,
// U.DownVotes,
// COUNT(DISTINCT P.Id) AS PostCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// COUNT(DISTINCT C.Id) AS CommentCount
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// GROUP BY
// U.Id, U.Reputation, U.Views, U.UpVotes, U.DownVotes
// ),
// PostHistoryMetrics AS (
// SELECT
// PH.UserId,
// COUNT(DISTINCT PH.Id) AS EditCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionEdits,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerEdits
// FROM
// PostHistory PH
// LEFT JOIN
// Posts P ON PH.PostId = P.Id
// GROUP BY
// PH.UserId
// )
// SELECT
// U.UserId,
// U.Reputation,
// U.Views,
// U.UpVotes,
// U.DownVotes,
// U.PostCount,
// U.QuestionCount,
// U.AnswerCount,
// U.CommentCount,
// COALESCE(PHM.EditCount, 0) AS EditCount,
// COALESCE(PHM.QuestionEdits, 0) AS QuestionEdits,
// COALESCE(PHM.AnswerEdits, 0) AS AnswerEdits
// FROM
// UserMetrics U
// LEFT JOIN
// PostHistoryMetrics PHM ON U.UserId = PHM.UserId
// ORDER BY
// U.Reputation DESC;
fn q10540(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "c", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let ph = db.post_history.group_by(&db.post_history.user).select((&db.post_history.post).select(&db.post.post_type_id).opt()).fold([0i64; 3], |a, t| {
        [a[0] + 1, a[1] + (t == Some(1)) as i64, a[2] + (t == Some(2)) as i64]
    });
    let mut v = Vec::new();
    (&us).and((&dp).opt()).and((&ph).opt()).drive(|u, ((a, d), h)| v.push((u, a, d.unwrap_or(0), h.unwrap_or([0; 3]))));
    rows(v.iter().map(|&(u, a, d, h)| {
        let mut f = ["uid", "rep", "uviews", "uup", "udown"].iter().map(|c| user_col(db, u, c)).collect::<Vec<_>>();
        f.extend(ints(&[d, a.q, a.a, a.cx]));
        f.extend(ints(&h));
        row(f)
    }))
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(DISTINCT P.Id) AS PostCount,
// COALESCE(SUM(V.BountyAmount), 0) AS TotalBounty,
// COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpVotes,
// COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownVotes,
// AVG(U.Reputation) AS AverageReputation
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// U.Id, U.DisplayName
// ),
// PostStats AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.Score,
// P.ViewCount,
// P.AnswerCount,
// P.CommentCount,
// P.FavoriteCount,
// U.DisplayName AS OwnerDisplayName,
// P.OwnerUserId
// FROM
// Posts P
// JOIN
// Users U ON P.OwnerUserId = U.Id
// )
// SELECT
// US.UserId,
// US.DisplayName,
// US.PostCount,
// US.TotalBounty,
// US.TotalUpVotes,
// US.TotalDownVotes,
// US.AverageReputation,
// PS.PostId,
// PS.Title,
// PS.CreationDate,
// PS.Score,
// PS.ViewCount,
// PS.AnswerCount,
// PS.CommentCount,
// PS.FavoriteCount,
// PS.OwnerDisplayName
// FROM
// UserStats US
// LEFT JOIN
// PostStats PS ON US.UserId = PS.OwnerUserId
// ORDER BY
// US.PostCount DESC, US.TotalBounty DESC;
fn q12677(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and(&us).and((&dp).opt()).and(posts_of(db).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(((u, a), d), p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(d.unwrap_or(0)), V::I(a.bounty_sum), V::I(a.up), V::I(a.down), V::F(db.user.reputation.get(u).unwrap() as f64)];
        f.extend(p.map_or(nulls(9), |p| post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "favorites", "owner"])));
        row(f)
    }))
}

// WITH PostActivity AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// p.AnswerCount,
// p.CommentCount,
// p.FavoriteCount,
// u.Id AS UserId,
// u.DisplayName AS OwnerDisplayName,
// u.Reputation AS OwnerReputation
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
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
// PostVotes AS (
// SELECT
// PostId,
// COUNT(CASE WHEN VoteTypeId = 2 THEN 1 END) AS UpVotes,
// COUNT(CASE WHEN VoteTypeId = 3 THEN 1 END) AS DownVotes
// FROM
// Votes
// GROUP BY
// PostId
// )
// SELECT
// pa.PostId,
// pa.Title,
// pa.CreationDate,
// pa.Score,
// pa.ViewCount,
// pa.AnswerCount,
// pa.CommentCount,
// pa.FavoriteCount,
// pa.OwnerDisplayName,
// pa.OwnerReputation,
// COALESCE(ub.BadgeCount, 0) AS OwnerBadgeCount,
// COALESCE(pv.UpVotes, 0) AS UpVotes,
// COALESCE(pv.DownVotes, 0) AS DownVotes
// FROM
// PostActivity pa
// LEFT JOIN
// UserBadges ub ON pa.UserId = ub.UserId
// LEFT JOIN
// PostVotes pv ON pa.PostId = pv.PostId
// ORDER BY
// pa.Score DESC,
// pa.ViewCount DESC, pa.PostId
// LIMIT 100;
// (rewrites/13658.sql: PostId tiebreak added)
fn q13658(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let pv = post_votes(db);
    let mut v = Vec::new();
    owned_since(db, year_ago()).select(Ident::<Post>::new().and((&db.post.owner_user).select(&bu)).and((&pv).opt())).drive(|_, ((p, b), x)| v.push((p, b, x.unwrap_or([0; 3]))));
    out(v, |&(p, ..)| (score_views(db, p), db.post.origid.get(p).unwrap()), 100, |&(p, b, x)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "favorites", "owner", "rep"]);
        f.extend(ints(&[b, x[1], x[2]]));
        f
    })
}

// WITH UserInfo AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// AVG(u.Reputation) AS AvgReputation
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN Votes v ON p.Id = v.PostId
// GROUP BY u.Id, u.DisplayName
// ),
// BadgeStats AS (
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
// ui.UserId,
// ui.DisplayName,
// ui.PostCount,
// ui.QuestionCount,
// ui.AnswerCount,
// ui.UpVotes,
// ui.DownVotes,
// ui.AvgReputation,
// COALESCE(bs.BadgeCount, 0) AS BadgeCount,
// COALESCE(bs.GoldBadges, 0) AS GoldBadges,
// COALESCE(bs.SilverBadges, 0) AS SilverBadges,
// COALESCE(bs.BronzeBadges, 0) AS BronzeBadges
// FROM UserInfo ui
// LEFT JOIN BadgeStats bs ON ui.UserId = bs.UserId
// ORDER BY ui.AvgReputation DESC
// LIMIT 100;
fn q14234(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&us).and((&dp).opt()).and((&bc).opt()).drive(|u, ((a, d), b)| v.push((u, a, d.unwrap_or(0), bz(b))));
    out(v, |&(u, ..)| rep_desc(db, u), 100, |&(u, a, d, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&[d, a.q, a.a, a.up, a.down]));
        f.push(V::F(db.user.reputation.get(u).unwrap() as f64));
        f.extend(ints(&b));
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
// SUM(CASE WHEN p.PostTypeId = 1 THEN p.Score ELSE 0 END) AS TotalQuestionScore,
// SUM(CASE WHEN p.PostTypeId = 2 THEN p.Score ELSE 0 END) AS TotalAnswerScore,
// SUM(COALESCE(v.Id, 0)) AS TotalVotes
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
// )
// SELECT
// us.UserId,
// us.DisplayName,
// us.PostCount,
// us.QuestionCount,
// us.AnswerCount,
// us.TotalQuestionScore,
// us.TotalAnswerScore,
// us.TotalVotes,
// bs.BadgeCount,
// bs.GoldBadges,
// bs.SilverBadges,
// bs.BronzeBadges
// FROM
// UserStats us
// LEFT JOIN
// BadgeStats bs ON us.UserId = bs.UserId
// ORDER BY
// us.TotalVotes DESC, us.PostCount DESC
// LIMIT 100;
fn q13478(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let uf = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(votes_of(db).select(&db.vote.origid).opt())).opt())
        .fold([0i64; 5], |a, p| match p {
            Some(((t, s), i)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + if t == 1 { s } else { 0 }, a[3] + if t == 2 { s } else { 0 }, a[4] + i.unwrap_or(0)],
            None => a,
        });
    let np = ud(db, UserWhere::All, posts_of(db));
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&uf).and((&np).opt()).and((&bc).opt()).drive(|u, ((a, n), b)| v.push((u, [n.unwrap_or(0), a[0], a[1], a[2], a[3], a[4]], b)));
    out(v, |&(_, p, _)| (Reverse(p[5]), Reverse(p[0])), 100, |&(u, p, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&p));
        f.extend(b.map_or(nulls(4), |b| ints(&b)));
        f
    })
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.Reputation,
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
// Votes V ON P.Id = V.PostId AND V.UserId = U.Id
// GROUP BY
// U.Id, U.Reputation
// ),
// PostStats AS (
// SELECT
// P.Id AS PostId,
// P.OwnerUserId,
// P.PostTypeId,
// P.CreationDate,
// P.Score,
// P.ViewCount,
// COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
// COUNT(CASE WHEN PH.Id IS NOT NULL THEN 1 END) AS HistoryCount
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// PostHistory PH ON P.Id = PH.PostId
// GROUP BY
// P.Id, P.OwnerUserId, P.PostTypeId, P.CreationDate, P.Score, P.ViewCount
// )
// SELECT
// U.UserId,
// U.Reputation,
// U.PostCount,
// U.BadgeCount,
// U.UpVotes,
// U.DownVotes,
// P.PostId,
// P.Score,
// P.ViewCount,
// P.CommentCount,
// P.HistoryCount
// FROM
// UserStats U
// JOIN
// PostStats P ON U.UserId = P.OwnerUserId
// ORDER BY
// U.Reputation DESC, P.Score DESC;
fn q10465(db: &'static So) -> String {
    let sv = self_votes(db);
    let bu = badges_per_user(db);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let uv = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&sv).select(&db.vote.vote_type_id).opt()).opt().and(badges_of(db).opt()))
        .fold([0i64; 2], |a, (t, _)| {
            let t = t.flatten();
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
        });
    let mut v = Vec::new();
    stats_fold(db, owned(db), Ident::<Post>::new(), "ch", &[])
        .and((&db.post.owner_user).select(Ident::<User>::new().and(&dp).and(&bu).and((&uv).opt())))
        .drive(|p, (s, (((u, d), b), x))| v.push((p, s, u, d, b, x.unwrap_or([0; 2]))));
    rows(v.iter().map(|&(p, s, u, d, b, x)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep"), V::I(d), V::I(b), V::I(x[0]), V::I(x[1])];
        f.extend(post_fields(db, p, &["id", "score", "views"]));
        f.extend([V::I(s.cx), V::I(s.hx)]);
        row(f)
    }))
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
// ActivePosts AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositiveScores,
// AVG(p.ViewCount) AS AvgViewCount
// FROM Posts p
// WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '6 months'
// GROUP BY p.OwnerUserId
// ),
// UserActivity AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COALESCE(ub.BadgeCount, 0) AS TotalBadges,
// COALESCE(ap.PostCount, 0) AS TotalPosts,
// COALESCE(ap.PositiveScores, 0) AS PositiveScores,
// COALESCE(ap.AvgViewCount, 0) AS AverageViewCount
// FROM Users u
// LEFT JOIN UserBadgeCounts ub ON u.Id = ub.UserId
// LEFT JOIN ActivePosts ap ON u.Id = ap.OwnerUserId
// )
// SELECT
// ua.UserId,
// ua.DisplayName,
// ua.TotalBadges,
// ua.TotalPosts,
// ua.PositiveScores,
// ua.AverageViewCount
// FROM UserActivity ua
// WHERE ua.TotalPosts > 0
// ORDER BY ua.TotalBadges DESC, ua.PositiveScores DESC, ua.AverageViewCount DESC
// LIMIT 10;
fn q6683(db: &'static So) -> String {
    let ub = ubc(db);
    let Post { score, view_count, .. } = &db.post;
    let ap = owned_since(db, ts(2024, 4, 1, 12, 34, 56)).group_by(&db.post.owner_user).select(score.and(view_count.opt())).fold([0i64; 4], |a, (s, w)| {
        [a[0] + 1, a[1] + (s > 0) as i64, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]
    });
    let mut v = Vec::new();
    (&ub).and(&ap).drive(|u, (b, p)| v.push((u, b[0], p)));
    let af = |p: [i64; 4]| if p[2] == 0 { 0.0 } else { p[3] as f64 / p[2] as f64 };
    out(v, |&(_, b, p)| (Reverse(b), Reverse(p[1]), Reverse(fkey(af(p)))), 10, |&(u, b, p)| {
        vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(b), V::I(p[0]), V::I(p[1]), or0(p[3], p[2])]
    })
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// COUNT(c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
// COUNT(DISTINCT ph.Id) AS EditHistoryCount
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
// p.Id, p.Title, p.CreationDate
// ),
// UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(b.Class) AS TotalBadgeClass
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
// ps.UpVoteCount,
// ps.DownVoteCount,
// ps.EditHistoryCount,
// us.UserId,
// us.DisplayName,
// us.PostCount,
// us.TotalBadgeClass
// FROM
// PostStats ps
// JOIN
// UserStats us ON ps.PostId = us.UserId
// ORDER BY
// ps.CreationDate DESC;
fn q12745(db: &'static So) -> String {
    let uid = uids(db);
    let ps = pstat(db, db.post.iq());
    let cs = user_class_rows(db);
    let mut v = Vec::new();
    stats_fold(db, since(db, year_ago()).with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cvh", &[])
        .and(votes_per_post(db))
        .and(history_per_post(db))
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and((&ps).opt()).and(&cs)))
        .drive(|p, (((s, x), h), ((u, q), c))| v.push((p, s, x, h, u, pz(q)[0], c)));
    rows(v.iter().map(|&(p, s, x, h, u, n, c)| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(s.cx), V::I(x), V::I(s.up), V::I(s.down), V::I(h), user_col(db, u, "uid"), user_col(db, u, "name"), V::I(n)]);
        f.push(nullable(c[1], c[0]));
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
// LEFT JOIN Badges b ON u.Id = b.UserId
// GROUP BY
// u.Id, u.DisplayName
// ),
// PostStatistics AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(p.Score) AS TotalScore,
// SUM(p.ViewCount) AS TotalViews
// FROM
// Posts p
// WHERE
// p.CreationDate > cast('2024-10-01' as date) - INTERVAL '1 year'
// GROUP BY
// p.OwnerUserId
// )
// SELECT
// ub.UserId,
// ub.DisplayName,
// ub.BadgeCount,
// ub.GoldCount,
// ub.SilverCount,
// ub.BronzeCount,
// COALESCE(ps.PostCount, 0) AS PostCount,
// COALESCE(ps.QuestionCount, 0) AS QuestionCount,
// COALESCE(ps.AnswerCount, 0) AS AnswerCount,
// COALESCE(ps.TotalScore, 0) AS TotalScore,
// COALESCE(ps.TotalViews, 0) AS TotalViews
// FROM
// UserBadges ub
// LEFT JOIN PostStatistics ps ON ub.UserId = ps.OwnerUserId
// ORDER BY
// ub.BadgeCount DESC,
// TotalScore DESC
// LIMIT 50;
fn q8020(db: &'static So) -> String {
    let ub = ubc(db);
    let ps = pstat(db, db.post.with((&db.post.creation_date).gt(date(2023, 10, 1))));
    let mut v = Vec::new();
    (&ub).and((&ps).opt()).drive(|u, (b, p)| v.push((u, b, pz(p))));
    out(v, |&(_, b, p)| (Reverse(b[0]), Reverse(p[3])), 50, |&(u, b, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&b));
        f.extend(ints(&[p[0], p[1], p[2], p[3], p[5]]));
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
// SUM(COALESCE(P.ViewCount, 0)) AS TotalViews,
// SUM(COALESCE(P.Score, 0)) AS TotalScore,
// AVG(COALESCE(P.Score, 0)) AS AvgScore
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
// )
// SELECT
// US.UserId,
// US.DisplayName,
// US.TotalPosts,
// US.TotalQuestions,
// US.TotalAnswers,
// US.TotalViews,
// US.TotalScore,
// US.AvgScore,
// COALESCE(BS.TotalBadges, 0) AS TotalBadges,
// COALESCE(BS.GoldBadges, 0) AS GoldBadges,
// COALESCE(BS.SilverBadges, 0) AS SilverBadges,
// COALESCE(BS.BronzeBadges, 0) AS BronzeBadges
// FROM
// UserStatistics US
// LEFT JOIN
// BadgeStatistics BS ON US.UserId = BS.UserId
// ORDER BY
// US.TotalScore DESC, US.TotalPosts DESC;
fn q14040(db: &'static So) -> String {
    let ps = pstat(db, db.post.iq());
    let bc = badge_classes(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&ps).opt()).and((&bc).opt())).drive(|_, ((u, p), b)| v.push((u, pz(p), bz(b))));
    rows(v.iter().map(|&(u, p, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&[p[0], p[1], p[2], p[5], p[3]]));
        f.push(or0(p[3], p[0]));
        f.extend(ints(&b));
        row(f)
    }))
}

// WITH UserStatistics AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(DISTINCT P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// SUM(CASE WHEN C.Id IS NOT NULL THEN 1 ELSE 0 END) AS Comments
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// GROUP BY
// U.Id, U.DisplayName
// ),
// PostActivity AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.ViewCount,
// P.CreationDate,
// P.LastActivityDate,
// CASE
// WHEN P.AcceptedAnswerId IS NOT NULL THEN 1
// ELSE 0
// END AS HasAcceptedAnswer,
// P.OwnerUserId -- added to the group by
// FROM
// Posts P
// )
// SELECT
// U.DisplayName,
// U.TotalPosts,
// U.Questions,
// U.Answers,
// U.UpVotes,
// U.DownVotes,
// U.Comments,
// P.Title,
// P.ViewCount,
// P.CreationDate,
// P.LastActivityDate,
// P.HasAcceptedAnswer
// FROM
// UserStatistics U
// JOIN
// PostActivity P ON U.UserId = P.OwnerUserId
// ORDER BY
// U.UpVotes DESC, U.TotalPosts DESC;
fn q14258(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "vc", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let mut v = Vec::new();
    owned(db).select(Ident::<Post>::new().and((&db.post.owner_user).select(Ident::<User>::new().and(&us).and(&dp)))).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(p, ((u, a), d))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(ints(&[d, a.q, a.a, a.up, a.down, a.cx]));
        f.extend(post_fields(db, p, &["title", "views", "created", "activity"]));
        f.push(V::I(db.post.accepted_answer_id.get(p).is_some() as i64));
        row(f)
    }))
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS TotalQuestions,
// COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS TotalAnswers,
// SUM(p.Score) AS TotalScore,
// SUM(COALESCE(p.ViewCount, 0)) AS TotalViews
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName
// ),
// PostHistoryStats AS (
// SELECT
// ph.UserId,
// COUNT(ph.Id) AS TotalEdits,
// COUNT(DISTINCT ph.PostId) AS UniquePostsEdited
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
// ups.TotalQuestions,
// ups.TotalAnswers,
// ups.TotalScore,
// ups.TotalViews,
// COALESCE(phs.TotalEdits, 0) AS TotalEdits,
// COALESCE(phs.UniquePostsEdited, 0) AS UniquePostsEdited
// FROM
// UserPostStats ups
// LEFT JOIN
// PostHistoryStats phs ON ups.UserId = phs.UserId
// )
// SELECT
// cs.DisplayName,
// cs.TotalPosts,
// cs.TotalQuestions,
// cs.TotalAnswers,
// cs.TotalScore,
// cs.TotalViews,
// cs.TotalEdits,
// cs.UniquePostsEdited
// FROM
// CombinedStats cs
// WHERE
// cs.TotalPosts > 0
// ORDER BY
// cs.TotalScore DESC,
// cs.TotalPosts DESC
// LIMIT 10;
fn q6300(db: &'static So) -> String {
    let ps = pstat(db, db.post.iq());
    let hn = db.post_history.group_by(&db.post_history.user).select(&db.post_history.post).fold(0i64, |a, _| a + 1);
    let hd = db.post_history.group_by(&db.post_history.user).select(&db.post_history.post).count_distinct();
    let mut v = Vec::new();
    (&ps).filt(|p: [i64; 13]| p[0] > 0).and((&hn).opt()).and((&hd).opt()).drive(|u, ((p, h), d)| v.push((u, p, h.unwrap_or(0), d.unwrap_or(0))));
    out(v, |&(_, p, _, _)| (Reverse(p[3]), Reverse(p[0])), 10, |&(u, p, h, d)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(ints(&[p[0], p[1], p[2], p[3], p[5], h, d]));
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
// SUM(CASE WHEN p.ViewCount IS NOT NULL THEN p.ViewCount ELSE 0 END) AS TotalViews,
// SUM(CASE WHEN p.Score IS NOT NULL THEN p.Score ELSE 0 END) AS TotalScore
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
// ups.TotalQuestions,
// ups.TotalAnswers,
// ups.TotalViews,
// ups.TotalScore,
// COALESCE(bs.TotalBadges, 0) AS TotalBadges,
// COALESCE(bs.GoldBadges, 0) AS GoldBadges,
// COALESCE(bs.SilverBadges, 0) AS SilverBadges,
// COALESCE(bs.BronzeBadges, 0) AS BronzeBadges
// FROM
// UserPostStats ups
// LEFT JOIN
// BadgeStats bs ON ups.UserId = bs.UserId
// ORDER BY
// ups.TotalScore DESC, ups.TotalPosts DESC
// LIMIT 100;
fn q10814(db: &'static So) -> String {
    let ps = pstat(db, db.post.iq());
    let bc = badge_classes(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&ps).opt()).and((&bc).opt())).drive(|_, ((u, p), b)| v.push((u, pz(p), bz(b))));
    out(v, |&(_, p, _)| (Reverse(p[3]), Reverse(p[0])), 100, |&(u, p, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&[p[0], p[1], p[2], p[5], p[3]]));
        f.extend(ints(&b));
        f
    })
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(CASE WHEN pt.Name = 'Question' THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN pt.Name = 'Answer' THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
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
// p.Score,
// p.ViewCount,
// p.CommentCount,
// p.AcceptedAnswerId,
// COALESCE(u.DisplayName, 'Community') AS OwnerDisplayName,
// p.OwnerUserId
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
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
// ps.CreationDate,
// ps.Score,
// ps.ViewCount,
// ps.CommentCount,
// ps.OwnerDisplayName
// FROM
// UserStats us
// LEFT JOIN
// PostStats ps ON us.UserId = ps.OwnerUserId
// ORDER BY
// us.PostCount DESC, us.UpVotes DESC;
fn q12730(db: &'static So) -> String {
    let uf = owned(db).group_by(&db.post.owner_user).select(name(db).and(votes_of(db).select(&db.vote.vote_type_id).opt())).fold([0i64; 4], |a, (n, t)| {
        [a[0] + (n == "Question") as i64, a[1] + (n == "Answer") as i64, a[2] + (t == Some(2)) as i64, a[3] + (t == Some(3)) as i64]
    });
    let dp = ud(db, UserWhere::All, posts_of(db));
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&uf).opt()).and((&dp).opt()).and(posts_of(db).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(((u, a), d), p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(d.unwrap_or(0))];
        f.extend(ints(&a.unwrap_or([0; 4])));
        f.extend(p.map_or(nulls(7), |p| post_fields(db, p, &["id", "title", "created", "score", "views", "comments", "owner"])));
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
// COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
// COUNT(CASE WHEN A.Id IS NOT NULL THEN 1 END) AS AnswerCount,
// COUNT(DISTINCT V.Id) AS VoteCount
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Posts A ON P.Id = A.ParentId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// P.Id, P.PostTypeId, P.CreationDate, P.Score, P.ViewCount
// ),
// UserStats AS (
// SELECT
// U.Id AS UserId,
// SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges,
// AVG(U.Reputation) AS AvgReputation
// FROM
// Users U
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// GROUP BY
// U.Id
// )
// SELECT
// PS.PostId,
// PS.PostTypeId,
// PS.CreationDate,
// PS.Score,
// PS.ViewCount,
// PS.CommentCount,
// PS.AnswerCount,
// PS.VoteCount,
// US.UserId,
// US.GoldBadges,
// US.SilverBadges,
// US.BronzeBadges,
// US.AvgReputation
// FROM
// PostStats PS
// JOIN
// Users U ON PS.PostTypeId = U.AccountId
// JOIN
// UserStats US ON U.Id = US.UserId
// ORDER BY
// PS.ViewCount DESC, PS.Score DESC;
fn q13586(db: &'static So) -> String {
    let acct: HashIdx<i64, Id<User>> = (&db.user.account_id).inv().collect();
    let bc = badge_classes(db);
    let mut v = Vec::new();
    stats_fold(db, db.post.iq(), Ident::<Post>::new(), "cav", &[])
        .and(votes_per_post(db))
        .and((&db.post.post_type_id).select(&acct).select(Ident::<User>::new().and((&bc).opt())))
        .drive(|p, ((s, x), (u, b))| v.push((p, s, x, u, bz(b))));
    rows(v.iter().map(|&(p, s, x, u, b)| {
        let mut f = post_fields(db, p, &["id", "type_id", "created", "score", "views"]);
        f.extend([V::I(s.cx), V::I(s.ax), V::I(x), user_col(db, u, "uid"), V::I(b[1]), V::I(b[2]), V::I(b[3]), V::F(db.user.reputation.get(u).unwrap() as f64)]);
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
// COALESCE(U.DisplayName, 'Community User') AS OwnerDisplayName,
// COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
// FROM
// Posts P
// LEFT JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.Score, P.ViewCount, P.AnswerCount, U.DisplayName
// ),
// AverageMetrics AS (
// SELECT
// AVG(Score) AS AvgScore,
// AVG(ViewCount) AS AvgViewCount,
// AVG(CommentCount) AS AvgCommentCount,
// AVG(UpVoteCount) AS AvgUpVoteCount,
// AVG(DownVoteCount) AS AvgDownVoteCount
// FROM
// PostMetrics
// )
// SELECT
// PM.PostId,
// PM.Title,
// PM.CreationDate,
// PM.Score,
// PM.ViewCount,
// PM.AnswerCount,
// PM.OwnerDisplayName,
// PM.CommentCount,
// PM.UpVoteCount,
// PM.DownVoteCount,
// AM.AvgScore,
// AM.AvgViewCount,
// AM.AvgCommentCount,
// AM.AvgUpVoteCount,
// AM.AvgDownVoteCount
// FROM
// PostMetrics PM
// CROSS JOIN
// AverageMetrics AM
// ORDER BY
// PM.Score DESC;
fn q13311(db: &'static So) -> String {
    let sf = stats_fold(db, db.post.iq(), Ident::<Post>::new(), "cv", &[]);
    let Post { score, view_count, .. } = &db.post;
    let t = db.post.select(score.and(view_count.opt()).and(&sf)).fold_flat([0i64; 7], |a, ((s, w), x)| {
        [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + x.cx, a[5] + x.up, a[6] + x.down]
    });
    let mut v = Vec::new();
    (&sf).drive(|p, s| v.push((p, s)));
    rows(v.iter().map(|&(p, s)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers"]);
        f.push(named_owner(db, p, "Community User"));
        f.extend(ints(&[s.cx, s.up, s.down]));
        f.extend([avg(t[1], t[0]), avg(t[3], t[2]), avg(t[4], t[0]), avg(t[5], t[0]), avg(t[6], t[0])]);
        row(f)
    }))
}

// WITH UserBadgeCounts AS (
// SELECT
// u.Id AS UserId,
// COUNT(b.Id) AS BadgeCount,
// MAX(u.Reputation) AS MaxReputation
// FROM Users u
// LEFT JOIN Badges b ON u.Id = b.UserId
// GROUP BY u.Id
// ),
// PostStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN p.Score ELSE 0 END) AS QuestionScore,
// SUM(CASE WHEN p.PostTypeId = 2 THEN p.Score ELSE 0 END) AS AnswerScore,
// AVG(p.ViewCount) AS AvgViewCount
// FROM Posts p
// WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY p.OwnerUserId
// ),
// UserActivity AS (
// SELECT
// u.Id AS UserId,
// COALESCE(ubc.BadgeCount, 0) AS BadgeCount,
// COALESCE(ps.PostCount, 0) AS PostCount,
// COALESCE(ps.QuestionScore, 0) AS QuestionScore,
// COALESCE(ps.AnswerScore, 0) AS AnswerScore,
// COALESCE(ps.AvgViewCount, 0) AS AvgViewCount
// FROM Users u
// LEFT JOIN UserBadgeCounts ubc ON u.Id = ubc.UserId
// LEFT JOIN PostStats ps ON u.Id = ps.OwnerUserId
// )
// SELECT
// u.DisplayName,
// ua.BadgeCount,
// ua.PostCount,
// ua.QuestionScore,
// ua.AnswerScore,
// ua.AvgViewCount,
// u.Reputation,
// u.CreationDate,
// u.LastAccessDate
// FROM Users u
// JOIN UserActivity ua ON u.Id = ua.UserId
// WHERE u.Reputation > 1000
// ORDER BY ua.QuestionScore DESC, ua.AnswerScore DESC
// LIMIT 50;
fn q8397(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let ps = owned_since(db, year_ago()).group_by(&db.post.owner_user).select(post_type_id.and(score).and(view_count.opt())).fold([0i64; 5], |a, ((t, s), w)| {
        [a[0] + 1, a[1] + if t == 1 { s } else { 0 }, a[2] + if t == 2 { s } else { 0 }, a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0)]
    });
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and(&bu).and((&ps).opt())).drive(|_, ((u, b), p)| v.push((u, b, p.unwrap_or([0; 5]))));
    out(v, |&(_, _, p)| (Reverse(p[1]), Reverse(p[2])), 50, |&(u, b, p)| {
        vec![user_col(db, u, "name"), V::I(b), V::I(p[0]), V::I(p[1]), V::I(p[2]), or0(p[4], p[3]), user_col(db, u, "rep"), user_col(db, u, "ucreated"), user_col(db, u, "last_access")]
    })
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.Reputation,
// U.CreationDate,
// COUNT(DISTINCT P.Id) AS TotalPosts,
// COUNT(DISTINCT CASE WHEN P.PostTypeId = 1 THEN P.Id END) AS Questions,
// COUNT(DISTINCT CASE WHEN P.PostTypeId = 2 THEN P.Id END) AS Answers,
// SUM(COALESCE(P.Score, 0)) AS TotalScore,
// SUM(COALESCE(P.ViewCount, 0)) AS TotalViews,
// SUM(COALESCE(P.CommentCount, 0)) AS TotalComments
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id, U.Reputation, U.CreationDate
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
// U.UserId,
// U.Reputation,
// U.CreationDate,
// U.TotalPosts,
// U.Questions,
// U.Answers,
// U.TotalScore,
// U.TotalViews,
// U.TotalComments,
// COALESCE(B.TotalBadges, 0) AS TotalBadges,
// COALESCE(B.GoldBadges, 0) AS GoldBadges,
// COALESCE(B.SilverBadges, 0) AS SilverBadges,
// COALESCE(B.BronzeBadges, 0) AS BronzeBadges
// FROM
// UserStats U
// LEFT JOIN
// BadgeStats B ON U.UserId = B.UserId
// ORDER BY
// U.Reputation DESC;
fn q14397(db: &'static So) -> String {
    let ps = pstat(db, db.post.iq());
    let pc = pcc(db);
    let bc = badge_classes(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&ps).opt()).and((&pc).opt()).and((&bc).opt())).drive(|_, (((u, p), c), b)| v.push((u, pz(p), c.map_or(0, |c| c[0]), bz(b))));
    rows(v.iter().map(|&(u, p, c, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep"), user_col(db, u, "ucreated")];
        f.extend(ints(&[p[0], p[1], p[2], p[3], p[5], c]));
        f.extend(ints(&b));
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
// UserPosts AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS PostCount,
// SUM(P.Score) AS TotalScore,
// AVG(P.ViewCount) AS AverageViews
// FROM
// Posts P
// WHERE
// P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 YEAR'
// GROUP BY
// P.OwnerUserId
// ),
// CombinedStats AS (
// SELECT
// UB.UserId,
// UB.DisplayName,
// UB.BadgeCount,
// UB.GoldBadges,
// UB.SilverBadges,
// UB.BronzeBadges,
// COALESCE(UP.PostCount, 0) AS PostCount,
// COALESCE(UP.TotalScore, 0) AS TotalScore,
// COALESCE(UP.AverageViews, 0) AS AverageViews
// FROM
// UserBadges UB
// LEFT JOIN
// UserPosts UP ON UB.UserId = UP.OwnerUserId
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
// AverageViews
// FROM
// CombinedStats
// ORDER BY
// TotalScore DESC, BadgeCount DESC
// LIMIT 10;
fn q5191(db: &'static So) -> String {
    let ub = ubc(db);
    let ps = pstat(db, since(db, year_ago()));
    let mut v = Vec::new();
    (&ub).and((&ps).opt()).drive(|u, (b, p)| v.push((u, b, pz(p))));
    out(v, |&(_, b, p)| (Reverse(p[3]), Reverse(b[0])), 10, |&(u, b, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&b));
        f.extend([V::I(p[0]), V::I(p[3]), or0(p[5], p[4])]);
        f
    })
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// COUNT(c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
// COALESCE(SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadgeCount,
// COALESCE(SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadgeCount,
// COALESCE(SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadgeCount
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
// PostHistoryStats AS (
// SELECT
// PostId,
// COUNT(*) AS EditCount,
// MAX(CreationDate) AS LastEditDate
// FROM
// PostHistory
// GROUP BY
// PostId
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.CommentCount,
// ps.VoteCount,
// ps.UpVoteCount,
// ps.DownVoteCount,
// ps.GoldBadgeCount,
// ps.SilverBadgeCount,
// ps.BronzeBadgeCount,
// COALESCE(ph.EditCount, 0) AS EditCount,
// ph.LastEditDate
// FROM
// PostStats ps
// LEFT JOIN
// PostHistoryStats ph ON ps.PostId = ph.PostId
// ORDER BY
// ps.CreationDate DESC
// LIMIT 100;
fn q13747(db: &'static So) -> String {
    let hf = history_n_max(db);
    let mut v = Vec::new();
    let bu = badges_by_uid(db);
    cvb_raw(db, db.post.iq(), &bu).and(votes_per_post(db)).and((&hf).opt()).drive(|p, ((s, x), h)| v.push((p, s, x, h)));
    out(v, |&(p, ..)| Reverse(db.post.creation_date.get(p).unwrap()), 100, |&(p, s, x, h)| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend(ints(&[s[0], x, s[2], s[3], s[4], s[5], s[6], h.map_or(0, |h| h.0)]));
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
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount
// ),
// UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(COALESCE(ps.Score, 0)) AS TotalScore
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// PostStats ps ON p.Id = ps.PostId
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
// us.PostCount,
// us.TotalScore
// FROM
// PostStats ps
// JOIN
// Users u ON ps.PostId = u.Id
// JOIN
// UserStats us ON u.Id = us.UserId
// ORDER BY
// ps.Score DESC, ps.ViewCount DESC;
fn q14594(db: &'static So) -> String {
    let uid = uids(db);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let bu = badges_per_user(db);
    let recent = Ident::<Post>::new().with((&db.post.creation_date).ge(date(2023, 1, 1)));
    let rs = db
        .user
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(posts_of(db).select(recent.select(&db.post.score).opt()).opt()))
        .fold(0i64, |a, (_, s)| a + s.flatten().unwrap_or(0));
    let mut v = Vec::new();
    stats_fold(db, since(db, date(2023, 1, 1)), Ident::<Post>::new(), "cv", &[])
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and((&dp).opt()).and(&bu).and(&rs)))
        .drive(|p, (s, (((u, d), _), r))| v.push((p, s, u, d.unwrap_or(0), r)));
    rows(v.iter().map(|&(p, s, u, d, r)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(s.cx), V::I(s.vx), user_col(db, u, "uid"), user_col(db, u, "name"), V::I(d), V::I(r)]);
        row(f)
    }))
}

// WITH UserBadges AS (
// SELECT
// UserId,
// COUNT(CASE WHEN Class = 1 THEN 1 END) AS GoldBadges,
// COUNT(CASE WHEN Class = 2 THEN 1 END) AS SilverBadges,
// COUNT(CASE WHEN Class = 3 THEN 1 END) AS BronzeBadges
// FROM Badges
// GROUP BY UserId
// ),
// PostStatistics AS (
// SELECT
// p.OwnerUserId,
// COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS QuestionCount,
// COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS AnswerCount,
// SUM(p.Score) AS TotalScore,
// SUM(p.ViewCount) AS TotalViews
// FROM Posts p
// LEFT JOIN Users u ON p.OwnerUserId = u.Id
// WHERE u.Reputation > 1000
// GROUP BY p.OwnerUserId
// ),
// RecentActivity AS (
// SELECT
// p.OwnerUserId,
// MAX(p.LastActivityDate) AS LastActivityDate
// FROM Posts p
// GROUP BY p.OwnerUserId
// )
// SELECT
// u.DisplayName,
// COALESCE(ub.GoldBadges, 0) AS GoldBadges,
// COALESCE(ub.SilverBadges, 0) AS SilverBadges,
// COALESCE(ub.BronzeBadges, 0) AS BronzeBadges,
// ps.QuestionCount,
// ps.AnswerCount,
// ps.TotalScore,
// ps.TotalViews,
// ra.LastActivityDate
// FROM Users u
// LEFT JOIN UserBadges ub ON u.Id = ub.UserId
// LEFT JOIN PostStatistics ps ON u.Id = ps.OwnerUserId
// LEFT JOIN RecentActivity ra ON u.Id = ra.OwnerUserId
// WHERE (ps.QuestionCount > 5 OR ps.AnswerCount > 10)
// AND ra.LastActivityDate > CURRENT_DATE - INTERVAL '6 months'
// ORDER BY ps.TotalScore DESC, u.DisplayName ASC;
fn q925(db: &'static So) -> String {
    let bc = badge_classes(db);
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let ps = db.post
        .with((&db.post.owner_user).select(&db.user.reputation).filt(|r: i64| r > 1000))
        .group_by(&db.post.owner_user)
        .select(post_type_id.and(score).and(view_count.opt()))
        .fold([0i64; 5], |a, ((t, s), w)| [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + s, a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0)]);
    let ra = owned(db).group_by(&db.post.owner_user).select(&db.post.last_activity_date).fold(i64::MIN, |a, d| a.max(d));
    let cut = add_months(current_date(), -6);
    let mut v = Vec::new();
    db.user
        .select(Ident::<User>::new().and((&bc).opt()).and((&ps).filt(|p: [i64; 5]| p[0] > 5 || p[1] > 10)).and((&ra).filt(move |d: i64| d > cut)))
        .drive(|_, (((u, b), p), r)| v.push((u, bz(b), p, r)));
    rows(v.iter().map(|&(u, b, p, r)| row(vec![user_col(db, u, "name"), V::I(b[1]), V::I(b[2]), V::I(b[3]), V::I(p[0]), V::I(p[1]), V::I(p[2]), nullable(p[4], p[3]), V::T(r)])))
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
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
// ),
// BadgeStats AS (
// SELECT
// b.UserId,
// COUNT(DISTINCT b.Id) AS TotalBadges,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM
// Badges b
// GROUP BY
// b.UserId
// )
// SELECT
// u.UserId,
// u.DisplayName,
// u.TotalPosts,
// u.Questions,
// u.Answers,
// u.TotalViews,
// u.UpVotes,
// u.DownVotes,
// COALESCE(b.TotalBadges, 0) AS TotalBadges,
// COALESCE(b.GoldBadges, 0) AS GoldBadges,
// COALESCE(b.SilverBadges, 0) AS SilverBadges,
// COALESCE(b.BronzeBadges, 0) AS BronzeBadges
// FROM
// UserStats u
// LEFT JOIN
// BadgeStats b ON u.UserId = b.UserId
// ORDER BY
// u.TotalPosts DESC
// LIMIT 100;
fn q13540(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&us).and((&dp).opt()).and((&bc).opt()).drive(|u, ((a, d), b)| v.push((u, a, d.unwrap_or(0), bz(b))));
    out(v, |&(_, _, d, _)| Reverse(d), 100, |&(u, a, d, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(d), V::I(a.q), V::I(a.a), ustat_field(&a, "views_sum"), V::I(a.up), V::I(a.down)];
        f.extend(ints(&b));
        f
    })
}

// WITH UserActivity AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS UpVotesCount,
// SUM(CASE WHEN p.Score < 0 THEN 1 ELSE 0 END) AS DownVotesCount,
// SUM(CASE WHEN b.Id IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount,
// COUNT(DISTINCT c.Id) AS CommentCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// GROUP BY
// u.Id, u.DisplayName
// ),
// PostSummary AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// pt.Name AS PostType,
// COUNT(DISTINCT c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount
// FROM
// Posts p
// LEFT JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.Title, pt.Name
// )
// SELECT
// ua.UserId,
// ua.DisplayName,
// ua.PostCount,
// ua.UpVotesCount,
// ua.DownVotesCount,
// ua.BadgeCount,
// ua.CommentCount,
// ps.PostId,
// ps.Title,
// ps.PostType,
// ps.CommentCount AS PostCommentCount,
// ps.VoteCount AS PostVoteCount
// FROM
// UserActivity ua
// JOIN
// PostSummary ps ON ua.UserId = ps.PostId
// ORDER BY
// ua.BadgeCount DESC
// LIMIT 100;
fn q14282(db: &'static So) -> String {
    let uid = uids(db);
    let cp = comments_per_post(db);
    let uf = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.score).and(comments_of(db).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 3], |a, (p, b)| {
            let s = p.map(|(s, _)| s);
            [a[0] + (s.map_or(false, |s| s > 0)) as i64, a[1] + (s.map_or(false, |s| s < 0)) as i64, a[2] + b.is_some() as i64]
        });
    let np = ud(db, UserWhere::All, posts_of(db));
    let nc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let mut v = Vec::new();
    db.post
        .select(Ident::<Post>::new().and(&cp).and(votes_per_post(db)).and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&uf).and((&np).opt()).and(&nc))))
        .drive(|_, (((p, c), x), (((u, a), n), k))| v.push((p, c, x, u, [n.unwrap_or(0), a[0], a[1], a[2], k])));
    out(v, |&(.., a)| Reverse(a[3]), 100, |&(p, c, x, u, a)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a));
        f.extend(post_fields(db, p, &["id", "title", "type"]));
        f.extend(ints(&[c, x]));
        f
    })
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(DISTINCT P.Id) AS PostCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(COALESCE(V.BountyAmount, 0)) AS TotalBountyAmount,
// SUM(COALESCE(C.Score, 0)) AS TotalCommentScore
// FROM Users U
// LEFT JOIN Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN Votes V ON P.Id = V.PostId
// LEFT JOIN Comments C ON P.Id = C.PostId
// GROUP BY U.Id, U.DisplayName
// ),
// PostStats AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.Score,
// P.ViewCount,
// COUNT(C.Id) AS CommentCount,
// SUM(CASE WHEN V.voteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN V.voteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
// FROM Posts P
// LEFT JOIN Comments C ON P.Id = C.PostId
// LEFT JOIN Votes V ON P.Id = V.PostId
// GROUP BY P.Id, P.Title, P.Score, P.ViewCount
// )
// SELECT
// U.DisplayName,
// U.PostCount,
// U.QuestionCount,
// U.AnswerCount,
// U.TotalBountyAmount,
// U.TotalCommentScore,
// P.Title AS PostTitle,
// P.Score AS PostScore,
// P.ViewCount AS PostViewCount,
// P.CommentCount AS PostCommentCount,
// P.UpVoteCount,
// P.DownVoteCount
// FROM UserStats U
// JOIN PostStats P ON U.UserId = P.PostId
// ORDER BY U.PostCount DESC, U.TotalBountyAmount DESC;
fn q12284(db: &'static So) -> String {
    let pid = pids(db);
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "vc", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let sf = stats_fold(db, db.post.iq(), Ident::<Post>::new(), "cv", &[]);
    let mut v = Vec::new();
    (&us).and((&dp).opt()).and((&db.user.origid).select(&pid).select(Ident::<Post>::new().and(&sf))).drive(|u, ((a, d), (p, s))| v.push((u, a, d.unwrap_or(0), p, s)));
    rows(v.iter().map(|&(u, a, d, p, s)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(ints(&[d, a.q, a.a, a.bounty_sum, a.cscore]));
        f.extend(post_fields(db, p, &["title", "score", "views"]));
        f.extend(ints(&[s.cx, s.up, s.down]));
        row(f)
    }))
}

// WITH UserBadges AS (
// SELECT U.Id AS UserId,
// U.DisplayName,
// COUNT(B.Id) AS BadgeCount,
// MAX(B.Date) AS LastBadgeDate
// FROM Users U
// LEFT JOIN Badges B ON U.Id = B.UserId
// GROUP BY U.Id, U.DisplayName
// ),
// TopUsers AS (
// SELECT UserId,
// DisplayName,
// BadgeCount,
// LastBadgeDate
// FROM UserBadges
// WHERE BadgeCount > 0
// ORDER BY BadgeCount DESC
// LIMIT 10
// ),
// PostStats AS (
// SELECT P.OwnerUserId,
// COUNT(P.Id) AS PostCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(COALESCE(P.ViewCount, 0)) AS TotalViews
// FROM Posts P
// WHERE P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY P.OwnerUserId
// ),
// UserPostStats AS (
// SELECT U.Id AS UserId,
// U.DisplayName,
// PS.PostCount,
// PS.QuestionCount,
// PS.AnswerCount,
// PS.TotalViews
// FROM Users U
// LEFT JOIN PostStats PS ON U.Id = PS.OwnerUserId
// )
// SELECT U.UserId,
// U.DisplayName,
// U.BadgeCount,
// U.LastBadgeDate,
// UPS.PostCount,
// UPS.QuestionCount,
// UPS.AnswerCount,
// UPS.TotalViews
// FROM TopUsers U
// LEFT JOIN UserPostStats UPS ON U.UserId = UPS.UserId
// ORDER BY U.BadgeCount DESC, UPS.TotalViews DESC;
fn q8936(db: &'static So) -> String {
    let ub = db.badge.group_by(&db.badge.user).select(&db.badge.date).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let ps = pstat(db, since(db, year_ago()));
    let mut v = Vec::new();
    (&ub).and((&ps).opt()).drive(|u, (b, p)| v.push((u, b, p)));
    cut(&mut v, |&(_, b, _)| Reverse(b.0), 10);
    rows(v.iter().map(|&(u, b, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(b.0), V::T(b.1)];
        f.extend(p.map_or(nulls(4), |p| ints(&[p[0], p[1], p[2], p[5]])));
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
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// u.Id, u.DisplayName
// ),
// BadgeCounts AS (
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
// us.UserId,
// us.DisplayName,
// us.PostCount,
// us.QuestionCount,
// us.AnswerCount,
// COALESCE(bc.BadgeCount, 0) AS TotalBadges,
// COALESCE(bc.GoldBadges, 0) AS GoldBadges,
// COALESCE(bc.SilverBadges, 0) AS SilverBadges,
// COALESCE(bc.BronzeBadges, 0) AS BronzeBadges,
// us.UpVotes,
// us.DownVotes
// FROM
// UserStats us
// LEFT JOIN
// BadgeCounts bc ON us.UserId = bc.UserId
// WHERE
// us.PostCount > 10
// ORDER BY
// us.UpVotes DESC, us.DownVotes ASC
// LIMIT 100;
fn q9110(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&us).and((&dp).filt(|d: i64| d > 10)).and((&bc).opt()).drive(|u, ((a, d), b)| v.push((u, a, d, bz(b))));
    out(v, |&(_, a, _, _)| (Reverse(a.up), a.down), 100, |&(u, a, d, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&[d, a.q, a.a]));
        f.extend(ints(&b));
        f.extend(ints(&[a.up, a.down]));
        f
    })
}

// WITH UserBadgeCounts AS (
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
// ActivePostStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(p.Score) AS TotalScore
// FROM
// Posts p
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY
// p.OwnerUserId
// ),
// UserPerformance AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COALESCE(ub.BadgeCount, 0) AS BadgeCount,
// COALESCE(ps.PostCount, 0) AS PostCount,
// COALESCE(ps.QuestionCount, 0) AS QuestionCount,
// COALESCE(ps.AnswerCount, 0) AS AnswerCount,
// COALESCE(ps.TotalScore, 0) AS TotalScore
// FROM
// Users u
// LEFT JOIN
// UserBadgeCounts ub ON u.Id = ub.UserId
// LEFT JOIN
// ActivePostStats ps ON u.Id = ps.OwnerUserId
// )
// SELECT
// UserId,
// DisplayName,
// Reputation,
// BadgeCount,
// PostCount,
// QuestionCount,
// AnswerCount,
// TotalScore
// FROM
// UserPerformance
// WHERE
// Reputation > 500
// ORDER BY
// TotalScore DESC, BadgeCount DESC
// LIMIT 10;
fn q6113(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let ps = pstat(db, since(db, year_ago()));
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(500)).select(Ident::<User>::new().and(&bu).and((&ps).opt())).drive(|_, ((u, b), p)| v.push((u, b, pz(p))));
    out(v, |&(_, b, p)| (Reverse(p[3]), Reverse(b)), 10, |&(u, b, p)| {
        vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep"), V::I(b), V::I(p[0]), V::I(p[1]), V::I(p[2]), V::I(p[3])]
    })
}

// WITH PostStats AS (
// SELECT
// P.Id AS PostId,
// P.PostTypeId,
// P.CreationDate,
// COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
// COUNT(CASE WHEN V.Id IS NOT NULL THEN 1 END) AS VoteCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// P.Id, P.PostTypeId, P.CreationDate
// ),
// UserStats AS (
// SELECT
// U.Id AS UserId,
// COUNT(DISTINCT P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersCount,
// SUM(CASE WHEN B.Id IS NOT NULL THEN 1 ELSE 0 END) AS BadgesCount
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// GROUP BY
// U.Id
// )
// SELECT
// PS.PostId,
// PS.PostTypeId,
// PS.CreationDate,
// PS.CommentCount,
// PS.VoteCount,
// PS.UpVoteCount,
// PS.DownVoteCount,
// US.UserId,
// US.TotalPosts,
// US.QuestionsCount,
// US.AnswersCount,
// US.BadgesCount
// FROM
// PostStats PS
// JOIN
// Users U ON U.Id = PS.PostId
// JOIN
// UserStats US ON U.Id = US.UserId
// ORDER BY
// PS.CreationDate DESC;
fn q14629(db: &'static So) -> String {
    let uid = uids(db);
    let ps = pstat(db, db.post.iq());
    let bu = user_pb(db);
    let mut v = Vec::new();
    stats_fold(db, db.post.with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cv", &[])
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and((&ps).opt()).and(&bu)))
        .drive(|p, (s, ((u, q), b))| v.push((p, s, u, pz(q), b)));
    rows(v.iter().map(|&(p, s, u, q, b)| {
        let mut f = post_fields(db, p, &["id", "type_id", "created"]);
        f.extend(ints(&[s.cx, s.vx, s.up, s.down]));
        f.push(user_col(db, u, "uid"));
        f.extend(ints(&[q[0], b[1], b[2], b[3]]));
        row(f)
    }))
}

// WITH StringProcessedPosts AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.Body,
// p.Tags,
// p.OwnerUserId,
// p.CreationDate,
// COALESCE(BadgesReceived.BadgeCount, 0) AS BadgeCount,
// COUNT(DISTINCT c.Id) AS CommentCount,
// ARRAY_LENGTH(string_to_array(p.Tags, '>'), 1) AS TagCount,
// LENGTH(p.Body) AS BodyLength,
// LENGTH(p.Title) AS TitleLength
// FROM
// Posts p
// LEFT JOIN (
// SELECT
// UserId,
// COUNT(*) AS BadgeCount
// FROM
// Badges
// GROUP BY
// UserId
// ) AS BadgesReceived ON p.OwnerUserId = BadgesReceived.UserId
// LEFT JOIN Comments c ON p.Id = c.PostId
// GROUP BY
// p.Id, p.Title, p.Body, p.Tags, p.OwnerUserId, p.CreationDate, BadgesReceived.BadgeCount
// ),
// AggregateData AS (
// SELECT
// TagCount,
// AVG(BodyLength) AS AvgBodyLength,
// AVG(TitleLength) AS AvgTitleLength,
// SUM(CommentCount) AS TotalComments,
// SUM(BadgeCount) AS TotalBadges
// FROM
// StringProcessedPosts
// GROUP BY
// TagCount
// )
// SELECT
// ad.TagCount,
// ad.AvgBodyLength,
// ad.AvgTitleLength,
// ad.TotalComments,
// ad.TotalBadges,
// CASE
// WHEN ad.TagCount < 5 THEN 'Low Tags'
// WHEN ad.TagCount BETWEEN 5 AND 10 THEN 'Medium Tags'
// ELSE 'High Tags'
// END AS TagCategory
// FROM
// AggregateData ad
// ORDER BY
// ad.TagCount;
fn q28876(db: &'static So) -> String {
    let bu = db.badge.group_by(&db.badge.user_id).fold(0i64, |n, _| n + 1);
    let cp = comments_per_post(db);
    let Post { body, title, .. } = &db.post;
    let tf = db.post
        .group_by((&db.post.tags_str).map(tag_n).opt())
        .select(body.and(title.opt()).and(&cp).and((&db.post.owner_user_id).select(&bu).opt()))
        .fold([0i64; 6], |a, (((b, t), c), n)| {
            [a[0] + 1, a[1] + chars(b), a[2] + t.is_some() as i64, a[3] + t.map_or(0, chars), a[4] + c, a[5] + n.unwrap_or(0)]
        });
    let mut v = Vec::new();
    (&tf).drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&(k, a)| {
        let cat = match k {
            Some(k) if k < 5 => "Low Tags",
            Some(k) if (5..=10).contains(&k) => "Medium Tags",
            _ => "High Tags",
        };
        row(vec![oint(k), avg(a[1], a[0]), avg(a[3], a[2]), V::I(a[4]), V::I(a[5]), V::S(cat)])
    }))
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.Reputation,
// COUNT(DISTINCT P.Id) AS PostCount,
// SUM(COALESCE(P.Score, 0)) AS TotalScore,
// SUM(COALESCE(C.CommentCount, 0)) AS TotalComments
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) C ON P.Id = C.PostId
// WHERE
// U.Reputation > 0
// GROUP BY
// U.Id, U.Reputation
// ),
// PostDetails AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// PT.Name AS PostType,
// COALESCE(V.UpVotes, 0) AS UpVotes,
// COALESCE(V.DownVotes, 0) AS DownVotes,
// P.OwnerUserId
// FROM
// Posts P
// JOIN
// PostTypes PT ON P.PostTypeId = PT.Id
// LEFT JOIN
// (SELECT
// PostId,
// SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Votes
// GROUP BY PostId) V ON P.Id = V.PostId
// )
// SELECT
// U.UserId,
// U.Reputation,
// U.PostCount,
// U.TotalScore,
// U.TotalComments,
// P.PostId,
// P.Title,
// P.CreationDate,
// P.PostType,
// P.UpVotes,
// P.DownVotes
// FROM
// UserStats U
// JOIN
// PostDetails P ON U.UserId = P.OwnerUserId
// ORDER BY
// U.Reputation DESC,
// U.TotalScore DESC,
// P.UpVotes DESC;
fn q10988(db: &'static So) -> String {
    let cp = comments_per_post(db);
    let pv = post_votes(db);
    let uf = owned(db).group_by(&db.post.owner_user).select((&db.post.score).and(&cp)).fold([0i64; 3], |a, (s, c)| [a[0] + 1, a[1] + s, a[2] + c]);
    let mut v = Vec::new();
    owned(db)
        .select(Ident::<Post>::new().and((&pv).opt()).and((&db.post.owner_user).with((&db.user.reputation).gt(0)).select(Ident::<User>::new().and(&uf))))
        .drive(|_, ((p, x), (u, a))| v.push((p, x.unwrap_or([0; 3]), u, a)));
    rows(v.iter().map(|&(p, x, u, a)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep")];
        f.extend(ints(&a));
        f.extend(post_fields(db, p, &["id", "title", "created", "type"]));
        f.extend(ints(&[x[1], x[2]]));
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
// COUNT(DISTINCT c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score
// ), UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(DISTINCT p.Id) AS PostsCreated,
// COUNT(DISTINCT b.Id) AS BadgesCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
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
// us.UserId,
// us.DisplayName AS AuthorName,
// us.Reputation AS AuthorReputation,
// us.PostsCreated AS TotalPosts,
// us.BadgesCount AS TotalBadges
// FROM
// PostStats ps
// JOIN
// Users u ON ps.PostId = u.Id
// JOIN
// UserStats us ON u.Id = us.UserId
// ORDER BY
// ps.CreationDate DESC
// LIMIT 100;
fn q14501(db: &'static So) -> String {
    let uid = uids(db);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    stats_fold(db, db.post.with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cv", &[])
        .and(comments_per_post(db))
        .and(votes_per_post(db))
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and((&dp).opt()).and(&bu)))
        .drive(|p, (((s, c), x), ((u, d), b))| v.push((p, s, c, x, u, d.unwrap_or(0), b)));
    out(v, |&(p, ..)| Reverse(db.post.creation_date.get(p).unwrap()), 100, |&(p, s, c, x, u, d, b)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score"]);
        f.extend(ints(&[c, x, s.up, s.down]));
        f.extend(["uid", "name", "rep"].iter().map(|k| user_col(db, u, k)));
        f.extend(ints(&[d, b]));
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
// COUNT(p.Id) AS PostCount,
// SUM(p.Score) AS TotalScore,
// COUNT(DISTINCT p.Tags) AS UniqueTags
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// ),
// VoteSummary AS (
// SELECT
// v.UserId,
// COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS Upvotes,
// COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS Downvotes
// FROM
// Votes v
// GROUP BY
// v.UserId
// )
// SELECT
// ub.UserId,
// ub.DisplayName,
// COALESCE(ps.PostCount, 0) AS TotalPosts,
// COALESCE(ps.TotalScore, 0) AS TotalScore,
// COALESCE(ps.UniqueTags, 0) AS UniqueTags,
// COALESCE(vs.Upvotes, 0) AS TotalUpvotes,
// COALESCE(vs.Downvotes, 0) AS TotalDownvotes,
// ub.BadgeCount,
// ub.GoldBadges,
// ub.SilverBadges,
// ub.BronzeBadges
// FROM
// UserBadges ub
// LEFT JOIN
// PostStats ps ON ub.UserId = ps.OwnerUserId
// LEFT JOIN
// VoteSummary vs ON ub.UserId = vs.UserId
// ORDER BY
// TotalScore DESC, TotalPosts DESC;
fn q9123(db: &'static So) -> String {
    let ub = ubc(db);
    let ps = owned(db).group_by(&db.post.owner_user).select(&db.post.score).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let dt = owned(db).group_by(&db.post.owner_user).select(&db.post.tags_str).count_distinct();
    let uv = uvotes(db);
    let mut v = Vec::new();
    (&ub).and((&ps).opt()).and((&dt).opt()).and((&uv).opt()).drive(|u, (((b, p), t), x)| v.push((u, b, p.unwrap_or([0; 2]), t.unwrap_or(0), x.unwrap_or([0; 3]))));
    rows(v.iter().map(|&(u, b, p, t, x)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&[p[0], p[1], t, x[1], x[2]]));
        f.extend(ints(&b));
        row(f)
    }))
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// COALESCE(COUNT(v.Id), 0) AS VoteCount,
// COALESCE(COUNT(c.Id), 0) AS CommentCount,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount
// FROM
// Posts p
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount
// ),
// UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(b.Id) AS BadgeCount,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(ps.ViewCount) AS TotalViews
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// PostStats ps ON p.Id = ps.PostId
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
// )
// SELECT
// us.UserId,
// us.DisplayName,
// us.Reputation,
// us.BadgeCount,
// us.PostCount,
// us.TotalViews,
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.ViewCount,
// ps.VoteCount,
// ps.CommentCount,
// ps.UpVoteCount,
// ps.DownVoteCount
// FROM
// UserStats us
// JOIN
// PostStats ps ON us.UserId = ps.PostId
// ORDER BY
// us.Reputation DESC, ps.ViewCount DESC
// LIMIT 100;
fn q10676(db: &'static So) -> String {
    let uid = uids(db);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let wv = db
        .user
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(posts_of(db).select((&db.post.view_count).opt()).opt()))
        .fold([0i64; 3], |a, (b, w)| {
            let w = w.flatten();
            [a[0] + b.is_some() as i64, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0)]
        });
    let mut v = Vec::new();
    stats_fold(db, db.post.with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "vc", &[])
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and((&dp).opt()).and(&wv)))
        .drive(|p, (s, ((u, d), w))| v.push((p, s, u, d.unwrap_or(0), w)));
    out(v, |&(p, _, u, ..)| (rep_desc(db, u), views_desc(db, p)), 100, |&(p, s, u, d, w)| {
        let mut f = ["uid", "name", "rep"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.extend([V::I(w[0]), V::I(d), nullable(w[2], w[1])]);
        f.extend(post_fields(db, p, &["id", "title", "created", "views"]));
        f.extend(ints(&[s.vx, s.cx, s.up, s.down]));
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
// COALESCE(u.Reputation, 0) AS UserReputation,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount,
// p.OwnerUserId
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.Reputation, p.OwnerUserId
// ),
// UserEngagement AS (
// SELECT
// u.Id AS UserId,
// COUNT(DISTINCT ps.PostId) AS PostsCreated,
// SUM(ps.ViewCount) AS TotalViews,
// SUM(ps.Score) AS TotalScore,
// SUM(ps.CommentCount) AS TotalComments,
// SUM(ps.UpvoteCount) AS TotalUpvotes,
// SUM(ps.DownvoteCount) AS TotalDownvotes
// FROM
// Users u
// LEFT JOIN
// PostStats ps ON u.Id = ps.OwnerUserId
// GROUP BY
// u.Id
// )
// SELECT
// ue.UserId,
// ue.PostsCreated,
// ue.TotalViews,
// ue.TotalScore,
// ue.TotalComments,
// ue.TotalUpvotes,
// ue.TotalDownvotes,
// u.Reputation AS UserReputation,
// u.CreationDate AS UserCreationDate
// FROM
// UserEngagement ue
// JOIN
// Users u ON ue.UserId = u.Id
// ORDER BY
// ue.TotalViews DESC;
fn q14605(db: &'static So) -> String {
    let sf = stats_fold(db, owned(db), &db.post.owner_user, "cv", &[]);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&ps).opt()).and((&sf).opt())).drive(|_, ((u, p), s)| v.push((u, pz(p), s)));
    rows(v.iter().map(|&(u, p, s)| {
        let mut f = vec![user_col(db, u, "uid"), V::I(p[0]), pviews(p), nullable(p[3], p[0])];
        f.extend(s.map_or(nulls(3), |s| ints(&[s.cx, s.up, s.down])));
        f.extend([user_col(db, u, "rep"), user_col(db, u, "ucreated")]);
        row(f)
    }))
}

// WITH UserStats AS (
// SELECT
// Users.Id AS UserId,
// Users.DisplayName,
// COUNT(DISTINCT Posts.Id) AS TotalPosts,
// SUM(CASE WHEN Posts.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsCount,
// SUM(CASE WHEN Posts.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersCount,
// SUM(Posts.Score) AS TotalScore,
// SUM(Posts.ViewCount) AS TotalViews,
// AVG(Posts.Score) AS AveragePostScore,
// AVG(Posts.ViewCount) AS AveragePostViewCount
// FROM
// Users
// LEFT JOIN
// Posts ON Users.Id = Posts.OwnerUserId
// GROUP BY
// Users.Id, Users.DisplayName
// ),
// BadgeStats AS (
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
// U.UserId,
// U.DisplayName,
// U.TotalPosts,
// U.QuestionsCount,
// U.AnswersCount,
// U.TotalScore,
// U.TotalViews,
// U.AveragePostScore,
// U.AveragePostViewCount,
// COALESCE(B.TotalBadges, 0) AS TotalBadges,
// COALESCE(B.GoldBadges, 0) AS GoldBadges,
// COALESCE(B.SilverBadges, 0) AS SilverBadges,
// COALESCE(B.BronzeBadges, 0) AS BronzeBadges
// FROM
// UserStats U
// LEFT JOIN
// BadgeStats B ON U.UserId = B.UserId
// ORDER BY
// U.TotalScore DESC, U.TotalPosts DESC;
fn q14916(db: &'static So) -> String {
    let ps = pstat(db, db.post.iq());
    let bc = badge_classes(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&ps).opt()).and((&bc).opt())).drive(|_, ((u, p), b)| v.push((u, pz(p), bz(b))));
    rows(v.iter().map(|&(u, p, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(p[0]), V::I(p[1]), V::I(p[2]), nullable(p[3], p[0]), pviews(p), pscore_avg(p), pviews_avg(p)];
        f.extend(ints(&b));
        row(f)
    }))
}

// WITH UserActivity AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// COUNT(p.Id) AS TotalPosts,
// COUNT(DISTINCT c.Id) AS TotalComments,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes,
// SUM(CASE WHEN v.VoteTypeId IN (8, 9) THEN 1 ELSE 0 END) AS Bounties
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
// ),
// PostMetrics AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// p.AnswerCount,
// COALESCE(u.DisplayName, 'Community') AS OwnerDisplayName,
// pt.Name AS PostType,
// p.OwnerUserId
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// )
// SELECT
// ua.UserId,
// ua.Reputation,
// ua.TotalPosts,
// ua.TotalComments,
// ua.Upvotes,
// ua.Downvotes,
// ua.Bounties,
// pm.PostId,
// pm.Title,
// pm.CreationDate,
// pm.Score,
// pm.ViewCount,
// pm.AnswerCount,
// pm.OwnerDisplayName,
// pm.PostType
// FROM
// UserActivity ua
// LEFT JOIN
// PostMetrics pm ON ua.UserId = pm.OwnerUserId
// ORDER BY
// ua.Reputation DESC, ua.TotalPosts DESC, pm.Score DESC;
fn q12869(db: &'static So) -> String {
    let sf = stats_fold(db, owned(db), &db.post.owner_user, "cv", &[]);
    let dc = udc(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&sf).opt()).and((&dc).opt()).and(posts_of(db).opt())).drive(|_, x| v.push(x));
    let pn = |p: Option<Id<Post>>, c: &str| p.map_or(V::Null, |p| post_fields(db, p, &[c]).remove(0));
    rows(v.iter().map(|&(((u, s), d), p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep")];
        f.extend(match s {
            Some(s) => ints(&[s.rows, d.unwrap_or(0), s.up, s.down, s.by_vt[8] + s.by_vt[9]]),
            None => ints(&[0, 0, 0, 0, 0]),
        });
        f.extend(["id", "title", "created", "score", "views", "answers", "owner", "type"].iter().map(|c| pn(p, c)));
        row(f)
    }))
}

// WITH UserActivity AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// COUNT(DISTINCT c.Id) AS TotalComments,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes
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
// ),
// PostStatistics AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// COALESCE(x.TotalComments, 0) AS CommentCount
// FROM
// Posts p
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS TotalComments
// FROM Comments
// GROUP BY PostId) x ON p.Id = x.PostId
// )
// SELECT
// ua.DisplayName,
// ua.TotalPosts,
// ua.TotalQuestions,
// ua.TotalAnswers,
// ua.TotalComments,
// ua.TotalUpvotes,
// ua.TotalDownvotes,
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.ViewCount,
// ps.Score,
// ps.CommentCount
// FROM
// UserActivity ua
// JOIN
// PostStatistics ps ON ua.UserId = ps.PostId
// ORDER BY
// ua.TotalPosts DESC, ua.TotalQuestions DESC;
fn q13533(db: &'static So) -> String {
    let pid = pids(db);
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "cv", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let dc = udc(db);
    let cp = comments_per_post(db);
    let mut v = Vec::new();
    (&us).and((&dp).opt()).and((&dc).opt()).and((&db.user.origid).select(&pid).select(Ident::<Post>::new().and(&cp))).drive(|u, (((a, d), c), (p, pc))| v.push((u, a, d.unwrap_or(0), c.unwrap_or(0), p, pc)));
    rows(v.iter().map(|&(u, a, d, c, p, pc)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(ints(&[d, a.q, a.a, c, a.up, a.down]));
        f.extend(post_fields(db, p, &["id", "title", "created", "views", "score"]));
        f.push(V::I(pc));
        row(f)
    }))
}

// WITH UserBadges AS (
// SELECT U.Id AS UserId, U.DisplayName, COUNT(B.Id) AS BadgeCount,
// SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM Users U
// LEFT JOIN Badges B ON U.Id = B.UserId
// GROUP BY U.Id, U.DisplayName
// ),
// PostStatistics AS (
// SELECT P.OwnerUserId, COUNT(C.Id) AS CommentCount,
// SUM(P.Score) AS TotalScore,
// COUNT(DISTINCT P.Id) AS PostCount
// FROM Posts P
// LEFT JOIN Comments C ON P.Id = C.PostId
// GROUP BY P.OwnerUserId
// ),
// CombinedData AS (
// SELECT U.Id AS UserId, U.DisplayName,
// COALESCE(UB.BadgeCount, 0) AS BadgeCount,
// COALESCE(UB.GoldBadges, 0) AS GoldBadges,
// COALESCE(UB.SilverBadges, 0) AS SilverBadges,
// COALESCE(UB.BronzeBadges, 0) AS BronzeBadges,
// COALESCE(PS.CommentCount, 0) AS CommentCount,
// COALESCE(PS.TotalScore, 0) AS TotalScore,
// COALESCE(PS.PostCount, 0) AS PostCount
// FROM Users U
// LEFT JOIN UserBadges UB ON U.Id = UB.UserId
// LEFT JOIN PostStatistics PS ON U.Id = PS.OwnerUserId
// )
// SELECT UserId, DisplayName, BadgeCount, GoldBadges, SilverBadges,
// BronzeBadges, CommentCount, TotalScore, PostCount
// FROM CombinedData
// WHERE BadgeCount > 0 AND PostCount > 5
// ORDER BY TotalScore DESC, BadgeCount DESC
// LIMIT 10;
fn q9933(db: &'static So) -> String {
    let ub = ubc(db);
    let pf = owned(db).group_by(&db.post.owner_user).select((&db.post.score).and(comments_of(db).opt())).fold([0i64; 2], |a, (s, c)| [a[0] + c.is_some() as i64, a[1] + s]);
    let np = owned(db).group_by(&db.post.owner_user).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let mut v = Vec::new();
    (&ub).filt(|b: [i64; 4]| b[0] > 0).and(&pf).and((&np).filt(|n| n > 5)).drive(|u, ((b, p), n)| v.push((u, b, [p[0], p[1], n])));
    out(v, |&(_, b, p)| (Reverse(p[1]), Reverse(b[0])), 10, |&(u, b, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&b));
        f.extend(ints(&p));
        f
    })
}

// WITH RecentPosts AS (
// SELECT p.Id, p.Title, p.OwnerUserId, p.CreationDate, p.ViewCount, p.Score
// FROM Posts p
// WHERE p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days'
// ),
// UserReputation AS (
// SELECT u.Id AS UserId, u.Reputation, u.DisplayName
// FROM Users u
// WHERE u.Reputation > 1000
// ),
// PostsWithTags AS (
// SELECT p.Id, p.Title, array_length(string_to_array(p.Tags, '>'), 1) AS TagCount
// FROM Posts p
// WHERE p.Tags IS NOT NULL
// ),
// TopAnswers AS (
// SELECT pa.Id AS PostId, COUNT(v.Id) AS VoteCount
// FROM Posts pa
// JOIN Votes v ON pa.Id = v.PostId
// WHERE pa.PostTypeId = 2
// GROUP BY pa.Id
// HAVING COUNT(v.Id) > 5
// ),
// ClosedPosts AS (
// SELECT ph.PostId, COUNT(ph.Id) AS ClosureCount
// FROM PostHistory ph
// WHERE ph.PostHistoryTypeId = 10
// GROUP BY ph.PostId
// )
// SELECT rp.Title,
// ur.DisplayName AS Owner,
// rp.ViewCount,
// rp.Score,
// pt.TagCount,
// COALESCE(cl.ClosureCount, 0) AS ClosureCount,
// ta.VoteCount AS TopAnswerVoteCount
// FROM RecentPosts rp
// LEFT JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId
// LEFT JOIN PostsWithTags pt ON rp.Id = pt.Id
// LEFT JOIN ClosedPosts cl ON rp.Id = cl.PostId
// LEFT JOIN TopAnswers ta ON rp.Id = ta.PostId
// WHERE (pt.TagCount > 3 OR rp.ViewCount > 100)
// AND (ta.VoteCount IS NOT NULL OR rp.Score > 10)
// AND rp.CreationDate < cast('2024-10-01' as date) - INTERVAL '1 week'
// ORDER BY rp.Score DESC, rp.ViewCount DESC
// LIMIT 100;
fn q152(db: &'static So) -> String {
    let (lo, hi) = (date(2024, 9, 1), date(2024, 9, 24));
    let ht = history_types(db);
    let ta = Ident::<Post>::new().with((&db.post.post_type_id).eq(2)).select(votes_per_post(db)).filt(|n: i64| n > 5);
    let owner = (&db.post.owner_user).with((&db.user.reputation).gt(1000)).select(&db.user.display_name);
    let mut v = Vec::new();
    db.post
        .with((&db.post.creation_date).filt(move |d: i64| d >= lo && d < hi))
        .select(Ident::<Post>::new().and((&db.post.tags_str).map(tag_n).opt()).and(ta.opt()).and(owner.opt()).and((&ht).opt()).and((&db.post.view_count).opt()).and(&db.post.score))
        .filt(|((((((_, t), a), _), _), w), s): ((((((Id<Post>, Option<i64>), Option<i64>), Option<Str>), Option<[i64; 4]>), Option<i64>), i64)| {
            (t.map_or(false, |t| t > 3) || w.map_or(false, |w| w > 100)) && (a.is_some() || s > 10)
        })
        .drive(|_, (x, _)| v.push(x.0));
    out(v, |&((((p, _), _), _), _)| score_views(db, p), 100, |&((((p, t), a), o), h)| {
        let mut f = post_fields(db, p, &["title"]);
        f.push(ostr(o));
        f.extend(post_fields(db, p, &["views", "score"]));
        f.extend([oint(t), V::I(h.map_or(0, |h| h[1])), oint(a)]);
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
// SUM(CASE WHEN P.Score IS NOT NULL THEN P.Score ELSE 0 END) AS TotalScore,
// AVG(P.ViewCount) AS AvgViewCount,
// MAX(P.CreationDate) AS LastPostDate
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
// U.UserId,
// U.DisplayName,
// COALESCE(U.TotalPosts, 0) AS TotalPosts,
// COALESCE(U.TotalQuestions, 0) AS TotalQuestions,
// COALESCE(U.TotalAnswers, 0) AS TotalAnswers,
// COALESCE(U.TotalScore, 0) AS TotalScore,
// COALESCE(U.AvgViewCount, 0) AS AvgViewCount,
// COALESCE(B.TotalBadges, 0) AS TotalBadges,
// COALESCE(B.GoldBadges, 0) AS GoldBadges,
// COALESCE(B.SilverBadges, 0) AS SilverBadges,
// COALESCE(B.BronzeBadges, 0) AS BronzeBadges,
// U.LastPostDate
// FROM UserPostStats U
// LEFT JOIN UserBadgeStats B ON U.UserId = B.UserId
// ORDER BY U.TotalPosts DESC;
fn q11772(db: &'static So) -> String {
    let ps = pstat(db, db.post.iq());
    let bc = badge_classes(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&ps).opt()).and((&bc).opt())).drive(|_, ((u, p), b)| v.push((u, pz(p), bz(b))));
    rows(v.iter().map(|&(u, p, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&[p[0], p[1], p[2], p[3]]));
        f.push(or0(p[5], p[4]));
        f.extend(ints(&b));
        f.push(if p[0] == 0 { V::Null } else { V::T(p[10]) });
        row(f)
    }))
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// COUNT(b.Id) AS BadgeCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// LEFT JOIN
// Votes v ON u.Id = v.UserId
// GROUP BY
// u.Id, u.Reputation
// ),
// PostStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS PostCount,
// SUM(COALESCE(p.Score, 0)) AS TotalScore,
// SUM(COALESCE(p.ViewCount, 0)) AS TotalViews,
// SUM(epoch_us(cast('2024-10-01 12:34:56' as timestamp)) - epoch_us(p.CreationDate))::DOUBLE / COUNT(p.CreationDate) / 1e6 / 60 AS AvgPostAgeInMinutes
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// ),
// Benchmark AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// us.Reputation,
// us.BadgeCount,
// us.TotalUpVotes,
// us.TotalDownVotes,
// ps.PostCount,
// ps.TotalScore,
// ps.TotalViews,
// ps.AvgPostAgeInMinutes
// FROM
// UserStats us
// JOIN
// PostStats ps ON us.UserId = ps.OwnerUserId
// JOIN
// Users u ON us.UserId = u.Id
// )
// SELECT
// UserId,
// DisplayName,
// Reputation,
// BadgeCount,
// TotalUpVotes,
// TotalDownVotes,
// PostCount,
// TotalScore,
// TotalViews,
// AvgPostAgeInMinutes
// FROM
// Benchmark
// ORDER BY
// Reputation DESC, PostCount DESC;
// (rewrites/14369.sql: exact AVG of the epoch)
fn q14369(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let bv = user_bv(db);
    let Post { score, view_count, creation_date, .. } = &db.post;
    let pf = owned(db).group_by(&db.post.owner_user).select(score.and(view_count.opt()).and(creation_date)).fold(([0i64; 3], 0i128), move |(a, e), ((s, w), c)| {
        ([a[0] + 1, a[1] + s, a[2] + w.unwrap_or(0)], e + (t0 - c) as i128)
    });
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and(&bv).and(&pf)).drive(|_, ((u, x), p)| v.push((u, x, p)));
    rows(v.iter().map(|&(u, x, (p, e))| {
        let mut f = ["uid", "name", "rep"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.extend(ints(&[x[0], x[1], x[2], p[0], p[1], p[2]]));
        f.push(V::F(e as f64 / p[0] as f64 / 1e6 / 60.0));
        row(f)
    }))
}

// WITH PostStatistics AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
// COUNT(b.Id) AS BadgeCount,
// p.OwnerUserId
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Badges b ON p.OwnerUserId = b.UserId
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.OwnerUserId
// ),
// UserStatistics AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(COALESCE(p.Score, 0)) AS TotalPostScore,
// SUM(COALESCE(b.Class, 0)) AS TotalBadges
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
// us.UserId,
// us.DisplayName,
// us.PostCount,
// us.TotalPostScore,
// us.TotalBadges
// FROM
// PostStatistics ps
// JOIN
// UserStatistics us ON ps.OwnerUserId = us.UserId
// ORDER BY
// ps.CreationDate DESC
// LIMIT 100;
fn q11104(db: &'static So) -> String {
    let dp = ud(db, UserWhere::All, posts_of(db));
    let pb = user_posts_badges(db);
    let mut v = Vec::new();
    stats_fold(db, owned_since(db, date(2023, 1, 1)), Ident::<Post>::new(), "cvb", &[])
        .and((&db.post.owner_user).select(Ident::<User>::new().and(&dp).and(&pb)))
        .drive(|p, (s, ((u, d), c))| v.push((p, s, u, d, c[1], c[5])));
    out(v, |&(p, ..)| Reverse(db.post.creation_date.get(p).unwrap()), 100, |&(p, s, u, d, x, c)| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend(ints(&[s.cx, s.vx, s.up, s.down]));
        f.extend([user_col(db, u, "uid"), user_col(db, u, "name")]);
        f.extend(ints(&[d, x, c]));
        f
    })
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// COUNT(b.Id) AS BadgeCount,
// COALESCE(SUM(v.BountyAmount), 0) AS TotalBounty
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// LEFT JOIN
// Votes v ON u.Id = v.UserId
// GROUP BY
// u.Id, u.Reputation
// ),
// PostStats AS (
// SELECT
// p.Id AS PostId,
// p.PostTypeId,
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
// GROUP BY
// p.Id, p.PostTypeId, p.OwnerUserId
// ),
// Benchmark AS (
// SELECT
// us.UserId,
// ps.PostId,
// us.Reputation,
// ps.PostTypeId,
// ps.CommentCount,
// ps.UpVotes,
// ps.DownVotes,
// us.BadgeCount,
// us.TotalBounty
// FROM
// UserStats us
// JOIN
// PostStats ps ON us.UserId = ps.OwnerUserId
// )
// SELECT
// UserId,
// AVG(Reputation) AS AverageReputation,
// AVG(BadgeCount) AS AverageBadgeCount,
// AVG(TotalBounty) AS AverageTotalBounty,
// SUM(CommentCount) AS TotalComments,
// SUM(UpVotes) AS TotalUpVotes,
// SUM(DownVotes) AS TotalDownVotes,
// COUNT(DISTINCT PostId) AS CountPosts
// FROM
// Benchmark
// GROUP BY
// UserId;
fn q13888(db: &'static So) -> String {
    let bv = user_bv(db);
    let sf = stats_fold(db, owned(db), &db.post.owner_user, "cv", &[]);
    let np = owned(db).group_by(&db.post.owner_user).select(&db.post.score).fold(0i64, |a, _| a + 1);
    let mut v = Vec::new();
    (&sf).and(&np).and(&bv).drive(|u, ((s, n), x)| v.push((u, s, n, x)));
    rows(v.iter().map(|&(u, s, n, x)| {
        row(vec![user_col(db, u, "uid"), V::F(db.user.reputation.get(u).unwrap() as f64), V::F(x[0] as f64), V::F(x[3] as f64), V::I(s.cx), V::I(s.up), V::I(s.down), V::I(n)])
    }))
}

// WITH UserPostStatistics AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(p.ViewCount) AS TotalViews,
// SUM(p.Score) AS TotalScore,
// COALESCE(SUM(epoch_us(p.LastActivityDate) - epoch_us(p.CreationDate))::DOUBLE / COUNT(p.Id) / 1e6, 0) AS AvgPostLifetimeInSeconds
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName
// ),
// BadgeStatistics AS (
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
// u.UserId,
// u.DisplayName,
// u.TotalPosts,
// u.TotalQuestions,
// u.TotalAnswers,
// u.TotalViews,
// u.TotalScore,
// u.AvgPostLifetimeInSeconds,
// COALESCE(b.TotalBadges, 0) AS TotalBadges,
// COALESCE(b.TotalGoldBadges, 0) AS TotalGoldBadges,
// COALESCE(b.TotalSilverBadges, 0) AS TotalSilverBadges,
// COALESCE(b.TotalBronzeBadges, 0) AS TotalBronzeBadges
// FROM
// UserPostStatistics u
// LEFT JOIN
// BadgeStatistics b ON u.UserId = b.UserId
// ORDER BY
// u.TotalPosts DESC;
// (rewrites/14795.sql: exact AVG of the epoch)
fn q14795(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, creation_date, last_activity_date, .. } = &db.post;
    let pf = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(score).and(view_count.opt()).and(creation_date).and(last_activity_date)).fold(([0i64; 6], 0i128), |(a, e), ((((t, s), w), c), l)| {
        ([a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + w.is_some() as i64, a[4] + w.unwrap_or(0), a[5] + s], e + (l - c) as i128)
    });
    let bc = badge_classes(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&pf).opt()).and((&bc).opt())).drive(|_, ((u, p), b)| v.push((u, p.unwrap_or(([0; 6], 0)), bz(b))));
    rows(v.iter().map(|&(u, (p, e), b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(p[0]), V::I(p[1]), V::I(p[2]), nullable(p[4], p[3]), nullable(p[5], p[0])];
        f.push(V::F(if p[0] == 0 { 0.0 } else { e as f64 / p[0] as f64 / 1e6 }));
        f.extend(ints(&b));
        row(f)
    }))
}

// WITH UserActivity AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(COALESCE(c.Score, 0)) AS TotalCommentScore,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN Comments c ON p.Id = c.PostId
// LEFT JOIN Votes v ON p.Id = v.PostId
// GROUP BY u.Id, u.DisplayName
// ),
// PostSummary AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.ViewCount,
// p.AnswerCount,
// p.CommentCount,
// p.FavoriteCount,
// p.CreationDate,
// u.DisplayName AS OwnerDisplayName,
// pt.Name AS PostType,
// p.OwnerUserId
// FROM Posts p
// JOIN Users u ON p.OwnerUserId = u.Id
// JOIN PostTypes pt ON p.PostTypeId = pt.Id
// )
// SELECT
// ua.DisplayName,
// ua.TotalPosts,
// ua.Questions,
// ua.Answers,
// ua.TotalCommentScore,
// ua.UpVotes,
// ua.DownVotes,
// ps.PostId,
// ps.Title,
// ps.ViewCount,
// ps.AnswerCount,
// ps.CommentCount,
// ps.FavoriteCount,
// ps.CreationDate,
// ps.OwnerDisplayName,
// ps.PostType
// FROM UserActivity ua
// JOIN PostSummary ps ON ua.UserId = ps.OwnerUserId
// ORDER BY ua.TotalPosts DESC, ps.ViewCount DESC;
fn q13759(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "cv", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let mut v = Vec::new();
    owned(db).select(Ident::<Post>::new().and((&db.post.owner_user).select(Ident::<User>::new().and(&us).and(&dp)))).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(p, ((u, a), d))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(ints(&[d, a.q, a.a, a.cscore, a.up, a.down]));
        f.extend(post_fields(db, p, &["id", "title", "views", "answers", "comments", "favorites", "created", "owner", "type"]));
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
// SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositiveScorePosts,
// AVG(p.ViewCount) AS AvgViewCount,
// AVG(p.Score) AS AvgScore
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName
// ),
// PostHistoryStats AS (
// SELECT
// ph.UserId,
// COUNT(ph.Id) AS EditCount,
// COUNT(DISTINCT ph.PostId) AS PostsEdited
// FROM
// PostHistory ph
// WHERE
// ph.PostHistoryTypeId IN (4, 5, 6, 24)
// GROUP BY
// ph.UserId
// ),
// CombinedStats AS (
// SELECT
// ups.UserId,
// ups.DisplayName,
// ups.PostCount,
// ups.QuestionCount,
// ups.AnswerCount,
// ups.PositiveScorePosts,
// ups.AvgViewCount,
// ups.AvgScore,
// COALESCE(phe.EditCount, 0) AS EditCount,
// COALESCE(phe.PostsEdited, 0) AS PostsEdited
// FROM
// UserPostStats ups
// LEFT JOIN
// PostHistoryStats phe ON ups.UserId = phe.UserId
// )
// SELECT
// UserId,
// DisplayName,
// PostCount,
// QuestionCount,
// AnswerCount,
// PositiveScorePosts,
// AvgViewCount,
// AvgScore,
// EditCount,
// PostsEdited
// FROM
// CombinedStats
// ORDER BY
// PostCount DESC;
fn q13165(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let pf = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(score).and(view_count.opt())).fold([0i64; 6], |a, ((t, s), w)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (s > 0) as i64, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0)]
    });
    let ss = owned(db).group_by(&db.post.owner_user).select(score).fold(0i64, |a, s| a + s);
    let hs = || db.post_history.with((&db.post_history.post_history_type_id).filt(|t: i64| matches!(t, 4 | 5 | 6 | 24)));
    let hn = hs().group_by(&db.post_history.user).select(&db.post_history.post).fold(0i64, |a, _| a + 1);
    let hd = hs().group_by(&db.post_history.user).select(&db.post_history.post).count_distinct();
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&pf).opt()).and((&ss).opt()).and((&hn).opt()).and((&hd).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((((u, p), s), h), d)| {
        let p = p.unwrap_or([0; 6]);
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&[p[0], p[1], p[2], p[3]]));
        f.extend([avg(p[5], p[4]), avg(s.unwrap_or(0), p[0]), V::I(h.unwrap_or(0)), V::I(d.unwrap_or(0))]);
        row(f)
    }))
}

// WITH UserReputation AS (
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
// PostStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS PostCount,
// SUM(p.Score) AS TotalScore,
// AVG(p.ViewCount) AS AverageViewCount,
// COUNT(DISTINCT p.Tags) AS UniqueTags
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// ),
// CombinedStats AS (
// SELECT
// u.UserId,
// u.Reputation,
// u.BadgeCount,
// u.GoldBadges,
// u.SilverBadges,
// u.BronzeBadges,
// p.PostCount,
// p.TotalScore,
// p.AverageViewCount,
// p.UniqueTags
// FROM
// UserReputation u
// LEFT JOIN
// PostStats p ON u.UserId = p.OwnerUserId
// )
// SELECT
// cs.UserId,
// cs.Reputation,
// cs.BadgeCount,
// cs.GoldBadges,
// cs.SilverBadges,
// cs.BronzeBadges,
// COALESCE(cs.PostCount, 0) AS PostCount,
// COALESCE(cs.TotalScore, 0) AS TotalScore,
// COALESCE(cs.AverageViewCount, 0) AS AverageViewCount,
// COALESCE(cs.UniqueTags, 0) AS UniqueTags
// FROM
// CombinedStats cs
// ORDER BY
// cs.Reputation DESC, cs.TotalScore DESC;
fn q5393(db: &'static So) -> String {
    let ub = ubc(db);
    let Post { score, view_count, .. } = &db.post;
    let ps = owned(db).group_by(&db.post.owner_user).select(score.and(view_count.opt())).fold([0i64; 4], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0)]);
    let dt = owned(db).group_by(&db.post.owner_user).select(&db.post.tags_str).count_distinct();
    let mut v = Vec::new();
    (&ub).and((&ps).opt()).and((&dt).opt()).drive(|u, ((b, p), t)| v.push((u, b, p.unwrap_or([0; 4]), t.unwrap_or(0))));
    rows(v.iter().map(|&(u, b, p, t)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep")];
        f.extend(ints(&b));
        f.extend([V::I(p[0]), V::I(p[1]), or0(p[3], p[2]), V::I(t)]);
        row(f)
    }))
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(DISTINCT P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN P.PostTypeId = 3 THEN 1 ELSE 0 END) AS TotalWikis,
// SUM(COALESCE(P.ViewCount, 0)) AS TotalViews,
// SUM(COALESCE(P.Score, 0)) AS TotalScore
// FROM Users U
// LEFT JOIN Posts P ON U.Id = P.OwnerUserId
// GROUP BY U.Id, U.DisplayName, U.Reputation
// ),
// TopBadges AS (
// SELECT
// B.UserId,
// B.Name AS BadgeName,
// COUNT(B.Id) AS BadgeCount
// FROM Badges B
// GROUP BY B.UserId, B.Name
// ),
// UserBadgeStats AS (
// SELECT
// U.UserId,
// U.DisplayName,
// U.Reputation,
// U.TotalPosts,
// U.TotalQuestions,
// U.TotalAnswers,
// U.TotalWikis,
// U.TotalViews,
// U.TotalScore,
// COALESCE(SUM(UB.BadgeCount), 0) AS TotalBadges
// FROM UserStats U
// LEFT JOIN TopBadges UB ON U.UserId = UB.UserId
// GROUP BY U.UserId, U.DisplayName, U.Reputation, U.TotalPosts, U.TotalQuestions, U.TotalAnswers, U.TotalWikis, U.TotalViews, U.TotalScore
// )
// SELECT
// UserBadgeStats.*,
// (SELECT COUNT(*) FROM Comments C WHERE C.UserId = UserBadgeStats.UserId) AS TotalComments
// FROM UserBadgeStats
// ORDER BY UserBadgeStats.Reputation DESC, UserBadgeStats.TotalPosts DESC
// LIMIT 10;
fn q6815(db: &'static So) -> String {
    let ps = pstat(db, db.post.iq());
    let bu = badges_per_user(db);
    let cu = comments_per_user(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&ps).opt()).and(&bu).and(&cu)).drive(|_, (((u, p), b), c)| v.push((u, pz(p), b, c)));
    out(v, |&(u, p, ..)| (rep_desc(db, u), Reverse(p[0])), 10, |&(u, p, b, c)| {
        let mut f = ["uid", "name", "rep"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.extend(ints(&[p[0], p[1], p[2], p[8], p[5], p[3], b, c]));
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
// SUM(COALESCE(V.Id, 0)) AS VoteCount
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// U.Id, U.Reputation
// ), BadgeStats AS (
// SELECT
// B.UserId,
// COUNT(B.Id) AS BadgeCount,
// SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadgeCount,
// SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadgeCount,
// SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadgeCount
// FROM
// Badges B
// GROUP BY
// B.UserId
// )
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COALESCE(US.PostCount, 0) AS TotalPosts,
// COALESCE(US.QuestionCount, 0) AS TotalQuestions,
// COALESCE(US.AnswerCount, 0) AS TotalAnswers,
// COALESCE(US.VoteCount, 0) AS TotalVotes,
// COALESCE(BS.BadgeCount, 0) AS TotalBadges,
// COALESCE(BS.GoldBadgeCount, 0) AS TotalGoldBadges,
// COALESCE(BS.SilverBadgeCount, 0) AS TotalSilverBadges,
// COALESCE(BS.BronzeBadgeCount, 0) AS TotalBronzeBadges
// FROM
// Users U
// LEFT JOIN
// UserStats US ON U.Id = US.UserId
// LEFT JOIN
// BadgeStats BS ON U.Id = BS.UserId
// ORDER BY
// U.Reputation DESC;
fn q10280(db: &'static So) -> String {
    let dp = ud(db, UserWhere::All, posts_of(db));
    let uf = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.origid).opt())).opt())
        .fold([0i64; 3], |a, p| match p {
            Some((t, i)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + i.unwrap_or(0)],
            None => a,
        });
    let bc = badge_classes(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&dp).opt()).and(&uf).and((&bc).opt())).drive(|_, (((u, d), p), b)| v.push((u, d.unwrap_or(0), p, bz(b))));
    rows(v.iter().map(|&(u, d, p, b)| {
        let mut f = ["uid", "name", "rep"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.extend(ints(&[d, p[0], p[1], p[2]]));
        f.extend(ints(&b));
        row(f)
    }))
}

// WITH PostStatistics AS (
// SELECT
// p.Id AS PostId,
// pt.Name AS PostType,
// COUNT(c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// MAX(p.CreationDate) AS LastActivityDate,
// SUM(COALESCE(p.ViewCount, 0)) AS TotalViews
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
// u.DisplayName,
// COUNT(b.Id) AS BadgeCount,
// SUM(u.UpVotes) AS UserUpVotes,
// SUM(u.DownVotes) AS UserDownVotes,
// COUNT(DISTINCT p.Id) AS PostsCount
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName
// )
// SELECT
// ps.PostId,
// ps.PostType,
// ps.CommentCount,
// ps.VoteCount,
// ps.UpVotes,
// ps.DownVotes,
// ps.LastActivityDate,
// ps.TotalViews,
// us.UserId,
// us.DisplayName AS OwnerDisplayName,
// us.BadgeCount,
// us.UserUpVotes,
// us.UserDownVotes,
// us.PostsCount
// FROM
// PostStatistics ps
// LEFT JOIN
// UserStatistics us ON ps.PostId = us.UserId
// ORDER BY
// ps.TotalViews DESC
// LIMIT 100;
fn q14100(db: &'static So) -> String {
    let uid = uids(db);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let us = db
        .user
        .group_by(Ident::<User>::new())
        .select((&db.user.up_votes).and(&db.user.down_votes).and(badges_of(db).opt()).and(posts_of(db).opt()))
        .fold([0i64; 3], |a, (((u, d), b), _)| [a[0] + b.is_some() as i64, a[1] + u, a[2] + d]);
    let ps = db
        .post
        .group_by(Ident::<Post>::new())
        .select((&db.post.view_count).opt().and(comments_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 4], |a, ((w, c), t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64, a[3] + w.unwrap_or(0)]);
    let mut v = Vec::new();
    (&ps)
        .and(votes_per_post(db))
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and((&dp).opt()).and(&us)).opt())
        .drive(|p, ((s, x), u)| v.push((p, s, x, u)));
    out(v, |&(_, s, _, _)| Reverse(s[3]), 100, |&(p, s, x, u)| {
        let mut f = post_fields(db, p, &["id", "type"]);
        f.extend(ints(&[s[0], x, s[1], s[2]]));
        f.extend(post_fields(db, p, &["created"]));
        f.push(V::I(s[3]));
        f.extend(match u {
            Some(((u, d), a)) => vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(d.unwrap_or(0))],
            None => nulls(6),
        });
        f
    })
}

// WITH UserPostMetrics AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS Questions,
// COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS Answers,
// SUM(p.ViewCount) AS TotalViews,
// SUM(p.Score) AS TotalScore,
// SUM(p.AnswerCount) AS TotalAnswers,
// SUM(p.CommentCount) AS TotalComments,
// AVG(COALESCE(p.Score, 0)) AS AverageScore
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName
// ),
// PostMetrics AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.ViewCount,
// p.Score,
// p.CommentCount,
// p.AnswerCount,
// p.CreationDate,
// (SELECT COUNT(*) FROM Comments c WHERE c.PostId = p.Id) AS TotalComments,
// p.OwnerUserId  -- Include OwnerUserId for the JOIN
// FROM
// Posts p
// )
// SELECT
// upm.UserId,
// upm.DisplayName,
// upm.TotalPosts,
// upm.Questions,
// upm.Answers,
// upm.TotalViews,
// upm.TotalScore,
// upm.TotalAnswers,
// upm.TotalComments,
// upm.AverageScore,
// pm.PostId,
// pm.Title,
// pm.ViewCount,
// pm.Score,
// pm.CommentCount,
// pm.AnswerCount,
// pm.CreationDate,
// pm.TotalComments
// FROM
// UserPostMetrics upm
// LEFT JOIN
// PostMetrics pm ON upm.UserId = pm.OwnerUserId
// ORDER BY
// upm.TotalPosts DESC, upm.TotalScore DESC;
fn q12890(db: &'static So) -> String {
    let ps = pstat(db, db.post.iq());
    let pc = pcc(db);
    let cp = comments_per_post(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&ps).opt()).and((&pc).opt()).and(posts_of(db).select(Ident::<Post>::new().and(&cp)).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(((u, p), c), x)| {
        let p = pz(p);
        let c = c.unwrap_or([0; 3]);
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(p[0]), V::I(p[1]), V::I(p[2]), pviews(p), nullable(p[3], p[0]), nullable(c[2], c[1]), nullable(c[0], p[0]), or0(p[3], p[0])];
        f.extend(match x {
            Some((q, n)) => {
                let mut g = post_fields(db, q, &["id", "title", "views", "score", "comments", "answers", "created"]);
                g.push(V::I(n));
                g
            }
            None => nulls(8),
        });
        row(f)
    }))
}

// WITH PostStats AS (
// SELECT
// P.Id AS PostId,
// P.Score,
// P.ViewCount,
// P.AnswerCount,
// P.CommentCount,
// P.FavoriteCount,
// COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVotes,
// COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVotes,
// P.CreationDate,
// U.Reputation AS OwnerReputation
// FROM
// Posts P
// LEFT JOIN Votes V ON P.Id = V.PostId
// LEFT JOIN Users U ON P.OwnerUserId = U.Id
// WHERE
// P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY
// P.Id, P.Score, P.ViewCount, P.AnswerCount, P.CommentCount, P.FavoriteCount, P.CreationDate, U.Reputation
// ),
// PostHistoryStats AS (
// SELECT
// PH.PostId,
// COUNT(*) AS EditCount,
// COUNT(CASE WHEN PH.PostHistoryTypeId IN (10, 11) THEN 1 END) AS CloseCount,
// COUNT(CASE WHEN PH.PostHistoryTypeId IN (12, 13) THEN 1 END) AS DeletionCount
// FROM
// PostHistory PH
// GROUP BY
// PH.PostId
// )
// SELECT
// PS.PostId,
// PS.Score,
// PS.ViewCount,
// PS.AnswerCount,
// PS.CommentCount,
// PS.FavoriteCount,
// PS.UpVotes,
// PS.DownVotes,
// PS.CreationDate,
// PS.OwnerReputation,
// COALESCE(PHS.EditCount, 0) AS EditCount,
// COALESCE(PHS.CloseCount, 0) AS CloseCount,
// COALESCE(PHS.DeletionCount, 0) AS DeletionCount
// FROM
// PostStats PS
// LEFT JOIN PostHistoryStats PHS ON PS.PostId = PHS.PostId
// ORDER BY
// PS.Score DESC,
// PS.ViewCount DESC;
fn q13457(db: &'static So) -> String {
    let hf = db.post_history.group_by(&db.post_history.post).select(&db.post_history.post_history_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + matches!(t, 10 | 11) as i64, a[2] + matches!(t, 12 | 13) as i64]);
    let mut v = Vec::new();
    stats_fold(db, since(db, year_ago()), Ident::<Post>::new(), "v", &[]).and((&hf).opt()).drive(|p, (s, h)| v.push((p, s, h.unwrap_or([0; 3]))));
    rows(v.iter().map(|&(p, s, h)| {
        let mut f = post_fields(db, p, &["id", "score", "views", "answers", "comments", "favorites"]);
        f.extend(ints(&[s.up, s.down]));
        f.extend(post_fields(db, p, &["created", "rep"]));
        f.extend(ints(&h));
        row(f)
    }))
}

// WITH UserStatistics AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(DISTINCT P.Id) AS PostCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN P.Score ELSE 0 END) AS TotalQuestionScore,
// SUM(CASE WHEN P.PostTypeId = 2 THEN P.Score ELSE 0 END) AS TotalAnswerScore,
// AVG(COALESCE(CAST(P.Score AS FLOAT), 0)) AS AveragePostScore
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
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
// PostHistoryStats AS (
// SELECT
// PH.UserId,
// COUNT(*) AS EditCount
// FROM
// PostHistory PH
// WHERE
// PH.PostHistoryTypeId IN (4, 5, 6)
// GROUP BY
// PH.UserId
// )
// SELECT
// US.UserId,
// US.DisplayName,
// US.Reputation,
// COALESCE(BC.BadgeCount, 0) AS BadgeCount,
// US.PostCount,
// US.QuestionCount,
// US.AnswerCount,
// US.TotalQuestionScore,
// US.TotalAnswerScore,
// US.AveragePostScore,
// COALESCE(PH.EditCount, 0) AS EditCount
// FROM
// UserStatistics US
// LEFT JOIN
// BadgeCounts BC ON US.UserId = BC.UserId
// LEFT JOIN
// PostHistoryStats PH ON US.UserId = PH.UserId
// ORDER BY
// US.Reputation DESC;
fn q14355(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let pf = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(score)).fold([0i64; 6], |a, (t, s)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + if t == 1 { s } else { 0 }, a[4] + if t == 2 { s } else { 0 }, a[5] + s]
    });
    let bu = badges_per_user(db);
    let he = db.post_history.with((&db.post_history.post_history_type_id).filt(|t: i64| matches!(t, 4 | 5 | 6))).group_by(&db.post_history.user).select(&db.post_history.post).fold(0i64, |a, _| a + 1);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and(&bu).and((&pf).opt()).and((&he).opt())).drive(|_, (((u, b), p), h)| v.push((u, b, p.unwrap_or([0; 6]), h.unwrap_or(0))));
    rows(v.iter().map(|&(u, b, p, h)| {
        let mut f = ["uid", "name", "rep"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.extend(ints(&[b, p[0], p[1], p[2], p[3], p[4]]));
        f.extend([V::F(if p[0] == 0 { 0.0 } else { p[5] as f64 / p[0] as f64 }), V::I(h)]);
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
// COALESCE(AC.AcceptedAnswerId, 0) AS HasAcceptedAnswer,
// COUNT(C.ID) AS CommentCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Posts P
// LEFT JOIN
// Posts AC ON P.Id = AC.AcceptedAnswerId
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// WHERE
// P.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year')
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.Score, P.ViewCount, AC.AcceptedAnswerId
// ),
// UserStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(B.Id) AS BadgeCount
// FROM
// Users U
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// GROUP BY
// U.Id, U.DisplayName, U.Reputation
// )
// SELECT
// PS.PostId,
// PS.Title,
// PS.CreationDate,
// PS.Score,
// PS.ViewCount,
// PS.HasAcceptedAnswer,
// PS.CommentCount,
// PS.UpVotes,
// PS.DownVotes,
// US.DisplayName AS AuthorDisplayName,
// US.Reputation AS AuthorReputation,
// US.BadgeCount AS AuthorBadgeCount
// FROM
// PostStats PS
// JOIN
// Users U ON PS.HasAcceptedAnswer = U.Id
// JOIN
// UserStats US ON U.Id = US.UserId
// ORDER BY
// PS.Score DESC, PS.ViewCount DESC;
fn q14880(db: &'static So) -> String {
    let uid = uids(db);
    let bu = badges_per_user(db);
    let accepted_by: HashIdx<Id<Post>, Id<Post>> = (&db.post.accepted_answer).inv().collect();
    let ps = since(db, year_ago())
        .group_by(Ident::<Post>::new())
        .select((&accepted_by).opt().and(comments_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 4], |a, ((k, c), t)| [a[0] + k.is_some() as i64, a[1] + c.is_some() as i64, a[2] + (t == Some(2)) as i64, a[3] + (t == Some(3)) as i64]);
    let has = (&db.post.origid).and((&accepted_by).opt()).map(|(o, k): (i64, Option<Id<Post>>)| if k.is_some() { o } else { 0 });
    let mut v = Vec::new();
    (&ps).and(has.select(&uid).select(Ident::<User>::new().and(&bu))).drive(|p, (a, (u, b))| v.push((p, a, u, b)));
    rows(v.iter().map(|&(p, a, u, b)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.push(V::I(if a[0] > 0 { db.post.origid.get(p).unwrap() } else { 0 }));
        f.extend(ints(&[a[1], a[2], a[3]]));
        f.extend([user_col(db, u, "name"), user_col(db, u, "rep"), V::I(b)]);
        row(f)
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("6139", q6139),
    ("12540", q12540),
    ("13252", q13252),
    ("8324", q8324),
    ("10285", q10285),
    ("11158", q11158),
    ("12386", q12386),
    ("10042", q10042),
    ("10540", q10540),
    ("12677", q12677),
    ("13658", q13658),
    ("14234", q14234),
    ("13478", q13478),
    ("10465", q10465),
    ("6683", q6683),
    ("12745", q12745),
    ("8020", q8020),
    ("14040", q14040),
    ("14258", q14258),
    ("6300", q6300),
    ("10814", q10814),
    ("12730", q12730),
    ("13586", q13586),
    ("13311", q13311),
    ("8397", q8397),
    ("14397", q14397),
    ("5191", q5191),
    ("13747", q13747),
    ("14594", q14594),
    ("925", q925),
    ("13540", q13540),
    ("14282", q14282),
    ("12284", q12284),
    ("8936", q8936),
    ("9110", q9110),
    ("6113", q6113),
    ("14629", q14629),
    ("28876", q28876),
    ("10988", q10988),
    ("14501", q14501),
    ("9123", q9123),
    ("10676", q10676),
    ("14605", q14605),
    ("14916", q14916),
    ("12869", q12869),
    ("13533", q13533),
    ("9933", q9933),
    ("152", q152),
    ("11772", q11772),
    ("14369", q14369),
    ("11104", q11104),
    ("13888", q13888),
    ("14795", q14795),
    ("13759", q13759),
    ("13165", q13165),
    ("5393", q5393),
    ("6815", q6815),
    ("10280", q10280),
    ("14100", q14100),
    ("12890", q12890),
    ("13457", q13457),
    ("14355", q14355),
    ("14880", q14880),
];
