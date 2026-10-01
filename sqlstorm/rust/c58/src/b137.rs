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

/// Per user over `Users LEFT JOIN Posts LEFT JOIN Badges` (the product): [rows,
/// gold rows, silver rows, bronze rows].
fn user_pb_classes<Q: Drive<D = Id<User>, R = Id<User>>>(db: &'static So, users: Q) -> Fold<Id<User>, [i64; 4]> {
    users
        .group_by(Ident::<User>::new())
        .select(posts_of(db).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 4], |a, (_, c)| [a[0] + 1, a[1] + (c == Some(1)) as i64, a[2] + (c == Some(2)) as i64, a[3] + (c == Some(3)) as i64])
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

// --- batch 137 --------------------------------------------------------------

// WITH UserReputation AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(b.Id) AS BadgeCount,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM Users u
// LEFT JOIN Badges b ON u.Id = b.UserId
// GROUP BY u.Id, u.DisplayName, u.Reputation
// ),
// PostStatistics AS (
// SELECT
// p.OwnerUserId,
// COUNT(*) AS TotalPosts,
// COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS QuestionCount,
// COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS AnswerCount,
// COUNT(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 END) AS AcceptedAnswers,
// SUM(p.ViewCount) AS TotalViews,
// AVG(p.Score) AS AverageScore
// FROM Posts p
// GROUP BY p.OwnerUserId
// ),
// ReputationEnhancements AS (
// SELECT
// ur.UserId,
// ur.DisplayName,
// ur.Reputation + COALESCE(ps.TotalPosts * 10, 0) + COALESCE(ps.AcceptedAnswers * 20, 0) AS EnhancedReputation
// FROM UserReputation ur
// LEFT JOIN PostStatistics ps ON ur.UserId = ps.OwnerUserId
// )
// SELECT
// ur.DisplayName,
// ur.Reputation,
// ur.BadgeCount,
// ur.GoldBadges,
// ur.SilverBadges,
// ur.BronzeBadges,
// ps.TotalPosts,
// ps.QuestionCount,
// ps.AcceptedAnswers,
// rs.EnhancedReputation
// FROM UserReputation ur
// LEFT JOIN PostStatistics ps ON ur.UserId = ps.OwnerUserId
// JOIN ReputationEnhancements rs ON ur.UserId = rs.UserId
// ORDER BY rs.EnhancedReputation DESC, ur.Reputation DESC;
fn q26725(db: &'static So) -> String {
    let ub = ubc(db);
    let Post { post_type_id, accepted_answer_id, .. } = &db.post;
    let ps = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(accepted_answer_id.opt())).fold([0i64; 3], |a, (t, acc)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + acc.is_some() as i64]);
    let mut v = Vec::new();
    (&ub).and((&ps).opt()).drive(|u, (b, p)| v.push((u, b, p)));
    rows(v.iter().map(|&(u, b, p)| {
        let r = db.user.reputation.get(u).unwrap();
        let mut f = vec![user_col(db, u, "name"), V::I(r)];
        f.extend(ints(&b));
        f.extend(p.map_or(nulls(3), |p| ints(&p)));
        f.push(V::I(r + p.map_or(0, |p| p[0] * 10 + p[2] * 20)));
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
// p.AnswerCount,
// p.CommentCount,
// COALESCE(p.AcceptedAnswerId, -1) AS AcceptedAnswerId,
// u.Reputation AS OwnerReputation,
// u.DisplayName AS OwnerDisplayName,
// COUNT(c.Id) AS TotalComments
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// WHERE
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 month'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AnswerCount, p.CommentCount,
// p.AcceptedAnswerId, u.Reputation, u.DisplayName
// ),
// UserEngagement AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(v.Id) AS TotalVotes,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Users u
// LEFT JOIN
// Votes v ON u.Id = v.UserId
// WHERE
// u.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 month'
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
// ps.OwnerDisplayName,
// ps.OwnerReputation,
// ue.TotalVotes,
// ue.UpVotes,
// ue.DownVotes,
// ps.TotalComments
// FROM
// PostStats ps
// LEFT JOIN
// UserEngagement ue ON ps.OwnerDisplayName = ue.DisplayName
// ORDER BY
// ps.CreationDate DESC;
fn q11836(db: &'static So) -> String {
    let uv = uvotes(db);
    let names: HashIdx<Str, Id<User>> = user_base(db, UserWhere::CreatedGe(month_ago())).select(&db.user.display_name).inv().collect();
    let cp = comments_per_post(db);
    let mut v = Vec::new();
    since(db, month_ago())
        .select(Ident::<Post>::new().and(&cp).and((&db.post.owner_user).select(&db.user.display_name).select(&names).select((&uv).opt()).opt()))
        .drive(|_, x| v.push(x));
    rows(v.iter().map(|&((p, c), x)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "answers", "comments", "owner", "rep"]);
        f.extend(match x {
            Some(x) => ints(&x.unwrap_or([0; 3])),
            None => nulls(3),
        });
        f.push(V::I(c));
        row(f)
    }))
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositiveScoredPosts,
// SUM(CASE WHEN p.ViewCount > 1000 THEN 1 ELSE 0 END) AS PostsWithHighViews
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// GROUP BY u.Id, u.DisplayName
// ),
// BadgeCounts AS (
// SELECT
// b.UserId,
// COUNT(*) AS BadgeCount
// FROM Badges b
// GROUP BY b.UserId
// ),
// PostHistoryCounts AS (
// SELECT
// ph.UserId,
// COUNT(*) AS HistoryCount,
// MAX(ph.CreationDate) AS LastEditDate
// FROM PostHistory ph
// GROUP BY ph.UserId
// ),
// CombinedStats AS (
// SELECT
// us.UserId,
// us.DisplayName,
// us.TotalPosts,
// us.TotalAnswers,
// us.TotalQuestions,
// us.PositiveScoredPosts,
// us.PostsWithHighViews,
// COALESCE(bc.BadgeCount, 0) AS BadgeCount,
// COALESCE(phc.HistoryCount, 0) AS HistoryCount,
// phc.LastEditDate
// FROM UserStats us
// LEFT JOIN BadgeCounts bc ON us.UserId = bc.UserId
// LEFT JOIN PostHistoryCounts phc ON us.UserId = phc.UserId
// )
// SELECT
// UserId,
// DisplayName,
// TotalPosts,
// TotalAnswers,
// TotalQuestions,
// PositiveScoredPosts,
// PostsWithHighViews,
// BadgeCount,
// HistoryCount,
// LastEditDate
// FROM CombinedStats
// WHERE TotalPosts > 10
// ORDER BY TotalPosts DESC, PositiveScoredPosts DESC
// LIMIT 50;
fn q8626(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let uf = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(score).and(view_count.opt())).fold([0i64; 5], |a, ((t, s), w)| {
        [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 1) as i64, a[3] + (s > 0) as i64, a[4] + (w.unwrap_or(0) > 1000) as i64]
    });
    let bu = badges_per_user(db);
    let ph = db.post_history.group_by(&db.post_history.user).select(&db.post_history.creation_date).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let mut v = Vec::new();
    (&uf).filt(|a: [i64; 5]| a[0] > 10).and(&bu).and((&ph).opt()).drive(|u, ((a, b), h)| v.push((u, a, b, h)));
    out(v, |&(_, a, ..)| (Reverse(a[0]), Reverse(a[3])), 50, |&(u, a, b, h)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a));
        f.extend([V::I(b), V::I(h.map_or(0, |h| h.0)), ots(h.map(|h| h.1))]);
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
// PostStatistics AS (
// SELECT
// p.OwnerUserId,
// COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS QuestionsCount,
// COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS AnswersCount,
// SUM(p.Score) AS TotalScore,
// SUM(p.ViewCount) AS TotalViews
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
// ub.DisplayName,
// ub.BadgeCount,
// ub.GoldBadges,
// ub.SilverBadges,
// ub.BronzeBadges,
// ps.QuestionsCount,
// ps.AnswersCount,
// ps.TotalScore,
// ps.TotalViews
// FROM
// UserBadgeCounts ub
// LEFT JOIN
// PostStatistics ps ON ub.UserId = ps.OwnerUserId
// )
// SELECT
// DisplayName,
// BadgeCount,
// GoldBadges,
// SilverBadges,
// BronzeBadges,
// COALESCE(QuestionsCount, 0) AS QuestionsCount,
// COALESCE(AnswersCount, 0) AS AnswersCount,
// COALESCE(TotalScore, 0) AS TotalScore,
// COALESCE(TotalViews, 0) AS TotalViews
// FROM
// CombinedStats
// ORDER BY
// BadgeCount DESC, TotalScore DESC
// LIMIT 10;
fn q6263(db: &'static So) -> String {
    let ub = ubc(db);
    let ps = pstat(db, since(db, year_ago()));
    let mut v = Vec::new();
    (&ub).and((&ps).opt()).drive(|u, (b, p)| v.push((u, b, pz(p))));
    out(v, |&(_, b, p)| (Reverse(b[0]), Reverse(p[3])), 10, |&(u, b, p)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(ints(&b));
        f.extend(ints(&[p[1], p[2], p[3], p[5]]));
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
// COUNT(DISTINCT p.Id) AS PostCount,
// COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS QuestionCount,
// COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS AnswerCount,
// SUM(p.Score) AS TotalScore
// FROM Posts p
// GROUP BY p.OwnerUserId
// ),
// CombinedStats AS (
// SELECT
// u.DisplayName,
// u.Reputation,
// COALESCE(ub.BadgeCount, 0) AS TotalBadges,
// COALESCE(ub.GoldCount, 0) AS TotalGold,
// COALESCE(ub.SilverCount, 0) AS TotalSilver,
// COALESCE(ub.BronzeCount, 0) AS TotalBronze,
// COALESCE(ps.PostCount, 0) AS TotalPosts,
// COALESCE(ps.QuestionCount, 0) AS TotalQuestions,
// COALESCE(ps.AnswerCount, 0) AS TotalAnswers,
// COALESCE(ps.TotalScore, 0) AS TotalScore
// FROM Users u
// LEFT JOIN UserBadgeCounts ub ON u.Id = ub.UserId
// LEFT JOIN PostStats ps ON u.Id = ps.OwnerUserId
// )
// SELECT
// DisplayName,
// Reputation,
// TotalBadges,
// TotalGold,
// TotalSilver,
// TotalBronze,
// TotalPosts,
// TotalQuestions,
// TotalAnswers,
// TotalScore
// FROM CombinedStats
// ORDER BY Reputation DESC, TotalScore DESC
// LIMIT 10;
fn q7562(db: &'static So) -> String {
    let ub = ubc(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    (&ub).and((&ps).opt()).drive(|u, (b, p)| v.push((u, b, pz(p))));
    out(v, |&(u, _, p)| (rep_desc(db, u), Reverse(p[3])), 10, |&(u, b, p)| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(ints(&b));
        f.extend(ints(&[p[0], p[1], p[2], p[3]]));
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
// COUNT(P.Id) AS PostCount,
// SUM(COALESCE(P.AnswerCount, 0)) AS TotalAnswers,
// AVG(P.Score) AS AverageScore,
// MAX(P.CreationDate) AS LastPostDate
// FROM
// Posts P
// GROUP BY
// P.OwnerUserId
// ),
// CombinedStats AS (
// SELECT
// U.DisplayName,
// COALESCE(UB.BadgeCount, 0) AS BadgeCount,
// COALESCE(PS.PostCount, 0) AS PostCount,
// COALESCE(PS.TotalAnswers, 0) AS TotalAnswers,
// COALESCE(PS.AverageScore, 0) AS AverageScore,
// PS.LastPostDate
// FROM
// Users U
// LEFT JOIN
// UserBadges UB ON U.Id = UB.UserId
// LEFT JOIN
// PostStats PS ON U.Id = PS.OwnerUserId
// )
// SELECT
// CS.DisplayName,
// CS.BadgeCount,
// CS.PostCount,
// CS.TotalAnswers,
// CS.AverageScore,
// CS.LastPostDate,
// CASE
// WHEN CS.AverageScore IS NULL THEN 'No Posts Yet'
// WHEN CS.AverageScore > 10 THEN 'High Performer'
// ELSE 'Needs Improvement'
// END AS PerformanceTier
// FROM
// CombinedStats CS
// WHERE
// CS.BadgeCount > 0
// OR CS.PostCount > 0
// ORDER BY
// CS.AverageScore DESC,
// CS.BadgeCount DESC;
fn q3290(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let Post { answer_count, score, creation_date, .. } = &db.post;
    let ps = owned(db).group_by(&db.post.owner_user).select(answer_count.opt().and(score).and(creation_date)).fold([0, 0, 0, i64::MIN], |a: [i64; 4], ((an, s), c)| {
        [a[0] + 1, a[1] + an.unwrap_or(0), a[2] + s, a[3].max(c)]
    });
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and(&bu).and((&ps).opt())).filt(|((_, b), p): ((Id<User>, i64), Option<[i64; 4]>)| b > 0 || p.is_some()).drive(|_, ((u, b), p)| v.push((u, b, p)));
    rows(v.iter().map(|&(u, b, p)| {
        let pp = p.unwrap_or([0; 4]);
        let avg = if pp[0] == 0 { 0.0 } else { pp[2] as f64 / pp[0] as f64 };
        vec![user_col(db, u, "name"), V::I(b), V::I(pp[0]), V::I(pp[1]), V::F(avg), p.map_or(V::Null, |p| V::T(p[3])), V::S(if avg > 10.0 { "High Performer" } else { "Needs Improvement" })]
    }).map(row))
}

// WITH UserStats AS (
// SELECT
// Users.Id AS UserId,
// Users.DisplayName,
// Users.Reputation,
// COUNT(DISTINCT Posts.Id) AS PostCount,
// SUM(CASE WHEN Posts.Score > 0 THEN 1 ELSE 0 END) AS PositivePostCount,
// SUM(CASE WHEN Posts.Score < 0 THEN 1 ELSE 0 END) AS NegativePostCount,
// SUM(CASE WHEN Posts.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN Posts.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN Votes.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN Votes.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Users
// LEFT JOIN
// Posts ON Users.Id = Posts.OwnerUserId
// LEFT JOIN
// Votes ON Posts.Id = Votes.PostId
// GROUP BY
// Users.Id, Users.DisplayName, Users.Reputation
// ),
// BadgeCounts AS (
// SELECT
// UserId,
// COUNT(CASE WHEN Class = 1 THEN 1 END) AS GoldBadges,
// COUNT(CASE WHEN Class = 2 THEN 1 END) AS SilverBadges,
// COUNT(CASE WHEN Class = 3 THEN 1 END) AS BronzeBadges
// FROM
// Badges
// GROUP BY
// UserId
// )
// SELECT
// us.UserId,
// us.DisplayName,
// us.Reputation,
// us.PostCount,
// us.PositivePostCount,
// us.NegativePostCount,
// us.QuestionCount,
// us.AnswerCount,
// COALESCE(bc.GoldBadges, 0) AS GoldBadges,
// COALESCE(bc.SilverBadges, 0) AS SilverBadges,
// COALESCE(bc.BronzeBadges, 0) AS BronzeBadges,
// us.UpVotes,
// us.DownVotes
// FROM
// UserStats us
// LEFT JOIN
// BadgeCounts bc ON us.UserId = bc.UserId
// ORDER BY
// us.Reputation DESC, us.PostCount DESC
// LIMIT 100;
fn q5134(db: &'static So) -> String {
    let nq = pnq(db);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let bc = badge_classes(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&nq).opt()).and((&dp).opt()).and((&bc).opt())).drive(|_, (((u, a), d), b)| v.push((u, a.unwrap_or([0; 6]), d.unwrap_or(0), bz(b))));
    out(v, |&(u, _, d, _)| (rep_desc(db, u), Reverse(d)), 100, |&(u, a, d, b)| {
        let mut f = ["uid", "name", "rep"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.extend(ints(&[d, a[0], a[1], a[2], a[3], b[1], b[2], b[3], a[4], a[5]]));
        f
    })
}

// WITH UserStatistics AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(DISTINCT P.Id) AS PostCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM Users U
// LEFT JOIN Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN Votes V ON P.Id = V.PostId
// GROUP BY U.Id, U.DisplayName, U.Reputation
// ),
// BadgeStatistics AS (
// SELECT
// B.UserId,
// COUNT(CASE WHEN B.Class = 1 THEN 1 END) AS GoldBadges,
// COUNT(CASE WHEN B.Class = 2 THEN 1 END) AS SilverBadges,
// COUNT(CASE WHEN B.Class = 3 THEN 1 END) AS BronzeBadges
// FROM Badges B
// GROUP BY B.UserId
// ),
// FinalStatistics AS (
// SELECT
// US.UserId,
// US.DisplayName,
// US.Reputation,
// US.PostCount,
// US.QuestionCount,
// US.AnswerCount,
// US.UpVotes,
// US.DownVotes,
// COALESCE(BS.GoldBadges, 0) AS GoldBadges,
// COALESCE(BS.SilverBadges, 0) AS SilverBadges,
// COALESCE(BS.BronzeBadges, 0) AS BronzeBadges
// FROM UserStatistics US
// LEFT JOIN BadgeStatistics BS ON US.UserId = BS.UserId
// )
// SELECT
// F.DisplayName,
// F.Reputation,
// F.PostCount,
// F.QuestionCount,
// F.AnswerCount,
// F.UpVotes,
// F.DownVotes,
// F.GoldBadges,
// F.SilverBadges,
// F.BronzeBadges
// FROM FinalStatistics F
// WHERE F.Reputation > 1000
// ORDER BY F.Reputation DESC, F.PostCount DESC;
fn q7475(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::RepGt(1000), "v", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&us).and((&dp).opt()).and((&bc).opt()).drive(|u, ((a, d), b)| v.push((u, a, d.unwrap_or(0), bz(b))));
    rows(v.iter().map(|&(u, a, d, b)| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(ints(&[d, a.q, a.a, a.up, a.down, b[1], b[2], b[3]]));
        row(f)
    }))
}

// WITH UserStatistics AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// AVG(CASE WHEN P.PostTypeId = 1 THEN P.Score END) AS AvgQuestionScore,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN P.PostTypeId = 1 AND P.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedQuestions
// FROM Users U
// LEFT JOIN Posts P ON U.Id = P.OwnerUserId
// GROUP BY U.Id, U.DisplayName, U.Reputation
// ),
// BadgesByUser AS (
// SELECT
// B.UserId,
// COUNT(*) AS BadgeCount,
// SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM Badges B
// GROUP BY B.UserId
// ),
// PostCommentStats AS (
// SELECT
// PC.UserId,
// COUNT(PC.Id) AS TotalComments,
// AVG(PC.Score) AS AvgCommentScore
// FROM Comments PC
// GROUP BY PC.UserId
// )
// SELECT
// US.DisplayName,
// US.Reputation,
// COALESCE(BU.BadgeCount, 0) AS BadgeCount,
// COALESCE(BU.GoldBadges, 0) AS GoldBadges,
// COALESCE(BU.SilverBadges, 0) AS SilverBadges,
// COALESCE(BU.BronzeBadges, 0) AS BronzeBadges,
// COALESCE(PCS.TotalComments, 0) AS TotalComments,
// COALESCE(PCS.AvgCommentScore, 0) AS AvgCommentScore,
// US.AvgQuestionScore,
// US.TotalAnswers,
// US.AcceptedQuestions
// FROM UserStatistics US
// LEFT JOIN BadgesByUser BU ON US.UserId = BU.UserId
// LEFT JOIN PostCommentStats PCS ON US.UserId = PCS.UserId
// WHERE US.Reputation > 1000
// ORDER BY US.Reputation DESC, US.DisplayName ASC;
fn q1246(db: &'static So) -> String {
    let Post { post_type_id, score, accepted_answer_id, .. } = &db.post;
    let uf = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(score).and(accepted_answer_id.opt())).fold([0i64; 4], |a, ((t, s), acc)| {
        [a[0] + (t == 1) as i64, a[1] + if t == 1 { s } else { 0 }, a[2] + (t == 2) as i64, a[3] + (t == 1 && acc.is_some()) as i64]
    });
    let bc = badge_classes(db);
    let cs = db.comment.group_by(&db.comment.user).select(&db.comment.score).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and((&uf).opt()).and((&bc).opt()).and((&cs).opt())).drive(|_, (((u, a), b), c)| v.push((u, a.unwrap_or([0; 4]), bz(b), c.unwrap_or([0; 2]))));
    rows(v.iter().map(|&(u, a, b, c)| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(ints(&b));
        f.extend([V::I(c[0]), or0(c[1], c[0]), avg(a[1], a[0]), V::I(a[2]), V::I(a[3])]);
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
// SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositiveScoreCount,
// SUM(CASE WHEN p.Score < 0 THEN 1 ELSE 0 END) AS NegativeScoreCount,
// AVG(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS AvgUpVotes,
// AVG(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS AvgDownVotes
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
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
// us.UserId,
// us.DisplayName,
// us.Reputation,
// us.PostCount,
// us.QuestionCount,
// us.AnswerCount,
// us.PositiveScoreCount,
// us.NegativeScoreCount,
// COALESCE(bc.BadgeCount, 0) AS BadgeCount,
// COALESCE(bc.GoldBadges, 0) AS GoldBadges,
// COALESCE(bc.SilverBadges, 0) AS SilverBadges,
// COALESCE(bc.BronzeBadges, 0) AS BronzeBadges
// FROM
// UserStats us
// LEFT JOIN
// BadgeCounts bc ON us.UserId = bc.UserId
// ORDER BY
// us.Reputation DESC,
// us.PostCount DESC
// LIMIT 50;
fn q7673(db: &'static So) -> String {
    let nq = pnq(db);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let bc = badge_classes(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&nq).opt()).and((&dp).opt()).and((&bc).opt())).drive(|_, (((u, a), d), b)| v.push((u, a.unwrap_or([0; 6]), d.unwrap_or(0), bz(b))));
    out(v, |&(u, _, d, _)| (rep_desc(db, u), Reverse(d)), 50, |&(u, a, d, b)| {
        let mut f = ["uid", "name", "rep"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.extend(ints(&[d, a[2], a[3], a[0], a[1]]));
        f.extend(ints(&b));
        f
    })
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.Reputation,
// U.Views,
// U.UpVotes,
// U.DownVotes,
// COUNT(DISTINCT P.Id) AS PostCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN C.Id IS NOT NULL THEN 1 ELSE 0 END) AS CommentCount,
// SUM(V.BountyAmount) AS TotalBounties
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId AND V.VoteTypeId IN (8, 9)
// GROUP BY
// U.Id, U.Reputation, U.Views, U.UpVotes, U.DownVotes
// ),
// PostStats AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.Score,
// P.ViewCount,
// COUNT(C.Id) AS CommentCount,
// COUNT(V.Id) AS VoteCount
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.Score, P.ViewCount
// )
// SELECT
// US.UserId,
// US.Reputation,
// US.Views,
// US.UpVotes,
// US.DownVotes,
// US.PostCount,
// US.QuestionCount,
// US.AnswerCount,
// US.CommentCount AS UserCommentCount,
// US.TotalBounties,
// PS.PostId,
// PS.Title,
// PS.CreationDate,
// PS.Score,
// PS.ViewCount,
// PS.CommentCount AS PostCommentCount,
// PS.VoteCount AS PostVoteCount
// FROM
// UserStats US
// JOIN
// PostStats PS ON US.UserId = PS.PostId
// ORDER BY
// US.Reputation DESC, PS.Score DESC;
fn q12379(db: &'static So) -> String {
    let pid = pids(db);
    let us = user_stats_fold_v(db, Ident::<User>::new(), UserWhere::All, "cv", any_post, &[8, 9]);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let sf = stats_fold(db, db.post.iq(), Ident::<Post>::new(), "cv", &[]);
    let mut v = Vec::new();
    (&us).and((&dp).opt()).and((&db.user.origid).select(&pid).select(Ident::<Post>::new().and(&sf))).drive(|u, ((a, d), (p, s))| v.push((u, a, d.unwrap_or(0), p, s)));
    rows(v.iter().map(|&(u, a, d, p, s)| {
        let mut f = ["uid", "rep", "uviews", "uup", "udown"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.extend(ints(&[d, a.q, a.a, a.cx]));
        f.push(ustat_field(&a, "bounty_sum"));
        f.extend(post_fields(db, p, &["id", "title", "created", "score", "views"]));
        f.extend(ints(&[s.cx, s.vx]));
        row(f)
    }))
}

// WITH UserStatistics AS (
// SELECT
// Users.Id AS UserId,
// Users.DisplayName,
// COUNT(DISTINCT Posts.Id) AS TotalPosts,
// SUM(CASE WHEN Posts.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN Posts.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(Posts.Score) AS TotalScore,
// AVG(Posts.ViewCount) AS AvgViews
// FROM
// Users
// LEFT JOIN
// Posts ON Users.Id = Posts.OwnerUserId
// GROUP BY
// Users.Id, Users.DisplayName
// ),
// BadgeStatistics AS (
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
// FinalStatistics AS (
// SELECT
// Us.UserId,
// Us.DisplayName,
// Us.TotalPosts,
// Us.TotalQuestions,
// Us.TotalAnswers,
// Us.TotalScore,
// Us.AvgViews,
// COALESCE(Bs.TotalBadges, 0) AS TotalBadges,
// COALESCE(Bs.GoldBadges, 0) AS GoldBadges,
// COALESCE(Bs.SilverBadges, 0) AS SilverBadges,
// COALESCE(Bs.BronzeBadges, 0) AS BronzeBadges
// FROM
// UserStatistics Us
// LEFT JOIN
// BadgeStatistics Bs ON Us.UserId = Bs.UserId
// )
// SELECT
// UserId,
// DisplayName,
// TotalPosts,
// TotalQuestions,
// TotalAnswers,
// TotalScore,
// AvgViews,
// TotalBadges,
// GoldBadges,
// SilverBadges,
// BronzeBadges
// FROM
// FinalStatistics
// ORDER BY
// TotalScore DESC, TotalPosts DESC;
fn q14644(db: &'static So) -> String {
    let ps = pstat(db, db.post.iq());
    let bc = badge_classes(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&ps).opt()).and((&bc).opt())).drive(|_, ((u, p), b)| v.push((u, pz(p), bz(b))));
    rows(v.iter().map(|&(u, p, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(p[0]), V::I(p[1]), V::I(p[2]), nullable(p[3], p[0]), pviews_avg(p)];
        f.extend(ints(&b));
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
// ub.BadgeCount,
// ub.GoldBadges,
// ub.SilverBadges,
// ub.BronzeBadges,
// ps.PostCount,
// ps.QuestionCount,
// ps.AnswerCount,
// ps.TotalScore,
// COALESCE(ub.BadgeCount, 0) + COALESCE(ps.TotalScore, 0) AS PerformanceScore
// FROM
// Users u
// LEFT JOIN
// UserBadges ub ON u.Id = ub.UserId
// LEFT JOIN
// PostStats ps ON u.Id = ps.OwnerUserId
// )
// SELECT
// UserId,
// DisplayName,
// BadgeCount,
// GoldBadges,
// SilverBadges,
// BronzeBadges,
// PostCount,
// QuestionCount,
// AnswerCount,
// TotalScore,
// PerformanceScore
// FROM
// UserPerformance
// WHERE
// PerformanceScore > 10
// ORDER BY
// PerformanceScore DESC
// LIMIT 10;
fn q8761(db: &'static So) -> String {
    let ub = ubc(db);
    let ps = pstat(db, since(db, year_ago()));
    let mut v = Vec::new();
    (&ub).and((&ps).opt()).filt(|(b, p): ([i64; 4], Option<[i64; 13]>)| b[0] + p.map_or(0, |p| p[3]) > 10).drive(|u, (b, p)| v.push((u, b, p, b[0] + p.map_or(0, |p| p[3]))));
    out(v, |&(.., s)| Reverse(s), 10, |&(u, b, p, s)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&b));
        f.extend(p.map_or(nulls(4), |p| ints(&[p[0], p[1], p[2], p[3]])));
        f.push(V::I(s));
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
// PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// COUNT(c.Id) AS TotalComments,
// CASE
// WHEN p.AcceptedAnswerId IS NOT NULL THEN 1
// ELSE 0
// END AS HasAcceptedAnswer,
// p.OwnerUserId  -- Added to allow joining in final select
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score, p.AcceptedAnswerId, p.OwnerUserId
// )
// SELECT
// u.DisplayName,
// us.TotalPosts,
// us.TotalQuestions,
// us.TotalAnswers,
// us.TotalUpvotes,
// us.TotalDownvotes,
// ps.PostId,
// ps.Title AS PostTitle,
// ps.CreationDate AS PostCreationDate,
// ps.ViewCount AS PostViewCount,
// ps.Score AS PostScore,
// ps.TotalComments,
// ps.HasAcceptedAnswer
// FROM
// UserStats us
// JOIN
// Users u ON us.UserId = u.Id
// JOIN
// PostStats ps ON u.Id = ps.OwnerUserId
// ORDER BY
// us.TotalPosts DESC, ps.ViewCount DESC;
fn q12854(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let cp = comments_per_post(db);
    let mut v = Vec::new();
    owned(db).select(Ident::<Post>::new().and(&cp).and((&db.post.owner_user).select(Ident::<User>::new().and(&us).and(&dp)))).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((p, c), ((u, a), d))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(ints(&[d, a.q, a.a, a.up, a.down]));
        f.extend(post_fields(db, p, &["id", "title", "created", "views", "score"]));
        f.extend(ints(&[c, db.post.accepted_answer_id.get(p).is_some() as i64]));
        row(f)
    }))
}

// WITH UserVoteStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(V.Id) AS TotalVotes,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes,
// SUM(V.BountyAmount) AS TotalBounty
// FROM
// Users U
// LEFT JOIN
// Votes V ON U.Id = V.UserId
// GROUP BY
// U.Id, U.DisplayName
// ),
// PostStats AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// COUNT(C.Id) AS CommentCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpvoteCount,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownvoteCount,
// MAX(PH.CreationDate) AS LastEditDate
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// LEFT JOIN
// PostHistory PH ON P.Id = PH.PostId
// GROUP BY
// P.Id, P.Title
// )
// SELECT
// U.DisplayName,
// U.TotalVotes,
// U.Upvotes,
// U.Downvotes,
// U.TotalBounty,
// P.Title,
// P.CommentCount,
// P.UpvoteCount,
// P.DownvoteCount,
// P.LastEditDate,
// CASE
// WHEN P.LastEditDate < TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' THEN 'Stale'
// WHEN P.LastEditDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' THEN 'Recent'
// END AS PostRecency
// FROM
// UserVoteStats U
// JOIN
// PostStats P ON P.UpvoteCount > 10 AND U.UserId IN (
// SELECT DISTINCT UserId
// FROM Votes
// WHERE PostId = P.PostId AND VoteTypeId = 2
// )
// WHERE
// U.TotalVotes > 0
// ORDER BY
// U.TotalVotes DESC, P.CommentCount DESC;
fn q4590(db: &'static So) -> String {
    let vf = db.vote.group_by(&db.vote.user).select((&db.vote.vote_type_id).and((&db.vote.bounty_amount).opt())).fold([0i64; 5], |a, (t, b)| {
        [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64, a[3] + b.is_some() as i64, a[4] + b.unwrap_or(0)]
    });
    let ups: MatSet<(Id<Post>, Id<User>)> = db.vote.with((&db.vote.vote_type_id).eq(2)).select((&db.vote.post).and(&db.vote.user)).collect();
    let voters: HashIdx<Id<Post>, (Id<Post>, Id<User>)> = (&ups).map(|(p, _)| p).inv().collect();
    let y = year_ago();
    let mut v = Vec::new();
    stats_fold(db, db.post.iq(), Ident::<Post>::new(), "cvh", &[])
        .filt(|s: Stats| s.up > 10)
        .and((&voters).map(|(_, u): (Id<Post>, Id<User>)| u).select(Ident::<User>::new().and(&vf)))
        .drive(|p, (s, (u, x))| v.push((p, s, u, x)));
    rows(v.iter().map(|&(p, s, u, x)| {
        let mut f = vec![user_col(db, u, "name"), V::I(x[0]), V::I(x[1]), V::I(x[2]), nullable(x[4], x[3])];
        f.extend(post_fields(db, p, &["title"]));
        f.extend(ints(&[s.cx, s.up, s.down]));
        let h = if s.hx > 0 { Some(s.hmax) } else { None };
        f.push(ots(h));
        f.push(h.map_or(V::Null, |h| V::S(if h < y { "Stale" } else { "Recent" })));
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
// ), UserPosts AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(p.Score) AS TotalScore
// FROM
// Posts p
// WHERE
// p.OwnerUserId IS NOT NULL
// GROUP BY
// p.OwnerUserId
// ), UserActivity AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// ub.BadgeCount,
// ub.GoldBadges,
// ub.SilverBadges,
// ub.BronzeBadges,
// up.PostCount,
// up.Questions,
// up.Answers,
// up.TotalScore
// FROM
// Users u
// LEFT JOIN
// UserBadges ub ON u.Id = ub.UserId
// LEFT JOIN
// UserPosts up ON u.Id = up.OwnerUserId
// )
// SELECT
// UserId,
// DisplayName,
// COALESCE(BadgeCount, 0) AS BadgeCount,
// COALESCE(GoldBadges, 0) AS GoldBadges,
// COALESCE(SilverBadges, 0) AS SilverBadges,
// COALESCE(BronzeBadges, 0) AS BronzeBadges,
// COALESCE(PostCount, 0) AS PostCount,
// COALESCE(Questions, 0) AS Questions,
// COALESCE(Answers, 0) AS Answers,
// COALESCE(TotalScore, 0) AS TotalScore
// FROM
// UserActivity
// ORDER BY
// TotalScore DESC,
// BadgeCount DESC
// LIMIT 10;
fn q9903(db: &'static So) -> String {
    let ub = ubc(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    (&ub).and((&ps).opt()).drive(|u, (b, p)| v.push((u, b, pz(p))));
    out(v, |&(_, b, p)| (Reverse(p[3]), Reverse(b[0])), 10, |&(u, b, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&b));
        f.extend(ints(&[p[0], p[1], p[2], p[3]]));
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
// HighScorePosts AS (
// SELECT
// P.OwnerUserId,
// P.Title,
// COUNT(C.Id) AS CommentCount,
// P.Score
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// WHERE
// P.PostTypeId = 1 AND P.Score > 0
// GROUP BY
// P.OwnerUserId, P.Title, P.Score
// ),
// UserPostMetrics AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COALESCE(SUM(HSP.Score), 0) AS TotalPostScore,
// COALESCE(SUM(HP.BadgeCount), 0) AS TotalBadges,
// COUNT(DISTINCT P.Id) AS TotalPosts,
// COUNT(DISTINCT C.Id) AS TotalComments
// FROM
// Users U
// LEFT JOIN
// HighScorePosts HSP ON U.Id = HSP.OwnerUserId
// LEFT JOIN
// UserBadges HP ON U.Id = HP.UserId
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// GROUP BY
// U.Id, U.DisplayName
// )
// SELECT
// UPM.UserId,
// UPM.DisplayName,
// UPM.TotalPostScore,
// UPM.TotalBadges,
// UPM.TotalPosts,
// UPM.TotalComments,
// (UPM.TotalPosts * 100 / NULLIF(UPM.TotalBadges, 0)) AS PostsPerBadge
// FROM
// UserPostMetrics UPM
// ORDER BY
// UPM.TotalPostScore DESC,
// UPM.TotalBadges DESC;
fn q25002(db: &'static So) -> String {
    let hs: MatSet<(Id<User>, (Option<Str>, i64))> = questions_only(db)
        .with((&db.post.score).gt(0))
        .with(&db.post.owner_user)
        .select((&db.post.owner_user).and((&db.post.title).opt().and(&db.post.score)))
        .collect();
    let hsp: HashIdx<Id<User>, (Id<User>, (Option<Str>, i64))> = (&hs).map(|(u, _)| u).inv().collect();
    let bu = badges_per_user(db);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let dc = udc(db);
    let pm = db
        .user
        .group_by(Ident::<User>::new())
        .select((&hsp).opt().and(&bu).and(posts_of(db).select(comments_of(db).opt()).opt()))
        .fold([0i64; 2], |a, ((h, b), _)| [a[0] + h.map_or(0, |(_, (_, s))| s), a[1] + b]);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and(&pm).and((&dp).opt()).and((&dc).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(((u, h), d), c)| {
        let d = d.unwrap_or(0);
        let tb = h[1];
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&[h[0], tb, d, c.unwrap_or(0)]));
        f.push(if tb == 0 { V::Null } else { V::F((d * 100) as f64 / tb as f64) });
        row(f)
    }))
}

// WITH UserBadges AS (
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
// PostStats AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS PostCount,
// SUM(P.ViewCount) AS TotalViews,
// AVG(P.Score) AS AverageScore
// FROM
// Posts P
// WHERE
// P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY
// P.OwnerUserId
// ),
// UserPostStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COALESCE(UB.PostCount, 0) AS PostCount,
// COALESCE(UB.TotalViews, 0) AS TotalViews,
// COALESCE(UB.AverageScore, 0) AS AverageScore,
// COALESCE(B.GoldBadges, 0) AS GoldBadges,
// COALESCE(B.SilverBadges, 0) AS SilverBadges,
// COALESCE(B.BronzeBadges, 0) AS BronzeBadges
// FROM
// Users U
// LEFT JOIN
// PostStats UB ON U.Id = UB.OwnerUserId
// LEFT JOIN
// UserBadges B ON U.Id = B.UserId
// )
// SELECT
// UserId,
// DisplayName,
// PostCount,
// TotalViews,
// AverageScore,
// GoldBadges,
// SilverBadges,
// BronzeBadges,
// CASE
// WHEN PostCount > 50 THEN 'Highly Active'
// WHEN PostCount BETWEEN 21 AND 50 THEN 'Moderately Active'
// ELSE 'Less Active'
// END AS ActivityLevel
// FROM
// UserPostStats
// WHERE
// TotalViews > 1000
// ORDER BY
// TotalViews DESC;
fn q3922(db: &'static So) -> String {
    let bc = badge_classes(db);
    let ps = pstat(db, since(db, year_ago()));
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&ps).filt(|p: [i64; 13]| p[5] > 1000)).and((&bc).opt())).drive(|_, ((u, p), b)| v.push((u, p, bz(b))));
    rows(v.iter().map(|&(u, p, b)| {
        let lvl = if p[0] > 50 { "Highly Active" } else if (21..=50).contains(&p[0]) { "Moderately Active" } else { "Less Active" };
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(p[0]), V::I(p[5]), or0(p[3], p[0])];
        f.extend(ints(&[b[1], b[2], b[3]]));
        f.push(V::S(lvl));
        row(f)
    }))
}

// WITH UserActivity AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(DISTINCT P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END) AS UpvotedPosts,
// SUM(CASE WHEN P.Score < 0 THEN 1 ELSE 0 END) AS DownvotedPosts
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
// COUNT(CASE WHEN B.Class = 1 THEN 1 END) AS GoldBadges,
// COUNT(CASE WHEN B.Class = 2 THEN 1 END) AS SilverBadges,
// COUNT(CASE WHEN B.Class = 3 THEN 1 END) AS BronzeBadges
// FROM
// Badges B
// GROUP BY
// B.UserId
// ),
// UserSummary AS (
// SELECT
// UA.UserId,
// UA.DisplayName,
// UA.TotalPosts,
// UA.TotalQuestions,
// UA.TotalAnswers,
// UA.UpvotedPosts,
// UA.DownvotedPosts,
// COALESCE(BS.GoldBadges, 0) AS GoldBadges,
// COALESCE(BS.SilverBadges, 0) AS SilverBadges,
// COALESCE(BS.BronzeBadges, 0) AS BronzeBadges
// FROM
// UserActivity UA
// LEFT JOIN
// BadgeStats BS ON UA.UserId = BS.UserId
// )
// SELECT
// US.DisplayName,
// US.TotalPosts,
// US.TotalQuestions,
// US.TotalAnswers,
// US.UpvotedPosts,
// US.DownvotedPosts,
// US.GoldBadges + US.SilverBadges + US.BronzeBadges AS TotalBadges,
// US.GoldBadges,
// US.SilverBadges,
// US.BronzeBadges
// FROM
// UserSummary US
// ORDER BY
// US.TotalPosts DESC, US.UpvotedPosts DESC
// LIMIT 10;
fn q5166(db: &'static So) -> String {
    let Post { post_type_id, score, .. } = &db.post;
    let uf = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(score)).fold([0i64; 5], |a, (t, s)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (s > 0) as i64, a[4] + (s < 0) as i64]
    });
    let bc = badge_classes(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&uf).opt()).and((&bc).opt())).drive(|_, ((u, a), b)| v.push((u, a.unwrap_or([0; 5]), bz(b))));
    out(v, |&(_, a, _)| (Reverse(a[0]), Reverse(a[3])), 10, |&(u, a, b)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(ints(&a));
        f.extend(ints(&[b[1] + b[2] + b[3], b[1], b[2], b[3]]));
        f
    })
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
// COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS Questions,
// COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS Answers,
// SUM(p.Score) AS TotalScore,
// SUM(p.ViewCount) AS TotalViewCount
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// ),
// TopUsers AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COALESCE(pb.TotalBadges, 0) AS TotalBadges,
// COALESCE(ps.TotalPosts, 0) AS TotalPosts,
// COALESCE(ps.Questions, 0) AS Questions,
// COALESCE(ps.Answers, 0) AS Answers,
// COALESCE(ps.TotalScore, 0) AS TotalScore,
// COALESCE(ps.TotalViewCount, 0) AS TotalViewCount
// FROM
// Users u
// LEFT JOIN
// UserBadges pb ON u.Id = pb.UserId
// LEFT JOIN
// PostStats ps ON u.Id = ps.OwnerUserId
// ORDER BY
// TotalScore DESC, TotalBadges DESC
// LIMIT 10
// )
// SELECT
// tu.UserId,
// tu.DisplayName,
// tu.TotalBadges,
// tu.TotalPosts,
// tu.Questions,
// tu.Answers,
// tu.TotalScore,
// tu.TotalViewCount
// FROM
// TopUsers tu
// JOIN
// (SELECT DISTINCT OwnerUserId FROM Posts) AS p ON tu.UserId = p.OwnerUserId
// ORDER BY
// tu.TotalScore DESC;
fn q6987(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let ps = pstat(db, db.post.iq());
    let top: MatSet<Id<User>> = whole(&db.user.id)
        .select(Ident::<User>::new().and((&ps).opt()).and(&bu))
        .window(row_number, |((_, p), b): ((Id<User>, Option<[i64; 13]>), i64)| (pz(p)[3], b), desc)
        .filt(|(_, n)| n <= 10)
        .map(|(((u, _), _), _)| u)
        .collect();
    let mut v = Vec::new();
    (&top).select(Ident::<User>::new().and(&ps).and(&bu)).drive(|_, ((u, p), b)| v.push((u, p, b)));
    rows(v.iter().map(|&(u, p, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&[b, p[0], p[1], p[2], p[3], p[5]]));
        row(f)
    }))
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(DISTINCT P.Id) AS PostCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN C.Id IS NOT NULL THEN 1 ELSE 0 END) AS CommentCount
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// WHERE
// U.Reputation > 0
// GROUP BY
// U.Id, U.DisplayName, U.Reputation
// ),
// PostDetails AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.Score,
// P.ViewCount,
// PT.Name AS PostType,
// PH.CreationDate AS HistoryDate,
// P.OwnerUserId
// FROM
// Posts P
// JOIN
// PostTypes PT ON P.PostTypeId = PT.Id
// LEFT JOIN
// PostHistory PH ON P.Id = PH.PostId
// )
// SELECT
// US.UserId,
// US.DisplayName,
// US.Reputation,
// US.PostCount,
// US.QuestionCount,
// US.AnswerCount,
// US.CommentCount,
// PD.PostId,
// PD.Title AS PostTitle,
// PD.CreationDate AS PostCreationDate,
// PD.Score AS PostScore,
// PD.ViewCount AS PostViewCount,
// PD.PostType,
// COUNT(PD.HistoryDate) AS HistoryCount
// FROM
// UserStats US
// LEFT JOIN
// PostDetails PD ON US.UserId = PD.OwnerUserId
// GROUP BY
// US.UserId, US.DisplayName, US.Reputation, US.PostCount,
// US.QuestionCount, US.AnswerCount, US.CommentCount,
// PD.PostId, PD.Title, PD.CreationDate, PD.Score,
// PD.ViewCount, PD.PostType
// ORDER BY
// US.Reputation DESC, US.PostCount DESC;
fn q10665(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::RepGt(0), "c", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let hp = history_per_post(db);
    let mut v = Vec::new();
    (&us).and((&dp).opt()).and(posts_of(db).select(Ident::<Post>::new().and(&hp)).opt()).drive(|u, ((a, d), p)| v.push((u, a, d.unwrap_or(0), p)));
    rows(v.iter().map(|&(u, a, d, p)| {
        let mut f = ["uid", "name", "rep"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.extend(ints(&[d, a.q, a.a, a.cx]));
        f.extend(match p {
            Some((p, h)) => {
                let mut g = post_fields(db, p, &["id", "title", "created", "score", "views", "type"]);
                g.push(V::I(h));
                g
            }
            None => {
                let mut g = nulls(6);
                g.push(V::I(0));
                g
            }
        });
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
// SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END) AS PositiveScoreCount,
// AVG(P.Score) AS AvgScore,
// MAX(P.CreationDate) AS LatestPostDate
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
// COALESCE(US.PostCount, 0) AS PostCount,
// COALESCE(US.QuestionCount, 0) AS QuestionCount,
// COALESCE(US.AnswerCount, 0) AS AnswerCount,
// COALESCE(US.PositiveScoreCount, 0) AS PositiveScoreCount,
// COALESCE(US.AvgScore, 0) AS AvgScore,
// COALESCE(US.LatestPostDate, '1970-01-01') AS LatestPostDate,
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
// ORDER BY
// U.Reputation DESC;
fn q12320(db: &'static So) -> String {
    let ps = pstat(db, db.post.iq());
    let pos = owned(db).group_by(&db.post.owner_user).select(&db.post.score).fold(0i64, |a, s| a + (s > 0) as i64);
    let bc = badge_classes(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&ps).opt()).and((&pos).opt()).and((&bc).opt())).drive(|_, (((u, p), o), b)| v.push((u, pz(p), o.unwrap_or(0), bz(b))));
    rows(v.iter().map(|&(u, p, o, b)| {
        let mut f = ["uid", "name", "rep"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.extend(ints(&[p[0], p[1], p[2], o]));
        f.extend([or0(p[3], p[0]), V::T(if p[0] == 0 { 0 } else { p[10] })]);
        f.extend(ints(&b));
        row(f)
    }))
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(DISTINCT P.Id) AS PostCount,
// SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts,
// SUM(CASE WHEN P.Score < 0 THEN 1 ELSE 0 END) AS NegativePosts,
// SUM(CASE WHEN P.ViewCount > 100 THEN 1 ELSE 0 END) AS PopularPosts,
// AVG(P.ViewCount) AS AverageViews
// FROM Users U
// LEFT JOIN Posts P ON U.Id = P.OwnerUserId
// GROUP BY U.Id, U.DisplayName, U.Reputation
// ),
// BadgeStats AS (
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
// US.PositivePosts,
// US.NegativePosts,
// US.PopularPosts,
// US.AverageViews,
// COALESCE(BS.BadgeCount, 0) AS BadgeCount,
// COALESCE(BS.GoldBadges, 0) AS GoldBadges,
// COALESCE(BS.SilverBadges, 0) AS SilverBadges,
// COALESCE(BS.BronzeBadges, 0) AS BronzeBadges
// FROM UserStats US
// LEFT JOIN BadgeStats BS ON US.UserId = BS.UserId
// )
// SELECT
// UserId,
// DisplayName,
// Reputation,
// PostCount,
// PositivePosts,
// NegativePosts,
// PopularPosts,
// AverageViews,
// BadgeCount,
// GoldBadges,
// SilverBadges,
// BronzeBadges
// FROM CombinedStats
// WHERE Reputation > 1000
// ORDER BY Reputation DESC, PostCount DESC
// LIMIT 50;
fn q5379(db: &'static So) -> String {
    let Post { score, view_count, .. } = &db.post;
    let uf = owned(db).group_by(&db.post.owner_user).select(score.and(view_count.opt())).fold([0i64; 6], |a, (s, w)| {
        [a[0] + 1, a[1] + (s > 0) as i64, a[2] + (s < 0) as i64, a[3] + (w.unwrap_or(0) > 100) as i64, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0)]
    });
    let bc = badge_classes(db);
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and((&uf).opt()).and((&bc).opt())).drive(|_, ((u, a), b)| v.push((u, a.unwrap_or([0; 6]), bz(b))));
    out(v, |&(u, a, _)| (rep_desc(db, u), Reverse(a[0])), 50, |&(u, a, b)| {
        let mut f = ["uid", "name", "rep"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.extend(ints(&[a[0], a[1], a[2], a[3]]));
        f.push(avg(a[5], a[4]));
        f.extend(ints(&b));
        f
    })
}

// WITH UserVoteSummary AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpvotes,
// COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownvotes
// FROM Users U
// LEFT JOIN Votes V ON U.Id = V.UserId
// GROUP BY U.Id, U.DisplayName
// ),
// PostActivitySummary AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.ViewCount,
// COALESCE(PS.TotalAnswers, 0) AS TotalAnswers,
// COALESCE(PS.TotalComments, 0) AS TotalComments
// FROM Posts P
// LEFT JOIN (
// SELECT
// A.ParentId,
// COUNT(*) AS TotalAnswers,
// COUNT(DISTINCT C.Id) AS TotalComments
// FROM Posts A
// LEFT JOIN Comments C ON A.Id = C.PostId
// WHERE A.PostTypeId = 2
// GROUP BY A.ParentId
// ) PS ON P.Id = PS.ParentId
// WHERE P.PostTypeId = 1
// ),
// ClosedPostSummary AS (
// SELECT
// PH.PostId,
// COUNT(PH.Id) AS CloseCount,
// MAX(PH.CreationDate) AS LastClosedDate
// FROM PostHistory PH
// WHERE PH.PostHistoryTypeId = 10
// GROUP BY PH.PostId
// )
// SELECT
// UPS.DisplayName AS UserName,
// UPS.TotalUpvotes,
// UPS.TotalDownvotes,
// PAS.PostId,
// PAS.Title AS PostTitle,
// PAS.ViewCount,
// PAS.TotalAnswers,
// PAS.TotalComments,
// CPS.CloseCount,
// CPS.LastClosedDate
// FROM UserVoteSummary UPS
// INNER JOIN Posts P ON UPS.UserId = P.OwnerUserId
// INNER JOIN PostActivitySummary PAS ON P.Id = PAS.PostId
// LEFT JOIN ClosedPostSummary CPS ON PAS.PostId = CPS.PostId
// WHERE UPS.TotalUpvotes > UPS.TotalDownvotes
// ORDER BY PAS.ViewCount DESC, UPS.TotalUpvotes DESC;
fn q3067(db: &'static So) -> String {
    let uv = uvotes(db);
    let pa = db.post.with((&db.post.post_type_id).eq(2)).group_by(&db.post.parent).select(comments_of(db).opt()).fold([0i64; 2], |a, c| [a[0] + 1, a[1] + c.is_some() as i64]);
    let cl = db.post_history.with((&db.post_history.post_history_type_id).eq(10)).group_by(&db.post_history.post).select(&db.post_history.creation_date).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let mut v = Vec::new();
    questions_only(db)
        .select(Ident::<Post>::new().and((&db.post.owner_user).select(Ident::<User>::new().and((&uv).filt(|x: [i64; 3]| x[1] > x[2])))).and((&pa).opt()).and((&cl).opt()))
        .drive(|_, x| v.push(x));
    rows(v.iter().map(|&(((p, (u, x)), a), c)| {
        let a = a.unwrap_or([0; 2]);
        let mut f = vec![user_col(db, u, "name"), V::I(x[1]), V::I(x[2])];
        f.extend(post_fields(db, p, &["id", "title", "views"]));
        f.extend(ints(&a));
        f.extend([oint(c.map(|c| c.0)), ots(c.map(|c| c.1))]);
        row(f)
    }))
}

// WITH UserStatistics AS (
// SELECT
// u.Id AS UserId,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(COALESCE(p.Score, 0)) AS TotalScore,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// AVG(COALESCE(p.ViewCount, 0)) AS AvgViewCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id
// ), BadgeStatistics AS (
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
// ), AggregateData AS (
// SELECT
// us.UserId,
// us.PostCount,
// us.TotalScore,
// us.QuestionCount,
// us.AnswerCount,
// us.AvgViewCount,
// COALESCE(bs.BadgeCount, 0) AS BadgeCount,
// COALESCE(bs.GoldBadgeCount, 0) AS GoldBadgeCount,
// COALESCE(bs.SilverBadgeCount, 0) AS SilverBadgeCount,
// COALESCE(bs.BronzeBadgeCount, 0) AS BronzeBadgeCount
// FROM
// UserStatistics us
// LEFT JOIN
// BadgeStatistics bs ON us.UserId = bs.UserId
// )
// SELECT
// a.UserId,
// a.PostCount,
// a.TotalScore,
// a.QuestionCount,
// a.AnswerCount,
// a.AvgViewCount,
// a.BadgeCount,
// a.GoldBadgeCount,
// a.SilverBadgeCount,
// a.BronzeBadgeCount
// FROM
// AggregateData a
// WHERE
// a.TotalScore > 100
// ORDER BY
// a.TotalScore DESC,
// a.PostCount DESC
// LIMIT 50;
fn q7830(db: &'static So) -> String {
    let ps = pstat(db, db.post.iq());
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&ps).filt(|p: [i64; 13]| p[3] > 100).and((&bc).opt()).drive(|u, (p, b)| v.push((u, p, bz(b))));
    out(v, |&(_, p, _)| (Reverse(p[3]), Reverse(p[0])), 50, |&(u, p, b)| {
        let mut f = vec![user_col(db, u, "uid")];
        f.extend(ints(&[p[0], p[3], p[1], p[2]]));
        f.push(or0(p[5], p[0]));
        f.extend(ints(&b));
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
// SUM(CASE WHEN p.PostTypeId = 2 AND p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswerCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName
// ),
// UserBadgeStats AS (
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
// ),
// CombinedStats AS (
// SELECT
// ups.UserId,
// ups.DisplayName,
// ups.PostCount,
// ups.QuestionCount,
// ups.AnswerCount,
// ups.AcceptedAnswerCount,
// ubs.BadgeCount,
// ubs.GoldBadgeCount,
// ubs.SilverBadgeCount,
// ubs.BronzeBadgeCount
// FROM
// UserPostStats ups
// LEFT JOIN
// UserBadgeStats ubs ON ups.UserId = ubs.UserId
// )
// SELECT
// UserId,
// DisplayName,
// PostCount,
// QuestionCount,
// AnswerCount,
// AcceptedAnswerCount,
// COALESCE(BadgeCount, 0) AS BadgeCount,
// COALESCE(GoldBadgeCount, 0) AS GoldBadgeCount,
// COALESCE(SilverBadgeCount, 0) AS SilverBadgeCount,
// COALESCE(BronzeBadgeCount, 0) AS BronzeBadgeCount
// FROM
// CombinedStats
// ORDER BY
// PostCount DESC,
// BadgeCount DESC;
fn q12841(db: &'static So) -> String {
    let Post { post_type_id, accepted_answer_id, .. } = &db.post;
    let uf = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(accepted_answer_id.opt())).fold([0i64; 4], |a, (t, acc)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (t == 2 && acc.is_some()) as i64]
    });
    let bc = badge_classes(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&uf).opt()).and((&bc).opt())).drive(|_, ((u, a), b)| v.push((u, a.unwrap_or([0; 4]), bz(b))));
    rows(v.iter().map(|&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a));
        f.extend(ints(&b));
        row(f)
    }))
}

// WITH UserPostStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(P.Id) AS TotalPosts,
// COALESCE(SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END), 0) AS TotalQuestions,
// COALESCE(SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalAnswers,
// COALESCE(AVG(P.Score), 0) AS AvgScore
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
// COALESCE(UBS.TotalBadges, 0) AS TotalBadges,
// COALESCE(UBS.GoldBadges, 0) AS GoldBadges,
// COALESCE(UBS.SilverBadges, 0) AS SilverBadges,
// COALESCE(UBS.BronzeBadges, 0) AS BronzeBadges,
// COUNT(DISTINCT C.Id) AS TotalComments,
// SUM(COALESCE(C.Score, 0)) AS TotalCommentScores
// FROM
// UserPostStats UPS
// LEFT JOIN
// UserBadgeStats UBS ON UPS.UserId = UBS.UserId
// LEFT JOIN
// Comments C ON UPS.UserId = C.UserId
// WHERE
// UPS.TotalPosts > 0
// GROUP BY
// UPS.UserId, UPS.DisplayName, UPS.TotalPosts, UPS.TotalQuestions, UPS.TotalAnswers, UPS.AvgScore,
// UBS.TotalBadges, UBS.GoldBadges, UBS.SilverBadges, UBS.BronzeBadges
// ORDER BY
// UPS.TotalPosts DESC, UPS.AvgScore DESC
// LIMIT 50 OFFSET 0;
fn q140(db: &'static So) -> String {
    let ps = pstat(db, db.post.iq());
    let bc = badge_classes(db);
    let cs = db.comment.group_by(&db.comment.user).select(&db.comment.score).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let mut v = Vec::new();
    (&ps).and((&bc).opt()).and((&cs).opt()).drive(|u, ((p, b), c)| v.push((u, p, bz(b), c.unwrap_or([0; 2]))));
    let af = |p: [i64; 13]| p[3] as f64 / p[0] as f64;
    out(v, |&(_, p, ..)| (Reverse(p[0]), Reverse(fkey(af(p)))), 50, |&(u, p, b, c)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&[p[0], p[1], p[2]]));
        f.push(V::F(af(p)));
        f.extend(ints(&b));
        f.extend(ints(&c));
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
// PostStatistics AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// AVG(CASE WHEN p.Score IS NOT NULL THEN p.Score ELSE 0 END) AS AvgScore
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// ),
// TopUsers AS (
// SELECT
// ub.UserId,
// ub.DisplayName,
// ub.BadgeCount,
// ps.TotalPosts,
// ps.QuestionCount,
// ps.AnswerCount,
// COALESCE(ps.AvgScore, 0) AS AvgScore
// FROM
// UserBadges ub
// LEFT JOIN
// PostStatistics ps ON ub.UserId = ps.OwnerUserId
// WHERE
// ub.BadgeCount > 0
// )
// SELECT
// u.DisplayName,
// u.Reputation,
// t.BadgeCount,
// t.TotalPosts,
// t.QuestionCount,
// t.AnswerCount,
// t.AvgScore,
// CASE
// WHEN t.TotalPosts IS NULL THEN 'No Posts'
// WHEN t.TotalPosts > 100 THEN 'Top Contributor'
// ELSE 'Regular Contributor'
// END AS ContributionLevel
// FROM
// Users u
// LEFT JOIN
// TopUsers t ON u.Id = t.UserId
// WHERE
// u.Reputation > 1000
// ORDER BY
// t.BadgeCount DESC,
// u.Reputation DESC
// FETCH FIRST 10 ROWS ONLY;
fn q2421(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let Post { post_type_id, score, .. } = &db.post;
    let ps = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(score)).fold([0i64; 4], |a, (t, s)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s]);
    let tu = (&bu).filt(|b: i64| b > 0).and((&ps).opt());
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and(tu.opt())).drive(|_, (u, t)| v.push((u, t)));
    out(v, |&(u, t)| ((t.is_none(), Reverse(t.map(|(b, _)| b))), rep_desc(db, u)), 10, |&(u, t)| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(match t {
            None => nulls(5),
            Some((b, p)) => {
                let mut g = vec![V::I(b)];
                g.extend(p.map_or(nulls(3), |p| ints(&[p[0], p[1], p[2]])));
                g.push(V::F(p.map_or(0.0, |p| p[3] as f64 / p[0] as f64)));
                g
            }
        });
        f.push(V::S(match t.and_then(|(_, p)| p) {
            None => "No Posts",
            Some(p) if p[0] > 100 => "Top Contributor",
            _ => "Regular Contributor",
        }));
        f
    })
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.Reputation,
// U.CreationDate,
// COUNT(DISTINCT B.Id) AS BadgeCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// SUM(COALESCE(P.ViewCount, 0)) AS TotalViews,
// SUM(COALESCE(P.Score, 0)) AS TotalScore
// FROM Users U
// LEFT JOIN Badges B ON U.Id = B.UserId
// LEFT JOIN Votes V ON U.Id = V.UserId
// LEFT JOIN Posts P ON U.Id = P.OwnerUserId
// GROUP BY U.Id, U.Reputation, U.CreationDate
// ),
// PostStats AS (
// SELECT
// P.Id AS PostId,
// P.PostTypeId,
// P.CreationDate,
// P.AcceptedAnswerId,
// P.ViewCount,
// P.Score,
// COUNT(DISTINCT C.Id) AS CommentCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM Posts P
// LEFT JOIN Comments C ON P.Id = C.PostId
// LEFT JOIN Votes V ON P.Id = V.PostId
// GROUP BY P.Id, P.PostTypeId, P.CreationDate, P.AcceptedAnswerId, P.ViewCount, P.Score
// )
// SELECT
// U.UserId,
// U.Reputation,
// U.BadgeCount,
// U.UpVotes AS UserUpVotes,
// U.DownVotes AS UserDownVotes,
// U.TotalViews AS UserTotalViews,
// U.TotalScore AS UserTotalScore,
// P.PostId,
// P.PostTypeId,
// P.CreationDate AS PostCreationDate,
// P.ViewCount AS PostViewCount,
// P.Score AS PostScore,
// P.CommentCount AS PostCommentCount,
// P.UpVotes AS PostUpVotes,
// P.DownVotes AS PostDownVotes
// FROM UserStats U
// JOIN PostStats P ON U.UserId = P.AcceptedAnswerId
// ORDER BY U.Reputation DESC, P.Score DESC
// LIMIT 100;
fn q10702(db: &'static So) -> String {
    let uid = uids(db);
    let bu = badges_per_user(db);
    let Post { score, view_count, accepted_answer_id, .. } = &db.post;
    let tu: MatSet<Id<User>> = db.post.select(accepted_answer_id.select(&uid)).collect();
    let us = (&tu)
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(votes_by(db).select(&db.vote.vote_type_id).opt()).and(posts_of(db).select(score.and(view_count.opt())).opt()))
        .fold([0i64; 4], |a, ((_, t), p)| {
            let (s, w) = p.map_or((0, None), |(s, w)| (s, w));
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + w.unwrap_or(0), a[3] + s]
        });
    let ps = db
        .post
        .with(accepted_answer_id.select(&uid))
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 2], |a, (_, t)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cp = comments_per_post(db);
    let mut v = Vec::new();
    (&ps).and(&cp).and(accepted_answer_id.select(&uid).select(Ident::<User>::new().and(&bu).and(&us))).drive(|p, x| v.push((p, x)));
    out(v, |&(p, (_, ((u, _), _)))| (rep_desc(db, u), score_desc(db, p)), 100, |&(p, ((x, c), ((u, b), y)))| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep")];
        f.extend(ints(&[b, y[0], y[1], y[2], y[3]]));
        f.extend(post_fields(db, p, &["id", "type_id", "created", "views", "score"]));
        f.extend(ints(&[c, x[0], x[1]]));
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
// AVG(p.Score) AS AverageScore,
// SUM(p.ViewCount) AS TotalViews
// FROM Posts p
// WHERE p.OwnerUserId IS NOT NULL
// GROUP BY p.OwnerUserId
// ),
// CombinedStats AS (
// SELECT
// ub.UserId,
// ub.DisplayName,
// ub.TotalBadges,
// ub.GoldBadges,
// ub.SilverBadges,
// ub.BronzeBadges,
// ps.TotalPosts,
// ps.Questions,
// ps.Answers,
// ps.AverageScore,
// ps.TotalViews
// FROM UserBadgeStats ub
// LEFT JOIN PostStats ps ON ub.UserId = ps.OwnerUserId
// )
// SELECT
// UserId,
// DisplayName,
// COALESCE(TotalBadges, 0) AS TotalBadges,
// COALESCE(GoldBadges, 0) AS GoldBadges,
// COALESCE(SilverBadges, 0) AS SilverBadges,
// COALESCE(BronzeBadges, 0) AS BronzeBadges,
// COALESCE(TotalPosts, 0) AS TotalPosts,
// COALESCE(Questions, 0) AS Questions,
// COALESCE(Answers, 0) AS Answers,
// COALESCE(AverageScore, 0) AS AverageScore,
// COALESCE(TotalViews, 0) AS TotalViews
// FROM CombinedStats
// ORDER BY TotalPosts DESC, TotalBadges DESC
// LIMIT 100;
fn q5638(db: &'static So) -> String {
    let ub = ubc(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    (&ub).and((&ps).opt()).drive(|u, (b, p)| v.push((u, b, pz(p))));
    out(v, |&(_, b, p)| (Reverse(p[0]), Reverse(b[0])), 100, |&(u, b, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&b));
        f.extend(ints(&[p[0], p[1], p[2]]));
        f.extend([or0(p[3], p[0]), V::I(p[5])]);
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
// PostStatistics AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// COUNT(c.Id) AS CommentCount,
// COALESCE((SELECT COUNT(DISTINCT v.Id) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 2), 0) AS UpVotes,
// COALESCE((SELECT COUNT(DISTINCT v.Id) FROM Votes v WHERE v.PostId = p.Id AND v.VoteTypeId = 3), 0) AS DownVotes
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount
// )
// SELECT
// ua.UserId,
// ua.DisplayName,
// ua.TotalPosts,
// ua.Questions,
// ua.Answers,
// ua.TotalViews,
// ua.UpVotes AS UserUpVotes,
// ua.DownVotes AS UserDownVotes,
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.Score,
// ps.ViewCount,
// ps.CommentCount,
// ps.UpVotes AS PostUpVotes,
// ps.DownVotes AS PostDownVotes
// FROM
// UserActivity ua
// JOIN
// PostStatistics ps ON ua.UserId = ps.PostId
// ORDER BY
// ua.TotalPosts DESC, ps.Score DESC
// LIMIT 100;
fn q10730(db: &'static So) -> String {
    let pid = pids(db);
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let cp = comments_per_post(db);
    let pv = post_votes(db);
    let mut v = Vec::new();
    (&us).and((&dp).opt()).and((&db.user.origid).select(&pid).select(Ident::<Post>::new().and(&cp).and((&pv).opt()))).drive(|u, ((a, d), ((p, c), x))| v.push((u, a, d.unwrap_or(0), p, c, x.unwrap_or([0; 3]))));
    out(v, |&(_, _, d, p, ..)| (Reverse(d), score_desc(db, p)), 100, |&(u, a, d, p, c, x)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&[d, a.q, a.a]));
        f.push(ustat_field(&a, "views_sum"));
        f.extend(ints(&[a.up, a.down]));
        f.extend(post_fields(db, p, &["id", "title", "created", "score", "views"]));
        f.extend(ints(&[c, x[1], x[2]]));
        f
    })
}

// WITH UserStatistics AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
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
// ClosedPosts AS (
// SELECT
// ph.UserId,
// COUNT(DISTINCT ph.PostId) AS TotalClosedPosts
// FROM
// PostHistory ph
// WHERE
// ph.PostHistoryTypeId = 10
// GROUP BY
// ph.UserId
// ),
// BadgesSummary AS (
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
// us.UserId,
// us.DisplayName,
// us.TotalPosts,
// us.TotalQuestions,
// us.TotalAnswers,
// COALESCE(cb.TotalClosedPosts, 0) AS TotalClosedPosts,
// COALESCE(bs.GoldBadges, 0) AS GoldBadges,
// COALESCE(bs.SilverBadges, 0) AS SilverBadges,
// COALESCE(bs.BronzeBadges, 0) AS BronzeBadges,
// (us.TotalUpvotes - us.TotalDownvotes) AS NetScore
// FROM
// UserStatistics us
// LEFT JOIN
// ClosedPosts cb ON us.UserId = cb.UserId
// LEFT JOIN
// BadgesSummary bs ON us.UserId = bs.UserId
// WHERE
// us.TotalPosts > 0
// ORDER BY
// NetScore DESC
// LIMIT 10;
fn q1276(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let cp = db.post_history.with((&db.post_history.post_history_type_id).eq(10)).group_by(&db.post_history.user).select(&db.post_history.post).count_distinct();
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&us).and(&dp).and((&cp).opt()).and((&bc).opt()).drive(|u, (((a, d), c), b)| v.push((u, a, d, c.unwrap_or(0), bz(b))));
    out(v, |&(_, a, ..)| Reverse(a.up - a.down), 10, |&(u, a, d, c, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&[d, a.q, a.a, c, b[1], b[2], b[3], a.up - a.down]));
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
// COUNT(c.Id) AS CommentCount,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
// COALESCE(MAX(b.Id), 0) AS UserBadges
// FROM
// Posts p
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Badges b ON p.OwnerUserId = b.UserId
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount
// ),
// UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges,
// COUNT(DISTINCT p.Id) AS PostsCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// WHERE
// u.CreationDate >= '2023-01-01'
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
// ps.UpVotes,
// ps.DownVotes,
// us.UserId,
// us.DisplayName,
// us.GoldBadges,
// us.SilverBadges,
// us.BronzeBadges,
// us.PostsCount
// FROM
// PostStats ps
// JOIN
// UserStats us ON ps.UserBadges = us.UserId
// ORDER BY
// ps.CreationDate DESC;
fn q10663(db: &'static So) -> String {
    let uid = uids(db);
    let by_uid: HashIdx<i64, Id<Badge>> = (&db.badge.user_id).inv().collect();
    let pf = since(db, date(2023, 1, 1))
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()).and((&db.post.owner_user_id).select(&by_uid).select(&db.badge.origid).opt()))
        .fold([0, 0, 0, i64::MIN], |a: [i64; 4], ((t, c), b)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64, b.map_or(a[3], |b| a[3].max(b))]);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let bc = user_pb_classes(db, db.user.with((&db.user.creation_date).ge(date(2023, 1, 1))));
    let mut v = Vec::new();
    (&pf)
        .and((&pf).map(|a: [i64; 4]| if a[3] == i64::MIN { 0 } else { a[3] }).select(&uid).select(Ident::<User>::new().and((&dp).opt()).and(&bc)))
        .drive(|p, (s, ((u, d), b))| v.push((p, s, u, d.unwrap_or(0), b)));
    rows(v.iter().map(|&(p, s, u, d, b)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers"]);
        f.extend(ints(&[s[0], s[1], s[2]]));
        f.extend([user_col(db, u, "uid"), user_col(db, u, "name")]);
        f.extend(ints(&[b[1], b[2], b[3], d]));
        row(f)
    }))
}

// WITH PostStatistics AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.PostTypeId,
// P.CreationDate,
// P.Score,
// P.ViewCount,
// COALESCE(COUNT(C.Id), 0) AS CommentCount,
// COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount,
// COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// P.Id, P.Title, P.PostTypeId, P.CreationDate, P.Score, P.ViewCount
// ),
// UserStatistics AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// U.CreationDate,
// COALESCE(COUNT(DISTINCT P.Id), 0) AS PostCount,
// COALESCE(SUM(P.Score), 0) AS TotalScore,
// COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS TotalUpVotes,
// COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS TotalDownVotes
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// U.Id, U.DisplayName, U.Reputation, U.CreationDate
// )
// SELECT
// PS.PostId,
// PS.Title,
// PS.PostTypeId,
// PS.CreationDate,
// PS.Score,
// PS.ViewCount,
// PS.CommentCount,
// PS.UpVoteCount,
// PS.DownVoteCount,
// US.UserId,
// US.DisplayName,
// US.Reputation,
// US.CreationDate AS UserCreationDate,
// US.PostCount,
// US.TotalScore,
// US.TotalUpVotes,
// US.TotalDownVotes
// FROM
// PostStatistics PS
// JOIN
// UserStatistics US ON PS.PostId = US.UserId
// ORDER BY
// PS.CreationDate DESC;
fn q10529(db: &'static So) -> String {
    let uid = uids(db);
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let mut v = Vec::new();
    stats_fold(db, db.post.with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cv", &[])
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&us).and((&dp).opt())))
        .drive(|p, (s, ((u, a), d))| v.push((p, s, u, a, d.unwrap_or(0))));
    rows(v.iter().map(|&(p, s, u, a, d)| {
        let mut f = post_fields(db, p, &["id", "title", "type_id", "created", "score", "views"]);
        f.extend(ints(&[s.cx, s.up, s.down]));
        f.extend(["uid", "name", "rep", "ucreated"].iter().map(|k| user_col(db, u, k)));
        f.extend(ints(&[d, a.score_sum, a.up, a.down]));
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
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// u.Id, u.DisplayName
// ),
// UserBadges AS (
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
// UserSummary AS (
// SELECT
// ua.UserId,
// ua.DisplayName,
// ua.PostCount,
// ua.QuestionCount,
// ua.AnswerCount,
// ua.Upvotes,
// ua.Downvotes,
// COALESCE(ub.BadgeCount, 0) AS BadgeCount,
// COALESCE(ub.GoldBadges, 0) AS GoldBadges,
// COALESCE(ub.SilverBadges, 0) AS SilverBadges,
// COALESCE(ub.BronzeBadges, 0) AS BronzeBadges
// FROM
// UserActivity ua
// LEFT JOIN
// UserBadges ub ON ua.UserId = ub.UserId
// )
// SELECT
// DisplayName,
// PostCount,
// QuestionCount,
// AnswerCount,
// Upvotes,
// Downvotes,
// BadgeCount,
// GoldBadges,
// SilverBadges,
// BronzeBadges
// FROM
// UserSummary
// ORDER BY
// PostCount DESC,
// Upvotes DESC
// FETCH FIRST 10 ROWS ONLY;
fn q7523(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&us).and((&dp).opt()).and((&bc).opt()).drive(|u, ((a, d), b)| v.push((u, a, d.unwrap_or(0), bz(b))));
    out(v, |&(_, a, d, _)| (Reverse(d), Reverse(a.up)), 10, |&(u, a, d, b)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(ints(&[d, a.q, a.a, a.up, a.down]));
        f.extend(ints(&b));
        f
    })
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesCount
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
// COUNT(CASE WHEN Class = 1 THEN 1 END) AS GoldBadges,
// COUNT(CASE WHEN Class = 2 THEN 1 END) AS SilverBadges,
// COUNT(CASE WHEN Class = 3 THEN 1 END) AS BronzeBadges
// FROM
// Badges
// GROUP BY
// UserId
// ),
// MergedStats AS (
// SELECT
// u.UserId,
// u.Reputation,
// u.PostCount,
// u.QuestionsCount,
// u.AnswersCount,
// u.UpVotesCount,
// u.DownVotesCount,
// COALESCE(b.GoldBadges, 0) AS GoldBadges,
// COALESCE(b.SilverBadges, 0) AS SilverBadges,
// COALESCE(b.BronzeBadges, 0) AS BronzeBadges
// FROM
// UserStats u
// LEFT JOIN
// BadgeStats b ON u.UserId = b.UserId
// )
// SELECT
// UserId,
// Reputation,
// PostCount,
// QuestionsCount,
// AnswersCount,
// UpVotesCount,
// DownVotesCount,
// GoldBadges,
// SilverBadges,
// BronzeBadges,
// (QuestionsCount * 3 + AnswersCount * 2 + UpVotesCount * 1 - DownVotesCount * 1) AS Score
// FROM
// MergedStats
// WHERE
// Reputation > 1000
// ORDER BY
// Score DESC
// FETCH FIRST 10 ROWS ONLY;
fn q9396(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::RepGt(1000), "v", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&us).and((&dp).opt()).and((&bc).opt()).drive(|u, ((a, d), b)| v.push((u, a, d.unwrap_or(0), bz(b), a.q * 3 + a.a * 2 + a.up - a.down)));
    out(v, |&(.., s)| Reverse(s), 10, |&(u, a, d, b, s)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep")];
        f.extend(ints(&[d, a.q, a.a, a.up, a.down, b[1], b[2], b[3], s]));
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
// FROM Users u
// LEFT JOIN Badges b ON u.Id = b.UserId
// GROUP BY u.Id
// ),
// PostStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS PostCount,
// SUM(p.Score) AS TotalScore,
// SUM(p.ViewCount) AS TotalViews,
// AVG(p.AnswerCount) AS AverageAnswers,
// AVG(p.CommentCount) AS AverageComments
// FROM Posts p
// WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY p.OwnerUserId
// ),
// CombinedStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COALESCE(pb.PostCount, 0) AS PostCount,
// COALESCE(pb.TotalScore, 0) AS TotalScore,
// COALESCE(pb.TotalViews, 0) AS TotalViews,
// COALESCE(pb.AverageAnswers, 0) AS AverageAnswers,
// COALESCE(pb.AverageComments, 0) AS AverageComments,
// COALESCE(ub.BadgeCount, 0) AS BadgeCount,
// COALESCE(ub.GoldBadges, 0) AS GoldBadges,
// COALESCE(ub.SilverBadges, 0) AS SilverBadges,
// COALESCE(ub.BronzeBadges, 0) AS BronzeBadges
// FROM Users u
// LEFT JOIN PostStats pb ON u.Id = pb.OwnerUserId
// LEFT JOIN UserBadges ub ON u.Id = ub.UserId
// )
// SELECT
// UserId,
// DisplayName,
// PostCount,
// TotalScore,
// TotalViews,
// AverageAnswers,
// AverageComments,
// BadgeCount,
// GoldBadges,
// SilverBadges,
// BronzeBadges
// FROM CombinedStats
// ORDER BY TotalScore DESC, PostCount DESC
// LIMIT 10;
fn q9432(db: &'static So) -> String {
    let bc = badge_classes(db);
    let ps = pstat(db, since(db, year_ago()));
    let cc = owned_since(db, year_ago()).group_by(&db.post.owner_user).select(&db.post.comment_count).fold(0i64, |a, c| a + c);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&ps).opt()).and((&cc).opt()).and((&bc).opt())).drive(|_, (((u, p), c), b)| v.push((u, pz(p), c.unwrap_or(0), bz(b))));
    out(v, |&(_, p, ..)| (Reverse(p[3]), Reverse(p[0])), 10, |&(u, p, c, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&[p[0], p[3], p[5]]));
        f.extend([or0(p[7], p[6]), or0(c, p[0])]);
        f.extend(ints(&b));
        f
    })
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
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
// u.Id, u.DisplayName, u.Reputation
// ),
// PostInteraction AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// COALESCE(SUM(CASE WHEN c.Id IS NOT NULL THEN 1 ELSE 0 END), 0) AS CommentCount,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score
// ),
// CombinedStats AS (
// SELECT
// us.UserId,
// us.DisplayName,
// us.Reputation,
// us.BadgeCount,
// pi.PostId,
// pi.Title,
// pi.CreationDate,
// pi.Score,
// pi.CommentCount,
// pi.UpVoteCount,
// pi.DownVoteCount
// FROM
// UserStats us
// JOIN
// PostInteraction pi ON us.UserId = pi.PostId
// )
// SELECT
// cs.DisplayName,
// cs.Reputation,
// cs.BadgeCount,
// cs.Title,
// cs.CreationDate,
// cs.Score,
// cs.CommentCount,
// cs.UpVoteCount,
// cs.DownVoteCount
// FROM
// CombinedStats cs
// WHERE
// cs.Reputation > 1000
// ORDER BY
// cs.Reputation DESC,
// cs.Score DESC;
fn q6256(db: &'static So) -> String {
    let pid = pids(db);
    let uv = user_bv(db);
    let sf = stats_fold(db, db.post.iq(), Ident::<Post>::new(), "cv", &[]);
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000))
        .select(Ident::<User>::new().and(&uv).and((&db.user.origid).select(&pid).select(Ident::<Post>::new().and(&sf))))
        .drive(|_, ((u, x), (p, s))| v.push((u, x, p, s)));
    rows(v.iter().map(|&(u, x, p, s)| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep"), V::I(x[0])];
        f.extend(post_fields(db, p, &["title", "created", "score"]));
        f.extend(ints(&[s.cx, s.up, s.down]));
        row(f)
    }))
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(B.Id) AS BadgeCount,
// SUM(CASE WHEN Vote.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesReceived,
// SUM(CASE WHEN Vote.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesReceived
// FROM
// Users U
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// LEFT JOIN
// Votes Vote ON U.Id = Vote.UserId
// GROUP BY
// U.Id, U.DisplayName, U.Reputation
// ), PostStats AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS TotalPosts,
// COUNT(DISTINCT CASE WHEN P.PostTypeId = 1 THEN P.Id END) AS Questions,
// COUNT(DISTINCT CASE WHEN P.PostTypeId = 2 THEN P.Id END) AS Answers,
// AVG(P.Score) AS AvgPostScore,
// SUM(P.ViewCount) AS TotalViews
// FROM
// Posts P
// GROUP BY
// P.OwnerUserId
// ), CombinedStats AS (
// SELECT
// U.UserId,
// U.DisplayName,
// U.Reputation,
// U.BadgeCount,
// U.UpVotesReceived,
// U.DownVotesReceived,
// P.TotalPosts,
// P.Questions,
// P.Answers,
// P.AvgPostScore,
// P.TotalViews
// FROM
// UserStats U
// JOIN
// PostStats P ON U.UserId = P.OwnerUserId
// )
// SELECT
// C.DisplayName,
// C.Reputation,
// C.BadgeCount,
// C.UpVotesReceived,
// C.DownVotesReceived,
// COALESCE(C.TotalPosts, 0) AS TotalPosts,
// COALESCE(C.Questions, 0) AS TotalQuestions,
// COALESCE(C.Answers, 0) AS TotalAnswers,
// COALESCE(C.AvgPostScore, 0) AS AvgPostScore,
// COALESCE(C.TotalViews, 0) AS TotalViews
// FROM
// CombinedStats C
// ORDER BY
// C.Reputation DESC,
// C.BadgeCount DESC,
// C.UpVotesReceived DESC
// LIMIT 100;
fn q6448(db: &'static So) -> String {
    let uv = user_bv(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    (&ps).and(&uv).drive(|u, (p, x)| v.push((u, p, x[0], x[1], x[2])));
    out(v, |&(u, _, b, up, _)| (rep_desc(db, u), Reverse(b), Reverse(up)), 100, |&(u, p, b, up, down)| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(ints(&[b, up, down, p[0], p[1], p[2]]));
        f.extend([pscore_avg(p), V::I(p[5])]);
        f
    })
}

// WITH UserBadgeCounts AS (
// SELECT
// U.Id AS UserId,
// COUNT(B.Id) FILTER (WHERE B.Class = 1) AS GoldBadgeCount,
// COUNT(B.Id) FILTER (WHERE B.Class = 2) AS SilverBadgeCount,
// COUNT(B.Id) FILTER (WHERE B.Class = 3) AS BronzeBadgeCount
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
// SUM(COALESCE(P.Score, 0)) AS TotalScore,
// AVG(COALESCE(P.ViewCount, 0)) AS AverageViews
// FROM
// Posts P
// WHERE
// P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY
// P.OwnerUserId
// ),
// TopUsers AS (
// SELECT
// U.Id,
// U.DisplayName,
// U.Reputation,
// COALESCE(UBC.GoldBadgeCount, 0) AS GoldBadges,
// COALESCE(UBC.SilverBadgeCount, 0) AS SilverBadges,
// COALESCE(UBC.BronzeBadgeCount, 0) AS BronzeBadges,
// PS.TotalPosts,
// PS.TotalScore,
// PS.AverageViews
// FROM
// Users U
// LEFT JOIN
// UserBadgeCounts UBC ON U.Id = UBC.UserId
// LEFT JOIN
// PostStats PS ON U.Id = PS.OwnerUserId
// WHERE
// U.Reputation > 1000
// )
// SELECT
// T.DisplayName,
// T.Reputation,
// T.GoldBadges,
// T.SilverBadges,
// T.BronzeBadges,
// T.TotalPosts,
// T.TotalScore,
// T.AverageViews,
// CASE
// WHEN T.TotalScore >= 100 THEN 'High Scorer'
// WHEN T.TotalScore BETWEEN 50 AND 99 THEN 'Moderate Scorer'
// ELSE 'Low Scorer'
// END AS ScoreCategory
// FROM
// TopUsers T
// WHERE
// T.TotalPosts > (SELECT AVG(TotalPosts) FROM TopUsers)
// ORDER BY
// T.Reputation DESC
// LIMIT 10;
fn q2074(db: &'static So) -> String {
    let bc = badge_classes(db);
    let Post { score, view_count, .. } = &db.post;
    let ps = owned_since(db, year_ago()).group_by(&db.post.owner_user).select(score.and(view_count.opt())).fold([0i64; 3], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.unwrap_or(0)]);
    let t = user_base(db, UserWhere::RepGt(1000)).select(&ps).fold_flat([0i64; 2], |a, p| [a[0] + 1, a[1] + p[0]]);
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000))
        .select(Ident::<User>::new().and(&ps).and((&bc).opt()))
        .filt(move |((_, p), _): ((Id<User>, [i64; 3]), Option<[i64; 4]>)| (p[0] * t[0]) as i128 > t[1] as i128)
        .drive(|_, ((u, p), b)| v.push((u, p, bz(b))));
    out(v, |&(u, ..)| rep_desc(db, u), 10, |&(u, p, b)| {
        let cat = if p[1] >= 100 { "High Scorer" } else if (50..=99).contains(&p[1]) { "Moderate Scorer" } else { "Low Scorer" };
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(ints(&[b[1], b[2], b[3], p[0], p[1]]));
        f.extend([V::F(p[2] as f64 / p[0] as f64), V::S(cat)]);
        f
    })
}

// WITH UserReputation AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(DISTINCT P.Id) AS PostCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id, U.DisplayName, U.Reputation
// ),
// UserBadges AS (
// SELECT
// B.UserId,
// COUNT(*) AS BadgeCount,
// SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM
// Badges B
// GROUP BY
// B.UserId
// ),
// PostsWithComments AS (
// SELECT
// P.OwnerUserId,
// COUNT(C.Id) AS CommentCount,
// SUM(P.ViewCount) AS TotalViewCount
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// WHERE
// P.CreationDate >= '2023-01-01'
// GROUP BY
// P.OwnerUserId
// )
// SELECT
// UR.UserId,
// UR.DisplayName,
// UR.Reputation,
// UR.PostCount,
// UR.QuestionCount,
// UR.AnswerCount,
// COALESCE(UB.BadgeCount, 0) AS BadgeCount,
// COALESCE(UB.GoldBadges, 0) AS GoldBadges,
// COALESCE(UB.SilverBadges, 0) AS SilverBadges,
// COALESCE(UB.BronzeBadges, 0) AS BronzeBadges,
// COALESCE(PC.CommentCount, 0) AS CommentCount,
// COALESCE(PC.TotalViewCount, 0) AS TotalViewCount
// FROM
// UserReputation UR
// LEFT JOIN
// UserBadges UB ON UR.UserId = UB.UserId
// LEFT JOIN
// PostsWithComments PC ON UR.UserId = PC.OwnerUserId
// ORDER BY
// UR.Reputation DESC, UR.PostCount DESC;
fn q7218(db: &'static So) -> String {
    let Post { post_type_id, view_count, .. } = &db.post;
    let ps = owned(db).group_by(&db.post.owner_user).select(post_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64]);
    let bc = badge_classes(db);
    let pc = owned_since(db, date(2023, 1, 1)).group_by(&db.post.owner_user).select(view_count.opt().and(comments_of(db).opt())).fold([0i64; 2], |a, (w, c)| [a[0] + c.is_some() as i64, a[1] + w.unwrap_or(0)]);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&ps).opt()).and((&bc).opt()).and((&pc).opt())).drive(|_, (((u, p), b), c)| v.push((u, p.unwrap_or([0; 3]), bz(b), c.unwrap_or([0; 2]))));
    rows(v.iter().map(|&(u, p, b, c)| {
        let mut f = ["uid", "name", "rep"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.extend(ints(&p));
        f.extend(ints(&b));
        f.extend(ints(&c));
        row(f)
    }))
}

// WITH UserBadgeStats AS (
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
// RecentPosts AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS RecentPostCount,
// AVG(p.Score) AS AvgScore
// FROM
// Posts p
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 month'
// GROUP BY
// p.OwnerUserId
// ),
// UsersWithActivity AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COALESCE(ubs.TotalBadges, 0) AS TotalBadges,
// COALESCE(ubs.GoldBadges, 0) AS GoldBadges,
// COALESCE(ubs.SilverBadges, 0) AS SilverBadges,
// COALESCE(ubs.BronzeBadges, 0) AS BronzeBadges,
// COALESCE(rp.RecentPostCount, 0) AS RecentPostCount,
// COALESCE(rp.AvgScore, 0) AS AvgScore
// FROM
// Users u
// LEFT JOIN
// UserBadgeStats ubs ON u.Id = ubs.UserId
// LEFT JOIN
// RecentPosts rp ON u.Id = rp.OwnerUserId
// )
// SELECT
// uwa.UserId,
// uwa.DisplayName,
// uwa.TotalBadges,
// uwa.GoldBadges,
// uwa.SilverBadges,
// uwa.BronzeBadges,
// uwa.RecentPostCount,
// uwa.AvgScore,
// CASE
// WHEN uwa.TotalBadges >= 10 THEN 'Expert'
// WHEN uwa.AvgScore > 50 THEN 'Active Contributor'
// ELSE 'Novice'
// END AS UserStatus
// FROM
// UsersWithActivity uwa
// WHERE
// uwa.RecentPostCount > 0
// ORDER BY
// uwa.TotalBadges DESC, uwa.AvgScore DESC;
fn q18(db: &'static So) -> String {
    let bc = badge_classes(db);
    let rp = owned_since(db, month_ago()).group_by(&db.post.owner_user).select(&db.post.score).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and(&rp).and((&bc).opt())).drive(|_, ((u, p), b)| v.push((u, p, bz(b))));
    let af = |p: [i64; 2]| p[1] as f64 / p[0] as f64;
    rows(v.iter().map(|&(u, p, b)| {
        let st = if b[0] >= 10 { "Expert" } else if af(p) > 50.0 { "Active Contributor" } else { "Novice" };
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&b));
        f.extend([V::I(p[0]), V::F(af(p)), V::S(st)]);
        row(f)
    }))
}

// WITH UserActivity AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(P.Id) AS PostCount,
// SUM(COALESCE(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END, 0)) AS QuestionCount,
// SUM(COALESCE(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END, 0)) AS AnswerCount,
// SUM(COALESCE(CASE WHEN P.PostTypeId IN (1, 2) THEN P.Score ELSE 0 END, 0)) AS TotalScore,
// SUM(COALESCE(CASE WHEN C.Id IS NOT NULL THEN 1 ELSE 0 END, 0)) AS CommentCount,
// SUM(COALESCE(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END, 0)) AS Upvotes,
// SUM(COALESCE(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END, 0)) AS Downvotes,
// MAX(P.CreationDate) AS LastPostDate
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
// BadgeCounts AS (
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
// UA.UserId,
// UA.DisplayName,
// UA.PostCount,
// UA.QuestionCount,
// UA.AnswerCount,
// UA.TotalScore,
// UA.CommentCount,
// UA.Upvotes,
// UA.Downvotes,
// B.BadgeCount,
// B.GoldBadges,
// B.SilverBadges,
// B.BronzeBadges,
// UA.LastPostDate
// FROM
// UserActivity UA
// LEFT JOIN
// BadgeCounts B ON UA.UserId = B.UserId
// WHERE
// UA.PostCount > 0
// ORDER BY
// UA.TotalScore DESC, UA.LastPostDate DESC
// LIMIT 100;
fn q9574(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, .. } = &db.post;
    let uf = owned(db)
        .group_by(&db.post.owner_user)
        .select(post_type_id.and(score).and(creation_date).and(comments_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0, 0, 0, 0, 0, 0, 0, i64::MIN], |a: [i64; 8], ((((t, s), d), c), v)| {
            [
                a[0] + 1,
                a[1] + (t == 1) as i64,
                a[2] + (t == 2) as i64,
                a[3] + if matches!(t, 1 | 2) { s } else { 0 },
                a[4] + c.is_some() as i64,
                a[5] + (v == Some(2)) as i64,
                a[6] + (v == Some(3)) as i64,
                a[7].max(d),
            ]
        });
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&uf).and((&bc).opt()).drive(|u, (a, b)| v.push((u, a, b)));
    out(v, |&(_, a, _)| (Reverse(a[3]), Reverse(a[7])), 100, |&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a[..7]));
        f.extend(b.map_or(nulls(4), |b| ints(&b)));
        f.push(V::T(a[7]));
        f
    })
}

// WITH UserActivity AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(DISTINCT P.Id) AS PostCount,
// SUM(COALESCE(P.ViewCount, 0)) AS TotalViews,
// SUM(COALESCE(P.Score, 0)) AS TotalScore,
// SUM(COALESCE(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END, 0)) AS QuestionsCount,
// SUM(COALESCE(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END, 0)) AS AnswersCount,
// SUM(COALESCE(CASE WHEN P.PostTypeId = 3 THEN 1 ELSE 0 END, 0)) AS WikisCount
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// WHERE
// U.Reputation > 1000
// GROUP BY
// U.Id, U.DisplayName
// ),
// UserBadges AS (
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
// FinalReport AS (
// SELECT
// UA.UserId,
// UA.DisplayName,
// UA.PostCount,
// UA.TotalViews,
// UA.TotalScore,
// UA.QuestionsCount,
// UA.AnswersCount,
// UA.WikisCount,
// COALESCE(UB.GoldBadges, 0) AS GoldBadges,
// COALESCE(UB.SilverBadges, 0) AS SilverBadges,
// COALESCE(UB.BronzeBadges, 0) AS BronzeBadges
// FROM
// UserActivity UA
// LEFT JOIN
// UserBadges UB ON UA.UserId = UB.UserId
// )
// SELECT
// FR.DisplayName,
// FR.PostCount,
// FR.TotalViews,
// FR.TotalScore,
// FR.QuestionsCount,
// FR.AnswersCount,
// FR.WikisCount,
// FR.GoldBadges,
// FR.SilverBadges,
// FR.BronzeBadges
// FROM
// FinalReport FR
// ORDER BY
// FR.TotalScore DESC, FR.PostCount DESC
// LIMIT 20;
fn q6337(db: &'static So) -> String {
    let ps = pstat(db, db.post.iq());
    let bc = badge_classes(db);
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and((&ps).opt()).and((&bc).opt())).drive(|_, ((u, p), b)| v.push((u, pz(p), bz(b))));
    out(v, |&(_, p, _)| (Reverse(p[3]), Reverse(p[0])), 20, |&(u, p, b)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(ints(&[p[0], p[5], p[3], p[1], p[2], p[8], b[1], b[2], b[3]]));
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
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// COALESCE(SUM(P.Score), 0) AS TotalScore,
// COALESCE(SUM(P.ViewCount), 0) AS TotalViews,
// COUNT(DISTINCT C.Id) AS TotalComments
// FROM
// Users U
// LEFT JOIN Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN Comments C ON P.Id = C.PostId
// WHERE
// U.Reputation > 1000
// GROUP BY
// U.Id, U.DisplayName, U.Reputation, U.CreationDate
// ),
// PostHistoryStats AS (
// SELECT
// PH.UserId,
// COUNT(PH.Id) AS TotalEdits,
// COUNT(DISTINCT PH.Comment) AS UniqueEditComments,
// MAX(PH.CreationDate) AS LastEditDate
// FROM
// PostHistory PH
// GROUP BY
// PH.UserId
// ),
// Summary AS (
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
// COALESCE(PHS.TotalEdits, 0) AS TotalEdits,
// COALESCE(PHS.UniqueEditComments, 0) AS UniqueEditComments,
// PHS.LastEditDate
// FROM
// UserStats US
// LEFT JOIN PostHistoryStats PHS ON US.UserId = PHS.UserId
// )
// SELECT
// DisplayName,
// Reputation,
// TotalPosts,
// TotalQuestions,
// TotalAnswers,
// TotalScore,
// TotalViews,
// TotalComments,
// TotalEdits,
// UniqueEditComments,
// LastEditDate
// FROM
// Summary
// ORDER BY
// TotalScore DESC,
// Reputation DESC
// LIMIT 10;
fn q7798(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::RepGt(1000), "c", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let ph = db.post_history.group_by(&db.post_history.user).select(&db.post_history.creation_date).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let pc = db.post_history.group_by(&db.post_history.user).select(&db.post_history.comment).count_distinct();
    let mut v = Vec::new();
    (&us).and((&dp).opt()).and((&ph).opt()).and((&pc).opt()).drive(|u, (((a, d), h), c)| v.push((u, a, d.unwrap_or(0), h, c.unwrap_or(0))));
    out(v, |&(u, a, ..)| (Reverse(a.score_sum), rep_desc(db, u)), 10, |&(u, a, d, h, c)| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(ints(&[d, a.q, a.a, a.score_sum, a.views_sum, a.cx, h.map_or(0, |h| h.0), c]));
        f.push(ots(h.map(|h| h.1)));
        f
    })
}

// WITH UserReputation AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(DISTINCT P.Id) AS TotalPosts,
// COUNT(DISTINCT CASE WHEN P.PostTypeId = 1 THEN P.Id END) AS Questions,
// COUNT(DISTINCT CASE WHEN P.PostTypeId = 2 THEN P.Id END) AS Answers,
// SUM(COALESCE(P.Score, 0)) AS TotalScore
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id, U.DisplayName, U.Reputation
// ),
// BadgeSummary AS (
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
// TopAnswers AS (
// SELECT
// P.OwnerUserId,
// COUNT(A.Id) AS TotalAcceptedAnswers,
// SUM(A.Score) AS AcceptedScore
// FROM
// Posts P
// JOIN
// Posts A ON P.AcceptedAnswerId = A.Id
// WHERE
// P.PostTypeId = 1
// GROUP BY
// P.OwnerUserId
// )
// SELECT
// UR.DisplayName AS UserName,
// UR.Reputation,
// UR.TotalPosts,
// UR.Questions,
// UR.Answers,
// UR.TotalScore,
// COALESCE(BS.GoldBadges, 0) AS GoldBadges,
// COALESCE(BS.SilverBadges, 0) AS SilverBadges,
// COALESCE(BS.BronzeBadges, 0) AS BronzeBadges,
// COALESCE(TA.TotalAcceptedAnswers, 0) AS TotalAcceptedAnswers,
// COALESCE(TA.AcceptedScore, 0) AS AcceptedScore
// FROM
// UserReputation UR
// LEFT JOIN
// BadgeSummary BS ON UR.UserId = BS.UserId
// LEFT JOIN
// TopAnswers TA ON UR.UserId = TA.OwnerUserId
// WHERE
// UR.Reputation > 1000
// ORDER BY
// UR.TotalScore DESC,
// UR.Reputation DESC
// LIMIT 10;
fn q25805(db: &'static So) -> String {
    let ps = pstat(db, db.post.iq());
    let bc = badge_classes(db);
    let ta = questions_only(db).group_by(&db.post.owner_user).select((&db.post.accepted_answer).select(&db.post.score)).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and((&ps).opt()).and((&bc).opt()).and((&ta).opt())).drive(|_, (((u, p), b), t)| v.push((u, pz(p), bz(b), t.unwrap_or([0; 2]))));
    out(v, |&(u, p, ..)| (Reverse(p[3]), rep_desc(db, u)), 10, |&(u, p, b, t)| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(ints(&[p[0], p[1], p[2], p[3], b[1], b[2], b[3], t[0], t[1]]));
        f
    })
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.Reputation,
// COUNT(DISTINCT P.Id) AS TotalPosts,
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
// U.Id, U.Reputation
// ),
// BadgeCounts AS (
// SELECT
// UserId,
// COUNT(*) AS TotalBadges
// FROM
// Badges
// GROUP BY
// UserId
// ),
// PostHistoryStats AS (
// SELECT
// PH.UserId,
// COUNT(*) AS TotalPostHistoryEdits,
// SUM(CASE WHEN PH.PostHistoryTypeId IN (4, 5) THEN 1 ELSE 0 END) AS TotalTitleEdits,
// SUM(CASE WHEN PH.PostHistoryTypeId = 10 THEN 1 ELSE 0 END) AS TotalClosures
// FROM
// PostHistory PH
// GROUP BY
// PH.UserId
// )
// SELECT
// U.DisplayName,
// US.Reputation,
// US.TotalPosts,
// US.TotalQuestions,
// US.TotalAnswers,
// US.TotalUpvotes,
// US.TotalDownvotes,
// COALESCE(BC.TotalBadges, 0) AS TotalBadges,
// COALESCE(PHS.TotalPostHistoryEdits, 0) AS TotalPostHistoryEdits,
// COALESCE(PHS.TotalTitleEdits, 0) AS TotalTitleEdits,
// COALESCE(PHS.TotalClosures, 0) AS TotalClosures
// FROM
// Users U
// JOIN
// UserStats US ON U.Id = US.UserId
// LEFT JOIN
// BadgeCounts BC ON U.Id = BC.UserId
// LEFT JOIN
// PostHistoryStats PHS ON U.Id = PHS.UserId
// WHERE
// U.Reputation > 1000
// ORDER BY
// U.Reputation DESC,
// US.TotalPosts DESC
// LIMIT 50;
fn q8112(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::RepGt(1000), "v", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let bu = badges_per_user(db);
    let ph = db.post_history.group_by(&db.post_history.user).select(&db.post_history.post_history_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + matches!(t, 4 | 5) as i64, a[2] + (t == 10) as i64]);
    let mut v = Vec::new();
    (&us).and((&dp).opt()).and(&bu).and((&ph).opt()).drive(|u, (((a, d), b), h)| v.push((u, a, d.unwrap_or(0), b, h.unwrap_or([0; 3]))));
    out(v, |&(u, _, d, ..)| (rep_desc(db, u), Reverse(d)), 50, |&(u, a, d, b, h)| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(ints(&[d, a.q, a.a, a.up, a.down, b]));
        f.extend(ints(&h));
        f
    })
}

// WITH UserActivity AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN c.Id IS NOT NULL THEN 1 ELSE 0 END) AS CommentCount,
// SUM(CASE WHEN v.Id IS NOT NULL THEN 1 ELSE 0 END) AS VoteCount
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
// PostSummary AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// pt.Name AS PostType,
// p.Score,
// p.ViewCount,
// COALESCE(p.AcceptedAnswerId, 0) AS AcceptedAnswerId,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate, pt.Name, p.Score, p.ViewCount, p.AcceptedAnswerId
// )
// SELECT
// ua.UserId,
// ua.DisplayName,
// ua.PostCount,
// ua.QuestionCount,
// ua.AnswerCount,
// ua.CommentCount,
// ua.VoteCount,
// ps.PostId,
// ps.Title AS PostTitle,
// ps.CreationDate AS PostCreationDate,
// ps.PostType,
// ps.Score AS PostScore,
// ps.ViewCount AS PostViewCount,
// ps.CommentCount AS PostCommentCount,
// ps.VoteCount AS PostVoteCount
// FROM
// UserActivity ua
// LEFT JOIN
// PostSummary ps ON ua.UserId = ps.AcceptedAnswerId
// ORDER BY
// ua.PostCount DESC;
fn q10842(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "cv", any_post);
    let acc: HashIdx<i64, Id<Post>> = db.post.select((&db.post.accepted_answer_id).opt().map(|a: Option<i64>| a.unwrap_or(0))).inv().collect();
    let sf = stats_fold(db, db.post.iq(), Ident::<Post>::new(), "cv", &[]);
    let mut v = Vec::new();
    (&us).and((&db.user.origid).select(&acc).select(Ident::<Post>::new().and(&sf)).opt()).drive(|u, (a, p)| v.push((u, a, p)));
    rows(v.iter().map(|&(u, a, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&[a.n, a.q, a.a, a.cx, a.vx]));
        f.extend(match p {
            Some((p, s)) => {
                let mut g = post_fields(db, p, &["id", "title", "created", "type", "score", "views"]);
                g.extend(ints(&[s.cx, s.vx]));
                g
            }
            None => nulls(8),
        });
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
// SUM(CASE WHEN p.PostTypeId IN (4, 5) THEN 1 ELSE 0 END) AS TotalTagWikis,
// SUM(p.Score) AS TotalScore,
// MAX(p.CreationDate) AS LastPostDate
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// GROUP BY u.Id, u.DisplayName
// ),
// PostHistoryStats AS (
// SELECT
// ph.UserId,
// COUNT(ph.Id) AS TotalEdits,
// COUNT(DISTINCT ph.PostId) AS TotalEditedPosts,
// COUNT(CASE WHEN ph.PostHistoryTypeId IN (4, 5, 6) THEN 1 END) AS TitleEdits,
// COUNT(CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN 1 END) AS CloseReopenEdits
// FROM PostHistory ph
// GROUP BY ph.UserId
// ),
// CombinedStats AS (
// SELECT
// ups.UserId,
// ups.DisplayName,
// ups.TotalPosts,
// ups.TotalQuestions,
// ups.TotalAnswers,
// ups.TotalTagWikis,
// ups.TotalScore,
// ups.LastPostDate,
// phs.TotalEdits,
// phs.TotalEditedPosts,
// phs.TitleEdits,
// phs.CloseReopenEdits
// FROM UserPostStats ups
// LEFT JOIN PostHistoryStats phs ON ups.UserId = phs.UserId
// )
// SELECT
// UserId,
// DisplayName,
// TotalPosts,
// TotalQuestions,
// TotalAnswers,
// TotalTagWikis,
// TotalScore,
// LastPostDate,
// COALESCE(TotalEdits, 0) AS TotalEdits,
// COALESCE(TotalEditedPosts, 0) AS TotalEditedPosts,
// COALESCE(TitleEdits, 0) AS TitleEdits,
// COALESCE(CloseReopenEdits, 0) AS CloseReopenEdits
// FROM CombinedStats
// ORDER BY TotalScore DESC, TotalPosts DESC;
fn q11032(db: &'static So) -> String {
    let Post { post_type_id, score, creation_date, .. } = &db.post;
    let uf = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(score).and(creation_date)).fold([0, 0, 0, 0, 0, i64::MIN], |a: [i64; 6], ((t, s), c)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + matches!(t, 4 | 5) as i64, a[4] + s, a[5].max(c)]
    });
    let ph = db.post_history.group_by(&db.post_history.user).select(&db.post_history.post_history_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + matches!(t, 4 | 5 | 6) as i64, a[2] + matches!(t, 10 | 11) as i64]);
    let pd = db.post_history.group_by(&db.post_history.user).select(&db.post_history.post).count_distinct();
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&uf).opt()).and((&ph).opt()).and((&pd).opt())).drive(|_, (((u, a), h), d)| v.push((u, a, h.unwrap_or([0; 3]), d.unwrap_or(0))));
    rows(v.iter().map(|&(u, a, h, d)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(match a {
            Some(a) => vec![V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4]), V::T(a[5])],
            None => vec![V::I(0), V::I(0), V::I(0), V::I(0), V::Null, V::Null],
        });
        f.extend(ints(&[h[0], d, h[1], h[2]]));
        row(f)
    }))
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(DISTINCT P.Id) AS TotalPosts,
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
// WHERE
// U.Reputation > 1000
// GROUP BY
// U.Id, U.DisplayName, U.Reputation
// ), BadgeStats AS (
// SELECT
// B.UserId,
// COUNT(CASE WHEN B.Class = 1 THEN 1 END) AS GoldBadges,
// COUNT(CASE WHEN B.Class = 2 THEN 1 END) AS SilverBadges,
// COUNT(CASE WHEN B.Class = 3 THEN 1 END) AS BronzeBadges
// FROM
// Badges B
// GROUP BY
// B.UserId
// ), CombinedStats AS (
// SELECT
// US.UserId,
// US.DisplayName,
// US.Reputation,
// US.TotalPosts,
// US.TotalAnswers,
// US.TotalQuestions,
// US.TotalUpvotes,
// US.TotalDownvotes,
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
// TotalPosts,
// TotalQuestions,
// TotalAnswers,
// TotalUpvotes,
// TotalDownvotes,
// GoldBadges,
// SilverBadges,
// BronzeBadges
// FROM
// CombinedStats
// ORDER BY
// TotalPosts DESC, Reputation DESC
// LIMIT 10;
fn q6663(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::RepGt(1000), "v", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&us).and((&dp).opt()).and((&bc).opt()).drive(|u, ((a, d), b)| v.push((u, a, d.unwrap_or(0), bz(b))));
    out(v, |&(u, _, d, _)| (Reverse(d), rep_desc(db, u)), 10, |&(u, a, d, b)| {
        let mut f = ["uid", "name", "rep"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.extend(ints(&[d, a.q, a.a, a.up, a.down, b[1], b[2], b[3]]));
        f
    })
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(DISTINCT B.Id) AS BadgeCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
// FROM Users U
// LEFT JOIN Badges B ON U.Id = B.UserId
// LEFT JOIN Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN Votes V ON P.Id = V.PostId AND V.UserId = U.Id
// WHERE U.Reputation > 0
// GROUP BY U.Id, U.DisplayName, U.Reputation
// ), UnclosedQuestions AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// U.DisplayName AS OwnerDisplayName,
// P.Score,
// P.ViewCount
// FROM Posts P
// JOIN Users U ON P.OwnerUserId = U.Id
// WHERE P.PostTypeId = 1 AND P.ClosedDate IS NULL
// ), TopVotedQuestions AS (
// SELECT
// P.Id AS PostId,
// COUNT(V.Id) AS VoteCount
// FROM Votes V
// JOIN Posts P ON V.PostId = P.Id
// WHERE P.PostTypeId = 1
// GROUP BY P.Id
// ORDER BY VoteCount DESC
// LIMIT 10
// )
// SELECT
// U.UserId,
// U.DisplayName,
// U.Reputation,
// U.BadgeCount,
// U.QuestionCount,
// U.AnswerCount,
// U.UpVoteCount,
// U.DownVoteCount,
// UQ.Title AS UnclosedQuestionTitle,
// UQ.Score AS UnclosedQuestionScore,
// UQ.ViewCount AS UnclosedQuestionViewCount,
// TQV.VoteCount AS TopVotedQuestionVotes
// FROM UserStats U
// LEFT JOIN UnclosedQuestions UQ ON UQ.OwnerDisplayName = U.DisplayName
// LEFT JOIN TopVotedQuestions TQV ON UQ.PostId = TQV.PostId
// ORDER BY U.Reputation DESC, U.BadgeCount DESC;
fn q7954(db: &'static So) -> String {
    let sv = self_votes(db);
    let bu = badges_per_user(db);
    let uf = user_base(db, UserWhere::RepGt(0))
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(posts_of(db).select((&db.post.post_type_id).and((&sv).select(&db.vote.vote_type_id).opt())).opt()))
        .fold([0i64; 4], |a, (_, p)| {
            let (t, v) = p.map_or((0, None), |(t, v)| (t, v));
            [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64]
        });
    let tv_base = questions_only(db);
    let vp = votes_per_post(db);
    let top: MatSet<Id<Post>> = whole(&tv_base)
        .with(&vp)
        .select(Ident::<Post>::new().and(&vp))
        .filt(|(_, n): (Id<Post>, i64)| n > 0)
        .window(row_number, |(_, n): (Id<Post>, i64)| n, desc)
        .filt(|(_, r)| r <= 10)
        .map(|((p, _), _)| p)
        .collect();
    let uq: HashIdx<Str, Id<Post>> = questions_only(db).with((&db.post.closed_date).opt().filt(|c: Option<i64>| c.is_none())).select((&db.post.owner_user).select(&db.user.display_name)).inv().collect();
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(0))
        .select(Ident::<User>::new().and(&bu).and((&uf).opt()).and((&db.user.display_name).select(&uq).select(Ident::<Post>::new().and((&top).select(&vp).opt())).opt()))
        .drive(|_, x| v.push(x));
    rows(v.iter().map(|&(((u, b), a), q)| {
        let a = a.unwrap_or([0; 4]);
        let mut f = ["uid", "name", "rep"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.push(V::I(b));
        f.extend(ints(&a));
        f.extend(match q {
            Some((p, t)) => {
                let mut g = post_fields(db, p, &["title", "score", "views"]);
                g.push(oint(t));
                g
            }
            None => nulls(4),
        });
        row(f)
    }))
}

// WITH UserBadgeStats AS (
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
// PostActivity AS (
// SELECT
// p.OwnerUserId AS UserId,
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
// UserPerformanceStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COALESCE(ubs.TotalBadges, 0) AS TotalBadges,
// COALESCE(pas.TotalPosts, 0) AS TotalPosts,
// COALESCE(pas.Questions, 0) AS Questions,
// COALESCE(pas.Answers, 0) AS Answers,
// COALESCE(pas.TotalViews, 0) AS TotalViews,
// COALESCE(pas.TotalScore, 0) AS TotalScore
// FROM
// Users u
// LEFT JOIN
// UserBadgeStats ubs ON u.Id = ubs.UserId
// LEFT JOIN
// PostActivity pas ON u.Id = pas.UserId
// )
// SELECT
// ups.UserId,
// ups.DisplayName,
// ups.Reputation,
// ups.TotalBadges,
// ups.TotalPosts,
// ups.Questions,
// ups.Answers,
// ups.TotalViews,
// ups.TotalScore
// FROM
// UserPerformanceStats ups
// ORDER BY
// ups.Reputation DESC,
// ups.TotalScore DESC
// LIMIT 10;
fn q9469(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let ps = pstat(db, since(db, year_ago()));
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and(&bu).and((&ps).opt())).drive(|_, ((u, b), p)| v.push((u, b, pz(p))));
    out(v, |&(u, _, p)| (rep_desc(db, u), Reverse(p[3])), 10, |&(u, b, p)| {
        let mut f = ["uid", "name", "rep"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.extend(ints(&[b, p[0], p[1], p[2], p[5], p[3]]));
        f
    })
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(CASE WHEN p.PostTypeId = 3 THEN 1 ELSE 0 END) AS Wikis,
// SUM(p.ViewCount) AS TotalViews,
// SUM(p.Score) AS TotalScore
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// GROUP BY u.Id, u.Reputation
// ),
// BadgeStats AS (
// SELECT
// b.UserId,
// COUNT(b.Id) AS TotalBadges,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM Badges b
// GROUP BY b.UserId
// ),
// VoteStats AS (
// SELECT
// v.UserId,
// COUNT(v.Id) AS TotalVotes,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM Votes v
// GROUP BY v.UserId
// )
// SELECT
// us.UserId,
// us.Reputation,
// us.TotalPosts,
// us.Questions,
// us.Answers,
// us.Wikis,
// us.TotalViews,
// us.TotalScore,
// COALESCE(bs.TotalBadges, 0) AS TotalBadges,
// COALESCE(bs.GoldBadges, 0) AS GoldBadges,
// COALESCE(bs.SilverBadges, 0) AS SilverBadges,
// COALESCE(bs.BronzeBadges, 0) AS BronzeBadges,
// COALESCE(vs.TotalVotes, 0) AS TotalVotes,
// COALESCE(vs.UpVotes, 0) AS UpVotes,
// COALESCE(vs.DownVotes, 0) AS DownVotes
// FROM UserStats us
// LEFT JOIN BadgeStats bs ON us.UserId = bs.UserId
// LEFT JOIN VoteStats vs ON us.UserId = vs.UserId
// ORDER BY us.Reputation DESC, us.TotalPosts DESC;
fn q5037(db: &'static So) -> String {
    let ps = pstat(db, db.post.iq());
    let bc = badge_classes(db);
    let uv = uvotes(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&ps).opt()).and((&bc).opt()).and((&uv).opt())).drive(|_, (((u, p), b), x)| v.push((u, pz(p), bz(b), x.unwrap_or([0; 3]))));
    rows(v.iter().map(|&(u, p, b, x)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep"), V::I(p[0]), V::I(p[1]), V::I(p[2]), V::I(p[8]), pviews(p), nullable(p[3], p[0])];
        f.extend(ints(&b));
        f.extend(ints(&x));
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
// COUNT(DISTINCT v.Id) AS VoteCount,
// AVG(c.Score) AS AverageCommentScore
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// WHERE
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount,
// p.CommentCount, p.FavoriteCount, u.DisplayName
// ),
// UserBadgeStats AS (
// SELECT
// b.UserId,
// COUNT(b.Id) AS BadgeCount,
// MAX(b.Class) AS HighestBadgeClass
// FROM
// Badges b
// GROUP BY
// b.UserId
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
// ps.FavoriteCount,
// ps.OwnerDisplayName,
// ps.VoteCount,
// ps.AverageCommentScore,
// ubs.BadgeCount,
// ubs.HighestBadgeClass
// FROM
// PostStats ps
// LEFT JOIN
// UserBadgeStats ubs ON ps.OwnerDisplayName = (SELECT DisplayName FROM Users WHERE Id = ubs.UserId)
// )
// SELECT
// *,
// CASE
// WHEN HighestBadgeClass = 1 THEN 'Gold'
// WHEN HighestBadgeClass = 2 THEN 'Silver'
// WHEN HighestBadgeClass = 3 THEN 'Bronze'
// ELSE 'No Badge'
// END AS HighestBadge
// FROM
// CombinedStats
// ORDER BY
// Score DESC, ViewCount DESC
// LIMIT 100;
fn q8140(db: &'static So) -> String {
    let bs = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold((0i64, i64::MIN), |(n, m), c| (n + 1, m.max(c)));
    let bn: HashIdx<Str, Id<User>> = db.user.with(&bs).select(&db.user.display_name).inv().collect();
    let vp = votes_per_post(db);
    let ca = db.comment.group_by(&db.comment.post).select(&db.comment.score).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let mut v = Vec::new();
    owned_since(db, year_ago())
        .select(Ident::<Post>::new().and(&vp).and((&ca).opt()).and((&db.post.owner_user).select(&db.user.display_name).select(&bn).select(&bs).opt()))
        .drive(|_, x| v.push(x));
    out(v, |&(((p, _), _), b)| (score_views(db, p), db.post.origid.get(p).unwrap(), (b.is_none(), b)), 100, |&(((p, x), c), b)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers", "comments", "favorites", "owner"]);
        f.push(V::I(x));
        f.push(c.map_or(V::Null, |c| avg(c[1], c[0])));
        f.extend(match b {
            Some((n, m)) => vec![V::I(n), V::I(m)],
            None => nulls(2),
        });
        f.push(V::S(match b.map(|b| b.1) {
            Some(1) => "Gold",
            Some(2) => "Silver",
            Some(3) => "Bronze",
            _ => "No Badge",
        }));
        f
    })
}

// WITH PostStats AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
// COUNT(CASE WHEN V.Id IS NOT NULL THEN 1 END) AS VoteCount,
// AVG(V.BountyAmount) AS AverageBounty,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
// P.Score,
// P.ViewCount
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.Score, P.ViewCount
// ),
// UserStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(DISTINCT P.Id) AS PostCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes,
// AVG(U.Reputation) AS AverageReputation
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// U.Id, U.DisplayName
// )
// SELECT
// PS.PostId,
// PS.Title,
// PS.CreationDate,
// PS.CommentCount,
// PS.VoteCount,
// PS.AverageBounty,
// PS.UpVoteCount,
// PS.DownVoteCount,
// PS.Score,
// PS.ViewCount,
// US.UserId,
// US.DisplayName AS OwnerDisplayName,
// US.PostCount AS UserPostCount,
// US.TotalUpVotes AS UserTotalUpVotes,
// US.TotalDownVotes AS UserTotalDownVotes,
// US.AverageReputation AS UserAverageReputation
// FROM
// PostStats PS
// JOIN
// Users U ON PS.PostId = U.Id
// JOIN
// UserStats US ON U.Id = US.UserId
// ORDER BY
// PS.Score DESC, PS.ViewCount DESC;
fn q12044(db: &'static So) -> String {
    let uid = uids(db);
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let mut v = Vec::new();
    stats_fold(db, db.post.with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cv", &[])
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&us).and((&dp).opt())))
        .drive(|p, (s, ((u, a), d))| v.push((p, s, u, a, d.unwrap_or(0))));
    rows(v.iter().map(|&(p, s, u, a, d)| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(s.cx), V::I(s.vx), avg(s.bounty_sum, s.bounty_n), V::I(s.up), V::I(s.down)]);
        f.extend(post_fields(db, p, &["score", "views"]));
        f.extend([user_col(db, u, "uid"), user_col(db, u, "name")]);
        f.extend(ints(&[d, a.up, a.down]));
        f.push(V::F(db.user.reputation.get(u).unwrap() as f64));
        row(f)
    }))
}

// WITH UserPostStatistics AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN p.PostTypeId = 2 AND p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers,
// SUM(CASE WHEN c.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalComments,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN Comments c ON p.Id = c.PostId
// LEFT JOIN Votes v ON p.Id = v.PostId
// GROUP BY u.Id, u.DisplayName
// ),
// BadgeStatistics AS (
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
// ups.UserId,
// ups.DisplayName,
// ups.TotalPosts,
// ups.TotalQuestions,
// ups.TotalAnswers,
// ups.AcceptedAnswers,
// ups.TotalComments,
// ups.TotalUpvotes,
// ups.TotalDownvotes,
// COALESCE(bs.TotalBadges, 0) AS TotalBadges,
// COALESCE(bs.GoldBadges, 0) AS GoldBadges,
// COALESCE(bs.SilverBadges, 0) AS SilverBadges,
// COALESCE(bs.BronzeBadges, 0) AS BronzeBadges,
// (ups.TotalUpvotes - ups.TotalDownvotes) AS ReputationScore
// FROM UserPostStatistics ups
// LEFT JOIN BadgeStatistics bs ON ups.UserId = bs.UserId
// ORDER BY ReputationScore DESC
// FETCH FIRST 10 ROWS ONLY;
fn q26984(db: &'static So) -> String {
    let Post { post_type_id, accepted_answer_id, .. } = &db.post;
    let uf = owned(db)
        .group_by(&db.post.owner_user)
        .select(post_type_id.and(accepted_answer_id.opt()).and(comments_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()))
        .fold([0i64; 7], |a, (((t, acc), c), v)| {
            [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (t == 2 && acc.is_some()) as i64, a[4] + c.is_some() as i64, a[5] + (v == Some(2)) as i64, a[6] + (v == Some(3)) as i64]
        });
    let bc = badge_classes(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&uf).opt()).and((&bc).opt())).drive(|_, ((u, a), b)| v.push((u, a.unwrap_or([0; 7]), bz(b))));
    out(v, |&(_, a, _)| Reverse(a[5] - a[6]), 10, |&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a));
        f.extend(ints(&b));
        f.push(V::I(a[5] - a[6]));
        f
    })
}

// WITH UserStatistics AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(DISTINCT P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN P.Score >= 0 THEN 1 ELSE 0 END) AS NonNegativeScorePosts,
// SUM(V.BountyAmount) AS TotalBounty
// FROM Users U
// LEFT JOIN Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId IN (8, 9)
// GROUP BY U.Id, U.DisplayName, U.Reputation
// ), UserBadges AS (
// SELECT
// B.UserId,
// COUNT(CASE WHEN B.Class = 1 THEN 1 END) AS GoldBadges,
// COUNT(CASE WHEN B.Class = 2 THEN 1 END) AS SilverBadges,
// COUNT(CASE WHEN B.Class = 3 THEN 1 END) AS BronzeBadges
// FROM Badges B
// GROUP BY B.UserId
// ), CombinedUserStats AS (
// SELECT
// US.UserId,
// US.DisplayName,
// US.Reputation,
// US.TotalPosts,
// US.TotalQuestions,
// US.TotalAnswers,
// US.NonNegativeScorePosts,
// US.TotalBounty,
// UB.GoldBadges,
// UB.SilverBadges,
// UB.BronzeBadges
// FROM UserStatistics US
// LEFT JOIN UserBadges UB ON US.UserId = UB.UserId
// )
// SELECT
// CUS.UserId,
// CUS.DisplayName,
// CUS.Reputation,
// CUS.TotalPosts,
// CUS.TotalQuestions,
// CUS.TotalAnswers,
// CUS.NonNegativeScorePosts,
// CUS.TotalBounty,
// COALESCE(CUS.GoldBadges, 0) AS GoldBadges,
// COALESCE(CUS.SilverBadges, 0) AS SilverBadges,
// COALESCE(CUS.BronzeBadges, 0) AS BronzeBadges
// FROM CombinedUserStats CUS
// WHERE CUS.Reputation > 1000
// ORDER BY CUS.TotalPosts DESC, CUS.Reputation DESC
// LIMIT 10;
fn q5163(db: &'static So) -> String {
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).filt(|t: i64| matches!(t, 8 | 9)))).select((&db.vote.bounty_amount).opt());
    let Post { post_type_id, score, .. } = &db.post;
    let uf = user_base(db, UserWhere::RepGt(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(bounty.opt())).opt())
        .fold([0i64; 6], |a, p| match p {
            Some(((t, s), b)) => {
                let b = b.flatten();
                [0, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (s >= 0) as i64, a[4] + b.is_some() as i64, a[5] + b.unwrap_or(0)]
            }
            None => a,
        });
    let np = ud(db, UserWhere::All, posts_of(db));
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&uf).and((&np).opt()).and((&bc).opt()).drive(|u, ((a, n), b)| v.push((u, [n.unwrap_or(0), a[1], a[2], a[3], a[4], a[5]], bz(b))));
    out(v, |&(u, a, _)| (Reverse(a[0]), rep_desc(db, u)), 10, |&(u, a, b)| {
        let mut f = ["uid", "name", "rep"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.extend(ints(&[a[0], a[1], a[2], a[3]]));
        f.push(nullable(a[5], a[4]));
        f.extend(ints(&[b[1], b[2], b[3]]));
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
// SUM(P.Score) AS TotalScore,
// SUM(P.ViewCount) AS TotalViews
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
// COUNT(PH.Id) AS HistoryCount,
// SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END) AS PositiveScoreUpdates
// FROM
// PostHistory PH
// JOIN
// Posts P ON PH.PostId = P.Id
// GROUP BY
// PH.UserId
// )
// SELECT
// US.UserId,
// US.Reputation,
// US.PostCount,
// US.QuestionCount,
// US.AnswerCount,
// US.TotalScore,
// US.TotalViews,
// COALESCE(BS.BadgeCount, 0) AS BadgeCount,
// COALESCE(BS.GoldBadges, 0) AS GoldBadges,
// COALESCE(BS.SilverBadges, 0) AS SilverBadges,
// COALESCE(BS.BronzeBadges, 0) AS BronzeBadges,
// COALESCE(PHS.HistoryCount, 0) AS HistoryCount,
// COALESCE(PHS.PositiveScoreUpdates, 0) AS PositiveScoreUpdates
// FROM
// UserStats US
// LEFT JOIN
// BadgeStats BS ON US.UserId = BS.UserId
// LEFT JOIN
// PostHistoryStats PHS ON US.UserId = PHS.UserId
// ORDER BY
// US.TotalScore DESC, US.Reputation DESC;
fn q11581(db: &'static So) -> String {
    let ps = pstat(db, db.post.iq());
    let bc = badge_classes(db);
    let ph = db.post_history.group_by(&db.post_history.user).select((&db.post_history.post).select(&db.post.score)).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + (s > 0) as i64]);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&ps).opt()).and((&bc).opt()).and((&ph).opt())).drive(|_, (((u, p), b), h)| v.push((u, pz(p), bz(b), h.unwrap_or([0; 2]))));
    rows(v.iter().map(|&(u, p, b, h)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep"), V::I(p[0]), V::I(p[1]), V::I(p[2]), nullable(p[3], p[0]), pviews(p)];
        f.extend(ints(&b));
        f.extend(ints(&h));
        row(f)
    }))
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// U.CreationDate,
// COUNT(DISTINCT P.Id) AS PostCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(CASE WHEN P.ViewCount IS NOT NULL THEN P.ViewCount ELSE 0 END) AS TotalViews,
// SUM(CASE WHEN P.Score IS NOT NULL THEN P.Score ELSE 0 END) AS TotalScore
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id, U.DisplayName, U.Reputation, U.CreationDate
// ),
// PostActivity AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.LastActivityDate,
// PH.PostHistoryTypeId,
// PH.CreationDate AS HistoryDate,
// PH.UserDisplayName,
// PH.UserId,
// COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount
// FROM
// Posts P
// LEFT JOIN
// PostHistory PH ON P.Id = PH.PostId
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.LastActivityDate, PH.PostHistoryTypeId, PH.CreationDate, PH.UserDisplayName, PH.UserId
// )
// SELECT
// US.UserId,
// US.DisplayName,
// US.Reputation,
// US.PostCount,
// US.Questions,
// US.Answers,
// US.TotalViews,
// US.TotalScore,
// PA.PostId,
// PA.Title,
// PA.CreationDate AS PostCreationDate,
// PA.LastActivityDate AS PostLastActivityDate,
// PA.PostHistoryTypeId,
// PA.HistoryDate,
// PA.UserDisplayName AS HistoryUserDisplayName,
// PA.CommentCount
// FROM
// UserStats US
// JOIN
// PostActivity PA ON PA.UserId = US.UserId
// ORDER BY
// US.Reputation DESC, PA.LastActivityDate DESC
// LIMIT 100;
fn q13186(db: &'static So) -> String {
    type K = ((((Id<Post>, i64), i64), Option<Str>), Id<User>);
    let ps = pstat(db, db.post.iq());
    let PostHistory { post, post_history_type_id, creation_date, user_display_name, user, .. } = &db.post_history;
    let pa = db.post_history
        .with(user)
        .group_by(post.and(post_history_type_id).and(creation_date).and(user_display_name.opt()).and(user))
        .select(post.select(comments_of(db)).opt())
        .fold(0i64, |a, c| a + c.is_some() as i64);
    let mut v = Vec::new();
    (&pa).and(Same::<K>::new().map(|(_, u)| u).select((&ps).opt())).drive(|k, (c, q)| v.push((k, c, pz(q))));
    let key = |&(((((p, t), d), nm), u), ..): &(K, i64, [i64; 13])| {
        (rep_desc(db, u), Reverse(db.post.last_activity_date.get(p).unwrap()), db.user.origid.get(u).unwrap(), db.post.origid.get(p).unwrap(), d, t, (nm.is_none(), nm))
    };
    out(v, key, 100, |&(((((p, t), d), nm), u), c, q)| {
        let mut f = ["uid", "name", "rep"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.extend(ints(&[q[0], q[1], q[2], q[5], q[3]]));
        f.extend(post_fields(db, p, &["id", "title", "created", "activity"]));
        f.extend([V::I(t), V::T(d), ostr(nm), V::I(c)]);
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
// COUNT(co.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Posts p
// LEFT JOIN
// Comments co ON p.Id = co.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount
// ),
// UserEngagement AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT b.Id) AS BadgeCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes,
// SUM(p.ViewCount) AS TotalViews,
// SUM(ps.Score) AS TotalPostScore
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// PostStats ps ON p.Id = ps.PostId
// GROUP BY
// u.Id, u.DisplayName
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.Score AS PostScore,
// ps.ViewCount AS PostViews,
// ps.AnswerCount,
// ps.CommentCount,
// ue.UserId,
// ue.DisplayName,
// ue.BadgeCount,
// ue.TotalUpVotes,
// ue.TotalDownVotes,
// ue.TotalViews AS UserTotalViews,
// ue.TotalPostScore AS UserTotalPostScore
// FROM
// PostStats ps
// JOIN
// UserEngagement ue ON ps.PostId = ue.UserId
// ORDER BY
// ps.ViewCount DESC, ps.Score DESC;
fn q11756(db: &'static So) -> String {
    let uid = uids(db);
    let bu = badges_per_user(db);
    let y = year_ago();
    let Post { score, view_count, creation_date, .. } = &db.post;
    let tu: MatSet<Id<User>> = since(db, y).select((&db.post.origid).select(&uid)).collect();
    let recent = Ident::<Post>::new().with(creation_date.ge(y));
    let uf = (&tu)
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(posts_of(db).select(view_count.opt().and(recent.select(score).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt()))
        .fold([0i64; 6], |a, (_, p)| match p {
            Some(((w, s), t)) => [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + s.is_some() as i64, a[5] + s.unwrap_or(0)],
            None => a,
        });
    let mut v = Vec::new();
    stats_fold(db, since(db, y), Ident::<Post>::new(), "cv", &[])
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&bu).and(&uf)))
        .drive(|p, (s, ((u, b), a))| v.push((p, s, u, b, a)));
    rows(v.iter().map(|&(p, s, u, b, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers"]);
        f.push(V::I(s.cx));
        f.extend([user_col(db, u, "uid"), user_col(db, u, "name")]);
        f.extend(ints(&[b, a[0], a[1]]));
        f.extend([nullable(a[3], a[2]), nullable(a[5], a[4])]);
        row(f)
    }))
}

// WITH UserReputation AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(DISTINCT P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(CASE WHEN P.PostTypeId = 3 THEN 1 ELSE 0 END) AS Wikis
// FROM Users U
// LEFT JOIN Posts P ON U.Id = P.OwnerUserId
// GROUP BY U.Id, U.DisplayName, U.Reputation
// ),
// PostStatistics AS (
// SELECT
// P.Id AS PostId,
// P.OwnerUserId,
// P.CreationDate,
// P.Score,
// P.ViewCount,
// COUNT(Cmt.Id) AS CommentCount,
// COUNT(V.Id) AS VoteCount
// FROM Posts P
// LEFT JOIN Comments Cmt ON P.Id = Cmt.PostId
// LEFT JOIN Votes V ON P.Id = V.PostId
// GROUP BY P.Id, P.OwnerUserId, P.CreationDate, P.Score, P.ViewCount
// ),
// PostHistoryDetails AS (
// SELECT
// PH.PostId,
// PHT.Name AS HistoryType,
// COUNT(PH.Id) AS HistoryCount,
// MAX(PH.CreationDate) AS LastModified
// FROM PostHistory PH
// JOIN PostHistoryTypes PHT ON PH.PostHistoryTypeId = PHT.Id
// WHERE PH.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '3 months'
// GROUP BY PH.PostId, PHT.Name
// )
// SELECT
// UR.UserId,
// UR.DisplayName,
// UR.Reputation,
// UR.TotalPosts,
// UR.Questions,
// UR.Answers,
// UR.Wikis,
// PS.PostId,
// PS.CreationDate,
// PS.Score,
// PS.ViewCount,
// PS.CommentCount,
// PS.VoteCount,
// PHD.HistoryType,
// PHD.HistoryCount,
// PHD.LastModified
// FROM UserReputation UR
// JOIN PostStatistics PS ON UR.UserId = PS.OwnerUserId
// LEFT JOIN PostHistoryDetails PHD ON PS.PostId = PHD.PostId
// ORDER BY UR.Reputation DESC, PS.Score DESC, PS.CreationDate DESC;
fn q9841(db: &'static So) -> String {
    let dp = ud(db, UserWhere::All, posts_of(db));
    let tc = owned(db).group_by(&db.post.owner_user).select(&db.post.post_type_id).fold([0i64; 3], |a, t| [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64]);
    let d3 = ts(2024, 7, 1, 12, 34, 56);
    let PostHistory { post, post_history_type, creation_date, .. } = &db.post_history;
    let rh = || db.post_history.with(creation_date.ge(d3));
    let hn = post_history_type.select(&db.post_history_type.name);
    let hk: MatSet<(Id<Post>, Str)> = rh().select(post.and(hn)).collect();
    let hd = rh().group_by(post.and(post_history_type.select(&db.post_history_type.name))).select(creation_date).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let hi: HashIdx<Id<Post>, (Id<Post>, Str)> = (&hk).map(|(p, _)| p).inv().collect();
    let sf = stats_fold(db, owned(db), Ident::<Post>::new(), "cv", &[]);
    let mut v = Vec::new();
    (&sf)
        .and((&db.post.owner_user).select(Ident::<User>::new().and(&dp).and(&tc)))
        .and((&hi).select(Same::<(Id<Post>, Str)>::new().and(&hd)).opt())
        .drive(|p, ((s, ((u, d), t)), h)| v.push((p, s, u, d, t, h)));
    rows(v.iter().map(|&(p, s, u, d, t, h)| {
        let mut f = ["uid", "name", "rep"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.extend(ints(&[d, t[0], t[1], t[2]]));
        f.extend(post_fields(db, p, &["id", "created", "score", "views"]));
        f.extend(ints(&[s.cx, s.vx]));
        f.extend(match h {
            Some(((_, n), (c, m))) => vec![V::S(n), V::I(c), V::T(m)],
            None => nulls(3),
        });
        row(f)
    }))
}

// WITH PostEngagement AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.PostTypeId,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// COUNT(DISTINCT b.Id) AS BadgeCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// SUM(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Badges b ON p.OwnerUserId = b.UserId
// WHERE
// p.CreationDate >= DATE '2023-01-01'
// GROUP BY
// p.Id, p.Title, p.PostTypeId
// ),
// UserEngagement AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS PostsCount,
// SUM(CASE WHEN b.Id IS NOT NULL THEN 1 ELSE 0 END) AS BadgesCount,
// SUM(p.ViewCount) AS TotalViews,
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
// WHERE
// u.CreationDate >= DATE '2023-01-01'
// GROUP BY
// u.Id, u.DisplayName
// )
// SELECT
// e.PostId,
// e.Title,
// e.PostTypeId,
// e.CommentCount,
// e.VoteCount,
// e.UpVotes,
// e.DownVotes,
// e.AcceptedAnswers,
// u.UserId,
// u.DisplayName,
// u.PostsCount,
// u.BadgesCount,
// u.TotalViews
// FROM
// PostEngagement e
// JOIN
// UserEngagement u ON e.PostId = u.UserId
// ORDER BY
// e.CommentCount DESC;
fn q10359(db: &'static So) -> String {
    let uid = uids(db);
    let d0 = date(2023, 1, 1);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let Post { view_count, .. } = &db.post;
    let uf = db
        .user
        .with((&db.user.creation_date).ge(d0))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(view_count.opt().and(votes_of(db).opt())).opt().and(badges_of(db).opt()))
        .fold([0i64; 3], |a, (p, b)| {
            let w = p.and_then(|(w, _)| w);
            [a[0] + b.is_some() as i64, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0)]
        });
    let by_uid: HashIdx<i64, Id<Badge>> = (&db.badge.user_id).inv().collect();
    let pf = since(db, d0)
        .group_by(Ident::<Post>::new())
        .select((&db.post.accepted_answer_id).opt().and(comments_of(db).opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt()).and((&db.post.owner_user_id).select(&by_uid).opt()))
        .fold([0i64; 5], |a, (((acc, c), t), _)| [a[0] + c.is_some() as i64, a[1] + t.is_some() as i64, a[2] + (t == Some(2)) as i64, a[3] + (t == Some(3)) as i64, a[4] + acc.is_some() as i64]);
    let mut v = Vec::new();
    (&pf)
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and((&dp).opt()).and(&uf)))
        .drive(|p, (s, ((u, d), a))| v.push((p, s, u, d.unwrap_or(0), a)));
    rows(v.iter().map(|&(p, s, u, d, a)| {
        let mut f = post_fields(db, p, &["id", "title", "type_id"]);
        f.extend(ints(&s));
        f.extend([user_col(db, u, "uid"), user_col(db, u, "name")]);
        f.extend(ints(&[d, a[0]]));
        f.push(nullable(a[2], a[1]));
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
// SUM(P.Score) AS TotalScore,
// AVG(P.ViewCount) AS AverageViews,
// MAX(P.CreationDate) AS MostRecentPostDate
// FROM
// Posts P
// GROUP BY
// P.OwnerUserId
// ),
// ActiveUsers AS (
// SELECT
// U.Id,
// U.Reputation,
// U.LastAccessDate,
// COALESCE(ub.BadgeCount, 0) AS BadgeCount,
// COALESCE(ps.PostCount, 0) AS PostCount,
// COALESCE(ps.TotalScore, 0) AS TotalScore
// FROM
// Users U
// LEFT JOIN
// UserBadges ub ON U.Id = ub.UserId
// LEFT JOIN
// PostStats ps ON U.Id = ps.OwnerUserId
// WHERE
// U.LastAccessDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'
// )
// SELECT
// U.DisplayName,
// AU.Reputation,
// AU.BadgeCount,
// AU.PostCount,
// AU.TotalScore,
// CASE
// WHEN AU.TotalScore > 100 THEN 'High Performer'
// WHEN AU.TotalScore BETWEEN 50 AND 100 THEN 'Moderate Performer'
// ELSE 'Needs Improvement'
// END AS PerformanceCategory
// FROM
// ActiveUsers AU
// JOIN
// Users U ON AU.Id = U.Id
// LEFT JOIN
// Votes V ON U.Id = V.UserId
// GROUP BY
// U.DisplayName, AU.Reputation, AU.BadgeCount, AU.PostCount, AU.TotalScore
// ORDER BY
// AU.TotalScore DESC,
// U.DisplayName ASC;
fn q1287(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let ps = owned(db).group_by(&db.post.owner_user).select(&db.post.score).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let rows_set: MatSet<(Str, (i64, (i64, [i64; 2])))> = db.user
        .with((&db.user.last_access_date).ge(month_ago()))
        .select((&db.user.display_name).and((&db.user.reputation).and((&bu).and((&ps).opt().map(|p: Option<[i64; 2]>| p.unwrap_or([0; 2]))))))
        .collect();
    let mut v = Vec::new();
    (&rows_set).drive(|x, _| v.push(x));
    rows(v.iter().map(|&(n, (r, (b, p)))| {
        let cat = if p[1] > 100 { "High Performer" } else if (50..=100).contains(&p[1]) { "Moderate Performer" } else { "Needs Improvement" };
        row(vec![V::S(n), V::I(r), V::I(b), V::I(p[0]), V::I(p[1]), V::S(cat)])
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("26725", q26725),
    ("11836", q11836),
    ("8626", q8626),
    ("6263", q6263),
    ("7562", q7562),
    ("3290", q3290),
    ("5134", q5134),
    ("7475", q7475),
    ("1246", q1246),
    ("7673", q7673),
    ("12379", q12379),
    ("14644", q14644),
    ("8761", q8761),
    ("12854", q12854),
    ("4590", q4590),
    ("9903", q9903),
    ("25002", q25002),
    ("3922", q3922),
    ("5166", q5166),
    ("6987", q6987),
    ("10665", q10665),
    ("12320", q12320),
    ("5379", q5379),
    ("3067", q3067),
    ("7830", q7830),
    ("12841", q12841),
    ("140", q140),
    ("2421", q2421),
    ("10702", q10702),
    ("5638", q5638),
    ("10730", q10730),
    ("1276", q1276),
    ("10663", q10663),
    ("10529", q10529),
    ("7523", q7523),
    ("9396", q9396),
    ("9432", q9432),
    ("6256", q6256),
    ("6448", q6448),
    ("2074", q2074),
    ("7218", q7218),
    ("18", q18),
    ("9574", q9574),
    ("6337", q6337),
    ("7798", q7798),
    ("25805", q25805),
    ("8112", q8112),
    ("10842", q10842),
    ("11032", q11032),
    ("6663", q6663),
    ("7954", q7954),
    ("9469", q9469),
    ("5037", q5037),
    ("8140", q8140),
    ("12044", q12044),
    ("26984", q26984),
    ("5163", q5163),
    ("11581", q11581),
    ("13186", q13186),
    ("11756", q11756),
    ("9841", q9841),
    ("10359", q10359),
    ("1287", q1287),
];
