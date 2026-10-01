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

/// Per user over `Users LEFT JOIN Posts`: [rows, rows with a post, SUM(u.UpVotes), SUM(u.DownVotes)].
fn user_left_posts(db: &'static So) -> Fold<Id<User>, [i64; 4]> {
    db.user
        .group_by(Ident::<User>::new())
        .select((&db.user.up_votes).and(&db.user.down_votes).and(posts_of(db).opt()))
        .fold([0i64; 4], |a, ((u, d), p)| [a[0] + 1, a[1] + p.is_some() as i64, a[2] + u, a[3] + d])
}

/// Per user over `Users LEFT JOIN Posts LEFT JOIN Badges` (the product): [rows with a badge, SUM(b.Class)].
fn user_class_rows(db: &'static So) -> Fold<Id<User>, [i64; 2]> {
    db.user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 2], |a, (_, c)| [a[0] + c.is_some() as i64, a[1] + c.unwrap_or(0)])
}

/// Per user over `Users LEFT JOIN Posts LEFT JOIN Badges` (the product): [rows,
/// question rows, answer rows, rows with a badge, SUM(u.Reputation), rows with a post].
fn user_pb(db: &'static So) -> Fold<Id<User>, [i64; 6]> {
    db.user
        .group_by(Ident::<User>::new())
        .select((&db.user.reputation).and(posts_of(db).select(&db.post.post_type_id).opt()).and(badges_of(db).opt()))
        .fold([0i64; 6], |a, ((r, t), b)| {
            [a[0] + 1, a[1] + (t == Some(1)) as i64, a[2] + (t == Some(2)) as i64, a[3] + b.is_some() as i64, a[4] + r, a[5] + t.is_some() as i64]
        })
}

/// Per user over `Users LEFT JOIN Votes LEFT JOIN Badges` (the product): [up rows, down rows].
fn user_vb(db: &'static So) -> Fold<Id<User>, [i64; 2]> {
    db.user
        .group_by(Ident::<User>::new())
        .select(votes_by(db).select(&db.vote.vote_type_id).opt().and(badges_of(db).opt()))
        .fold([0i64; 2], |a, (t, _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64])
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

// --- batch 134 --------------------------------------------------------------

// WITH PostEngagement AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// COALESCE(a.AnswerCount, 0) AS AnswerCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// (SELECT
// ParentId,
// COUNT(*) AS AnswerCount
// FROM
// Posts
// WHERE
// PostTypeId = 2
// GROUP BY
// ParentId) a ON p.Id = a.ParentId
// WHERE
// p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '30 days'
// GROUP BY
// p.Id, p.Title, p.CreationDate, a.AnswerCount
// )
// SELECT
// pe.PostId,
// pe.Title,
// pe.CreationDate,
// pe.CommentCount,
// pe.VoteCount,
// pe.UpVotes,
// pe.DownVotes,
// pe.AnswerCount,
// CASE WHEN pe.VoteCount > 0 THEN ROUND((pe.UpVotes::decimal / pe.VoteCount) * 100, 2) ELSE 0 END AS UpVotePercentage
// FROM
// PostEngagement pe
// ORDER BY
// pe.VoteCount DESC, pe.CreationDate DESC;
fn q10519(db: &'static So) -> String {
    let ta = typed_answers_per_post(db);
    let mut v = Vec::new();
    stats_fold(db, since(db, date(2024, 9, 1)), Ident::<Post>::new(), "cv", &[]).and(&ta).drive(|p, (s, a)| v.push((p, s, a)));
    rows(v.iter().map(|&(p, s, a)| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(s.cx), V::I(s.vx), V::I(s.up), V::I(s.down), V::I(a), V::F(if s.vx > 0 { round2(s.up as f64 / s.vx as f64 * 100.0) } else { 0.0 })]);
        row(f)
    }))
}

// WITH PostVoteSummary AS (
// SELECT
// p.PostTypeId,
// COUNT(v.Id) AS TotalVotes,
// COUNT(DISTINCT p.OwnerUserId) AS UniqueUsers,
// SUM(u.Reputation) AS TotalReputation
// FROM
// Posts p
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// GROUP BY
// p.PostTypeId
// ),
// PostCount AS (
// SELECT
// PostTypeId,
// COUNT(*) AS PostCount
// FROM
// Posts
// GROUP BY
// PostTypeId
// )
// SELECT
// pt.Id AS PostTypeId,
// pt.Name AS PostTypeName,
// COALESCE(pvs.TotalVotes, 0) AS TotalVotes,
// COALESCE(pvs.UniqueUsers, 0) AS UniqueUsers,
// COALESCE(pvs.TotalReputation, 0) AS TotalReputation,
// COALESCE(pc.PostCount, 0) AS PostCount,
// CASE
// WHEN COALESCE(pc.PostCount, 0) > 0 THEN COALESCE(pvs.TotalVotes, 0) * 1.0 / pc.PostCount
// ELSE 0
// END AS AvgVotesPerPost,
// CASE
// WHEN COALESCE(pvs.UniqueUsers, 0) > 0 THEN COALESCE(pvs.TotalReputation, 0) * 1.0 / pvs.UniqueUsers
// ELSE 0
// END AS AvgReputationPerUser
// FROM
// PostTypes pt
// LEFT JOIN
// PostVoteSummary pvs ON pt.Id = pvs.PostTypeId
// LEFT JOIN
// PostCount pc ON pt.Id = pc.PostTypeId
// ORDER BY
// pt.Id;
fn q14858(db: &'static So) -> String {
    let f = db.post.group_by(&db.post.post_type).select(votes_of(db).opt().and((&db.post.owner_user).select(&db.user.reputation).opt())).fold([0i64; 2], |a, (v, r)| [a[0] + v.is_some() as i64, a[1] + r.unwrap_or(0)]);
    let du = db.post.group_by(&db.post.post_type).select(&db.post.owner_user).count_distinct();
    let pc = db.post.group_by(&db.post.post_type).select(&db.post.score).fold(0i64, |a, _| a + 1);
    let mut v = Vec::new();
    db.post_type.select(Ident::<PostType>::new().and((&f).opt()).and((&du).opt()).and((&pc).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(((t, f), d), c)| {
        let f = f.unwrap_or([0; 2]);
        let d = d.unwrap_or(0);
        let c = c.unwrap_or(0);
        row(vec![
            V::I(db.post_type.origid.get(t).unwrap()),
            V::S(db.post_type.name.get(t).unwrap()),
            V::I(f[0]),
            V::I(d),
            V::I(f[1]),
            V::I(c),
            V::F(if c > 0 { f[0] as f64 / c as f64 } else { 0.0 }),
            V::F(if d > 0 { f[1] as f64 / d as f64 } else { 0.0 }),
        ])
    }))
}

// WITH PostStats AS (
// SELECT
// P.Id AS PostId,
// P.Score,
// P.ViewCount,
// COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
// COUNT(CASE WHEN V.Id IS NOT NULL THEN 1 END) AS VoteCount
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// WHERE
// P.CreationDate >= '2023-01-01'
// GROUP BY
// P.Id, P.Score, P.ViewCount
// ),
// UserStats AS (
// SELECT
// U.Id AS UserId,
// U.Reputation,
// COUNT(DISTINCT B.Id) AS BadgeCount,
// SUM(P.ViewCount) AS TotalViews,
// AVG(P.Score) AS AveragePostScore
// FROM
// Users U
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// WHERE
// U.CreationDate >= '2023-01-01'
// GROUP BY
// U.Id, U.Reputation
// )
// SELECT
// PS.PostId,
// PS.Score,
// PS.ViewCount,
// PS.CommentCount,
// PS.VoteCount,
// US.UserId,
// US.Reputation,
// US.BadgeCount,
// US.TotalViews,
// US.AveragePostScore
// FROM
// PostStats PS
// JOIN
// Users U ON PS.PostId = U.Id
// JOIN
// UserStats US ON U.Id = US.UserId
// ORDER BY
// PS.Score DESC,
// US.Reputation DESC;
fn q11809(db: &'static So) -> String {
    let uid = uids(db);
    let us = user_bp(db, db.user.with((&db.user.creation_date).ge(date(2023, 1, 1))));
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    stats_fold(db, since(db, date(2023, 1, 1)), Ident::<Post>::new(), "cv", &[])
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&us).and(&bu)))
        .drive(|p, (s, ((u, x), b))| v.push((p, s, u, x, b)));
    rows(v.iter().map(|&(p, s, u, x, b)| {
        let mut f = post_fields(db, p, &["id", "score", "views"]);
        f.extend([V::I(s.cx), V::I(s.vx), user_col(db, u, "uid"), user_col(db, u, "rep"), V::I(b), nullable(x[5], x[4]), avg(x[3], x[2])]);
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
// SUM(COALESCE(v.BountyAmount, 0)) AS TotalBounties,
// AVG(COALESCE(CHAR_LENGTH(p.Body), 0)) AS AvgPostLength,
// MAX(p.CreationDate) AS LastPostDate
// FROM
// Users u
// LEFT JOIN
// Posts p ON p.OwnerUserId = u.Id
// LEFT JOIN
// Votes v ON v.UserId = u.Id
// WHERE
// u.Reputation > 0
// GROUP BY
// u.Id, u.Reputation
// ),
// PostHistoryStats AS (
// SELECT
// ph.UserId,
// COUNT(*) AS EditCount,
// COUNT(DISTINCT ph.PostId) AS EditedPostCount
// FROM
// PostHistory ph
// WHERE
// ph.PostHistoryTypeId IN (4, 5, 6)
// GROUP BY
// ph.UserId
// )
// SELECT
// us.UserId,
// us.Reputation,
// us.PostCount,
// us.QuestionCount,
// us.AnswerCount,
// us.TotalBounties,
// us.AvgPostLength,
// us.LastPostDate,
// phs.EditCount,
// phs.EditedPostCount
// FROM
// UserStats us
// LEFT JOIN
// PostHistoryStats phs ON us.UserId = phs.UserId
// ORDER BY
// us.Reputation DESC;
fn q14898(db: &'static So) -> String {
    let Post { post_type_id, body, creation_date, .. } = &db.post;
    let us = user_base(db, UserWhere::RepGt(0))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(body.map(chars)).and(creation_date)).opt().and(votes_by(db).select((&db.vote.bounty_amount).opt()).opt()))
        .fold([0, 0, 0, 0, 0, i64::MIN, 0], |a: [i64; 7], (p, b)| {
            let (t, c, d) = p.map_or((0, 0, i64::MIN), |((t, c), d)| (t, c, d));
            [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + b.flatten().unwrap_or(0), a[4] + c, a[5].max(d), a[6] + p.is_some() as i64]
        });
    let pn = user_distinct_posts(db);
    let hs = || db.post_history.with((&db.post_history.post_history_type_id).filt(|t: i64| matches!(t, 4 | 5 | 6)));
    let hn = hs().group_by(&db.post_history.user).select(&db.post_history.post).fold(0i64, |a, _| a + 1);
    let hd = hs().group_by(&db.post_history.user).select(&db.post_history.post).count_distinct();
    let mut v = Vec::new();
    (&us).and(&pn).and((&hn).opt()).and((&hd).opt()).drive(|u, x| v.push((u, x)));
    rows(v.iter().map(|&(u, (((a, n), h), d))| {
        row(vec![
            user_col(db, u, "uid"),
            user_col(db, u, "rep"),
            V::I(n),
            V::I(a[1]),
            V::I(a[2]),
            V::I(a[3]),
            V::F(a[4] as f64 / a[0] as f64),
            if a[6] == 0 { V::Null } else { V::T(a[5]) },
            oint(h),
            oint(d),
        ])
    }))
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
// UserPosts AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS PostCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount
// FROM
// Posts P
// GROUP BY
// P.OwnerUserId
// ),
// RecentComments AS (
// SELECT
// C.UserId,
// COUNT(C.Id) AS CommentCount
// FROM
// Comments C
// WHERE
// C.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'
// GROUP BY
// C.UserId
// )
// SELECT
// UB.UserId,
// UB.DisplayName,
// COALESCE(UP.PostCount, 0) AS TotalPosts,
// COALESCE(UP.QuestionCount, 0) AS TotalQuestions,
// COALESCE(UP.AnswerCount, 0) AS TotalAnswers,
// COALESCE(RC.CommentCount, 0) AS RecentComments,
// UB.BadgeCount
// FROM
// UserBadges UB
// LEFT JOIN
// UserPosts UP ON UB.UserId = UP.OwnerUserId
// LEFT JOIN
// RecentComments RC ON UB.UserId = RC.UserId
// ORDER BY
// UB.BadgeCount DESC, TotalPosts DESC, UB.DisplayName;
fn q6119(db: &'static So) -> String {
    let up = owned(db).group_by(&db.post.owner_user).select(&db.post.post_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64]);
    let rc = db.comment.with((&db.comment.creation_date).ge(month_ago())).group_by(&db.comment.user).select(&db.comment.score).fold(0i64, |a, _| a + 1);
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&up).opt()).and((&rc).opt()).and(&bu)).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(((u, p), c), b)| {
        let p = p.unwrap_or([0; 3]);
        row(vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(p[0]), V::I(p[1]), V::I(p[2]), V::I(c.unwrap_or(0)), V::I(b)])
    }))
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(DISTINCT P.Id) AS PostsCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM Users U
// LEFT JOIN Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN Votes V ON P.Id = V.PostId
// GROUP BY U.Id, U.DisplayName
// ),
// PostAverages AS (
// SELECT
// AVG(ViewCount) AS AvgViewCount,
// AVG(Score) AS AvgScore,
// AVG(AnswerCount) AS AvgAnswerCount,
// AVG(CommentCount) AS AvgCommentCount
// FROM Posts
// ),
// BadgeCounts AS (
// SELECT
// UserId,
// COUNT(*) AS BadgeCount
// FROM Badges
// GROUP BY UserId
// )
// SELECT
// U.DisplayName,
// U.PostsCount,
// U.QuestionsCount,
// U.AnswersCount,
// U.UpVotes,
// U.DownVotes,
// COALESCE(B.BadgeCount, 0) AS BadgeCount,
// P.AvgViewCount,
// P.AvgScore,
// P.AvgAnswerCount,
// P.AvgCommentCount
// FROM UserStats U
// CROSS JOIN PostAverages P
// LEFT JOIN BadgeCounts B ON U.UserId = B.UserId
// ORDER BY U.PostsCount DESC;
fn q13744(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let bu = badges_per_user(db);
    let Post { view_count, score, answer_count, comment_count, .. } = &db.post;
    let t = db.post.select(view_count.opt().and(score).and(answer_count.opt()).and(comment_count)).fold_flat([0i64; 7], |a, (((w, s), an), c)| {
        [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s, a[4] + an.is_some() as i64, a[5] + an.unwrap_or(0), a[6] + c]
    });
    let dp = ud(db, UserWhere::All, posts_of(db));
    let mut v = Vec::new();
    (&us).and(&bu).and((&dp).opt()).drive(|u, ((a, b), d)| v.push((u, a, b, d.unwrap_or(0))));
    rows(v.iter().map(|&(u, a, b, d)| row(vec![user_col(db, u, "name"), V::I(d), V::I(a.q), V::I(a.a), V::I(a.up), V::I(a.down), V::I(b), avg(t[2], t[1]), avg(t[3], t[0]), avg(t[5], t[4]), avg(t[6], t[0])])))
}

// WITH UserReputation AS (
// SELECT Id, Reputation
// FROM Users
// WHERE Reputation > 1000
// ),
// PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// COUNT(c.Id) AS TotalComments,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
// COUNT(DISTINCT p2.Id) AS LinkedPosts
// FROM Posts p
// LEFT JOIN Comments c ON p.Id = c.PostId
// LEFT JOIN Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (2, 3)
// LEFT JOIN PostLinks pl ON p.Id = pl.PostId
// LEFT JOIN Posts p2 ON pl.RelatedPostId = p2.Id
// GROUP BY p.Id, p.Title
// ),
// TopPosts AS (
// SELECT
// ps.PostId,
// ps.Title,
// ps.TotalComments,
// ps.UpVotes,
// ps.DownVotes,
// ps.LinkedPosts,
// u.Reputation AS UserReputation
// FROM PostStats ps
// JOIN Posts p ON ps.PostId = p.Id
// JOIN UserReputation u ON p.OwnerUserId = u.Id
// WHERE ps.UpVotes > ps.DownVotes
// ORDER BY ps.UpVotes DESC
// LIMIT 10
// )
// SELECT
// tp.PostId,
// tp.Title,
// tp.TotalComments,
// tp.UpVotes,
// tp.DownVotes,
// tp.LinkedPosts,
// tp.UserReputation
// FROM TopPosts tp
// ORDER BY tp.UserReputation DESC;
fn q8888(db: &'static So) -> String {
    let lp = db.post_link.group_by(&db.post_link.post).select(&db.post_link.related_post).count_distinct();
    let base = db.post.with((&db.post.owner_user).select(&db.user.reputation).filt(|r: i64| r > 1000));
    let mut v = Vec::new();
    stats_fold(db, base, Ident::<Post>::new(), "cvl", &[2, 3]).filt(|s: Stats| s.up > s.down).and((&lp).opt()).drive(|p, (s, l)| v.push((p, s, l.unwrap_or(0))));
    cut(&mut v, |&(_, s, _)| Reverse(s.up), 10);
    rows(v.iter().map(|&(p, s, l)| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([V::I(s.cx), V::I(s.up), V::I(s.down), V::I(l)]);
        f.extend(post_fields(db, p, &["rep"]));
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
// )
// SELECT
// U.UserId,
// U.Reputation,
// U.PostCount,
// U.QuestionCount,
// U.AnswerCount,
// U.TotalScore,
// U.TotalViews,
// COALESCE(B.BadgeCount, 0) AS BadgeCount,
// COALESCE(B.GoldBadges, 0) AS GoldBadges,
// COALESCE(B.SilverBadges, 0) AS SilverBadges,
// COALESCE(B.BronzeBadges, 0) AS BronzeBadges
// FROM
// UserStats U
// LEFT JOIN
// BadgeStats B ON U.UserId = B.UserId
// ORDER BY
// U.Reputation DESC;
fn q13177(db: &'static So) -> String {
    let ps = pstat(db, db.post.iq());
    let bc = badge_classes(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&ps).opt()).and((&bc).opt())).drive(|_, ((u, p), b)| v.push((u, pz(p), bz(b))));
    rows(v.iter().map(|&(u, p, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep"), V::I(p[0]), V::I(p[1]), V::I(p[2]), nullable(p[3], p[0]), pviews(p)];
        f.extend(ints(&b));
        row(f)
    }))
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// u.CreationDate,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS Questions,
// COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS Answers,
// SUM(COALESCE(p.Score, 0)) AS TotalScore,
// SUM(COALESCE(b.Class, 0)) AS TotalBadges
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// u.Id, u.DisplayName, u.Reputation, u.CreationDate
// ),
// PostHistoryCount AS (
// SELECT
// p.Id AS PostId,
// COUNT(ph.Id) AS RevisionCount
// FROM
// Posts p
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// GROUP BY
// p.Id
// )
// SELECT
// us.UserId,
// us.DisplayName,
// us.Reputation,
// us.CreationDate,
// us.TotalPosts,
// us.Questions,
// us.Answers,
// us.TotalScore,
// us.TotalBadges,
// COALESCE(phc.RevisionCount, 0) AS TotalRevisions
// FROM
// UserStats us
// LEFT JOIN
// PostHistoryCount phc ON us.UserId = phc.PostId
// WHERE
// us.Reputation > 1000
// ORDER BY
// us.Reputation DESC, us.TotalScore DESC
// LIMIT 50;
fn q5621(db: &'static So) -> String {
    let pid = pids(db);
    let ps = pstat(db, db.post.iq());
    let pb = user_posts_badges(db);
    let hp = history_per_post(db);
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000))
        .select(Ident::<User>::new().and((&ps).opt()).and(&pb).and((&db.user.origid).select(&pid).select(&hp).opt()))
        .drive(|_, (((u, p), c), h)| v.push((u, pz(p), c, h.unwrap_or(0))));
    out(v, |&(u, _, c, _)| (rep_desc(db, u), Reverse(c[1])), 50, |&(u, p, c, h)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep"), user_col(db, u, "ucreated")];
        f.extend(ints(&[p[0], p[1], p[2], c[1], c[5], h]));
        f
    })
}

// WITH PostDetails AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// pt.Name AS PostType,
// u.Reputation AS OwnerReputation
// FROM
// Posts p
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// ),
// CommentCounts AS (
// SELECT
// PostId,
// COUNT(*) AS TotalComments
// FROM
// Comments
// GROUP BY
// PostId
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
// pd.Score,
// pd.ViewCount,
// pd.PostType,
// pd.OwnerReputation,
// COALESCE(cc.TotalComments, 0) AS TotalComments,
// COALESCE(vc.UpVotes, 0) AS UpVotes,
// COALESCE(vc.DownVotes, 0) AS DownVotes
// FROM
// PostDetails pd
// LEFT JOIN
// CommentCounts cc ON pd.PostId = cc.PostId
// LEFT JOIN
// VoteCounts vc ON pd.PostId = vc.PostId
// ORDER BY
// pd.ViewCount DESC;
fn q11691(db: &'static So) -> String {
    let pv = post_votes(db);
    let cp = comments_per_post(db);
    let mut v = Vec::new();
    owned_since(db, year_ago()).select(Ident::<Post>::new().and(&cp).and((&pv).opt())).drive(|_, ((p, c), x)| v.push((p, c, x.unwrap_or([0; 3]))));
    rows(v.iter().map(|&(p, c, x)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "type", "rep"]);
        f.extend(ints(&[c, x[1], x[2]]));
        row(f)
    }))
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.PostTypeId,
// COUNT(c.Id) AS CommentCount,
// COUNT(DISTINCT v.UserId) AS UniqueVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// AVG(LENGTH(p.Body)) AS AvgPostLength,
// MAX(p.CreationDate) AS LastActivityDate
// FROM Posts p
// LEFT JOIN Comments c ON p.Id = c.PostId
// LEFT JOIN Votes v ON p.Id = v.PostId
// WHERE p.CreationDate BETWEEN '2022-01-01' AND '2023-12-31'
// GROUP BY p.Id, p.PostTypeId
// ),
// UserStats AS (
// SELECT
// u.Id AS UserId,
// COUNT(b.Id) AS BadgeCount,
// AVG(u.Reputation) AS AvgReputation,
// SUM(CASE WHEN p.ViewCount IS NOT NULL THEN p.ViewCount ELSE 0 END) AS TotalViews
// FROM Users u
// LEFT JOIN Badges b ON u.Id = b.UserId
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// GROUP BY u.Id
// )
// SELECT
// ps.PostId,
// ps.PostTypeId,
// ps.CommentCount,
// ps.UniqueVoteCount,
// ps.UpVotes,
// ps.DownVotes,
// ps.AvgPostLength,
// us.UserId,
// us.BadgeCount,
// us.AvgReputation,
// us.TotalViews
// FROM PostStats ps
// JOIN UserStats us ON ps.PostId = us.UserId
// ORDER BY ps.PostId
// LIMIT 100;
fn q13402(db: &'static So) -> String {
    let (lo, hi) = (date(2022, 1, 1), date(2023, 12, 31));
    let base = db.post.with((&db.post.creation_date).filt(move |d: i64| d >= lo && d <= hi));
    let dv = db.vote.group_by(&db.vote.post).select(&db.vote.user).count_distinct();
    let uid = uids(db);
    let ub = user_bp(db, db.user.iq());
    let mut v = Vec::new();
    stats_fold(db, base, Ident::<Post>::new(), "cv", &[])
        .and((&dv).opt())
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&ub)))
        .drive(|p, ((s, d), (u, a))| v.push((p, s, d.unwrap_or(0), u, a)));
    out(v, |&(p, ..)| db.post.origid.get(p).unwrap(), 100, |&(p, s, d, u, a)| {
        let mut f = post_fields(db, p, &["id", "type_id"]);
        f.extend([V::I(s.cx), V::I(d), V::I(s.up), V::I(s.down), V::F(chars(db.post.body.get(p).unwrap()) as f64), user_col(db, u, "uid"), V::I(a[1])]);
        f.push(V::F(db.user.reputation.get(u).unwrap() as f64));
        f.push(V::I(a[5]));
        f
    })
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
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
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.OwnerUserId
// ),
// UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS PostsCreated,
// SUM(COALESCE(b.Class, 0)) AS TotalBadgePoints
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
// us.PostsCreated,
// us.TotalBadgePoints
// FROM
// PostStats ps
// JOIN
// UserStats us ON ps.OwnerUserId = us.UserId
// ORDER BY
// ps.CreationDate DESC;
fn q13928(db: &'static So) -> String {
    let ps = pstat(db, db.post.iq());
    let pb = user_posts_badges(db);
    let mut v = Vec::new();
    stats_fold(db, owned_since(db, date(2023, 1, 1)), Ident::<Post>::new(), "cv", &[])
        .and((&db.post.owner_user).select(Ident::<User>::new().and(&ps).and(&pb)))
        .drive(|p, (s, ((u, x), c))| v.push((p, s, u, x, c)));
    rows(v.iter().map(|&(p, s, u, x, c)| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(s.cx), V::I(s.vx), V::I(s.up), V::I(s.down), user_col(db, u, "uid"), user_col(db, u, "name"), V::I(x[0]), V::I(c[5])]);
        row(f)
    }))
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(DISTINCT P.Id) AS PostCount,
// SUM(COALESCE(P.Score, 0)) AS TotalScore,
// SUM(COALESCE(P.ViewCount, 0)) AS TotalViews,
// COUNT(DISTINCT C.Id) AS CommentCount
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// GROUP BY
// U.Id, U.DisplayName, U.Reputation
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
// U.Reputation,
// U.PostCount,
// U.TotalScore,
// U.TotalViews,
// U.CommentCount,
// COALESCE(B.BadgeCount, 0) AS BadgeCount,
// COALESCE(B.GoldBadges, 0) AS GoldBadges,
// COALESCE(B.SilverBadges, 0) AS SilverBadges,
// COALESCE(B.BronzeBadges, 0) AS BronzeBadges
// FROM
// UserStats U
// LEFT JOIN
// BadgeStats B ON U.UserId = B.UserId
// ORDER BY
// U.Reputation DESC;
fn q14055(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "c", any_post);
    let bc = badge_classes(db);
    let mut v = Vec::new();
    let dp = ud(db, UserWhere::All, posts_of(db));
    (&us).and((&bc).opt()).and((&dp).opt()).drive(|u, ((a, b), d)| v.push((u, a, bz(b), d.unwrap_or(0))));
    rows(v.iter().map(|&(u, a, b, d)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep"), V::I(d)];
        f.extend(["score_sum0", "views_sum0", "#cx"].iter().map(|c| ustat_field(&a, c)));
        f.extend(ints(&b));
        row(f)
    }))
}

// WITH PostMetrics AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// p.AnswerCount,
// COALESCE(u.DisplayName, 'Community User') AS OwnerDisplayName,
// COUNT(DISTINCT c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= DATE('2024-10-01') - INTERVAL '1 year'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, u.DisplayName
// ),
// PostHistoryMetrics AS (
// SELECT
// ph.PostId,
// COUNT(ph.Id) AS RevisionCount,
// MAX(ph.CreationDate) AS LastEditedDate
// FROM
// PostHistory ph
// GROUP BY
// ph.PostId
// )
// SELECT
// pm.PostId,
// pm.Title,
// pm.CreationDate,
// pm.Score,
// pm.ViewCount,
// pm.AnswerCount,
// pm.CommentCount,
// pm.OwnerDisplayName,
// phm.RevisionCount,
// phm.LastEditedDate,
// pm.VoteCount
// FROM
// PostMetrics pm
// LEFT JOIN
// PostHistoryMetrics phm ON pm.PostId = phm.PostId
// ORDER BY
// pm.Score DESC, pm.ViewCount DESC
// LIMIT 100;
fn q13255(db: &'static So) -> String {
    let hf = history_n_max(db);
    let cp = comments_per_post(db);
    let vp = votes_per_post(db);
    let mut v = Vec::new();
    since(db, date(2023, 10, 1)).select(Ident::<Post>::new().and(&cp).and(&vp).and((&hf).opt())).drive(|_, x| v.push(x));
    out(v, |&(((p, _), _), _)| (score_views(db, p), db.post.origid.get(p).unwrap()), 100, |&(((p, c), x), h)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers"]);
        f.extend([V::I(c), named_owner(db, p, "Community User"), oint(h.map(|h| h.0)), ots(h.map(|h| h.1)), V::I(x)]);
        f
    })
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.Reputation,
// COUNT(DISTINCT P.Id) AS PostCount,
// SUM(COALESCE(P.Score, 0)) AS TotalScore,
// SUM(COALESCE(P.ViewCount, 0)) AS TotalViews,
// SUM(COALESCE(C.Score, 0)) AS TotalCommentScore,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount
// FROM Users U
// LEFT JOIN Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN Comments C ON P.Id = C.PostId
// GROUP BY U.Id, U.Reputation
// ),
// BadgeStats AS (
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
// U.Id AS UserId,
// U.DisplayName,
// US.PostCount,
// US.TotalScore,
// US.TotalViews,
// US.TotalCommentScore,
// US.QuestionCount,
// US.AnswerCount,
// BS.BadgeCount,
// BS.GoldBadges,
// BS.SilverBadges,
// BS.BronzeBadges
// FROM Users U
// JOIN UserStats US ON U.Id = US.UserId
// LEFT JOIN BadgeStats BS ON U.Id = BS.UserId
// ORDER BY US.TotalScore DESC
// LIMIT 10;
fn q10849(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "c", any_post);
    let bc = badge_classes(db);
    let mut v = Vec::new();
    let dp = ud(db, UserWhere::All, posts_of(db));
    (&us).and((&bc).opt()).and((&dp).opt()).drive(|u, ((a, b), d)| v.push((u, a, b, d.unwrap_or(0))));
    out(v, |&(_, a, _, _)| Reverse(a.score_sum), 10, |&(u, a, b, d)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(d)];
        f.extend(["score_sum0", "views_sum0", "cscore_sum0", "#q", "#a"].iter().map(|c| ustat_field(&a, c)));
        f.extend(b.map_or(nulls(4), |b| ints(&b)));
        f
    })
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// COUNT(c.Id) AS CommentCount,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
// COALESCE(SUM(CASE WHEN b.Id IS NOT NULL THEN 1 ELSE 0 END), 0) AS BadgeCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON c.PostId = p.Id
// LEFT JOIN
// Votes v ON v.PostId = p.Id
// LEFT JOIN
// Badges b ON b.UserId = p.OwnerUserId
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Id, p.Title
// ),
// UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS PostCount,
// SUM(u.UpVotes) AS UserUpVotes,
// SUM(u.DownVotes) AS UserDownVotes
// FROM
// Users u
// LEFT JOIN
// Posts p ON p.OwnerUserId = u.Id
// GROUP BY
// u.Id, u.DisplayName
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.CommentCount,
// ps.UpVotes,
// ps.DownVotes,
// us.UserId,
// us.DisplayName,
// us.PostCount,
// us.UserUpVotes,
// us.UserDownVotes
// FROM
// PostStats ps
// JOIN
// UserStats us ON ps.PostId = us.UserId
// ORDER BY
// ps.UpVotes DESC, ps.CommentCount DESC;
fn q14157(db: &'static So) -> String {
    let uid = uids(db);
    let lp = user_left_posts(db);
    let mut v = Vec::new();
    stats_fold(db, since(db, date(2023, 1, 1)), Ident::<Post>::new(), "cvb", &[])
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&lp)))
        .drive(|p, (s, (u, x))| v.push((p, s, u, x)));
    rows(v.iter().map(|&(p, s, u, x)| {
        let mut f = post_fields(db, p, &["id", "title"]);
        f.extend([V::I(s.cx), V::I(s.up), V::I(s.down), user_col(db, u, "uid"), user_col(db, u, "name"), V::I(x[1])]);
        f.extend([V::I(x[2]), V::I(x[3])]);
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
// SUM(v.BountyAmount) AS TotalBounties
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// u.Id, u.Reputation
// ),
// PostDetails AS (
// SELECT
// p.Id AS PostId,
// p.OwnerUserId,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// p.Tags,
// COALESCE(c.CommentCount, 0) AS CommentCount
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
// )
// SELECT
// us.UserId,
// us.Reputation,
// us.PostCount,
// us.QuestionCount,
// us.AnswerCount,
// us.TotalBounties,
// pd.PostId,
// pd.CreationDate,
// pd.Score,
// pd.ViewCount,
// pd.Tags,
// pd.CommentCount
// FROM
// UserStats us
// JOIN
// PostDetails pd ON us.UserId = pd.OwnerUserId
// ORDER BY
// us.Reputation DESC,
// pd.Score DESC;
fn q12610(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let cp = comments_per_post(db);
    let mut v = Vec::new();
    let dp = ud(db, UserWhere::All, posts_of(db));
    owned(db).select(Ident::<Post>::new().and(&cp).and((&db.post.owner_user).select(Ident::<User>::new().and(&us).and(&dp)))).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((p, c), ((u, a), d))| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep"), V::I(d)];
        f.extend(["#q", "#a", "bounty_sum"].iter().map(|c| ustat_field(&a, c)));
        f.extend(post_fields(db, p, &["id", "created", "score", "views", "tags"]));
        f.push(V::I(c));
        row(f)
    }))
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// COUNT(DISTINCT c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 1 THEN 1 ELSE 0 END), 0) AS AcceptedVote
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.Title, p.CreationDate
// ),
// UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(b.Class) AS TotalBadges
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
// us.TotalBadges
// FROM
// PostStats ps
// JOIN
// Users u ON ps.PostId = u.Id
// JOIN
// UserStats us ON u.Id = us.UserId
// ORDER BY
// ps.CreationDate DESC;
fn q12075(db: &'static So) -> String {
    let uid = uids(db);
    let ps = pstat(db, db.post.iq());
    let cs = user_class_rows(db);
    let mut v = Vec::new();
    stats_fold(db, db.post.with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cv", &[])
        .and(comments_per_post(db))
        .and(votes_per_post(db))
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and((&ps).opt()).and(&cs)))
        .drive(|p, (((s, c), x), ((u, q), b))| v.push((p, s, c, x, u, pz(q)[0], b)));
    rows(v.iter().map(|&(p, s, c, x, u, n, b)| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(c), V::I(x), V::I(s.up), V::I(s.down), user_col(db, u, "uid"), user_col(db, u, "name"), V::I(n)]);
        f.push(nullable(b[1], b[0]));
        row(f)
    }))
}

// WITH AggregatedPostStats AS (
// SELECT
// p.Id AS PostId,
// p.PostTypeId,
// COUNT(c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
// MAX(COALESCE(p.LastEditDate, p.CreationDate)) AS LastActivityDate
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// p.Id, p.PostTypeId
// ),
// UserEngagement AS (
// SELECT
// u.Id AS UserId,
// COUNT(DISTINCT p.Id) AS PostsCount,
// SUM(u.UpVotes) AS TotalUpVotes,
// SUM(u.DownVotes) AS TotalDownVotes,
// MAX(u.LastAccessDate) AS LastAccess
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id
// )
// SELECT
// aps.PostId,
// aps.PostTypeId,
// aps.CommentCount,
// aps.VoteCount,
// aps.UpVoteCount,
// aps.DownVoteCount,
// ue.UserId,
// ue.PostsCount,
// ue.TotalUpVotes,
// ue.TotalDownVotes,
// ue.LastAccess
// FROM
// AggregatedPostStats aps
// JOIN
// Users u ON aps.PostId = u.Id
// JOIN
// UserEngagement ue ON u.Id = ue.UserId
// ORDER BY
// aps.PostId;
fn q11310(db: &'static So) -> String {
    let uid = uids(db);
    let lp = user_left_posts(db);
    let mut v = Vec::new();
    stats_fold(db, db.post.with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cv", &[])
        .and(votes_per_post(db))
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&lp)))
        .drive(|p, ((s, x), (u, q))| v.push((p, s, x, u, q)));
    rows(v.iter().map(|&(p, s, x, u, q)| {
        let mut f = post_fields(db, p, &["id", "type_id"]);
        f.extend([V::I(s.cx), V::I(x), V::I(s.up), V::I(s.down), user_col(db, u, "uid"), V::I(q[1])]);
        f.extend([V::I(q[2]), V::I(q[3]), user_col(db, u, "last_access")]);
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
// PostStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS TotalPosts,
// SUM(COALESCE(p.Score, 0)) AS TotalScore,
// SUM(COALESCE(p.ViewCount, 0)) AS TotalViews,
// COUNT(DISTINCT p.Tags) AS UniqueTagsCount
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// )
// SELECT
// ubc.UserId,
// ubc.DisplayName,
// ubc.BadgeCount,
// ubc.GoldBadges,
// ubc.SilverBadges,
// ubc.BronzeBadges,
// COALESCE(ps.TotalPosts, 0) AS TotalPosts,
// COALESCE(ps.TotalScore, 0) AS TotalScore,
// COALESCE(ps.TotalViews, 0) AS TotalViews,
// COALESCE(ps.UniqueTagsCount, 0) AS UniqueTagsCount,
// (ubc.BadgeCount + COALESCE(ps.TotalPosts, 0)) AS PerformanceScore
// FROM
// UserBadgeCounts ubc
// LEFT JOIN
// PostStats ps ON ubc.UserId = ps.OwnerUserId
// ORDER BY
// PerformanceScore DESC
// LIMIT 10;
fn q5540(db: &'static So) -> String {
    let ub = ubc(db);
    let Post { score, view_count, .. } = &db.post;
    let ps = owned(db).group_by(&db.post.owner_user).select(score.and(view_count.opt())).fold([0i64; 3], |a, (s, w)| [a[0] + 1, a[1] + s, a[2] + w.unwrap_or(0)]);
    let dt = owned(db).group_by(&db.post.owner_user).select(&db.post.tags_str).count_distinct();
    let mut v = Vec::new();
    (&ub).and((&ps).opt()).and((&dt).opt()).drive(|u, ((b, p), t)| v.push((u, b, p.unwrap_or([0; 3]), t.unwrap_or(0))));
    out(v, |&(_, b, p, _)| Reverse(b[0] + p[0]), 10, |&(u, b, p, t)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&b));
        f.extend(ints(&[p[0], p[1], p[2], t, b[0] + p[0]]));
        f
    })
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.Reputation,
// COUNT(DISTINCT P.Id) AS PostCount,
// SUM(COALESCE(P.Score, 0)) AS TotalScore,
// SUM(COALESCE(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END, 0)) AS QuestionCount,
// SUM(COALESCE(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END, 0)) AS AnswerCount
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
// COUNT(*) AS BadgeCount,
// SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldBadgeCount,
// SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverBadgeCount,
// SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadgeCount
// FROM
// Badges B
// GROUP BY
// B.UserId
// )
// SELECT
// US.UserId,
// US.Reputation,
// US.PostCount,
// US.TotalScore,
// US.QuestionCount,
// US.AnswerCount,
// COALESCE(BS.BadgeCount, 0) AS BadgeCount,
// COALESCE(BS.GoldBadgeCount, 0) AS GoldBadgeCount,
// COALESCE(BS.SilverBadgeCount, 0) AS SilverBadgeCount,
// COALESCE(BS.BronzeBadgeCount, 0) AS BronzeBadgeCount
// FROM
// UserStats US
// LEFT JOIN
// BadgeStats BS ON US.UserId = BS.UserId
// ORDER BY
// US.Reputation DESC, US.TotalScore DESC;
fn q11304(db: &'static So) -> String {
    let ps = pstat(db, db.post.iq());
    let bc = badge_classes(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&ps).opt()).and((&bc).opt())).drive(|_, ((u, p), b)| v.push((u, pz(p), bz(b))));
    rows(v.iter().map(|&(u, p, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep")];
        f.extend(ints(&[p[0], p[3], p[1], p[2]]));
        f.extend(ints(&b));
        row(f)
    }))
}

// WITH PostStats AS (
// SELECT
// P.Id AS PostId,
// P.PostTypeId,
// P.OwnerUserId,
// COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVoteCount,
// COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVoteCount,
// COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
// COUNT(CASE WHEN PH.Id IS NOT NULL THEN 1 END) AS HistoryCount
// FROM
// Posts P
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// PostHistory PH ON P.Id = PH.PostId
// GROUP BY
// P.Id, P.PostTypeId, P.OwnerUserId
// ),
// UserPostStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(DISTINCT PS.PostId) AS TotalPosts,
// SUM(PS.UpVoteCount) AS TotalUpVotes,
// SUM(PS.DownVoteCount) AS TotalDownVotes,
// SUM(PS.CommentCount) AS TotalComments,
// SUM(PS.HistoryCount) AS TotalHistoryEntries
// FROM
// Users U
// LEFT JOIN
// PostStats PS ON U.Id = PS.OwnerUserId
// GROUP BY
// U.Id, U.DisplayName
// )
// SELECT
// UserId,
// DisplayName,
// TotalPosts,
// TotalUpVotes,
// TotalDownVotes,
// TotalComments,
// TotalHistoryEntries
// FROM
// UserPostStats
// ORDER BY
// TotalPosts DESC, TotalUpVotes DESC;
fn q10781(db: &'static So) -> String {
    let sf = stats_fold(db, owned(db), &db.post.owner_user, "cvh", &[]);
    let np = owned(db).group_by(&db.post.owner_user).select(&db.post.score).fold(0i64, |a, _| a + 1);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&sf).opt()).and((&np).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((u, s), n)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(n.unwrap_or(0))];
        f.extend(s.map_or(nulls(4), |s| ints(&[s.up, s.down, s.cx, s.hx])));
        row(f)
    }))
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(DISTINCT P.Id) AS PostCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UPVoteCount,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
// SUM(COALESCE(CAST(P.ViewCount AS INT), 0)) AS TotalViews
// FROM Users U
// LEFT JOIN Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN Votes V ON P.Id = V.PostId
// GROUP BY U.Id, U.DisplayName
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
// )
// SELECT
// U.UserId,
// U.DisplayName,
// U.PostCount,
// U.QuestionCount,
// U.AnswerCount,
// U.UPVoteCount,
// U.DownVoteCount,
// U.TotalViews,
// B.BadgeCount,
// B.GoldBadgeCount,
// B.SilverBadgeCount,
// B.BronzeBadgeCount
// FROM UserStats U
// LEFT JOIN BadgeStats B ON U.UserId = B.UserId
// ORDER BY U.TotalViews DESC;
fn q14892(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let bc = badge_classes(db);
    let mut v = Vec::new();
    let dp = ud(db, UserWhere::All, posts_of(db));
    (&us).and((&bc).opt()).and((&dp).opt()).drive(|u, ((a, b), d)| v.push((u, a, b, d.unwrap_or(0))));
    rows(v.iter().map(|&(u, a, b, d)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(d)];
        f.extend(["#q", "#a", "#up", "#down", "views_sum0"].iter().map(|c| ustat_field(&a, c)));
        f.extend(b.map_or(nulls(4), |b| ints(&b)));
        row(f)
    }))
}

// WITH UserVotes AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// COUNT(DISTINCT P.Id) AS PostCount
// FROM Users U
// LEFT JOIN Votes V ON U.Id = V.UserId
// LEFT JOIN Posts P ON V.PostId = P.Id
// WHERE U.Reputation > 1000
// GROUP BY U.Id, U.DisplayName
// ),
// TopPosters AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(P.Id) AS TotalPosts,
// SUM(P.ViewCount) AS TotalViews,
// SUM(P.Score) AS TotalScore
// FROM Users U
// INNER JOIN Posts P ON U.Id = P.OwnerUserId
// GROUP BY U.Id, U.DisplayName
// HAVING COUNT(P.Id) > 10
// ),
// CombinedStats AS (
// SELECT
// UV.UserId,
// UV.DisplayName,
// UV.UpVotes,
// UV.DownVotes,
// TP.TotalPosts,
// TP.TotalViews,
// TP.TotalScore
// FROM UserVotes UV
// JOIN TopPosters TP ON UV.UserId = TP.UserId
// )
// SELECT
// DisplayName,
// UpVotes,
// DownVotes,
// TotalPosts,
// TotalViews,
// TotalScore,
// (UpVotes * 1.0 / NULLIF(DownVotes, 0)) AS VoteRatio,
// (TotalScore * 1.0 / NULLIF(TotalPosts, 0)) AS AvgScorePerPost
// FROM CombinedStats
// ORDER BY VoteRatio DESC, TotalScore DESC
// LIMIT 10;
fn q7873(db: &'static So) -> String {
    let uv = uvotes(db);
    let dv = db.vote.group_by(&db.vote.user).select(&db.vote.post).count_distinct();
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000))
        .select(Ident::<User>::new().and((&uv).opt()).and((&dv).opt()).and((&ps).filt(|p: [i64; 13]| p[0] > 10)))
        .drive(|_, (((u, x), d), p)| v.push((u, x.unwrap_or([0; 3]), d.unwrap_or(0), p)));
    out(v, |&(_, x, _, p)| (x[2] == 0, Reverse(if x[2] == 0 { 0 } else { fkey(x[1] as f64 / x[2] as f64) }), Reverse(p[3])), 10, |&(u, x, _, p)| {
        vec![user_col(db, u, "name"), V::I(x[1]), V::I(x[2]), V::I(p[0]), pviews(p), V::I(p[3]), ratio(x[1], x[2]), ratio(p[3], p[0])]
    })
}

// WITH UserPosts AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(CASE WHEN p.PostTypeId = 2 AND p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers,
// AVG(CASE WHEN p.PostTypeId = 1 THEN p.Score END) AS AvgQuestionScore
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName
// ),
// TopUsers AS (
// SELECT
// UserId,
// DisplayName,
// TotalPosts,
// Questions,
// Answers,
// AcceptedAnswers,
// AvgQuestionScore
// FROM
// UserPosts
// WHERE
// TotalPosts > 10
// ORDER BY
// TotalPosts DESC
// LIMIT 5
// ),
// UserBadges AS (
// SELECT
// b.UserId,
// COUNT(b.Id) AS BadgeCount
// FROM
// Badges b
// GROUP BY
// b.UserId
// )
// SELECT
// tu.DisplayName,
// tu.TotalPosts,
// tu.Questions,
// tu.Answers,
// tu.AcceptedAnswers,
// tu.AvgQuestionScore,
// COALESCE(ub.BadgeCount, 0) AS BadgeCount
// FROM
// TopUsers tu
// LEFT JOIN
// UserBadges ub ON tu.UserId = ub.UserId
// ORDER BY
// tu.AvgQuestionScore DESC;
fn q9829(db: &'static So) -> String {
    let Post { post_type_id, score, accepted_answer_id, .. } = &db.post;
    let uf = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(score).and(accepted_answer_id.opt())).fold([0i64; 5], |a, ((t, s), acc)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (t == 2 && acc.is_some()) as i64, a[4] + if t == 1 { s } else { 0 }]
    });
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    (&uf).filt(|a: [i64; 5]| a[0] > 10).and(&bu).drive(|u, (a, b)| v.push((u, a, b)));
    cut(&mut v, |&(_, a, _)| Reverse(a[0]), 5);
    rows(v.iter().map(|&(u, a, b)| row(vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), avg(a[4], a[1]), V::I(b)])))
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN b.Id IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount,
// AVG(u.Reputation) AS AverageReputation
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN Badges b ON u.Id = b.UserId
// GROUP BY u.Id, u.DisplayName
// ),
// PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// COUNT(DISTINCT c.Id) AS CommentCount,
// SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS UpvoteCount
// FROM Posts p
// LEFT JOIN Comments c ON p.Id = c.PostId
// GROUP BY p.Id, p.Title, p.CreationDate, p.ViewCount, p.Score
// )
// SELECT
// us.UserId,
// us.DisplayName,
// us.PostCount,
// us.QuestionCount,
// us.AnswerCount,
// us.BadgeCount,
// us.AverageReputation,
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.ViewCount,
// ps.Score,
// ps.CommentCount,
// ps.UpvoteCount
// FROM UserStats us
// JOIN PostStats ps ON us.UserId = ps.PostId
// ORDER BY us.AverageReputation DESC, us.PostCount DESC;
fn q11997(db: &'static So) -> String {
    let pid = pids(db);
    let ps = pstat(db, db.post.iq());
    let ub = user_pb(db);
    let pc = db
        .post
        .with((&db.post.origid).select(&uids(db)))
        .group_by(Ident::<Post>::new())
        .select((&db.post.score).and(comments_of(db).opt()))
        .fold([0i64; 2], |a, (s, c)| [a[0] + c.is_some() as i64, a[1] + (s > 0) as i64]);
    let mut v = Vec::new();
    db.user
        .select(Ident::<User>::new().and((&ps).opt()).and(&ub).and((&db.user.origid).select(&pid).select(Ident::<Post>::new().and(&pc))))
        .drive(|_, (((u, x), b), (p, c))| v.push((u, pz(x), b, p, c)));
    rows(v.iter().map(|&(u, x, b, p, c)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(x[0]), V::I(b[1]), V::I(b[2]), V::I(b[3])];
        f.push(V::F(db.user.reputation.get(u).unwrap() as f64));
        f.extend(post_fields(db, p, &["id", "title", "created", "views", "score"]));
        f.extend([V::I(c[0]), V::I(c[1])]);
        row(f)
    }))
}

// WITH PostsStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN p.ViewCount IS NOT NULL THEN p.ViewCount ELSE 0 END) AS TotalViews,
// SUM(CASE WHEN p.Score IS NOT NULL THEN p.Score ELSE 0 END) AS TotalScore,
// AVG(p.Score) AS AvgScore,
// AVG(p.ViewCount) AS AvgViews
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// ),
// UsersStats AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// us.TotalPosts,
// us.TotalQuestions,
// us.TotalAnswers,
// us.TotalViews,
// us.TotalScore,
// us.AvgScore,
// us.AvgViews
// FROM
// Users u
// LEFT JOIN
// PostsStats us ON u.Id = us.OwnerUserId
// )
// SELECT
// u.UserId,
// u.Reputation,
// COALESCE(u.TotalPosts, 0) AS TotalPosts,
// COALESCE(u.TotalQuestions, 0) AS TotalQuestions,
// COALESCE(u.TotalAnswers, 0) AS TotalAnswers,
// COALESCE(u.TotalViews, 0) AS TotalViews,
// COALESCE(u.TotalScore, 0) AS TotalScore,
// COALESCE(u.AvgScore, 0) AS AvgScore,
// COALESCE(u.AvgViews, 0) AS AvgViews
// FROM
// UsersStats u
// ORDER BY
// u.TotalPosts DESC;
fn q13725(db: &'static So) -> String {
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&ps).opt())).drive(|_, (u, p)| v.push((u, pz(p))));
    rows(v.iter().map(|&(u, p)| row(vec![user_col(db, u, "uid"), user_col(db, u, "rep"), V::I(p[0]), V::I(p[1]), V::I(p[2]), V::I(p[5]), V::I(p[3]), or0(p[3], p[0]), or0(p[5], p[4])])))
}

// WITH UserScore AS (
// SELECT
// u.Id AS UserId,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// u.Id
// ),
// BadgeCount AS (
// SELECT
// b.UserId,
// COUNT(b.Id) AS TotalBadges
// FROM
// Badges b
// GROUP BY
// b.UserId
// ),
// PostActivity AS (
// SELECT
// p.OwnerUserId,
// MAX(p.LastActivityDate) AS LastActivity
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// )
// SELECT
// u.DisplayName,
// us.UpVotes,
// us.DownVotes,
// us.PostCount,
// us.QuestionCount,
// us.AnswerCount,
// COALESCE(bc.TotalBadges, 0) AS TotalBadges,
// pa.LastActivity
// FROM
// Users u
// JOIN
// UserScore us ON u.Id = us.UserId
// LEFT JOIN
// BadgeCount bc ON u.Id = bc.UserId
// LEFT JOIN
// PostActivity pa ON u.Id = pa.OwnerUserId
// WHERE
// us.UpVotes >= 10
// ORDER BY
// us.PostCount DESC,
// us.UpVotes DESC;
fn q5029(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let bu = badges_per_user(db);
    let la = owned(db).group_by(&db.post.owner_user).select(&db.post.last_activity_date).fold(i64::MIN, |a, d| a.max(d));
    let mut v = Vec::new();
    let dp = ud(db, UserWhere::All, posts_of(db));
    (&us).filt(|a: UStats| a.up >= 10).and(&bu).and((&la).opt()).and(&dp).drive(|u, (((a, b), l), d)| v.push((u, a, b, l, d)));
    rows(v.iter().map(|&(u, a, b, l, d)| row(vec![user_col(db, u, "name"), V::I(a.up), V::I(a.down), V::I(d), V::I(a.q), V::I(a.a), V::I(b), ots(l)])))
}

// WITH UserActivity AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// COUNT(DISTINCT c.Id) AS TotalComments,
// SUM(v.BountyAmount) AS TotalBounties,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON u.Id = v.UserId
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
// p.AnswerCount,
// p.CommentCount,
// p.FavoriteCount,
// COALESCE(ua.TotalPosts, 0) AS UserTotalPosts
// FROM
// Posts p
// LEFT JOIN
// UserActivity ua ON p.OwnerUserId = ua.UserId
// ),
// BenchmarkResults AS (
// SELECT
// COUNT(*) AS TotalPosts,
// AVG(Score) AS AvgPostScore,
// SUM(ViewCount) AS TotalViewCount,
// SUM(AnswerCount) AS TotalAnswers,
// SUM(CommentCount) AS TotalComments,
// SUM(FavoriteCount) AS TotalFavorites
// FROM
// PostStats
// )
// SELECT
// *
// FROM
// BenchmarkResults;
fn q11391(db: &'static So) -> String {
    let Post { score, view_count, answer_count, comment_count, favorite_count, .. } = &db.post;
    let t = db.post.select(score.and(view_count.opt()).and(answer_count.opt()).and(comment_count).and(favorite_count.opt())).fold_flat([0i64; 9], |a, ((((s, w), an), c), fv)| {
        [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), a[4] + an.is_some() as i64, a[5] + an.unwrap_or(0), a[6] + c, a[7] + fv.is_some() as i64, a[8] + fv.unwrap_or(0)]
    });
    row(vec![V::I(t[0]), avg(t[1], t[0]), nullable(t[3], t[2]), nullable(t[5], t[4]), nullable(t[6], t[0]), nullable(t[8], t[7])])
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
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
// p.Id, p.Title, p.ViewCount, p.Score
// ),
// UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(u.Reputation) AS TotalReputation,
// COUNT(DISTINCT b.Id) AS BadgeCount
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
// ps.ViewCount,
// ps.Score,
// ps.CommentCount,
// ps.VoteCount,
// ps.UpVotes,
// ps.DownVotes,
// us.UserId,
// us.DisplayName,
// us.PostCount,
// us.TotalReputation,
// us.BadgeCount
// FROM
// PostStats ps
// JOIN
// UserStats us ON ps.PostId = us.UserId
// ORDER BY
// ps.ViewCount DESC, ps.Score DESC;
fn q11865(db: &'static So) -> String {
    let uid = uids(db);
    let ps = pstat(db, db.post.iq());
    let bu = badges_per_user(db);
    let ub = user_pb(db);
    let mut v = Vec::new();
    stats_fold(db, db.post.with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cv", &[])
        .and(comments_per_post(db))
        .and(votes_per_post(db))
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and((&ps).opt()).and(&bu).and(&ub)))
        .drive(|p, (((s, c), x), (((u, q), b), a))| v.push((p, s, c, x, u, pz(q)[0], b, a)));
    rows(v.iter().map(|&(p, s, c, x, u, n, b, a)| {
        let mut f = post_fields(db, p, &["id", "title", "views", "score"]);
        f.extend([V::I(c), V::I(x), V::I(s.up), V::I(s.down), user_col(db, u, "uid"), user_col(db, u, "name"), V::I(n)]);
        f.extend([V::I(a[4]), V::I(b)]);
        row(f)
    }))
}

// WITH UserBadgeCounts AS (
// SELECT UserId, COUNT(*) as BadgeCount
// FROM Badges
// GROUP BY UserId
// ),
// PostStats AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) as TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) as TotalQuestions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) as TotalAnswers,
// AVG(P.Score) as AvgScore
// FROM Posts P
// WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY P.OwnerUserId
// ),
// UserPerformance AS (
// SELECT
// U.Id,
// U.DisplayName,
// U.Reputation,
// U.LastAccessDate,
// COALESCE(UB.BadgeCount, 0) as BadgeCount,
// COALESCE(PS.TotalPosts, 0) as TotalPosts,
// COALESCE(PS.TotalQuestions, 0) as TotalQuestions,
// COALESCE(PS.TotalAnswers, 0) as TotalAnswers,
// COALESCE(PS.AvgScore, 0) as AvgScore
// FROM Users U
// LEFT JOIN UserBadgeCounts UB ON U.Id = UB.UserId
// LEFT JOIN PostStats PS ON U.Id = PS.OwnerUserId
// )
// SELECT
// UP.Id,
// UP.DisplayName,
// UP.Reputation,
// UP.LastAccessDate,
// UP.BadgeCount,
// UP.TotalPosts,
// UP.TotalQuestions,
// UP.TotalAnswers,
// UP.AvgScore
// FROM UserPerformance UP
// WHERE UP.Reputation > 1000
// ORDER BY UP.AvgScore DESC, UP.TotalPosts DESC
// LIMIT 10;
fn q7915(db: &'static So) -> String {
    let ps = pstat(db, since(db, year_ago()));
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and(&bu).and((&ps).opt())).drive(|_, ((u, b), p)| v.push((u, b, pz(p))));
    let af = |p: [i64; 13]| if p[0] == 0 { 0.0 } else { p[3] as f64 / p[0] as f64 };
    out(v, |&(_, _, p)| (Reverse(fkey(af(p))), Reverse(p[0])), 10, |&(u, b, p)| {
        vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep"), user_col(db, u, "last_access"), V::I(b), V::I(p[0]), V::I(p[1]), V::I(p[2]), or0(p[3], p[0])]
    })
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// u.CreationDate,
// u.LastAccessDate,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
// COUNT(DISTINCT b.Id) AS BadgeCount
// FROM Users u
// LEFT JOIN Votes v ON u.Id = v.UserId
// LEFT JOIN Badges b ON u.Id = b.UserId
// GROUP BY u.Id, u.Reputation, u.CreationDate, u.LastAccessDate
// ),
// PostStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS TotalPosts,
// COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS Questions,
// COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS Answers,
// SUM(p.Score) AS TotalScore,
// SUM(p.ViewCount) AS TotalViews,
// COUNT(DISTINCT c.Id) AS TotalComments
// FROM Posts p
// LEFT JOIN Comments c ON p.Id = c.PostId
// GROUP BY p.OwnerUserId
// )
// SELECT
// us.UserId,
// us.Reputation,
// us.CreationDate,
// us.LastAccessDate,
// us.UpVotes,
// us.DownVotes,
// us.BadgeCount,
// ps.TotalPosts,
// ps.Questions,
// ps.Answers,
// ps.TotalScore,
// ps.TotalViews,
// ps.TotalComments
// FROM UserStats us
// LEFT JOIN PostStats ps ON us.UserId = ps.OwnerUserId
// ORDER BY us.Reputation DESC, us.UserId;
fn q11631(db: &'static So) -> String {
    let uv = user_vb(db);
    let bu = badges_per_user(db);
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let pf = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(score).and(view_count.opt()).and(comments_of(db).opt())).fold([0i64; 7], |a, (((t, s), w), c)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0), a[6] + c.is_some() as i64]
    });
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&uv).opt()).and(&bu).and((&pf).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(((u, x), b), p)| {
        let x = x.unwrap_or([0; 2]);
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep"), user_col(db, u, "ucreated"), user_col(db, u, "last_access"), V::I(x[0]), V::I(x[1]), V::I(b)];
        f.extend(p.map_or(nulls(6), |p| vec![V::I(p[0]), V::I(p[1]), V::I(p[2]), V::I(p[3]), nullable(p[5], p[4]), V::I(p[6])]));
        row(f)
    }))
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.PostTypeId,
// COUNT(c.Id) AS CommentCount,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount,
// COUNT(b.Id) AS BadgeCount
// FROM
// Posts p
// LEFT JOIN Comments c ON p.Id = c.PostId
// LEFT JOIN Votes v ON p.Id = v.PostId
// LEFT JOIN Badges b ON p.OwnerUserId = b.UserId
// GROUP BY
// p.Id, p.Title, p.PostTypeId
// ),
// UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS PostedCount,
// SUM(COALESCE(p.ViewCount, 0)) AS TotalViews,
// SUM(COALESCE(p.Score, 0)) AS TotalScore
// FROM
// Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.PostTypeId,
// ps.CommentCount,
// ps.UpVoteCount,
// ps.DownVoteCount,
// us.UserId,
// us.DisplayName,
// us.PostedCount,
// us.TotalViews,
// us.TotalScore
// FROM
// PostStats ps
// JOIN
// UserStats us ON ps.PostTypeId = 1 AND us.UserId = ps.PostId
// ORDER BY
// ps.UpVoteCount DESC, ps.CommentCount DESC, us.TotalScore DESC
// LIMIT 100;
fn q11661(db: &'static So) -> String {
    let uid = uids(db);
    let op = owner_posts(db);
    let mut v = Vec::new();
    stats_fold(db, questions_only(db).with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cvb", &[])
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and((&op).opt())))
        .drive(|p, (s, (u, o))| v.push((p, s, u, o.unwrap_or([0; 6]))));
    out(v, |&(_, s, _, o)| (Reverse(s.up), Reverse(s.cx), Reverse(o[5])), 100, |&(p, s, u, o)| {
        let mut f = post_fields(db, p, &["id", "title", "type_id"]);
        f.extend([V::I(s.cx), V::I(s.up), V::I(s.down), user_col(db, u, "uid"), user_col(db, u, "name"), V::I(o[0]), V::I(o[4]), V::I(o[5])]);
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
// u.Reputation AS OwnerReputation,
// u.DisplayName AS OwnerDisplayName
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// WHERE
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, u.Reputation, u.DisplayName
// ),
// PostHistoryStats AS (
// SELECT
// ph.PostId,
// COUNT(ph.Id) AS EditCount,
// SUM(CASE WHEN ph.PostHistoryTypeId IN (4, 5, 6) THEN 1 ELSE 0 END) AS ModificationsCount
// FROM
// PostHistory ph
// GROUP BY
// ph.PostId
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.Score,
// ps.ViewCount,
// ps.AnswerCount,
// ps.CommentCount,
// ps.OwnerReputation,
// ps.OwnerDisplayName,
// COALESCE(phs.EditCount, 0) AS TotalEdits,
// COALESCE(phs.ModificationsCount, 0) AS ModificationsCount
// FROM
// PostStats ps
// LEFT JOIN
// PostHistoryStats phs ON ps.PostId = phs.PostId
// ORDER BY
// ps.Score DESC, ps.CreationDate DESC;
fn q11968(db: &'static So) -> String {
    let hf = db.post_history.group_by(&db.post_history.post).select(&db.post_history.post_history_type_id).fold([0i64; 2], |a, t| [a[0] + 1, a[1] + matches!(t, 4 | 5 | 6) as i64]);
    let cp = comments_per_post(db);
    let mut v = Vec::new();
    since(db, year_ago()).select(Ident::<Post>::new().and(&cp).and((&hf).opt())).drive(|_, ((p, c), h)| v.push((p, c, h.unwrap_or([0; 2]))));
    rows(v.iter().map(|&(p, c, h)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers"]);
        f.push(V::I(c));
        f.extend(post_fields(db, p, &["rep", "owner"]));
        f.extend(ints(&h));
        row(f)
    }))
}

// WITH UserScores AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// SUM(CASE WHEN p.PostTypeId = 1 THEN p.Score ELSE 0 END) AS QuestionScore,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// COUNT(DISTINCT c.Id) AS TotalComments
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN Votes v ON p.Id = v.PostId
// LEFT JOIN Comments c ON p.Id = c.PostId
// GROUP BY u.Id, u.DisplayName
// ),
// BadgeCounts AS (
// SELECT
// b.UserId,
// COUNT(*) FILTER (WHERE b.Class = 1) AS GoldBadges,
// COUNT(*) FILTER (WHERE b.Class = 2) AS SilverBadges,
// COUNT(*) FILTER (WHERE b.Class = 3) AS BronzeBadges
// FROM Badges b
// GROUP BY b.UserId
// )
// SELECT
// us.UserId,
// us.DisplayName,
// us.UpVotes,
// us.DownVotes,
// us.QuestionScore,
// us.TotalPosts,
// us.TotalComments,
// COALESCE(bc.GoldBadges, 0) AS GoldBadges,
// COALESCE(bc.SilverBadges, 0) AS SilverBadges,
// COALESCE(bc.BronzeBadges, 0) AS BronzeBadges,
// (us.UpVotes - us.DownVotes) AS NetScore
// FROM UserScores us
// LEFT JOIN BadgeCounts bc ON us.UserId = bc.UserId
// ORDER BY NetScore DESC, us.TotalPosts DESC
// LIMIT 10;
fn q6785(db: &'static So) -> String {
    let bc = badge_classes(db);
    let Post { post_type_id, score, .. } = &db.post;
    let uf = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(post_type_id.and(score).and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(comments_of(db).opt())).opt())
        .fold([0i64; 3], |a, p| match p {
            Some((((t, s), v), _)) => [a[0] + (v == Some(2)) as i64, a[1] + (v == Some(3)) as i64, a[2] + if t == 1 { s } else { 0 }],
            None => a,
        });
    let np = user_distinct_posts(db);
    let nc = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(comments_of(db)).opt()).fold(0i64, |n, c| n + c.is_some() as i64);
    let mut v = Vec::new();
    (&uf).and(&np).and(&nc).and((&bc).opt()).drive(|u, (((a, n), c), b)| v.push((u, [a[0], a[1], a[2], n, c], bz(b))));
    out(v, |&(_, a, _)| (Reverse(a[0] - a[1]), Reverse(a[3])), 10, |&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a));
        f.extend(ints(&[b[1], b[2], b[3], a[0] - a[1]]));
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
// SUM(CASE WHEN p.ViewCount IS NOT NULL THEN p.ViewCount ELSE 0 END) AS TotalViews,
// SUM(p.Score) AS TotalScore
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
// SUM(CASE WHEN ph.PostHistoryTypeId IN (4, 5) THEN 1 ELSE 0 END) AS TitleEditCount,
// SUM(CASE WHEN ph.PostHistoryTypeId IN (24) THEN 1 ELSE 0 END) AS SuggestedEditCount
// FROM
// PostHistory ph
// GROUP BY
// ph.UserId
// )
// SELECT
// ups.UserId,
// ups.DisplayName,
// ups.PostCount,
// ups.QuestionCount,
// ups.AnswerCount,
// ups.TotalViews,
// ups.TotalScore,
// COALESCE(phs.EditCount, 0) AS TotalEdits,
// COALESCE(phs.TitleEditCount, 0) AS TitleEdits,
// COALESCE(phs.SuggestedEditCount, 0) AS SuggestedEdits
// FROM
// UserPostStats ups
// LEFT JOIN
// PostHistoryStats phs ON ups.UserId = phs.UserId
// ORDER BY
// ups.PostCount DESC,
// ups.TotalScore DESC;
fn q12614(db: &'static So) -> String {
    let ps = pstat(db, db.post.iq());
    let ph = db.post_history.group_by(&db.post_history.user).select(&db.post_history.post_history_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + matches!(t, 4 | 5) as i64, a[2] + (t == 24) as i64]);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&ps).opt()).and((&ph).opt())).drive(|_, ((u, p), h)| v.push((u, pz(p), h.unwrap_or([0; 3]))));
    rows(v.iter().map(|&(u, p, h)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(p[0]), V::I(p[1]), V::I(p[2]), V::I(p[5]), nullable(p[3], p[0])];
        f.extend(ints(&h));
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
// SUM(p.ViewCount) AS TotalViews,
// AVG(p.Score) AS AverageScore
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
// ups.AverageScore,
// COALESCE(bs.TotalBadges, 0) AS TotalBadges,
// COALESCE(bs.GoldBadges, 0) AS GoldBadges,
// COALESCE(bs.SilverBadges, 0) AS SilverBadges,
// COALESCE(bs.BronzeBadges, 0) AS BronzeBadges
// FROM
// UserPostStats ups
// LEFT JOIN
// BadgeStats bs ON ups.UserId = bs.UserId
// ORDER BY
// ups.TotalPosts DESC, ups.TotalViews DESC;
fn q10114(db: &'static So) -> String {
    let ps = pstat(db, db.post.iq());
    let bc = badge_classes(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&ps).opt()).and((&bc).opt())).drive(|_, ((u, p), b)| v.push((u, pz(p), bz(b))));
    rows(v.iter().map(|&(u, p, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(p[0]), V::I(p[1]), V::I(p[2]), pviews(p), pscore_avg(p)];
        f.extend(ints(&b));
        row(f)
    }))
}

// WITH UserBadgeCount AS (
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
// TopUsers AS (
// SELECT
// U.Id,
// U.DisplayName,
// U.Reputation,
// U.LastAccessDate,
// COALESCE(UB.BadgeCount, 0) AS BadgeCount
// FROM Users U
// LEFT JOIN UserBadgeCount UB ON U.Id = UB.UserId
// WHERE U.Reputation > 1000
// ORDER BY U.Reputation DESC
// LIMIT 10
// ),
// PostStats AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS PostCount,
// AVG(P.Score) AS AvgScore,
// SUM(P.ViewCount) AS TotalViews
// FROM Posts P
// WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY P.OwnerUserId
// )
// SELECT
// TU.DisplayName,
// TU.Reputation,
// TU.BadgeCount,
// COALESCE(PS.PostCount, 0) AS PostCount,
// COALESCE(PS.AvgScore, 0) AS AvgScore,
// COALESCE(PS.TotalViews, 0) AS TotalViews
// FROM TopUsers TU
// LEFT JOIN PostStats PS ON TU.Id = PS.OwnerUserId
// WHERE TU.BadgeCount > 0
// ORDER BY TU.Reputation DESC, TU.BadgeCount DESC;
fn q3054(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let ps = pstat(db, since(db, year_ago()));
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and(&bu).and((&ps).opt())).drive(|_, ((u, b), p)| v.push((u, b, pz(p))));
    cut(&mut v, |&(u, _, _)| rep_desc(db, u), 10);
    rows(drain(rel(v).filt(|(_, b, _)| b > 0)).into_iter().map(|(_, (u, b, p))| row(vec![user_col(db, u, "name"), user_col(db, u, "rep"), V::I(b), V::I(p[0]), or0(p[3], p[0]), V::I(p[5])])))
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostID,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
// COUNT(DISTINCT c.Id) AS CommentCount,
// COUNT(DISTINCT b.Id) AS BadgeCount,
// u.Reputation AS UserReputation
// FROM
// Posts p
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, u.Reputation
// ),
// PostHistoryStats AS (
// SELECT
// PostId,
// COUNT(*) AS RevisionCount,
// MAX(CreationDate) AS LastUpdated
// FROM
// PostHistory
// GROUP BY
// PostId
// )
// SELECT
// ps.PostID,
// ps.Title,
// ps.CreationDate,
// ps.Score,
// ps.ViewCount,
// ps.UpVotes,
// ps.DownVotes,
// ps.CommentCount,
// ps.BadgeCount,
// ps.UserReputation,
// ph.RevisionCount,
// ph.LastUpdated
// FROM
// PostStats ps
// LEFT JOIN
// PostHistoryStats ph ON ps.PostID = ph.PostId
// ORDER BY
// ps.Score DESC, ps.ViewCount DESC
// LIMIT 100;
fn q11498(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let hf = history_n_max(db);
    let cp = comments_per_post(db);
    let mut v = Vec::new();
    stats_fold(db, db.post.iq(), Ident::<Post>::new(), "vcb", &[]).and(&cp).and((&db.post.owner_user).select(&bu).opt()).and((&hf).opt()).drive(|p, (((s, c), b), h)| v.push((p, s, c, b.unwrap_or(0), h)));
    out(v, |&(p, ..)| score_views(db, p), 100, |&(p, s, c, b, h)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views"]);
        f.extend([V::I(s.up), V::I(s.down), V::I(c), V::I(b)]);
        f.extend(post_fields(db, p, &["rep"]));
        f.extend([oint(h.map(|h| h.0)), ots(h.map(|h| h.1))]);
        f
    })
}

// WITH UserReputation AS (
// SELECT Id, DisplayName, Reputation, CreationDate, LastAccessDate, UpVotes, DownVotes, (UpVotes - DownVotes) AS Score
// FROM Users
// ), RecentPosts AS (
// SELECT Id, Title, ViewCount, CreationDate, OwnerUserId, PostTypeId, AcceptedAnswerId
// FROM Posts
// WHERE CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'
// ), PostsWithOwners AS (
// SELECT rp.*, ur.DisplayName AS OwnerDisplayName
// FROM RecentPosts rp
// JOIN UserReputation ur ON rp.OwnerUserId = ur.Id
// ), AcceptedAnswers AS (
// SELECT p.Id AS PostId, p.AcceptedAnswerId, a.Title AS AcceptedAnswerTitle
// FROM Posts p
// LEFT JOIN Posts a ON p.AcceptedAnswerId = a.Id
// WHERE p.PostTypeId = 1
// ), VotesCount AS (
// SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
// SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
// FROM Votes
// GROUP BY PostId
// )
// SELECT p.Title, p.OwnerDisplayName, p.ViewCount, p.CreationDate,
// COALESCE(a.AcceptedAnswerTitle, 'N/A') AS AcceptedAnswerTitle,
// COALESCE(vc.Upvotes, 0) AS Upvotes, COALESCE(vc.Downvotes, 0) AS Downvotes
// FROM PostsWithOwners p
// LEFT JOIN AcceptedAnswers a ON p.Id = a.PostId
// LEFT JOIN VotesCount vc ON p.Id = vc.PostId
// ORDER BY p.ViewCount DESC, p.CreationDate DESC
// LIMIT 50;
fn q8528(db: &'static So) -> String {
    let pv = post_votes(db);
    let acc = Ident::<Post>::new().with((&db.post.post_type_id).eq(1)).select(&db.post.accepted_answer).select(&db.post.title);
    let mut v = Vec::new();
    owned_since(db, month_ago()).select(Ident::<Post>::new().and((&pv).opt()).and(acc.opt())).drive(|_, ((p, x), t)| v.push((p, x.unwrap_or([0; 3]), t)));
    out(v, |&(p, ..)| (views_desc(db, p), Reverse(db.post.creation_date.get(p).unwrap())), 50, |&(p, x, t)| {
        let mut f = post_fields(db, p, &["title", "owner", "views", "created"]);
        f.extend([V::S(t.unwrap_or("N/A")), V::I(x[1]), V::I(x[2])]);
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
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN Votes v ON v.PostId = p.Id
// GROUP BY u.Id, u.Reputation
// ),
// BadgeStats AS (
// SELECT
// UserId,
// COUNT(DISTINCT Id) AS BadgeCount,
// SUM(CASE WHEN Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM Badges
// GROUP BY UserId
// )
// SELECT
// us.UserId,
// us.Reputation,
// us.PostCount,
// us.QuestionCount,
// us.AnswerCount,
// us.UpVotes,
// us.DownVotes,
// COALESCE(bs.BadgeCount, 0) AS BadgeCount,
// COALESCE(bs.GoldBadges, 0) AS GoldBadges,
// COALESCE(bs.SilverBadges, 0) AS SilverBadges,
// COALESCE(bs.BronzeBadges, 0) AS BronzeBadges
// FROM UserStats us
// LEFT JOIN BadgeStats bs ON us.UserId = bs.UserId
// ORDER BY us.Reputation DESC, us.PostCount DESC;
fn q11157(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let bc = badge_classes(db);
    let mut v = Vec::new();
    let dp = ud(db, UserWhere::All, posts_of(db));
    (&us).and((&bc).opt()).and((&dp).opt()).drive(|u, ((a, b), d)| v.push((u, a, bz(b), d.unwrap_or(0))));
    rows(v.iter().map(|&(u, a, b, d)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep")];
        f.extend(ints(&[d, a.q, a.a, a.up, a.down]));
        f.extend(ints(&b));
        row(f)
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
// ), PostStats AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS TotalPosts,
// SUM(CASE WHEN P.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers,
// AVG(P.Score) AS AvgScore,
// SUM(P.ViewCount) AS TotalViews
// FROM
// Posts P
// GROUP BY
// P.OwnerUserId
// ), ActiveUsers AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COALESCE(P.TotalPosts, 0) AS TotalPosts,
// COALESCE(P.AcceptedAnswers, 0) AS AcceptedAnswers,
// COALESCE(P.AvgScore, 0) AS AvgScore,
// COALESCE(P.TotalViews, 0) AS TotalViews,
// COALESCE(UB.BadgeCount, 0) AS BadgeCount
// FROM
// Users U
// LEFT JOIN
// PostStats P ON U.Id = P.OwnerUserId
// LEFT JOIN
// UserBadgeCounts UB ON U.Id = UB.UserId
// WHERE
// U.Reputation > 1000
// )
// SELECT
// A.UserId,
// A.DisplayName,
// A.TotalPosts,
// A.AcceptedAnswers,
// A.AvgScore,
// A.TotalViews,
// A.BadgeCount
// FROM
// ActiveUsers A
// ORDER BY
// A.TotalPosts DESC,
// A.AcceptedAnswers DESC,
// A.AvgScore DESC
// LIMIT 10;
fn q6315(db: &'static So) -> String {
    let Post { score, view_count, accepted_answer_id, .. } = &db.post;
    let pa = owned(db).group_by(&db.post.owner_user).select(score.and(view_count.opt()).and(accepted_answer_id.opt())).fold([0i64; 4], |a, ((s, w), acc)| {
        [a[0] + 1, a[1] + acc.is_some() as i64, a[2] + s, a[3] + w.unwrap_or(0)]
    });
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and((&pa).opt()).and(&bu)).drive(|_, ((u, p), b)| v.push((u, p.unwrap_or([0; 4]), b)));
    let af = |p: [i64; 4]| if p[0] == 0 { 0.0 } else { p[2] as f64 / p[0] as f64 };
    out(v, |&(_, p, _)| (Reverse(p[0]), Reverse(p[1]), Reverse(fkey(af(p)))), 10, |&(u, p, b)| {
        vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(p[0]), V::I(p[1]), or0(p[2], p[0]), V::I(p[3]), V::I(b)]
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
// ActivePosts AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS TotalPosts,
// COUNT(DISTINCT p.ParentId) AS AnsweredQuestions,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// COUNT(DISTINCT p.Tags) AS UniqueTags
// FROM
// Posts p
// WHERE
// p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 month'
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
// ap.TotalPosts,
// ap.AnsweredQuestions,
// ap.Questions,
// ap.Answers,
// ap.UniqueTags
// FROM
// UserBadges ub
// LEFT JOIN
// ActivePosts ap ON ub.UserId = ap.OwnerUserId
// WHERE
// ub.BadgeCount > 0
// ORDER BY
// ub.BadgeCount DESC, ap.TotalPosts DESC;
fn q7833(db: &'static So) -> String {
    let ub = ubc(db);
    let rp = || db.post.with((&db.post.creation_date).gt(month_ago()));
    let ap = rp().group_by(&db.post.owner_user).select(&db.post.post_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64]);
    let dpar = rp().group_by(&db.post.owner_user).select(&db.post.parent_id).count_distinct();
    let dtag = rp().group_by(&db.post.owner_user).select(&db.post.tags_str).count_distinct();
    let mut v = Vec::new();
    (&ub).filt(|b: [i64; 4]| b[0] > 0).and((&ap).opt()).and((&dpar).opt()).and((&dtag).opt()).drive(|u, (((b, a), d), t)| v.push((u, b, a, d, t)));
    rows(v.iter().map(|&(u, b, a, d, t)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&b));
        f.extend(a.map_or(nulls(5), |a| ints(&[a[0], d.unwrap_or(0), a[1], a[2], t.unwrap_or(0)])));
        row(f)
    }))
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// u.Views,
// u.UpVotes,
// u.DownVotes,
// COUNT(DISTINCT p.Id) AS PostCount,
// COUNT(DISTINCT b.Id) AS BadgeCount
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN Badges b ON u.Id = b.UserId
// GROUP BY u.Id, u.Reputation, u.Views, u.UpVotes, u.DownVotes
// ),
// PostStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS Questions,
// COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS Answers,
// SUM(p.ViewCount) AS TotalViews,
// SUM(p.Score) AS TotalScore,
// AVG(p.Score) AS AvgScore
// FROM Posts p
// GROUP BY p.OwnerUserId
// )
// SELECT
// u.UserId,
// u.Reputation,
// u.Views,
// u.UpVotes,
// u.DownVotes,
// u.PostCount AS TotalPostsByUser,
// COALESCE(ps.TotalPosts, 0) AS TotalPostsByOwner,
// COALESCE(ps.Questions, 0) AS TotalQuestions,
// COALESCE(ps.Answers, 0) AS TotalAnswers,
// COALESCE(ps.TotalViews, 0) AS TotalViews,
// COALESCE(ps.TotalScore, 0) AS TotalScore,
// COALESCE(ps.AvgScore, 0) AS AverageScore,
// u.BadgeCount AS TotalBadges
// FROM UserStats u
// LEFT JOIN PostStats ps ON u.UserId = ps.OwnerUserId
// ORDER BY u.Reputation DESC, u.Views DESC;
fn q11981(db: &'static So) -> String {
    let dp = ud(db, UserWhere::All, posts_of(db));
    let bu = badges_per_user(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&dp).opt()).and(&bu).and((&ps).opt())).drive(|_, (((u, d), b), p)| v.push((u, d.unwrap_or(0), b, pz(p))));
    rows(v.iter().map(|&(u, d, b, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep"), user_col(db, u, "uviews"), user_col(db, u, "uup"), user_col(db, u, "udown")];
        f.extend(ints(&[d, p[0], p[1], p[2], p[5], p[3]]));
        f.extend([or0(p[3], p[0]), V::I(b)]);
        row(f)
    }))
}

// WITH PostStats AS (
// SELECT
// p.PostTypeId,
// COUNT(*) AS TotalPosts,
// SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositiveScorePosts,
// SUM(CASE WHEN p.ViewCount > 100 THEN 1 ELSE 0 END) AS HighViewCountPosts,
// AVG(COALESCE(p.Score, 0)) AS AverageScore,
// AVG(COALESCE(c.CommentCount, 0)) AS AverageCommentCount
// FROM
// Posts p
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
// GROUP BY
// p.PostTypeId
// ),
// UserReputation AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// COUNT(DISTINCT b.Id) AS TotalBadges
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// u.Id, u.Reputation
// )
// SELECT
// pst.PostTypeId,
// pst.TotalPosts,
// pst.PositiveScorePosts,
// pst.HighViewCountPosts,
// pst.AverageScore,
// pst.AverageCommentCount,
// AVG(ur.Reputation) AS AverageUserReputation,
// SUM(ur.TotalBadges) AS TotalBadgesEarned
// FROM
// PostStats pst
// JOIN
// UserReputation ur ON ur.UserId IN (SELECT OwnerUserId FROM Posts WHERE PostTypeId = pst.PostTypeId)
// GROUP BY
// pst.PostTypeId, pst.TotalPosts, pst.PositiveScorePosts, pst.HighViewCountPosts, pst.AverageScore, pst.AverageCommentCount
// ORDER BY
// pst.PostTypeId;
fn q14405(db: &'static So) -> String {
    let cp = comments_per_post(db);
    let Post { score, view_count, .. } = &db.post;
    let ps = db.post.group_by(&db.post.post_type_id).select(score.and(view_count.opt()).and(&cp)).fold([0i64; 5], |a, ((s, w), c)| {
        [a[0] + 1, a[1] + (s > 0) as i64, a[2] + (w.unwrap_or(0) > 100) as i64, a[3] + s, a[4] + c]
    });
    let bu = badges_per_user(db);
    let tu: MatSet<(i64, Id<User>)> = owned(db).select((&db.post.post_type_id).and(&db.post.owner_user)).collect();
    let ou = (&tu)
        .group_by(Same::<(i64, Id<User>)>::new().map(|(t, _)| t))
        .select(Same::<(i64, Id<User>)>::new().map(|(_, u)| u).select((&db.user.reputation).and(&bu)))
        .fold([0i64; 3], |a, (r, b)| [a[0] + 1, a[1] + r, a[2] + b]);
    let mut v = Vec::new();
    (&ps).and(&ou).drive(|t, (p, o)| v.push((t, p, o)));
    rows(v.iter().map(|&(t, p, o)| row(vec![V::I(t), V::I(p[0]), V::I(p[1]), V::I(p[2]), avg(p[3], p[0]), avg(p[4], p[0]), avg(o[1], o[0]), V::I(o[2])])))
}

// WITH UserReputation AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN Votes v ON p.Id = v.PostId
// GROUP BY u.Id, u.DisplayName, u.Reputation
// ),
// TopUsers AS (
// SELECT
// ur.UserId,
// ur.DisplayName,
// ur.Reputation,
// ur.PostCount,
// (ur.UpVotes - ur.DownVotes) AS NetVotes
// FROM UserReputation ur
// WHERE ur.PostCount > 0
// ORDER BY ur.Reputation DESC
// LIMIT 10
// ),
// PostDetails AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// COUNT(c.Id) AS CommentCount
// FROM Posts p
// LEFT JOIN Comments c ON p.Id = c.PostId
// WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY p.Id, p.Title, p.CreationDate, p.Score
// )
// SELECT
// tu.DisplayName,
// tu.Reputation,
// tu.PostCount,
// tu.NetVotes,
// pd.PostId,
// pd.Title,
// pd.CreationDate,
// pd.CommentCount,
// pd.Score
// FROM TopUsers tu
// JOIN PostDetails pd ON tu.UserId = pd.PostId
// ORDER BY tu.Reputation DESC, pd.Score DESC
fn q8617(db: &'static So) -> String {
    let pid = pids(db);
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let cp = comments_per_post(db);
    let mut v = Vec::new();
    let dp = ud(db, UserWhere::All, posts_of(db));
    db.user
        .select(Ident::<User>::new().and(&us).and((&dp).filt(|d: i64| d > 0)).and((&db.user.origid).select(&pid).with((&db.post.creation_date).ge(year_ago())).select(Ident::<Post>::new().and(&cp)).opt()))
        .drive(|_, x| v.push(x));
    cut(&mut v, |&(((u, _), _), _)| rep_desc(db, u), 10);
    rows(v.iter().filter_map(|&(((u, a), d), p)| {
        p.map(|(p, c)| {
            let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep"), V::I(d), V::I(a.up - a.down)];
            f.extend(post_fields(db, p, &["id", "title", "created"]));
            f.extend([V::I(c)]);
            f.extend(post_fields(db, p, &["score"]));
            row(f)
        })
    }))
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(DISTINCT P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(CASE WHEN P.PostTypeId IN (4, 5) THEN 1 ELSE 0 END) AS TagWikis,
// SUM(CASE WHEN C.Id IS NOT NULL THEN 1 ELSE 0 END) AS Comments
// FROM Users U
// LEFT JOIN Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN Comments C ON P.Id = C.PostId
// GROUP BY U.Id, U.DisplayName, U.Reputation
// ),
// PostStats AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.ViewCount,
// P.Score,
// PT.Name AS PostType,
// P.OwnerUserId
// FROM Posts P
// JOIN PostTypes PT ON P.PostTypeId = PT.Id
// )
// SELECT
// US.UserId,
// US.DisplayName,
// US.Reputation,
// US.TotalPosts,
// US.Questions,
// US.Answers,
// US.TagWikis,
// US.Comments,
// COUNT(DISTINCT PS.PostId) AS TotalPostStats,
// SUM(PS.ViewCount) AS TotalViews,
// SUM(PS.Score) AS TotalScore
// FROM UserStats US
// LEFT JOIN PostStats PS ON US.UserId = PS.OwnerUserId
// GROUP BY US.UserId, US.DisplayName, US.Reputation,
// US.TotalPosts, US.Questions, US.Answers, US.TagWikis, US.Comments
// ORDER BY US.Reputation DESC;
fn q10348(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "c", any_post);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    let dp = ud(db, UserWhere::All, posts_of(db));
    (&us).and((&ps).opt()).and((&dp).opt()).drive(|u, ((a, p), d)| v.push((u, a, pz(p), d.unwrap_or(0))));
    rows(v.iter().map(|&(u, a, p, d)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(ints(&[d, a.q, a.a, a.t45, a.cx, p[0]]));
        f.extend([pviews(p), nullable(p[3], p[0])]);
        row(f)
    }))
}

// WITH UserVoteCounts AS (
// SELECT
// V.UserId,
// COUNT(CASE WHEN V.VoteTypeId IN (2, 3) THEN 1 END) AS VoteCount,
// COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpvoteCount,
// COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownvoteCount
// FROM Votes V
// GROUP BY V.UserId
// ),
// PostStatistics AS (
// SELECT
// P.Id AS PostId,
// P.OwnerUserId,
// COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
// SUM(P.ViewCount) AS TotalViews,
// SUM(P.Score) AS TotalScore
// FROM Posts P
// LEFT JOIN Comments C ON P.Id = C.PostId
// GROUP BY P.Id, P.OwnerUserId
// ),
// UserPostStatistics AS (
// SELECT
// U.Id AS UserId,
// COUNT(DISTINCT P.Id) AS PostCount,
// SUM(PS.CommentCount) AS TotalComments,
// SUM(PS.TotalViews) AS TotalViews,
// SUM(PS.TotalScore) AS TotalScore
// FROM Users U
// LEFT JOIN PostStatistics PS ON U.Id = PS.OwnerUserId
// LEFT JOIN Posts P ON U.Id = P.OwnerUserId
// GROUP BY U.Id
// )
// SELECT
// U.DisplayName,
// U.Reputation,
// UPC.PostCount,
// UPC.TotalComments,
// UPC.TotalViews,
// UPC.TotalScore,
// UVC.VoteCount,
// UVC.UpvoteCount,
// UVC.DownvoteCount
// FROM Users U
// JOIN UserPostStatistics UPC ON U.Id = UPC.UserId
// LEFT JOIN UserVoteCounts UVC ON U.Id = UVC.UserId
// ORDER BY U.Reputation DESC;
fn q11296(db: &'static So) -> String {
    let Post { score, view_count, .. } = &db.post;
    let ps = owned(db).group_by(Ident::<Post>::new()).select(score.and(view_count.opt()).and(comments_of(db).opt())).fold([0i64; 4], |a, ((s, w), c)| {
        [a[0] + c.is_some() as i64, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s]
    });
    let up = db.user.group_by(Ident::<User>::new()).select(posts_of(db).select(&ps).opt().and(posts_of(db).opt())).fold([0i64; 5], |a, (x, _)| match x {
        Some(x) => [a[0] + 1, a[1] + x[0], a[2] + (x[1] > 0) as i64, a[3] + x[2], a[4] + x[3]],
        None => a,
    });
    let np = user_distinct_posts(db);
    let vc = db.vote.group_by(&db.vote.user).select(&db.vote.vote_type_id).fold([0i64; 3], |a, t| [a[0] + matches!(t, 2 | 3) as i64, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64]);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and(&up).and(&np).and((&vc).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(((u, p), n), x)| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend([V::I(n), nullable(p[1], p[0]), nullable(p[3], p[2]), nullable(p[4], p[0])]);
        f.extend(x.map_or(nulls(3), |x| ints(&x)));
        row(f)
    }))
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(COALESCE(p.Score, 0)) AS TotalScore,
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
// PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// COALESCE(c.CommentCount, 0) AS CommentCount,
// COALESCE(a.AnswerCount, 0) AS AnswerCount,
// p.OwnerUserId
// FROM
// Posts p
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) c ON p.Id = c.PostId
// LEFT JOIN
// (SELECT ParentId, COUNT(*) AS AnswerCount FROM Posts WHERE PostTypeId = 2 GROUP BY ParentId) a ON p.Id = a.ParentId
// )
// SELECT
// us.DisplayName,
// us.PostCount,
// us.TotalScore,
// us.Upvotes,
// us.Downvotes,
// ps.Title,
// ps.CreationDate,
// ps.ViewCount,
// ps.CommentCount,
// ps.AnswerCount
// FROM
// UserStats us
// JOIN
// PostStats ps ON us.UserId = ps.OwnerUserId
// ORDER BY
// us.TotalScore DESC, us.PostCount DESC;
fn q13567(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let cp = comments_per_post(db);
    let ta = typed_answers_per_post(db);
    let mut v = Vec::new();
    let dp = ud(db, UserWhere::All, posts_of(db));
    owned(db).select(Ident::<Post>::new().and(&cp).and(&ta).and((&db.post.owner_user).select(Ident::<User>::new().and(&us).and(&dp)))).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(((p, c), n), ((u, a), d))| {
        let mut f = vec![user_col(db, u, "name"), V::I(d), V::I(a.score_sum), V::I(a.up), V::I(a.down)];
        f.extend(post_fields(db, p, &["title", "created", "views"]));
        f.extend(ints(&[c, n]));
        row(f)
    }))
}

// WITH UserBadges AS (
// SELECT u.Id AS UserId, COUNT(b.Id) AS BadgeCount
// FROM Users u
// LEFT JOIN Badges b ON u.Id = b.UserId
// GROUP BY u.Id
// ),
// PostStats AS (
// SELECT p.OwnerUserId, COUNT(p.Id) AS PostCount, SUM(p.Score) AS TotalScore, SUM(p.ViewCount) AS TotalViews
// FROM Posts p
// WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY p.OwnerUserId
// ),
// VoteSummary AS (
// SELECT v.UserId, COUNT(v.Id) AS VoteCount, SUM(CASE WHEN vt.Name = 'UpMod' THEN 1 ELSE 0 END) AS UpvoteCount
// FROM Votes v
// JOIN VoteTypes vt ON v.VoteTypeId = vt.Id
// GROUP BY v.UserId
// ),
// UserPerformance AS (
// SELECT u.Id, u.DisplayName, COALESCE(ub.BadgeCount, 0) AS BadgeCount,
// COALESCE(ps.PostCount, 0) AS PostCount, COALESCE(ps.TotalScore, 0) AS TotalScore,
// COALESCE(ps.TotalViews, 0) AS TotalViews, COALESCE(vs.VoteCount, 0) AS VoteCount,
// COALESCE(vs.UpvoteCount, 0) AS UpvoteCount
// FROM Users u
// LEFT JOIN UserBadges ub ON u.Id = ub.UserId
// LEFT JOIN PostStats ps ON u.Id = ps.OwnerUserId
// LEFT JOIN VoteSummary vs ON u.Id = vs.UserId
// )
// SELECT u.DisplayName, u.BadgeCount, u.PostCount, u.TotalScore, u.TotalViews, u.VoteCount, u.UpvoteCount
// FROM UserPerformance u
// WHERE u.PostCount > 0
// ORDER BY u.TotalScore DESC, u.VoteCount DESC
// LIMIT 10;
fn q7466(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let ps = pstat(db, since(db, year_ago()));
    let vn = vote_named(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and(&bu).and(&ps).and((&vn).opt())).drive(|_, (((u, b), p), x)| v.push((u, b, p, x.unwrap_or([0; 3]))));
    out(v, |&(_, _, p, x)| (Reverse(p[3]), Reverse(x[0])), 10, |&(u, b, p, x)| vec![user_col(db, u, "name"), V::I(b), V::I(p[0]), V::I(p[3]), V::I(p[5]), V::I(x[0]), V::I(x[1])])
}

// WITH UserStats AS (
// SELECT
// u.Id,
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
// PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// EXISTS (SELECT 1 FROM Comments c WHERE c.PostId = p.Id) AS HasComments,
// p.OwnerUserId
// FROM
// Posts p
// WHERE
// p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 YEAR'
// )
// SELECT
// u.DisplayName,
// u.Reputation,
// u.PostCount,
// u.QuestionCount,
// u.AnswerCount,
// u.UpVoteCount,
// u.DownVoteCount,
// p.PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// p.HasComments
// FROM
// UserStats u
// JOIN
// PostStats p ON u.Id = p.OwnerUserId
// ORDER BY
// u.Reputation DESC,
// p.Score DESC;
fn q12484(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let cp = comments_per_post(db);
    let mut v = Vec::new();
    let dp = ud(db, UserWhere::All, posts_of(db));
    owned_since(db, year_ago()).select(Ident::<Post>::new().and(&cp).and((&db.post.owner_user).select(Ident::<User>::new().and(&us).and(&dp)))).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((p, c), ((u, a), d))| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(ints(&[d, a.q, a.a, a.up, a.down]));
        f.extend(post_fields(db, p, &["id", "title", "created", "score", "views"]));
        f.push(V::B(c > 0));
        row(f)
    }))
}

// WITH PostAggregates AS (
// SELECT
// p.Id AS PostId,
// p.PostTypeId,
// COUNT(c.Id) AS TotalComments,
// COUNT(v.Id) AS TotalVotes,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// SUM(CASE WHEN v.VoteTypeId = 10 THEN 1 ELSE 0 END) AS DeletionVotes
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY
// p.Id, p.PostTypeId
// ),
// UserEngagement AS (
// SELECT
// u.Id AS UserId,
// SUM(p.ViewCount) AS TotalPostViews,
// COUNT(DISTINCT p.Id) AS PostsCreated
// FROM
// Users u
// JOIN
// Posts p ON u.Id = p.OwnerUserId
// WHERE
// u.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY
// u.Id
// )
// SELECT
// pa.PostId,
// pa.PostTypeId,
// pa.TotalComments,
// pa.TotalVotes,
// pa.UpVotes,
// pa.DownVotes,
// pa.DeletionVotes,
// ueng.UserId,
// ueng.TotalPostViews,
// ueng.PostsCreated
// FROM
// PostAggregates pa
// JOIN
// UserEngagement ueng ON pa.PostId = ueng.UserId
// ORDER BY
// pa.TotalVotes DESC, pa.TotalComments DESC;
fn q12171(db: &'static So) -> String {
    let uid = uids(db);
    let ue = owned(db).group_by(&db.post.owner_user).select((&db.post.view_count).opt()).fold([0i64; 3], |a, w| [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0)]);
    let mut v = Vec::new();
    stats_fold(db, since(db, year_ago()), Ident::<Post>::new(), "cv", &[])
        .and((&db.post.origid).select(&uid).with((&db.user.creation_date).ge(year_ago())).select(Ident::<User>::new().and(&ue)))
        .drive(|p, (s, (u, e))| v.push((p, s, u, e)));
    rows(v.iter().map(|&(p, s, u, e)| {
        let mut f = post_fields(db, p, &["id", "type_id"]);
        f.extend([V::I(s.cx), V::I(s.vx), V::I(s.up), V::I(s.down), V::I(s.by_vt[10]), user_col(db, u, "uid"), nullable(e[2], e[1]), V::I(e[0])]);
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
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN Votes v ON p.Id = v.PostId
// GROUP BY u.Id, u.DisplayName
// ),
// BadgeStats AS (
// SELECT
// b.UserId,
// COUNT(b.Id) AS BadgeCount,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadgeCount,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadgeCount,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadgeCount
// FROM Badges b
// GROUP BY b.UserId
// )
// SELECT
// u.UserId,
// u.DisplayName,
// u.PostCount,
// u.QuestionCount,
// u.AnswerCount,
// u.UpVoteCount,
// u.DownVoteCount,
// COALESCE(b.BadgeCount, 0) AS BadgeCount,
// COALESCE(b.GoldBadgeCount, 0) AS GoldBadgeCount,
// COALESCE(b.SilverBadgeCount, 0) AS SilverBadgeCount,
// COALESCE(b.BronzeBadgeCount, 0) AS BronzeBadgeCount
// FROM UserStats u
// LEFT JOIN BadgeStats b ON u.UserId = b.UserId
// ORDER BY u.UserId DESC
// LIMIT 100;
fn q12840(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let bc = badge_classes(db);
    let mut v = Vec::new();
    let dp = ud(db, UserWhere::All, posts_of(db));
    (&us).and((&bc).opt()).and((&dp).opt()).drive(|u, ((a, b), d)| v.push((u, a, bz(b), d.unwrap_or(0))));
    out(v, |&(u, ..)| Reverse(db.user.origid.get(u).unwrap()), 100, |&(u, a, b, d)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&[d, a.q, a.a, a.up, a.down]));
        f.extend(ints(&b));
        f
    })
}

// WITH UserActivity AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(P.Id) AS PostCount,
// SUM(CASE WHEN P.Score > 0 THEN 1 ELSE 0 END) AS PositiveScorePosts,
// SUM(CASE WHEN P.Score < 0 THEN 1 ELSE 0 END) AS NegativeScorePosts,
// SUM(COALESCE(V.BountyAmount, 0)) AS TotalBounty
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Votes V ON P.Id = V.PostId AND V.VoteTypeId IN (8, 9)
// WHERE
// U.Reputation > 1000
// GROUP BY
// U.Id, U.DisplayName
// ),
// PostEngagement AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// COUNT(C.Id) AS CommentCount,
// SUM(CASE WHEN P.ViewCount > 100 THEN 1 ELSE 0 END) AS HighViewCount,
// MAX(P.CreationDate) AS LastActivity
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// WHERE
// P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY
// P.Id, P.Title
// )
// SELECT
// UA.DisplayName,
// UA.PostCount,
// UA.PositiveScorePosts,
// UA.NegativeScorePosts,
// UA.TotalBounty,
// PE.Title,
// PE.CommentCount,
// PE.HighViewCount,
// PE.LastActivity
// FROM
// UserActivity UA
// JOIN
// PostEngagement PE ON UA.UserId = PE.PostId
// ORDER BY
// UA.TotalBounty DESC, UA.PostCount DESC
// LIMIT 50;
fn q5314(db: &'static So) -> String {
    let pid = pids(db);
    let bounty = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).filt(|t: i64| matches!(t, 8 | 9)))).select((&db.vote.bounty_amount).opt());
    let uf = user_base(db, UserWhere::RepGt(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.score).and(bounty.opt())).opt())
        .fold([0i64; 4], |a, p| match p {
            Some((s, b)) => [a[0] + 1, a[1] + (s > 0) as i64, a[2] + (s < 0) as i64, a[3] + b.flatten().unwrap_or(0)],
            None => a,
        });
    let pe = db
        .post
        .with((&db.post.creation_date).ge(year_ago()))
        .group_by(Ident::<Post>::new())
        .select((&db.post.view_count).opt().and(comments_of(db).opt()))
        .fold([0i64; 2], |a, (w, c)| [a[0] + c.is_some() as i64, a[1] + (w.unwrap_or(0) > 100) as i64]);
    let mut v = Vec::new();
    (&uf)
        .and((&db.user.origid).select(&pid).select(Ident::<Post>::new().and(&pe)))
        .drive(|u, (a, (p, c))| v.push((u, a, p, c)));
    out(v, |&(_, a, _, _)| (Reverse(a[3]), Reverse(a[0])), 50, |&(u, a, p, c)| {
        let mut f = vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3])];
        f.extend(post_fields(db, p, &["title"]));
        f.extend([V::I(c[0]), V::I(c[1])]);
        f.extend(post_fields(db, p, &["created"]));
        f
    })
}

// WITH UserEngagement AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges,
// SUM(CASE WHEN c.Id IS NOT NULL THEN 1 ELSE 0 END) AS CommentCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// GROUP BY
// u.Id, u.DisplayName
// ), UserScores AS (
// SELECT
// UserId,
// DisplayName,
// PostCount * 10 + QuestionCount * 20 + AnswerCount * 15 + UpVotes - DownVotes + GoldBadges * 30 + SilverBadges * 15 + BronzeBadges * 5 AS Score
// FROM
// UserEngagement
// )
// SELECT
// UserId,
// DisplayName,
// Score
// FROM
// UserScores
// ORDER BY
// Score DESC
// FETCH FIRST 10 ROWS ONLY;
fn q9124(db: &'static So) -> String {
    let uf = db
        .user
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt()).and(comments_of(db).opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 7], |a, (p, b)| {
            let (t, v) = p.map_or((0, None), |((t, v), _)| (t, v));
            [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64, a[4] + (b == Some(1)) as i64, a[5] + (b == Some(2)) as i64, a[6] + (b == Some(3)) as i64]
        });
    let np = user_distinct_posts(db);
    let mut v = Vec::new();
    (&uf).and(&np).drive(|u, (a, n)| v.push((u, n * 10 + a[0] * 20 + a[1] * 15 + a[2] - a[3] + a[4] * 30 + a[5] * 15 + a[6] * 5)));
    out(v, |&(_, s)| Reverse(s), 10, |&(u, s)| vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(s)])
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
// SUM(P.ViewCount) AS TotalViews,
// AVG(P.Score) AS AvgScore
// FROM
// Posts P
// WHERE
// P.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'
// GROUP BY
// P.OwnerUserId
// ),
// FinalStats AS (
// SELECT
// UB.UserId,
// UB.DisplayName,
// COALESCE(PS.PostCount, 0) AS PostCount,
// COALESCE(PS.TotalScore, 0) AS TotalScore,
// COALESCE(PS.TotalViews, 0) AS TotalViews,
// COALESCE(PS.AvgScore, 0) AS AvgScore,
// UB.BadgeCount,
// UB.GoldBadges,
// UB.SilverBadges,
// UB.BronzeBadges
// FROM
// UserBadges UB
// LEFT JOIN
// PostStats PS ON UB.UserId = PS.OwnerUserId
// )
// SELECT
// *
// FROM
// FinalStats
// WHERE
// BadgeCount > 0
// ORDER BY
// TotalScore DESC, BadgeCount DESC;
fn q6435(db: &'static So) -> String {
    let ub = ubc(db);
    let ps = pstat(db, since(db, date(2023, 10, 1)));
    let mut v = Vec::new();
    (&ub).filt(|b: [i64; 4]| b[0] > 0).and((&ps).opt()).drive(|u, (b, p)| v.push((u, b, pz(p))));
    rows(v.iter().map(|&(u, b, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(p[0]), V::I(p[3]), V::I(p[5]), or0(p[3], p[0])];
        f.extend(ints(&b));
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
// SUM(p.ViewCount) AS TotalViews,
// AVG(EXTRACT(EPOCH FROM p.CreationDate)) AS AvgPostCreationDate
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
// COUNT(CASE WHEN b.Class = 1 THEN 1 END) AS GoldBadges,
// COUNT(CASE WHEN b.Class = 2 THEN 1 END) AS SilverBadges,
// COUNT(CASE WHEN b.Class = 3 THEN 1 END) AS BronzeBadges
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
// ups.TotalScore,
// ups.TotalViews,
// ups.AvgPostCreationDate,
// COALESCE(bs.TotalBadges, 0) AS TotalBadges,
// COALESCE(bs.GoldBadges, 0) AS GoldBadges,
// COALESCE(bs.SilverBadges, 0) AS SilverBadges,
// COALESCE(bs.BronzeBadges, 0) AS BronzeBadges
// FROM
// UserPostStats ups
// LEFT JOIN
// BadgeStats bs ON ups.UserId = bs.UserId
// ORDER BY
// ups.TotalScore DESC;
fn q10226(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, creation_date, .. } = &db.post;
    let pf = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(score).and(view_count.opt()).and(creation_date)).fold(([0i64; 6], 0i128), |(a, e), (((t, s), w), c)| {
        ([a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + s, a[4] + w.is_some() as i64, a[5] + w.unwrap_or(0)], e + c as i128)
    });
    let bc = badge_classes(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&pf).opt()).and((&bc).opt())).drive(|_, ((u, p), b)| v.push((u, p.unwrap_or(([0; 6], 0)), bz(b))));
    rows(v.iter().map(|&(u, (p, e), b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(p[0]), V::I(p[1]), V::I(p[2]), nullable(p[3], p[0]), nullable(p[5], p[4])];
        f.push(if p[0] == 0 { V::Null } else { V::F(e as f64 / p[0] as f64 / 1e6) });
        f.extend(ints(&b));
        row(f)
    }))
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(DISTINCT B.Id) AS BadgeCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
// FROM Users U
// LEFT JOIN Badges B ON U.Id = B.UserId
// LEFT JOIN Votes V ON U.Id = V.UserId
// GROUP BY U.Id, U.DisplayName, U.Reputation
// ),
// PostStats AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS PostCount,
// SUM(P.Score) AS TotalScore,
// COUNT(DISTINCT C.Id) AS CommentCount
// FROM Posts P
// LEFT JOIN Comments C ON P.Id = C.PostId
// GROUP BY P.OwnerUserId
// ),
// CombinedStats AS (
// SELECT
// U.UserId,
// U.DisplayName,
// U.Reputation,
// U.BadgeCount,
// U.UpVoteCount,
// U.DownVoteCount,
// P.PostCount,
// P.TotalScore,
// P.CommentCount
// FROM UserStats U
// LEFT JOIN PostStats P ON U.UserId = P.OwnerUserId
// )
// SELECT
// CS.DisplayName,
// CS.Reputation,
// CS.BadgeCount,
// CS.UpVoteCount,
// CS.DownVoteCount,
// COALESCE(CS.PostCount, 0) AS PostCount,
// COALESCE(CS.TotalScore, 0) AS TotalScore,
// COALESCE(CS.CommentCount, 0) AS CommentCount
// FROM CombinedStats CS
// WHERE CS.Reputation > 100
// ORDER BY CS.Reputation DESC, CS.BadgeCount DESC
// LIMIT 10;
fn q6386(db: &'static So) -> String {
    let uv = user_vb(db);
    let bu = badges_per_user(db);
    let pf = owned(db).group_by(&db.post.owner_user).select((&db.post.score).and(comments_of(db).opt())).fold([0i64; 3], |a, (s, c)| [a[0] + 1, a[1] + s, a[2] + c.is_some() as i64]);
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(100)).select(Ident::<User>::new().and(&uv).and(&bu).and((&pf).opt())).drive(|_, (((u, x), b), p)| v.push((u, x, b, p.unwrap_or([0; 3]))));
    out(v, |&(u, _, b, _)| (rep_desc(db, u), Reverse(b)), 10, |&(u, x, b, p)| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep"), V::I(b), V::I(x[0]), V::I(x[1])];
        f.extend(ints(&p));
        f
    })
}

// WITH PostMetrics AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.ViewCount,
// P.Score,
// P.AnswerCount,
// COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
// COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes,
// COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes,
// U.DisplayName AS OwnerDisplayName
// FROM Posts P
// LEFT JOIN Users U ON P.OwnerUserId = U.Id
// LEFT JOIN Votes V ON P.Id = V.PostId
// LEFT JOIN Comments C ON P.Id = C.PostId
// GROUP BY P.Id, P.Title, P.CreationDate, P.ViewCount, P.Score, P.AnswerCount, U.DisplayName
// ),
// PostHistoryMetrics AS (
// SELECT
// PH.PostId,
// COUNT(CASE WHEN PH.PostHistoryTypeId = 10 THEN 1 END) AS CloseCount,
// COUNT(CASE WHEN PH.PostHistoryTypeId = 11 THEN 1 END) AS ReopenCount
// FROM PostHistory PH
// GROUP BY PH.PostId
// )
// SELECT
// PM.PostId,
// PM.Title,
// PM.CreationDate,
// PM.ViewCount,
// PM.Score,
// PM.AnswerCount,
// PM.CommentCount,
// PM.OwnerDisplayName,
// PM.Upvotes,
// PM.Downvotes,
// COALESCE(PHM.CloseCount, 0) AS CloseCount,
// COALESCE(PHM.ReopenCount, 0) AS ReopenCount
// FROM PostMetrics PM
// LEFT JOIN PostHistoryMetrics PHM ON PM.PostId = PHM.PostId
// ORDER BY PM.Score DESC, PM.ViewCount DESC
// LIMIT 100;
fn q10972(db: &'static So) -> String {
    let ht = history_types(db);
    let mut v = Vec::new();
    stats_fold(db, db.post.iq(), Ident::<Post>::new(), "vc", &[]).and((&ht).opt()).drive(|p, (s, h)| v.push((p, s, h.unwrap_or([0; 4]))));
    out(v, |&(p, ..)| score_views(db, p), 100, |&(p, s, h)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "answers"]);
        f.push(V::I(s.cx));
        f.extend(post_fields(db, p, &["owner"]));
        f.extend(ints(&[s.up, s.down, h[1], h[2]]));
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
// SUM(p.Score) AS TotalScore,
// SUM(p.ViewCount) AS TotalViews
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
// )
// SELECT
// us.UserId,
// us.DisplayName,
// us.Reputation,
// us.TotalPosts,
// us.TotalQuestions,
// us.TotalAnswers,
// us.TotalScore,
// us.TotalViews,
// COALESCE(bs.TotalBadges, 0) AS TotalBadges,
// COALESCE(bs.GoldBadges, 0) AS GoldBadges,
// COALESCE(bs.SilverBadges, 0) AS SilverBadges,
// COALESCE(bs.BronzeBadges, 0) AS BronzeBadges
// FROM
// UserStats us
// LEFT JOIN
// BadgeStats bs ON us.UserId = bs.UserId
// ORDER BY
// us.Reputation DESC, us.TotalPosts DESC;
fn q11547(db: &'static So) -> String {
    let ps = pstat(db, db.post.iq());
    let bc = badge_classes(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&ps).opt()).and((&bc).opt())).drive(|_, ((u, p), b)| v.push((u, pz(p), bz(b))));
    rows(v.iter().map(|&(u, p, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep"), V::I(p[0]), V::I(p[1]), V::I(p[2]), nullable(p[3], p[0]), pviews(p)];
        f.extend(ints(&b));
        row(f)
    }))
}

pub static ENTRIES: &[harness::Entry] = &[
    ("10519", q10519),
    ("14858", q14858),
    ("11809", q11809),
    ("14898", q14898),
    ("6119", q6119),
    ("13744", q13744),
    ("8888", q8888),
    ("13177", q13177),
    ("5621", q5621),
    ("11691", q11691),
    ("13402", q13402),
    ("13928", q13928),
    ("14055", q14055),
    ("13255", q13255),
    ("10849", q10849),
    ("14157", q14157),
    ("12610", q12610),
    ("12075", q12075),
    ("11310", q11310),
    ("5540", q5540),
    ("11304", q11304),
    ("10781", q10781),
    ("14892", q14892),
    ("7873", q7873),
    ("9829", q9829),
    ("11997", q11997),
    ("13725", q13725),
    ("5029", q5029),
    ("11391", q11391),
    ("11865", q11865),
    ("7915", q7915),
    ("11631", q11631),
    ("11661", q11661),
    ("11968", q11968),
    ("6785", q6785),
    ("12614", q12614),
    ("10114", q10114),
    ("3054", q3054),
    ("11498", q11498),
    ("8528", q8528),
    ("11157", q11157),
    ("6315", q6315),
    ("7833", q7833),
    ("11981", q11981),
    ("14405", q14405),
    ("8617", q8617),
    ("10348", q10348),
    ("11296", q11296),
    ("13567", q13567),
    ("7466", q7466),
    ("12484", q12484),
    ("12171", q12171),
    ("12840", q12840),
    ("5314", q5314),
    ("9124", q9124),
    ("6435", q6435),
    ("10226", q10226),
    ("6386", q6386),
    ("10972", q10972),
    ("11547", q11547),
];
