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

// --- batch 133 --------------------------------------------------------------

// WITH PostStats AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
// COUNT(DISTINCT V.Id) AS VoteCount,
// COUNT(DISTINCT A.Id) AS AnswerCount,
// (SELECT COUNT(*) FROM PostHistory PH WHERE PH.PostId = P.Id) AS HistoryCount
// FROM
// Posts P
// LEFT JOIN Comments C ON P.Id = C.PostId
// LEFT JOIN Votes V ON P.Id = V.PostId
// LEFT JOIN Posts A ON P.Id = A.ParentId
// WHERE
// P.PostTypeId = 1
// GROUP BY
// P.Id, P.Title, P.CreationDate
// )
// SELECT
// PS.PostId,
// PS.Title,
// PS.CreationDate,
// PS.CommentCount,
// PS.VoteCount,
// PS.AnswerCount,
// PS.HistoryCount,
// (PS.CommentCount + PS.VoteCount + PS.AnswerCount + PS.HistoryCount) AS TotalEngagement
// FROM
// PostStats PS
// ORDER BY
// TotalEngagement DESC
// LIMIT 10;
fn q13934(db: &'static So) -> String {
    let mut v = Vec::new();
    stats_fold(db, questions_only(db), Ident::<Post>::new(), "cva", &[]).and(votes_per_post(db)).and(answers_per_post(db)).and(history_per_post(db)).drive(|p, (((s, x), a), h)| v.push((p, s.cx, x, a, h)));
    out(v, |&(_, c, x, a, h)| Reverse(c + x + a + h), 10, |&(p, c, x, a, h)| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend(ints(&[c, x, a, h, c + x + a + h]));
        f
    })
}

// WITH PostCounts AS (
// SELECT
// OwnerUserId,
// SUM(CASE WHEN PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount
// FROM
// Posts
// WHERE
// OwnerUserId IS NOT NULL
// GROUP BY
// OwnerUserId
// ),
// UserReputation AS (
// SELECT
// U.Id AS UserId,
// U.Reputation,
// PC.QuestionCount,
// PC.AnswerCount
// FROM
// Users U
// LEFT JOIN
// PostCounts PC ON U.Id = PC.OwnerUserId
// )
// SELECT
// AVG(CASE WHEN QuestionCount > 0 THEN Reputation END) AS AvgReputationForQuestions,
// AVG(CASE WHEN AnswerCount > 0 THEN Reputation END) AS AvgReputationForAnswers,
// COUNT(DISTINCT CASE WHEN QuestionCount > 0 THEN UserId END) AS TotalQuestionCreators,
// COUNT(DISTINCT CASE WHEN AnswerCount > 0 THEN UserId END) AS TotalAnswerCreators
// FROM
// UserReputation;
fn q12588(db: &'static So) -> String {
    let pc = owned(db).group_by(&db.post.owner_user).select(&db.post.post_type_id).fold([0i64; 2], |a, t| [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64]);
    let t = db.user.select((&db.user.reputation).and((&pc).opt())).fold_flat([0i64; 4], |a, (r, p)| {
        let p = p.unwrap_or([0; 2]);
        [a[0] + (p[0] > 0) as i64, a[1] + if p[0] > 0 { r } else { 0 }, a[2] + (p[1] > 0) as i64, a[3] + if p[1] > 0 { r } else { 0 }]
    });
    row(vec![avg(t[1], t[0]), avg(t[3], t[2]), V::I(t[0]), V::I(t[2])])
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS TotalQuestions,
// COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS TotalAnswers,
// SUM(p.ViewCount) AS TotalViews,
// SUM(p.Score) AS TotalScore,
// AVG(COALESCE(c.CommentCount, 0)) AS AvgCommentsPerPost
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN (
// SELECT PostId, COUNT(*) AS CommentCount
// FROM Comments
// GROUP BY PostId
// ) c ON p.Id = c.PostId
// GROUP BY u.Id, u.DisplayName
// )
// SELECT
// UserId,
// DisplayName,
// TotalPosts,
// TotalQuestions,
// TotalAnswers,
// TotalViews,
// TotalScore,
// AvgCommentsPerPost
// FROM UserPostStats
// WHERE TotalPosts > 0
// ORDER BY TotalScore DESC
// FETCH FIRST 10 ROWS ONLY;
fn q14302(db: &'static So) -> String {
    let ps = pstat(db, db.post.iq());
    let cs = owned(db).group_by(&db.post.owner_user).select(comments_per_post(db)).fold(0i64, |a, c| a + c);
    let mut v = Vec::new();
    (&ps).filt(|p: [i64; 13]| p[0] > 0).and(&cs).drive(|u, (p, c)| v.push((u, p, c)));
    out(v, |&(_, p, _)| Reverse(p[3]), 10, |&(u, p, c)| vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(p[0]), V::I(p[1]), V::I(p[2]), pviews(p), V::I(p[3]), avg(c, p[0])])
}

// WITH UserActivity AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(DISTINCT P.Id) AS PostCount,
// SUM(COALESCE(V.Upvotes, 0)) AS TotalUpVotes,
// SUM(COALESCE(V.DownVotes, 0)) AS TotalDownVotes,
// COUNT(DISTINCT B.Id) AS BadgeCount
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// (SELECT
// PostId,
// SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
// SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// Votes
// GROUP BY
// PostId) V ON P.Id = V.PostId
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// GROUP BY
// U.Id, U.DisplayName
// )
// SELECT
// UserId,
// DisplayName,
// PostCount,
// TotalUpVotes,
// TotalDownVotes,
// BadgeCount
// FROM
// UserActivity
// ORDER BY
// PostCount DESC
// LIMIT 100;
fn q11695(db: &'static So) -> String {
    let pv = post_votes(db);
    let uf = g(db).select(posts_of(db).select((&pv).opt()).opt().and(badges_of(db).opt())).fold([0i64; 2], |a, (p, _)| {
        let x = p.flatten().unwrap_or([0; 3]);
        [a[0] + x[1], a[1] + x[2]]
    });
    let dp = ud(db, UserWhere::All, posts_of(db));
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    (&uf).and((&dp).opt()).and(&bu).drive(|u, ((a, d), b)| v.push((u, a, d.unwrap_or(0), b)));
    out(v, |&(_, _, d, _)| Reverse(d), 100, |&(u, a, d, b)| vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(d), V::I(a[0]), V::I(a[1]), V::I(b)])
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS PostCount,
// COUNT(DISTINCT v.Id) AS VoteCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Votes v ON u.Id = v.UserId
// GROUP BY
// u.Id, u.DisplayName
// ),
// UserBadgeStats AS (
// SELECT
// UserId,
// COUNT(Id) AS BadgeCount
// FROM
// Badges
// GROUP BY
// UserId
// )
// SELECT
// ups.UserId,
// ups.DisplayName,
// ups.PostCount,
// ups.VoteCount,
// ups.QuestionCount,
// ups.AnswerCount,
// COALESCE(ubs.BadgeCount, 0) AS BadgeCount
// FROM
// UserPostStats ups
// LEFT JOIN
// UserBadgeStats ubs ON ups.UserId = ubs.UserId
// ORDER BY
// ups.PostCount DESC,
// ups.VoteCount DESC;
fn q12733(db: &'static So) -> String {
    let uf = g(db).select(posts_of(db).select(&db.post.post_type_id).opt().and(votes_by(db).opt())).fold([0i64; 2], |a, (t, _)| [a[0] + (t == Some(1)) as i64, a[1] + (t == Some(2)) as i64]);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let dv = ud(db, UserWhere::All, votes_by(db));
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    (&uf).and((&dp).opt()).and((&dv).opt()).and(&bu).drive(|u, (((a, p), x), b)| v.push((u, a, p.unwrap_or(0), x.unwrap_or(0), b)));
    rows(v.iter().map(|&(u, a, p, x, b)| row(vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(p), V::I(x), V::I(a[0]), V::I(a[1]), V::I(b)])))
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.Reputation,
// U.Views,
// COUNT(DISTINCT P.Id) AS PostCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN P.PostTypeId IN (1, 2) THEN P.Score ELSE 0 END) AS TotalScore
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id, U.Reputation, U.Views
// ),
// TopBadgeUsers AS (
// SELECT
// B.UserId,
// COUNT(B.Id) AS BadgeCount
// FROM
// Badges B
// GROUP BY
// B.UserId
// )
// SELECT
// U.UserId,
// U.Reputation,
// U.Views,
// U.PostCount,
// U.QuestionCount,
// U.AnswerCount,
// U.TotalScore,
// COALESCE(B.BadgeCount, 0) AS BadgeCount
// FROM
// UserStats U
// LEFT JOIN
// TopBadgeUsers B ON U.UserId = B.UserId
// ORDER BY
// U.Reputation DESC, U.TotalScore DESC
// LIMIT 100;
fn q13845(db: &'static So) -> String {
    let ps = owned(db).group_by(&db.post.owner_user).select((&db.post.post_type_id).and(&db.post.score)).fold([0i64; 4], |a, (t, s)| {
        [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + if matches!(t, 1 | 2) { s } else { 0 }]
    });
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&ps).opt()).and(&bu)).drive(|_, ((u, p), b)| v.push((u, p.unwrap_or([0; 4]), b)));
    out(v, |&(u, p, _)| (rep_desc(db, u), Reverse(p[3])), 100, |&(u, p, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep"), user_col(db, u, "uviews")];
        f.extend(ints(&p));
        f.push(V::I(b));
        f
    })
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// COUNT(DISTINCT c.Id) AS CommentCount,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= '2023-01-01'
// GROUP BY
// p.Id, p.Title, p.CreationDate, p.ViewCount
// ),
// PostVoteSummary AS (
// SELECT
// PostId,
// SUM(UpVotes - DownVotes) AS NetVotes
// FROM
// PostStats
// GROUP BY
// PostId
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.ViewCount,
// ps.CommentCount,
// pvs.NetVotes
// FROM
// PostStats ps
// JOIN
// PostVoteSummary pvs ON ps.PostId = pvs.PostId
// ORDER BY
// ps.CreationDate DESC;
fn q11657(db: &'static So) -> String {
    let mut v = Vec::new();
    stats_fold(db, since(db, date(2023, 1, 1)), Ident::<Post>::new(), "cv", &[]).and(comments_per_post(db)).drive(|p, (s, c)| v.push((p, s, c)));
    rows(v.iter().map(|&(p, s, c)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views"]);
        f.extend([V::I(c), V::I(s.up - s.down)]);
        row(f)
    }))
}

// WITH UserPostStatistics AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS TotalPosts,
// COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS TotalQuestions,
// COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS TotalAnswers,
// SUM(p.Score) AS TotalScore,
// SUM(p.ViewCount) AS TotalViews,
// AVG(EXTRACT(EPOCH FROM p.CreationDate)) AS AvgPostCreationDate
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// GROUP BY u.Id, u.DisplayName
// ),
// TopPostUsers AS (
// SELECT
// UserId,
// DisplayName,
// TotalPosts,
// TotalQuestions,
// TotalAnswers,
// TotalScore,
// TotalViews,
// AvgPostCreationDate
// FROM UserPostStatistics
// ORDER BY TotalPosts DESC
// LIMIT 10
// )
// SELECT
// UserId,
// DisplayName,
// TotalPosts,
// TotalQuestions,
// TotalAnswers,
// TotalScore,
// TotalViews,
// AvgPostCreationDate
// FROM TopPostUsers;
fn q13355(db: &'static So) -> String {
    let ps = pstat(db, db.post.iq());
    let ep = owned(db).group_by(&db.post.owner_user).select(&db.post.creation_date).fold(0i128, |a, c| a + c as i128);
    let top: MatSet<Id<User>> = whole(&db.user.id)
        .select(Ident::<User>::new().and((&ps).opt()))
        .window(row_number, |(_, p): (Id<User>, Option<[i64; 13]>)| p.map_or(0, |p| p[0]), desc)
        .filt(|(_, n)| n <= 10)
        .map(|((u, _), _)| u)
        .collect();
    let mut v = Vec::new();
    (&top).select(Ident::<User>::new().and((&ps).opt()).and((&ep).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((u, p), e)| {
        let q = p.unwrap_or(Z);
        let avg_date = match (p, e) {
            (Some(p), Some(e)) => V::F(e as f64 / p[0] as f64 / 1e6),
            _ => V::Null,
        };
        row(vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(q[0]), V::I(q[1]), V::I(q[2]), oint(p.map(|p| p[3])), onull(p, pviews), avg_date])
    }))
}

// WITH UserStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
// COUNT(DISTINCT p.Id) AS PostCount,
// COUNT(DISTINCT c.Id) AS CommentCount,
// SUM(COALESCE(b.Class, 0)) AS BadgeCount
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN Comments c ON p.Id = c.PostId
// LEFT JOIN Votes v ON p.Id = v.PostId AND v.UserId = u.Id
// LEFT JOIN Badges b ON u.Id = b.UserId
// WHERE u.Reputation >= 1000
// GROUP BY u.Id, u.DisplayName
// ),
// TopUsers AS (
// SELECT
// UserId,
// DisplayName,
// UpVotes - DownVotes AS VoteNet,
// PostCount + CommentCount AS Engagement,
// BadgeCount
// FROM UserStats
// )
// SELECT
// UserId,
// DisplayName,
// VoteNet,
// Engagement,
// BadgeCount
// FROM TopUsers
// ORDER BY Engagement DESC, VoteNet DESC
// LIMIT 10;
fn q6458(db: &'static So) -> String {
    let w = UserWhere::RepGt(999);
    let own = self_votes(db);
    let uf = user_base(db, w)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(comments_of(db).opt().and((&own).select(&db.vote.vote_type_id).opt())).opt().and(badges_of(db).select(&db.badge.class).opt()))
        .fold([0i64; 3], |a, (p, c)| {
            let t = p.and_then(|(_, t)| t);
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + c.unwrap_or(0)]
        });
    let dp = ud(db, w, posts_of(db));
    let dc = ud(db, w, posts_of(db).select(comments_of(db)));
    let mut v = Vec::new();
    (&uf).and((&dp).opt()).and((&dc).opt()).drive(|u, ((a, p), c)| v.push((u, a[0] - a[1], p.unwrap_or(0) + c.unwrap_or(0), a[2])));
    out(v, |&(_, n, e, _)| (Reverse(e), Reverse(n)), 10, |&(u, n, e, c)| vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(n), V::I(e), V::I(c)])
}

// WITH UserActivity AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(DISTINCT P.Id) AS PostCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount,
// SUM(COALESCE(CLOSED.Rate, 0)) AS ClosedPostRate
// FROM Users U
// LEFT JOIN Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN Votes V ON P.Id = V.PostId
// LEFT JOIN (
// SELECT PostId, COUNT(*) AS Rate
// FROM PostHistory
// WHERE PostHistoryTypeId = 10
// GROUP BY PostId
// ) AS CLOSED ON P.Id = CLOSED.PostId
// GROUP BY U.Id, U.DisplayName
// )
// SELECT
// UserId,
// DisplayName,
// PostCount,
// QuestionCount,
// AnswerCount,
// UpVoteCount,
// DownVoteCount,
// ClosedPostRate
// FROM UserActivity
// WHERE PostCount > 10
// ORDER BY PostCount DESC;
fn q11259(db: &'static So) -> String {
    let cr = db.post_history.with((&db.post_history.post_history_type_id).eq(10)).group_by(&db.post_history.post).fold(0i64, |a, _| a + 1);
    let uf = g(db).select(posts_of(db).select((&db.post.post_type_id).and(votes_of(db).select(&db.vote.vote_type_id).opt()).and((&cr).opt())).opt()).fold([0i64; 5], |a, p| match p {
        Some(((t, x), r)) => [a[0] + (t == 1) as i64, a[1] + (t == 2) as i64, a[2] + (x == Some(2)) as i64, a[3] + (x == Some(3)) as i64, a[4] + r.unwrap_or(0)],
        None => a,
    });
    let dp = ud(db, UserWhere::All, posts_of(db));
    let mut v = Vec::new();
    (&uf).and((&dp).filt(|d: i64| d > 10)).drive(|u, (a, d)| v.push((u, a, d)));
    rows(v.iter().map(|&(u, a, d)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(d)];
        f.extend(ints(&a));
        row(f)
    }))
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(DISTINCT P.Id) AS PostCount,
// COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
// COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// U.Id, U.DisplayName, U.Reputation
// ),
// AggregatedStats AS (
// SELECT
// AVG(Reputation) AS AvgReputation,
// AVG(PostCount) AS AvgPostCount,
// AVG(UpVotes) AS AvgUpVotes,
// AVG(DownVotes) AS AvgDownVotes
// FROM
// UserStats
// )
// SELECT
// U.UserId,
// U.DisplayName,
// U.Reputation,
// U.PostCount,
// U.UpVotes,
// U.DownVotes,
// A.AvgReputation,
// A.AvgPostCount,
// A.AvgUpVotes,
// A.AvgDownVotes
// FROM
// UserStats U, AggregatedStats A
// WHERE
// U.Reputation > 1000
// ORDER BY
// U.Reputation DESC;
fn q14741(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let t = db.user.select((&db.user.reputation).and(&us).and((&dp).opt())).fold_flat([0i64; 5], |a, ((r, s), d)| [a[0] + 1, a[1] + r, a[2] + d.unwrap_or(0), a[3] + s.up, a[4] + s.down]);
    let m = |i: usize| V::F(t[i] as f64 / t[0] as f64);
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(1000)).select(Ident::<User>::new().and(&us).and((&dp).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((u, a), d)| row(vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep"), V::I(d.unwrap_or(0)), V::I(a.up), V::I(a.down), m(1), m(2), m(3), m(4)])))
}

// WITH UserReputation AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(p.Score) AS TotalScore,
// SUM(p.ViewCount) AS TotalViews
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// GROUP BY u.Id, u.Reputation
// ),
// ActiveUsers AS (
// SELECT
// u.Id AS UserId,
// COUNT(pt.Id) AS PostTypeCount,
// SUM(CASE WHEN pt.Name = 'Question' THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN pt.Name = 'Answer' THEN 1 ELSE 0 END) AS AnswerCount
// FROM Users u
// JOIN Posts p ON u.Id = p.OwnerUserId
// JOIN PostTypes pt ON p.PostTypeId = pt.Id
// WHERE u.LastAccessDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY u.Id
// )
// SELECT
// ur.UserId,
// ur.Reputation,
// ur.PostCount,
// ur.TotalScore,
// ur.TotalViews,
// au.PostTypeCount,
// au.QuestionCount,
// au.AnswerCount
// FROM UserReputation ur
// JOIN ActiveUsers au ON ur.UserId = au.UserId
// ORDER BY ur.Reputation DESC;
fn q13873(db: &'static So) -> String {
    let ps = pstat(db, db.post.iq());
    let au = owned(db).group_by(&db.post.owner_user).select(ptype_name(db)).fold([0i64; 3], |a, n| [a[0] + 1, a[1] + (n == "Question") as i64, a[2] + (n == "Answer") as i64]);
    let mut v = Vec::new();
    db.user.with((&db.user.last_access_date).ge(year_ago())).select(Ident::<User>::new().and(&ps).and(&au)).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((u, p), a)| row(vec![user_col(db, u, "uid"), user_col(db, u, "rep"), V::I(p[0]), V::I(p[3]), pviews(p), V::I(a[0]), V::I(a[1]), V::I(a[2])])))
}

// WITH UserVoteStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(V.Id) AS TotalVotes,
// SUM(CASE WHEN V.VoteTypeId IN (2, 6) THEN 1 ELSE 0 END) AS Upvotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes,
// AVG(COALESCE(P.Score, 0)) AS AvgScore
// FROM
// Users U
// LEFT JOIN
// Votes V ON U.Id = V.UserId
// LEFT JOIN
// Posts P ON V.PostId = P.Id
// GROUP BY
// U.Id, U.DisplayName
// )
// SELECT
// U.UserId,
// U.DisplayName,
// U.TotalVotes,
// U.Upvotes,
// U.Downvotes,
// (U.Upvotes - U.Downvotes) AS NetVotes,
// CASE
// WHEN U.AvgScore IS NOT NULL THEN U.AvgScore
// ELSE 0
// END AS AverageScore,
// P.Title AS HighScorePost,
// P.Score AS HighScore
// FROM
// UserVoteStats U
// LEFT JOIN
// Posts P ON U.UserId = P.OwnerUserId
// WHERE
// P.Score = (SELECT MAX(Score) FROM Posts WHERE OwnerUserId = U.UserId AND Score IS NOT NULL)
// AND U.TotalVotes > 0
// ORDER BY
// NetVotes DESC,
// U.DisplayName ASC;
fn q4255(db: &'static So) -> String {
    let uv = g(db).select(votes_by(db).select((&db.vote.vote_type_id).and((&db.vote.post).select(&db.post.score).opt())).opt()).fold([0i64; 5], |a, x| match x {
        Some((t, s)) => [a[0] + 1, a[1] + matches!(t, 2 | 6) as i64, a[2] + (t == 3) as i64, a[3] + s.unwrap_or(0), a[4] + 1],
        None => [a[0], a[1], a[2], a[3], a[4] + 1],
    });
    let mx = owned(db).group_by(&db.post.owner_user).select(&db.post.score).fold(i64::MIN, |a, s| a.max(s));
    let mut v = Vec::new();
    (&uv)
        .filt(|a: [i64; 5]| a[0] > 0)
        .and(&mx)
        .and(posts_of(db).select(Ident::<Post>::new().and(&db.post.score)))
        .filt(|((_, m), (_, s)): (([i64; 5], i64), (Id<Post>, i64))| s == m)
        .drive(|u, ((a, _), (p, _))| v.push((u, a, p)));
    rows(v.iter().map(|&(u, a, p)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), V::I(a[1] - a[2]), V::F(a[3] as f64 / a[4] as f64)];
        f.extend(post_fields(db, p, &["title", "score"]));
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
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// U.Id, U.DisplayName
// ),
// BadgeStats AS (
// SELECT
// B.UserId,
// COUNT(B.Id) AS BadgeCount
// FROM
// Badges B
// GROUP BY
// B.UserId
// )
// SELECT
// US.UserId,
// US.DisplayName,
// US.PostCount,
// US.QuestionCount,
// US.AnswerCount,
// US.UpVoteCount,
// US.DownVoteCount,
// COALESCE(BS.BadgeCount, 0) AS BadgeCount
// FROM
// UserStats US
// LEFT JOIN
// BadgeStats BS ON US.UserId = BS.UserId
// ORDER BY
// US.PostCount DESC;
fn q13245(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    (&us).and((&dp).opt()).and(&bu).drive(|u, ((a, d), b)| v.push((u, a, d.unwrap_or(0), b)));
    rows(v.iter().map(|&(u, a, d, b)| row(vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(d), V::I(a.q), V::I(a.a), V::I(a.up), V::I(a.down), V::I(b)])))
}

// WITH PostCount AS (
// SELECT
// COUNT(*) AS TotalPosts,
// COUNT(DISTINCT OwnerUserId) AS TotalUsers,
// SUM(CASE WHEN PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions
// FROM
// Posts
// ),
// VoteCount AS (
// SELECT
// COUNT(*) AS TotalVotes,
// SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
// FROM
// Votes
// ),
// AvgResponseTime AS (
// SELECT
// AVG(EXTRACT(EPOCH FROM (FirstAnswer.CreationDate - Q.CreationDate))) AS AvgTimeToFirstAnswer
// FROM
// Posts Q
// LEFT JOIN
// Posts FirstAnswer ON Q.Id = FirstAnswer.ParentId
// WHERE
// Q.PostTypeId = 1
// )
// SELECT
// PC.TotalPosts,
// PC.TotalUsers,
// PC.TotalQuestions,
// PC.TotalAnswers,
// VC.TotalVotes,
// VC.TotalUpVotes,
// VC.TotalDownVotes,
// ART.AvgTimeToFirstAnswer
// FROM
// PostCount PC,
// VoteCount VC,
// AvgResponseTime ART;
fn q10537(db: &'static So) -> String {
    let p = db.post.select(&db.post.post_type_id).fold_flat([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64]);
    let du = one(whole(db.post.iq()).select(&db.post.owner_user_id).count_distinct());
    let x = db.vote.select(&db.vote.vote_type_id).fold_flat([0i64; 3], |a, t| [a[0] + 1, a[1] + (t == 2) as i64, a[2] + (t == 3) as i64]);
    let r = questions_only(db).select((&db.post.creation_date).and(children_of(db).select(&db.post.creation_date))).fold_flat((0i64, 0.0f64), |(n, s), (q, a)| (n + 1, s + (a - q) as f64 / 1e6));
    row(vec![V::I(p[0]), V::I(du), V::I(p[1]), V::I(p[2]), V::I(x[0]), V::I(x[1]), V::I(x[2]), if r.0 == 0 { V::Null } else { V::F(r.1 / r.0 as f64) }])
}

// WITH PostStats AS (
// SELECT
// PostTypeId,
// COUNT(*) AS TotalPosts,
// AVG(ViewCount) AS AverageViewCount,
// AVG(Score) AS AverageScore,
// SUM(CASE WHEN PostTypeId = 1 THEN AnswerCount ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN CommentCount IS NOT NULL THEN CommentCount ELSE 0 END) AS TotalComments
// FROM Posts
// GROUP BY PostTypeId
// ),
// UserReputation AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// COUNT(b.Id) AS BadgeCount,
// AVG(p.ViewCount) AS AvgPostViews
// FROM Users u
// LEFT JOIN Badges b ON u.Id = b.UserId
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// GROUP BY u.Id, u.Reputation
// )
// SELECT
// p.PostTypeId,
// p.TotalPosts,
// p.AverageViewCount,
// p.AverageScore,
// p.TotalAnswers,
// p.TotalComments,
// u.UserId,
// u.Reputation,
// u.BadgeCount,
// u.AvgPostViews
// FROM PostStats p
// JOIN UserReputation u ON u.UserId IN (SELECT DISTINCT OwnerUserId FROM Posts WHERE PostTypeId = p.PostTypeId)
// ORDER BY p.PostTypeId, u.Reputation DESC;
fn q11396(db: &'static So) -> String {
    let Post { view_count, score, answer_count, post_type_id, comment_count, owner_user, .. } = &db.post;
    let pt = db.post.group_by(post_type_id).select(view_count.opt().and(score).and(answer_count.opt()).and(post_type_id).and(comment_count)).fold([0i64; 6], |a, ((((w, s), an), t), c)| {
        [a[0] + 1, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0), a[3] + s, a[4] + if t == 1 { an.unwrap_or(0) } else { 0 }, a[5] + c]
    });
    let ur = g(db).select(badges_of(db).opt().and(posts_of(db).select(view_count.opt()).opt())).fold([0i64; 3], |a, (b, w)| {
        let w = w.flatten();
        [a[0] + b.is_some() as i64, a[1] + w.is_some() as i64, a[2] + w.unwrap_or(0)]
    });
    let pairs: MatSet<(i64, Id<User>)> = owned(db).select(post_type_id.and(owner_user)).collect();
    let mut v = Vec::new();
    (&pairs)
        .select(Same::<(i64, Id<User>)>::new().map(|(t, _)| t).select(&pt).and(Same::<(i64, Id<User>)>::new().map(|(_, u)| u).select(&ur)))
        .drive(|(t, u), (a, b)| v.push((t, u, a, b)));
    rows(v.iter().map(|&(t, u, a, b)| row(vec![V::I(t), V::I(a[0]), avg(a[2], a[1]), avg(a[3], a[0]), V::I(a[4]), V::I(a[5]), user_col(db, u, "uid"), user_col(db, u, "rep"), V::I(b[0]), avg(b[2], b[1])])))
}

// WITH UserPostStats AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS PostCount,
// COALESCE(SUM(vote.Score), 0) AS TotalVotes,
// COALESCE(SUM(c.CommentCount), 0) AS TotalComments,
// AVG(EXTRACT(EPOCH FROM (p.LastActivityDate - p.CreationDate))) AS AvgActivityDuration
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// (SELECT PostId, COUNT(*) AS CommentCount
// FROM Comments
// GROUP BY PostId) c ON p.Id = c.PostId
// LEFT JOIN
// (SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 WHEN VoteTypeId = 3 THEN -1 ELSE 0 END) AS Score
// FROM Votes
// GROUP BY PostId) vote ON p.Id = vote.PostId
// GROUP BY
// u.Id, u.DisplayName
// )
// SELECT
// ups.DisplayName,
// ups.PostCount,
// ups.TotalVotes,
// ups.TotalComments,
// ups.AvgActivityDuration
// FROM
// UserPostStats ups
// JOIN
// Users u ON ups.UserId = u.Id
// ORDER BY
// ups.PostCount DESC, ups.TotalVotes DESC
// FETCH FIRST 10 ROWS ONLY;
fn q10766(db: &'static So) -> String {
    let vs = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold(0i64, |a, t| {
        a + match t {
            2 => 1,
            3 => -1,
            _ => 0,
        }
    });
    let Post { last_activity_date, creation_date, .. } = &db.post;
    let uf = g(db)
        .select(posts_of(db).select(comments_per_post(db).and((&vs).opt()).and(last_activity_date).and(creation_date)).opt())
        .fold(([0i64; 3], 0i128), |(a, f): ([i64; 3], i128), p| match p {
            Some((((c, x), la), cd)) => ([a[0] + 1, a[1] + x.unwrap_or(0), a[2] + c], f + (la - cd) as i128),
            None => (a, f),
        });
    let mut v = Vec::new();
    (&uf).drive(|u, (a, f)| v.push((u, a, f)));
    out(v, |&(_, a, _)| (Reverse(a[0]), Reverse(a[1])), 10, |&(u, a, f)| vec![user_col(db, u, "name"), V::I(a[0]), V::I(a[1]), V::I(a[2]), if a[0] == 0 { V::Null } else { V::F(f as f64 / a[0] as f64 / 1e6) }])
}

// WITH RecursiveTags AS (
// SELECT
// p.Id AS PostId,
// string_to_array(substring(p.Tags, 2, length(p.Tags) - 2), '><') AS TagsArray
// FROM
// Posts p
// WHERE
// p.PostTypeId = 1
// ), UnnestTags AS (
// SELECT
// PostId,
// unnest(TagsArray) AS Tag
// FROM
// RecursiveTags
// ), TagCounts AS (
// SELECT
// Tag,
// COUNT(PostId) AS TagCount
// FROM
// UnnestTags
// GROUP BY
// Tag
// ), TopTags AS (
// SELECT
// Tag,
// TagCount
// FROM
// TagCounts
// ORDER BY
// TagCount DESC
// LIMIT 10
// ), PostsWithTopTags AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.Body,
// p.CreationDate,
// tt.Tag
// FROM
// Posts p
// JOIN
// UnnestTags ut ON p.Id = ut.PostId
// JOIN
// TopTags tt ON ut.Tag = tt.Tag
// )
// SELECT
// PostId,
// Title,
// CreationDate,
// Tag,
// LEFT(Body, 150) || '...' AS BodySnippet
// FROM
// PostsWithTopTags
// ORDER BY
// CreationDate DESC;
fn q28744(db: &'static So) -> String {
    let tc = questions_only(db).group_by((&db.post.tags_str).flat_map(tag_list)).fold(0i64, |a, _| a + 1);
    let tcs: &'static Fold<Str, i64> = Box::leak(Box::new(tc));
    let top = top_by(tcs, 10);
    let topidx: HashIdx<Str, Str> = (&top).inv().collect();
    let mut v = Vec::new();
    questions_only(db).select(Ident::<Post>::new().and((&db.post.tags_str).flat_map(tag_list).select(&topidx))).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(p, t)| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        let b: String = db.post.body.get(p).unwrap().chars().take(150).collect();
        f.extend([V::S(t), V::Owned(b + "...")]);
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
// SUM(p.Score) AS TotalScore,
// SUM(p.ViewCount) AS TotalViewCount,
// AVG(EXTRACT(epoch FROM p.CreationDate)) AS AvgPostDate, -- Using standard SQL for date handling
// COALESCE(SUM(c.CommentCount), 0) AS TotalComments
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN (
// SELECT
// PostId,
// COUNT(*) AS CommentCount
// FROM
// Comments
// GROUP BY
// PostId
// ) c ON p.Id = c.PostId
// GROUP BY
// u.Id, u.DisplayName
// )
// SELECT
// UserId,
// DisplayName,
// TotalPosts,
// Questions,
// Answers,
// TotalScore,
// TotalViewCount,
// AvgPostDate,
// TotalComments
// FROM
// UserPostStats
// WHERE
// TotalPosts > 0
// ORDER BY
// TotalScore DESC;
fn q10137(db: &'static So) -> String {
    let ps = pstat(db, db.post.iq());
    let ex = owned(db).group_by(&db.post.owner_user).select((&db.post.creation_date).and(comments_per_post(db))).fold((0i128, 0i64), |(e, c), (cd, n)| (e + cd as i128, c + n));
    let mut v = Vec::new();
    (&ps).filt(|p: [i64; 13]| p[0] > 0).and(&ex).drive(|u, (p, e)| v.push((u, p, e)));
    rows(v.iter().map(|&(u, p, (e, c))| row(vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(p[0]), V::I(p[1]), V::I(p[2]), V::I(p[3]), pviews(p), V::F(e as f64 / p[0] as f64 / 1e6), V::I(c)])))
}

// WITH TopUsers AS (
// SELECT Id, DisplayName, Reputation
// FROM Users
// WHERE Reputation > 5000
// ORDER BY Reputation DESC
// LIMIT 20
// ), UserPostCounts AS (
// SELECT OwnerUserId, COUNT(*) AS PostCount
// FROM Posts
// WHERE CreationDate >= cast('2024-10-01' as date) - interval '1 year'
// GROUP BY OwnerUserId
// ), UserBadges AS (
// SELECT UserId, COUNT(*) AS BadgeCount
// FROM Badges
// WHERE Date >= cast('2024-10-01' as date) - interval '1 year'
// GROUP BY UserId
// ), UserScores AS (
// SELECT u.Id AS UserId, u.DisplayName, COALESCE(UPC.PostCount, 0) AS PostCount, COALESCE(UB.BadgeCount, 0) AS BadgeCount
// FROM TopUsers u
// LEFT JOIN UserPostCounts UPC ON u.Id = UPC.OwnerUserId
// LEFT JOIN UserBadges UB ON u.Id = UB.UserId
// )
// SELECT u.UserId, u.DisplayName, u.PostCount, u.BadgeCount
// FROM UserScores u
// JOIN (
// SELECT UserId, SUM(DISTINCT Score) AS TotalScore
// FROM Posts
// JOIN Votes ON Posts.Id = Votes.PostId
// GROUP BY UserId
// ) v ON u.UserId = v.UserId
// ORDER BY u.PostCount DESC, v.TotalScore DESC
// LIMIT 10;
fn q8901(db: &'static So) -> String {
    let base = user_base(db, UserWhere::RepGt(5000));
    let top: MatSet<Id<User>> = whole(&base).select(Ident::<User>::new().and(&db.user.reputation)).window(row_number, |(_, r)| r, desc).filt(|(_, n)| n <= 20).map(|((u, _), _)| u).collect();
    let t0 = date(2023, 10, 1);
    let upc = owned_since(db, t0).group_by(&db.post.owner_user).fold(0i64, |a, _| a + 1);
    let ub = db.badge.with((&db.badge.date).ge(t0)).group_by(&db.badge.user).fold(0i64, |a, _| a + 1);
    let pairs: MatSet<(Id<User>, i64)> = db.vote.select((&db.vote.user).and((&db.vote.post).select(&db.post.score))).collect();
    let vs = (&pairs).group_by(Same::<(Id<User>, i64)>::new().map(|(u, _)| u)).select(Same::<(Id<User>, i64)>::new().map(|(_, s)| s)).fold(0i64, |a, s| a + s);
    let mut v = Vec::new();
    (&top).select(Ident::<User>::new().and((&upc).opt()).and((&ub).opt()).and(&vs)).drive(|_, (((u, p), b), s)| v.push((u, p.unwrap_or(0), b.unwrap_or(0), s)));
    out(v, |&(_, p, _, s)| (Reverse(p), Reverse(s)), 10, |&(u, p, b, _)| vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(p), V::I(b)])
}

// WITH UserMetrics AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(DISTINCT P.Id) AS PostCount,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(COALESCE(V.VoteAmount, 0)) AS TotalVotes
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// (SELECT
// PostId,
// SUM(CASE WHEN VoteTypeId = 2 THEN 1 WHEN VoteTypeId = 3 THEN -1 ELSE 0 END) AS VoteAmount
// FROM
// Votes
// GROUP BY
// PostId) V ON P.Id = V.PostId
// GROUP BY
// U.Id, U.DisplayName
// )
// SELECT
// UMetrics.UserId,
// UMetrics.DisplayName,
// UMetrics.PostCount,
// UMetrics.QuestionCount,
// UMetrics.AnswerCount,
// UMetrics.TotalVotes,
// B.Count AS BadgeCount
// FROM
// UserMetrics UMetrics
// LEFT JOIN
// (SELECT UserId, COUNT(*) AS Count FROM Badges GROUP BY UserId) B ON UMetrics.UserId = B.UserId
// ORDER BY
// UMetrics.TotalVotes DESC;
fn q14628(db: &'static So) -> String {
    let va = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold(0i64, |a, t| {
        a + match t {
            2 => 1,
            3 => -1,
            _ => 0,
        }
    });
    let uf = g(db).select(posts_of(db).select((&db.post.post_type_id).and((&va).opt())).opt()).fold([0i64; 4], |a, p| match p {
        Some((t, x)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + x.unwrap_or(0)],
        None => a,
    });
    let bf = db.badge.group_by(&db.badge.user).fold(0i64, |a, _| a + 1);
    let mut v = Vec::new();
    (&uf).and((&bf).opt()).drive(|u, (a, b)| v.push((u, a, b)));
    rows(v.iter().map(|&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a));
        f.push(oint(b));
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
// SUM(CASE WHEN p.PostTypeId IN (3, 4, 5) THEN 1 ELSE 0 END) AS TotalWikis,
// SUM(c.Score) AS TotalCommentScore
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// GROUP BY
// u.Id, u.DisplayName, u.Reputation
// ),
// BadgeCounts AS (
// SELECT
// UserId,
// COUNT(*) AS TotalBadges
// FROM
// Badges
// GROUP BY
// UserId
// )
// SELECT
// us.UserId,
// us.DisplayName,
// us.Reputation,
// us.TotalPosts,
// us.TotalQuestions,
// us.TotalAnswers,
// us.TotalWikis,
// us.TotalCommentScore,
// COALESCE(bc.TotalBadges, 0) AS TotalBadges
// FROM
// UserStats us
// LEFT JOIN
// BadgeCounts bc ON us.UserId = bc.UserId
// ORDER BY
// us.Reputation DESC, us.TotalPosts DESC;
fn q11848(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "c", any_post);
    let dp = ud(db, UserWhere::All, posts_of(db));
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    (&us).and((&dp).opt()).and(&bu).drive(|u, ((a, d), b)| v.push((u, a, d.unwrap_or(0), b)));
    rows(v.iter().map(|&(u, a, d, b)| row(vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep"), V::I(d), V::I(a.q), V::I(a.a), V::I(a.t3 + a.t45), ustat_field(&a, "cscore_sum"), V::I(b)])))
}

// WITH PostStatistics AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// COUNT(c.Id) AS CommentCount,
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
// p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year'
// GROUP BY
// p.Id, p.Title, p.CreationDate
// ),
// PostHistoryCounts AS (
// SELECT
// ph.PostId,
// COUNT(*) AS HistoryCount
// FROM
// PostHistory ph
// GROUP BY
// ph.PostId
// )
// SELECT
// ps.PostId,
// ps.Title,
// ps.CreationDate,
// ps.CommentCount,
// ps.UpVotes,
// ps.DownVotes,
// phc.HistoryCount
// FROM
// PostStatistics ps
// LEFT JOIN
// PostHistoryCounts phc ON ps.PostId = phc.PostId
// ORDER BY
// ps.UpVotes DESC, ps.CommentCount DESC;
fn q10893(db: &'static So) -> String {
    let braw: HashIdx<i64, Id<Badge>> = (&db.badge.user_id).inv().collect();
    let hf = history_n_max(db);
    let pf = since(db, year_ago())
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(&db.vote.vote_type_id).opt()).and((&db.post.owner_user_id).select(&braw).opt()))
        .fold([0i64; 3], |a, ((c, t), _)| [a[0] + c.is_some() as i64, a[1] + (t == Some(2)) as i64, a[2] + (t == Some(3)) as i64]);
    let mut v = Vec::new();
    (&pf).and((&hf).opt()).drive(|p, (a, h)| v.push((p, a, h)));
    rows(v.iter().map(|&(p, a, h)| {
        let mut f = post_fields(db, p, &["id", "title", "created"]);
        f.extend([V::I(a[0]), V::I(a[1]), V::I(a[2]), oint(h.map(|h| h.0))]);
        row(f)
    }))
}

// WITH UserActivity AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(DISTINCT P.Id) AS PostCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotesReceived,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotesReceived,
// SUM(CASE WHEN B.Id IS NOT NULL THEN 1 ELSE 0 END) AS BadgesCount
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// WHERE
// U.Reputation > 1000
// GROUP BY
// U.Id, U.DisplayName
// ),
// TopUsers AS (
// SELECT
// UserId,
// DisplayName,
// PostCount,
// UpVotesReceived - DownVotesReceived AS NetVotes,
// BadgesCount
// FROM
// UserActivity
// ORDER BY
// PostCount DESC, NetVotes DESC
// LIMIT 10
// )
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// U.CreationDate,
// T.PostCount,
// T.NetVotes,
// T.BadgesCount
// FROM
// Users U
// JOIN
// TopUsers T ON U.Id = T.UserId
// ORDER BY
// U.Reputation DESC;
fn q7653(db: &'static So) -> String {
    let w = UserWhere::RepGt(1000);
    let uf = user_base(db, w)
        .group_by(Ident::<User>::new())
        .select(posts_of(db).select(votes_of(db).select(&db.vote.vote_type_id).opt()).opt().and(badges_of(db).opt()))
        .fold([0i64; 3], |a, (p, b)| {
            let t = p.flatten();
            [a[0] + (t == Some(2)) as i64, a[1] + (t == Some(3)) as i64, a[2] + b.is_some() as i64]
        });
    let dp = ud(db, w, posts_of(db));
    let base = user_base(db, w);
    let top: MatSet<Id<User>> = whole(&base)
        .select(Ident::<User>::new().and((&dp).opt()).and(&uf))
        .window(row_number, |((_, d), a): ((Id<User>, Option<i64>), [i64; 3])| (d.unwrap_or(0), a[0] - a[1]), desc)
        .filt(|(_, n)| n <= 10)
        .map(|(((u, _), _), _)| u)
        .collect();
    let mut v = Vec::new();
    (&top).select(Ident::<User>::new().and((&dp).opt()).and(&uf)).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((u, d), a)| row(vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep"), user_col(db, u, "ucreated"), V::I(d.unwrap_or(0)), V::I(a[0] - a[1]), V::I(a[2])])))
}

// WITH PostVoteStats AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.Score,
// P.CreationDate,
// COALESCE(SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount,
// COALESCE(SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount,
// COALESCE(COUNT(CASE WHEN C.PostId IS NOT NULL THEN 1 END), 0) AS CommentCount
// FROM
// Posts P
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// GROUP BY
// P.Id, P.Title, P.Score, P.CreationDate
// ),
// PostHistoryStats AS (
// SELECT
// PH.PostId,
// COUNT(PH.Id) AS RevisionCount,
// MAX(PH.CreationDate) AS LastEditDate
// FROM
// PostHistory PH
// GROUP BY
// PH.PostId
// )
// SELECT
// PVS.PostId,
// PVS.Title,
// PVS.Score,
// PVS.CreationDate,
// PVS.UpVoteCount,
// PVS.DownVoteCount,
// PVS.CommentCount,
// PHS.RevisionCount,
// PHS.LastEditDate
// FROM
// PostVoteStats PVS
// LEFT JOIN
// PostHistoryStats PHS ON PVS.PostId = PHS.PostId
// ORDER BY
// PVS.Score DESC, PVS.UpVoteCount DESC;
fn q13428(db: &'static So) -> String {
    let hf = history_n_max(db);
    let mut v = Vec::new();
    stats_fold(db, db.post.iq(), Ident::<Post>::new(), "cv", &[]).and((&hf).opt()).drive(|p, (s, h)| v.push((p, s, h)));
    rows(v.iter().map(|&(p, s, h)| {
        let mut f = post_fields(db, p, &["id", "title", "score", "created"]);
        f.extend([V::I(s.up), V::I(s.down), V::I(s.cx), oint(h.map(|h| h.0)), ots(h.map(|h| h.1))]);
        row(f)
    }))
}

// WITH RecentPosts AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.Score,
// p.AnswerCount,
// p.CommentCount,
// u.DisplayName AS OwnerDisplayName
// FROM
// Posts p
// JOIN
// Users u ON p.OwnerUserId = u.Id
// WHERE
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 month'
// ),
// PostStatistics AS (
// SELECT
// rp.PostId,
// COUNT(DISTINCT c.Id) AS CommentCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
// FROM
// RecentPosts rp
// LEFT JOIN
// Comments c ON rp.PostId = c.PostId
// LEFT JOIN
// Votes v ON rp.PostId = v.PostId
// GROUP BY
// rp.PostId
// )
// SELECT
// rp.Title,
// rp.CreationDate,
// rp.ViewCount,
// rp.Score,
// ps.CommentCount,
// ps.UpVotes,
// ps.DownVotes,
// rp.OwnerDisplayName
// FROM
// RecentPosts rp
// JOIN
// PostStatistics ps ON rp.PostId = ps.PostId
// ORDER BY
// rp.ViewCount DESC,
// rp.CreationDate DESC;
fn q13865(db: &'static So) -> String {
    let mut v = Vec::new();
    stats_fold(db, owned_since(db, month_ago()), Ident::<Post>::new(), "cv", &[]).and(comments_per_post(db)).drive(|p, (s, c)| v.push((p, s, c)));
    rows(v.iter().map(|&(p, s, c)| {
        let mut f = post_fields(db, p, &["title", "created", "views", "score"]);
        f.extend([V::I(c), V::I(s.up), V::I(s.down)]);
        f.extend(post_fields(db, p, &["owner"]));
        row(f)
    }))
}

// WITH PostStats AS (
// SELECT
// P.Id AS PostId,
// P.PostTypeId,
// P.CreationDate,
// COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
// COUNT(DISTINCT V.Id) AS VoteCount,
// SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
// SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
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
// U.Reputation,
// COUNT(DISTINCT B.Id) AS BadgeCount
// FROM
// Users U
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// GROUP BY
// U.Id, U.Reputation
// )
// SELECT
// PS.PostId,
// PS.PostTypeId,
// PS.CreationDate,
// PS.CommentCount,
// PS.VoteCount,
// PS.UpVotes,
// PS.DownVotes,
// US.UserId,
// US.Reputation,
// US.BadgeCount
// FROM
// PostStats PS
// JOIN
// Users U ON PS.PostTypeId = U.Id
// JOIN
// UserStats US ON U.Id = US.UserId
// ORDER BY
// PS.CreationDate DESC;
fn q14090(db: &'static So) -> String {
    let uid = uids(db);
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    stats_fold(db, db.post.with((&db.post.post_type_id).select(&uid)), Ident::<Post>::new(), "cv", &[])
        .and(votes_per_post(db))
        .and((&db.post.post_type_id).select(&uid).select(Ident::<User>::new().and(&bu)))
        .drive(|p, ((s, x), (u, b))| v.push((p, s, x, u, b)));
    rows(v.iter().map(|&(p, s, x, u, b)| {
        let mut f = post_fields(db, p, &["id", "type_id", "created"]);
        f.extend([V::I(s.cx), V::I(x), V::I(s.up), V::I(s.down), user_col(db, u, "uid"), user_col(db, u, "rep"), V::I(b)]);
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
// SUM(p.Score) AS TotalScore,
// SUM(p.ViewCount) AS TotalViews
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName
// ),
// PostHistoryStats AS (
// SELECT
// p.OwnerUserId,
// COUNT(ph.Id) AS TotalPostHistoryEntries,
// COUNT(DISTINCT ph.PostId) AS UniquePostsEdited
// FROM
// PostHistory ph
// JOIN
// Posts p ON ph.PostId = p.Id
// GROUP BY
// p.OwnerUserId
// )
// SELECT
// u.UserId,
// u.DisplayName,
// u.TotalPosts,
// u.Questions,
// u.Answers,
// u.TotalScore,
// u.TotalViews,
// COALESCE(ph.TotalPostHistoryEntries, 0) AS TotalPostHistoryEntries,
// COALESCE(ph.UniquePostsEdited, 0) AS UniquePostsEdited
// FROM
// UserPostStats u
// LEFT JOIN
// PostHistoryStats ph ON u.UserId = ph.OwnerUserId
// ORDER BY
// u.TotalScore DESC;
fn q13247(db: &'static So) -> String {
    let ps = pstat(db, db.post.iq());
    let hs = owned(db).group_by(&db.post.owner_user).select(history_per_post(db)).fold([0i64; 2], |a, h| [a[0] + h, a[1] + (h > 0) as i64]);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&ps).opt()).and((&hs).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((u, p), h)| {
        let q = p.unwrap_or(Z);
        let h = h.unwrap_or([0; 2]);
        row(vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(q[0]), V::I(q[1]), V::I(q[2]), oint(p.map(|p| p[3])), onull(p, pviews), V::I(h[0]), V::I(h[1])])
    }))
}

// WITH UserActivity AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(COALESCE(p.ViewCount, 0)) AS TotalViews,
// SUM(COALESCE(v.VoteAmount, 0)) AS TotalVotes,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// (SELECT
// PostId,
// SUM(CASE WHEN VoteTypeId = 2 THEN 1 WHEN VoteTypeId = 3 THEN -1 ELSE 0 END) AS VoteAmount
// FROM
// Votes
// GROUP BY
// PostId) v ON p.Id = v.PostId
// GROUP BY
// u.Id, u.DisplayName
// )
// SELECT
// ua.UserId,
// ua.DisplayName,
// ua.PostCount,
// ua.TotalViews,
// ua.TotalVotes,
// ua.QuestionCount,
// ua.AnswerCount,
// CAST(ua.TotalViews AS FLOAT) / NULLIF(ua.PostCount, 0) AS AverageViewsPerPost,
// CAST(ua.TotalVotes AS FLOAT) / NULLIF(ua.PostCount, 0) AS AverageVotesPerPost
// FROM
// UserActivity ua
// ORDER BY
// ua.TotalVotes DESC
// LIMIT 100;
fn q10349(db: &'static So) -> String {
    let va = db.vote.group_by(&db.vote.post).select(&db.vote.vote_type_id).fold(0i64, |a, t| {
        a + match t {
            2 => 1,
            3 => -1,
            _ => 0,
        }
    });
    let uf = g(db).select(posts_of(db).select((&db.post.view_count).opt().and((&va).opt()).and(&db.post.post_type_id)).opt()).fold([0i64; 5], |a, p| match p {
        Some(((w, x), t)) => [a[0] + 1, a[1] + w.unwrap_or(0), a[2] + x.unwrap_or(0), a[3] + (t == 1) as i64, a[4] + (t == 2) as i64],
        None => a,
    });
    let per = |x: i64, n: i64| if n == 0 { V::Null } else { V::F((x as f32 / n as f32) as f64) };
    let mut v = Vec::new();
    (&uf).drive(|u, a| v.push((u, a)));
    out(v, |&(_, a)| Reverse(a[2]), 100, |&(u, a)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a));
        f.extend([per(a[1], a[0]), per(a[2], a[0])]);
        f
    })
}

// WITH PostStats AS (
// SELECT
// P.Id AS PostId,
// P.CreationDate,
// P.Score,
// P.ViewCount,
// COUNT(CASE WHEN C.PostId IS NOT NULL THEN 1 END) AS CommentCount,
// COUNT(DISTINCT V.UserId) AS VoteCount
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// P.Id, P.CreationDate, P.Score, P.ViewCount
// ),
// UserStats AS (
// SELECT
// U.Id AS UserId,
// SUM(CASE WHEN P.OwnerUserId IS NOT NULL THEN 1 ELSE 0 END) AS PostsCreated,
// SUM(CASE WHEN B.UserId IS NOT NULL THEN 1 ELSE 0 END) AS BadgesEarned
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
// ps.PostId,
// ps.CreationDate,
// ps.Score,
// ps.ViewCount,
// ps.CommentCount,
// ps.VoteCount,
// us.UserId,
// us.PostsCreated,
// us.BadgesEarned
// FROM
// PostStats ps
// JOIN
// Users U ON ps.PostId = U.Id
// JOIN
// UserStats us ON U.Id = us.UserId
// ORDER BY
// ps.Score DESC, ps.ViewCount DESC;
fn q10266(db: &'static So) -> String {
    let uid = uids(db);
    let dv = per_post_distinct(db, votes_of(db).select(&db.vote.user_id));
    let us = g(db).select(posts_of(db).opt().and(badges_of(db).opt())).fold([0i64; 2], |a, (p, b)| [a[0] + p.is_some() as i64, a[1] + b.is_some() as i64]);
    let mut v = Vec::new();
    stats_fold(db, db.post.with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cv", &[])
        .and((&dv).opt())
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&us)))
        .drive(|p, ((s, d), (u, a))| v.push((p, s, d.unwrap_or(0), u, a)));
    rows(v.iter().map(|&(p, s, d, u, a)| {
        let mut f = post_fields(db, p, &["id", "created", "score", "views"]);
        f.extend([V::I(s.cx), V::I(d), user_col(db, u, "uid"), V::I(a[0]), V::I(a[1])]);
        row(f)
    }))
}

// WITH PostStatistics AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.Score,
// P.ViewCount,
// U.DisplayName AS OwnerDisplayName,
// COUNT(CASE WHEN C.PostId IS NOT NULL THEN 1 END) AS CommentCount,
// COUNT(CASE WHEN A.Id IS NOT NULL THEN 1 END) AS AnswerCount,
// PT.Name AS PostType,
// COALESCE(MAX(Vote.UserId), -1) AS LastVoterId
// FROM
// Posts P
// LEFT JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Posts A ON P.Id = A.ParentId
// LEFT JOIN
// PostTypes PT ON P.PostTypeId = PT.Id
// LEFT JOIN
// Votes Vote ON P.Id = Vote.PostId
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.Score, P.ViewCount, U.DisplayName, PT.Name
// )
// SELECT
// PS.PostId,
// PS.Title,
// PS.CreationDate,
// PS.Score,
// PS.ViewCount,
// PS.OwnerDisplayName,
// PS.PostType,
// PS.CommentCount,
// PS.AnswerCount,
// (SELECT COUNT(*) FROM Votes V WHERE V.PostId = PS.PostId) AS TotalVotes
// FROM
// PostStatistics PS
// ORDER BY
// PS.CreationDate DESC
// LIMIT 100;
fn q11993(db: &'static So) -> String {
    let mut v = Vec::new();
    stats_fold(db, db.post.iq(), Ident::<Post>::new(), "cav", &[]).and(votes_per_post(db)).drive(|p, (s, x)| v.push((p, s, x)));
    out(v, |&(p, _, _)| newest(db, p), 100, |&(p, s, x)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "score", "views", "owner", "type"]);
        f.extend(ints(&[s.cx, s.ax, x]));
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
// SUM(CASE WHEN p.PostTypeId = 3 THEN 1 ELSE 0 END) AS Wikes,
// SUM(p.Score) AS TotalScore,
// SUM(p.ViewCount) AS TotalViews
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
// )
// SELECT
// ups.UserId,
// ups.DisplayName,
// ups.TotalPosts,
// ups.Questions,
// ups.Answers,
// ups.Wikes,
// ups.TotalScore,
// ups.TotalViews,
// COALESCE(phs.TotalEdits, 0) AS TotalEdits,
// COALESCE(phs.UniquePostsEdited, 0) AS UniquePostsEdited
// FROM
// UserPostStats ups
// LEFT JOIN
// PostHistoryStats phs ON ups.UserId = phs.UserId
// ORDER BY
// ups.TotalScore DESC, ups.TotalPosts DESC;
fn q13191(db: &'static So) -> String {
    let ps = pstat(db, db.post.iq());
    let ph = phu(db);
    let hp = db.post_history.group_by(&db.post_history.user).select(&db.post_history.post).count_distinct();
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&ps).opt()).and((&ph).opt()).and((&hp).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(((u, p), h), d)| {
        let q = p.unwrap_or(Z);
        row(vec![
            user_col(db, u, "uid"),
            user_col(db, u, "name"),
            V::I(q[0]),
            V::I(q[1]),
            V::I(q[2]),
            V::I(q[8]),
            oint(p.map(|p| p[3])),
            onull(p, pviews),
            V::I(h.map_or(0, |h| h[0])),
            V::I(d.unwrap_or(0)),
        ])
    }))
}

// WITH UserActivity AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
// SUM(COALESCE(c.CommentCount, 0)) AS TotalCommentCount,
// SUM(COALESCE(v.VoteCount, 0)) AS TotalVoteCount,
// MAX(u.CreationDate) AS AccountCreated
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
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
// PostId,
// COUNT(Id) AS VoteCount
// FROM
// Votes
// GROUP BY
// PostId
// ) v ON p.Id = v.PostId
// GROUP BY
// u.Id, u.DisplayName
// )
// SELECT
// UserId,
// DisplayName,
// PostCount,
// QuestionCount,
// AnswerCount,
// TotalCommentCount,
// TotalVoteCount,
// AccountCreated
// FROM
// UserActivity
// ORDER BY
// PostCount DESC
// LIMIT 10;
fn q10981(db: &'static So) -> String {
    let ps = pstat(db, db.post.iq());
    let cv = owned(db).group_by(&db.post.owner_user).select(comments_per_post(db).and(votes_per_post(db))).fold([0i64; 2], |a, (c, x)| [a[0] + c, a[1] + x]);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&ps).opt()).and((&cv).opt())).drive(|_, ((u, p), c)| v.push((u, p.unwrap_or(Z), c.unwrap_or([0; 2]))));
    out(v, |&(_, p, _)| Reverse(p[0]), 10, |&(u, p, c)| vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(p[0]), V::I(p[1]), V::I(p[2]), V::I(c[0]), V::I(c[1]), user_col(db, u, "ucreated")])
}

// WITH PostStatistics AS (
// SELECT
// p.Id AS PostId,
// pt.Name AS PostType,
// COUNT(c.Id) AS CommentCount,
// COALESCE(SUM(v.BountyAmount), 0) AS TotalBountyAmount,
// AVG(p.Score) AS AverageScore,
// COUNT(DISTINCT b.Id) AS BadgeCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId AND v.VoteTypeId IN (8, 9)
// LEFT JOIN
// Badges b ON p.OwnerUserId = b.UserId
// JOIN
// PostTypes pt ON p.PostTypeId = pt.Id
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY
// p.Id, pt.Name
// ),
// PostTypeSummary AS (
// SELECT
// PostType,
// COUNT(PostId) AS TotalPosts,
// SUM(CommentCount) AS TotalComments,
// SUM(TotalBountyAmount) AS TotalBounty,
// AVG(AverageScore) AS AvgScore,
// SUM(BadgeCount) AS TotalBadges
// FROM
// PostStatistics
// GROUP BY
// PostType
// )
// SELECT
// PostType,
// TotalPosts,
// TotalComments,
// TotalBounty,
// AvgScore,
// TotalBadges
// FROM
// PostTypeSummary
// ORDER BY
// TotalPosts DESC;
fn q12306(db: &'static So) -> String {
    let braw: HashIdx<i64, Id<Badge>> = (&db.badge.user_id).inv().collect();
    let bcnt = db.badge.group_by(&db.badge.user_id).fold(0i64, |a, _| a + 1);
    let Vote { vote_type_id, bounty_amount, .. } = &db.vote;
    let pf = since(db, year_ago())
        .group_by(Ident::<Post>::new())
        .select(comments_of(db).opt().and(votes_of(db).select(vote_type_id.filt(|t| t == 8 || t == 9).and(bounty_amount.opt())).opt()).and((&db.post.owner_user_id).select(&braw).opt()))
        .fold([0i64; 2], |a, ((c, v), _)| [a[0] + c.is_some() as i64, a[1] + v.and_then(|(_, b)| b).unwrap_or(0)]);
    let f = by_key(since(db, year_ago()), name(db), (&pf).and(&db.post.score).and((&db.post.owner_user_id).select(&bcnt).opt()), [0i64; 5], |a, ((s, sc), b)| {
        [a[0] + 1, a[1] + s[0], a[2] + s[1], a[3] + sc, a[4] + b.unwrap_or(0)]
    });
    let mut v = Vec::new();
    (&f).drive(|k, a| v.push((k, a)));
    rows(v.iter().map(|&(k, a)| row(vec![V::S(k), V::I(a[0]), V::I(a[1]), V::I(a[2]), avg(a[3], a[0]), V::I(a[4])])))
}

// WITH UsersReputation AS (
// SELECT
// Id AS UserId,
// Reputation,
// CreationDate,
// LastAccessDate
// FROM Users
// ),
// PostStats AS (
// SELECT
// P.Id AS PostId,
// P.PostTypeId,
// COUNT(CASE WHEN C.PostId IS NOT NULL THEN 1 END) AS CommentCount,
// COUNT(DISTINCT V.Id) AS VoteCount,
// AVG(V.BountyAmount) AS AvgBountyAmount
// FROM Posts P
// LEFT JOIN Comments C ON P.Id = C.PostId
// LEFT JOIN Votes V ON P.Id = V.PostId
// GROUP BY P.Id, P.PostTypeId
// ),
// UserPosts AS (
// SELECT
// U.Id AS UserId,
// COUNT(P.Id) AS TotalPosts,
// SUM(P.ViewCount) AS TotalViews,
// AVG(P.Score) AS AvgScore
// FROM Users U
// JOIN Posts P ON U.Id = P.OwnerUserId
// GROUP BY U.Id
// )
// SELECT
// U.UserId,
// U.Reputation,
// U.CreationDate,
// U.LastAccessDate,
// PS.TotalPosts,
// PS.TotalViews,
// PS.AvgScore,
// PST.PostId,
// PST.PostTypeId,
// PST.CommentCount,
// PST.VoteCount,
// PST.AvgBountyAmount
// FROM UsersReputation U
// JOIN UserPosts PS ON U.UserId = PS.UserId
// JOIN PostStats PST ON PS.UserId = PST.PostId
// ORDER BY U.Reputation DESC, PS.TotalPosts DESC;
fn q10644(db: &'static So) -> String {
    let uid = uids(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    stats_fold(db, db.post.with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cv", &[])
        .and(votes_per_post(db))
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&ps)))
        .drive(|p, ((s, x), (u, a))| v.push((p, s, x, u, a)));
    rows(v.iter().map(|&(p, s, x, u, a)| {
        let mut f: Vec<V> = ["uid", "rep", "ucreated", "last_access"].iter().map(|c| user_col(db, u, c)).collect();
        f.extend([V::I(a[0]), pviews(a), pscore_avg(a)]);
        f.extend(post_fields(db, p, &["id", "type_id"]));
        f.extend([V::I(s.cx), V::I(x), stat_field(&s, "bounty_avg").unwrap()]);
        row(f)
    }))
}

// WITH PostTagCounts AS (
// SELECT
// p.Id AS PostId,
// p.Title AS PostTitle,
// unnest(string_to_array(substring(p.Tags, 2, length(p.Tags) - 2), '><')) AS Tag
// FROM
// Posts p
// WHERE
// p.PostTypeId = 1
// ),
// TagPopularities AS (
// SELECT
// Tag,
// COUNT(*) AS QuestionCount
// FROM
// PostTagCounts
// GROUP BY
// Tag
// ORDER BY
// QuestionCount DESC
// LIMIT 10
// ),
// LatestPosts AS (
// SELECT
// p.Id,
// p.Title,
// p.CreationDate,
// p.ViewCount,
// p.OwnerDisplayName,
// pt.QuestionCount
// FROM
// Posts p
// INNER JOIN
// TagPopularities pt ON pt.Tag = ANY(string_to_array(substring(p.Tags, 2, length(p.Tags) - 2), '><'))
// WHERE
// p.PostTypeId = 1
// ORDER BY
// p.LastActivityDate DESC
// LIMIT 5
// )
// SELECT
// lp.Title AS RecentQuestion,
// lp.CreationDate AS PostedOn,
// lp.ViewCount AS Views,
// lp.OwnerDisplayName AS Author,
// tp.Tag AS PopularTag,
// tp.QuestionCount AS TagUsage
// FROM
// LatestPosts lp
// JOIN
// TagPopularities tp ON lp.QuestionCount = tp.QuestionCount
// ORDER BY
// lp.CreationDate DESC;
fn q26918(db: &'static So) -> String {
    let tc = questions_only(db).group_by((&db.post.tags_str).flat_map(tag_list)).fold(0i64, |a, _| a + 1);
    let tcs: &'static Fold<Str, i64> = Box::leak(Box::new(tc));
    let top = top_by(tcs, 10);
    let topidx: HashIdx<Str, Str> = (&top).inv().collect();
    let bycount: HashIdx<i64, Str> = (&top).select(tcs).inv().collect();
    let pairs: MatSet<(Id<Post>, Str)> = questions_only(db).select(Ident::<Post>::new().and((&db.post.tags_str).flat_map(tag_list).select(&topidx))).collect();
    let top5: MatSet<(Id<Post>, Str)> = whole(&pairs)
        .select(Same::<(Id<Post>, Str)>::new().and(Same::<(Id<Post>, Str)>::new().map(|(p, _)| p).select(&db.post.last_activity_date)))
        .window(row_number, |(_, la): ((Id<Post>, Str), i64)| la, desc)
        .filt(|(_, n)| n <= 5)
        .map(|((k, _), _)| k)
        .collect();
    let mut v = Vec::new();
    (&top5).select(Same::<(Id<Post>, Str)>::new().map(|(_, t)| t).select(tcs).select(&bycount)).drive(|(p, _), t2| v.push((p, t2)));
    rows(v.iter().map(|&(p, t2)| {
        let mut f = post_fields(db, p, &["title", "created", "views", "owner_name"]);
        f.extend([V::S(t2), V::I(tcs.get(t2).unwrap())]);
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
// SUM(COALESCE(P.Score, 0)) AS TotalScore,
// SUM(COALESCE(P.ViewCount, 0)) AS TotalViews,
// COUNT(CASE WHEN B.Id IS NOT NULL THEN 1 END) AS TotalBadges
// FROM
// Users U
// LEFT JOIN
// Posts P ON P.OwnerUserId = U.Id
// LEFT JOIN
// Badges B ON B.UserId = U.Id
// GROUP BY
// U.Id, U.DisplayName
// ),
// PostHistoryStats AS (
// SELECT
// PH.PostId,
// COUNT(PH.Id) AS TotalHistoryEntries,
// MAX(PH.CreationDate) AS LastUpdated
// FROM
// PostHistory PH
// GROUP BY
// PH.PostId
// )
// SELECT
// UPS.UserId,
// UPS.DisplayName,
// UPS.TotalPosts,
// UPS.TotalQuestions,
// UPS.TotalAnswers,
// UPS.TotalScore,
// UPS.TotalViews,
// UPS.TotalBadges,
// PHS.TotalHistoryEntries,
// PHS.LastUpdated
// FROM
// UserPostStats UPS
// LEFT JOIN
// PostHistoryStats PHS ON UPS.UserId = PHS.PostId
// ORDER BY
// UPS.TotalPosts DESC;
fn q11808(db: &'static So) -> String {
    let pid = pids(db);
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "b", any_post);
    let hf = history_n_max(db);
    let mut v = Vec::new();
    (&us).and((&db.user.origid).select(&pid).select(&hf).opt()).drive(|u, (a, h)| v.push((u, a, h)));
    rows(v.iter().map(|&(u, a, h)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(["#n", "#q", "#a", "score_sum0", "views_sum0", "#bx"].iter().map(|c| ustat_field(&a, c)));
        f.extend([oint(h.map(|h| h.0)), ots(h.map(|h| h.1))]);
        row(f)
    }))
}

// WITH PostsSummary AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.ViewCount,
// P.Score,
// U.DisplayName AS OwnerDisplayName,
// COUNT(CASE WHEN C.PostId IS NOT NULL THEN 1 END) AS CommentCount,
// COUNT(CASE WHEN A.ParentId IS NOT NULL THEN 1 END) AS AnswerCount,
// COALESCE(BadgeCount.BadgeCount, 0) AS BadgeCount
// FROM
// Posts P
// LEFT JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Posts A ON P.Id = A.ParentId
// LEFT JOIN (
// SELECT
// UserId,
// COUNT(*) AS BadgeCount
// FROM
// Badges
// GROUP BY
// UserId
// ) BadgeCount ON U.Id = BadgeCount.UserId
// WHERE
// P.PostTypeId = 1
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.ViewCount, P.Score, U.DisplayName, BadgeCount.BadgeCount
// )
// SELECT
// PS.PostId,
// PS.Title,
// PS.CreationDate,
// PS.ViewCount,
// PS.Score,
// PS.OwnerDisplayName,
// PS.CommentCount,
// PS.AnswerCount,
// PS.BadgeCount
// FROM
// PostsSummary PS
// ORDER BY
// PS.CreationDate DESC
// LIMIT 100;
fn q13095(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    stats_fold(db, questions_only(db), Ident::<Post>::new(), "ca", &[]).and((&db.post.owner_user).select(&bu).opt()).drive(|p, (s, b)| v.push((p, s, b)));
    out(v, |&(p, _, _)| newest(db, p), 100, |&(p, s, b)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "owner"]);
        f.extend([V::I(s.cx), V::I(s.ax), V::I(b.unwrap_or(0))]);
        f
    })
}

// WITH UserScoreData AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// u.Reputation,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
// COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
// COALESCE(COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END), 0) AS QuestionCount,
// COALESCE(COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END), 0) AS AnswerCount,
// COALESCE(SUM(p.Score), 0) AS TotalScore
// FROM Users u
// LEFT JOIN Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN Votes v ON p.Id = v.PostId
// WHERE u.Reputation > 1000
// GROUP BY u.Id, u.DisplayName, u.Reputation
// ),
// UserBadgeCount AS (
// SELECT
// UserId,
// COUNT(*) AS BadgeCount
// FROM Badges
// GROUP BY UserId
// )
// SELECT
// usd.UserId,
// usd.DisplayName,
// usd.Reputation,
// usd.UpVotes,
// usd.DownVotes,
// usd.QuestionCount,
// usd.AnswerCount,
// usd.TotalScore,
// COALESCE(ubc.BadgeCount, 0) AS BadgeCount
// FROM UserScoreData usd
// LEFT JOIN UserBadgeCount ubc ON usd.UserId = ubc.UserId
// ORDER BY usd.TotalScore DESC, usd.Reputation DESC
// FETCH FIRST 10 ROWS ONLY;
fn q8785(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::RepGt(1000), "v", any_post);
    let ps = pstat(db, db.post.iq());
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    (&us).and((&ps).opt()).and(&bu).drive(|u, ((a, p), b)| v.push((u, a, p.unwrap_or(Z), b)));
    out(v, |&(u, a, _, _)| (Reverse(a.score_sum), rep_desc(db, u)), 10, |&(u, a, p, b)| {
        vec![user_col(db, u, "uid"), user_col(db, u, "name"), user_col(db, u, "rep"), V::I(a.up), V::I(a.down), V::I(p[1]), V::I(p[2]), V::I(a.score_sum), V::I(b)]
    })
}

// WITH PostStats AS (
// SELECT
// p.Id AS PostId,
// p.PostTypeId,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 month'
// GROUP BY
// p.Id, p.PostTypeId
// ),
// UserStats AS (
// SELECT
// u.Id AS UserId,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(u.UpVotes) AS TotalUpVotes,
// SUM(u.DownVotes) AS TotalDownVotes
// FROM
// Users u
// JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id
// )
// SELECT
// u.UserId,
// u.PostCount,
// u.TotalUpVotes,
// u.TotalDownVotes,
// ps.PostId,
// ps.PostTypeId,
// ps.CommentCount,
// ps.VoteCount,
// ps.UpVoteCount,
// ps.DownVoteCount
// FROM
// UserStats u
// JOIN
// PostStats ps ON u.UserId = ps.PostId
// ORDER BY
// u.PostCount DESC, ps.VoteCount DESC
// LIMIT 100;
fn q13200(db: &'static So) -> String {
    let uid = uids(db);
    let us = owned(db).group_by(&db.post.owner_user).select((&db.post.owner_user).select((&db.user.up_votes).and(&db.user.down_votes))).fold([0i64; 3], |a, (u, d)| [a[0] + 1, a[1] + u, a[2] + d]);
    let mut v = Vec::new();
    stats_fold(db, since(db, month_ago()).with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cv", &[])
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and(&us)))
        .drive(|p, (s, (u, a))| v.push((p, s, u, a)));
    out(v, |&(_, s, _, a)| (Reverse(a[0]), Reverse(s.vx)), 100, |&(p, s, u, a)| {
        let mut f = vec![user_col(db, u, "uid")];
        f.extend(ints(&a));
        f.extend(post_fields(db, p, &["id", "type_id"]));
        f.extend(ints(&[s.cx, s.vx, s.up, s.down]));
        f
    })
}

// WITH PostMetrics AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// COUNT(c.Id) AS CommentCount,
// COUNT(v.Id) AS VoteCount,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
// FROM
// Posts p
// LEFT JOIN
// Comments c ON c.PostId = p.Id
// LEFT JOIN
// Votes v ON v.PostId = p.Id
// WHERE
// p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
// GROUP BY
// p.Id, p.Title, p.CreationDate
// ), UserEngagement AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(DISTINCT p.Id) AS PostCount,
// SUM(pm.CommentCount) AS TotalComments,
// SUM(pm.VoteCount) AS TotalVotes
// FROM
// Users u
// LEFT JOIN
// Posts p ON p.OwnerUserId = u.Id
// LEFT JOIN
// PostMetrics pm ON pm.PostId = p.Id
// GROUP BY
// u.Id, u.DisplayName
// )
// SELECT
// ue.UserId,
// ue.DisplayName,
// ue.PostCount,
// ue.TotalComments,
// ue.TotalVotes
// FROM
// UserEngagement ue
// ORDER BY
// ue.PostCount DESC,
// ue.TotalVotes DESC
// LIMIT 50;
fn q11991(db: &'static So) -> String {
    let pf = stats_fold(db, since(db, year_ago()), Ident::<Post>::new(), "cv", &[]);
    let uf = g(db).select(posts_of(db).select((&pf).opt()).opt()).fold([0i64; 4], |a, p| match p {
        Some(s) => [a[0] + 1, a[1] + s.is_some() as i64, a[2] + s.map_or(0, |s| s.cx), a[3] + s.map_or(0, |s| s.vx)],
        None => a,
    });
    let mut v = Vec::new();
    (&uf).drive(|u, a| v.push((u, a)));
    out(v, |&(_, a)| (Reverse(a[0]), (a[1] == 0, Reverse(a[3]))), 50, |&(u, a)| {
        vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(a[0]), nullable(a[2], a[1]), nullable(a[3], a[1])]
    })
}

// WITH Benchmark AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// U.DisplayName AS OwnerDisplayName,
// P.Score,
// P.ViewCount,
// P.CreationDate,
// COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
// COUNT(CASE WHEN V.Id IS NOT NULL THEN 1 END) AS VoteCount,
// COUNT(DISTINCT B.Id) AS BadgeCount
// FROM
// Posts P
// LEFT JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// LEFT JOIN
// Badges B ON U.Id = B.UserId
// WHERE
// P.CreationDate >= '2023-01-01'
// GROUP BY
// P.Id, P.Title, U.DisplayName, P.Score, P.ViewCount, P.CreationDate
// )
// SELECT
// PostId,
// Title,
// OwnerDisplayName,
// Score,
// ViewCount,
// CreationDate,
// CommentCount,
// VoteCount,
// BadgeCount,
// (SELECT COUNT(*) FROM Posts) AS TotalPosts,
// (SELECT COUNT(*) FROM Users) AS TotalUsers,
// (SELECT COUNT(*) FROM Comments) AS TotalComments,
// (SELECT COUNT(*) FROM Votes) AS TotalVotes,
// (SELECT COUNT(*) FROM Badges) AS TotalBadges
// FROM
// Benchmark
// ORDER BY
// Score DESC, ViewCount DESC;
fn q10030(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let tot = [count(db.post.iq()), count(db.user.iq()), count(db.comment.iq()), count(db.vote.iq()), count(db.badge.iq())];
    let mut v = Vec::new();
    stats_fold(db, since(db, date(2023, 1, 1)), Ident::<Post>::new(), "cvb", &[]).and((&db.post.owner_user).select(&bu).opt()).drive(|p, (s, b)| v.push((p, s, b)));
    rows(v.iter().map(|&(p, s, b)| {
        let mut f = post_fields(db, p, &["id", "title", "owner", "score", "views", "created"]);
        f.extend([V::I(s.cx), V::I(s.vx), V::I(b.unwrap_or(0))]);
        f.extend(ints(&tot));
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
// COALESCE(SUM(p.Score), 0) AS TotalScore,
// COALESCE(SUM(p.ViewCount), 0) AS TotalViews,
// AVG(EXTRACT(EPOCH FROM (CURRENT_TIMESTAMP - p.CreationDate))) AS AvgPostAge
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// GROUP BY
// u.Id, u.DisplayName
// ),
// PostHistoryStats AS (
// SELECT
// ph.PostId,
// COUNT(ph.Id) AS TotalHistoryEntries,
// MAX(ph.CreationDate) AS LastEdited
// FROM
// PostHistory ph
// GROUP BY
// ph.PostId
// )
// SELECT
// ups.UserId,
// ups.DisplayName,
// ups.TotalPosts,
// ups.TotalQuestions,
// ups.TotalAnswers,
// ups.TotalScore,
// ups.TotalViews,
// phs.TotalHistoryEntries,
// phs.LastEdited
// FROM
// UserPostStats ups
// LEFT JOIN
// PostHistoryStats phs ON ups.UserId = (SELECT OwnerUserId FROM Posts WHERE Id = phs.PostId FETCH FIRST 1 ROW ONLY)
// ORDER BY
// ups.TotalScore DESC, ups.TotalPosts DESC;
fn q10992(db: &'static So) -> String {
    let ps = pstat(db, db.post.iq());
    let hf = history_n_max(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&ps).opt()).and(posts_of(db).select(&hf).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((u, p), h)| {
        let q = p.unwrap_or(Z);
        row(vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(q[0]), V::I(q[1]), V::I(q[2]), V::I(q[3]), V::I(q[5]), oint(h.map(|h| h.0)), ots(h.map(|h| h.1))])
    }))
}

// WITH UserPostStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// COUNT(DISTINCT P.Id) AS TotalPosts,
// SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(P.ViewCount) AS TotalViews,
// SUM(P.Score) AS TotalScore
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id, U.DisplayName
// ),
// UserCommentStats AS (
// SELECT
// C.UserId,
// COUNT(C.Id) AS TotalComments
// FROM
// Comments C
// GROUP BY
// C.UserId
// ),
// FinalResults AS (
// SELECT
// UPS.UserId,
// UPS.DisplayName,
// UPS.TotalPosts,
// UPS.TotalQuestions,
// UPS.TotalAnswers,
// UPS.TotalViews,
// UPS.TotalScore,
// COALESCE(UCS.TotalComments, 0) AS TotalComments
// FROM
// UserPostStats UPS
// LEFT JOIN
// UserCommentStats UCS ON UPS.UserId = UCS.UserId
// )
// SELECT
// UserId,
// DisplayName,
// TotalPosts,
// TotalQuestions,
// TotalAnswers,
// TotalComments,
// TotalViews,
// TotalScore
// FROM
// FinalResults
// ORDER BY
// TotalScore DESC;
fn q10788(db: &'static So) -> String {
    let ps = pstat(db, db.post.iq());
    let cu = comments_per_user(db);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&ps).opt()).and(&cu)).drive(|_, x| v.push(x));
    rows(v.iter().map(|&((u, p), c)| {
        let q = p.unwrap_or(Z);
        row(vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(q[0]), V::I(q[1]), V::I(q[2]), V::I(c), onull(p, pviews), oint(p.map(|p| p[3]))])
    }))
}

// WITH UserStats AS (
// SELECT
// U.Id AS UserId,
// U.DisplayName,
// U.Reputation,
// COUNT(DISTINCT P.Id) AS PostCount,
// SUM(CASE WHEN P.ViewCount > 1000 THEN 1 ELSE 0 END) AS PopularPosts,
// SUM(CASE WHEN P.Score > 50 THEN 1 ELSE 0 END) AS HighScorePosts
// FROM Users U
// LEFT JOIN Posts P ON U.Id = P.OwnerUserId
// WHERE U.Reputation > 100
// GROUP BY U.Id, U.DisplayName, U.Reputation
// ),
// BadgeCounts AS (
// SELECT
// B.UserId,
// COUNT(*) AS BadgeCount
// FROM Badges B
// GROUP BY B.UserId
// ),
// Report AS (
// SELECT
// US.UserId,
// US.DisplayName,
// US.Reputation,
// US.PostCount,
// US.PopularPosts,
// US.HighScorePosts,
// COALESCE(BC.BadgeCount, 0) AS BadgeCount
// FROM UserStats US
// LEFT JOIN BadgeCounts BC ON US.UserId = BC.UserId
// )
// SELECT
// R.DisplayName,
// R.Reputation,
// R.PostCount,
// R.PopularPosts,
// R.HighScorePosts,
// R.BadgeCount,
// CASE
// WHEN R.Reputation > 1000 THEN 'Elite'
// WHEN R.Reputation > 500 THEN 'Pro'
// ELSE 'Novice'
// END AS UserTier
// FROM Report R
// WHERE R.PostCount > 10
// ORDER BY R.Reputation DESC, R.PostCount DESC
// LIMIT 20;
fn q6597(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let ps = owned(db).group_by(&db.post.owner_user).select((&db.post.view_count).opt().and(&db.post.score)).fold([0i64; 3], |a, (w, s)| [a[0] + 1, a[1] + w.map_or(false, |w| w > 1000) as i64, a[2] + (s > 50) as i64]);
    let mut v = Vec::new();
    user_base(db, UserWhere::RepGt(100)).select(Ident::<User>::new().and((&ps).filt(|p: [i64; 3]| p[0] > 10)).and(&bu)).drive(|_, x| v.push(x));
    out(v, |&((u, p), _)| (rep_desc(db, u), Reverse(p[0])), 20, |&((u, p), b)| {
        let r = db.user.reputation.get(u).unwrap();
        let mut f = vec![user_col(db, u, "name"), V::I(r)];
        f.extend(ints(&p));
        f.push(V::I(b));
        f.push(V::S(if r > 1000 {
            "Elite"
        } else if r > 500 {
            "Pro"
        } else {
            "Novice"
        }));
        f
    })
}

// WITH user_post_counts AS (
// SELECT
// u.Id AS UserId,
// u.DisplayName,
// COUNT(p.Id) AS PostCount,
// COUNT(DISTINCT c.Id) AS CommentCount,
// SUM(CASE WHEN p.Score > 0 THEN 1 ELSE 0 END) AS UpvotedPostCount
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Comments c ON p.Id = c.PostId
// GROUP BY
// u.Id, u.DisplayName
// ),
// user_badges AS (
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
// up.UserId,
// up.DisplayName,
// up.PostCount,
// up.CommentCount,
// up.UpvotedPostCount,
// COALESCE(ub.BadgeCount, 0) AS TotalBadges,
// COALESCE(ub.GoldBadgeCount, 0) AS GoldBadges,
// COALESCE(ub.SilverBadgeCount, 0) AS SilverBadges,
// COALESCE(ub.BronzeBadgeCount, 0) AS BronzeBadges
// FROM
// user_post_counts up
// LEFT JOIN
// user_badges ub ON up.UserId = ub.UserId
// ORDER BY
// up.PostCount DESC;
fn q11318(db: &'static So) -> String {
    let uf = g(db).select(posts_of(db).select((&db.post.score).and(comments_of(db).opt())).opt()).fold([0i64; 2], |a, p| match p {
        Some((s, _)) => [a[0] + 1, a[1] + (s > 0) as i64],
        None => a,
    });
    let dc = ud(db, UserWhere::All, posts_of(db).select(comments_of(db)));
    let bc = badge_classes(db);
    let mut v = Vec::new();
    (&uf).and((&dc).opt()).and((&bc).opt()).drive(|u, ((a, c), b)| v.push((u, a, c.unwrap_or(0), b.unwrap_or([0; 4]))));
    rows(v.iter().map(|&(u, a, c, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(a[0]), V::I(c), V::I(a[1])];
        f.extend(ints(&b));
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
// SUM(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 ELSE 0 END) AS TotalAcceptedAnswers,
// SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
// SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
// FROM
// Users u
// LEFT JOIN
// Posts p ON u.Id = p.OwnerUserId
// LEFT JOIN
// Votes v ON p.Id = v.PostId
// GROUP BY
// u.Id, u.DisplayName
// ),
// BadgesPerUser AS (
// SELECT
// b.UserId,
// COUNT(b.Id) AS TotalBadges
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
// ups.TotalAcceptedAnswers,
// ups.TotalUpVotes,
// ups.TotalDownVotes,
// COALESCE(bpu.TotalBadges, 0) AS TotalBadges
// FROM
// UserPostStats ups
// LEFT JOIN
// BadgesPerUser bpu ON ups.UserId = bpu.UserId
// ORDER BY
// ups.TotalPosts DESC
// LIMIT 100;
fn q11335(db: &'static So) -> String {
    let Post { post_type_id, accepted_answer_id, .. } = &db.post;
    let uf = g(db).select(posts_of(db).select(post_type_id.and(accepted_answer_id.opt()).and(votes_of(db).select(&db.vote.vote_type_id).opt())).opt()).fold([0i64; 6], |a, p| match p {
        Some(((t, ac), x)) => [a[0] + 1, a[1] + (t == 1) as i64, a[2] + (t == 2) as i64, a[3] + ac.is_some() as i64, a[4] + (x == Some(2)) as i64, a[5] + (x == Some(3)) as i64],
        None => a,
    });
    let bu = badges_per_user(db);
    let mut v = Vec::new();
    (&uf).and(&bu).drive(|u, (a, b)| v.push((u, a, b)));
    out(v, |&(_, a, _)| Reverse(a[0]), 100, |&(u, a, b)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "name")];
        f.extend(ints(&a));
        f.push(V::I(b));
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
// U.DisplayName AS OwnerDisplayName,
// COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
// COUNT(A.Id) AS AnswerCount
// FROM
// Posts P
// LEFT JOIN
// Users U ON P.OwnerUserId = U.Id
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Posts A ON P.Id = A.ParentId AND A.PostTypeId = 2
// WHERE
// P.CreationDate >= '2023-01-01'
// GROUP BY
// P.Id, P.Title, P.CreationDate, P.ViewCount, P.Score, U.DisplayName
// ),
// VoteMetrics AS (
// SELECT
// postId,
// COUNT(CASE WHEN VoteTypeId = 2 THEN 1 END) AS UpVotes,
// COUNT(CASE WHEN VoteTypeId = 3 THEN 1 END) AS DownVotes
// FROM
// Votes
// GROUP BY
// postId
// )
// SELECT
// PM.PostId,
// PM.Title,
// PM.CreationDate,
// PM.ViewCount,
// PM.Score,
// PM.OwnerDisplayName,
// PM.CommentCount,
// PM.AnswerCount,
// COALESCE(VM.UpVotes, 0) AS UpVotes,
// COALESCE(VM.DownVotes, 0) AS DownVotes
// FROM
// PostMetrics PM
// LEFT JOIN
// VoteMetrics VM ON PM.PostId = VM.postId
// ORDER BY
// PM.CreationDate DESC;
fn q12567(db: &'static So) -> String {
    let pv = post_votes(db);
    let mut v = Vec::new();
    stats_fold(db, since(db, date(2023, 1, 1)), Ident::<Post>::new(), "cA", &[]).and((&pv).opt()).drive(|p, (s, x)| v.push((p, s, x.unwrap_or([0; 3]))));
    rows(v.iter().map(|&(p, s, x)| {
        let mut f = post_fields(db, p, &["id", "title", "created", "views", "score", "owner"]);
        f.extend(ints(&[s.cx, s.ax, x[1], x[2]]));
        row(f)
    }))
}

// WITH PostStatistics AS (
// SELECT
// P.Id AS PostId,
// P.PostTypeId,
// P.CreationDate,
// P.Score,
// P.ViewCount,
// COUNT(CASE WHEN C.PostId IS NOT NULL THEN 1 END) AS CommentCount,
// COUNT(CASE WHEN V.PostId IS NOT NULL THEN 1 END) AS VoteCount
// FROM
// Posts P
// LEFT JOIN
// Comments C ON P.Id = C.PostId
// LEFT JOIN
// Votes V ON P.Id = V.PostId
// GROUP BY
// P.Id, P.PostTypeId, P.CreationDate, P.Score, P.ViewCount
// ),
// UserStatistics AS (
// SELECT
// U.Id AS UserId,
// U.Reputation,
// COUNT(DISTINCT B.Id) AS BadgeCount,
// SUM(P.ViewCount) AS UserPostViewCount
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
// US.UserPostViewCount
// FROM
// PostStatistics PS
// JOIN
// Users U ON PS.PostTypeId = U.Id
// JOIN
// UserStatistics US ON U.Id = US.UserId
// ORDER BY
// PS.ViewCount DESC, PS.Score DESC;
fn q11477(db: &'static So) -> String {
    let uid = uids(db);
    let bu = badges_per_user(db);
    let uw = g(db).select(badges_of(db).opt().and(posts_of(db).select((&db.post.view_count).opt()).opt())).fold([0i64; 2], |a, (_, w)| {
        let w = w.flatten();
        [a[0] + w.is_some() as i64, a[1] + w.unwrap_or(0)]
    });
    let mut v = Vec::new();
    stats_fold(db, db.post.with((&db.post.post_type_id).select(&uid)), Ident::<Post>::new(), "cv", &[])
        .and((&db.post.post_type_id).select(&uid).select(Ident::<User>::new().and(&bu).and(&uw)))
        .drive(|p, (s, ((u, b), w))| v.push((p, s, u, b, w)));
    rows(v.iter().map(|&(p, s, u, b, w)| {
        let mut f = post_fields(db, p, &["id", "type_id", "created", "score", "views"]);
        f.extend([V::I(s.cx), V::I(s.vx), user_col(db, u, "uid"), user_col(db, u, "rep"), V::I(b), nullable(w[1], w[0])]);
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
// AveragePostScore AS (
// SELECT
// AVG(Score) AS AvgScore
// FROM
// Posts
// WHERE
// Score IS NOT NULL
// ),
// BadgesCount AS (
// SELECT
// UserId,
// COUNT(*) AS BadgeCount
// FROM
// Badges
// GROUP BY
// UserId
// )
// SELECT
// ups.UserId,
// ups.DisplayName,
// ups.PostCount,
// ups.QuestionCount,
// ups.AnswerCount,
// ups.UpvoteCount,
// ups.DownvoteCount,
// COALESCE(bc.BadgeCount, 0) AS BadgeCount,
// aps.AvgScore
// FROM
// UserPostStats ups
// LEFT JOIN
// BadgesCount bc ON ups.UserId = bc.UserId
// CROSS JOIN
// AveragePostScore aps
// ORDER BY
// ups.PostCount DESC;
fn q14011(db: &'static So) -> String {
    let us = user_stats_fold(db, Ident::<User>::new(), UserWhere::All, "v", any_post);
    let bu = badges_per_user(db);
    let p = post_totals(db);
    let mut v = Vec::new();
    (&us).and(&bu).drive(|u, (a, b)| v.push((u, a, b)));
    rows(v.iter().map(|&(u, a, b)| row(vec![user_col(db, u, "uid"), user_col(db, u, "name"), V::I(a.n), V::I(a.q), V::I(a.a), V::I(a.up), V::I(a.down), V::I(b), avg(p[1], p[0])])))
}

// WITH UserReputation AS (
// SELECT
// U.Id AS UserId,
// U.Reputation,
// COUNT(DISTINCT P.Id) AS PostCount,
// SUM(COALESCE(P.ViewCount, 0)) AS TotalViewCount,
// SUM(COALESCE(P.Score, 0)) AS TotalScore
// FROM
// Users U
// LEFT JOIN
// Posts P ON U.Id = P.OwnerUserId
// GROUP BY
// U.Id, U.Reputation
// ),
// PostStatistics AS (
// SELECT
// P.Id AS PostId,
// P.Title,
// P.CreationDate,
// P.Score,
// P.ViewCount,
// COUNT(C.Id) AS CommentCount,
// COUNT(CASE WHEN V.VoteTypeId = 2 THEN 1 END) AS UpVotes,
// COUNT(CASE WHEN V.VoteTypeId = 3 THEN 1 END) AS DownVotes
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
// U.UserId,
// U.Reputation,
// U.PostCount,
// U.TotalViewCount,
// U.TotalScore,
// PS.PostId,
// PS.Title,
// PS.CreationDate,
// PS.Score,
// PS.ViewCount,
// PS.CommentCount,
// PS.UpVotes,
// PS.DownVotes
// FROM
// UserReputation U
// JOIN
// PostStatistics PS ON U.UserId = PS.PostId
// ORDER BY
// U.Reputation DESC, PS.Score DESC;
fn q13926(db: &'static So) -> String {
    let uid = uids(db);
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    stats_fold(db, db.post.with((&db.post.origid).select(&uid)), Ident::<Post>::new(), "cv", &[])
        .and((&db.post.origid).select(&uid).select(Ident::<User>::new().and((&ps).opt())))
        .drive(|p, (s, (u, a))| v.push((p, s, u, a.unwrap_or(Z))));
    rows(v.iter().map(|&(p, s, u, a)| {
        let mut f = vec![user_col(db, u, "uid"), user_col(db, u, "rep"), V::I(a[0]), V::I(a[5]), V::I(a[3])];
        f.extend(post_fields(db, p, &["id", "title", "created", "score", "views"]));
        f.extend(ints(&[s.cx, s.up, s.down]));
        row(f)
    }))
}

// WITH PostSummary AS (
// SELECT
// p.OwnerUserId,
// COUNT(DISTINCT p.Id) AS TotalPosts,
// SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
// SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
// SUM(CASE WHEN p.PostTypeId = 3 THEN 1 ELSE 0 END) AS TotalWikis,
// AVG(p.Score) AS AverageScore,
// AVG(p.ViewCount) AS AverageViewCount
// FROM
// Posts p
// GROUP BY
// p.OwnerUserId
// ),
// UserSummary AS (
// SELECT
// u.Id AS UserId,
// u.Reputation,
// u.CreationDate,
// COALESCE(ps.TotalPosts, 0) AS TotalPosts,
// COALESCE(ps.TotalQuestions, 0) AS TotalQuestions,
// COALESCE(ps.TotalAnswers, 0) AS TotalAnswers,
// COALESCE(ps.TotalWikis, 0) AS TotalWikis,
// COALESCE(ps.AverageScore, 0) AS AverageScore,
// COALESCE(ps.AverageViewCount, 0) AS AverageViewCount
// FROM
// Users u
// LEFT JOIN
// PostSummary ps ON u.Id = ps.OwnerUserId
// )
// SELECT
// u.UserId,
// u.Reputation,
// u.CreationDate,
// u.TotalPosts,
// u.TotalQuestions,
// u.TotalAnswers,
// u.TotalWikis,
// u.AverageScore,
// u.AverageViewCount
// FROM
// UserSummary u
// ORDER BY
// u.Reputation DESC;
fn q11736(db: &'static So) -> String {
    let ps = pstat(db, db.post.iq());
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and((&ps).opt())).drive(|_, x| v.push(x));
    rows(v.iter().map(|&(u, p)| {
        let q = p.unwrap_or(Z);
        row(vec![
            user_col(db, u, "uid"),
            user_col(db, u, "rep"),
            user_col(db, u, "ucreated"),
            V::I(q[0]),
            V::I(q[1]),
            V::I(q[2]),
            V::I(q[8]),
            p.map_or(V::F(0.0), pscore_avg),
            if q[4] > 0 { pviews_avg(q) } else { V::F(0.0) },
        ])
    }))
}

// WITH UsersWithBadges AS (
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
// ActivePosts AS (
// SELECT
// p.Id AS PostId,
// p.Title,
// p.CreationDate,
// p.Score,
// p.ViewCount,
// p.OwnerUserId
// FROM
// Posts p
// WHERE
// p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days'
// ),
// UserPostActivity AS (
// SELECT
// u.UserId,
// COUNT(DISTINCT ap.PostId) AS ActivePostCount,
// SUM(COALESCE(ap.ViewCount, 0)) AS TotalViews,
// SUM(COALESCE(ap.Score, 0)) AS TotalScore
// FROM
// UsersWithBadges u
// LEFT JOIN
// ActivePosts ap ON u.UserId = ap.OwnerUserId
// GROUP BY
// u.UserId
// )
// SELECT
// ub.DisplayName,
// ub.BadgeCount,
// COALESCE(upa.ActivePostCount, 0) AS ActivePostCount,
// COALESCE(upa.TotalViews, 0) AS TotalViews,
// COALESCE(upa.TotalScore, 0) AS TotalScore
// FROM
// UsersWithBadges ub
// LEFT JOIN
// UserPostActivity upa ON ub.UserId = upa.UserId
// ORDER BY
// ub.BadgeCount DESC,
// TotalViews DESC,
// TotalScore DESC
// LIMIT 10;
fn q8930(db: &'static So) -> String {
    let bu = badges_per_user(db);
    let ap = owned_since(db, month_ago()).group_by(&db.post.owner_user).select((&db.post.view_count).opt().and(&db.post.score)).fold([0i64; 3], |a, (w, s)| [a[0] + 1, a[1] + w.unwrap_or(0), a[2] + s]);
    let mut v = Vec::new();
    db.user.select(Ident::<User>::new().and(&bu).and((&ap).opt())).drive(|_, ((u, b), p)| v.push((u, b, p.unwrap_or([0; 3]))));
    out(v, |&(_, b, p)| (Reverse(b), Reverse(p[1]), Reverse(p[2])), 10, |&(u, b, p)| vec![user_col(db, u, "name"), V::I(b), V::I(p[0]), V::I(p[1]), V::I(p[2])])
}

pub static ENTRIES: &[harness::Entry] = &[
    ("13934", q13934),
    ("12588", q12588),
    ("14302", q14302),
    ("11695", q11695),
    ("12733", q12733),
    ("13845", q13845),
    ("11657", q11657),
    ("13355", q13355),
    ("6458", q6458),
    ("11259", q11259),
    ("14741", q14741),
    ("13873", q13873),
    ("4255", q4255),
    ("13245", q13245),
    ("10537", q10537),
    ("11396", q11396),
    ("10766", q10766),
    ("28744", q28744),
    ("10137", q10137),
    ("8901", q8901),
    ("14628", q14628),
    ("11848", q11848),
    ("10893", q10893),
    ("7653", q7653),
    ("13428", q13428),
    ("13865", q13865),
    ("14090", q14090),
    ("13247", q13247),
    ("10349", q10349),
    ("10266", q10266),
    ("11993", q11993),
    ("13191", q13191),
    ("10981", q10981),
    ("12306", q12306),
    ("10644", q10644),
    ("26918", q26918),
    ("11808", q11808),
    ("13095", q13095),
    ("8785", q8785),
    ("13200", q13200),
    ("11991", q11991),
    ("10030", q10030),
    ("10992", q10992),
    ("10788", q10788),
    ("6597", q6597),
    ("11318", q11318),
    ("11335", q11335),
    ("12567", q12567),
    ("11477", q11477),
    ("14011", q14011),
    ("13926", q13926),
    ("11736", q11736),
    ("8930", q8930),
];
