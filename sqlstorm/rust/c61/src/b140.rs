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

fn round2f(x: f32) -> f64 {
    (((x as f64) * 100.0).round() / 100.0) as f32 as f64
}

fn badges_by_uid(db: &'static So) -> HashIdx<i64, Id<Badge>> {
    (&db.badge.user_id).inv().collect()
}

/// Per post over `LEFT JOIN Comments, Votes, Badges ON p.OwnerUserId = b.UserId`
/// (raw ids): [comment rows, vote rows, upvote rows, downvote rows].
fn cvb<Q: Drive<D = Id<Post>, R = Id<Post>>>(db: &'static So, base: Q) -> Fold<Id<Post>, [i64; 4]> {
    let bidx = badges_by_uid(db);
    base.group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and((&db.post.owner_user_id).select(&bidx).opt()))
        .fold([0i64; 4], |a, ((c, v), _)| [a[0] + c.is_some() as i64, a[1] + v.is_some() as i64, a[2] + (v == Some(2)) as i64, a[3] + (v == Some(3)) as i64])
}

// --- batch 140 --------------------------------------------------------------

// WITH RecentPosts AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.Score,
// P.ViewCount,
// P.CreationDate,
// U.DisplayName AS OwnerName,
// COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
// COUNT(DISTINCT V.UserId) AS UpvoteCount
// FROM
// Posts P
// LEFT JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId AND V.VoteTypeId = 2
// WHERE
// P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'
// GROUP BY
// P.Id, P.Title, P.Score, P.ViewCount, P.CreationDate, U.DisplayName
// ), PostHistorySummary AS (
// SELECT
// PH.PostId,
// MAX(PH.CreationDate) AS LastEditDate,
// COUNT(PH.Id) AS EditCount
// FROM
// PostHistory PH
// WHERE
// PH.PostHistoryTypeId IN (4, 5, 6)
// GROUP BY
// PH.PostId
// )
// SELECT
// RP.PostId,
// RP.Title,
// RP.Score,
// RP.ViewCount,
// RP.OwnerName,
// RP.CommentCount,
// RP.UpvoteCount,
// PHS.LastEditDate,
// PHS.EditCount,
// CASE
// WHEN RP.Score > 50 THEN 'Popular'
// WHEN RP.Score BETWEEN 20 AND 50 THEN 'Moderate'
// ELSE 'New'
// END AS PopularityRank
// FROM
// RecentPosts RP
// LEFT JOIN
// PostHistorySummary PHS ON RP.PostId = PHS.PostId
// ORDER BY
// RP.ViewCount DESC, RP.Score DESC;
fn q5180(db: &'static So) -> String {
    let up = votes_of(db).select(Ident::<Vote>::new().with((&db.vote.vote_type_id).eq(2)));
    let cp = since(db, month_ago()).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(up.opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let uv = db.vote.with((&db.vote.vote_type_id).eq(2)).group_by(&db.vote.post).select(&db.vote.user_id).count_distinct();
    let ph = db.post_history.with((&db.post_history.post_history_type_id).filt(|t: i64| matches!(t, 4 | 5 | 6))).group_by(&db.post_history.post).select(&db.post_history.creation_date).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let mut v = Vec::new();
    (&cp).and((&uv).opt()).and((&ph).opt()).drive(|p, x| v.push((p, x)));
    rows(v.iter().map(|&(p, ((c, d), h))| {
        let s = db.post.score.get(p).unwrap();
        let mut f = post_fields(db, p, &["id", "title", "score", "views", "owner"]);
        f.extend(ints(&[c, d.unwrap_or(0)]));
        f.extend([ots(h.map(|h| h.1)), oint(h.map(|h| h.0)), V::S(if s > 50 { "Popular" } else if (20..=50).contains(&s) { "Moderate" } else { "New" })]);
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
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// AVG(COALESCE(p.Score, 0)) AS AvgScore,
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
// u.Id, u.DisplayName, u.Reputation, u.CreationDate
// ),
// PostHistoryCounts AS (
// SELECT
// ph.UserId,
// COUNT(*) AS EditCount,
// SUM(CASE WHEN ph.PostHistoryTypeId IN (4, 5) THEN 1 ELSE 0 END) AS TitleBodyEdits
// FROM
// PostHistory ph
// WHERE
// ph.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'
// GROUP BY
// ph.UserId
// )
// SELECT
// us.DisplayName,
// us.Reputation,
// us.TotalPosts,
// us.TotalAnswers,
// us.AvgScore,
// COALESCE(ph.EditCount, 0) AS TotalEdits,
// COALESCE(ph.TitleBodyEdits, 0) AS TotalTitleBodyEdits,
// CONCAT('Gold: ', us.GoldBadges, ', Silver: ', us.SilverBadges, ', Bronze: ', us.BronzeBadges) AS BadgeSummary
// FROM
// UserStats us
// LEFT JOIN
// PostHistoryCounts ph ON us.UserId = ph.UserId
// WHERE
// us.Reputation > 1000 AND
// (us.TotalPosts > 50 OR us.TotalAnswers > 20)
// ORDER BY
// us.Reputation DESC
// LIMIT 10;
fn q4230(db: &'static So) -> String {
    let dp = ud(db, UserWhere::All, posts_of(db));
    let pb = user_base(db, UserWhere::RepGt(1000))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select((&db.post.post_type_id).and(&db.post.score)).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 6], |a, (p, c)| {
            let (t, s) = p.map_or((0, 0), |x| x);
            [a[0] + 1, a[1] + (t == 2) as i64, a[2] + s, a[3] + (c == Some(1)) as i64, a[4] + (c == Some(2)) as i64, a[5] + (c == Some(3)) as i64]
        });
    let y = year_ago();
    let ph = db.post_history.with((&db.post_history.creation_date).ge(y)).group_by(&db.post_history.user).select(&db.post_history.post_history_type_id).fold([0i64; 2], |a, t| [a[0] + 1, a[1] + matches!(t, 4 | 5) as i64]);
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000))
        .select(Ident::<User>::new().and((&dp).opt()).and(&pb).and((&ph).opt()))
        .filt(|(((_, d), a), _): (((Id<User>, Option<i64>), [i64; 6]), Option<[i64; 2]>)| d.unwrap_or(0) > 50 || a[1] > 20)
        .drive(|_, (((u, d), a), h)| v.push((u, d.unwrap_or(0), a, h.unwrap_or([0; 2]))));
    out(v, |&(u, ..)| rep_desc(db, u), 10, |&(u, d, a, h)| {
        vec![
            user_col(db, u, "name"),
            user_col(db, u, "rep"),
            V::I(d),
            V::I(a[1]),
            avg(a[2], a[0]),
            V::I(h[0]),
            V::I(h[1]),
            V::Owned(format!("Gold: {}, Silver: {}, Bronze: {}", a[3], a[4], a[5])),
        ]
    })
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(DISTINCT C.Id) AS CommentCount,
// COUNT(DISTINCT P.Id) AS PostCount,
// SUM(COALESCE(V.UpVotes, 0)) AS TotalUpVotes,
// SUM(COALESCE(V.DownVotes, 0)) AS TotalDownVotes
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// (SELECT
// PostId,
// SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Votes
// GROUP BY
// PostId) V ON P.Id = V.PostId
// GROUP BY
// U.Id, U.DisplayName, U.Reputation
// ),
// PostStats AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS TotalPosts,
// SUM(P.ViewCount) AS TotalViews,
// SUM(P.AnswerCount) AS TotalAnswers,
// COUNT(DISTINCT PH.Id) AS TotalHistoryEntries
// FROM
// Posts P
// LEFT JOIN
// PostHistory PH ON P.Id = PH.PostId
// WHERE
// P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY
// P.OwnerUserId
// )
// SELECT
// US.UserId,
// US.DisplayName,
// US.Reputation,
// US.PostCount,
// PS.TotalPosts,
// PS.TotalViews,
// PS.TotalAnswers,
// PS.TotalHistoryEntries,
// US.TotalUpVotes,
// US.TotalDownVotes
// FROM
// UserStats US
// LEFT JOIN
// PostStats PS ON US.UserId = PS.OwnerUserId
// WHERE
// US.Reputation > 1000
// ORDER BY
// US.Reputation DESC, US.PostCount DESC;
fn q6991(db: &'static So) -> String {
    let dc = udc(db);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let pv = post_votes(db);
    let uv = owned(db).group_by(&db.post.owner_user).select(comments_of(db).opt().and((&pv).opt())).fold([0i64; 2], |a, (_, x)| {
        let x = x.unwrap_or([0; 3]);
        [a[0] + x[1], a[1] + x[2]]
    });
    let Post { view_count, answer_count, .. } = &db.post;
    let dh = owned_since(db, year_ago()).group_by(&db.post.owner_user).select(history_of(db)).fold(0i64, |n, _| n + 1);
    let pr = owned_since(db, year_ago()).group_by(&db.post.owner_user).select(view_count.opt().and(answer_count.opt()).and(history_of(db).opt())).fold([0i64; 6], |a, ((w, an), _)| {
        [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + an.is_some() as i64, a[4] + an.unwrap_or(0), 0]
    });
    let ps: HashIdx<Id<User>, [i64; 6]> = (&pr).and((&dh).opt()).map(|(a, h): ([i64; 6], Option<i64>)| [a[0], a[1], a[2], a[3], a[4], h.unwrap_or(0)]).collect();
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and((&dp).opt()).and((&dc).opt()).and((&uv).opt()).and((&ps).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((((u, d), _), x), p)| {
        let x = x.unwrap_or([0; 2]);
        let mut f = ["uid", "name", "rep"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.push(V::I(d.unwrap_or(0)));
        f.extend(match p {
            Some(p) => vec![V::I(p[0]), nullable(p[2], p[1]), nullable(p[4], p[3]), V::I(p[5])],
            None => nulls(4),
        });
        f.extend(ints(&x));
        row(f)
    }))
}

// WITH UserActivity AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(DISTINCT P.Id) AS PostCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// COALESCE(SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END), 0) AS GoldBadgeCount,
// COALESCE(SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END), 0) AS SilverBadgeCount,
// COALESCE(SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END), 0) AS BronzeBadgeCount,
// U.Reputation
// FROM Users U
// LEFT JOIN Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN Votes V ON P.Id = V.PostId
// LEFT JOIN Badges B ON U.Id = B.UserId
// WHERE U.Reputation > 100 AND U.LastAccessDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY U.Id, U.DisplayName, U.Reputation
// ),
// PostStatistics AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS TotalPosts,
// AVG(P.Score) AS AverageScore,
// SUM(P.ViewCount) AS TotalViews,
// COUNT(DISTINCT C.Id) AS CommentCount
// FROM Posts P
// LEFT JOIN Comments C ON P.Id = C.PostId
// WHERE P.CreationDate > TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY P.OwnerUserId
// )
// SELECT
// UA.DisplayName,
// UA.PostCount,
// UA.UpVotes,
// UA.DownVotes,
// UA.GoldBadgeCount,
// UA.SilverBadgeCount,
// UA.BronzeBadgeCount,
// PS.TotalPosts,
// PS.AverageScore,
// PS.TotalViews,
// PS.CommentCount
// FROM UserActivity UA
// JOIN PostStatistics PS ON UA.UserId = PS.OwnerUserId
// ORDER BY UA.Reputation DESC, UA.PostCount DESC
// LIMIT 50;
fn q9972(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::RepGt(100), "vb", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let Post { score, view_count, .. } = &db.post;
    let recent = || db.post.with((&db.post.creation_date).gt(year_ago()));
    let dc = recent().group_by(&db.post.owner_user).select(comments_of(db)).fold(0i64, |n, _| n + 1);
    let pr = recent().group_by(&db.post.owner_user).select(score.and(view_count.opt()).and(comments_of(db).opt())).fold([0i64; 5], |a, ((s, w), _)| {
        [a[0] + 1, a[1] + s, a[2] + w.is_some() as i64, a[3] + w.unwrap_or(0), 0]
    });
    let ps: HashIdx<Id<User>, [i64; 5]> = (&pr).and((&dc).opt()).map(|(a, c): ([i64; 5], Option<i64>)| [a[0], a[1], a[2], a[3], c.unwrap_or(0)]).collect();
    let y = year_ago();
    let mut v = Vec::new();
    db.user.with((&db.user.last_access_date).gt(y)).select(Ident::<User>::new().and(&us).and((&dp).opt()).and(&ps)).drive(|_, (((u, a), d), p)| v.push((u, a, d.unwrap_or(0), p)));
    out(v, |&(u, _, d, _)| (rep_desc(db, u), Reverse(d)), 50, |&(u, a, d, p)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(ints(&[d, a.up, a.down, a.bcls[1], a.bcls[2], a.bcls[3], p[0]]));
        f.extend([avg(p[1], p[0]), nullable(p[3], p[2]), V::I(p[4])]);
        f
    })
}

// WITH PostAggregate AS (
// SELECT
// p.Id AS PostId,
// p.PostTypeId,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
// COUNT(DISTINCT ba.Id) AS BadgeCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Badges ba ON p.OwnerUserId = ba.UserId
// WHERE
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'
// GROUP BY
// p.Id, p.PostTypeId
// ),
// PostTypeAggregate AS (
// SELECT
// pt.Id AS PostTypeId,
// pt.Name AS PostTypeName,
// COUNT(*) AS TotalPosts,
// SUM(CommentCount) AS TotalComments,
// SUM(VoteCount) AS TotalVotes,
// SUM(UpVoteCount) AS TotalUpVotes,
// SUM(DownVoteCount) AS TotalDownVotes,
// SUM(BadgeCount) AS TotalBadges
// FROM
// PostAggregate pa
// JOIN
// PostTypes pt ON pa.PostTypeId = pt.Id
// GROUP BY
// pt.Id, pt.Name
// )
// SELECT
// pta.PostTypeId,
// pta.PostTypeName,
// pta.TotalPosts,
// pta.TotalComments,
// pta.TotalVotes,
// pta.TotalUpVotes,
// pta.TotalDownVotes,
// pta.TotalBadges,
// ROUND(COALESCE(pta.TotalVotes, 0)::DECIMAL / NULLIF(pta.TotalPosts, 0), 2) AS AvgVotesPerPost,
// ROUND(COALESCE(pta.TotalComments, 0)::DECIMAL / NULLIF(pta.TotalPosts, 0), 2) AS AvgCommentsPerPost
// FROM
// PostTypeAggregate pta
// ORDER BY
// pta.TotalPosts DESC;
fn q11073(db: &'static So) -> String {
    let sf = cvb(db, since(db, month_ago()));
    let bidx = badges_by_uid(db);
    let bc = since(db, month_ago()).group_by(Ident::<Post>::new()).select((&db.post.owner_user_id).select(&bidx)).fold(0i64, |n, _| n + 1);
    let tf = since(db, month_ago()).group_by(&db.post.post_type).select((&sf).and((&bc).opt())).fold([0i64; 6], |a, (s, b)| {
        [a[0] + 1, a[1] + s[0], a[2] + s[1], a[3] + s[2], a[4] + s[3], a[5] + b.unwrap_or(0)]
    });
    let mut v = Vec::new();
    (&tf).drive(|t, a| v.push((t, a)));
    rows(v.iter().map(|&(t, a)| {
        let mut f = vec![V::I(db.post_type.origid.get(t).unwrap()), V::S(db.post_type.name.get(t).unwrap())];
        f.extend(ints(&a));
        f.extend([V::F(round2(a[2] as f64 / a[0] as f64)), V::F(round2(a[1] as f64 / a[0] as f64))]);
        row(f)
    }))
}

// WITH UserActivity AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(DISTINCT P.Id) AS PostCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes,
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
// WHERE
// U.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'
// GROUP BY
// U.Id, U.DisplayName
// ),
// TopUsers AS (
// SELECT
// UserId,
// DisplayName,
// PostCount,
// AnswerCount,
// QuestionCount,
// Upvotes,
// Downvotes,
// GoldBadges,
// SilverBadges,
// BronzeBadges
// FROM
// UserActivity
// ORDER BY
// PostCount DESC
// LIMIT 10
// )
// SELECT
// U.DisplayName,
// U.PostCount,
// U.AnswerCount,
// U.QuestionCount,
// U.Upvotes,
// U.Downvotes,
// U.GoldBadges,
// U.SilverBadges,
// U.BronzeBadges,
// COALESCE(ROUND((CAST(U.Upvotes AS FLOAT) / NULLIF(U.PostCount, 0)) * 100, 2), 0) AS UpvotePercentage
// FROM
// TopUsers U
// ORDER BY
// U.PostCount DESC;
fn q5114(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::CreatedGe(year_ago()), "vb", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let mut v = Vec::new();
    (&us).and((&dp).opt()).drive(|u, (a, d)| v.push((u, a, d.unwrap_or(0))));
    out(v, |&(u, _, d)| (Reverse(d), db.user.origid.get(u).unwrap()), 10, |&(u, a, d)| {
        let pct = if d == 0 { V::F(0.0) } else { V::F(round2f((a.up as f32 / d as f32) * 100.0)) };
        let mut f = vec![user_col(db, u, "name")];
        f.extend(ints(&[d, a.a, a.q, a.up, a.down, a.bcls[1], a.bcls[2], a.bcls[3]]));
        f.push(pct);
        f
    })
}

// WITH UserVotes AS (
// SELECT
// U.Id AS UserId,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
// FROM
// Users U
// LEFT JOIN
// Votes V ON U.Id = V.UserId
// GROUP BY
// U.Id
// ),
// PostStats AS (
// SELECT
// P.Id AS PostId,
// P.OwnerUserId,
// P.Title,
// COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
// COUNT(CASE WHEN PH.Id IS NOT NULL THEN 1 END) AS EditHistoryCount
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// PostHistory PH ON P.Id = PH.PostId
// WHERE
// P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY
// P.Id, P.OwnerUserId, P.Title
// ),
// PostOwnerStats AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS TotalPosts,
// COUNT(CASE WHEN PS.EditHistoryCount > 0 THEN 1 END) AS PostsWithEditHistory,
// SUM(CASE WHEN PS.CommentCount > 5 THEN 1 ELSE 0 END) AS PopularPosts
// FROM
// PostStats PS
// JOIN
// Posts P ON PS.PostId = P.Id
// GROUP BY
// P.OwnerUserId
// )
// SELECT
// U.DisplayName,
// U.Reputation,
// COALESCE(UP.Upvotes, 0) AS UserUpvotes,
// COALESCE(UP.Downvotes, 0) AS UserDownvotes,
// POS.TotalPosts,
// POS.PostsWithEditHistory,
// POS.PopularPosts
// FROM
// Users U
// LEFT JOIN
// UserVotes UP ON U.Id = UP.UserId
// LEFT JOIN
// PostOwnerStats POS ON U.Id = POS.OwnerUserId
// WHERE
// U.Reputation > 500
// ORDER BY
// U.Reputation DESC,
// UserUpvotes DESC;
fn q26(db: &'static So) -> String {
    let uv = uvotes(db);
    let ps = owned_since(db, year_ago())
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(history_of(db).opt()))
        .fold([0i64; 2], |a, (c, h)| [a[0] + c.is_some() as i64, a[1] + h.is_some() as i64]);
    let pos = owned_since(db, year_ago()).group_by(&db.post.owner_user).select(&ps).fold([0i64; 3], |a, x| [a[0] + 1, a[1] + (x[1] > 0) as i64, a[2] + (x[0] > 5) as i64]);
    let mut v = Vec::new();
    db.user.with((&db.user.reputation).gt(500)).select(Ident::<User>::new().and((&uv).opt()).and((&pos).opt())).drive(|_, ((u, x), p)| v.push((u, x.unwrap_or([0; 3]), p)));
    rows(v.iter().map(|&(u, x, p)| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep"), V::I(x[1]), V::I(x[2])];
        f.extend(p.map_or(nulls(3), |p| ints(&p)));
        row(f)
    }))
}

// WITH RecentPosts AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.Score,
// p.CreationDate,
// p.OwnerUserId,
// p.AnswerCount,
// p.CommentCount,
// COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes,
// COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes
// FROM
// Posts p
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate > (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days')
// GROUP BY
// p.Id, p.Title, p.Score, p.CreationDate, p.OwnerUserId, p.AnswerCount, p.CommentCount
// ),
// TopUsers AS (
// SELECT
// u.Id,
// u.DisplayName,
// SUM(COALESCE(b.Class, 0)) AS TotalBadges,
// COUNT(DISTINCT p.Id) AS TotalPosts
// FROM
// Users u
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName
// ORDER BY
// TotalPosts DESC, TotalBadges DESC
// LIMIT 10
// ),
// PostInteraction AS (
// SELECT
// rp.PostId,
// rp.Title,
// rp.Score,
// rp.CreationDate,
// u.DisplayName AS OwnerDisplayName,
// rp.UpVotes,
// rp.DownVotes,
// tu.TotalBadges,
// tu.TotalPosts
// FROM
// RecentPosts rp
// JOIN
// Users u ON rp.OwnerUserId = u.Id
// JOIN
// TopUsers tu ON u.Id = tu.Id
// )
// SELECT
// pi.Title,
// pi.CreationDate,
// pi.OwnerDisplayName,
// pi.Score,
// pi.UpVotes,
// pi.DownVotes,
// pi.TotalBadges,
// pi.TotalPosts
// FROM
// PostInteraction pi
// ORDER BY
// pi.Score DESC, pi.UpVotes DESC, pi.DownVotes ASC;
fn q5315(db: &'static So) -> String {
    let dp = ud(db, UserWhere::All, posts_of(db));
    let cs = g(db).select(badges_of(db).select(&db.badge.class).opt().and(posts_of(db).opt())).fold(0i64, |n, (c, _)| n + c.unwrap_or(0));
    let tu = g(db).select((&dp).opt().and(&cs)).fold((0i64, 0i64), |_, (d, c): (Option<i64>, i64)| (d.unwrap_or(0), c));
    let top: MatSet<Id<User>> = whole(&db.user.id).select(Ident::<User>::new().and(&tu)).window(row_number, |(_, t): (Id<User>, (i64, i64))| t, desc).filt(|(_, n)| n <= 10).map(|((u, _), _)| u).collect();
    let pv = post_votes(db);
    let mut v = Vec::new();
    db.post
        .with((&db.post.creation_date).gt(month_ago()))
        .with((&db.post.owner_user).select(&top))
        .select(Ident::<Post>::new().and((&pv).opt()).and((&db.post.owner_user).select(&tu)))
        .drive(|_, x| v.push(x));
    rows(v.iter().map(|&((p, x), (d, t))| {
        let x = x.unwrap_or([0; 3]);
        let mut f = post_fields(db, p, &["title", "created", "owner", "score"]);
        f.extend(ints(&[x[1], x[2], t, d]));
        row(f)
    }))
}

// WITH PostStats AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
// COUNT(CASE WHEN V.Id IS NOT NULL THEN 1 END) AS VoteCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// SUM(CASE WHEN B.Id IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount
// FROM
// Posts P
// LEFT JOIN Comments C ON P.Id = C.PostId
// LEFT JOIN Votes V ON P.Id = V.PostId
// LEFT JOIN Badges B ON P.OwnerUserId = B.UserId
// WHERE
// P.CreationDate > '2024-10-01 12:34:56'::timestamp - INTERVAL '1 year'
// GROUP BY
// P.Id, P.Title, P.CreationDate
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
// LEFT JOIN Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN Badges B ON U.Id = B.UserId
// WHERE
// U.CreationDate > '2024-10-01 12:34:56'::timestamp - INTERVAL '1 year'
// GROUP BY
// U.Id, U.DisplayName
// )
// SELECT
// PS.PostId,
// PS.Title,
// PS.CreationDate,
// PS.CommentCount,
// PS.VoteCount,
// PS.UpVotes,
// PS.DownVotes,
// US.UserId,
// US.DisplayName,
// US.PostCount,
// US.TotalUpVotes,
// US.TotalDownVotes,
// US.BadgeCount
// FROM
// PostStats PS
// JOIN
// UserStats US ON PS.PostId = US.UserId
// ORDER BY
// PS.VoteCount DESC, PS.CommentCount DESC;
fn q12160(db: &'static So) -> String {
    let uid = uids(db);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let bu = badges_per_user(db);
    let y = year_ago();
    let uvs = db
        .user
        .with((&db.user.creation_date).gt(y))
        .group_by(Ident::<User>::new())
        .select((&db.user.up_votes).and(&db.user.down_votes).and(posts_of(db).opt().and(badges_of(db).opt())))
        .fold([0i64; 2], |a, ((up, dn), _)| [a[0] + up, a[1] + dn]);
    let mut v = Vec::new();
    cvb(db, db.post.with((&db.post.creation_date).gt(y)))
        .and((&db.post.origid).select(&uid).with((&db.user.creation_date).gt(y)).select(Ident::<User>::new().and((&dp).opt()).and(&bu).and(&uvs)))
        .drive(|p, (s, (((u, d), b), w))| v.push((p, s, u, d.unwrap_or(0), b, w)));
    rows(v.iter().map(|&(p, s, u, d, b, w)| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend(ints(&s));
        f.extend([user_col(db, u, "uid"), user_col(db, u, "name")]);
        f.extend(ints(&[d, w[0], w[1], b]));
        row(f)
    }))
}

// WITH RecentPostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// u.DisplayName AS Owner,
// COUNT(DISTINCT c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// AVG(CAST(COALESCE(v.BountyAmount, 0) AS FLOAT)) AS AverageBounty
// FROM
// Posts p
// LEFT JOIN Users u ON p.OwnerUserId = u.Id
// LEFT JOIN Comments c ON p.Id = c.PostId
// LEFT JOIN Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'
// GROUP BY
// p.Id, p.Title, p.CreationDate, u.DisplayName
// ), EnhancedPostStats AS (
// SELECT
// r.PostId,
// r.Title,
// r.CreationDate,
// r.Owner,
// r.CommentCount,
// r.VoteCount,
// r.UpVotes,
// r.DownVotes,
// r.AverageBounty,
// COALESCE((SELECT MAX(Score) FROM Posts p WHERE p.OwnerUserId = u.Id), 0) AS HighestScoreByOwner
// FROM
// RecentPostStats r
// JOIN Users u ON r.Owner = u.DisplayName
// )
// SELECT
// eps.PostId,
// eps.Title,
// eps.CreationDate,
// eps.Owner,
// eps.CommentCount,
// eps.VoteCount,
// eps.UpVotes,
// eps.DownVotes,
// eps.AverageBounty,
// eps.HighestScoreByOwner,
// CASE
// WHEN eps.UpVotes > eps.DownVotes THEN 'Positive'
// WHEN eps.DownVotes > eps.UpVotes THEN 'Negative'
// ELSE 'Neutral'
// END AS Sentiment
// FROM
// EnhancedPostStats eps
// ORDER BY
// eps.CreationDate DESC, eps.VoteCount DESC;
fn q8199(db: &'static So) -> String {
    let cp = comments_per_post(db);
    let pv = post_votes(db);
    let pr = since(db, month_ago())
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select((&db.vote.vote_type_id).and((&db.vote.bounty_amount).opt())).opt()))
        .fold([0i64; 4], |a, (_, v)| {
            let (t, b) = v.map_or((0, None), |x| x);
            [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64, a[3] + b.unwrap_or(0)]
        });
    let ms = owned(db).group_by(&db.post.owner_user).select(&db.post.score).fold(i64::MIN, |a, s| a.max(s));
    let names: HashIdx<Str, Id<User>> = db.user.select(&db.user.display_name).inv().collect();
    let mut v = Vec::new();
    since(db, month_ago())
        .select(Ident::<Post>::new().and(&cp).and((&pv).opt()).and(&pr).and((&db.post.owner_user).select(&db.user.display_name).select(&names).select((&ms).opt())))
        .drive(|_, x| v.push(x));
    rows(v.iter().map(|&((((p, c), x), b), m)| {
        let x = x.unwrap_or([0; 3]);
        let (up, dn) = (b[1], b[2]);
        let mut f = post_fields(db, p, &["id", "title", "created", "owner"]);
        f.extend(ints(&[c, x[0], up, dn]));
        f.push(V::F(b[3] as f64 / b[0] as f64));
        f.push(V::I(m.unwrap_or(0)));
        f.push(V::S(if up > dn { "Positive" } else if dn > up { "Negative" } else { "Neutral" }));
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
// SUM(P.ViewCount) AS TotalViews,
// SUM(CASE WHEN V.CreationDate IS NOT NULL THEN 1 ELSE 0 END) AS VoteCount
// FROM Users U
// LEFT JOIN Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN Votes V ON P.Id = V.PostId
// GROUP BY U.Id, U.Reputation
// ),
// PostStats AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.ViewCount,
// P.Score,
// P.AnswerCount,
// P.CommentCount,
// P.FavoriteCount,
// P.LastActivityDate,
// P.OwnerUserId
// FROM Posts P
// WHERE P.CreationDate > CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'
// ),
// VoteSummary AS (
// SELECT
// PostId,
// COUNT(*) AS TotalVotes,
// SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
// SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
// FROM Votes
// GROUP BY PostId
// )
// SELECT
// U.UserId,
// U.Reputation,
// U.PostCount,
// U.QuestionCount,
// U.AnswerCount,
// U.TotalViews,
// U.VoteCount,
// P.PostId,
// P.Title,
// P.CreationDate,
// P.ViewCount,
// P.Score,
// P.AnswerCount AS PostAnswerCount,
// P.CommentCount,
// P.FavoriteCount,
// P.LastActivityDate,
// V.TotalVotes,
// V.Upvotes,
// V.Downvotes
// FROM UserStats U
// JOIN PostStats P ON U.UserId = P.OwnerUserId
// LEFT JOIN VoteSummary V ON P.PostId = V.PostId
// ORDER BY U.Reputation DESC, P.ViewCount DESC;
fn q11103(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let pv = post_votes(db);
    let mut v = Vec::new();
    db.post
        .with(&db.post.owner_user)
        .with((&db.post.creation_date).gt(year_ago()))
        .select(Ident::<Post>::new().and((&pv).opt()).and((&db.post.owner_user).select(Ident::<User>::new().and(&us).and(&dp))))
        .drive(|_, x| v.push(x));
    rows(v.iter().map(|&((p, x), ((u, a), d))| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep")];
        f.extend(ints(&[d, a.q, a.a]));
        f.extend([ustat_field(&a, "views_sum"), V::I(a.vx)]);
        f.extend(post_fields(db, p, &["id", "title", "created", "views", "score", "answers", "comments", "favorites", "activity"]));
        f.extend(x.map_or(nulls(3), |x| ints(&x)));
        row(f)
    }))
}

// WITH UserStatistics AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// AVG(u.Reputation) AS AverageReputation
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName
// ),
// PostStatistics AS (
// SELECT
// p.OwnerUserId,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// SUM(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS TotalAcceptedAnswers,
// AVG(p.ViewCount) AS AverageViews,
// AVG(p.Score) AS AverageScore
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// ),
// UserPostStats AS (
// SELECT
// us.UserId,
// us.DisplayName,
// us.TotalPosts,
// us.TotalQuestions,
// us.TotalAnswers,
// us.AverageReputation,
// ps.TotalPosts AS UserTotalPosts,
// ps.TotalAcceptedAnswers,
// ps.AverageViews,
// ps.AverageScore
// FROM
// UserStatistics us
// LEFT JOIN
// PostStatistics ps ON us.UserId = ps.OwnerUserId
// )
// SELECT
// ups.UserId,
// ups.DisplayName,
// ups.TotalPosts,
// ups.TotalQuestions,
// ups.TotalAnswers,
// ups.AverageReputation,
// COALESCE(ups.UserTotalPosts, 0) AS UserTotalPosts,
// COALESCE(ups.TotalAcceptedAnswers, 0) AS TotalAcceptedAnswers,
// COALESCE(ups.AverageViews, 0) AS AverageViews,
// COALESCE(ups.AverageScore, 0) AS AverageScore
// FROM
// UserPostStats ups
// ORDER BY
// ups.AverageReputation DESC,
// ups.TotalPosts DESC;
fn q11231(db: &'static So) -> String {
    let ps = pstat(db, db.post.iq());
    let acc = owned(db).with(&db.post.accepted_answer_id).group_by(&db.post.owner_user).select(&db.post.score).fold(0i64, |a, _| a + 1);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&ps).opt()).and((&acc).opt())).drive(|_, ((u, p), a)| v.push((u, pz(p), a.unwrap_or(0))));
    rows(v.iter().map(|&(u, p, a)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&[p[0], p[1], p[2]]));
        f.extend([V::F(db.user.reputation.get(u).unwrap() as f64), V::I(p[0]), V::I(a), or0(p[5], p[4]), or0(p[3], p[0])]);
        row(f)
    }))
}

// WITH UserBadgeCounts AS (
// SELECT UserId,
// COUNT(CASE WHEN Class = 1 THEN 1 END) AS GoldBadges,
// COUNT(CASE WHEN Class = 2 THEN 1 END) AS SilverBadges,
// COUNT(CASE WHEN Class = 3 THEN 1 END) AS BronzeBadges
// FROM Badges
// GROUP BY UserId
// ),
// PostStats AS (
// SELECT OwnerUserId,
// COUNT(CASE WHEN PostTypeId = 1 THEN 1 END) AS QuestionCount,
// SUM(COALESCE(ViewCount, 0)) AS TotalViews,
// AVG(COALESCE(Score, 0)) AS AverageScore
// FROM Posts
// WHERE CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'
// GROUP BY OwnerUserId
// ),
// ClosedPosts AS (
// SELECT p.OwnerUserId,
// COUNT(*) AS ClosedPostCount,
// MAX(p.ClosedDate) AS LastClosedDate
// FROM Posts p
// JOIN PostHistory ph ON p.Id = ph.PostId
// WHERE ph.PostHistoryTypeId = 10
// GROUP BY p.OwnerUserId
// )
// SELECT u.DisplayName,
// COALESCE(ubc.GoldBadges, 0) AS GoldBadges,
// COALESCE(ubc.SilverBadges, 0) AS SilverBadges,
// COALESCE(ubc.BronzeBadges, 0) AS BronzeBadges,
// COALESCE(ps.QuestionCount, 0) AS QuestionCount,
// COALESCE(ps.TotalViews, 0) AS TotalViews,
// COALESCE(ps.AverageScore, 0) AS AverageScore,
// COALESCE(cp.ClosedPostCount, 0) AS ClosedPostCount,
// CASE
// WHEN cp.LastClosedDate IS NOT NULL THEN 'Yes'
// ELSE 'No'
// END AS HasClosedPosts
// FROM Users u
// LEFT JOIN UserBadgeCounts ubc ON u.Id = ubc.UserId
// LEFT JOIN PostStats ps ON u.Id = ps.OwnerUserId
// LEFT JOIN ClosedPosts cp ON u.Id = cp.OwnerUserId
// WHERE (ubc.GoldBadges IS NOT NULL OR ps.QuestionCount IS NOT NULL OR cp.ClosedPostCount > 0)
// ORDER BY u.Reputation DESC
// LIMIT 100;
fn q3860(db: &'static So) -> String {
    let bc = badge_classes(db);
    let Post { post_type_id, view_count, score, .. } = &db.post;
    let ps = owned_since(db, date(2023, 10, 1)).group_by(&db.post.owner_user).select(post_type_id.and(view_count.opt()).and(score)).fold([0i64; 4], |a, ((t, w), s)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + w.unwrap_or(0), a[3] + s]);
    let cp = db.post_history
        .with((&db.post_history.post_history_type_id).eq(10))
        .group_by((&db.post_history.post).select(&db.post.owner_user))
        .select((&db.post_history.post).select((&db.post.closed_date).opt()))
        .fold((0i64, i64::MIN), |(n, m), c| (n + 1, c.map_or(m, |c| m.max(c))));
    let mut v = Vec::new();
    db.user
        .select(Ident::<User>::new().and((&bc).opt()).and((&ps).opt()).and((&cp).opt()))
        .filt(|(((_, b), p), c): (((Id<User>, Option<[i64; 4]>), Option<[i64; 4]>), Option<(i64, i64)>)| b.is_some() || p.is_some() || c.map_or(0, |c| c.0) > 0)
        .drive(|_, x| v.push(x));
    out(v, |&(((u, _), _), _)| rep_desc(db, u), 100, |&(((u, b), p), c)| {
        let b = bz(b);
        let p = p.unwrap_or([0; 4]);
        let mut f = vec![user_col(db, u, "name")];
        f.extend(ints(&[b[1], b[2], b[3], p[1], p[2]]));
        f.extend([or0(p[3], p[0]), V::I(c.map_or(0, |c| c.0)), V::S(if c.map_or(false, |c| c.1 != i64::MIN) { "Yes" } else { "No" })]);
        f
    })
}

// WITH RecentPosts AS (
// SELECT
// p.Id,
// p.Title,
// p.CreationDate,
// p.OwnerUserId,
// p.ViewCount,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
// COALESCE(SUM(CASE WHEN c.Id IS NOT NULL THEN 1 ELSE 0 END), 0) AS CommentCount
// FROM Posts p
// LEFT JOIN Votes v ON p.Id = v.PostId
// LEFT JOIN Comments c ON p.Id = c.PostId
// WHERE p.CreationDate >= CAST('2024-10-01 12:34:56' AS timestamp) - INTERVAL '30 days'
// GROUP BY p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.ViewCount
// ),
// UserReputation AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// CASE
// WHEN u.Reputation >= 1000 THEN 'Active Contributor'
// WHEN u.Reputation BETWEEN 500 AND 999 THEN 'Moderate Contributor'
// ELSE 'New Contributor'
// END AS ContributorLevel
// FROM Users u
// ),
// PostWithUser AS (
// SELECT
// rp.*,
// ur.Reputation,
// ur.ContributorLevel
// FROM RecentPosts rp
// LEFT JOIN UserReputation ur ON rp.OwnerUserId = ur.UserId
// )
// SELECT
// p.Id,
// p.Title,
// p.CreationDate,
// p.UpVotes,
// p.DownVotes,
// p.CommentCount,
// p.ViewCount,
// COALESCE(ROUND((CAST(p.UpVotes AS FLOAT) / NULLIF((p.UpVotes + p.DownVotes), 0)) * 100, 2), 0) AS ApprovalRate,
// p.Reputation,
// p.ContributorLevel,
// CASE
// WHEN p.CommentCount > 5 THEN 'Highly Engaged'
// WHEN p.CommentCount BETWEEN 1 AND 5 THEN 'Moderately Engaged'
// ELSE 'Not Engaged'
// END AS EngagementLevel
// FROM PostWithUser p
// ORDER BY p.CreationDate DESC
// LIMIT 100;
fn q2684(db: &'static So) -> String {
    let rp = since(db, month_ago())
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()))
        .fold([0i64; 3], |a, (t, c)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.is_some() as i64]);
    let v = drain(&rp);
    out(v, |&(p, _)| Reverse(db.post.creation_date.get(p).unwrap()), 100, |&(p, [up, dn, cc])| {
        let ar = if up + dn == 0 { V::F(0.0) } else { V::F(round2f((up as f32 / (up + dn) as f32) * 100.0)) };
        let rep = db.post.owner_user.get(p).map(|u| db.user.reputation.get(u).unwrap());
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend(ints(&[up, dn, cc]));
        f.extend(post_fields(db, p, &["views"]));
        f.extend([ar, oint(rep)]);
        f.push(match rep {
            Some(r) if r >= 1000 => V::S("Active Contributor"),
            Some(r) if (500..=999).contains(&r) => V::S("Moderate Contributor"),
            Some(_) => V::S("New Contributor"),
            None => V::Null,
        });
        f.push(V::S(if cc > 5 { "Highly Engaged" } else if (1..=5).contains(&cc) { "Moderately Engaged" } else { "Not Engaged" }));
        f
    })
}

// WITH UserActivity AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes,
// MAX(p.CreationDate) AS LastPostDate
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// u.Reputation >= 1000
// GROUP BY
// u.Id, u.DisplayName
// ),
// RecentActivity AS (
// SELECT
// ua.UserId,
// ua.DisplayName,
// ua.TotalPosts,
// ua.TotalAnswers,
// ua.TotalQuestions,
// ua.TotalUpVotes,
// ua.TotalDownVotes,
// DATE_PART('day', TIMESTAMP '2024-10-01 12:34:56' - ua.LastPostDate) AS DaysSinceLastPost
// FROM
// UserActivity ua
// WHERE
// ua.LastPostDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'
// ),
// BadgeCounts AS (
// SELECT
// UserId,
// COUNT(*) AS BadgeCount
// FROM
// Badges
// WHERE
// Date >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '365 days'
// GROUP BY
// UserId
// )
// SELECT
// r.UserId,
// r.DisplayName,
// r.TotalPosts,
// r.TotalAnswers,
// r.TotalQuestions,
// r.TotalUpVotes,
// r.TotalDownVotes,
// r.DaysSinceLastPost,
// COALESCE(b.BadgeCount, 0) AS BadgeCount
// FROM
// RecentActivity r
// LEFT JOIN
// BadgeCounts b ON r.UserId = b.UserId
// ORDER BY
// r.TotalPosts DESC,
// r.TotalUpVotes DESC
// LIMIT 10;
fn q8145(db: &'static So) -> String {
    let t0 = ts(2024, 10, 1, 12, 34, 56);
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let bd = db.badge.with((&db.badge.date).ge(t0 - 365 * 86_400_000_000)).group_by(&db.badge.user).select(&db.badge.class).fold(0i64, |a, _| a + 1);
    let m = month_ago();
    let mut v = Vec::new();
    db.user.with((&db.user.reputation).ge(1000)).select((&us).filt(move |a: UStats| a.n > 0 && a.pmax >= m)).and((&dp).opt()).and((&bd).opt()).drive(|u, ((a, d), b)| {
        v.push((u, a, d.unwrap_or(0), b.unwrap_or(0)))
    });
    out(v, |&(_, a, d, _)| (Reverse(d), Reverse(a.up)), 10, |&(u, a, d, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&[d, a.a, a.q, a.up, a.down, (t0 - a.pmax) / 86_400_000_000, b]));
        f
    })
}

// WITH UserBadgeCounts AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(b.Id) AS BadgeCount
// FROM Users u
// LEFT JOIN Badges b ON u.Id = b.UserId
// GROUP BY u.Id, u.DisplayName
// ),
// PostStatistics AS (
// SELECT
// p.OwnerUserId,
// COUNT(p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(p.ViewCount) AS TotalViews,
// AVG(p.Score) AS AvgScore
// FROM Posts p
// GROUP BY p.OwnerUserId
// ),
// ActiveUserPosts AS (
// SELECT
// p.OwnerUserId,
// COUNT(CASE WHEN p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days' THEN 1 END) AS RecentPostCount
// FROM Posts p
// WHERE p.OwnerUserId IS NOT NULL
// GROUP BY p.OwnerUserId
// ),
// UserPerformance AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COALESCE(ub.BadgeCount, 0) AS BadgeCount,
// COALESCE(ps.PostCount, 0) AS PostCount,
// COALESCE(ps.QuestionCount, 0) AS QuestionCount,
// COALESCE(ps.AnswerCount, 0) AS AnswerCount,
// COALESCE(ps.TotalViews, 0) AS TotalViews,
// COALESCE(ps.AvgScore, 0) AS AvgScore,
// COALESCE(ap.RecentPostCount, 0) AS RecentPostCount
// FROM Users u
// LEFT JOIN UserBadgeCounts ub ON u.Id = ub.UserId
// LEFT JOIN PostStatistics ps ON u.Id = ps.OwnerUserId
// LEFT JOIN ActiveUserPosts ap ON u.Id = ap.OwnerUserId
// )
// SELECT
// UserId,
// DisplayName,
// BadgeCount,
// PostCount,
// QuestionCount,
// AnswerCount,
// TotalViews,
// AvgScore,
// RecentPostCount
// FROM UserPerformance
// ORDER BY AvgScore DESC, BadgeCount DESC, TotalViews DESC
// LIMIT 100;
fn q7861(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let ps = pstat(db, db.post.iq());
    let rp = owned_since(db, month_ago()).group_by(&db.post.owner_user).select(&db.post.score).fold(0i64, |a, _| a + 1);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and(&bu).and((&ps).opt()).and((&rp).opt())).drive(|_, (((u, b), p), r)| v.push((u, b, pz(p), r.unwrap_or(0))));
    let af = |p: [i64; 13]| if p[0] == 0 { 0.0 } else { p[3] as f64 / p[0] as f64 };
    out(v, |&(_, b, p, _)| (Reverse(fkey(af(p))), Reverse(b), Reverse(p[5])), 100, |&(u, b, p, r)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&[b, p[0], p[1], p[2], p[5]]));
        f.extend([or0(p[3], p[0]), V::I(r)]);
        f
    })
}

// WITH UserVoteStats AS (
// SELECT
// u.Id AS UserId,
// COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotesCount,
// COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotesCount,
// COUNT(v.Id) AS TotalVotesCount
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
// p.Title,
// p.Score,
// p.ViewCount,
// COALESCE(u.DisplayName, 'Community User') AS OwnerDisplayName,
// COUNT(DISTINCT c.Id) AS CommentCount,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND
// p.Score > 0
// GROUP BY
// p.Id, p.Title, p.Score, p.ViewCount, u.DisplayName
// ),
// PostHistoryStats AS (
// SELECT
// ph.PostId,
// COUNT(ph.Id) AS EditCount,
// MAX(ph.CreationDate) AS LastEditDate
// FROM
// PostHistory ph
// WHERE
// ph.PostHistoryTypeId IN (4, 5, 6, 24)
// GROUP BY
// ph.PostId
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.Score,
// ps.ViewCount,
// ps.OwnerDisplayName,
// ps.CommentCount,
// ps.UpVotes,
// ps.DownVotes,
// COALESCE(phs.EditCount, 0) AS EditCount,
// phs.LastEditDate
// FROM
// PostStats ps
// LEFT JOIN
// PostHistoryStats phs ON ps.PostId = phs.PostId
// ORDER BY
// ps.Score DESC, ps.CommentCount DESC;
fn q6446(db: &'static So) -> String {
    let cp = comments_per_post(db);
    let ph = db.post_history.with((&db.post_history.post_history_type_id).filt(|t: i64| matches!(t, 4 | 5 | 6 | 24))).group_by(&db.post_history.post).select(&db.post_history.creation_date).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let mut v = Vec::new();
    stats_fold(db, since(db, year_ago()).with((&db.post.score).gt(0)), Ident::<Post>::new(), "cv", &[]).and(&cp).and((&ph).opt()).drive(|p, ((s, c), h)| v.push((p, s, c, h)));
    rows(v.iter().map(|&(p, s, c, h)| {
        let mut f = post_fields(db, p, &["id", "title", "score", "views"]);
        f.push(named_owner(db, p, "Community User"));
        f.extend(ints(&[c, s.up, s.down, h.map_or(0, |h| h.0)]));
        f.push(ots(h.map(|h| h.1)));
        row(f)
    }))
}

// WITH UserBadgeCounts AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(B.Id) AS BadgeCount,
// SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldCount,
// SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverCount,
// SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeCount
// FROM
// Users U
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// WHERE
// U.Reputation > 1000
// GROUP BY
// U.Id, U.DisplayName
// ),
// PostStatistics AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS TotalPosts,
// COUNT(DISTINCT P.Id) FILTER (WHERE P.PostTypeId IN (1, 2)) AS QuestionAnswerCount,
// SUM(P.ViewCount) AS TotalViews
// FROM
// Posts P
// WHERE
// P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY
// P.OwnerUserId
// ),
// UserPerformance AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COALESCE(UBC.BadgeCount, 0) AS BadgeCount,
// COALESCE(PS.TotalPosts, 0) AS TotalPosts,
// COALESCE(PS.QuestionAnswerCount, 0) AS QuestionAnswerCount,
// COALESCE(PS.TotalViews, 0) AS TotalViews,
// COALESCE(UBC.GoldCount, 0) AS GoldCount,
// COALESCE(UBC.SilverCount, 0) AS SilverCount,
// COALESCE(UBC.BronzeCount, 0) AS BronzeCount
// FROM
// Users U
// LEFT JOIN
// UserBadgeCounts UBC ON U.Id = UBC.UserId
// LEFT JOIN
// PostStatistics PS ON U.Id = PS.OwnerUserId
// )
// SELECT
// UP.DisplayName,
// UP.BadgeCount,
// UP.TotalPosts,
// UP.QuestionAnswerCount,
// UP.TotalViews,
// UP.GoldCount,
// UP.SilverCount,
// UP.BronzeCount
// FROM
// UserPerformance UP
// ORDER BY
// UP.TotalViews DESC, UP.BadgeCount DESC;
fn q6821(db: &'static So) -> String {
    let bc = badge_classes(db);
    let Post { post_type_id, view_count, .. } = &db.post;
    let ps = owned_since(db, year_ago()).group_by(&db.post.owner_user).select(post_type_id.and(view_count.opt())).fold([0i64; 3], |a, (t, w)| [a[0] + 1, a[1] + matches!(t, 1 | 2) as i64, a[2] + w.unwrap_or(0)]);
    let mut v = Vec::new();
    let ubc = Ident::<User>::new().with((&db.user.reputation).gt(1000)).select(&bc);
    db.user.select(Ident::<User>::new().and(ubc.opt()).and((&ps).opt())).drive(|_, ((u, b), p)| v.push((u, bz(b), p.unwrap_or([0; 3]))));
    rows(v.iter().map(|&(u, b, p)| {
        let mut f = vec![user_col(db, u, "name"), V::I(b[0])];
        f.extend(ints(&p));
        f.extend(ints(&[b[1], b[2], b[3]]));
        row(f)
    }))
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(DISTINCT P.Id) AS PostCount,
// COUNT(DISTINCT V.Id) AS VoteCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Votes V ON U.Id = V.UserId
// GROUP BY
// U.Id, U.DisplayName, U.Reputation
// ),
// PostEngagement AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
// COUNT(DISTINCT PH.UserId) AS EditCount,
// MAX(PH.CreationDate) AS LastEditDate
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// PostHistory PH ON P.Id = PH.PostId
// WHERE
// P.CreationDate >= '2024-10-01 12:34:56'::timestamp - INTERVAL '1 year'
// GROUP BY
// P.Id, P.Title
// ),
// HighEngagementPosts AS (
// SELECT
// PE.PostId,
// PE.Title,
// PE.CommentCount,
// PE.EditCount,
// U.DisplayName,
// U.Reputation
// FROM
// PostEngagement PE
// JOIN
// Users U ON U.Id = (SELECT OwnerUserId FROM Posts WHERE Id = PE.PostId)
// WHERE
// PE.CommentCount > 5 AND PE.EditCount > 2
// )
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(HEP.PostId) AS EngagedPostCount
// FROM
// Users U
// LEFT JOIN
// HighEngagementPosts HEP ON U.Id = (SELECT OwnerUserId FROM Posts WHERE Id = HEP.PostId)
// GROUP BY
// U.Id, U.DisplayName, U.Reputation
// ORDER BY
// EngagedPostCount DESC, U.Reputation DESC
// LIMIT 10;
fn q6835(db: &'static So) -> String {
    let pc = owned_since(db, year_ago()).group_by(Ident::<Post>::new()).select(comments_of(db).opt().and(history_of(db).opt())).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let hu = db.post_history.group_by(&db.post_history.post).select(&db.post_history.user_id).count_distinct();
    let he = owned_since(db, year_ago())
        .with((&pc).filt(|c: i64| c > 5))
        .with((&hu).filt(|n: i64| n > 2))
        .group_by(&db.post.owner_user)
        .select(&db.post.score)
        .fold(0i64, |a, _| a + 1);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&he).opt())).drive(|_, (u, n)| v.push((u, n.unwrap_or(0))));
    out(v, |&(u, n)| (Reverse(n), rep_desc(db, u)), 10, |&(u, n)| vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep"), V::I(n)])
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS TotalQuestions,
// COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS TotalAnswers,
// SUM(COALESCE(p.Score, 0)) AS TotalScore,
// SUM(COALESCE(p.ViewCount, 0)) AS TotalViews,
// AVG(COALESCE(b.Class, 0)) AS AverageBadgeClass,
// COUNT(DISTINCT b.Id) AS TotalBadges
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// u.Id, u.DisplayName
// ),
// PostEngagement AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// COUNT(c.Id) AS CommentCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
// COUNT(DISTINCT pl.RelatedPostId) AS RelatedPostCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// PostLinks pl ON p.Id = pl.PostId
// WHERE
// p.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year')
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount
// )
// SELECT
// us.DisplayName,
// us.TotalPosts,
// us.TotalQuestions,
// us.TotalAnswers,
// us.TotalScore,
// us.TotalViews,
// us.AverageBadgeClass,
// us.TotalBadges,
// pe.PostId,
// pe.Title,
// pe.CreationDate,
// pe.ViewCount,
// pe.CommentCount,
// pe.VoteCount,
// pe.RelatedPostCount
// FROM
// UserStats us
// JOIN
// PostEngagement pe ON us.UserId = pe.PostId
// ORDER BY
// us.TotalScore DESC, pe.ViewCount DESC
// LIMIT 100 OFFSET 0;
fn q5241(db: &'static So) -> String {
    let uid = uids(db);
    let ps = pstat(db, db.post.iq());
    let bc = badges_per_user(db);
    let cs = g(db)
        .select(posts_of(db).select((&db.post.score).and((&db.post.view_count).opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 4], |a, (p, c)| {
            let (s, w) = p.map_or((0, None), |x| x);
            [a[0] + s, a[1] + w.unwrap_or(0), a[2] + c.unwrap_or(0), a[3] + 1]
        });
    let dl = db.post_link.group_by(&db.post_link.post).select(&db.post_link.related_post_id).count_distinct();
    let mut v = Vec::new();
    stats_fold(db, since(db, year_ago()).with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cvl", &[])
        .and(votes_per_post(db))
        .and((&dl).opt())
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and((&ps).opt()).and(&cs).and(&bc)))
        .drive(|p, (((s, x), l), (((u, q), c), b))| v.push((p, s, x, l.unwrap_or(0), u, pz(q), c, b)));
    out(v, |&(p, _, _, _, _, _, c, _)| (Reverse(c[0]), views_desc(db, p)), 100, |&(p, s, x, l, u, q, c, b)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(ints(&[q[0], q[1], q[2], c[0], c[1]]));
        f.extend([V::F(c[2] as f64 / c[3] as f64), V::I(b)]);
        f.extend(post_fields(db, p, &["id", "title", "created", "views"]));
        f.extend(ints(&[s.cx, x, l]));
        f
    })
}

// WITH UserVotes AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(V.Id) FILTER (WHERE V.VoteTypeId = 2) AS UpVotesCount,
// COUNT(V.Id) FILTER (WHERE V.VoteTypeId = 3) AS DownVotesCount,
// COUNT(V.Id) AS TotalVotes
// FROM
// Users U
// LEFT JOIN
// Votes V ON U.Id = V.UserId
// WHERE
// U.Reputation > 1000
// GROUP BY
// U.Id, U.DisplayName
// ),
// PostStatistics AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.Score,
// P.ViewCount,
// COALESCE(COUNT(C.Id), 0) AS CommentCount,
// COALESCE(AVG(CASE WHEN C.UserId IS NOT NULL THEN C.Score END), 0) AS AvgCommentScore
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// WHERE
// P.CreationDate >= DATE '2024-10-01' - INTERVAL '1 year'
// GROUP BY
// P.Id, P.Title, P.Score, P.ViewCount
// ),
// FinalReport AS (
// SELECT
// U.DisplayName,
// U.UpVotesCount,
// U.DownVotesCount,
// P.Title,
// P.Score,
// P.ViewCount,
// P.CommentCount,
// P.AvgCommentScore,
// CASE
// WHEN U.TotalVotes > 100 THEN 'Highly Active'
// WHEN U.TotalVotes BETWEEN 50 AND 100 THEN 'Moderately Active'
// ELSE 'Less Active'
// END AS UserActivityStatus
// FROM
// UserVotes U
// JOIN
// PostStatistics P ON U.UserId = P.PostId
// )
// SELECT
// DisplayName AS User,
// Title AS PostTitle,
// Score AS PostScore,
// ViewCount AS PostViews,
// CommentCount AS PostComments,
// AvgCommentScore AS AverageCommentScore,
// UserActivityStatus
// FROM
// FinalReport
// WHERE
// (CommentCount > 10 OR AvgCommentScore > 4)
// ORDER BY
// Score DESC,
// ViewCount DESC
// LIMIT 50;
fn q23975(db: &'static So) -> String {
    let pid = pids(db);
    let uv = uvotes(db);
    let cs = db.comment.group_by(&db.comment.post).select((&db.comment.score).and((&db.comment.user_id).opt())).fold([0i64; 3], |a, (s, u)| [a[0] + 1, a[1] + u.is_some() as i64, a[2] + if u.is_some() { s } else { 0 }]);
    let d0 = date(2023, 10, 1);
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000))
        .select(Ident::<User>::new().and((&uv).opt()).and((&db.user.origid).select(&pid).with((&db.post.creation_date).ge(d0)).select(Ident::<Post>::new().and((&cs).opt().map(|c: Option<[i64; 3]>| {
            let c = c.unwrap_or([0; 3]);
            (c[0], if c[1] == 0 { 0.0 } else { c[2] as f64 / c[1] as f64 })
        })))))
        .filt(|(_, (_, (n, a))): ((Id<User>, Option<[i64; 3]>), (Id<Post>, (i64, f64)))| n > 10 || a > 4.0)
        .drive(|_, ((u, x), (p, (c, a)))| v.push((u, x.unwrap_or([0; 3]), p, c, a)));
    out(v, |&(_, _, p, ..)| score_views(db, p), 50, |&(u, x, p, c, a)| {
        let st = if x[0] > 100 { "Highly Active" } else if (50..=100).contains(&x[0]) { "Moderately Active" } else { "Less Active" };
        let mut f = vec![user_col(db, u, "name")];
        f.extend(post_fields(db, p, &["title", "score", "views"]));
        f.extend([V::I(c), V::F(a), V::S(st)]);
        f
    })
}

// WITH RankedPosts AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.Body,
// p.CreationDate,
// p.Score,
// COUNT(DISTINCT CASE WHEN p.Tags IS NOT NULL THEN p.Tags END) AS TagCount,
// COALESCE(u.DisplayName, 'Deleted User') AS OwnerDisplayName,
// COUNT(c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.PostTypeId = 1
// GROUP BY
// p.Id, p.Title, p.Body, p.CreationDate, p.Score, u.DisplayName
// ),
// PostHistorySummary AS (
// SELECT
// ph.PostId,
// MAX(CASE WHEN pht.Name = 'Post Closed' THEN ph.CreationDate END) AS ClosedDate,
// COUNT(CASE WHEN pht.Name = 'Edit Body' THEN 1 END) AS EditBodyCount,
// COUNT(CASE WHEN pht.Name = 'Suggested Edit Applied' THEN 1 END) AS SuggestedEditCount
// FROM
// PostHistory ph
// INNER JOIN
// PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id
// GROUP BY
// ph.PostId
// )
// SELECT
// rp.PostId,
// rp.Title,
// rp.Body,
// rp.CreationDate,
// rp.Score,
// rp.TagCount,
// rp.OwnerDisplayName,
// rp.CommentCount,
// rp.UpVoteCount,
// rp.DownVoteCount,
// phs.ClosedDate,
// phs.EditBodyCount,
// phs.SuggestedEditCount,
// CASE
// WHEN phs.ClosedDate IS NOT NULL THEN 'Closed'
// ELSE 'Open'
// END AS PostStatus
// FROM
// RankedPosts rp
// LEFT JOIN
// PostHistorySummary phs ON rp.PostId = phs.PostId
// ORDER BY
// rp.Score DESC, rp.CreationDate DESC
// LIMIT 100;
fn q28615(db: &'static So) -> String {
    let ph = db.post_history.group_by(&db.post_history.post).select((&db.post_history.post_history_type).select(&db.post_history_type.name).and(&db.post_history.creation_date)).fold((i64::MIN, 0i64, 0i64), |(m, e, s), (n, d)| {
        (if n == "Post Closed" { m.max(d) } else { m }, e + (n == "Edit Body") as i64, s + (n == "Suggested Edit Applied") as i64)
    });
    let mut v = Vec::new();
    stats_fold(db, questions_only(db), Ident::<Post>::new(), "cv", &[]).and((&ph).opt()).drive(|p, (s, h)| v.push((p, s, h)));
    out(v, |&(p, ..)| (score_desc(db, p), Reverse(db.post.creation_date.get(p).unwrap())), 100, |&(p, s, h)| {
        let cl = h.and_then(|h| if h.0 == i64::MIN { None } else { Some(h.0) });
        let mut f = post_fields(db, p, &["id", "title", "body", "created", "score"]);
        f.push(V::I(db.post.tags_str.get(p).is_some() as i64));
        f.push(named_owner(db, p, "Deleted User"));
        f.extend(ints(&[s.cx, s.up, s.down]));
        f.extend([ots(cl), oint(h.map(|h| h.1)), oint(h.map(|h| h.2)), V::S(if cl.is_some() { "Closed" } else { "Open" })]);
        f
    })
}

// WITH RecentPosts AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.ViewCount,
// p.CreationDate,
// u.DisplayName AS OwnerDisplayName,
// COUNT(DISTINCT c.Id) AS CommentCount,
// COUNT(DISTINCT a.Id) AS AnswerCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Posts p
// LEFT JOIN
// Users u ON p.OwnerUserId = u.Id
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Posts a ON p.Id = a.ParentId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'
// GROUP BY
// p.Id, p.Title, p.ViewCount, p.CreationDate, u.DisplayName
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
// rp.PostId,
// rp.Title,
// rp.ViewCount,
// rp.OwnerDisplayName,
// rp.CommentCount,
// rp.AnswerCount,
// rp.UpVotes,
// rp.DownVotes,
// COALESCE(pHS.CloseCount, 0) AS CloseCount,
// COALESCE(pHS.ReopenCount, 0) AS ReopenCount,
// COALESCE(pHS.DeleteCount, 0) AS DeleteCount,
// COALESCE(pHS.UndeleteCount, 0) AS UndeleteCount
// FROM
// RecentPosts rp
// LEFT JOIN
// PostHistorySummary pHS ON rp.PostId = pHS.PostId
// ORDER BY
// rp.ViewCount DESC,
// rp.CreationDate DESC
// LIMIT 100;
fn q9275(db: &'static So) -> String {
    let ht = db.post_history.group_by(&db.post_history.post).select(&db.post_history.post_history_type_id).fold([0i64; 4], |a, t| {
        [a[0] + (t == 10) as i64, a[1] + (t == 11) as i64, a[2] + (t == 12) as i64, a[3] + (t == 13) as i64]
    });
    let mut v = Vec::new();
    stats_fold(db, db.post.with((&db.post.creation_date).gt(month_ago())), Ident::<Post>::new(), "cav", &[])
        .and(comments_per_post(db))
        .and(answers_per_post(db))
        .and((&ht).opt())
        .drive(|p, (((s, c), a), h)| v.push((p, s, c, a, h.unwrap_or([0; 4]))));
    out(v, |&(p, ..)| (views_desc(db, p), Reverse(db.post.creation_date.get(p).unwrap())), 100, |&(p, s, c, a, h)| {
        let mut f = post_fields(db, p, &["id", "title", "views", "owner"]);
        f.extend(ints(&[c, a, s.up, s.down]));
        f.extend(ints(&h));
        f
    })
}

// WITH UserReputation AS (
// SELECT
// U.Id AS UserId,
// SUM(CASE WHEN P.PostTypeId = 1 THEN P.Score ELSE 0 END) AS QuestionsScore,
// SUM(CASE WHEN P.PostTypeId = 2 THEN P.Score ELSE 0 END) AS AnswersScore,
// COUNT(DISTINCT P.Id) AS PostCount,
// COUNT(DISTINCT CASE WHEN P.PostTypeId = 1 THEN P.Id END) AS QuestionCount,
// COUNT(DISTINCT CASE WHEN P.PostTypeId = 2 THEN P.Id END) AS AnswerCount
// FROM
// Users U
// JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id
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
// PostActivity AS (
// SELECT
// P.OwnerUserId,
// COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
// COUNT(CASE WHEN PH.Id IS NOT NULL THEN 1 END) AS EditCount,
// SUM(COALESCE(P.ViewCount, 0)) AS TotalViews
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// PostHistory PH ON P.Id = PH.PostId
// GROUP BY
// P.OwnerUserId
// )
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// UR.QuestionsScore,
// UR.AnswersScore,
// UR.PostCount,
// UB.GoldBadges,
// UB.SilverBadges,
// UB.BronzeBadges,
// PA.CommentCount,
// PA.EditCount,
// PA.TotalViews
// FROM
// Users U
// LEFT JOIN
// UserReputation UR ON U.Id = UR.UserId
// LEFT JOIN
// UserBadges UB ON U.Id = UB.UserId
// LEFT JOIN
// PostActivity PA ON U.Id = PA.OwnerUserId
// WHERE
// U.Reputation > 1000
// ORDER BY
// U.Reputation DESC,
// UR.PostCount DESC;
fn q8910(db: &'static So) -> String {
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let ur = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(score)).fold([0i64; 3], |a, (t, s)| [a[0] + if t == 1 { s } else { 0 }, a[1] + if t == 2 { s } else { 0 }, a[2] + 1]);
    let bc = badge_classes(db);
    let pa = owned(db).group_by(&db.post.owner_user).select(view_count.opt().and(comments_of(db).opt()).and(history_of(db).opt())).fold([0i64; 3], |a, ((w, c), h)| {
        [a[0] + c.is_some() as i64, a[1] + h.is_some() as i64, a[2] + w.unwrap_or(0)]
    });
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and((&ur).opt()).and((&bc).opt()).and((&pa).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(((u, r), b), p)| {
        let mut f = ["uid", "name", "rep"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.extend(r.map_or(nulls(3), |r| ints(&r)));
        f.extend(b.map_or(nulls(3), |b| ints(&[b[1], b[2], b[3]])));
        f.extend(p.map_or(nulls(3), |p| ints(&p)));
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
// SUM(CASE WHEN p.PostTypeId = 1 AND p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswersCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.Reputation
// ),
// PostMetrics AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// COALESCE(c.CommentCount, 0) AS CommentCount,
// COALESCE(pl.LinkedPostCount, 0) AS LinkedPostCount,
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
// COUNT(*) AS LinkedPostCount
// FROM
// PostLinks
// GROUP BY
// PostId
// ) pl ON p.Id = pl.PostId
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
// us.Reputation,
// us.PostCount,
// us.QuestionCount,
// us.AnswerCount,
// us.AcceptedAnswersCount,
// pm.PostId,
// pm.Title,
// pm.CreationDate,
// pm.Score,
// pm.CommentCount,
// pm.LinkedPostCount,
// pm.VoteCount
// FROM
// UserStats us
// JOIN
// PostMetrics pm ON us.UserId = pm.PostId
// ORDER BY
// us.Reputation DESC, pm.Score DESC
// LIMIT 100;
fn q14766(db: &'static So) -> String {
    let pid = pids(db);
    let Post { post_type_id, accepted_answer_id, .. } = &db.post;
    let uf = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(accepted_answer_id.opt())).fold([0i64; 4], |a, (t, acc)| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (t == 1 && acc.is_some()) as i64]);
    let ln = db.post_link.group_by(&db.post_link.post).select(&db.post_link.related_post_id).fold(0i64, |a, _| a + 1);
    let mut v = Vec::new();
    db.user
        .select(Ident::<User>::new().and((&uf).opt()).and((&db.user.origid).select(&pid).select(Ident::<Post>::new().and(comments_per_post(db)).and((&ln).opt()).and(votes_per_post(db)))))
        .drive(|_, ((u, a), (((p, c), l), x))| v.push((u, a.unwrap_or([0; 4]), p, c, l.unwrap_or(0), x)));
    out(v, |&(u, _, p, ..)| (rep_desc(db, u), score_desc(db, p)), 100, |&(u, a, p, c, l, x)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep")];
        f.extend(ints(&a));
        f.extend(post_fields(db, p, &["id", "title", "created", "score"]));
        f.extend(ints(&[c, l, x]));
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
// SUM(CASE WHEN c.Id IS NOT NULL THEN 1 ELSE 0 END) AS CommentCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// WHERE
// u.Reputation > 100
// GROUP BY
// u.Id, u.DisplayName
// ), UserBadges AS (
// SELECT
// b.UserId,
// COUNT(b.Id) AS BadgeCount,
// MAX(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadge,
// MAX(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadge,
// MAX(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadge
// FROM
// Badges b
// GROUP BY
// b.UserId
// ), PostScores AS (
// SELECT
// p.OwnerUserId,
// SUM(p.Score) AS TotalScore
// FROM
// Posts p
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY
// p.OwnerUserId
// )
// SELECT
// ua.UserId,
// ua.DisplayName,
// ua.PostCount,
// ua.QuestionCount,
// ua.AnswerCount,
// ua.CommentCount,
// COALESCE(ub.BadgeCount, 0) AS BadgeCount,
// COALESCE(ub.GoldBadge, 0) AS GoldBadge,
// COALESCE(ub.SilverBadge, 0) AS SilverBadge,
// COALESCE(ub.BronzeBadge, 0) AS BronzeBadge,
// COALESCE(ps.TotalScore, 0) AS TotalPostScore
// FROM
// UserActivity ua
// LEFT JOIN
// UserBadges ub ON ua.UserId = ub.UserId
// LEFT JOIN
// PostScores ps ON ua.UserId = ps.OwnerUserId
// WHERE
// ua.PostCount > 0
// ORDER BY
// TotalPostScore DESC,
// ua.QuestionCount DESC,
// ua.PostCount DESC;
fn q5657(db: &'static So) -> String {
    let uf = owned(db).group_by(&db.post.owner_user).select((&db.post.post_type_id).and(comments_of(db).opt())).fold([0i64; 4], |a, (t, c)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + c.is_some() as i64]
    });
    let bf = db.badge.group_by(&db.badge.user).select(&db.badge.class).fold([0i64; 4], |a, c| [a[0] + 1, a[1].max((c == 1) as i64), a[2].max((c == 2) as i64), a[3].max((c == 3) as i64)]);
    let ps = owned_since(db, year_ago()).group_by(&db.post.owner_user).select(&db.post.score).fold(0i64, |a, s| a + s);
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(100)).select(Ident::<User>::new().and(&uf).and((&bf).opt()).and((&ps).opt())).drive(|_, (((u, a), b), s)| v.push((u, a, b.unwrap_or([0; 4]), s.unwrap_or(0))));
    rows(v.iter().map(|&(u, a, b, s)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a));
        f.extend(ints(&b));
        f.push(V::I(s));
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
// PostStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS QuestionCount,
// COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS AnswerCount,
// SUM(p.Score) AS TotalScore,
// SUM(p.ViewCount) AS TotalViews,
// COUNT(DISTINCT p.Tags) AS UniqueTagCount
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
// COALESCE(ps.QuestionCount, 0) AS QuestionCount,
// COALESCE(ps.AnswerCount, 0) AS AnswerCount,
// COALESCE(ps.TotalScore, 0) AS TotalScore,
// COALESCE(ps.TotalViews, 0) AS TotalViews,
// COALESCE(ps.UniqueTagCount, 0) AS UniqueTagCount,
// ub.BadgeCount,
// ub.GoldBadges,
// ub.SilverBadges,
// ub.BronzeBadges
// FROM
// UserBadges ub
// LEFT JOIN
// PostStats ps ON ub.UserId = ps.OwnerUserId
// )
// SELECT
// UserId,
// DisplayName,
// QuestionCount,
// AnswerCount,
// TotalScore,
// TotalViews,
// UniqueTagCount,
// BadgeCount,
// GoldBadges,
// SilverBadges,
// BronzeBadges
// FROM
// UserPerformance
// WHERE
// TotalViews > 1000
// ORDER BY
// TotalScore DESC, QuestionCount DESC
// LIMIT 50;
fn q5798(db: &'static So) -> String {
    let ub = ubc(db);
    let Post { post_type_id, score, view_count, .. } = &db.post;
    let ps = owned_since(db, year_ago()).group_by(&db.post.owner_user).select(post_type_id.and(score).and(view_count.opt())).fold([0i64; 4], |a, ((t, s), w)| [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + s, a[3] + w.unwrap_or(0)]);
    let dt = owned_since(db, year_ago()).group_by(&db.post.owner_user).select(&db.post.tags_str).count_distinct();
    let mut v = Vec::new();
    (&ub).and(&ps).and((&dt).opt()).filt(|((_, p), _): (([i64; 4], [i64; 4]), Option<i64>)| p[3] > 1000).drive(|u, ((b, p), t)| v.push((u, b, p, t.unwrap_or(0))));
    out(v, |&(_, _, p, _)| (Reverse(p[2]), Reverse(p[0])), 50, |&(u, b, p, t)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&[p[0], p[1], p[2], p[3], t]));
        f.extend(ints(&b));
        f
    })
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(DISTINCT P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN P.ClosedDate IS NOT NULL THEN 1 ELSE 0 END) AS TotalClosedPosts,
// MAX(P.CreationDate) AS LastPostDate
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
// COUNT(*) AS TotalBadges,
// SUM(CASE WHEN Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
// FROM
// Badges
// GROUP BY
// UserId
// ),
// TopUsers AS (
// SELECT
// US.UserId,
// US.DisplayName,
// US.Reputation,
// US.TotalPosts,
// US.TotalQuestions,
// US.TotalAnswers,
// US.TotalClosedPosts,
// BC.TotalBadges,
// BC.GoldBadges,
// BC.SilverBadges,
// BC.BronzeBadges
// FROM
// UserStats US
// LEFT JOIN
// BadgeCounts BC ON US.UserId = BC.UserId
// ORDER BY
// US.Reputation DESC, US.TotalPosts DESC
// LIMIT 10
// )
// SELECT
// TU.DisplayName,
// TU.Reputation,
// TU.TotalPosts,
// TU.TotalQuestions,
// TU.TotalAnswers,
// TU.TotalClosedPosts,
// TU.TotalBadges,
// TU.GoldBadges,
// TU.SilverBadges,
// TU.BronzeBadges,
// (SELECT COUNT(*) FROM Posts P WHERE P.OwnerUserId = TU.UserId AND P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 YEAR') AS PostsLastYear
// FROM
// TopUsers TU;
fn q8546(db: &'static So) -> String {
    let ps = pstat(db, db.post.iq());
    let bc = badge_classes(db);
    let rp = owned_since(db, year_ago()).group_by(&db.post.owner_user).select(&db.post.score).fold(0i64, |a, _| a + 1);
    let top: MatSet<Id<User>> = whole(&db.user.id)
        .select(Ident::<User>::new().and(&db.user.reputation).and((&ps).opt()))
        .window(row_number, |((_, r), p): ((Id<User>, i64), Option<[i64; 13]>)| (r, pz(p)[0]), desc)
        .filt(|(_, n)| n <= 10)
        .map(|(((u, _), _), _)| u)
        .collect();
    let mut v = Vec::new();
    (&top).select(Ident::<User>::new().and((&ps).opt()).and((&bc).opt()).and((&rp).opt())).drive(|_, (((u, p), b), r)| v.push((u, pz(p), b, r.unwrap_or(0))));
    rows(v.iter().map(|&(u, p, b, r)| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(ints(&[p[0], p[1], p[2], p[11]]));
        f.extend(b.map_or(nulls(4), |b| ints(&b)));
        f.push(V::I(r));
        row(f)
    }))
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(DISTINCT B.Id) AS TotalBadges,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(COALESCE(P.ViewCount, 0)) AS TotalViews,
// SUM(COALESCE(P.Score, 0)) AS TotalScore,
// AVG(CASE WHEN P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' THEN P.Score ELSE NULL END) AS AvgScoreLastYear
// FROM Users U
// LEFT JOIN Badges B ON U.Id = B.UserId
// LEFT JOIN Posts P ON U.Id = P.OwnerUserId
// WHERE U.Reputation > 1000
// GROUP BY U.Id, U.DisplayName, U.Reputation
// ),
// EligibleUsers AS (
// SELECT
// UserId,
// DisplayName,
// Reputation,
// TotalBadges,
// QuestionCount,
// AnswerCount,
// TotalViews,
// TotalScore,
// AvgScoreLastYear
// FROM UserStats
// WHERE TotalViews > 1000
// AND AvgScoreLastYear > 3
// ),
// VotedPosts AS (
// SELECT
// P.Id AS PostId,
// COUNT(V.Id) AS VoteCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM Posts P
// LEFT JOIN Votes V ON P.Id = V.PostId
// WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 month'
// GROUP BY P.Id
// )
// SELECT
// EU.UserId,
// EU.DisplayName,
// EU.Reputation,
// EU.TotalBadges,
// EU.QuestionCount,
// EU.AnswerCount,
// EU.TotalViews,
// EU.TotalScore,
// VP.PostId,
// VP.VoteCount,
// VP.UpVotes,
// VP.DownVotes
// FROM EligibleUsers EU
// JOIN VotedPosts VP ON EU.UserId = VP.PostId
// ORDER BY EU.Reputation DESC, VP.VoteCount DESC
// LIMIT 100;
fn q6605(db: &'static So) -> String {
    let pid = pids(db);
    let bu = badges_per_user(db);
    let y = year_ago();
    let Post { post_type_id, view_count, score, creation_date, .. } = &db.post;
    let uf = user_base(db, UserWhere::RepGt(1000))
        .group_by(Ident::<User>::new())
        .select(badges_of(db).opt().and(posts_of(db).select(post_type_id.and(view_count.opt()).and(score).and(creation_date)).opt()))
        .fold([0i64; 6], move |a, (_, p)| match p {
            Some((((t, w), s), c)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + w.unwrap_or(0), a[3] + s, a[4] + (c >= y) as i64, a[5] + if c >= y { s } else { 0 }],
            None => a,
        });
    let eligible = (&uf).filt(|a: [i64; 6]| a[2] > 1000 && a[4] > 0 && a[5] as f64 / a[4] as f64 > 3.0);
    let vp = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64]);
    let m = month_ago();
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000))
        .select(Ident::<User>::new().and(&bu).and(eligible).and((&db.user.origid).select(&pid).with((&db.post.creation_date).ge(m)).select(Ident::<Post>::new().and((&vp).opt()))))
        .drive(|_, (((u, b), a), (p, x))| v.push((u, b, a, p, x.unwrap_or([0; 3]))));
    out(v, |&(u, _, _, _, x)| (rep_desc(db, u), Reverse(x[0])), 100, |&(u, b, a, p, x)| {
        let mut f = ["uid", "name", "rep"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.extend(ints(&[b, a[0], a[1], a[2], a[3]]));
        f.extend(post_fields(db, p, &["id"]));
        f.extend(ints(&x));
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
// SUM(CASE WHEN p.PostTypeId = 1 AND p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers,
// AVG(EXTRACT(EPOCH FROM (p.LastActivityDate - p.CreationDate))) AS AvgPostLifeSeconds
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
// COUNT(ph.Id) AS TotalHistoryEdits,
// SUM(CASE WHEN ph.PostHistoryTypeId IN (4, 5, 6) THEN 1 ELSE 0 END) AS TotalTitleBodyTagEdits
// FROM
// PostHistory ph
// GROUP BY
// ph.UserId
// )
// SELECT
// ups.UserId,
// ups.DisplayName,
// ups.TotalPosts,
// ups.Questions,
// ups.Answers,
// ups.AcceptedAnswers,
// ups.AvgPostLifeSeconds,
// COALESCE(ub.GoldBadges, 0) AS GoldBadges,
// COALESCE(ub.SilverBadges, 0) AS SilverBadges,
// COALESCE(ub.BronzeBadges, 0) AS BronzeBadges,
// COALESCE(phs.TotalHistoryEdits, 0) AS TotalHistoryEdits,
// COALESCE(phs.TotalTitleBodyTagEdits, 0) AS TotalTitleBodyTagEdits
// FROM
// UserPostStats ups
// LEFT JOIN
// UserBadges ub ON ups.UserId = ub.UserId
// LEFT JOIN
// PostHistoryStats phs ON ups.UserId = phs.UserId
// ORDER BY
// ups.TotalPosts DESC, ups.DisplayName;
fn q29019(db: &'static So) -> String {
    let Post { post_type_id, accepted_answer_id, last_activity_date, creation_date, .. } = &db.post;
    let uf = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(accepted_answer_id.opt()).and(last_activity_date).and(creation_date)).fold(([0i64; 4], 0i128), |(a, e), (((t, acc), l), c)| {
        ([a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (t == 1 && acc.is_some()) as i64], e + (l - c) as i128)
    });
    let bc = badge_classes(db);
    let ph = db.post_history.group_by(&db.post_history.user).select(&db.post_history.post_history_type_id).fold([0i64; 2], |a, t| [a[0] + 1, a[1] + matches!(t, 4 | 5 | 6) as i64]);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&uf).opt()).and((&bc).opt()).and((&ph).opt())).drive(|_, (((u, a), b), h)| v.push((u, a.unwrap_or(([0; 4], 0)), bz(b), h.unwrap_or([0; 2]))));
    rows(v.iter().map(|&(u, (a, e), b, h)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a));
        f.push(if a[0] == 0 { V::Null } else { V::F(e as f64 / a[0] as f64 / 1e6) });
        f.extend(ints(&[b[1], b[2], b[3], h[0], h[1]]));
        row(f)
    }))
}

// WITH UserBadges AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(B.Id) AS BadgeCount,
// SUM(CASE WHEN B.Class = 1 THEN 1 ELSE 0 END) AS GoldCount,
// SUM(CASE WHEN B.Class = 2 THEN 1 ELSE 0 END) AS SilverCount,
// SUM(CASE WHEN B.Class = 3 THEN 1 ELSE 0 END) AS BronzeCount
// FROM
// Users U
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// GROUP BY
// U.Id, U.DisplayName, U.Reputation
// ),
// PostStatistics AS (
// SELECT
// P.OwnerUserId,
// COUNT(P.Id) AS PostCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(CASE WHEN P.PostTypeId IN (4, 5) THEN 1 ELSE 0 END) AS TagWikis,
// AVG(P.Score) AS AverageScore
// FROM
// Posts P
// WHERE
// P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY
// P.OwnerUserId
// ),
// UserEngagement AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COALESCE(P.PostCount, 0) AS PostCount,
// COALESCE(B.BadgeCount, 0) AS BadgeCount,
// COALESCE(P.AverageScore, 0) AS AverageScore
// FROM
// Users U
// LEFT JOIN
// PostStatistics P ON U.Id = P.OwnerUserId
// LEFT JOIN
// UserBadges B ON U.Id = B.UserId
// )
// SELECT
// UEG.UserId,
// UEG.DisplayName,
// UEG.PostCount,
// UEG.BadgeCount,
// UEG.AverageScore,
// CASE
// WHEN UEG.BadgeCount > 5 THEN 'High Activity'
// WHEN UEG.PostCount > 50 THEN 'Frequent Contributor'
// ELSE 'New User'
// END AS UserCategory
// FROM
// UserEngagement UEG
// WHERE
// UEG.PostCount IS NOT NULL
// AND UEG.BadgeCount IS NOT NULL
// ORDER BY
// UEG.PostCount DESC, UEG.BadgeCount DESC;
fn q23241(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let ps = owned_since(db, year_ago()).group_by(&db.post.owner_user).select(&db.post.score).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and(&bu).and((&ps).opt())).drive(|_, ((u, b), p)| v.push((u, b, p.unwrap_or([0; 2]))));
    rows(v.iter().map(|&(u, b, p)| {
        let cat = if b > 5 { "High Activity" } else if p[0] > 50 { "Frequent Contributor" } else { "New User" };
        row(vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(p[0]), V::I(b), or0(p[1], p[0]), V::S(cat)])
    }))
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(CASE WHEN p.PostTypeId = 1 AND p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers,
// COUNT(DISTINCT c.Id) AS CommentCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
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
// CombinedStats AS (
// SELECT
// us.UserId,
// us.DisplayName,
// us.Reputation,
// us.PostCount,
// us.Questions,
// us.Answers,
// us.AcceptedAnswers,
// us.CommentCount,
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
// Questions,
// Answers,
// AcceptedAnswers,
// CommentCount,
// BadgeCount,
// GoldBadges,
// SilverBadges,
// BronzeBadges
// FROM
// CombinedStats
// ORDER BY
// Reputation DESC, PostCount DESC
// FETCH FIRST 10 ROWS ONLY;
fn q7840(db: &'static So) -> String {
    let Post { post_type_id, accepted_answer_id, .. } = &db.post;
    let pn = owned(db).group_by(&db.post.owner_user).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let pr = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(accepted_answer_id.opt()).and(comments_of(db).opt())).fold([0i64; 5], |a, ((t, acc), c)| {
        [0, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (t == 1 && acc.is_some()) as i64, a[4] + c.is_some() as i64]
    });
    let uf: HashIdx<Id<User>, [i64; 5]> = (&pr).and(&pn).map(|(a, n): ([i64; 5], i64)| [n, a[1], a[2], a[3], a[4]]).collect();
    let bc = badge_classes(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&uf).opt()).and((&bc).opt())).drive(|_, ((u, a), b)| v.push((u, a.unwrap_or([0; 5]), bz(b))));
    out(v, |&(u, a, _)| (rep_desc(db, u), Reverse(a[0])), 10, |&(u, a, b)| {
        let mut f = ["uid", "name", "rep"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.extend(ints(&a));
        f.extend(ints(&b));
        f
    })
}

// WITH UserBadges AS (
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
// PostStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(CASE WHEN p.PostTypeId = 1 THEN 1 END) AS QuestionsAsked,
// COUNT(CASE WHEN p.PostTypeId = 2 THEN 1 END) AS AnswersGiven,
// SUM(COALESCE(p.Score, 0)) AS TotalScore,
// COUNT(DISTINCT c.Id) AS CommentCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// GROUP BY
// p.OwnerUserId
// ),
// UserMetrics AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COALESCE(ub.GoldBadges, 0) AS GoldBadges,
// COALESCE(ub.SilverBadges, 0) AS SilverBadges,
// COALESCE(ub.BronzeBadges, 0) AS BronzeBadges,
// COALESCE(ps.QuestionsAsked, 0) AS QuestionsAsked,
// COALESCE(ps.AnswersGiven, 0) AS AnswersGiven,
// COALESCE(ps.TotalScore, 0) AS TotalScore,
// COALESCE(ps.CommentCount, 0) AS CommentCount
// FROM
// Users u
// LEFT JOIN
// UserBadges ub ON u.Id = ub.UserId
// LEFT JOIN
// PostStats ps ON u.Id = ps.OwnerUserId
// )
// SELECT
// um.UserId,
// um.DisplayName,
// um.GoldBadges,
// um.SilverBadges,
// um.BronzeBadges,
// um.QuestionsAsked,
// um.AnswersGiven,
// um.TotalScore,
// um.CommentCount,
// CASE
// WHEN um.TotalScore > 1000 THEN 'High Contributor'
// WHEN um.TotalScore BETWEEN 500 AND 1000 THEN 'Medium Contributor'
// ELSE 'Low Contributor'
// END AS ContributionLevel
// FROM
// UserMetrics um
// WHERE
// um.TotalScore IS NOT NULL
// ORDER BY
// um.TotalScore DESC
// FETCH FIRST 10 ROWS ONLY;
fn q3047(db: &'static So) -> String {
    let bc = badge_classes(db);
    let Post { post_type_id, score, .. } = &db.post;
    let ps = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(score).and(comments_of(db).opt())).fold([0i64; 4], |a, ((t, s), c)| {
        [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + s, a[3] + c.is_some() as i64]
    });
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&bc).opt()).and((&ps).opt())).drive(|_, ((u, b), p)| v.push((u, bz(b), p.unwrap_or([0; 4]))));
    out(v, |&(_, _, p)| Reverse(p[2]), 10, |&(u, b, p)| {
        let lvl = if p[2] > 1000 { "High Contributor" } else if (500..=1000).contains(&p[2]) { "Medium Contributor" } else { "Low Contributor" };
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&[b[1], b[2], b[3], p[0], p[1], p[2], p[3]]));
        f.push(V::S(lvl));
        f
    })
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(DISTINCT B.Id) AS BadgeCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN P.Score ELSE 0 END) AS QuestionScore,
// SUM(CASE WHEN P.PostTypeId = 2 THEN P.Score ELSE 0 END) AS AnswerScore,
// COUNT(DISTINCT P.Id) AS PostCount
// FROM
// Users U
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id, U.DisplayName, U.Reputation
// ),
// PostInteraction AS (
// SELECT
// P.OwnerUserId,
// COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
// SUM(CASE WHEN V.CreationDate IS NOT NULL THEN 1 ELSE 0 END) AS VoteCount,
// COUNT(DISTINCT PL.RelatedPostId) AS RelatedPostsCount
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// LEFT JOIN
// PostLinks PL ON P.Id = PL.PostId
// GROUP BY
// P.OwnerUserId
// ),
// CombinedStats AS (
// SELECT
// US.UserId,
// US.DisplayName,
// US.Reputation,
// US.BadgeCount,
// US.QuestionScore,
// US.AnswerScore,
// US.PostCount,
// COALESCE(PI.CommentCount, 0) AS CommentCount,
// COALESCE(PI.VoteCount, 0) AS VoteCount,
// COALESCE(PI.RelatedPostsCount, 0) AS RelatedPostsCount
// FROM
// UserStats US
// LEFT JOIN
// PostInteraction PI ON US.UserId = PI.OwnerUserId
// )
// SELECT
// UserId,
// DisplayName,
// Reputation,
// BadgeCount,
// QuestionScore,
// AnswerScore,
// PostCount,
// CommentCount,
// VoteCount,
// RelatedPostsCount
// FROM
// CombinedStats
// WHERE
// Reputation > 1000
// ORDER BY
// Reputation DESC, BadgeCount DESC, PostCount DESC
// FETCH FIRST 100 ROWS ONLY;
fn q8049(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let Post { post_type_id, score, .. } = &db.post;
    let pn = owned(db).group_by(&db.post.owner_user).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let us = user_base(db, UserWhere::RepGt(1000)).group_by(Ident::<User>::new()).select(badges_of(db).opt().and(posts_of(db).select(post_type_id.and(score)))).fold([0i64; 2], |a, (_, (t, s))| {
        [a[0] + if t == 1 { s } else { 0 }, a[1] + if t == 2 { s } else { 0 }]
    });
    let uf: HashIdx<Id<User>, [i64; 3]> = (&us).and(&pn).map(|(a, n): ([i64; 2], i64)| [a[0], a[1], n]).collect();
    let lk: HashIdx<Id<Post>, Id<PostLink>> = (&db.post_link.post).inv().collect();
    let pi = owned(db)
        .with((&db.post.owner_user).select(Ident::<User>::new().with((&db.user.reputation).gt(1000))))
        .group_by(&db.post.owner_user)
        .select(comments_of(db).opt().and(votes_of(db).opt()).and(links_of(db).opt()))
        .fold([0i64; 2], |a, ((c, x), _)| [a[0] + c.is_some() as i64, a[1] + x.is_some() as i64]);
    let dl = owned(db).group_by(&db.post.owner_user).select((&lk).select(&db.post_link.related_post_id)).count_distinct();
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and(&bu).and((&uf).opt()).and((&pi).opt()).and((&dl).opt())).drive(|_, x| v.push(x));
    out(v, |&((((u, b), a), _), _)| (rep_desc(db, u), Reverse(b), Reverse(a.map_or(0, |a| a[2]))), 100, |&((((u, b), a), p), d)| {
        let a = a.unwrap_or([0; 3]);
        let p = p.unwrap_or([0; 2]);
        let mut f = ["uid", "name", "rep"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.extend(ints(&[b, a[0], a[1], a[2], p[0], p[1], d.unwrap_or(0)]));
        f
    })
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// U.Views,
// U.UpVotes,
// U.DownVotes,
// COUNT(DISTINCT P.Id) AS PostCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN P.ClosedDate IS NOT NULL THEN 1 ELSE 0 END) AS ClosedPostCount
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// WHERE
// U.Reputation > 1000
// GROUP BY
// U.Id, U.DisplayName, U.Reputation, U.Views, U.UpVotes, U.DownVotes
// ),
// TopBadges AS (
// SELECT
// B.UserId,
// COUNT(*) AS BadgeCount
// FROM
// Badges B
// WHERE
// B.Class = 1
// GROUP BY
// B.UserId
// ),
// PostHistoryCounts AS (
// SELECT
// PH.UserId,
// COUNT(*) AS EditCount
// FROM
// PostHistory PH
// WHERE
// PH.PostHistoryTypeId IN (4, 5, 6)
// GROUP BY
// PH.UserId
// ),
// FinalStats AS (
// SELECT
// US.UserId,
// US.DisplayName,
// US.Reputation,
// US.Views,
// US.UpVotes,
// US.DownVotes,
// US.PostCount,
// US.QuestionCount,
// US.AnswerCount,
// US.ClosedPostCount,
// COALESCE(TB.BadgeCount, 0) AS GoldBadgeCount,
// COALESCE(PHC.EditCount, 0) AS EditCount
// FROM
// UserStats US
// LEFT JOIN
// TopBadges TB ON US.UserId = TB.UserId
// LEFT JOIN
// PostHistoryCounts PHC ON US.UserId = PHC.UserId
// )
// SELECT
// UserId,
// DisplayName,
// Reputation,
// Views,
// UpVotes,
// DownVotes,
// PostCount,
// QuestionCount,
// AnswerCount,
// ClosedPostCount,
// GoldBadgeCount,
// EditCount
// FROM
// FinalStats
// ORDER BY
// Reputation DESC, PostCount DESC
// LIMIT 10;
fn q5737(db: &'static So) -> String {
    let ps = pstat(db, db.post.iq());
    let gb = db.badge.with((&db.badge.class).eq(1)).group_by(&db.badge.user).select(&db.badge.class).fold(0i64, |a, _| a + 1);
    let ed = db.post_history.with((&db.post_history.post_history_type_id).filt(|t: i64| matches!(t, 4 | 5 | 6))).group_by(&db.post_history.user).select(&db.post_history.post).fold(0i64, |a, _| a + 1);
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and((&ps).opt()).and((&gb).opt()).and((&ed).opt())).drive(|_, (((u, p), g), e)| v.push((u, pz(p), g.unwrap_or(0), e.unwrap_or(0))));
    out(v, |&(u, p, ..)| (rep_desc(db, u), Reverse(p[0])), 10, |&(u, p, g, e)| {
        let mut f = ["uid", "name", "rep", "uviews", "uup", "udown"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.extend(ints(&[p[0], p[1], p[2], p[11], g, e]));
        f
    })
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
// PostStatistics AS (
// SELECT
// P.OwnerUserId,
// COUNT(C.Id) AS CommentCount,
// COALESCE(SUM(P.ViewCount), 0) AS TotalViews,
// AVG(P.Score) AS AverageScore,
// SUM(CASE WHEN P.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// WHERE
// P.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'
// GROUP BY
// P.OwnerUserId
// ),
// UserMetrics AS (
// SELECT
// U.Id,
// U.DisplayName,
// COALESCE(UB.BadgeCount, 0) AS BadgeCount,
// COALESCE(PS.CommentCount, 0) AS TotalComments,
// COALESCE(PS.TotalViews, 0) AS TotalViews,
// COALESCE(PS.AverageScore, 0) AS AverageScore,
// COALESCE(PS.AcceptedAnswers, 0) AS AcceptedAnswers
// FROM
// Users U
// LEFT JOIN
// UserBadges UB ON U.Id = UB.UserId
// LEFT JOIN
// PostStatistics PS ON U.Id = PS.OwnerUserId
// )
// SELECT
// U.DisplayName,
// U.BadgeCount,
// U.TotalComments,
// U.TotalViews,
// U.AverageScore,
// U.AcceptedAnswers,
// CASE
// WHEN U.BadgeCount >= 10 THEN 'Star Contributor'
// WHEN U.TotalComments > 100 THEN 'Comment King'
// WHEN U.TotalViews > 1000 THEN 'View Magnet'
// ELSE 'New Contributor'
// END AS ContributorLevel
// FROM
// UserMetrics U
// WHERE
// U.TotalComments > 0
// ORDER BY
// U.TotalViews DESC
// LIMIT 10;
fn q1345(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let Post { view_count, score, accepted_answer_id, .. } = &db.post;
    let ps = owned_since(db, date(2023, 10, 1))
        .group_by(&db.post.owner_user)
        .select(view_count.opt().and(score).and(accepted_answer_id.opt()).and(comments_of(db).opt()))
        .fold([0i64; 5], |a, (((w, s), acc), c)| [a[0] + c.is_some() as i64, a[1] + w.unwrap_or(0), a[2] + s, a[3] + 1, a[4] + acc.is_some() as i64]);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and(&bu).and((&ps).filt(|p: [i64; 5]| p[0] > 0))).drive(|_, ((u, b), p)| v.push((u, b, p)));
    out(v, |&(_, _, p)| Reverse(p[1]), 10, |&(u, b, p)| {
        let lvl = if b >= 10 { "Star Contributor" } else if p[0] > 100 { "Comment King" } else if p[1] > 1000 { "View Magnet" } else { "New Contributor" };
        vec![user_col(db, u, "name"), V::I(b), V::I(p[0]), V::I(p[1]), avg(p[2], p[3]), V::I(p[4]), V::S(lvl)]
    })
}

// WITH UserBadges AS (
// SELECT
// u.Id AS UserId,
// COUNT(b.Id) FILTER (WHERE b.Class = 1) AS GoldBadges,
// COUNT(b.Id) FILTER (WHERE b.Class = 2) AS SilverBadges,
// COUNT(b.Id) FILTER (WHERE b.Class = 3) AS BronzeBadges,
// COUNT(b.Id) AS TotalBadges,
// SUM(CASE WHEN b.Date >= DATE '2024-10-01' - INTERVAL '1 year' THEN 1 ELSE 0 END) AS RecentBadges
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
// AVG(p.Score) AS AvgScore
// FROM Posts p
// GROUP BY p.OwnerUserId
// ),
// RecentComments AS (
// SELECT
// c.UserId,
// COUNT(c.Id) AS CommentCount,
// COUNT(DISTINCT c.PostId) AS PostsCommented
// FROM Comments c
// WHERE c.CreationDate >= DATE '2024-10-01' - INTERVAL '30 days'
// GROUP BY c.UserId
// )
// SELECT
// u.DisplayName,
// COALESCE(ub.GoldBadges, 0) AS GoldBadges,
// COALESCE(ub.SilverBadges, 0) AS SilverBadges,
// COALESCE(ub.BronzeBadges, 0) AS BronzeBadges,
// COALESCE(ps.PostCount, 0) AS TotalPosts,
// COALESCE(ps.QuestionCount, 0) AS TotalQuestions,
// COALESCE(ps.AnswerCount, 0) AS TotalAnswers,
// COALESCE(ps.AvgScore, 0) AS AveragePostScore,
// COALESCE(rc.CommentCount, 0) AS RecentCommentCount,
// COALESCE(rc.PostsCommented, 0) AS RecentCommentedPosts,
// CASE
// WHEN COALESCE(ub.RecentBadges, 0) > 0 THEN 'Active'
// ELSE 'Inactive'
// END AS UserActivity
// FROM Users u
// LEFT JOIN UserBadges ub ON u.Id = ub.UserId
// LEFT JOIN PostStats ps ON u.Id = ps.OwnerUserId
// LEFT JOIN RecentComments rc ON u.Id = rc.UserId
// WHERE u.Reputation > 1000
// ORDER BY u.DisplayName;
fn q1974(db: &'static So) -> String {
    let d0 = date(2023, 10, 1);
    let ub = db.badge.group_by(&db.badge.user).select((&db.badge.class).and(&db.badge.date)).fold([0i64; 4], move |a, (c, d)| [a[0] + (c == 1) as i64, a[1] + (c == 2) as i64, a[2] + (c == 3) as i64, a[3] + (d >= d0) as i64]);
    let ps = pstat(db, db.post.iq());
    let rc = || db.comment.with((&db.comment.creation_date).ge(date(2024, 9, 1)));
    let rn = rc().group_by(&db.comment.user).select(&db.comment.post).fold(0i64, |a, _| a + 1);
    let rd = rc().group_by(&db.comment.user).select(&db.comment.post).count_distinct();
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and((&ub).opt()).and((&ps).opt()).and((&rn).opt()).and((&rd).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((((u, b), p), n), d)| {
        let b = b.unwrap_or([0; 4]);
        let p = pz(p);
        let mut f = vec![user_col(db, u, "name")];
        f.extend(ints(&[b[0], b[1], b[2], p[0], p[1], p[2]]));
        f.extend([or0(p[3], p[0]), V::I(n.unwrap_or(0)), V::I(d.unwrap_or(0)), V::S(if b[3] > 0 { "Active" } else { "Inactive" })]);
        row(f)
    }))
}

// WITH UserStatistics AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(DISTINCT P.Id) AS TotalPosts,
// COUNT(DISTINCT C.Id) AS TotalComments,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpvotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownvotes
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Comments C ON U.Id = C.UserId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// WHERE
// U.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'
// GROUP BY
// U.Id, U.DisplayName, U.Reputation
// ), UserBadgeStatistics AS (
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
// ), CombinedStatistics AS (
// SELECT
// US.UserId,
// US.DisplayName,
// US.Reputation,
// US.TotalPosts,
// US.TotalComments,
// US.TotalUpvotes,
// US.TotalDownvotes,
// COALESCE(UBS.TotalBadges, 0) AS TotalBadges,
// COALESCE(UBS.GoldBadges, 0) AS GoldBadges,
// COALESCE(UBS.SilverBadges, 0) AS SilverBadges,
// COALESCE(UBS.BronzeBadges, 0) AS BronzeBadges
// FROM
// UserStatistics US
// LEFT JOIN
// UserBadgeStatistics UBS ON US.UserId = UBS.UserId
// )
// SELECT
// UserId,
// DisplayName,
// Reputation,
// TotalPosts,
// TotalComments,
// TotalUpvotes,
// TotalDownvotes,
// TotalBadges,
// GoldBadges,
// SilverBadges,
// BronzeBadges
// FROM
// CombinedStatistics
// ORDER BY
// Reputation DESC, TotalPosts DESC
// LIMIT 10;
fn q6733(db: &'static So) -> String {
    let dp = ud(db, UserWhere::All, posts_of(db));
    let cu = comments_per_user(db);
    let uv = user_base(db, UserWhere::CreatedGe(date(2023, 10, 1)))
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt().and(comments_by(db).opt()))
        .fold([0i64; 2], |a, (t, _)| {
            let t = t.flatten();
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]
        });
    let bc = badge_classes(db);
    let mut v = Vec::new();
    user_base(db, UserWhere::CreatedGe(date(2023, 10, 1))).select(Ident::<User>::new().and((&dp).opt()).and(&cu).and((&uv).opt()).and((&bc).opt())).drive(|_, x| v.push(x));
    out(v, |&((((u, d), _), _), _)| (rep_desc(db, u), Reverse(d.unwrap_or(0))), 10, |&((((u, d), c), x), b)| {
        let x = x.unwrap_or([0; 2]);
        let mut f = ["uid", "name", "rep"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.extend(ints(&[d.unwrap_or(0), c, x[0], x[1]]));
        f.extend(ints(&bz(b)));
        f
    })
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN p.PostTypeId = 2 AND p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswerCount,
// SUM(CASE WHEN b.UserId IS NOT NULL THEN 1 ELSE 0 END) AS BadgeCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Badges b ON u.Id = b.UserId
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
// ),
// PostEngagement AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.ViewCount,
// p.Score,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= CURRENT_DATE - INTERVAL '30 days'
// GROUP BY
// p.Id, p.Title, p.ViewCount, p.Score
// ),
// UserPostEngagement AS (
// SELECT
// us.DisplayName,
// us.Reputation,
// us.TotalPosts,
// us.QuestionCount,
// us.AnswerCount,
// us.AcceptedAnswerCount,
// pe.PostId,
// pe.Title,
// pe.ViewCount,
// pe.Score,
// pe.CommentCount,
// pe.VoteCount
// FROM
// UserStats us
// JOIN
// PostEngagement pe ON us.UserId = pe.PostId
// )
// SELECT
// DisplayName AS UserDisplayName,
// Reputation,
// TotalPosts,
// QuestionCount,
// AnswerCount,
// AcceptedAnswerCount,
// Title AS RecentPostTitle,
// ViewCount,
// Score,
// CommentCount,
// VoteCount
// FROM
// UserPostEngagement
// ORDER BY
// Reputation DESC,
// TotalPosts DESC
// LIMIT 10;
fn q7611(db: &'static So) -> String {
    let uid = uids(db);
    let Post { post_type_id, accepted_answer_id, .. } = &db.post;
    let dp = ud(db, UserWhere::All, posts_of(db));
    let ub = g(db).select(posts_of(db).select(post_type_id.and(accepted_answer_id.opt())).opt().and(badges_of(db).opt())).fold([0i64; 4], |a, (p, _)| match p {
        Some((t, acc)) => [0, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (t == 2 && acc.is_some()) as i64],
        None => a,
    });
    let cut = current_date() - 30 * 86_400_000_000;
    let mut v = Vec::new();
    stats_fold(db, db.post.with((&db.post.creation_date).ge(cut)), Ident::<Post>::new(), "cv", &[])
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and((&dp).opt()).and(&ub)))
        .drive(|p, (s, ((u, d), a))| v.push((p, s, u, d.unwrap_or(0), a)));
    out(v, |&(_, _, u, d, _)| (rep_desc(db, u), Reverse(d)), 10, |&(p, s, u, d, a)| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(ints(&[d, a[1], a[2], a[3]]));
        f.extend(post_fields(db, p, &["title", "views", "score"]));
        f.extend(ints(&[s.cx, s.vx]));
        f
    })
}

// WITH UserVoteStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// COUNT(DISTINCT P.Id) AS PostCount
// FROM
// Users U
// LEFT JOIN
// Votes V ON U.Id = V.UserId
// LEFT JOIN
// Posts P ON P.OwnerUserId = U.Id
// GROUP BY
// U.Id, U.DisplayName
// ),
// PostInteraction AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// COALESCE(COUNT(Cm.Id), 0) AS CommentCount,
// COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount,
// COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount,
// P.OwnerUserId
// FROM
// Posts P
// LEFT JOIN
// Comments Cm ON Cm.PostId = P.Id
// LEFT JOIN
// Votes V ON V.PostId = P.Id
// WHERE
// P.CreationDate >= (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year')
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.OwnerUserId
// ),
// ClosedPostStats AS (
// SELECT
// PH.PostId,
// COUNT(*) AS CloseCount,
// MAX(PH.CreationDate) AS LastClosedDate
// FROM
// PostHistory PH
// WHERE
// PH.PostHistoryTypeId = 10
// GROUP BY
// PH.PostId
// )
// SELECT
// U.UserId,
// U.DisplayName,
// U.UpVotes,
// U.DownVotes,
// PI.PostId,
// PI.Title,
// PI.CreationDate,
// PI.CommentCount,
// PI.UpVoteCount,
// PI.DownVoteCount,
// CPS.CloseCount,
// CPS.LastClosedDate
// FROM
// UserVoteStats U
// JOIN
// PostInteraction PI ON U.UserId = PI.OwnerUserId
// LEFT JOIN
// ClosedPostStats CPS ON PI.PostId = CPS.PostId
// WHERE
// (U.UpVotes - U.DownVotes) > 10
// AND (PI.CommentCount > 5 OR PI.UpVoteCount > 20)
// ORDER BY
// U.DisplayName, PI.CreationDate DESC;
fn q967(db: &'static So) -> String {
    let uvs = g(db).select(votes_by(db).select(&db.vote.vote_type_id).opt().and(posts_of(db).opt())).fold([0i64; 2], |a, (t, _)| [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64]);
    let cl = db.post_history.with((&db.post_history.post_history_type_id).eq(10)).group_by(&db.post_history.post).select(&db.post_history.creation_date).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let mut v = Vec::new();
    stats_fold(db, owned_since(db, year_ago()), Ident::<Post>::new(), "cv", &[])
        .filt(|s: Stats| s.cx > 5 || s.up > 20)
        .and((&db.post.owner_user).select(Ident::<User>::new().and((&uvs).filt(|a: [i64; 2]| a[0] - a[1] > 10))))
        .and((&cl).opt())
        .drive(|p, ((s, (u, x)), c)| v.push((p, s, u, x[0], x[1], c)));
    rows(v.iter().map(|&(p, s, u, up, dn, c)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(up), V::I(dn)];
        f.extend(post_fields(db, p, &["id", "title", "created"]));
        f.extend(ints(&[s.cx, s.up, s.down]));
        f.extend([oint(c.map(|c| c.0)), ots(c.map(|c| c.1))]);
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
// SUM(CASE WHEN p.PostTypeId = 1 AND p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers,
// SUM(CASE WHEN p.ViewCount > 100 THEN 1 ELSE 0 END) AS PopularPosts
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
// us.UserId,
// us.DisplayName,
// us.Reputation,
// us.PostCount,
// us.QuestionCount,
// us.AnswerCount,
// us.AcceptedAnswers,
// us.PopularPosts,
// COALESCE(bc.BadgeCount, 0) AS BadgeCount,
// COALESCE(bc.GoldBadges, 0) AS GoldBadges,
// COALESCE(bc.SilverBadges, 0) AS SilverBadges,
// COALESCE(bc.BronzeBadges, 0) AS BronzeBadges
// FROM
// UserStats us
// LEFT JOIN
// BadgeCounts bc ON us.UserId = bc.UserId
// )
// SELECT
// c.DisplayName,
// c.Reputation,
// c.PostCount,
// c.QuestionCount,
// c.AnswerCount,
// c.AcceptedAnswers,
// c.PopularPosts,
// c.BadgeCount,
// c.GoldBadges,
// c.SilverBadges,
// c.BronzeBadges
// FROM
// CombinedStats c
// WHERE
// c.Reputation > 1000
// AND c.PostCount > 10
// ORDER BY
// c.Reputation DESC,
// c.PostCount DESC
// LIMIT 100;
fn q7281(db: &'static So) -> String {
    let Post { post_type_id, accepted_answer_id, view_count, .. } = &db.post;
    let uf = owned(db).group_by(&db.post.owner_user).select(post_type_id.and(accepted_answer_id.opt()).and(view_count.opt())).fold([0i64; 5], |a, ((t, acc), w)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + (t == 1 && acc.is_some()) as i64, a[4] + (w.unwrap_or(0) > 100) as i64]
    });
    let bc = badge_classes(db);
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and((&uf).filt(|a: [i64; 5]| a[0] > 10)).and((&bc).opt())).drive(|_, ((u, a), b)| v.push((u, a, bz(b))));
    out(v, |&(u, a, _)| (rep_desc(db, u), Reverse(a[0])), 100, |&(u, a, b)| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep")];
        f.extend(ints(&a));
        f.extend(ints(&b));
        f
    })
}

// WITH UserPostStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(DISTINCT P.Id) AS TotalPosts,
// COUNT(DISTINCT C.Id) AS TotalComments,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionsCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswersCount,
// SUM(P.ViewCount) AS TotalViews,
// SUM(P.Score) AS TotalScore,
// SUM(V.BountyAmount) AS TotalBounty
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
// PostEngagement AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.ViewCount,
// P.Score,
// COALESCE(C.CommentCount, 0) AS CommentCount,
// COALESCE(B.BadgeCount, 0) AS BadgeCount,
// P.OwnerUserId
// FROM
// Posts P
// LEFT JOIN (
// SELECT
// PostId,
// COUNT(*) AS CommentCount
// FROM
// Comments
// GROUP BY
// PostId
// ) C ON P.Id = C.PostId
// LEFT JOIN (
// SELECT
// UserId,
// COUNT(*) AS BadgeCount
// FROM
// Badges
// GROUP BY
// UserId
// ) B ON P.OwnerUserId = B.UserId
// WHERE
// P.CreationDate >= DATE('2024-10-01') - INTERVAL '30 days'
// )
// SELECT
// UPS.UserId,
// UPS.DisplayName,
// UPS.TotalPosts,
// UPS.TotalComments,
// UPS.QuestionsCount,
// UPS.AnswersCount,
// UPS.TotalViews,
// UPS.TotalScore,
// UPS.TotalBounty,
// PE.PostId,
// PE.Title,
// PE.CreationDate,
// PE.ViewCount,
// PE.Score,
// PE.CommentCount,
// PE.BadgeCount
// FROM
// UserPostStats UPS
// LEFT JOIN
// PostEngagement PE ON UPS.UserId = PE.OwnerUserId
// ORDER BY
// UPS.TotalScore DESC, UPS.TotalPosts DESC;
fn q14507(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "cv", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let dc = udc(db);
    let cp = comments_per_post(db);
    let bu = badges_per_user(db);
    let d0 = date(2024, 9, 1);
    let mut v = Vec::new();
    (&us).and((&dp).opt()).and((&dc).opt()).and(&bu).and(posts_of(db).with((&db.post.creation_date).ge(d0)).select(Ident::<Post>::new().and(&cp)).opt()).drive(|u, ((((a, d), c), b), p)| v.push((u, a, d.unwrap_or(0), c.unwrap_or(0), b, p)));
    rows(v.iter().map(|&(u, a, d, c, b, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&[d, c, a.q, a.a]));
        f.extend(["views_sum", "score_sum", "bounty_sum"].iter().map(|k| ustat_field(&a, k)));
        f.extend(match p {
            Some((p, n)) => {
                let mut g = post_fields(db, p, &["id", "title", "created", "views", "score"]);
                g.extend(ints(&[n, b]));
                g
            }
            None => nulls(7),
        });
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
// SUM(CASE WHEN P.PostTypeId = 1 AND P.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes
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
// CombinedStats AS (
// SELECT
// U.UserId,
// U.DisplayName,
// U.Reputation,
// U.TotalPosts,
// U.TotalQuestions,
// U.TotalAnswers,
// U.AcceptedAnswers,
// U.Upvotes,
// U.Downvotes,
// COALESCE(B.TotalBadges, 0) AS TotalBadges,
// COALESCE(B.GoldBadges, 0) AS GoldBadges,
// COALESCE(B.SilverBadges, 0) AS SilverBadges,
// COALESCE(B.BronzeBadges, 0) AS BronzeBadges
// FROM
// UserStats U
// LEFT JOIN
// BadgeStats B ON U.UserId = B.UserId
// )
// SELECT
// DisplayName,
// Reputation,
// TotalPosts,
// TotalQuestions,
// TotalAnswers,
// AcceptedAnswers,
// Upvotes,
// Downvotes,
// TotalBadges,
// GoldBadges,
// SilverBadges,
// BronzeBadges
// FROM
// CombinedStats
// ORDER BY
// Reputation DESC, TotalPosts DESC
// LIMIT 10;
fn q7580(db: &'static So) -> String {
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
    out(v, |&(u, _, d, _)| (rep_desc(db, u), Reverse(d)), 10, |&(u, a, d, b)| {
        let mut f = vec![user_col(db, u, "name"), user_col(db, u, "rep"), V::I(d)];
        f.extend(ints(&a));
        f.extend(ints(&b));
        f
    })
}

// WITH UserActivity AS (
// SELECT U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(DISTINCT P.Id) AS PostCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN UP.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN UP.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM Users U
// LEFT JOIN Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN Votes UP ON P.Id = UP.PostId
// GROUP BY U.Id, U.DisplayName, U.Reputation
// ),
// RecentPostActivity AS (
// SELECT P.OwnerUserId,
// COUNT(*) AS RecentPosts,
// MAX(P.CreationDate) AS LastPostDate
// FROM Posts P
// WHERE P.CreationDate > (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days')
// GROUP BY P.OwnerUserId
// ),
// AverageScores AS (
// SELECT P.OwnerUserId,
// AVG(P.Score) AS AvgScore
// FROM Posts P
// GROUP BY P.OwnerUserId
// ),
// ActiveUsers AS (
// SELECT UA.*,
// COALESCE(RPA.RecentPosts, 0) AS RecentPosts,
// COALESCE(RPA.LastPostDate, '1970-01-01'::DATE) AS LastPostDate,
// COALESCE(AScores.AvgScore, 0) AS AvgScore
// FROM UserActivity UA
// LEFT JOIN RecentPostActivity RPA ON UA.UserId = RPA.OwnerUserId
// LEFT JOIN AverageScores AScores ON UA.UserId = AScores.OwnerUserId
// )
// SELECT U.UserId,
// U.DisplayName,
// U.Reputation,
// U.PostCount,
// U.QuestionCount,
// U.AnswerCount,
// U.UpVotes,
// U.DownVotes,
// U.RecentPosts,
// U.LastPostDate,
// U.AvgScore,
// CASE
// WHEN U.Reputation > 1000 THEN 'Experienced'
// WHEN U.Reputation BETWEEN 500 AND 1000 THEN 'Moderate'
// ELSE 'Beginner'
// END AS UserLevel
// FROM ActiveUsers U
// WHERE U.RecentPosts > 5
// ORDER BY U.Reputation DESC, U.LastPostDate DESC;
fn q3697(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let rp = db.post.with((&db.post.creation_date).gt(month_ago())).group_by(&db.post.owner_user).select(&db.post.creation_date).fold((0i64, i64::MIN), |(n, m), d| (n + 1, m.max(d)));
    let sc = owned(db).group_by(&db.post.owner_user).select(&db.post.score).fold([0i64; 2], |a, s| [a[0] + 1, a[1] + s]);
    let mut v = Vec::new();
    (&us).and((&dp).opt()).and((&rp).filt(|r: (i64, i64)| r.0 > 5)).and((&sc).opt()).drive(|u, (((a, d), r), s)| v.push((u, a, d.unwrap_or(0), r, s.unwrap_or([0; 2]))));
    rows(v.iter().map(|&(u, a, d, r, s)| {
        let rep = db.user.reputation.get(u).unwrap();
        let lvl = if rep > 1000 { "Experienced" } else if (500..=1000).contains(&rep) { "Moderate" } else { "Beginner" };
        let mut f = ["uid", "name", "rep"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.extend(ints(&[d, a.q, a.a, a.up, a.down, r.0]));
        f.extend([V::T(r.1), or0(s[1], s[0]), V::S(lvl)]);
        row(f)
    }))
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS PositivePosts,
// AVG(COALESCE(p.Score, 0)) AS AvgScore,
// SUM(b.Class) AS TotalBadges
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN Badges b ON u.Id = b.UserId
// GROUP BY u.Id
// ),
// PostActivity AS (
// SELECT
// p.Id AS PostId,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// MAX(ph.CreationDate) AS LastEditDate
// FROM Posts p
// LEFT JOIN Comments c ON p.Id = c.PostId
// LEFT JOIN Votes v ON p.Id = v.PostId
// LEFT JOIN PostHistory ph ON p.Id = ph.PostId
// GROUP BY p.Id
// ),
// ClosedPosts AS (
// SELECT
// ph.PostId,
// MAX(ph.CreationDate) AS ClosedDate,
// MAX(CASE WHEN ph.PostHistoryTypeId = 10 THEN ph.Comment END) AS CloseReason
// FROM PostHistory ph
// WHERE ph.PostHistoryTypeId IN (10, 11)
// GROUP BY ph.PostId
// ),
// AllData AS (
// SELECT
// us.UserId,
// us.TotalPosts,
// us.PositivePosts,
// us.AvgScore,
// us.TotalBadges,
// pa.CommentCount,
// pa.VoteCount,
// pa.LastEditDate,
// cp.ClosedDate,
// cp.CloseReason
// FROM UserStats us
// LEFT JOIN PostActivity pa ON us.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = pa.PostId LIMIT 1)
// LEFT JOIN ClosedPosts cp ON pa.PostId = cp.PostId
// )
// SELECT
// UserId,
// TotalPosts,
// PositivePosts,
// AvgScore,
// TotalBadges,
// COALESCE(CommentCount, 0) AS TotalComments,
// COALESCE(VoteCount, 0) AS TotalVotes,
// LastEditDate,
// CASE
// WHEN ClosedDate IS NOT NULL THEN 'Closed'
// ELSE 'Active'
// END AS PostStatus,
// COALESCE(CloseReason, 'N/A') AS CloseReason
// FROM AllData
// WHERE TotalPosts > 10
// AND (AvgScore BETWEEN 0 AND 5 OR PositivePosts > 5)
// ORDER BY AvgScore DESC, TotalPosts DESC;
fn q24889(db: &'static So) -> String {
    let Post { score, .. } = &db.post;
    let pn = owned(db).group_by(&db.post.owner_user).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let pb = g(db).select(posts_of(db).select(score).opt().and(badges_of(db).select(&db.badge.class).opt())).fold([0i64; 5], |a, (s, c)| {
        [a[0] + 1, a[1] + (s.unwrap_or(0) > 0) as i64, a[2] + s.unwrap_or(0), a[3] + c.unwrap_or(0), a[4] + c.is_some() as i64]
    });
    let us = (&pn).and(&pb).filt(|(n, a): (i64, [i64; 5])| {
        let avg = a[2] as f64 / a[0] as f64;
        n > 10 && ((0.0..=5.0).contains(&avg) || a[1] > 5)
    });
    let sf = stats_fold(db, owned(db), Ident::<Post>::new(), "cvh", &[]);
    let cp = db.post_history.with((&db.post_history.post_history_type_id).filt(|t: i64| matches!(t, 10 | 11))).group_by(&db.post_history.post).select((&db.post_history.creation_date).and((&db.post_history.post_history_type_id).and((&db.post_history.comment).opt()))).fold((i64::MIN, None::<Str>), |(m, r), (d, (t, c))| {
        (m.max(d), if t == 10 { match (r, c) { (Some(a), Some(b)) => Some(if b > a { b } else { a }), (a, b) => a.or(b) } } else { r })
    });
    let mut v = Vec::new();
    db.user
        .select(Ident::<User>::new().and(us).and(posts_of(db).select(Ident::<Post>::new().and(&sf).and((&cp).opt()))))
        .drive(|_, ((u, (n, a)), ((p, s), cl))| v.push((u, n, a, p, s, cl)));
    rows(v.iter().map(|&(u, n, a, _, s, cl)| {
        let mut f = vec![user_col(db, u, "uid"), V::I(n), V::I(a[1]), V::F(a[2] as f64 / a[0] as f64), if a[4] == 0 { V::Null } else { V::I(a[3]) }];
        f.extend(ints(&[s.cx, s.vx]));
        f.push(if s.hx > 0 { V::T(s.hmax) } else { V::Null });
        f.push(V::S(if cl.is_some() { "Closed" } else { "Active" }));
        f.push(V::S(cl.and_then(|c| c.1).unwrap_or("N/A")));
        row(f)
    }))
}

// WITH UserBadges AS (
// SELECT
// u.Id AS UserId,
// SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
// SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
// SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges,
// COUNT(b.Id) AS TotalBadges
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
// AVG(p.ViewCount) AS AvgViews
// FROM
// Posts p
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY
// p.OwnerUserId
// ),
// HighVotePosts AS (
// SELECT
// p.OwnerUserId,
// COUNT(v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Posts p
// JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY
// p.OwnerUserId
// )
// SELECT
// u.DisplayName,
// COALESCE(ub.GoldBadges, 0) AS GoldBadges,
// COALESCE(ub.SilverBadges, 0) AS SilverBadges,
// COALESCE(ub.BronzeBadges, 0) AS BronzeBadges,
// COALESCE(ps.PostCount, 0) AS PostsWritten,
// COALESCE(ps.TotalScore, 0) AS TotalScores,
// COALESCE(ps.AvgViews, 0) AS AverageViews,
// COALESCE(hv.VoteCount, 0) AS TotalVotes,
// COALESCE(hv.UpVotes, 0) AS TotalUpVotes,
// COALESCE(hv.DownVotes, 0) AS TotalDownVotes
// FROM
// Users u
// LEFT JOIN
// UserBadges ub ON u.Id = ub.UserId
// LEFT JOIN
// PostStats ps ON u.Id = ps.OwnerUserId
// LEFT JOIN
// HighVotePosts hv ON u.Id = hv.OwnerUserId
// WHERE
// u.Reputation > 1000
// ORDER BY
// GoldBadges DESC, SilverBadges DESC, BronzeBadges DESC, TotalScores DESC;
fn q4682(db: &'static So) -> String {
    let bc = badge_classes(db);
    let ps = pstat(db, since(db, year_ago()));
    let hv = owned_since(db, year_ago()).group_by(&db.post.owner_user).select(votes_of(db).select(&db.vote.vote_type_id)).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64]);
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and((&bc).opt()).and((&ps).opt()).and((&hv).opt())).drive(|_, (((u, b), p), x)| v.push((u, bz(b), pz(p), x.unwrap_or([0; 3]))));
    rows(v.iter().map(|&(u, b, p, x)| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(ints(&[b[1], b[2], b[3], p[0], p[3]]));
        f.push(or0(p[5], p[4]));
        f.extend(ints(&x));
        row(f)
    }))
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN p.Id IS NOT NULL THEN 1 ELSE 0 END) AS TotalVotes
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// WHERE
// u.Reputation > 1000
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
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
// ),
// PostHistoryStats AS (
// SELECT
// ph.UserId,
// COUNT(*) AS TotalHistoryEntries,
// SUM(CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN 1 ELSE 0 END) AS ClosedPosts,
// SUM(CASE WHEN ph.PostHistoryTypeId IN (12, 13) THEN 1 ELSE 0 END) AS DeletedPosts
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
// us.TotalVotes,
// COALESCE(bs.TotalBadges, 0) AS TotalBadges,
// COALESCE(bs.GoldBadges, 0) AS GoldBadges,
// COALESCE(bs.SilverBadges, 0) AS SilverBadges,
// COALESCE(bs.BronzeBadges, 0) AS BronzeBadges,
// COALESCE(phs.TotalHistoryEntries, 0) AS TotalHistoryEntries,
// COALESCE(phs.ClosedPosts, 0) AS ClosedPosts,
// COALESCE(phs.DeletedPosts, 0) AS DeletedPosts
// FROM
// UserStats us
// LEFT JOIN
// BadgeStats bs ON us.UserId = bs.UserId
// LEFT JOIN
// PostHistoryStats phs ON us.UserId = phs.UserId
// ORDER BY
// us.Reputation DESC, us.TotalPosts DESC;
fn q6678(db: &'static So) -> String {
    let ps = pstat(db, db.post.iq());
    let bc = badge_classes(db);
    let ph = db.post_history.group_by(&db.post_history.user).select(&db.post_history.post_history_type_id).fold([0i64; 3], |a, t| [a[0] + 1, a[1] + matches!(t, 10 | 11) as i64, a[2] + matches!(t, 12 | 13) as i64]);
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and((&ps).opt()).and((&bc).opt()).and((&ph).opt())).drive(|_, (((u, p), b), h)| v.push((u, pz(p), bz(b), h.unwrap_or([0; 3]))));
    rows(v.iter().map(|&(u, p, b, h)| {
        let mut f = ["uid", "name", "rep"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.extend(ints(&[p[0], p[1], p[2], p[0]]));
        f.extend(ints(&b));
        f.extend(ints(&h));
        row(f)
    }))
}

// WITH UserVotes AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(V.Id) AS TotalVotes,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// SUM(CASE WHEN V.VoteTypeId IN (10, 12) THEN 1 ELSE 0 END) AS Deletions
// FROM
// Users U
// LEFT JOIN
// Votes V ON U.Id = V.UserId
// GROUP BY
// U.Id, U.DisplayName
// ), PostStats AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.ViewCount,
// P.Score,
// COUNT(C.Id) AS CommentCount,
// COUNT(DISTINCT COALESCE(B.UserId, -1)) AS BadgeCount,
// MAX(CASE WHEN PH.PostHistoryTypeId IN (10, 11) THEN 1 ELSE 0 END) AS IsClosed,
// SUM(CASE WHEN U.DisplayName IS NOT NULL THEN 1 ELSE 0 END) AS ActiveUserCount
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Badges B ON P.OwnerUserId = B.UserId
// LEFT JOIN
// PostHistory PH ON P.Id = PH.PostId
// LEFT JOIN
// Users U ON P.OwnerUserId = U.Id
// GROUP BY
// P.Id, P.Title, P.ViewCount, P.Score
// ), CombinedStats AS (
// SELECT
// U.DisplayName,
// U.TotalVotes,
// U.UpVotes,
// U.DownVotes,
// U.Deletions,
// PS.PostId,
// PS.Title,
// PS.ViewCount,
// PS.Score,
// PS.CommentCount,
// PS.BadgeCount,
// PS.IsClosed,
// PS.ActiveUserCount
// FROM
// UserVotes U
// JOIN
// PostStats PS ON U.UserId = PS.PostId
// )
// SELECT
// CS.DisplayName,
// CS.TotalVotes,
// CS.UpVotes,
// CS.DownVotes,
// CS.Deletions,
// CS.Title,
// CS.ViewCount,
// CS.Score,
// CS.CommentCount,
// CASE WHEN CS.IsClosed = 1 THEN 'Closed' ELSE 'Open' END AS PostStatus,
// CS.BadgeCount,
// CS.ActiveUserCount
// FROM
// CombinedStats CS
// WHERE
// CS.TotalVotes > 10
// ORDER BY
// CS.Score DESC, CS.ViewCount DESC;
fn q6440(db: &'static So) -> String {
    let pid = pids(db);
    let uv = db.vote.group_by(&db.vote.user).select(&db.vote.vote_type_id).fold([0i64; 4], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64, a[3] + matches!(t, 10 | 12) as i64]);
    let hv: MatSet<Id<Post>> = db.user.with((&uv).filt(|x: [i64; 4]| x[0] > 10)).select((&db.user.origid).select(&pid)).collect();
    let owner = &db.post.owner_user;
    let bidx: HashIdx<i64, Id<Badge>> = (&db.badge.user_id).inv().collect();
    let ob = || (&db.post.owner_user_id).select(&bidx);
    let ps = (&hv)
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(ob().opt()).and(history_of(db).select(&db.post_history.post_history_type_id).opt()).and(owner.opt()))
        .fold([0i64; 3], |a, (((c, _), h), u)| [a[0] + c.is_some() as i64, a[1].max(matches!(h, Some(10 | 11)) as i64), a[2] + u.is_some() as i64]);
    let bd = (&hv).group_by(Ident::<Post>::new()).select(ob().select(&db.badge.user_id).opt().map(|b: Option<i64>| b.unwrap_or(-1))).count_distinct();
    let mut v = Vec::new();
    db.user
        .select(Ident::<User>::new().and((&uv).filt(|x: [i64; 4]| x[0] > 10)).and((&db.user.origid).select(&pid).select(Ident::<Post>::new().and(&ps).and(&bd))))
        .drive(|_, x| v.push(x));
    rows(v.iter().map(|&((u, x), ((p, a), bd))| {
        let mut f = vec![user_col(db, u, "name")];
        f.extend(ints(&x));
        f.extend(post_fields(db, p, &["title", "views", "score"]));
        f.push(V::I(a[0]));
        f.push(V::S(if a[1] > 0 { "Closed" } else { "Open" }));
        f.extend(ints(&[bd, a[2]]));
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
// COALESCE(SUM(V.BountyAmount), 0) AS TotalBounty,
// COALESCE(SUM(CASE WHEN V.UserId IS NOT NULL THEN 1 ELSE 0 END), 0) AS TotalVotes
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
// BadgeCounts AS (
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
// PostEngagement AS (
// SELECT
// P.OwnerUserId,
// COUNT(C.Id) AS TotalComments,
// SUM(CASE WHEN P.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS AcceptedAnswers
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// GROUP BY
// P.OwnerUserId
// )
// SELECT
// UA.UserId,
// UA.DisplayName,
// UA.TotalPosts,
// UA.TotalQuestions,
// UA.TotalAnswers,
// UA.TotalBounty,
// UA.TotalVotes,
// COALESCE(BC.TotalBadges, 0) AS TotalBadges,
// COALESCE(BC.GoldBadges, 0) AS GoldBadges,
// COALESCE(BC.SilverBadges, 0) AS SilverBadges,
// COALESCE(BC.BronzeBadges, 0) AS BronzeBadges,
// COALESCE(PE.TotalComments, 0) AS TotalComments,
// COALESCE(PE.AcceptedAnswers, 0) AS AcceptedAnswers
// FROM
// UserActivity UA
// LEFT JOIN
// BadgeCounts BC ON UA.UserId = BC.UserId
// LEFT JOIN
// PostEngagement PE ON UA.UserId = PE.OwnerUserId
// ORDER BY
// UA.TotalPosts DESC, UA.TotalVotes DESC;
fn q8002(db: &'static So) -> String {
    let Post { post_type_id, .. } = &db.post;
    let pn = owned(db).group_by(&db.post.owner_user).select(Ident::<Post>::new()).fold(0i64, |n, _| n + 1);
    let pv = owned(db)
        .group_by(&db.post.owner_user)
        .select(post_type_id.and(votes_of(db).select((&db.vote.bounty_amount).opt().and((&db.vote.user_id).opt())).opt()))
        .fold([0i64; 5], |a, (t, v)| {
            let (b, u) = v.map_or((None, None), |x| x);
            [0, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + b.unwrap_or(0), a[4] + u.is_some() as i64]
        });
    let uf: HashIdx<Id<User>, [i64; 5]> = (&pv).and(&pn).map(|(a, n): ([i64; 5], i64)| [n, a[1], a[2], a[3], a[4]]).collect();
    let bc = badge_classes(db);
    let pe = owned(db).group_by(&db.post.owner_user).select(comments_of(db).opt().and((&db.post.accepted_answer_id).opt())).fold([0i64; 2], |a, (c, acc)| [a[0] + c.is_some() as i64, a[1] + acc.is_some() as i64]);
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and((&uf).opt()).and((&bc).opt()).and((&pe).opt())).drive(|_, (((u, a), b), e)| v.push((u, a.unwrap_or([0; 5]), bz(b), e.unwrap_or([0; 2]))));
    rows(v.iter().map(|&(u, a, b, e)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a));
        f.extend(ints(&b));
        f.extend(ints(&e));
        row(f)
    }))
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// COUNT(DISTINCT b.Id) AS BadgeCount
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN Votes v ON p.Id = v.PostId
// LEFT JOIN Badges b ON u.Id = b.UserId
// WHERE u.CreationDate < TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
// GROUP BY u.Id, u.DisplayName
// ),
// PostAnalytics AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// COUNT(c.Id) AS CommentCount,
// pt.Name AS PostType,
// COALESCE(ph.Comment, '') AS CloseReason
// FROM Posts p
// LEFT JOIN Comments c ON p.Id = c.PostId
// LEFT JOIN PostTypes pt ON p.PostTypeId = pt.Id
// LEFT JOIN PostHistory ph ON p.Id = ph.PostId AND ph.PostHistoryTypeId = 10
// WHERE p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'
// GROUP BY p.Id, p.Title, p.CreationDate, p.Score, pt.Name, ph.Comment
// ),
// FinalReport AS (
// SELECT
// us.UserId,
// us.DisplayName,
// us.QuestionCount,
// us.AnswerCount,
// us.UpVotes,
// us.DownVotes,
// us.BadgeCount,
// pa.PostId,
// pa.Title,
// pa.CreationDate,
// pa.Score,
// pa.CommentCount,
// pa.PostType,
// pa.CloseReason
// FROM UserStats us
// JOIN PostAnalytics pa ON us.UserId = pa.PostId
// )
// SELECT
// UserId,
// DisplayName,
// QuestionCount,
// AnswerCount,
// UpVotes,
// DownVotes,
// BadgeCount,
// PostId,
// Title,
// CreationDate,
// Score,
// CommentCount,
// PostType,
// CloseReason
// FROM FinalReport
// ORDER BY QuestionCount DESC, AnswerCount DESC;
fn q8635(db: &'static So) -> String {
    let pid = pids(db);
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::CreatedLt(year_ago()), "vb", any_post);
    let bu = badges_per_user(db);
    type K = (Id<Post>, Option<Str>);
    let PostHistory { post, comment, post_history_type_id, .. } = &db.post_history;
    let ck: MatSet<K> = db.post_history.with(post_history_type_id.eq(10)).select(post.and(comment.opt())).collect();
    let lh: HashIdx<K, Id<PostHistory>> = db.post_history.with(post_history_type_id.eq(10)).select(post.and(comment.opt())).inv().collect();
    let cf = (&ck).group_by(Same::<K>::new()).select(Same::<K>::new().map(|(p, _): K| p).select(comments_of(db).opt()).and(&lh)).fold(0i64, |n, (c, _)| n + c.is_some() as i64);
    let ci: HashIdx<Id<Post>, K> = (&ck).map(|(p, _)| p).inv().collect();
    let cp = comments_per_post(db);
    let mut v = Vec::new();
    (&us).and(&bu).and((&db.user.origid).select(&pid).with((&db.post.creation_date).ge(month_ago())).select(Ident::<Post>::new().and(&cp).and((&ci).select(Same::<K>::new().and(&cf)).opt()))).drive(|u, ((a, b), x)| v.push((u, a, b, x)));
    rows(v.iter().map(|&(u, a, b, ((p, c), h))| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&[a.q, a.a, a.up, a.down, b]));
        f.extend(post_fields(db, p, &["id", "title", "created", "score"]));
        f.push(V::I(h.map_or(c, |h| h.1)));
        f.extend(post_fields(db, p, &["type"]));
        f.push(V::S(h.and_then(|h| (h.0).1).unwrap_or("")));
        row(f)
    }))
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(DISTINCT P.Id) AS PostCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
// SUM(CASE WHEN P.ViewCount IS NOT NULL THEN P.ViewCount ELSE 0 END) AS TotalViews,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
// FROM
// Users U
// LEFT JOIN Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN Votes V ON P.Id = V.PostId
// GROUP BY
// U.Id, U.DisplayName
// ),
// TagStats AS (
// SELECT
// T.Id AS TagId,
// T.TagName,
// COUNT(P.Id) AS PostCount,
// SUM(P.ViewCount) AS TotalViews
// FROM
// Tags T
// LEFT JOIN Posts P ON P.Tags LIKE CONCAT('%', T.TagName, '%')
// GROUP BY
// T.Id, T.TagName
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
// COALESCE(U.PostCount, 0) AS TotalPosts,
// COALESCE(U.Questions, 0) AS TotalQuestions,
// COALESCE(U.Answers, 0) AS TotalAnswers,
// COALESCE(U.TotalViews, 0) AS TotalViews,
// COALESCE(U.TotalUpVotes, 0) AS TotalUpVotes,
// COALESCE(U.TotalDownVotes, 0) AS TotalDownVotes,
// COALESCE(B.BadgeCount, 0) AS TotalBadges,
// COALESCE(B.GoldBadges, 0) AS TotalGoldBadges,
// COALESCE(B.SilverBadges, 0) AS TotalSilverBadges,
// COALESCE(B.BronzeBadges, 0) AS TotalBronzeBadges
// FROM
// UserStats U
// LEFT JOIN BadgeStats B ON U.UserId = B.UserId
// ORDER BY
// U.TotalViews DESC, U.PostCount DESC;
fn q13449(db: &'static So) -> String {
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

// WITH PostActivity AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.Score,
// P.ViewCount,
// P.AnswerCount,
// COUNT(CM.Id) AS CommentCount,
// COALESCE(V.UpVotes, 0) AS UpVotes,
// COALESCE(V.DownVotes, 0) AS DownVotes
// FROM
// Posts P
// LEFT JOIN
// (SELECT PostId,
// SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM Votes
// GROUP BY PostId) V ON P.Id = V.PostId
// LEFT JOIN
// Comments CM ON P.Id = CM.PostId
// WHERE
// P.CreationDate >= '2023-01-01'
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.Score, P.ViewCount, P.AnswerCount, V.UpVotes, V.DownVotes
// ),
// UserEngagement AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(DISTINCT P.Id) AS PostsCreated,
// SUM(CASE WHEN B.Id IS NOT NULL THEN 1 ELSE 0 END) AS BadgesEarned,
// SUM(P.ViewCount) AS TotalViews
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// GROUP BY
// U.Id, U.DisplayName
// ),
// PostVoteStats AS (
// SELECT
// P.Id AS PostId,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
// FROM
// Posts P
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// P.Id
// )
// SELECT
// A.PostId,
// A.Title,
// A.CreationDate,
// A.Score,
// A.ViewCount,
// A.AnswerCount,
// A.CommentCount,
// U.UserId,
// U.DisplayName,
// U.PostsCreated,
// U.BadgesEarned,
// U.TotalViews,
// V.TotalUpVotes,
// V.TotalDownVotes
// FROM
// PostActivity A
// JOIN
// UserEngagement U ON A.PostId = U.PostsCreated
// JOIN
// PostVoteStats V ON A.PostId = V.PostId
// ORDER BY
// A.CreationDate DESC;
fn q14838(db: &'static So) -> String {
    let dp = ud(db, UserWhere::All, posts_of(db));
    let ue = g(db).select(posts_of(db).select((&db.post.view_count).opt()).opt().and(badges_of(db).opt())).fold([0i64; 3], |a, (w, b)| {
        let w = w.flatten();
        [a[0] + b.is_some() as i64, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0)]
    });
    let byn: HashIdx<i64, Id<User>> = db.user.select((&dp).opt()).map(|d: Option<i64>| d.unwrap_or(0)).inv().collect();
    let pv = post_votes(db);
    let cp = comments_per_post(db);
    let mut v = Vec::new();
    since(db, date(2023, 1, 1)).select(Ident::<Post>::new().and(&cp).and((&pv).opt()).and((&db.post.origid).select(&byn).select(Ident::<User>::new().and((&dp).opt()).and(&ue)))).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(((p, c), x), ((u, d), e))| {
        let d = d.unwrap_or(0);
        let x = x.unwrap_or([0; 3]);
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "answers"]);
        f.push(V::I(c));
        f.extend([user_col(db, u, "uid"), user_col(db, u, "name")]);
        f.extend([V::I(d), V::I(e[0]), nullable(e[2], e[1]), V::I(x[1]), V::I(x[2])]);
        row(f)
    }))
}

// WITH UserVoteSummary AS (
// SELECT
// u.Id AS UserId,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// COUNT(DISTINCT p.Id) AS PostCount
// FROM
// Users u
// LEFT JOIN
// Votes v ON u.Id = v.UserId
// LEFT JOIN
// Posts p ON v.PostId = p.Id
// GROUP BY
// u.Id
// ),
// PostStatistics AS (
// SELECT
// p.Id AS PostId,
// p.OwnerUserId,
// p.PostTypeId,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) - SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS VoteBalance,
// COUNT(c.Id) AS CommentCount,
// COUNT(DISTINCT pl.RelatedPostId) AS RelatedPostCount
// FROM
// Posts p
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// PostLinks pl ON p.Id = pl.PostId
// GROUP BY
// p.Id, p.OwnerUserId, p.PostTypeId
// ),
// PostHistoryDetails AS (
// SELECT
// ph.PostId,
// MAX(CASE WHEN ph.PostHistoryTypeId = 10 THEN ph.CreationDate END) AS ClosedDate,
// MAX(CASE WHEN ph.PostHistoryTypeId = 12 THEN ph.CreationDate END) AS DeletedDate,
// COUNT(CASE WHEN ph.PostHistoryTypeId IN (10, 12) THEN 1 END) AS ClosureOrDeletionCount
// FROM
// PostHistory ph
// GROUP BY
// ph.PostId
// )
// SELECT
// u.DisplayName,
// us.UpVotes,
// us.DownVotes,
// ps.VoteBalance,
// ps.CommentCount,
// ps.RelatedPostCount,
// ph.ClosedDate,
// ph.DeletedDate,
// ps.PostId
// FROM
// UserVoteSummary us
// JOIN
// Users u ON us.UserId = u.Id
// JOIN
// PostStatistics ps ON u.Id = ps.OwnerUserId
// LEFT JOIN
// PostHistoryDetails ph ON ps.PostId = ph.PostId
// WHERE
// (us.PostCount > 5 OR us.UpVotes > us.DownVotes)
// AND (ph.ClosureOrDeletionCount = 0 OR (ph.ClosedDate IS NOT NULL AND ph.DeletedDate IS NULL))
// ORDER BY
// us.UpVotes DESC,
// ps.VoteBalance DESC;
fn q2360(db: &'static So) -> String {
    let uv = db.vote.group_by(&db.vote.user).select(&db.vote.vote_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 2) as i64, a[1] + (t == 3) as i64]);
    let dv = db.vote.with(&db.vote.post).group_by(&db.vote.user).select(&db.vote.post).count_distinct();
    let dl = db.post_link.group_by(&db.post_link.post).select(&db.post_link.related_post_id).count_distinct();
    let ps = owned(db)
        .group_by(Ident::<Post>::new())
        .select(votes_of(db).select(&db.vote.vote_type_id).opt().and(comments_of(db).opt()).and(links_of(db).opt()))
        .fold([0i64; 2], |a, ((t, c), _)| [a[0] + (t == Some(2)) as i64 - (t == Some(3)) as i64, a[1] + c.is_some() as i64]);
    let ph = db.post_history.group_by(&db.post_history.post).select((&db.post_history.post_history_type_id).and(&db.post_history.creation_date)).fold((i64::MIN, i64::MIN, 0i64), |(c, d, n), (t, x)| {
        (if t == 10 { c.max(x) } else { c }, if t == 12 { d.max(x) } else { d }, n + matches!(t, 10 | 12) as i64)
    });
    let phk = (&ph).filt(|h: (i64, i64, i64)| h.2 == 0 || (h.0 != i64::MIN && h.1 == i64::MIN));
    let us = Ident::<User>::new().and((&uv).opt()).and((&dv).opt()).filt(|((_, x), d): ((Id<User>, Option<[i64; 2]>), Option<i64>)| {
        let x = x.unwrap_or([0; 2]);
        d.unwrap_or(0) > 5 || x[0] > x[1]
    });
    let mut v = Vec::new();
    (&ps)
        .and((&dl).opt())
        .and(phk)
        .and((&db.post.owner_user).select(us))
        .drive(|p, x| v.push((p, x)));
    rows(v.iter().map(|&(p, (((x, d), h), ((u, y), _)))| {
        let y = y.unwrap_or([0; 2]);
        let tm = |t: i64| if t == i64::MIN { V::Null } else { V::T(t) };
        let mut f = vec![user_col(db, u, "name"), V::I(y[0]), V::I(y[1])];
        f.extend(ints(&[x[0], x[1], d.unwrap_or(0)]));
        f.extend([tm(h.0), tm(h.1)]);
        f.extend(post_fields(db, p, &["id"]));
        row(f)
    }))
}

// WITH UserReputation AS (
// SELECT
// UserId,
// SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
// SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes,
// SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE -1 END) AS ReputationChange
// FROM
// Votes
// GROUP BY
// UserId
// ),
// PostDetails AS (
// SELECT
// p.Id AS PostId,
// p.OwnerUserId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// COALESCE(SUM(CASE WHEN c.Id IS NOT NULL THEN 1 ELSE 0 END), 0) AS CommentCount,
// COALESCE(SUM(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 ELSE 0 END), 0) AS ClosedCount,
// COALESCE(SUM(CASE WHEN ph.PostHistoryTypeId = 11 THEN 1 ELSE 0 END), 0) AS ReopenedCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// PostHistory ph ON p.Id = ph.PostId
// GROUP BY
// p.Id, p.OwnerUserId, p.Title, p.CreationDate, p.Score, p.ViewCount
// ),
// UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// ur.Upvotes,
// ur.Downvotes,
// ur.ReputationChange,
// COUNT(DISTINCT pd.PostId) AS TotalPosts,
// SUM(pd.Score) AS TotalScore,
// COALESCE(SUM(pd.CommentCount), 0) AS TotalComments,
// COALESCE(SUM(pd.ClosedCount), 0) AS TotalClosed,
// COALESCE(SUM(pd.ReopenedCount), 0) AS TotalReopened
// FROM
// Users u
// LEFT JOIN
// UserReputation ur ON u.Id = ur.UserId
// LEFT JOIN
// PostDetails pd ON u.Id = pd.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName, u.Reputation, ur.Upvotes, ur.Downvotes, ur.ReputationChange
// )
// SELECT
// us.UserId,
// us.DisplayName,
// us.Reputation,
// us.TotalPosts,
// us.TotalScore,
// us.TotalComments,
// us.TotalClosed,
// us.TotalReopened
// FROM
// UserStats us
// WHERE
// us.Reputation > 1000
// ORDER BY
// us.Reputation DESC, us.TotalScore DESC
// LIMIT 10;
fn q7629(db: &'static So) -> String {
    let Post { score, .. } = &db.post;
    let mine = || owned(db).with((&db.post.owner_user).select(Ident::<User>::new().with((&db.user.reputation).gt(1000))));
    let pd = mine()
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(history_of(db).select(&db.post_history.post_history_type_id).opt()))
        .fold([0i64; 3], |a, (c, t)| [a[0] + c.is_some() as i64, a[1] + (t == Some(10)) as i64, a[2] + (t == Some(11)) as i64]);
    let uf = mine().group_by(&db.post.owner_user).select(score.and(&pd)).fold([0i64; 5], |a, (s, x)| [a[0] + 1, a[1] + s, a[2] + x[0], a[3] + x[1], a[4] + x[2]]);
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and((&uf).opt())).drive(|_, (u, a)| v.push((u, a)));
    out(v, |&(u, a)| (rep_desc(db, u), (a.is_none(), Reverse(a.map_or(0, |a| a[1])))), 10, |&(u, a)| {
        let mut f = ["uid", "name", "rep"].iter().map(|k| user_col(db, u, k)).collect::<Vec<_>>();
        f.extend(match a {
            Some(a) => vec![V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[3]), V::I(a[4])],
            None => vec![V::I(0), V::Null, V::I(0), V::I(0), V::I(0)],
        });
        f
    })
}

pub static ENTRIES: &[harness::Entry] = &[
    ("5180", q5180),
    ("4230", q4230),
    ("6991", q6991),
    ("9972", q9972),
    ("11073", q11073),
    ("5114", q5114),
    ("26", q26),
    ("5315", q5315),
    ("12160", q12160),
    ("8199", q8199),
    ("11103", q11103),
    ("11231", q11231),
    ("3860", q3860),
    ("2684", q2684),
    ("8145", q8145),
    ("7861", q7861),
    ("6446", q6446),
    ("6821", q6821),
    ("6835", q6835),
    ("5241", q5241),
    ("23975", q23975),
    ("28615", q28615),
    ("9275", q9275),
    ("8910", q8910),
    ("14766", q14766),
    ("5657", q5657),
    ("5798", q5798),
    ("8546", q8546),
    ("6605", q6605),
    ("29019", q29019),
    ("23241", q23241),
    ("7840", q7840),
    ("3047", q3047),
    ("8049", q8049),
    ("5737", q5737),
    ("1345", q1345),
    ("1974", q1974),
    ("6733", q6733),
    ("7611", q7611),
    ("967", q967),
    ("7281", q7281),
    ("14507", q14507),
    ("7580", q7580),
    ("3697", q3697),
    ("24889", q24889),
    ("4682", q4682),
    ("6678", q6678),
    ("6440", q6440),
    ("8002", q8002),
    ("8635", q8635),
    ("13449", q13449),
    ("14838", q14838),
    ("2360", q2360),
    ("7629", q7629),
];
